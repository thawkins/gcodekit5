//! gcodekit5-ui/src/ui/gtk/machine_control/grbl_sender.rs
//! GRBL stream sender.
//! Improved file for raster engraving

// use crate::t;
use crate::firmware::Sender;
use crate::{Communicator, SerialCommunicator};
use gcodekit5_core::ThreadSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

pub struct GrblSender {
    communicator: ThreadSafe<SerialCommunicator>,
    // Shared with MachineControlView: the status poller checks this to yield the port during a job.
    is_streaming: ThreadSafe<bool>,
    should_stop: Arc<AtomicBool>,
    progress_tx: mpsc::Sender<String>,
}

impl GrblSender {
    pub fn new(
        communicator: ThreadSafe<SerialCommunicator>,
        is_streaming: ThreadSafe<bool>,
        _is_paused: ThreadSafe<bool>, // We retain the parameter for compatibility but do not use it
        _waiting_for_ack: ThreadSafe<bool>,
    ) -> (Self, mpsc::Receiver<String>) {
        let stop_flag = Arc::new(AtomicBool::new(false));
        *is_streaming.lock() = false;

        let (tx, rx) = mpsc::channel();
        (
            Self {
                communicator,
                is_streaming,
                should_stop: stop_flag,
                progress_tx: tx,
            },
            rx,
        )
    }

    fn send_progress(&self, message: String) {
        let _ = self.progress_tx.send(message);
    }

    fn send_gcode(&self, content: &str) {
        if *self.is_streaming.lock() {
            return;
        }

        let lines: Vec<String> = content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with(';') && !s.starts_with('('))
            .collect();

        if lines.is_empty() {
            return;
        }

        let total_lines = lines.len();
        self.send_progress(format!(
            "{} {} {}",
            "> Starting work:",
            total_lines,
            "lines"
        ));

        *self.is_streaming.lock() = true;
        self.should_stop.store(false, Ordering::SeqCst);

        let comm = self.communicator.clone();
        let streaming_flag = self.is_streaming.clone();
        let stop_flag = self.should_stop.clone();
        let progress_tx = self.progress_tx.clone();

