//! # Direct G-code sender
//!
//! Streams a G-code program to a GRBL controller on a background thread using GRBL's
//! character-counting protocol (see [`super::grbl_stream`]), polls status with `?` while
//! the job runs and reports progress, status and the outcome to the UI over a channel.

use super::grbl_stream::{
    classify_response, CharCountWindow, GrblResponseLine, ResponseLineSplitter,
};
use crate::t;
use gcodekit5_communication::{Communicator, SerialCommunicator};
use gcodekit5_core::ThreadSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

/// How often the streaming thread asks GRBL for a status report (`?`).
const STATUS_QUERY_INTERVAL: Duration = Duration::from_millis(200);

/// How a streamed job ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobOutcome {
    /// Every line was sent and acknowledged.
    Completed,
    /// The user stopped the job (Stop / E-stop).
    Cancelled,
    /// The controller or the connection stopped the job; holds the reason.
    Stopped(String),
}

/// Events sent from the streaming thread to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenderEvent {
    /// Text for the device console.
    Log(String),
    /// Percentage of lines acknowledged by the controller.
    Progress(u32),
    /// A `<...>` real-time status report, for the DRO / state display.
    Status(String),
    /// Any other controller response (`error:`, `ALARM:`, `[MSG:]`, ...), for the console.
    Response(String),
    /// The job ended; the worker no longer owns the port.
    Finished(JobOutcome),
}

/// Streams a G-code program to GRBL from a background thread using character counting.
pub struct DirectSender {
    communicator: ThreadSafe<SerialCommunicator>,
    // Shared with MachineControlView: the status poller checks this to yield the port during a job.
    is_streaming: ThreadSafe<bool>,
    should_stop: Arc<AtomicBool>,
    // True only while this sender's worker thread owns the serial port.
    running: Arc<AtomicBool>,
    progress_tx: mpsc::Sender<SenderEvent>,
}

impl DirectSender {
    /// Creates a sender and the receiver for its [`SenderEvent`]s.
    ///
    /// # Arguments
    /// * `communicator` - The shared serial connection.
    /// * `is_streaming` - Shared "job is streaming" flag, set while the worker runs.
    ///
    /// # Returns
    /// The sender and the event receiver the UI polls.
    pub fn new(
        communicator: ThreadSafe<SerialCommunicator>,
        is_streaming: ThreadSafe<bool>,
    ) -> (Self, mpsc::Receiver<SenderEvent>) {
        let (tx, rx) = mpsc::channel();
        (
            Self {
                communicator,
                is_streaming,
                should_stop: Arc::new(AtomicBool::new(false)),
                running: Arc::new(AtomicBool::new(false)),
                progress_tx: tx,
            },
            rx,
        )
    }

