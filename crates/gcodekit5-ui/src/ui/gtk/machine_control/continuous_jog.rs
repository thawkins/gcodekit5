//! Continuous (hold-to-jog) jogging for the Machine Control panel.
//!
//! A single long `$J=` move toward an arbitrary far target is rejected outright by
//! GRBL (error 15) when soft limits are enabled and the target lies outside the
//! machine's travel. Following GRBL's recommended method, a held jog is streamed as
//! short increments instead: each increment takes about one tick to execute, and only
//! a few are sent ahead of the controller's `ok` acknowledgements. Releasing the
//! button or key sends jog-cancel (0x85), which stops motion and flushes any queued
//! increments. At a soft limit GRBL rejects the next increment, which ends the jog
//! within one increment of the boundary.

use super::*;
use std::time::Duration;

/// Interval between continuous-jog increments.
const CONTINUOUS_JOG_TICK: Duration = Duration::from_millis(50);
/// Maximum increments sent but not yet acknowledged with `ok`/`error`.
const CONTINUOUS_JOG_LOOKAHEAD: u32 = 3;

/// Live state of a continuous jog, shared with the status poller.
#[derive(Debug, Default)]
pub struct ContinuousJogState {
    /// Axis and signed increment (mm) and feed (mm/min) of the jog in progress.
    pub active: Option<(char, f32, f32)>,
    /// Increments sent but not yet answered by `ok` or `error:`.
    pub in_flight: u32,
    /// Set by the status poller when the controller rejects an increment.
    pub rejected: bool,
    /// Bumped on every start/stop so a stale timer ends itself.
    pub generation: u64,
}

impl ContinuousJogState {
    /// Records an `ok` or `error:` reply from the controller.
    ///
    /// # Arguments
    /// * `is_error` - True for an `error:` reply.
    ///
    /// # Returns
    /// True if the reply was attributed to a continuous-jog increment. The poller
    /// then logs a rejected increment instead of showing an error dialog.
    pub fn on_reply(&mut self, is_error: bool) -> bool {
        if self.in_flight == 0 {
            return false;
        }
        self.in_flight -= 1;
        if is_error {
            self.rejected = true;
        }
        true
    }
}

impl MachineControlView {
    /// Starts jogging `axis` in `direction` (+1/-1) until `stop_continuous_jog`.
    ///
    /// Repeated calls for the same axis and direction (key auto-repeat) are ignored.
    /// Jogging is refused while disconnected or while a job is streaming.
    ///
    /// # Arguments
    /// * `axis` - Axis letter (X, Y or Z).
    /// * `direction` - Positive or negative direction.
    pub(crate) fn start_continuous_jog(&self, axis: char, direction: f32) {
        let feed = *self.jog_feed_mm_per_min.lock();
        let step = feed / 60.0 * CONTINUOUS_JOG_TICK.as_secs_f32();
        let signed = step.copysign(direction * crate::device_status::jog_direction(axis));

        {
            let state = self.continuous_jog.lock();
            if let Some((a, s, _)) = state.active
                && a == axis
                && s.signum() == signed.signum()
            {
                return;
            }
        }
        self.stop_continuous_jog();

        if *self.is_streaming.lock() {
            self.log_jog(&t!("Jog ignored: a job is running."));
            return;
        }
        if !self.communicator.lock().is_connected() {
            return;
        }
        if !(feed.is_finite() && feed > 0.0) {
            self.log_jog(&t!("Jog ignored: invalid jog feed rate."));
            return;
        }

        let generation = {
            let mut state = self.continuous_jog.lock();
            state.generation += 1;
            state.active = Some((axis, signed, feed));
            state.in_flight = 0;
            state.rejected = false;
            state.generation
        };
        let sign = if signed > 0.0 { '+' } else { '-' };
        self.log_jog(&format!("> {} {axis}{sign}", t!("Continuous jog")));

        let view = self.clone();
        // Send the first increment right away, then one per tick.
        let tick = move || -> glib::ControlFlow {
            let mut state = view.continuous_jog.lock();
            if state.generation != generation {
                return glib::ControlFlow::Break;
            }
            let Some((axis, step, feed)) = state.active else {
                return glib::ControlFlow::Break;
            };
            if state.rejected {
                drop(state);
                view.stop_continuous_jog();
                view.log_jog(&t!(
                    "Continuous jog stopped: the controller rejected a move (travel limit?)"
                ));
                return glib::ControlFlow::Break;
            }
            if *view.is_streaming.lock() || !view.communicator.lock().is_connected() {
                drop(state);
                view.stop_continuous_jog();
                return glib::ControlFlow::Break;
            }
            if state.in_flight < CONTINUOUS_JOG_LOOKAHEAD {
                let cmd = format!("$J=G21 G91 {axis}{step:.4} F{feed:.0}\n");
                if view.communicator.lock().send(cmd.as_bytes()).is_ok() {
                    state.in_flight += 1;
                }
            }
            glib::ControlFlow::Continue
        };
        if tick() == glib::ControlFlow::Continue {
            glib::timeout_add_local(CONTINUOUS_JOG_TICK, tick);
        }
    }

