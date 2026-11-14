//! Feed rate and spindle override handlers
//!
//! The ±/reset buttons send one GRBL realtime byte per click. The "Set" controls
//! move an override to an exact percentage by sending one step at a time and
//! waiting for the controller's `Ov:` status report to confirm each step, since
//! GRBL can merge identical realtime bytes that arrive back-to-back.

use super::*;
use gcodekit5_communication::firmware::grbl::{
    FEED_OVERRIDE_CODES, MAX_OVERRIDE_PERCENT, MIN_OVERRIDE_PERCENT, OverrideStepCodes,
    SPINDLE_OVERRIDE_CODES, next_override_step,
};
use std::time::{Duration, Instant};

/// How often the "Set override" sequence checks for the controller's `Ov:` report.
const OVERRIDE_SEQUENCE_TICK: Duration = Duration::from_millis(20);
/// How long to wait for a status report to confirm one override step.
const OVERRIDE_STEP_TIMEOUT: Duration = Duration::from_secs(2);
/// Upper bound on steps for one "Set" request (a reset plus 9 x 10% and 9 x 1% is 19).
const MAX_OVERRIDE_STEPS: u32 = 32;

/// Which override a "Set" sequence adjusts.
#[derive(Clone, Copy)]
enum OverrideKind {
    Feed,
    Spindle,
}

impl OverrideKind {
    fn name(self) -> &'static str {
        match self {
            OverrideKind::Feed => "Feed",
            OverrideKind::Spindle => "Spindle",
        }
    }

    fn codes(self) -> OverrideStepCodes {
        match self {
            OverrideKind::Feed => FEED_OVERRIDE_CODES,
            OverrideKind::Spindle => SPINDLE_OVERRIDE_CODES,
        }
    }

    /// The value last reported by the controller in its `Ov:` field.
    fn reported(self, ov: &OverrideState) -> u16 {
        match self {
            OverrideKind::Feed => ov.feed,
            OverrideKind::Spindle => ov.spindle,
        }
    }
}

