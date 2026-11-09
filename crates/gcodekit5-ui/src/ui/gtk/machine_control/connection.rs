//! # Connection handling and status polling
//!
//! Connects and disconnects the serial port for the Machine Control panel. The port is
//! opened on a worker thread so a slow device never blocks the GTK main loop; the result
//! is applied back on the main loop. While connected and no DirectSender job owns the
//! port, a 50 ms main-loop poller reads controller responses, drives the legacy
//! line-by-line job queue, tracks quick probes and requests status reports.

use super::grbl_stream::{
    classify_response, network_driver_for, parse_probe_report, GrblResponseLine,
    ResponseLineSplitter,
};
use super::status_report::QuickProbeState;
use super::*;

/// Interval of the idle status poller.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
/// The poller requests a status report every this many ticks (250 ms).
const POLL_STATUS_EVERY: u32 = 5;
/// Baud rate used when the active profile does not name one.
const DEFAULT_BAUD_RATE: u32 = 115200;

/// What a connect attempt will open.
pub(crate) struct ConnectionTarget {
    /// Parameters passed to the communicator.
    pub params: ConnectionParams,
    /// Text shown in the port area (serial port name or `host:port`).
    pub label: String,
    /// True for TCP/WebSocket targets taken from the active device profile.
    pub network: bool,
}

impl MachineControlView {
    /// Handles a click on the Connect/Disconnect button.
    pub(crate) fn on_connect_clicked(&self) {
        // Never block the main loop: a connect in progress holds the lock on its worker.
        let Some(is_connected) = self.communicator.try_lock().map(|c| c.is_connected()) else {
            return;
        };
        if is_connected {
            self.disconnect();
        } else {
            self.begin_connect();
        }
    }

    /// Sets the device profile manager whose active profile chooses the connection
    /// (serial port and baud rate, or TCP/WebSocket host and port).
    ///
    /// # Arguments
    /// * `manager` - The application's device manager.
    pub fn set_device_manager(&self, manager: std::sync::Arc<gcodekit5_devicedb::DeviceManager>) {
        *self.device_manager.borrow_mut() = Some(manager);
        self.refresh_connection_target();
    }

    /// Works out what Connect will open from the active device profile.
    ///
    /// A profile whose connection type is TCP/IP or WebSocket connects to its host and
    /// port; otherwise the port selected in the combo box is opened at the profile's
    /// baud rate (115200 without a profile).
    ///
    /// # Returns
    /// The target, or `None` when a serial connection is needed but no port is listed.
    pub(crate) fn connection_target(&self) -> Option<ConnectionTarget> {
        let profile = self
            .device_manager
            .borrow()
            .as_ref()
            .and_then(|dm| dm.get_active_profile());

        if let Some(profile) = profile.as_ref() {
            if let Some(driver) = network_driver_for(&profile.connection_type) {
                let host = profile.tcp_host.trim().to_string();
                let label = format!("{host}:{}", profile.tcp_port);
                let mut params = ConnectionParams {
                    driver,
                    port: host,
                    network_port: profile.tcp_port,
                    ..Default::default()
                };
                if profile.timeout_ms > 0 {
                    params.timeout_ms = profile.timeout_ms;
                }
                return Some(ConnectionTarget {
                    params,
                    label,
                    network: true,
                });
            }
        }

        if self.port_combo.active_id().as_deref() == Some("none") {
            return None;
        }
        let port_name = self.port_combo.active_text()?.to_string();
        let baud_rate = profile
            .as_ref()
            .map(|p| p.baud_rate)
            .filter(|&b| b > 0)
            .unwrap_or(DEFAULT_BAUD_RATE);
        Some(ConnectionTarget {
            params: ConnectionParams {
                driver: ConnectionDriver::Serial,
                port: port_name.clone(),
                baud_rate,
                ..Default::default()
            },
            label: port_name,
            network: false,
        })
    }

