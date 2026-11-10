//! # Status Bar
//!
//! The application status bar displaying connection state,
//! machine status, current position, and progress information.

use gcodekit5_core::units::{
    format_feed_rate, format_length, get_unit_label, FeedRateUnits, MeasurementSystem,
};
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation, ProgressBar};
use std::cell::Cell;
use std::rc::Rc;

use crate::t;
use gcodekit5_core::{shared_none, SharedOption};
use gtk4::pango::EllipsizeMode; // Importación de tu macro de traducción

#[derive(Clone)]
// Complex type due to GTK widget and machine state fields.
#[allow(clippy::type_complexity)]
pub struct StatusBar {
    pub widget: Box,
    pub estop_btn: Button,
    status_indicator: Label,
    port_label: Label,
    version_label: Label,
    state_separator: Label,
    state_label: Label,
    position_separator: Label,
    position_label: Label,
    feed_spindle_separator: Label,
    feed_spindle_label: Label,
    elapsed_label: Label,
    remaining_label: Label,
    progress_bar: ProgressBar,
    cancel_btn: Button,
    cancel_action: SharedOption<std::boxed::Box<dyn Fn() + 'static>>,
    pub device_info_label: Label,
    pub alert_label: Label, // <-- NUEVA: Para los avisos de la derecha
    /// Timer that clears `alert_label`; replaced by each new notice so an
    /// older notice's timer cannot clear a newer one.
    alert_timer: Rc<Cell<Option<glib::SourceId>>>,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBar {
    /// Builds the status bar (connection, state, position, progress and E-stop).
    pub fn new() -> Self {
        let widget = Box::new(Orientation::Horizontal, 0);
        widget.set_height_request(30);
        widget.add_css_class("status-bar");
        widget.set_margin_start(5);
        widget.set_margin_end(5);
        widget.set_margin_top(2);
        widget.set_margin_bottom(2);

        // Left side container
        let left_box = Box::new(Orientation::Horizontal, 10);
        left_box.set_hexpand(true);
        left_box.set_halign(Align::Start);
        left_box.set_valign(Align::Center);

        // eStop Button
        let estop_btn = Button::with_label(&t!("eStop"));
        estop_btn.add_css_class("estop-button");
        estop_btn.set_sensitive(false);
        left_box.append(&estop_btn);

        // Separator
        left_box.append(&Label::new(Some("  ")));

        // Status Indicator
        let status_indicator = Label::new(Some("■"));
        status_indicator.add_css_class("status-indicator");
        status_indicator.add_css_class("disconnected");
        left_box.append(&status_indicator);

        // Port
        let port_label = Label::new(Some(&t!("Disconnected")));
        port_label.add_css_class("status-text");
        left_box.append(&port_label);

        // Version
        let version_label = Label::new(None);
        version_label.add_css_class("status-text");
        left_box.append(&version_label);

        // Separator (for state)
        let state_separator = Label::new(Some("|"));
        state_separator.set_visible(false);
        left_box.append(&state_separator);

        // State
        let state_label = Label::new(None);
        state_label.add_css_class("status-text");
        state_label.set_visible(false);
        left_box.append(&state_label);

        // Separator (for position)
        let position_separator = Label::new(Some("|"));
        position_separator.set_visible(false);
        left_box.append(&position_separator);

        // Position
        let position_label = Label::new(None);
        position_label.add_css_class("status-text");
        position_label.add_css_class("monospace");
        position_label.set_visible(false);
        left_box.append(&position_label);

        // Separator (for feed/spindle)
        let feed_spindle_separator = Label::new(Some("|"));
        feed_spindle_separator.set_visible(false);
        left_box.append(&feed_spindle_separator);

        // Feed & Spindle
        let feed_spindle_label = Label::new(None);
        feed_spindle_label.add_css_class("status-text");
        feed_spindle_label.add_css_class("monospace");
        feed_spindle_label.set_visible(false);
        left_box.append(&feed_spindle_label);

        widget.append(&left_box);

        // Right side container
        let right_box = Box::new(Orientation::Horizontal, 10);
        right_box.set_halign(Align::End);
        right_box.set_valign(Align::Center);

        // Alert / Notification Label (Nueva etiqueta dedicada a avisos)
        let alert_label = Label::new(None);
        alert_label.add_css_class("status-alert-text"); // Puedes darle estilo en el CSS
        right_box.append(&alert_label);

        // Elapsed
        let elapsed_label = Label::new(None);
        elapsed_label.add_css_class("status-text");
        right_box.append(&elapsed_label);

        // Remaining
        let remaining_label = Label::new(None);
        remaining_label.add_css_class("status-text");
        right_box.append(&remaining_label);

        // Progress
        let progress_bar = ProgressBar::new();
        progress_bar.set_width_request(120);
        progress_bar.set_visible(false);
        progress_bar.set_show_text(true);
        right_box.append(&progress_bar);

        // Add active machine name to status bar
        let device_info_label = Label::new(Some(&t!("No device selected")));
        device_info_label.add_css_class("dim-label");
        device_info_label.set_halign(Align::Start);
        device_info_label.set_ellipsize(EllipsizeMode::End);
        left_box.append(&device_info_label);

        // Cancel (for long-running, cancellable UI tasks)
        #[allow(clippy::type_complexity)]
        let cancel_action: SharedOption<std::boxed::Box<dyn Fn() + 'static>> = shared_none();

        let cancel_btn = Button::builder().tooltip_text(t!("Cancel")).build();
        cancel_btn.set_visible(false);
        {
            let child = Box::new(Orientation::Horizontal, 6);
            child.append(&Image::from_icon_name("process-stop-symbolic"));
            child.append(&Label::new(Some(&t!("Cancel"))));
            cancel_btn.set_child(Some(&child));
        }
        {
            let cancel_action = cancel_action.clone();
            cancel_btn.connect_clicked(move |btn| {
                // Take the callback out before running it: callbacks commonly end up calling
                // `set_cancel_action(None)` (e.g. via a toggled handler), which would panic
                // with "already borrowed" if the RefCell were still borrowed here.
                // Cancelling is one-shot, so the action is not restored afterwards.
                let cb = cancel_action.borrow_mut().take();
                btn.set_visible(false);
                if let Some(cb) = cb {
                    cb();
                }
            });
        }
        right_box.append(&cancel_btn);

        widget.append(&right_box);

        Self {
            widget,
            estop_btn,
            status_indicator,
            port_label,
            version_label,
            state_separator,
            state_label,
            position_separator,
            position_label,
            feed_spindle_separator,
            feed_spindle_label,
            elapsed_label,
            remaining_label,
            progress_bar,
            cancel_btn,
            cancel_action,
            device_info_label,
            alert_label,
            alert_timer: Rc::new(Cell::new(None)),
        }
    }

    /// Shows the connection state and port name.
    pub fn set_connected(&self, connected: bool, port: &str) {
        self.estop_btn.set_sensitive(connected);
        if connected {
            self.status_indicator.remove_css_class("disconnected");
            self.status_indicator.add_css_class("connected");
            self.port_label.set_text(port);
            self.state_separator.set_visible(true);
            self.state_label.set_visible(true);
            self.position_separator.set_visible(true);
            self.position_label.set_visible(true);
            self.feed_spindle_separator.set_visible(true);
            self.feed_spindle_label.set_visible(true);
        } else {
            self.status_indicator.remove_css_class("connected");
            self.status_indicator.add_css_class("disconnected");
            self.port_label.set_text(&t!("Disconnected"));
            self.version_label.set_text("");
            self.state_separator.set_visible(false);
            self.state_label.set_visible(false);
            self.state_label.set_text("");
            self.position_separator.set_visible(false);
            self.position_label.set_visible(false);
            self.position_label.set_text("");
            self.feed_spindle_separator.set_visible(false);
            self.feed_spindle_label.set_visible(false);
            self.feed_spindle_label.set_text("");
        }
    }

    /// Shows the firmware version.
    pub fn set_version(&self, version: &str) {
        self.version_label
            .set_text(&format!("{} {}", t!("Version:"), version));
    }

    /// Shows the machine state, coloured by state class, on the left.
    ///
    /// Any other text (e.g. "Importing image...") is shown on the right as a
    /// notice and cleared after 4 seconds.
    pub fn set_state(&self, state: &str) {
        // GRBL reports e.g. "Alarm", "Hold:0", "Run", "Idle" (case varies by firmware).
        let low = state.to_ascii_lowercase();
        let is_machine_state = [
            "alarm", "run", "hold", "idle", "jog", "home", "door", "check", "sleep",
        ]
        .iter()
        .any(|prefix| low.starts_with(prefix));

        if is_machine_state {
            self.state_label.set_text(state);

            self.state_label.remove_css_class("state-alarm");
            self.state_label.remove_css_class("state-run");
            self.state_label.remove_css_class("state-hold");
            self.state_label.remove_css_class("state-idle");

            if low.starts_with("alarm") {
                self.state_label.add_css_class("state-alarm");
            } else if low.starts_with("run") {
                self.state_label.add_css_class("state-run");
            } else if low.starts_with("hold") {
                self.state_label.add_css_class("state-hold");
            } else if low.starts_with("idle") {
                self.state_label.add_css_class("state-idle");
            }
        } else {
            // Es un aviso personalizado de la interfaz (Ej: "Importing image...")
            // En la etiqueta de alertas a la derecha
            self.alert_label.set_text(state);
            // Temporizador: Limpia automáticamente el aviso de la derecha tras 4 segundos (4000ms)
            let alert_label_clone = self.alert_label.clone();
            let timer = self.alert_timer.clone();
            if let Some(previous) = self.alert_timer.take() {
                previous.remove();
            }
            let id = glib::timeout_add_local(std::time::Duration::from_millis(4000), move || {
                alert_label_clone.set_text("");
                timer.set(None);
                glib::ControlFlow::Break
            });
            self.alert_timer.set(Some(id));
        }
    }

    /// Shows the machine position (mm in, display units out).
    // Status bar update requires all machine state fields.
    #[allow(clippy::too_many_arguments)]
    pub fn set_position(
        &self,
        x: f32,
        y: f32,
        z: f32,
        a: f32,
        b: f32,
        c: f32,
        system: MeasurementSystem,
    ) {
        let unit_label = get_unit_label(system);
        self.position_label.set_text(&format!(
            "X: {}  Y: {}  Z: {}  A: {}  B: {}  C: {} ({})",
            format_length(x, system),
            format_length(y, system),
            format_length(z, system),
            format_length(a, system),
            format_length(b, system),
            format_length(c, system),
            unit_label
        ));
    }

    /// Shows feed (mm/min in, display units out) and spindle speed.
    pub fn set_feed_spindle(&self, feed_rate: f64, spindle_speed: u32, feed_units: FeedRateUnits) {
        self.feed_spindle_label.set_text(&format!(
            "F: {}  S: {} RPM",
            format_feed_rate(feed_rate as f32, feed_units),
            spindle_speed
        ));
    }

    /// Shows job progress (percent) with elapsed and remaining time; 0 hides it.
    pub fn set_progress(&self, progress: f64, elapsed: &str, remaining: &str) {
        if progress > 0.0 {
            self.progress_bar.set_visible(true);
            self.progress_bar
                .set_fraction((progress / 100.0).clamp(0.0, 1.0));
            self.progress_bar
                .set_text(Some(&format!("{:.1}%", progress)));
            if elapsed.is_empty() {
                self.elapsed_label.set_text("");
            } else {
                self.elapsed_label
                    .set_text(&format!("{} {}", t!("Elapsed:"), elapsed));
            }
            if remaining.is_empty() {
                self.remaining_label.set_text("");
            } else {
                self.remaining_label
                    .set_text(&format!("{} {}", t!("Remaining:"), remaining));
            }
        } else {
            self.progress_bar.set_visible(false);
            self.elapsed_label.set_text("");
            self.remaining_label.set_text("");
        }
    }

    /// Sets (or clears) the action run by the Cancel button.
    pub fn set_cancel_action(&self, action: Option<std::boxed::Box<dyn Fn() + 'static>>) {
        *self.cancel_action.borrow_mut() = action;
        let visible = self.cancel_action.borrow().is_some();
        self.cancel_btn.set_visible(visible);
        self.cancel_btn.set_sensitive(visible);
    }

    /// Shows the active device name, controller and type.
    pub fn set_device_info(&self, name: &str, controller: &str, device_type: &str) {
        let text = format!("{} | {} | {}", name, controller, device_type);
        self.device_info_label.set_text(&text);
    }

    /// Shows the active device profile from the device manager.
    pub fn refresh_device_info(&self, device_manager: &gcodekit5_devicedb::DeviceManager) {
        if let Some(profile) = device_manager.get_active_profile() {
            let controller_type = format!("{:?}", profile.controller_type);
            let device_name = &profile.name;
            let device_type = format!("{:?}", profile.device_type);
            self.set_device_info(device_name, &controller_type, &device_type);
        }
    }
}
