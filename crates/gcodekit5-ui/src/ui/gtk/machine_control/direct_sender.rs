use gcodekit5_communication::{Communicator, SerialCommunicator};
use gcodekit5_core::ThreadSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::thread;

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

        (Self {
            communicator,
            is_streaming: streaming_flag,
            is_paused: paused_flag,
            should_stop: stop_flag,
            progress_tx: tx,
        }, rx)
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
        self.send_progress(format!("Starting laser engraving: {} lines", total_lines));

        self.is_streaming.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        self.should_stop.store(false, Ordering::SeqCst);

        let comm = self.communicator.clone();
        let streaming_flag = self.is_streaming.clone();
        let paused_flag = self.is_paused.clone();
        let stop_flag = self.should_stop.clone();
        let progress_tx = self.progress_tx.clone();
        let start_time = std::time::Instant::now();

        thread::spawn(move || {
            for (i, line) in lines.iter().enumerate() {
                if stop_flag.load(Ordering::SeqCst) {
                    let _ = progress_tx.send(format!("Arrested online {}/{}", i, total_lines));
                    break;
                }

                while paused_flag.load(Ordering::SeqCst) && !stop_flag.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(10));
                }

                let send_result = {
                    let mut comm = comm.lock();
                    let cmd_with_newline = format!("{}\n", line);
                    comm.send(cmd_with_newline.as_bytes())
                };

                match send_result {
                    Ok(_) => {
                        let mut retries = 0;
                        let max_retries = 50;

                        loop {
                            if stop_flag.load(Ordering::SeqCst) { break; }

                            let response = {
                                let mut comm = comm.lock();
                                comm.receive()
                            };


                            match response {
                                Ok(data) if !data.is_empty() => {
                                    let resp_str = String::from_utf8_lossy(&data);
                                    if resp_str.contains("ok") { break; }
                                    if resp_str.contains("error") {
                                        let _ = progress_tx.send(format!("⚠️ Error: {}", resp_str));
                                        break;
                                    }
                                }
                                _ => {
                                    retries += 1;
                                    if retries >= max_retries {
                                        break;
                                    }
                                    thread::sleep(Duration::from_micros(10));
                                }
                            }

                        }
                        thread::sleep(Duration::from_micros(10));
                    }
                    Err(e) => {
                        let _ = progress_tx.send(format!("❌ Error: {}", e));
                        break;
                    }
                }

                if i > 0 && i % 1000 == 0 {
                    let progress = (i as f64 / total_lines as f64) * 100.0;
                    let _ = progress_tx.send(format!("* {:.1}% ({}/{})",
                                                      progress, i, total_lines));

                }
            }

            let elapsed = start_time.elapsed().as_secs_f64();
            let _ = progress_tx.send(format!("✅ Completed in {:.1}s | {} lines", elapsed, total_lines));

            streaming_flag.store(false, Ordering::SeqCst);
            paused_flag.store(false, Ordering::SeqCst);
            stop_flag.store(false, Ordering::SeqCst);
        });
    }


    pub fn stop(&self) {
        let _ = self.progress_tx.send("Stopping...".to_string());
        self.should_stop.store(true, Ordering::SeqCst);
    }

    pub fn pause(&self) {
        let _ = self.progress_tx.send("Paused".to_string());
        self.is_paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        let _ = self.progress_tx.send("Resuming".to_string());
        self.is_paused.store(false, Ordering::SeqCst);
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