    /// Returns true while the background streaming thread is active and owns the port.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn send_event(&self, event: SenderEvent) {
        let _ = self.progress_tx.send(event);
    }

    /// Starts streaming `content` on a background thread.
    ///
    /// Comment and blank lines are skipped. The job is refused (with a log message and a
    /// `Finished(Stopped)` event) if a line can never fit into GRBL's RX buffer.
    ///
    /// # Arguments
    /// * `content` - The G-code program.
    pub fn send_gcode(&self, content: &str) {
        if *self.is_streaming.lock() || self.is_running() {
            return;
        }

        let lines: Vec<String> = content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with(';') && !s.starts_with('('))
            .collect();

        if lines.is_empty() {
            self.send_event(SenderEvent::Finished(JobOutcome::Stopped(
                t!("no G-code lines to send").to_string(),
            )));
            return;
        }

        let window = CharCountWindow::default();
        if let Some((n, line)) = lines
            .iter()
            .enumerate()
            .find(|(_, l)| !window.fits_at_all(l))
        {
            self.send_event(SenderEvent::Finished(JobOutcome::Stopped(format!(
                "{} {} ({} bytes)",
                t!("line too long for the controller buffer:"),
                n + 1,
                line.len()
            ))));
            return;
        }

        let total_lines = lines.len();
        self.send_event(SenderEvent::Log(format!(
            "{} {} {}",
            t!("> Starting job:"),
            total_lines,
            t!("lines")
        )));

        *self.is_streaming.lock() = true;
        self.should_stop.store(false, Ordering::SeqCst);
        self.running.store(true, Ordering::SeqCst);

        let comm = self.communicator.clone();
        let streaming_flag = self.is_streaming.clone();
        let running_flag = self.running.clone();
        let stop_flag = self.should_stop.clone();
        let tx = self.progress_tx.clone();

        thread::spawn(move || {
            let outcome = stream_lines(&lines, &comm, &stop_flag, &tx);
            *streaming_flag.lock() = false;
            running_flag.store(false, Ordering::SeqCst);
            let _ = tx.send(SenderEvent::Finished(outcome));
        });
    }

    /// Stops the job: no further line is sent once this returns.
    ///
    /// The worker re-checks the flag while holding the communicator lock, so a caller
    /// that sends a soft reset after this (under the lock) cannot be followed by a line.
    pub fn stop(&self) {
        self.send_event(SenderEvent::Log(t!("Stopping...").to_string()));
        self.should_stop.store(true, Ordering::SeqCst);
    }

    /// Pause the current job by sending the GRBL feed hold command (!)
    ///
    /// This sends the ASCII character '!' (0x21) to GRBL which immediately
    /// pauses the machine. The command buffer in GRBL remains intact.
    /// The send thread keeps the RX buffer topped up and keeps polling status.
    pub fn pause(&self) {
        // Realtime command: send the raw byte only. `send_command("!")` would append a
        // newline, which GRBL answers with an extra "ok" and corrupts the byte count.
        // The lock is taken blocking so a pause is never silently dropped under contention.
        if let Err(e) = self.communicator.lock().send(b"!") {
            tracing::error!("Failed to send pause to GRBL: {}", e);
        }
        self.send_event(SenderEvent::Log(t!("Paused").to_string()));
    }

    /// Resume the current job by sending the GRBL resume command (~)
    ///
    /// This sends the ASCII character '~' (0x7E) to GRBL which resumes
    /// the paused job exactly where it was interrupted.
    pub fn resume(&self) {
        // Raw realtime byte (see `pause`).
        if let Err(e) = self.communicator.lock().send(b"~") {
            tracing::error!("Failed to send resume to GRBL: {}", e);
        }
        self.send_event(SenderEvent::Log(t!("Resuming").to_string()));
    }

    /// Calcula el tiempo estimado de ejecución en segundos analizando el G-code
    pub fn estimate_execution_time(gcode: &str) -> f64 {
        let mut total_time = 0.0;
        let mut current_feed = 0.0;
        let mut last_x = 0.0;
        let mut last_y = 0.0;
        let mut last_z = 0.0;
        let mut is_absolute = true;

        for line in gcode.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }

            if line.contains("G90") {
                is_absolute = true;
            } else if line.contains("G91") {
                is_absolute = false;
            }

            if let Some(f_pos) = line.find('F') {
                let f_end = line[f_pos + 1..]
                    .find(|c: char| !c.is_ascii_digit() && c != '.')
                    .map_or(line.len(), |idx| f_pos + 1 + idx);
                if let Ok(feed) = line[f_pos + 1..f_end].parse::<f64>() {
                    current_feed = feed;
                }
            }

            let is_move = line.contains("G0")
                || line.contains("G1")
                || line.contains("G00")
                || line.contains("G01");

            if is_move {
                let mut x = last_x;
                let mut y = last_y;
                let mut z = last_z;

                if let Some(x_pos) = line.find('X') {
                    let x_end = line[x_pos + 1..]
                        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                        .map_or(line.len(), |idx| x_pos + 1 + idx);
                    if let Ok(val) = line[x_pos + 1..x_end].parse::<f64>() {
                        x = if is_absolute { val } else { last_x + val };
                    }
                }

                if let Some(y_pos) = line.find('Y') {
                    let y_end = line[y_pos + 1..]
                        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                        .map_or(line.len(), |idx| y_pos + 1 + idx);
                    if let Ok(val) = line[y_pos + 1..y_end].parse::<f64>() {
                        y = if is_absolute { val } else { last_y + val };
                    }
                }

                if let Some(z_pos) = line.find('Z') {
                    let z_end = line[z_pos + 1..]
                        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                        .map_or(line.len(), |idx| z_pos + 1 + idx);
                    if let Ok(val) = line[z_pos + 1..z_end].parse::<f64>() {
                        z = if is_absolute { val } else { last_z + val };
                    }
                }

                let dx = x - last_x;
                let dy = y - last_y;
                let dz = z - last_z;
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                if distance > 0.0 && current_feed > 0.0 {
                    total_time += distance / (current_feed / 60.0);
                }

                last_x = x;
                last_y = y;
                last_z = z;
            }
        }

        total_time
    }
}

impl Clone for DirectSender {
    fn clone(&self) -> Self {
        Self {
            communicator: self.communicator.clone(),
            is_streaming: self.is_streaming.clone(),
            should_stop: self.should_stop.clone(),
            running: self.running.clone(),
            progress_tx: self.progress_tx.clone(),
        }
    }
}

