//! # Repetier-Specific Stream Sender with Block Control & Auto-Rewind
//!
//! Controls the streaming thread for 3D Printers using the Repetier protocol,
//! preventing buffer overflow with a safe 10-line window and processing
//! Resend requests in real-time by rolling back the file pointer.

use crate::communication::Communicator;
use crate::firmware::repetier::processor::RepetierEvent;
use crate::firmware::Sender;
use crate::SerialCommunicator;
use gcodekit5_core::ThreadSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender as MpscSender, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// How long one line may keep failing to send before the job is stopped.
/// A full port clears within milliseconds; a failure that lasts this long
/// means the controller is gone.
const SEND_RETRY_LIMIT: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct RepetierSender {
    tx_control: MpscSender<SenderCommand>,
    communicator: ThreadSafe<SerialCommunicator>,
    is_streaming: ThreadSafe<bool>,
    /// Pen-plotter mode: rewrite laser on/off as pen down/up Z moves.
    pen_mode: Arc<AtomicBool>,
}

enum SenderCommand {
    SendGcode(String),
    Pause,
    Resume,
    Stop,
}

impl RepetierSender {
    /// Starts the streaming thread and returns the sender plus the channel
    /// it reports on (`> ...` log lines, `*NN%` progress, `Completed`).
    ///
    /// The thread exits once every clone of the returned sender is dropped.
    pub fn new(
        communicator: ThreadSafe<SerialCommunicator>,
        is_streaming: ThreadSafe<bool>,
        is_paused: ThreadSafe<bool>,
        _waiting_for_ack: ThreadSafe<bool>,
    ) -> (Self, Receiver<String>) {
        let (tx_control, rx_control) = channel();
        let (tx_ui, rx_ui) = channel();
        let tx_ui_thread = tx_ui.clone();

        // Clones for the struct. The `move` closure below takes ownership of
        // the originals, so we save copies here.
        let communicator_for_struct = communicator.clone();
        let is_streaming_for_struct = is_streaming.clone();
        let pen_mode = Arc::new(AtomicBool::new(false));
        let pen_mode_for_struct = Arc::clone(&pen_mode);

        thread::spawn(move || {
            let mut file_lines: Vec<String> = Vec::new();
            let mut current_idx: usize = 0;
            let mut commands_in_flight: usize = 0;
            let mut local_streaming = false;
            let mut local_paused = false;
            let mut send_failing_since: Option<Instant> = None;

            loop {
                // Process control commands from the UI
                loop {
                    let cmd = match rx_control.try_recv() {
                        Ok(cmd) => cmd,
                        Err(TryRecvError::Empty) => break,
                        // Every sender handle is gone: nothing can start or
                        // stop a job any more, so the thread ends.
                        Err(TryRecvError::Disconnected) => return,
                    };
                    match cmd {
                        SenderCommand::SendGcode(content) => {
                            let content = if pen_mode.load(Ordering::Relaxed) {
                                apply_pen_mode(&content)
                            } else {
                                content
                            };
                            file_lines = content
                                .lines()
                                .map(|s| s.trim())
                                .filter(|s| {
                                    let s = s.trim();
                                    !s.is_empty() && !s.starts_with(';') && !s.starts_with('(')
                                })
                                .map(|s| s.to_string())
                                .collect();
                            tracing::debug!(
                                "Repetier sender: loaded {} command lines",
                                file_lines.len()
                            );

                            // Clear the flow control flags and any acks or
                            // resends left over from before this job.
                            {
                                let mut comm = communicator.lock();
                                comm.repetier_reset_flow_control();
                                let stale = comm.take_repetier_events();
                                if !stale.is_empty() {
                                    tracing::debug!(
                                        "Repetier sender: discarded {} stale events",
                                        stale.len()
                                    );
                                }
                            }
                            send_failing_since = None;

                            current_idx = 0;
                            commands_in_flight = 0;
                            local_streaming = true;
                            local_paused = false;
                            *is_streaming.lock() = true;
                            *is_paused.lock() = false;
                        }
                        SenderCommand::Pause => {
                            local_paused = true;
                            *is_paused.lock() = true;
                        }
                        SenderCommand::Resume => {
                            local_paused = false;
                            *is_paused.lock() = false;
                        }
                        SenderCommand::Stop => {
                            local_streaming = false;
                            *is_streaming.lock() = false;
                            let _ = tx_ui_thread.send("Completed".to_string());
                        }
                    }
                }

                if local_streaming && !local_paused {
                    // Check the communicator's flow control status
                    let (flow_paused, flow_stopped, connected) =
                        if let Some(comm) = communicator.try_lock() {
                            (
                                comm.repetier_is_paused(),
                                comm.repetier_is_stopped(),
                                comm.is_connected(),
                            )
                        } else {
                            (false, false, true)
                        };

                    if !connected {
                        fail_job(
                            &tx_ui_thread,
                            &is_streaming,
                            "controller not connected",
                            current_idx,
                            file_lines.len(),
                        );
                        local_streaming = false;
                        continue;
                    }

                    if flow_stopped {
                        tracing::debug!("Repetier sender: stop requested, ending stream");
                        local_streaming = false;
                        *is_streaming.lock() = false;
                        let _ = tx_ui_thread.send("Completed".to_string());
                    } else if flow_paused {
                    } else if commands_in_flight < 1 && current_idx < file_lines.len() {
                        let raw_line = file_lines[current_idx].clone();

                        if let Some(mut comm) = communicator.try_lock() {
                            let before = comm.repetier_line_counter();
                            match comm.send(raw_line.as_bytes()) {
                                Ok(written) if written > 0 => {
                                    let after = comm.repetier_line_counter();
                                    let _ = tx_ui_thread.send(format!("> {}", raw_line.trim()));
                                    // One ack is due per numbered line sent.
                                    commands_in_flight +=
                                        (after.saturating_sub(before) as usize).max(1);
                                    current_idx += 1;
                                    send_failing_since = None;
                                }
                                Ok(_) => {
                                    let _ = tx_ui_thread.send(format!(
                                        "> Skipped (no Repetier equivalent): {}",
                                        raw_line.trim()
                                    ));
                                    current_idx += 1;
                                }
                                Err(e) => {
                                    // Retry briefly (a full port clears quickly);
                                    // a failure that persists ends the job.
                                    let since = *send_failing_since.get_or_insert_with(|| {
                                        tracing::debug!(
                                            "Repetier sender: retrying line {} after: {}",
                                            current_idx,
                                            e
                                        );
                                        Instant::now()
                                    });
                                    if since.elapsed() >= SEND_RETRY_LIMIT {
                                        fail_job(
                                            &tx_ui_thread,
                                            &is_streaming,
                                            &format!("send failed: {e}"),
                                            current_idx,
                                            file_lines.len(),
                                        );
                                        local_streaming = false;
                                        continue;
                                    }
                                }
                            }
                        }

                        let progress = (current_idx as f64 / file_lines.len() as f64) * 100.0;
                        let _ = tx_ui_thread.send(format!("*{:.1}%", progress));
                    }

                    // Listening for Confirmations and Handling OKs
                    if let Some(mut comm) = communicator.try_lock() {
                        let events = comm.take_repetier_events();

                        for event in events {
                            match event {
                                RepetierEvent::Ack => {
                                    commands_in_flight = commands_in_flight.saturating_sub(1);
                                    tracing::trace!(
                                        "Repetier sender: ack, in flight {}",
                                        commands_in_flight
                                    );
                                }
                                RepetierEvent::Resend(target_line) => {
                                    // Replay the exact numbered lines from the
                                    // requested one, whoever sent them (job or
                                    // console); each replayed line needs an ack.
                                    match comm.repetier_resend(target_line) {
                                        Ok(count) => {
                                            commands_in_flight = count;
                                            let _ = tx_ui_thread.send(format!(
                                                "> Resend requested for N{}: re-sent {} line(s)",
                                                target_line, count
                                            ));
                                        }
                                        Err(e) => {
                                            fail_job(
                                                &tx_ui_thread,
                                                &is_streaming,
                                                &format!("resend of N{target_line} failed: {e}"),
                                                current_idx,
                                                file_lines.len(),
                                            );
                                            local_streaming = false;
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        if !local_streaming {
                            continue;
                        }

                        // Drain the byte buffer to avoid accumulation. A read
                        // error means the controller can no longer answer.
                        if let Err(e) = comm.receive() {
                            fail_job(
                                &tx_ui_thread,
                                &is_streaming,
                                &format!("receive failed: {e}"),
                                current_idx,
                                file_lines.len(),
                            );
                            local_streaming = false;
                            continue;
                        }
                    }

                    if current_idx >= file_lines.len() && commands_in_flight == 0 {
                        local_streaming = false;
                        *is_streaming.lock() = false;
                        let _ = tx_ui_thread.send("Completed".to_string());
                    }
                }

                thread::sleep(Duration::from_millis(1));
            }
        });

        (
            Self {
                tx_control,
                communicator: communicator_for_struct,
                is_streaming: is_streaming_for_struct,
                pen_mode: pen_mode_for_struct,
            },
            rx_ui,
        )
    }
}

impl RepetierSender {
    /// Turns pen-plotter mode on or off for jobs sent after this call.
    ///
    /// When on, laser on (`M3`/`M4`) becomes a pen-down `G0 Z0` and laser
    /// off (`M5`) a pen-up `G0 Z5`. When off (the default) the program is
    /// sent as written. This is a per-machine setting (`pen_mode` in the
    /// device profile).
    pub fn set_pen_mode(&self, enabled: bool) {
        self.pen_mode.store(enabled, Ordering::Relaxed);
    }
}

impl Sender for RepetierSender {
    fn send_gcode(&self, content: &str) {
        let _ = self
            .tx_control
            .send(SenderCommand::SendGcode(content.to_string()));
    }

    fn pause(&self) {
        let _ = self.tx_control.send(SenderCommand::Pause);
    }

    fn resume(&self) {
        let _ = self.tx_control.send(SenderCommand::Resume);
    }

    fn stop(&self) {
        let _ = self.tx_control.send(SenderCommand::Stop);
    }

    fn unlock(&self) {
        // Repetier/Marlin do not have a `$X` equivalent.
        // Some Marlin variants accept M999, but it is not a drop-in
        // replacement for GRBL's unlock and could reset the printer
        // unexpectedly, so we treat it as a no-op.
        tracing::debug!("unlock() called on RepetierSender (no-op)");
    }

    fn is_streaming(&self) -> bool {
        *self.is_streaming.lock()
    }
}

/// Reports a job-ending error to the UI and marks streaming as finished.
///
/// The caller clears its own `local_streaming` flag.
fn fail_job(
    tx_ui: &MpscSender<String>,
    is_streaming: &ThreadSafe<bool>,
    reason: &str,
    line: usize,
    total: usize,
) {
    tracing::warn!("Repetier sender: {reason}; job stopped at line {line} of {total}");
    let _ = tx_ui.send(format!(
        "> Error: {reason}; job stopped at line {line} of {total}"
    ));
    *is_streaming.lock() = false;
    let _ = tx_ui.send("Completed".to_string());
}

/// Replace laser on/off commands with a Z move, for pen-plotter use.
///
/// Used only when the machine's profile turns pen mode on
/// ([`RepetierSender::set_pen_mode`]). The laser commands become Z
/// moves so the pen is lowered when the "laser" would be on and
/// lifted when it would be off.
fn apply_pen_mode(gcode: &str) -> String {
    let mut out = String::with_capacity(gcode.len() + 256);
    // Whether the tool is currently up (laser off). Start with
    // "false" so the initial M5 (which is always present in laser
    // G-code, to guarantee the laser starts off) triggers a lift.
    let mut tool_is_up = false;

    for line in gcode.lines() {
        let upper = line.trim().to_ascii_uppercase();
        let first_token = upper.split_whitespace().next().unwrap_or("");

        match first_token {
            "M3" | "M4" => {
                if !tool_is_up {
                    // Already down, drop the redundant command.
                    continue;
                }
                // Lower the tool to the contact position (Z0).
                out.push_str("G0 Z0\n");
                tool_is_up = false;
            }
            "M5" => {
                if tool_is_up {
                    // Already up, drop the redundant command.
                    continue;
                }
                // Lift the tool clear of the surface.
                out.push_str("G0 Z5\n");
                tool_is_up = true;
            }
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    out
}
