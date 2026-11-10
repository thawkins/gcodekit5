//! # Status report display
//!
//! Applies GRBL real-time status reports (`<Idle|MPos:...|...>`) to the Machine Control
//! panel: state label, Unlock button, DROs (including rotary axes), buffer, feed/spindle
//! and override values, the status bar and the global `device_status`.
//!
//! The same code serves both the idle status poller and the DirectSender worker, which
//! forwards the reports it reads while a job streams.

use super::grbl_stream::reported_axis_count;
use super::*;

/// State of a quick probe (Probe Z / edge find) started from the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuickProbeState {
    /// No quick probe in progress.
    #[default]
    Idle,
    /// A `G91 G38.2` probe line was sent; waiting for `[PRB:...]` / `ALARM:` / `error:`.
    AwaitingResult,
    /// The probe failed and left the controller in `G91`; `G90` must be restored once
    /// the alarm is cleared.
    NeedsAbsoluteRestore,
}

impl MachineControlView {
    /// Applies one `<...>` status report to the panel and to `device_status`.
    ///
    /// # Arguments
    /// * `line` - A complete status report including the angle brackets.
    pub(crate) fn apply_status_report(&self, line: &str) {
        if let Some(state) = StatusParser::parse_machine_state(line) {
            self.apply_machine_state(&state);
        }

        let full_status = StatusParser::parse_full(line);
        let units = *self.current_units.lock();
        let unit_label = get_unit_label(units);

        if let Some(mpos) = full_status.mpos {
            self.world_x.set_text(&format!(
                "MX: {} {}",
                format_length(mpos.x as f32, units),
                unit_label
            ));
            self.world_y.set_text(&format!(
                "MY: {} {}",
                format_length(mpos.y as f32, units),
                unit_label
            ));
            self.world_z.set_text(&format!(
                "MZ: {} {}",
                format_length(mpos.z as f32, units),
                unit_label
            ));
            for (label, value, name) in [
                (&self.world_a, mpos.a, "MA"),
                (&self.world_b, mpos.b, "MB"),
                (&self.world_c, mpos.c, "MC"),
            ] {
                if let (Some(label), Some(v)) = (label.as_ref(), value) {
                    label.set_text(&format!("{name}: {v:.2}°"));
                }
            }

            device_status::update_machine_position(mpos);
            self.note_reported_axes(reported_axis_count(&mpos));

            if let Some(sb) = self.status_bar.as_ref() {
                sb.set_position(
                    mpos.x as f32,
                    mpos.y as f32,
                    mpos.z as f32,
                    mpos.a.unwrap_or(0.0) as f32,
                    mpos.b.unwrap_or(0.0) as f32,
                    mpos.c.unwrap_or(0.0) as f32,
                    units,
                );
            }
        }

        // Cache the last Work Coordinate Offset: GRBL only includes WCO in some reports.
        if let Some(wco) = full_status.wco {
            self.last_wco.set(Some(wco));
            device_status::update_work_coordinate_offset(wco);
        }

        // WPos is either reported directly, derived by parse_full, or derived here from
        // MPos and the cached WCO.
        let wpos = full_status
            .wpos
            .or_else(|| match (full_status.mpos, self.last_wco.get()) {
                (Some(mpos), Some(wco)) => Some(StatusParser::wpos_from_mpos_wco(mpos, wco)),
                _ => None,
            });
        if let Some(wpos) = wpos {
            self.x_dro.set_text(&format!(
                "{} {}",
                format_length(wpos.x as f32, units),
                unit_label
            ));
            self.y_dro.set_text(&format!(
                "{} {}",
                format_length(wpos.y as f32, units),
                unit_label
            ));
            self.z_dro.set_text(&format!(
                "{} {}",
                format_length(wpos.z as f32, units),
                unit_label
            ));
            for (label, value) in [
                (&self.a_dro, wpos.a),
                (&self.b_dro, wpos.b),
                (&self.c_dro, wpos.c),
            ] {
                if let (Some(label), Some(v)) = (label.as_ref(), value) {
                    label.set_text(&format!("{v:.2}°"));
                }
            }

            device_status::update_work_position(wpos);

            // The visualizer shows the tool in work coordinates.
            if let Some(vis) = self.visualizer.as_ref() {
                vis.set_current_position(wpos.x as f32, wpos.y as f32, wpos.z as f32);
            }
        }

        if let Some(buffer) = full_status.buffer {
            self.state_buffer_label.set_text(&format!(
                "{} {}/{}",
                t!("Buffer:"),
                buffer.plan,
                buffer.rx
            ));
            device_status::update_buffer_state(buffer);
        }

        let (feed_ov, spindle_ov) = match full_status.overrides {
            Some(ov) => {
                *self.last_overrides.lock() = ov;
                (ov.feed, ov.spindle)
            }
            None => {
                let last = self.last_overrides.lock();
                (last.feed, last.spindle)
            }
        };

        // Show the commanded feed/spindle when known (the reported values fluctuate).
        let (commanded_feed, commanded_spindle) = {
            let status = device_status::DEVICE_STATUS.read();
            (status.commanded_feed_rate, status.commanded_spindle_speed)
        };
        let display_feed = commanded_feed.or(full_status.feed_rate.map(|f| f as f32));
        let display_spindle = commanded_spindle.or(full_status.spindle_speed.map(|s| s as f32));
        let feed_units = *self.current_feed_units.lock();

        if let Some(feed_rate) = display_feed {
            let feed = format_feed_rate(feed_rate, feed_units);
            self.state_feed_label
                .set_text(&format!("{} {} {}", t!("Feed:"), feed, feed_units));

            let adjusted = format_feed_rate(feed_rate * (feed_ov as f32 / 100.0), feed_units);
            let mut text = format!("{adjusted} ({feed_ov}%)");
            if feed_ov >= 200 {
                text.push_str(" MAX");
            }
            if feed_ov <= 10 {
                text.push_str(" MIN");
            }
            self.feed_value.set_text(&text);
            if feed_ov >= 200 || feed_ov <= 10 {
                self.feed_value.add_css_class("error");
            } else {
                self.feed_value.remove_css_class("error");
            }
        }

        if let Some(spindle_speed) = display_spindle {
            self.state_spindle_label
                .set_text(&format!("{} {} S", t!("Spindle:"), spindle_speed));

            let adjusted = spindle_speed * (spindle_ov as f32 / 100.0);
            let mut text = format!("{adjusted:.0} S ({spindle_ov}%)");
            if spindle_ov >= 200 {
                text.push_str(" MAX");
            }
            if spindle_ov <= 50 {
                text.push_str(" MIN");
            }
            self.spindle_value.set_text(&text);
            if spindle_ov >= 200 || spindle_ov <= 50 {
                self.spindle_value.add_css_class("error");
            } else {
                self.spindle_value.remove_css_class("error");
            }
        }

        if let (Some(feed_rate), Some(spindle_speed)) = (display_feed, display_spindle) {
            if let Some(sb) = self.status_bar.as_ref() {
                sb.set_feed_spindle(feed_rate as f64, spindle_speed as u32, feed_units);
            }
            device_status::update_feed_spindle_state(FeedSpindleState {
                feed_rate: feed_rate as f64,
                spindle_speed: spindle_speed as u32,
            });
        }
    }