        thread::spawn(move || {
            let mut lines_in_flight: usize = 0;
            let max_window: usize = 40;
            let mut i = 0;
            let total = lines.len();
            let mut last_percent: u32 = 0;

            while i < total && !stop_flag.load(Ordering::SeqCst) {
                // Send until the window is full
                {
                    if let Some(mut c) = comm.try_lock() {
                        while lines_in_flight < max_window && i < total {
                            let cmd = format!("{}\n", lines[i]);
                            if c.send(cmd.as_bytes()).is_ok() {
                                let _ = progress_tx.send(format!("> {}", lines[i]));
                                lines_in_flight += 1;
                                i += 1;
                            } else {
                                break;
                            }
                        }
                    } else {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                }

                // Read responses and process approvals
                let mut attempts = 0;
                while lines_in_flight >= max_window && !stop_flag.load(Ordering::SeqCst) {
                    if let Some(mut c) = comm.try_lock() {
                        if let Ok(data) = c.receive() {
                            if !data.is_empty() {
                                let resp = String::from_utf8_lossy(&data);
                                let ok_count = resp.matches("ok").count();
                                lines_in_flight = lines_in_flight.saturating_sub(ok_count);
                                if resp.contains("Grbl") || resp.contains("ALARM") {
                                    stop_flag.store(true, Ordering::SeqCst);
                                    break;
                                }
                            }
                        }
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }

                    attempts += 1;
                    if attempts > 100 {
                        thread::sleep(Duration::from_millis(1));
                        attempts = 0;
                    }
                }

                // Residual reading
                if let Some(mut c) = comm.try_lock() {
                    if let Ok(data) = c.receive() {
                        if !data.is_empty() {
                            let resp = String::from_utf8_lossy(&data);
                            let ok_count = resp.matches("ok").count();
                            lines_in_flight = lines_in_flight.saturating_sub(ok_count);

                            if resp.contains("Grbl") || resp.contains("ALARM") {
                                stop_flag.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                } else {
                    thread::sleep(Duration::from_millis(1));
                }

                thread::yield_now();

                let percent = ((i as f64 / total as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u32;
                if percent != last_percent {
                    last_percent = percent;
                    let _ = progress_tx.send(format!("* {}%", percent));
                }
            }

            *streaming_flag.lock() = false;
            let _ = progress_tx.send("Job Completed".to_string());
        });
    }

    fn stop(&self) {
        let _ = self.progress_tx.send("Stopping...".to_string());
        self.should_stop.store(true, Ordering::SeqCst);
    }

    /// Pause the current job by sending the GRBL feed hold command (!)
    fn pause(&self) {
        if let Some(mut comm) = self.communicator.try_lock() {
            if let Err(e) = comm.send_command("!") {
                tracing::error!("Failed to send pause to GRBL: {}", e);
            } else {
                tracing::debug!("Pause command (!) sent to GRBL");
            }
        } else {
            tracing::warn!("Could not acquire lock for pause");
        }

        let _ = self.progress_tx.send("Paused".to_string());
    }

    /// Resume the current job by sending the GRBL resume command (~)
    fn resume(&self) {
        if let Some(mut comm) = self.communicator.try_lock() {
            if let Err(e) = comm.send_command("~") {
                tracing::error!("Failed to send resume to GRBL: {}", e);
            } else {
                tracing::debug!("Resume command (~) sent to GRBL");
            }
        } else {
            tracing::warn!("Could not acquire lock for resume");
        }

        let _ = self.progress_tx.send("Resuming".to_string());
    }

    fn unlock(&self) {
        if let Some(mut comm) = self.communicator.try_lock() {
            let _ = comm.send_command("$X");
        } else {
            tracing::warn!("Could not acquire lock for unlock");
        }

        let _ = self.progress_tx.send("Unlocking...".to_string());
    }
}

impl Clone for GrblSender {
    fn clone(&self) -> Self {
        Self {
            communicator: self.communicator.clone(),
            is_streaming: self.is_streaming.clone(),
            should_stop: self.should_stop.clone(),
            progress_tx: self.progress_tx.clone(),
        }
    }
}

impl Sender for GrblSender {
    fn send_gcode(&self, content: &str) {
        if *self.is_streaming.lock() {
            return;
        }

        let lines: Vec<String> = content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with(';') && !s.starts_with('('))
            .collect();

        if lines.is_empty() {
            return;
        }

        let total_lines = lines.len();
        self.send_progress(format!("> Starting work: {} lines", total_lines));

        *self.is_streaming.lock() = true;
        self.should_stop.store(false, Ordering::SeqCst);

        let comm = self.communicator.clone();
        let streaming_flag = self.is_streaming.clone();
        let stop_flag = self.should_stop.clone();
        let progress_tx = self.progress_tx.clone();

        thread::spawn(move || {
            let mut lines_in_flight: usize = 0;
            let max_window: usize = 40;
            let mut i = 0;
            let total = lines.len();
            let mut last_percent: u32 = 0;

            while i < total && !stop_flag.load(Ordering::SeqCst) {
                // Send until the window is full
                {
                    if let Some(mut c) = comm.try_lock() {
                        while lines_in_flight < max_window && i < total {
                            let cmd = format!("{}\n", lines[i]);
                            if c.send(cmd.as_bytes()).is_ok() {
                                let _ = progress_tx.send(format!("> {}", lines[i]));
                                lines_in_flight += 1;
                                i += 1;
                            } else {
                                break;
                            }
                        }
                    } else {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                }

                // Read responses and process approvals
                let mut attempts = 0;
                while lines_in_flight >= max_window && !stop_flag.load(Ordering::SeqCst) {
                    if let Some(mut c) = comm.try_lock() {
                        if let Ok(data) = c.receive() {
                            if !data.is_empty() {
                                let resp = String::from_utf8_lossy(&data);
                                let ok_count = resp.matches("ok").count();
                                lines_in_flight = lines_in_flight.saturating_sub(ok_count);
                                if resp.contains("Grbl") || resp.contains("ALARM") {
                                    stop_flag.store(true, Ordering::SeqCst);
                                    break;
                                }
                            }
                        }
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }

                    attempts += 1;
                    if attempts > 100 {
                        thread::sleep(Duration::from_millis(1));
                        attempts = 0;
                    }
                }

                // Residual reading
                if let Some(mut c) = comm.try_lock() {
                    if let Ok(data) = c.receive() {
                        if !data.is_empty() {
                            let resp = String::from_utf8_lossy(&data);
                            let ok_count = resp.matches("ok").count();
                            lines_in_flight = lines_in_flight.saturating_sub(ok_count);

                            if resp.contains("Grbl") || resp.contains("ALARM") {
                                stop_flag.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                } else {
                    thread::sleep(Duration::from_millis(1));
                }

                thread::yield_now();

                let percent = ((i as f64 / total as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u32;
                if percent != last_percent {
                    last_percent = percent;
                    let _ = progress_tx.send(format!("* {}%", percent));
                }
            }

            *streaming_flag.lock() = false;
            let _ = progress_tx.send("Job Completed".to_string());
        });
    }

    fn pause(&self) {
        if let Some(mut comm) = self.communicator.try_lock() {
            if let Err(e) = comm.send_command("!") {
                tracing::error!("Failed to send pause to GRBL: {}", e);
            } else {
                tracing::debug!("Pause command (!) sent to GRBL");
                let _ = self.progress_tx.send("> !".to_string());
            }
        } else {
            tracing::warn!("Could not acquire lock for pause");
        }

        let _ = self.progress_tx.send("Paused".to_string());
    }

    fn resume(&self) {
        if let Some(mut comm) = self.communicator.try_lock() {
            if let Err(e) = comm.send_command("~") {
                tracing::error!("Failed to send resume to GRBL: {}", e);
            } else {
                tracing::debug!("Resume command (~) sent to GRBL");
                let _ = self.progress_tx.send("> ~".to_string());
            }
        } else {
            tracing::warn!("Could not acquire lock for resume");
        }

        let _ = self.progress_tx.send("Resuming".to_string());
    }

    fn stop(&self) {
        let _ = self.progress_tx.send("Stopping...".to_string());
        self.should_stop.store(true, Ordering::SeqCst);
    }

    fn unlock(&self) {
        if let Some(mut comm) = self.communicator.try_lock() {
            if comm.send_command("$X").is_ok() {
                let _ = self.progress_tx.send("> $X".to_string());
            }
        } else {
            tracing::warn!("Could not acquire lock for unlock");
        }

        let _ = self.progress_tx.send("Unlocking...".to_string());
    }

    fn is_streaming(&self) -> bool {
        *self.is_streaming.lock()
    }
}