    /// Shows what Connect will open while disconnected.
    ///
    /// For a network profile the port area shows `host:port` and the serial port
    /// selector is disabled; for serial it shows the profile's baud rate. Does nothing
    /// while connected or connecting.
    pub fn refresh_connection_target(&self) {
        if !self.connect_btn.is_sensitive() {
            return; // connecting
        }
        let connected = self
            .communicator
            .try_lock()
            .is_none_or(|c| c.is_connected());
        if connected {
            return;
        }
        match self.connection_target() {
            Some(target) if target.network => {
                let text = format!(
                    "{} {} ({})",
                    t!("Host:"),
                    target.label,
                    target.params.driver
                );
                if self.conn_status_port.text() != text {
                    self.conn_status_port.set_text(&text);
                    self.conn_status_baud.set_text("");
                    self.port_combo.set_sensitive(false);
                    self.refresh_btn.set_sensitive(false);
                    self.port_combo.set_tooltip_text(Some(&t!(
                        "The active device profile connects over the network; change it in \
                         the Device Manager to use a serial port."
                    )));
                }
            }
            target => {
                let baud = target.map_or(DEFAULT_BAUD_RATE, |t| t.params.baud_rate);
                let baud_text = format!("{} {baud}", t!("Baud:"));
                if !self.port_combo.is_sensitive() || self.conn_status_baud.text() != baud_text {
                    self.conn_status_port.set_text(&t!("Port: -"));
                    self.conn_status_baud.set_text(&baud_text);
                    self.port_combo.set_sensitive(true);
                    self.refresh_btn.set_sensitive(true);
                    self.port_combo.set_tooltip_text(None);
                }
            }
        }
    }

    /// Enables or disables every connection-dependent control.
    ///
    /// Pause/Resume/Stop are then narrowed to the job state by `update_job_controls`.
    ///
    /// # Arguments
    /// * `enabled` - True when connected.
    pub(crate) fn set_connection_controls(&self, enabled: bool) {
        state::set_controls_enabled(
            &self.send_btn,
            &self.stop_btn,
            &self.pause_btn,
            &self.resume_btn,
            &self.home_btn,
            &self.unlock_btn,
            &self.wcs_btns,
            &self.x_zero_btn,
            &self.y_zero_btn,
            &self.z_zero_btn,
            &self.zero_all_btn,
            &self.goto_zero_btn,
            &self.step_combo,
            &self.jog_feed_entry,
            &self.jog_x_pos,
            &self.jog_x_neg,
            &self.jog_y_pos,
            &self.jog_y_neg,
            &self.jog_z_pos,
            &self.jog_z_neg,
            &self.estop_btn,
            self.a_zero_btn.as_ref(),
            self.b_zero_btn.as_ref(),
            self.c_zero_btn.as_ref(),
            self.jog_a_pos.as_ref(),
            self.jog_a_neg.as_ref(),
            self.jog_b_pos.as_ref(),
            self.jog_b_neg.as_ref(),
            self.jog_c_pos.as_ref(),
            self.jog_c_neg.as_ref(),
            enabled,
        );
        for btn in [
            &self.probe_z_btn,
            &self.probe_x_neg_btn,
            &self.probe_x_pos_btn,
            &self.probe_y_neg_btn,
            &self.probe_y_pos_btn,
        ]
        .into_iter()
        .flatten()
        {
            btn.set_sensitive(enabled);
        }
        self.update_job_controls();
    }

    /// Closes the port and resets the panel to the disconnected state.
    fn disconnect(&self) {
        // A running job would otherwise keep writing to a port that is going away.
        if let Some(sender) = self.direct_sender.lock().as_ref() {
            if sender.is_running() {
                sender.stop();
            }
        }
        self.stop_continuous_jog();

        let result = self.communicator.lock().disconnect();
        match result {
            Ok(()) => {
                set_button_icon_label(&self.connect_btn, "network-wired-symbolic", &t!("Connect"));
                self.connect_btn.remove_css_class("destructive-action");
                self.connect_btn.add_css_class("suggested-action");
                self.port_combo.set_sensitive(true);
                self.refresh_btn.set_sensitive(true);

                self.conn_status_port.set_text(&t!("Port: -"));
                self.conn_status_state.set_text(&t!("State: Disconnected"));
                self.state_label.set_text(&t!("Disconnected"));
                self.disabled_reason_label
                    .set_text(&t!("Connect to enable controls."));

                device_status::update_connection_status(false, None);
                *self.is_streaming.lock() = false;
                *self.is_paused.lock() = false;
                *self.job_start_time.lock() = None;
                self.quick_probe.set(QuickProbeState::Idle);
                self.last_wco.set(None);

                self.set_connection_controls(false);
                self.update_axis_visibility(device_status::get_active_num_axes());

                if let Some(sb) = self.status_bar.as_ref() {
                    sb.set_connected(false, "");
                    sb.set_progress(0.0, "", "");
                }
                self.log(&t!("Disconnected"));
                self.refresh_connection_target();
            }
            Err(e) => {
                self.log(&format!("{}: {}", t!("Error disconnecting"), e));
            }
        }
    }

