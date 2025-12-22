use gcodekit5_core::units::{
    format_feed_rate, format_length, get_unit_label, FeedRateUnits, MeasurementSystem,
};
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation, ProgressBar};

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
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
    cancel_action: Rc<RefCell<Option<std::boxed::Box<dyn Fn() + 'static>>>>,
}

impl StatusBar {
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
        let estop_btn = Button::with_label("eStop");
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
        let port_label = Label::new(Some("Disconnected"));
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

        // Cancel (for long-running, cancellable UI tasks)
        let cancel_action: Rc<RefCell<Option<std::boxed::Box<dyn Fn() + 'static>>>> =
            Rc::new(RefCell::new(None));

        let cancel_btn = Button::builder().tooltip_text("Cancel").build();
        cancel_btn.set_visible(false);
        {
            let child = Box::new(Orientation::Horizontal, 6);
            child.append(&Image::from_icon_name("process-stop-symbolic"));
            child.append(&Label::new(Some("Cancel")));
            cancel_btn.set_child(Some(&child));
        }
        {
            let cancel_action = cancel_action.clone();
            cancel_btn.connect_clicked(move |_| {
                if let Some(cb) = cancel_action.borrow().as_ref() {
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
        }
    }

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
            self.port_label.set_text("Disconnected");
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

    pub fn set_version(&self, version: &str) {
        self.version_label
            .set_text(&format!("Version: {}", version));
    }

    pub fn set_state(&self, state: &str) {
        self.state_label.set_text(state);

        // Update color based on state
        self.state_label.remove_css_class("state-alarm");
        self.state_label.remove_css_class("state-run");
        self.state_label.remove_css_class("state-hold");
        self.state_label.remove_css_class("state-idle");

        match state {
            "ALARM" => self.state_label.add_css_class("state-alarm"),
            "Run" => self.state_label.add_css_class("state-run"),
            s if s.starts_with("Hold") => self.state_label.add_css_class("state-hold"),
            "Idle" | "IDLE" => self.state_label.add_css_class("state-idle"),
            _ => {}
        }
    }

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

    pub fn set_feed_spindle(&self, feed_rate: f64, spindle_speed: u32, feed_units: FeedRateUnits) {
        self.feed_spindle_label.set_text(&format!(
            "F: {}  S: {} RPM",
            format_feed_rate(feed_rate as f32, feed_units),
            spindle_speed
        ));
    }

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
                    .set_text(&format!("Elapsed: {}", elapsed));
            }
            if remaining.is_empty() {
                self.remaining_label.set_text("");
            } else {
                self.remaining_label
                    .set_text(&format!("Remaining: {}", remaining));
            }
        } else {
            self.progress_bar.set_visible(false);
            self.elapsed_label.set_text("");
            self.remaining_label.set_text("");
        }
    }

    pub fn set_cancel_action(&self, action: Option<std::boxed::Box<dyn Fn() + 'static>>) {
        *self.cancel_action.borrow_mut() = action;
        let visible = self.cancel_action.borrow().is_some();
        self.cancel_btn.set_visible(visible);
        self.cancel_btn.set_sensitive(visible);
    }
}