    /// Stops a continuous jog, if one is running, with jog-cancel (0x85).
    ///
    /// GRBL's jog-cancel decelerates to a stop and discards every queued jog
    /// increment, so nothing sent before the cancel moves the machine afterwards.
    pub(crate) fn stop_continuous_jog(&self) {
        let was_active = {
            let mut state = self.continuous_jog.lock();
            state.generation += 1;
            state.active.take().is_some()
        };
        if !was_active {
            return;
        }
        {
            let mut comm = self.communicator.lock();
            if comm.is_connected() {
                let _ = comm.send(&[0x85]);
            }
        }
        self.log_jog(&format!("> {}", t!("Jog cancel (0x85)")));
    }

    /// True when the "Continuous" jog mode checkbox is on.
    pub(crate) fn continuous_jog_enabled(&self) -> bool {
        self.continuous_jog_check.is_active()
    }

    /// Makes a jog button start a continuous jog while pressed (in continuous mode).
    ///
    /// The button's normal click handler performs a single step; it checks
    /// `continuous_jog_enabled` and does nothing in continuous mode.
    ///
    /// # Arguments
    /// * `button` - The jog button.
    /// * `axis` - Axis letter it jogs.
    /// * `direction` - +1.0 or -1.0.
    pub(crate) fn attach_continuous_jog(&self, button: &Button, axis: char, direction: f32) {
        let gesture = gtk4::GestureClick::new();
        gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);

        let view = self.clone();
        gesture.connect_pressed(move |_, _, _, _| {
            if view.continuous_jog_enabled() {
                view.start_continuous_jog(axis, direction);
            }
        });
        let view = self.clone();
        gesture.connect_released(move |_, _, _, _| view.stop_continuous_jog());
        // Ends without a release, e.g. a grab broken by a dialog or the pointer leaving.
        let view = self.clone();
        gesture.connect_stopped(move |_| view.stop_continuous_jog());
        let view = self.clone();
        gesture.connect_cancel(move |_, _| view.stop_continuous_jog());
        let view = self.clone();
        gesture.connect_unpaired_release(move |_, _, _, _, _| view.stop_continuous_jog());

        button.add_controller(gesture);
    }

    fn log_jog(&self, msg: &str) {
        if let Some(c) = self.device_console.as_ref() {
            c.append_log(&format!("{msg}\n"));
        }
    }
}

/// Maps a numpad jog key to its axis and direction (same layout as step jogging).
///
/// # Arguments
/// * `ch` - The key's character.
///
/// # Returns
/// `(axis, direction)` for 8/2 (Y), 4/6 (X) and 9/3 (Z), otherwise `None`.
pub(crate) fn numpad_jog(ch: char) -> Option<(char, f32)> {
    match ch {
        '8' => Some(('Y', 1.0)),
        '2' => Some(('Y', -1.0)),
        '4' => Some(('X', -1.0)),
        '6' => Some(('X', 1.0)),
        '9' => Some(('Z', 1.0)),
        '3' => Some(('Z', -1.0)),
        _ => None,
    }
}
