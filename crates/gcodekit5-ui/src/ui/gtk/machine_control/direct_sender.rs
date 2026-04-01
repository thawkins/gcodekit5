use crate::t;
use gcodekit5_communication::{Communicator, SerialCommunicator};
use gcodekit5_core::ThreadSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

pub struct DirectSender {
    communicator: ThreadSafe<SerialCommunicator>,
    is_streaming: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    should_stop: Arc<AtomicBool>,
    progress_tx: mpsc::Sender<String>,
}

impl DirectSender {
    pub fn new(
        communicator: ThreadSafe<SerialCommunicator>,
        is_streaming: ThreadSafe<bool>,
        is_paused: ThreadSafe<bool>,
        _waiting_for_ack: ThreadSafe<bool>,
    ) -> (Self, mpsc::Receiver<String>) {
        let streaming_flag = Arc::new(AtomicBool::new(*is_streaming.lock()));
        let paused_flag = Arc::new(AtomicBool::new(*is_paused.lock()));
        let stop_flag = Arc::new(AtomicBool::new(false));

        *is_streaming.lock() = streaming_flag.load(Ordering::SeqCst);
        *is_paused.lock() = paused_flag.load(Ordering::SeqCst);

        let (tx, rx) = mpsc::channel();

        (
            Self {
                communicator,
                is_streaming: streaming_flag,
                is_paused: paused_flag,
                should_stop: stop_flag,
                progress_tx: tx,
            },
            rx,
        )
    }

    fn send_progress(&self, message: String) {
        let _ = self.progress_tx.send(message);
    }

    pub fn send_gcode(&self, content: &str) {
        if self.is_streaming.load(Ordering::SeqCst) {
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
            t!("Starting laser engraving:"),
            total_lines,
            t!("lines")
        ));

        self.is_streaming.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        self.should_stop.store(false, Ordering::SeqCst);

        let comm = self.communicator.clone();
        let streaming_flag = self.is_streaming.clone();
        let paused_flag = self.is_paused.clone();
        let stop_flag = self.should_stop.clone();
        let progress_tx = self.progress_tx.clone();

        thread::spawn(move || {
            let mut lines_in_flight: usize = 0;
            let max_window: usize = 40;
            let mut i = 0;
            let total = lines.len();
            let mut last_percent: u32 = 0;

            while i < total && !stop_flag.load(Ordering::SeqCst) {
                // SEND until the window is full
                while lines_in_flight < max_window && i < total {
                    if paused_flag.load(Ordering::SeqCst) {
                        break;
                    }

                    let cmd = format!("{}\n", lines[i]);
                    let mut c = comm.lock();
                    if c.send(cmd.as_bytes()).is_ok() {
                        lines_in_flight += 1;
                        i += 1;
                    } else {
                        break;
                    }
                }

                // If the window is full, force waiting for a real 'ok'
                let mut attempts = 0;
                while lines_in_flight >= max_window && !stop_flag.load(Ordering::SeqCst) {
                    {
                        let mut c = comm.lock();
                        if let Ok(data) = c.receive() {
                            let resp = String::from_utf8_lossy(&data);
                            let ok_count = resp.matches("ok").count();
                            lines_in_flight = lines_in_flight.saturating_sub(ok_count);
                        }
                    }

                    attempts += 1;
                    if attempts > 100 {
                        // Si tras muchos intentos no hay 'ok', damos un respiro al CPU
                        thread::sleep(Duration::from_millis(1));
                        attempts = 0;
                    }
                }

                // Read whatever has arrived even if the window is not full
                if let Ok(data) = {
                    let mut c = comm.lock();
                    c.receive()
                } {
                    let ok_count = String::from_utf8_lossy(&data).matches("ok").count();
                    lines_in_flight = lines_in_flight.saturating_sub(ok_count);
                }

                let percent = ((i as f64 / total as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u32;
                if percent != last_percent {
                    last_percent = percent;
                    let _ = progress_tx.send(format!("* {}%", percent));
                }
            }

            if !stop_flag.load(Ordering::SeqCst) {
                let _ = progress_tx.send("* 100%".to_string());
                let _ = progress_tx.send(t!("Work completed.").to_string());
            } else {
                let _ = progress_tx.send(t!("Work stopped.").to_string());
            }

            streaming_flag.store(false, Ordering::SeqCst);
        });
    }

    pub fn stop(&self) {
        let _ = self.progress_tx.send(t!("Stopping...").to_string());
        self.should_stop.store(true, Ordering::SeqCst);
    }

    pub fn pause(&self) {
        let _ = self.progress_tx.send(t!("Paused").to_string());
        self.is_paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        let _ = self.progress_tx.send(t!("Resuming").to_string());
        self.is_paused.store(false, Ordering::SeqCst);
    }

    pub fn unlock(&self) {
        let _ = self.progress_tx.send(t!("Unlocking...").to_string());

        let comm = self.communicator.clone();
        let _ = thread::spawn(move || {
            let mut comm = comm.lock();
            let _ = comm.send_command("$X");
        });
    }
}

impl Clone for DirectSender {
    fn clone(&self) -> Self {
        Self {
            communicator: self.communicator.clone(),
            is_streaming: self.is_streaming.clone(),
            is_paused: self.is_paused.clone(),
            should_stop: self.should_stop.clone(),
            progress_tx: self.progress_tx.clone(),
        }
    }
}