    /// Starts opening the connection target on a worker thread.
    fn begin_connect(&self) {
        let Some(target) = self.connection_target() else {
            return;
        };
        let ConnectionTarget {
            params,
            label: port_name,
            network,
        } = target;

        self.connect_btn.set_sensitive(false);
        self.port_combo.set_sensitive(false);
        self.refresh_btn.set_sensitive(false);
        self.state_label.set_text(&t!("Connecting…"));
        self.conn_status_state.set_text(&t!("State: Connecting…"));
        self.conn_status_port
            .set_text(&format!("{} {}", t!("Port:"), port_name));
        let baud_text = if network {
            String::new()
        } else {
            format!("{} {}", t!("Baud:"), params.baud_rate)
        };
        self.conn_status_baud.set_text(&baud_text);
        tracing::info!(target = %port_name, driver = %params.driver, "Connecting");

        let communicator = self.communicator.clone();
        let handle = gtk4::gio::spawn_blocking(move || {
            communicator
                .lock()
                .connect(&params)
                .map_err(|e| e.to_string())
        });

        let view = self.clone();
        glib::spawn_future_local(async move {
            let result = match handle.await {
                Ok(result) => result,
                Err(_) => Err(t!("the connection thread failed").to_string()),
            };
            view.finish_connect(&port_name, &baud_text, result);
        });
    }