    /// Updates the state label, Unlock button and job timer from a machine state.
    fn apply_machine_state(&self, state: &str) {
        self.conn_status_state
            .set_text(&format!("{} {}", t!("State:"), state));
        self.state_label.set_text(state);

        for cls in ["state-idle", "state-run", "state-hold", "state-alarm"] {
            self.state_label.remove_css_class(cls);
        }
        let low = state.to_lowercase();
        let is_alarm = low.starts_with("alarm");
        let cls = if is_alarm {
            "state-alarm"
        } else if low.starts_with("run") {
            "state-run"
        } else if low.starts_with("hold") {
            "state-hold"
        } else {
            "state-idle"
        };
        self.state_label.add_css_class(cls);

        device_status::update_state(state.to_string());
        if let Some(sb) = self.status_bar.as_ref() {
            sb.set_state(state);
        }

        // Once the machine is idle after a job, the job (and its timer) is over.
        if state == "Idle" && !*self.is_streaming.lock() {
            let mut start_time = self.job_start_time.lock();
            if start_time.is_some() {
                *start_time = None;
                if let Some(sb) = self.status_bar.as_ref() {
                    sb.set_progress(0.0, "", "");
                }
            }
        }

        // A machine that is running again has been resumed, whether by the
        // Resume button, the controller's cycle-start button or a console `~`.
        if low.starts_with("run") || low.starts_with("jog") {
            *self.is_paused.lock() = false;
        }

        // Unlock is only useful in ALARM.
        self.unlock_btn.set_sensitive(is_alarm);
        if is_alarm {
            self.disabled_reason_label
                .set_text(&t!("ALARM: Unlock required."));
        } else {
            self.disabled_reason_label.set_text("");
        }

        self.update_job_controls();
    }

    /// Shows rotary axes that a status report carries but the panel does not show yet.
    fn note_reported_axes(&self, reported: u8) {
        if reported > self.axis_count.get() {
            self.update_axis_visibility(reported);
        }
    }

    /// Returns true while a job streams or its buffered moves are still executing.
    pub(crate) fn job_active(&self) -> bool {
        *self.is_streaming.lock() || self.job_start_time.lock().is_some()
    }

    /// Enables Pause/Stop only while a job is active and Resume only while it is paused.
    pub(crate) fn update_job_controls(&self) {
        let (connected, held) = {
            let status = device_status::DEVICE_STATUS.read();
            (
                status.is_connected,
                status.state.to_ascii_lowercase().starts_with("hold"),
            )
        };
        device_status::set_job_active(self.job_active());
        let active = connected && self.job_active();
        let paused = *self.is_paused.lock() || held;
        self.stop_btn.set_sensitive(active);
        self.pause_btn.set_sensitive(active && !paused);
        self.resume_btn.set_sensitive(active && paused);
    }

    /// Appends a decoded description to an `error:N` / `ALARM:N` line.
    ///
    /// # Arguments
    /// * `line` - A controller response line.
    ///
    /// # Returns
    /// `"error:N - description"` for decodable errors and alarms, otherwise the line.
    pub(crate) fn decode_response_line(line: &str) -> String {
        let lower = line.trim().to_ascii_lowercase();
        let (rest, is_alarm) = if let Some(rest) = lower.strip_prefix("error:") {
            (rest, false)
        } else if let Some(rest) = lower.strip_prefix("alarm:") {
            (rest, true)
        } else {
            return line.to_string();
        };
        let digits: String = rest
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        match digits.parse::<u8>() {
            Ok(code) if is_alarm => format!(
                "{line} - {}",
                gcodekit5_communication::firmware::grbl::decode_alarm(code)
            ),
            Ok(code) => format!(
                "{line} - {}",
                gcodekit5_communication::firmware::grbl::decode_error(code)
            ),
            Err(_) => line.to_string(),
        }
    }
}
