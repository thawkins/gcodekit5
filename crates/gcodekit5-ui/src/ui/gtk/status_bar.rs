use gtk4::prelude::*;
use gtk4::{Box, Label, Button, ProgressBar, Orientation, Align};

#[derive(Clone)]
pub struct StatusBar {
    pub widget: Box,
    pub estop_btn: Button,
    status_indicator: Label,
    port_label: Label,
    version_label: Label,
    state_label: Label,
    position_label: Label,
    elapsed_label: Label,
    remaining_label: Label,
    progress_bar: ProgressBar,
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

        // Separator
        left_box.append(&Label::new(Some("|")));

        // Version
        let version_label = Label::new(None);
        version_label.add_css_class("status-text");
        left_box.append(&version_label);

        // Separator
        left_box.append(&Label::new(Some("|")));

        // State
        let state_label = Label::new(Some("DISCONNECTED"));
        state_label.add_css_class("status-text");
        left_box.append(&state_label);

        // Separator
        left_box.append(&Label::new(Some("|")));

        // Position
        let position_label = Label::new(Some("Position: ---"));
        position_label.add_css_class("status-text");
        position_label.add_css_class("monospace");
        left_box.append(&position_label);

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
        progress_bar.set_width_request(100);
        progress_bar.set_visible(false);
        progress_bar.set_show_text(true);
        right_box.append(&progress_bar);

        widget.append(&right_box);

        Self {
            widget,
            estop_btn,
            status_indicator,
            port_label,
            version_label,
            state_label,
            position_label,
            elapsed_label,
            remaining_label,
            progress_bar,
        }
    }

    pub fn set_connected(&self, connected: bool, port: &str) {
        if connected {
            self.status_indicator.remove_css_class("disconnected");
            self.status_indicator.add_css_class("connected");
            self.port_label.set_text(port);
        } else {
            self.status_indicator.remove_css_class("connected");
            self.status_indicator.add_css_class("disconnected");
            self.port_label.set_text("Disconnected");
            self.version_label.set_text("");
            self.state_label.set_text("DISCONNECTED");
            self.position_label.set_text("Position: ---");
        }
    }

    pub fn set_version(&self, version: &str) {
        self.version_label.set_text(&format!("Version: {}", version));
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

    pub fn set_position(&self, x: f32, y: f32, z: f32, a: f32, b: f32, c: f32) {
        self.position_label.set_text(&format!(
            "X: {:.3}  Y: {:.3}  Z: {:.3}  A: {:.3}  B: {:.3}  C: {:.3}",
            x, y, z, a, b, c
        ));
    }

    pub fn set_progress(&self, progress: f64, elapsed: &str, remaining: &str) {
        if progress > 0.0 {
            self.progress_bar.set_visible(true);
            self.progress_bar.set_fraction(progress / 100.0);
            self.progress_bar.set_text(Some(&format!("{:.1}%", progress)));
            
            self.elapsed_label.set_text(&format!("Elapsed: {}", elapsed));
            self.remaining_label.set_text(&format!("Remaining: {}", remaining));
        } else {
            self.progress_bar.set_visible(false);
            self.elapsed_label.set_text("");
            self.remaining_label.set_text("");
        }
    }
}