    /// Applies the result of a connection attempt (runs on the main loop).
    ///
    /// # Arguments
    /// * `port_name` - The port (or `host:port`) that was opened.
    /// * `baud_text` - Baud rate label text (empty for network connections).
    /// * `result` - The outcome of `connect`.
    fn finish_connect(&self, port_name: &str, baud_text: &str, result: Result<(), String>) {
        if let Err(e) = result {
            self.connect_btn.set_sensitive(true);
            self.port_combo.set_sensitive(true);
            self.refresh_btn.set_sensitive(true);
            self.conn_status_port.set_text(&t!("Port: -"));
            self.refresh_connection_target();
            self.state_label.set_text(&t!("Disconnected"));
            self.conn_status_state.set_text(&t!("State: Disconnected"));
            self.disabled_reason_label
                .set_text(&t!("Connect to enable controls."));
            tracing::warn!(port = port_name, error = %e, "Serial connection failed");
            self.log(&format!("{}: {}", t!("Error connecting"), e));
            return;
        }

        set_button_icon_label(
            &self.connect_btn,
            "network-offline-symbolic",
            &t!("Disconnect"),
        );
        self.connect_btn.remove_css_class("suggested-action");
        self.connect_btn.add_css_class("destructive-action");
        self.connect_btn.set_sensitive(true);
        self.port_combo.set_sensitive(false);
        self.refresh_btn.set_sensitive(false);

        self.conn_status_port
            .set_text(&format!("{} {}", t!("Port:"), port_name));
        self.conn_status_baud.set_text(baud_text);
        self.conn_status_state.set_text(&t!("State: Connected"));
        self.state_label.set_text(&t!("Connected"));
        self.disabled_reason_label.set_text("");

        device_status::update_connection_status(true, Some(port_name.to_string()));
        if let Some(sb) = self.status_bar.as_ref() {
            sb.set_connected(true, port_name);
        }
        self.log(&format!("{} {}", t!("Connected to"), port_name));

        self.set_connection_controls(true);
        self.update_axis_visibility(device_status::get_active_num_axes());
        // Unlock is only enabled once an ALARM state is reported.
        self.unlock_btn.set_sensitive(false);

        // Trigger the startup banner (some firmwares only emit it after reset), then query
        // firmware and settings. Some controllers ignore commands sent right after Ctrl-X.
        {
            let mut comm = self.communicator.lock();
            self.log("> 0x18 (Reset)");
            let _ = comm.send(&[0x18]);
        }
        {
            let view = self.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(250), move || {
                if let Some(mut comm) = view.communicator.try_lock() {
                    if comm.is_connected() {
                        view.log("> $I");
                        let _ = comm.send_command("$I");
                    }
                }
            });
        }
        {
            let view = self.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                if let Some(mut comm) = view.communicator.try_lock() {
                    if comm.is_connected() {
                        view.log("> $$");
                        let _ = comm.send_command("$$");
                        // Status report mask: WPos | Buf | Ln | FS | Ov  ($10=47).
                        view.log("> $10=47");
                        let _ = comm.send_command("$10=47");
                    }
                }
            });
        }

        self.start_status_poller();
    }

    /// Starts the main-loop poller that reads responses while no DirectSender job runs.
    fn start_status_poller(&self) {
        let generation = self.poll_generation.get().wrapping_add(1);
        self.poll_generation.set(generation);

        let view = self.clone();
        let mut query_counter = 0u32;
        let mut splitter = ResponseLineSplitter::new();
        let mut firmware_detected = false;

        glib::timeout_add_local(POLL_INTERVAL, move || {
            if view.poll_generation.get() != generation {
                return glib::ControlFlow::Break;
            }
            query_counter = query_counter.wrapping_add(1);

            // Yield the port completely while DirectSender streams: it reads the replies
            // itself (its byte count needs every `ok`) and forwards status reports. The
            // legacy queue streamer (`start_job`) relies on this loop, so it keeps running.
            let direct_sender_running = view
                .direct_sender
                .lock()
                .as_ref()
                .is_some_and(|s| s.is_running());
            if direct_sender_running {
                return glib::ControlFlow::Continue;
            }

            let Some(mut comm) = view.communicator.try_lock() else {
                return glib::ControlFlow::Continue;
            };
            if !comm.is_connected() {
                return glib::ControlFlow::Break;
            }

            if let Ok(bytes) = comm.receive() {
                if !bytes.is_empty() {
                    for line in splitter.push(&bytes) {
                        view.handle_poll_line(&line, &mut comm, &mut firmware_detected);
                    }
                }
            }

            if query_counter.is_multiple_of(POLL_STATUS_EVERY) {
                let _ = comm.send(b"?");
            }
            glib::ControlFlow::Continue
        });
    }

    /// Processes one complete response line read by the status poller.
    ///
    /// # Arguments
    /// * `line` - The trimmed response line.
    /// * `comm` - The locked communicator (used to send the next queued job line).
    /// * `firmware_detected` - Set once the firmware banner/version was recognised.
    fn handle_poll_line(
        &self,
        line: &str,
        comm: &mut SerialCommunicator,
        firmware_detected: &mut bool,
    ) {
        if line.contains("Program End") {
            let elapsed = self
                .job_start_time
                .lock()
                .map_or(0.0, |start| start.elapsed().as_secs_f64());
            let mins = (elapsed / 60.0).floor() as u32;
            let secs = (elapsed % 60.0) as u32;
            self.log(&format!(
                "{} {}:{:02}",
                t!("✅ Work completed in:"),
                mins,
                secs
            ));
            *self.job_start_time.lock() = None;
            if let Some(sb) = self.status_bar.as_ref() {
                sb.set_progress(0.0, "", "");
            }
        }

        if !*firmware_detected && (line.starts_with("[VER:") || line.contains("Grbl")) {
            use gcodekit5_communication::firmware::firmware_detector::FirmwareDetector;
            if let Ok(detection) = FirmwareDetector::parse_response(line) {
                let fw_type = format!("{:?}", detection.firmware_type);
                device_status::update_firmware_info(fw_type, detection.version_string, None);
                *firmware_detected = true;
            }
        }

        // Capture `$$` settings lines for Device Config / CAM.
        if line.starts_with('$') && line.contains('=') {
            if let Some((n, v)) =
                gcodekit5_communication::firmware::grbl::settings::SettingsManager::parse_setting_line(line)
            {
                device_status::update_grbl_setting(n, v);
            }
        }

        let kind = classify_response(line);
        let is_ack = kind == GrblResponseLine::Ok;
        let is_error = matches!(kind, GrblResponseLine::Error(_));

        let probe_reply = self.track_quick_probe(line, &kind, comm);

        // Replies to continuous-jog increments are counted there; a rejected increment
        // (e.g. soft limit) ends the jog and is logged, not shown as a dialog.
        let jog_reply = (is_ack || is_error) && self.continuous_jog.lock().on_reply(is_error);

        let line_for_log = Self::decode_response_line(line);
        if is_error && !*self.is_streaming.lock() && !jog_reply && !probe_reply {
            self.show_controller_error(&line_for_log);
        }

        // Log everything except status reports and plain acks.
        if !matches!(kind, GrblResponseLine::Status(_)) && !is_ack {
            self.log(&line_for_log);
        }

        if (is_ack || is_error) && !jog_reply {
            self.advance_legacy_queue(comm);
        }

        if let GrblResponseLine::Status(status) = &kind {
            self.apply_status_report(status);
            self.restore_absolute_mode_after_probe(comm);
        }
    }

    /// Sends the next line of a `start_job` (line-by-line) job after an ack.
    fn advance_legacy_queue(&self, comm: &mut SerialCommunicator) {
        *self.waiting_for_ack.lock() = false;
        if !*self.is_streaming.lock() || *self.is_paused.lock() {
            return;
        }

        let mut queue = self.send_queue.lock();
        let total = *self.total_lines.lock();
        let remaining = queue.len();
        let sent = total.saturating_sub(remaining);
        let progress = if total > 0 {
            (sent as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        Self::update_progress(
            &self.status_bar,
            &self.job_start_time,
            progress,
            Some(sent),
            Some(total),
            Some(remaining),
        );

        if let Some(next_cmd) = queue.pop_front() {
            drop(queue);
            self.log(&format!("> {next_cmd}"));
            let _ = comm.send_command(&next_cmd);
            *self.waiting_for_ack.lock() = true;
        } else {
            drop(queue);
            // Done streaming; the job timer ends once the machine reports Idle.
            *self.is_streaming.lock() = false;
            *self.is_paused.lock() = false;
            self.log(&t!("Streaming Completed."));
            self.update_job_controls();
        }
    }

    /// Follows a quick probe started from the panel.
    ///
    /// # Returns
    /// True if `line` was the reply to the probe line, so an `error:` must not open the
    /// generic error dialog.
    fn track_quick_probe(
        &self,
        line: &str,
        kind: &GrblResponseLine,
        comm: &mut SerialCommunicator,
    ) -> bool {
        if self.quick_probe.get() != QuickProbeState::AwaitingResult {
            return false;
        }
        if let Some(success) = parse_probe_report(line) {
            if success {
                self.log(&format!("{} {line}", t!("Probe contact:")));
                self.finish_probe_success(comm);
            } else {
                self.quick_probe.set(QuickProbeState::NeedsAbsoluteRestore);
                self.log(&t!(
                    "Probe failed: no contact within the probe distance. Unlock ($X) to \
                     continue; absolute mode (G90) is restored afterwards."
                ));
                tracing::warn!(report = line, "Quick probe failed");
            }
            return false;
        }
        match kind {
            GrblResponseLine::Alarm(_) => {
                self.quick_probe.set(QuickProbeState::NeedsAbsoluteRestore);
                self.log(&format!(
                    "{} {}",
                    t!("Probe failed:"),
                    Self::decode_response_line(line)
                ));
                tracing::warn!(alarm = line, "Quick probe raised an alarm");
                false
            }
            GrblResponseLine::Error(_) => {
                // The probe line itself was rejected, so its G91 never took effect.
                self.quick_probe.set(QuickProbeState::Idle);
                self.log(&format!(
                    "{} {}",
                    t!("Probe rejected by the controller:"),
                    Self::decode_response_line(line)
                ));
                tracing::warn!(error = line, "Quick probe rejected");
                true
            }
            // Firmware without probe reports: an `ok` means the probe completed.
            GrblResponseLine::Ok => {
                self.finish_probe_success(comm);
                false
            }
            _ => false,
        }
    }

    /// Restores absolute distance mode after a successful quick probe.
    fn finish_probe_success(&self, comm: &mut SerialCommunicator) {
        self.quick_probe.set(QuickProbeState::Idle);
        self.log("> G90");
        let _ = comm.send_command("G90");
    }

    /// Restores `G90` after a failed probe once the controller is out of ALARM.
    fn restore_absolute_mode_after_probe(&self, comm: &mut SerialCommunicator) {
        if self.quick_probe.get() != QuickProbeState::NeedsAbsoluteRestore {
            return;
        }
        if device_status::get_status().state != "Idle" {
            return;
        }
        self.quick_probe.set(QuickProbeState::Idle);
        self.log(&format!(
            "> G90 ({})",
            t!("restore absolute mode after failed probe")
        ));
        let _ = comm.send_command("G90");
    }

    /// Shows a modal "Controller error" dialog.
    pub(crate) fn show_controller_error(&self, secondary: &str) {
        let dialog = gtk4::MessageDialog::builder()
            .message_type(gtk4::MessageType::Error)
            .buttons(gtk4::ButtonsType::Ok)
            .text(t!("Controller error"))
            .secondary_text(secondary)
            .build();
        dialog.connect_response(|d, _| d.close());
        if let Some(root) = self.widget.root() {
            if let Ok(win) = root.downcast::<gtk4::Window>() {
                dialog.set_transient_for(Some(&win));
                dialog.set_modal(true);
            }
        }
        dialog.show();
    }

    /// Appends a line to the device console, if there is one.
    pub(crate) fn log(&self, msg: &str) {
        if let Some(c) = self.device_console.as_ref() {
            c.append_log(&format!("{msg}\n"));
        }
    }
}