/// Writes all of `data`, retrying short writes.
fn send_all(comm: &mut SerialCommunicator, data: &[u8]) -> Result<(), String> {
    let mut offset = 0;
    let mut zero_writes = 0;
    while offset < data.len() {
        match comm.send(&data[offset..]) {
            Ok(0) => {
                zero_writes += 1;
                if zero_writes > 100 {
                    return Err("serial port accepted no data".to_string());
                }
                thread::sleep(Duration::from_millis(1));
            }
            Ok(n) => offset += n,
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

/// The streaming loop run by the worker thread.
///
/// Implements GRBL character counting: a line is only written when it fits into the
/// controller's RX buffer together with all unacknowledged lines. Each `ok` or `error:N`
/// line acknowledges one line. A status query (`?`) is sent every
/// [`STATUS_QUERY_INTERVAL`] and the reports are forwarded to the UI.
///
/// # Returns
/// How the job ended.
fn stream_lines(
    lines: &[String],
    comm: &ThreadSafe<SerialCommunicator>,
    stop_flag: &AtomicBool,
    tx: &mpsc::Sender<SenderEvent>,
) -> JobOutcome {
    let total = lines.len();
    let mut next = 0usize;
    let mut acked = 0usize;
    let mut window = CharCountWindow::default();
    let mut splitter = ResponseLineSplitter::new();
    let mut last_query: Option<Instant> = None;
    let mut last_percent: Option<u32> = None;

    loop {
        if stop_flag.load(Ordering::SeqCst) {
            return JobOutcome::Cancelled;
        }
        if next >= total && window.is_empty() {
            let _ = tx.send(SenderEvent::Progress(100));
            return JobOutcome::Completed;
        }

        let received = {
            let Some(mut c) = comm.try_lock() else {
                thread::sleep(Duration::from_millis(1));
                continue;
            };
            if !c.is_connected() {
                return JobOutcome::Stopped(t!("connection lost").to_string());
            }

            // Re-check the stop flag under the lock: E-stop sets it before taking the
            // lock to send the soft reset, so no line can follow the reset.
            while next < total && !stop_flag.load(Ordering::SeqCst) && window.can_send(&lines[next])
            {
                let cmd = format!("{}\n", lines[next]);
                if let Err(e) = send_all(&mut c, cmd.as_bytes()) {
                    return JobOutcome::Stopped(format!("{} {e}", t!("send failed:")));
                }
                window.on_sent(&lines[next]);
                next += 1;
            }

            if !stop_flag.load(Ordering::SeqCst)
                && last_query.is_none_or(|t| t.elapsed() >= STATUS_QUERY_INTERVAL)
            {
                // `?` is a realtime byte: GRBL does not store it in the RX buffer.
                if c.send(b"?").is_ok() {
                    last_query = Some(Instant::now());
                }
            }

            c.receive()
        };

        let data = match received {
            Ok(data) => data,
            Err(e) => return JobOutcome::Stopped(format!("{} {e}", t!("receive failed:"))),
        };
        if data.is_empty() {
            // `receive` already waits briefly for data; yield so UI actions get the lock.
            thread::yield_now();
            continue;
        }

        for line in splitter.push(&data) {
            match classify_response(&line) {
                GrblResponseLine::Ok => {
                    if window.on_ack() {
                        acked += 1;
                    }
                }
                GrblResponseLine::Error(code) => {
                    window.on_ack();
                    let _ = tx.send(SenderEvent::Response(line.clone()));
                    return JobOutcome::Stopped(format!("error:{code}"));
                }
                GrblResponseLine::Alarm(code) => {
                    let _ = tx.send(SenderEvent::Response(line.clone()));
                    return JobOutcome::Stopped(format!("ALARM:{code}"));
                }
                GrblResponseLine::Reset(banner) => {
                    let _ = tx.send(SenderEvent::Response(line.clone()));
                    return JobOutcome::Stopped(format!(
                        "{} ({banner})",
                        t!("controller was reset")
                    ));
                }
                GrblResponseLine::Status(status) => {
                    let _ = tx.send(SenderEvent::Status(status));
                }
                GrblResponseLine::Other(other) => {
                    let _ = tx.send(SenderEvent::Response(other));
                }
            }
        }

        let percent = ((acked as f64 / total as f64) * 100.0)
            .floor()
            .clamp(0.0, 100.0) as u32;
        if last_percent != Some(percent) {
            last_percent = Some(percent);
            let _ = tx.send(SenderEvent::Progress(percent));
        }
    }
}