impl MachineControlView {
    // Feed Rate Override Controls Setup
    pub(crate) fn setup_override_handlers(view: &Self) {
        // Feed Rate Override Controls
        // GRBL Realtime Commands for Feed Override:
        // 0x90 = Reset to 100%
        // 0x91 = +10%
        // 0x92 = -10%
        // 0x93 = +1%
        // 0x94 = -1%
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_inc10.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed +10%\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x91]); // Feed +10%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_inc1.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed +1%\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x93]); // Feed +1%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_dec1.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed -1%\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x94]); // Feed -1%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_dec10.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed -10%\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x92]); // Feed -10%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_reset.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed Reset (100%)\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x90]); // Feed override reset to 100%
                }
            });
        }

        // Spindle Override Controls
        // GRBL Realtime Commands for Spindle Override:
        // 0x99 = 100% (Reset)
        // 0x9A = +10%
        // 0x9B = -10%
        // 0x9C = +1%
        // 0x9D = -1%
        // 0x9E = Spindle Stop (Toggle)
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_inc10.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle +10%\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x9A]); // Spindle +10%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_inc1.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle +1%\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x9C]); // Spindle +1%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_dec1.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle -1%\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x9D]); // Spindle -1%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_dec10.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle -10%\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x9B]); // Spindle -10%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_stop.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle Stop\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x9E]); // Spindle stop
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_reset.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle Reset (100%)\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x99]); // Spindle override reset to 100%
                }
            });
        }

        Self::setup_override_set(view, OverrideKind::Feed, &view.feed_target, &view.feed_set);
        Self::setup_override_set(
            view,
            OverrideKind::Spindle,
            &view.spindle_target,
            &view.spindle_set,
        );
    }

    /// Builds the target-percentage spin button and "Set" button for one override row.
    ///
    /// # Arguments
    /// * `target_tooltip` - Tooltip for the percentage field.
    /// * `set_tooltip` - Tooltip for the "Set" button.
    ///
    /// # Returns
    /// The spin button (10–200%, initially 100%) and the "Set" button.
    pub(crate) fn override_target_widgets(
        target_tooltip: &str,
        set_tooltip: &str,
    ) -> (SpinButton, Button) {
        let target = SpinButton::with_range(
            f64::from(MIN_OVERRIDE_PERCENT),
            f64::from(MAX_OVERRIDE_PERCENT),
            1.0,
        );
        target.set_value(100.0);
        target.set_digits(0);
        target.set_width_chars(3);
        target.set_margin_start(6);
        target.set_tooltip_text(Some(target_tooltip));

        let set = Button::with_label(&t!("Set %"));
        set.set_tooltip_text(Some(set_tooltip));
        (target, set)
    }

    /// Wires a "Set" button to move one override to its target percentage.
    ///
    /// Each step is a single realtime byte; the next step is only chosen once a status
    /// report has stored a new `Ov:` value that differs from the one before the step.
    /// Reports come from the status poller when idle and from the DirectSender worker
    /// (which polls `?` every 200 ms) while a job streams, so this also works mid-job.
    /// GRBL reports `Ov:` right after an override changes; a step that is not
    /// confirmed within the timeout ends the sequence.
    fn setup_override_set(view: &Self, kind: OverrideKind, target: &SpinButton, set: &Button) {
        let communicator = view.communicator.clone();
        let console = view.device_console.clone();
        let last_overrides = view.last_overrides.clone();
        let target = target.clone();

        set.connect_clicked(move |set_btn| {
            let log = {
                let console = console.clone();
                move |msg: &str| {
                    if let Some(c) = console.as_ref() {
                        c.append_log(&format!("{msg}\n"));
                    }
                }
            };

            if !communicator.lock().is_connected() {
                log(&format!("! {} override: not connected", kind.name()));
                return;
            }
            let goal = target.value().round() as u16;
            log(&format!("> {} override: set to {goal}%", kind.name()));
            set_btn.set_sensitive(false);

            let communicator = communicator.clone();
            let last_overrides = last_overrides.clone();
            let set_btn = set_btn.clone();
            // (value before the step, deadline for the controller to report a change)
            let mut awaiting: Option<(u16, Instant)> = None;
            let mut steps = 0u32;

            glib::timeout_add_local(OVERRIDE_SEQUENCE_TICK, move || {
                let finish = |msg: Option<String>| {
                    if let Some(msg) = msg {
                        log(&msg);
                    }
                    set_btn.set_sensitive(true);
                    glib::ControlFlow::Break
                };

                if !communicator.lock().is_connected() {
                    return finish(Some(format!("! {} override: disconnected", kind.name())));
                }
                let current = kind.reported(&last_overrides.lock());
                if let Some((before, deadline)) = awaiting {
                    if current == before {
                        if Instant::now() < deadline {
                            return glib::ControlFlow::Continue;
                        }
                        return finish(Some(format!(
                            "! {} override: controller did not confirm the change from \
                             {before}% (no Ov: report)",
                            kind.name()
                        )));
                    }
                    awaiting = None;
                }

                let Some(byte) = next_override_step(current, goal, kind.codes()) else {
                    return finish(Some(format!("> {} override: now {current}%", kind.name())));
                };
                if steps >= MAX_OVERRIDE_STEPS {
                    return finish(Some(format!(
                        "! {} override: stopped at {current}% after {steps} steps",
                        kind.name()
                    )));
                }
                if let Err(e) = communicator.lock().send(&[byte]) {
                    return finish(Some(format!(
                        "! {} override: send failed: {e}",
                        kind.name()
                    )));
                }
                steps += 1;
                awaiting = Some((current, Instant::now() + OVERRIDE_STEP_TIMEOUT));
                glib::ControlFlow::Continue
            });
        });
    }
}
