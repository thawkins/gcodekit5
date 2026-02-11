use gcodekit5_communication::firmware::grbl::status_parser::{
    FeedSpindleState, OverrideState, StatusParser,
};
use gcodekit5_communication::{
    Communicator, ConnectionDriver, ConnectionParams, SerialCommunicator,
};
use gcodekit5_core::units::{
    format_feed_rate, format_length, get_unit_label, parse_feed_rate, FeedRateUnits,
    MeasurementSystem,
};
use gcodekit5_settings::controller::SettingsController;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    accessible::Property as AccessibleProperty, pango::EllipsizeMode, Align, Box, Button,
    CheckButton, ComboBoxText, EventControllerKey, Grid, Image, Label, Orientation, Overlay, Paned,
    PolicyType, ScrolledWindow, SizeGroup, SizeGroupMode, ToggleButton,
};
use std::cell::Cell;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::device_status;
use crate::t;
use crate::ui::gtk::device_console::DeviceConsoleView;
use crate::ui::gtk::editor::GcodeEditor;
use crate::ui::gtk::help_browser;
use crate::ui::gtk::status_bar::StatusBar;
use crate::ui::gtk::visualizer::GcodeVisualizer;
use std::rc::Rc;

fn set_button_icon_label(btn: &Button, icon: &str, label: &str) {
    let content = Box::new(Orientation::Horizontal, 6);
    content.set_halign(Align::Center);
    content.set_valign(Align::Center);

    let img = Image::from_icon_name(icon);
    img.set_pixel_size(16);

    let lbl = Label::new(Some(label));
    lbl.set_valign(Align::Center);

    content.append(&img);
    content.append(&lbl);

    btn.set_child(Some(&content));
}

fn make_icon_label_button(icon: &str, label: &str) -> Button {
    let btn = Button::new();
    set_button_icon_label(&btn, icon, label);
    btn
}

fn make_icon_label_toggle(icon: &str, label: &str) -> ToggleButton {
    let btn = ToggleButton::new();
    let content = Box::new(Orientation::Horizontal, 6);
    content.set_halign(Align::Center);
    content.set_valign(Align::Center);

    let img = Image::from_icon_name(icon);
    img.set_pixel_size(16);

    let lbl = Label::new(Some(label));
    lbl.set_valign(Align::Center);

    content.append(&img);
    content.append(&lbl);

    btn.set_child(Some(&content));
    btn
}

#[derive(Clone)]
pub struct MachineControlView {
    pub widget: Paned,
    pub port_combo: ComboBoxText,
    pub connect_btn: Button,
    pub refresh_btn: Button,
    pub send_btn: Button,
    pub stop_btn: Button,
    pub pause_btn: Button,
    pub resume_btn: Button,

    // Sidebar status + state details
    pub conn_status_port: Label,
    pub conn_status_baud: Label,
    pub conn_status_state: Label,
    pub disabled_reason_label: Label,

    pub state_label: Label,
    pub state_feed_label: Label,
    pub state_spindle_label: Label,
    pub state_buffer_label: Label,

    pub home_btn: Button,
    pub unlock_btn: Button,
    pub wcs_btns: Vec<ToggleButton>,

    // Feed Rate & Spindle Override Controls
    pub feed_value: Label,
    pub feed_dec10: Button,
    pub feed_dec1: Button,
    pub feed_reset: Button,
    pub feed_inc1: Button,
    pub feed_inc10: Button,
    pub spindle_value: Label,
    pub spindle_dec10: Button,
    pub spindle_dec1: Button,
    pub spindle_reset: Button,
    pub spindle_stop: Button,
    pub spindle_inc1: Button,
    pub spindle_inc10: Button,

    pub x_dro: Label,
    pub y_dro: Label,
    pub z_dro: Label,
    pub x_zero_btn: Button,
    pub y_zero_btn: Button,
    pub z_zero_btn: Button,
    pub zero_all_btn: Button,
    pub goto_zero_btn: Button,
    pub goto_zero_include_z: CheckButton,
    pub world_x: Label,
    pub world_y: Label,
    pub world_z: Label,
    pub step_combo: ComboBoxText,
    pub step_label: Label,
    pub jog_feed_entry: gtk4::Entry,
    pub jog_feed_units: Label,
    pub jog_step_mm: Arc<Mutex<f32>>,
    pub jog_feed_mm_per_min: Arc<Mutex<f32>>,
    pub current_feed_units: Arc<Mutex<FeedRateUnits>>,
    pub jog_x_pos: Button,
    pub jog_x_neg: Button,
    pub jog_y_pos: Button,
    pub jog_y_neg: Button,
    pub jog_z_pos: Button,
    pub jog_z_neg: Button,
    pub estop_btn: Button,
    pub communicator: Arc<Mutex<SerialCommunicator>>,
    pub status_bar: Option<StatusBar>,
    pub device_console: Option<Rc<DeviceConsoleView>>,
    pub editor: Option<Rc<GcodeEditor>>,
    pub visualizer: Option<Rc<GcodeVisualizer>>,
    pub send_queue: Arc<Mutex<VecDeque<String>>>,
    pub total_lines: Arc<Mutex<usize>>,
    pub is_streaming: Arc<Mutex<bool>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub waiting_for_ack: Arc<Mutex<bool>>,
    pub current_units: Arc<Mutex<MeasurementSystem>>,
    pub last_overrides: Arc<Mutex<OverrideState>>,
    pub job_start_time: Arc<Mutex<Option<std::time::Instant>>>,
}

impl MachineControlView {
    pub fn new(
        status_bar: Option<StatusBar>,
        device_console: Option<Rc<DeviceConsoleView>>,
        editor: Option<Rc<GcodeEditor>>,
        visualizer: Option<Rc<GcodeVisualizer>>,
        settings_controller: Option<Rc<SettingsController>>,
    ) -> Self {
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        fn make_section(title: &str, child: &impl IsA<gtk4::Widget>) -> Box {
            let section = Box::new(Orientation::Vertical, 4);
            section.add_css_class("mc-section");

            let header = Label::new(Some(title));
            header.add_css_class("mc-section-title");
            header.set_halign(Align::Start);

            section.append(&header);
            section.append(child);
            section
        }

        // Helper function to disable connection-dependent buttons
        #[allow(clippy::too_many_arguments)]
        fn set_controls_enabled(
            send_btn: &Button,
            stop_btn: &Button,
            pause_btn: &Button,
            resume_btn: &Button,
            home_btn: &Button,
            unlock_btn: &Button,
            wcs_btns: &[ToggleButton],
            x_zero_btn: &Button,
            y_zero_btn: &Button,
            z_zero_btn: &Button,
            zero_all_btn: &Button,
            goto_zero_btn: &Button,
            step_combo: &ComboBoxText,
            jog_feed_entry: &gtk4::Entry,
            jog_x_pos: &Button,
            jog_x_neg: &Button,
            jog_y_pos: &Button,
            jog_y_neg: &Button,
            jog_z_pos: &Button,
            jog_z_neg: &Button,
            estop_btn: &Button,
            enabled: bool,
        ) {
            send_btn.set_sensitive(enabled);
            stop_btn.set_sensitive(enabled);
            pause_btn.set_sensitive(enabled);
            resume_btn.set_sensitive(enabled);
            home_btn.set_sensitive(enabled);
            unlock_btn.set_sensitive(enabled);
            for btn in wcs_btns {
                btn.set_sensitive(enabled);
            }
            x_zero_btn.set_sensitive(enabled);
            y_zero_btn.set_sensitive(enabled);
            z_zero_btn.set_sensitive(enabled);
            zero_all_btn.set_sensitive(enabled);
            goto_zero_btn.set_sensitive(enabled);
            step_combo.set_sensitive(enabled);
            jog_feed_entry.set_sensitive(enabled);
            jog_x_pos.set_sensitive(enabled);
            jog_x_neg.set_sensitive(enabled);
            jog_y_pos.set_sensitive(enabled);
            jog_y_neg.set_sensitive(enabled);
            jog_z_pos.set_sensitive(enabled);
            jog_z_neg.set_sensitive(enabled);
            estop_btn.set_sensitive(enabled);
        }

        // ═════════════════════════════════════════════
        // LEFT SIDEBAR
        // ═════════════════════════════════════════════
        let sidebar = Box::new(Orientation::Vertical, 8);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_start(8);
        sidebar.set_margin_end(8);
        sidebar.set_margin_top(8);
        sidebar.set_margin_bottom(8);

        let sidebar_scroller = ScrolledWindow::new();
        sidebar_scroller.set_hexpand(true);
        sidebar_scroller.set_vexpand(true);
        sidebar_scroller.set_policy(PolicyType::Never, PolicyType::Automatic);
        sidebar_scroller.set_child(Some(&sidebar));

        // Connection status strip (always visible)
        let status_strip = Box::new(Orientation::Horizontal, 10);
        status_strip.add_css_class("mc-status-strip");

        let conn_status_port = Label::new(Some(&t!("Port: -")));
        conn_status_port.add_css_class("dim-label");
        conn_status_port.set_halign(Align::Start);
        conn_status_port.set_single_line_mode(true);
        conn_status_port.set_ellipsize(EllipsizeMode::End);

        let conn_status_baud = Label::new(Some(&t!("Baud: 115200")));
        conn_status_baud.add_css_class("dim-label");
        conn_status_baud.set_halign(Align::Start);
        conn_status_baud.set_single_line_mode(true);
        conn_status_baud.set_ellipsize(EllipsizeMode::End);

        let conn_status_state = Label::new(Some(&t!("State: Disconnected")));
        conn_status_state.add_css_class("dim-label");
        conn_status_state.set_halign(Align::Start);
        conn_status_state.set_single_line_mode(true);
        conn_status_state.set_ellipsize(EllipsizeMode::End);
        conn_status_state.set_hexpand(true);

        status_strip.append(&conn_status_port);
        status_strip.append(&conn_status_baud);
        status_strip.append(&conn_status_state);
        sidebar.append(&status_strip);

        // Connection Section
        let conn_box = Box::new(Orientation::Vertical, 4);

        let port_combo = ComboBoxText::new();
        port_combo.append(Some("none"), &t!("No ports available"));
        port_combo.set_active_id(Some("none"));
        conn_box.append(&port_combo);

        let conn_btn_box = Box::new(Orientation::Horizontal, 6);
        let connect_btn = make_icon_label_button("network-wired-symbolic", &t!("Connect"));
        connect_btn.add_css_class("suggested-action");
        connect_btn.set_hexpand(true);

        let refresh_btn = Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_tooltip_text(Some(&t!("Refresh ports")));
        refresh_btn.update_property(&[AccessibleProperty::Label(&t!("Refresh ports"))]);
        conn_btn_box.append(&connect_btn);
        conn_btn_box.append(&refresh_btn);
        conn_box.append(&conn_btn_box);
        sidebar.append(&make_section(&t!("Connection"), &conn_box));

        // Machine State Section
        let state_box = Box::new(Orientation::Vertical, 4);

        let state_label = Label::new(Some(&t!("Disconnected")));
        state_label.add_css_class("title-2");
        state_label.set_height_request(32);
        state_box.append(&state_label);

        let state_details = Box::new(Orientation::Horizontal, 10);
        let state_feed_label = Label::new(Some(&t!("Feed: -")));
        state_feed_label.add_css_class("dim-label");
        state_feed_label.set_halign(Align::Start);
        state_feed_label.set_single_line_mode(true);
        state_feed_label.set_ellipsize(EllipsizeMode::End);

        let state_spindle_label = Label::new(Some(&t!("Spindle: -")));
        state_spindle_label.add_css_class("dim-label");
        state_spindle_label.set_halign(Align::Start);
        state_spindle_label.set_single_line_mode(true);
        state_spindle_label.set_ellipsize(EllipsizeMode::End);

        let state_buffer_label = Label::new(Some(&t!("Buffer: -")));
        state_buffer_label.add_css_class("dim-label");
        state_buffer_label.set_halign(Align::Start);
        state_buffer_label.set_single_line_mode(true);
        state_buffer_label.set_ellipsize(EllipsizeMode::End);
        state_buffer_label.set_hexpand(true);

        state_details.append(&state_feed_label);
        state_details.append(&state_spindle_label);
        state_details.append(&state_buffer_label);
        state_box.append(&state_details);

        let state_btn_box = Box::new(Orientation::Horizontal, 6);
        let home_btn = make_icon_label_button("go-home-symbolic", &t!("Home"));
        home_btn.set_tooltip_text(Some(&t!("Home machine ($H)")));
        home_btn.set_hexpand(true);

        let unlock_btn = make_icon_label_button("lock-open-symbolic", &t!("Unlock"));
        unlock_btn.set_tooltip_text(Some(&t!("Unlock from ALARM ($X)")));
        unlock_btn.set_hexpand(true);

        state_btn_box.append(&home_btn);
        state_btn_box.append(&unlock_btn);
        state_box.append(&state_btn_box);
        sidebar.append(&make_section(&t!("Machine State"), &state_box));

        // Work Coordinates Section
        let wcs_box = Box::new(Orientation::Vertical, 4);

        let wcs_grid = Grid::new();
        wcs_grid.set_column_spacing(6);
        wcs_grid.set_row_spacing(6);
        wcs_grid.set_halign(Align::Center);

        let mut wcs_btns: Vec<ToggleButton> = Vec::new();
        for i in 0..6 {
            let label = format!("G{}", 54 + i);
            let btn = make_icon_label_toggle("bookmark-new-symbolic", &label);
            btn.add_css_class("mc-wcs-toggle");
            btn.set_hexpand(true);

            if let Some(first) = wcs_btns.first() {
                btn.set_group(Some(first));
            }

            wcs_btns.push(btn.clone());
            wcs_grid.attach(&btn, i % 3, i / 3, 1, 1);
        }
        if let Some(first) = wcs_btns.first() {
            first.set_active(true);
        }
        wcs_box.append(&wcs_grid);
        sidebar.append(&make_section(&t!("Work Coordinates"), &wcs_box));

        // Transmission / Job controls
        let trans_box = Box::new(Orientation::Vertical, 4);

        let trans_row1 = Box::new(Orientation::Horizontal, 6);

        // Keep job control buttons aligned/same width.
        let job_btn_group = SizeGroup::new(SizeGroupMode::Horizontal);

        let send_btn = make_icon_label_button("mail-send-symbolic", &t!("Send"));
        send_btn.set_tooltip_text(Some(&t!("Send G-code from the editor")));
        send_btn.add_css_class("suggested-action");
        send_btn.set_hexpand(true);
        job_btn_group.add_widget(&send_btn);

        let stop_btn = make_icon_label_button("media-playback-stop-symbolic", &t!("Stop"));
        stop_btn.set_tooltip_text(Some(&t!(
            "Stop job: abort streaming (does not reset controller)"
        )));
        stop_btn.add_css_class("destructive-action");
        stop_btn.set_hexpand(true);
        job_btn_group.add_widget(&stop_btn);

        trans_row1.append(&send_btn);
        trans_row1.append(&stop_btn);
        trans_box.append(&trans_row1);

        let trans_row2 = Box::new(Orientation::Horizontal, 6);
        let pause_btn = make_icon_label_button("media-playback-pause-symbolic", &t!("Pause"));
        pause_btn.set_tooltip_text(Some(&t!("Pause streaming (!)")));
        pause_btn.set_hexpand(true);
        job_btn_group.add_widget(&pause_btn);

        let resume_btn = make_icon_label_button("media-playback-start-symbolic", &t!("Resume"));
        resume_btn.set_tooltip_text(Some(&t!("Resume streaming (~)")));
        resume_btn.set_hexpand(true);
        job_btn_group.add_widget(&resume_btn);

        trans_row2.append(&pause_btn);
        trans_row2.append(&resume_btn);
        trans_box.append(&trans_row2);

        let stop_hint = Label::new(Some(&t!(
            "Stop aborts streaming; E‑Stop resets the controller."
        )));
        stop_hint.add_css_class("dim-label");
        stop_hint.set_wrap(true);
        stop_hint.set_halign(Align::Start);
        trans_box.append(&stop_hint);

        sidebar.append(&make_section(&t!("Job Control"), &trans_box));

        let disabled_reason_label = Label::new(Some(&t!("Connect to enable controls.")));
        disabled_reason_label.add_css_class("dim-label");
        disabled_reason_label.set_halign(Align::Start);
        disabled_reason_label.set_margin_top(4);
        disabled_reason_label.set_single_line_mode(true);
        disabled_reason_label.set_ellipsize(EllipsizeMode::End);
        sidebar.append(&disabled_reason_label);

        // widget.append(&sidebar); // Moved to Paned setup

        // ═════════════════════════════════════════════
        // MAIN AREA
        // ═════════════════════════════════════════════
        let main_area = Box::new(Orientation::Vertical, 12);
        main_area.set_hexpand(true);
        main_area.set_vexpand(true);
        main_area.set_margin_top(12);
        main_area.set_margin_bottom(12);
        main_area.set_margin_start(12);
        main_area.set_margin_end(12);
        main_area.set_valign(Align::Center);

        // Feed Rate & Spindle Speed Override Section
        let override_section = Box::new(Orientation::Vertical, 8);
        override_section.set_halign(Align::Center);
        override_section.set_margin_bottom(12);

        let override_title = Label::new(Some(&t!("Current Feed & Spindle")));
        override_title.add_css_class("dim-label");
        override_section.append(&override_title);

        let override_box = Box::new(Orientation::Vertical, 6);
        override_box.set_halign(Align::Center);

        // Feed Rate Row (compact)
        let feed_row = Box::new(Orientation::Horizontal, 8);

        let feed_label = Label::new(Some(&t!("Feed:")));
        feed_label.add_css_class("dim-label");
        feed_label.set_width_request(50);

        let feed_value = Label::new(Some("0.0 mm/min (100%)"));
        feed_value.add_css_class("mc-override-value");
        feed_value.set_width_chars(20);

        let feed_controls = Box::new(Orientation::Horizontal, 2);
        let feed_dec10 = Button::with_label("-10");
        feed_dec10.set_tooltip_text(Some(&t!("Decrease feed rate by 10%")));
        let feed_dec1 = Button::with_label("-1");
        feed_dec1.set_tooltip_text(Some(&t!("Decrease feed rate by 1%")));
        let feed_reset = Button::with_label("⟲");
        feed_reset.set_tooltip_text(Some(&t!("Reset feed rate to 100%")));
        let feed_inc1 = Button::with_label("+1");
        feed_inc1.set_tooltip_text(Some(&t!("Increase feed rate by 1%")));
        let feed_inc10 = Button::with_label("+10");
        feed_inc10.set_tooltip_text(Some(&t!("Increase feed rate by 10%")));

        feed_controls.append(&feed_dec10);
        feed_controls.append(&feed_dec1);
        feed_controls.append(&feed_reset);
        feed_controls.append(&feed_inc1);
        feed_controls.append(&feed_inc10);

        feed_row.append(&feed_label);
        feed_row.append(&feed_value);
        feed_row.append(&feed_controls);

        // Spindle Speed Row (compact)
        let spindle_row = Box::new(Orientation::Horizontal, 8);

        let spindle_label = Label::new(Some(&t!("Spindle:")));
        spindle_label.add_css_class("dim-label");
        spindle_label.set_width_request(50);

        let spindle_value = Label::new(Some("0 S (100%)"));
        spindle_value.add_css_class("mc-override-value");
        spindle_value.set_width_chars(20);

        let spindle_controls = Box::new(Orientation::Horizontal, 2);
        let spindle_dec10 = Button::with_label("-10");
        spindle_dec10.set_tooltip_text(Some(&t!("Decrease spindle speed by 10%")));
        let spindle_dec1 = Button::with_label("-1");
        spindle_dec1.set_tooltip_text(Some(&t!("Decrease spindle speed by 1%")));
        let spindle_reset = Button::with_label("⟲");
        spindle_reset.set_tooltip_text(Some(&t!("Reset spindle speed to 100%")));
        let spindle_stop = Button::with_label("■");
        spindle_stop.set_tooltip_text(Some(&t!("Stop spindle")));
        spindle_stop.add_css_class("destructive-action");
        let spindle_inc1 = Button::with_label("+1");
        spindle_inc1.set_tooltip_text(Some(&t!("Increase spindle speed by 1%")));
        let spindle_inc10 = Button::with_label("+10");
        spindle_inc10.set_tooltip_text(Some(&t!("Increase spindle speed by 10%")));

        spindle_controls.append(&spindle_dec10);
        spindle_controls.append(&spindle_dec1);
        spindle_controls.append(&spindle_reset);
        spindle_controls.append(&spindle_stop);
        spindle_controls.append(&spindle_inc1);
        spindle_controls.append(&spindle_inc10);

        spindle_row.append(&spindle_label);
        spindle_row.append(&spindle_value);
        spindle_row.append(&spindle_controls);

        override_box.append(&feed_row);
        override_box.append(&spindle_row);
        override_section.append(&override_box);
        main_area.append(&override_section);

        // DRO Section with buttons on the right
        let dro_container = Box::new(Orientation::Horizontal, 12);
        dro_container.set_hexpand(true);
        dro_container.set_halign(Align::Center);

        // Left side: DRO displays
        let dro_box = Box::new(Orientation::Vertical, 4);

        let work_title = Label::new(Some(&t!("Work Coordinates (WPos)")));
        work_title.add_css_class("dim-label");
        dro_box.append(&work_title);

        let create_dro = |axis: &str, display: &str| -> (Box, Label, Button) {
            let b = Box::new(Orientation::Horizontal, 8);
            b.add_css_class("dro-axis");
            b.set_height_request(38);

            let l = Label::new(Some(display));
            l.add_css_class("dro-label");
            l.add_css_class("mc-dro-label");
            l.set_width_request(52);

            let v = Label::new(Some("0.000"));
            v.add_css_class("dro-value");
            v.add_css_class("mc-dro-value");
            v.set_hexpand(true);
            v.set_halign(Align::End);

            let z = make_icon_label_button("edit-clear-symbolic", &t!("Zero"));
            z.add_css_class("circular");
            z.set_valign(Align::Center);
            let tooltip = format!("{} {axis}", t!("Set work axis to zero"));
            z.set_tooltip_text(Some(&tooltip));
            let a11y_label = format!("{} {axis}", t!("Zero"));
            z.update_property(&[AccessibleProperty::Label(&a11y_label)]);

            b.append(&l);
            b.append(&v);
            b.append(&z);
            (b, v, z)
        };

        let (x_box, x_dro, x_zero_btn) = create_dro("X", "WX");
        let (y_box, y_dro, y_zero_btn) = create_dro("Y", "WY");
        let (z_box, z_dro, z_zero_btn) = create_dro("Z", "WZ");

        dro_box.append(&x_box);
        dro_box.append(&y_box);
        dro_box.append(&z_box);

        // Right side: Zero All and Go to Work Zero buttons
        let zero_actions = Box::new(Orientation::Vertical, 8);
        zero_actions.set_valign(Align::Center);

        let zero_all_btn = make_icon_label_button("edit-clear-symbolic", &t!("Zero All Axes"));
        zero_all_btn.set_tooltip_text(Some(&t!("Set active work position to 0 for X/Y/Z")));
        zero_all_btn.add_css_class("destructive-action");
        zero_actions.append(&zero_all_btn);

        let goto_zero_btn = make_icon_label_button("go-jump-symbolic", &t!("Go to Work Zero"));
        goto_zero_btn.set_tooltip_text(Some(&t!(
            "Rapid move to work origin (X/Y, or X/Y/Z if Include Z is checked)"
        )));

        // Checkbox for including Z axis in go to work zero
        let goto_zero_include_z = CheckButton::with_label(&t!("Include Z"));
        goto_zero_include_z.set_active(false);
        goto_zero_include_z.set_tooltip_text(Some(&t!(
            "When checked, Go to Work Zero will also move to Z0"
        )));

        let goto_zero_row = Box::new(Orientation::Horizontal, 8);
        goto_zero_row.set_halign(Align::Start);
        goto_zero_row.append(&goto_zero_btn);
        goto_zero_row.append(&goto_zero_include_z);
        zero_actions.append(&goto_zero_row);

        dro_container.append(&dro_box);
        dro_container.append(&zero_actions);

        // Machine Coordinates (compact)
        let world_box = Box::new(Orientation::Vertical, 2);
        world_box.set_margin_top(8);
        let world_title = Label::new(Some(&t!("Machine Coordinates (MPos)")));
        world_title.add_css_class("dim-label");
        world_box.append(&world_title);

        let world_vals = Box::new(Orientation::Horizontal, 20);
        world_vals.set_halign(Align::Center);
        let world_x = Label::new(Some("MX: 0.000"));
        let world_y = Label::new(Some("MY: 0.000"));
        let world_z = Label::new(Some("MZ: 0.000"));
        world_vals.append(&world_x);
        world_vals.append(&world_y);
        world_vals.append(&world_z);
        world_box.append(&world_vals);

        main_area.append(&dro_container);
        main_area.append(&world_box);

        // Jog Controls (step and feed on same line)
        let jog_area = Box::new(Orientation::Vertical, 8);
        jog_area.set_halign(Align::Center);
        jog_area.set_margin_top(12);

        let jog_step_mm = Arc::new(Mutex::new(1.0_f32));
        let jog_feed_mm_per_min = Arc::new(Mutex::new(2000.0_f32));

        // Combined step and feed controls on one line
        let jog_controls_box = Box::new(Orientation::Horizontal, 16);
        jog_controls_box.set_halign(Align::Center);

        // Step controls
        let step_box = Box::new(Orientation::Horizontal, 8);
        let step_label = Label::new(None);
        step_label.add_css_class("dim-label");
        step_box.append(&step_label);

        let step_combo = ComboBoxText::new();
        step_combo.set_width_request(140);
        step_box.append(&step_combo);

        // Feed controls
        let feed_box = Box::new(Orientation::Horizontal, 8);
        let feed_label = Label::new(Some(&t!("Jog Feed:")));
        feed_label.add_css_class("dim-label");
        feed_box.append(&feed_label);

        let jog_feed_entry = gtk4::Entry::new();
        jog_feed_entry.set_width_chars(8);
        feed_box.append(&jog_feed_entry);

        let jog_feed_units = Label::new(None);
        jog_feed_units.add_css_class("dim-label");
        feed_box.append(&jog_feed_units);

        jog_controls_box.append(&step_box);
        jog_controls_box.append(&feed_box);
        jog_area.append(&jog_controls_box);

        let kb_hint = Label::new(Some(&t!("Keyboard jog: 8/2/4/6 (XY), 9/3 (Z)")));
        kb_hint.add_css_class("dim-label");
        jog_area.append(&kb_hint);

        // Directional Pads
        let pads_box = Box::new(Orientation::Horizontal, 60);
        pads_box.set_halign(Align::Center);
        pads_box.set_valign(Align::Center);

        // XY Pad
        let xy_grid = Grid::new();
        xy_grid.set_column_spacing(5);
        xy_grid.set_row_spacing(5);

        let jog_y_pos = make_icon_label_button("go-up-symbolic", "Y+");
        jog_y_pos.set_tooltip_text(Some(&t!("Jog Y+")));
        jog_y_pos.update_property(&[AccessibleProperty::Label(&t!("Jog Y+"))]);
        let jog_x_neg = make_icon_label_button("go-previous-symbolic", "X-");
        jog_x_neg.set_tooltip_text(Some(&t!("Jog X-")));
        jog_x_neg.update_property(&[AccessibleProperty::Label(&t!("Jog X-"))]);
        let jog_x_pos = make_icon_label_button("go-next-symbolic", "X+");
        jog_x_pos.set_tooltip_text(Some(&t!("Jog X+")));
        jog_x_pos.update_property(&[AccessibleProperty::Label(&t!("Jog X+"))]);
        let jog_y_neg = make_icon_label_button("go-down-symbolic", "Y-");
        jog_y_neg.set_tooltip_text(Some(&t!("Jog Y-")));
        jog_y_neg.update_property(&[AccessibleProperty::Label(&t!("Jog Y-"))]);
        let home_center = Label::new(Some("XY"));

        // Style buttons
        for btn in [&jog_y_pos, &jog_x_neg, &jog_x_pos, &jog_y_neg] {
            btn.set_width_request(56);
            btn.set_height_request(56);
        }

        xy_grid.attach(&jog_y_pos, 1, 0, 1, 1);
        xy_grid.attach(&jog_x_neg, 0, 1, 1, 1);
        xy_grid.attach(&home_center, 1, 1, 1, 1);
        xy_grid.attach(&jog_x_pos, 2, 1, 1, 1);
        xy_grid.attach(&jog_y_neg, 1, 2, 1, 1);
        pads_box.append(&xy_grid);

        // Z Pad & eStop
        let z_estop_box = Box::new(Orientation::Horizontal, 20);
        z_estop_box.set_valign(Align::Center);

        let z_box = Box::new(Orientation::Vertical, 5);
        z_box.set_valign(Align::Center);
        let jog_z_pos = make_icon_label_button("go-up-symbolic", "Z+");
        jog_z_pos.set_tooltip_text(Some(&t!("Jog Z+")));
        jog_z_pos.update_property(&[AccessibleProperty::Label(&t!("Jog Z+"))]);
        let z_label = Label::new(Some("Z"));
        // Match the XY pad's center row height so Z+/Z- align with Y+/Y-.
        z_label.set_width_request(56);
        z_label.set_height_request(56);
        z_label.set_halign(Align::Center);
        z_label.set_valign(Align::Center);
        let jog_z_neg = make_icon_label_button("go-down-symbolic", "Z-");
        jog_z_neg.set_tooltip_text(Some(&t!("Jog Z-")));
        jog_z_neg.update_property(&[AccessibleProperty::Label(&t!("Jog Z-"))]);

        for btn in [&jog_z_pos, &jog_z_neg] {
            btn.set_width_request(56);
            btn.set_height_request(56);
        }

        z_box.append(&jog_z_pos);
        z_box.append(&z_label);
        z_box.append(&jog_z_neg);
        z_estop_box.append(&z_box);

        // eStop
        let estop_btn = Button::new();
        let estop_content = Box::new(Orientation::Vertical, 2);
        estop_content.set_halign(Align::Center);
        estop_content.set_valign(Align::Center);
        let estop_picture = Image::from_resource("/com/gcodekit5/images/eStop2.png");
        estop_picture.set_pixel_size(64);
        let estop_text = Label::new(Some(&t!("E-STOP")));
        estop_text.add_css_class("title-4");
        estop_content.append(&estop_picture);
        estop_content.append(&estop_text);
        estop_btn.set_child(Some(&estop_content));
        estop_btn.set_tooltip_text(Some(&t!("Emergency stop (soft reset Ctrl-X)")));
        estop_btn.update_property(&[AccessibleProperty::Label(&t!("Emergency stop"))]);

        estop_btn.add_css_class("estop-big");
        // Shorter button, centered; width can be a bit larger.
        estop_btn.set_width_request(112);
        estop_btn.set_height_request(80);
        estop_btn.set_hexpand(false);
        estop_btn.set_vexpand(false);
        estop_btn.set_valign(Align::Center);
        z_estop_box.append(&estop_btn);

        pads_box.append(&z_estop_box);
        jog_area.append(&pads_box);

        main_area.append(&jog_area);

        // Setup Paned
        // Use an inner paned so we have: [sidebar] | [main area] | [device console]
        let main_scroller = ScrolledWindow::new();
        main_scroller.set_hexpand(true);
        main_scroller.set_vexpand(true);
        main_scroller.set_policy(PolicyType::Never, PolicyType::Automatic);
        main_scroller.set_child(Some(&main_area));

        let inner_paned = Paned::new(Orientation::Horizontal);
        inner_paned.set_start_child(Some(&main_scroller));

        let console_container = Box::new(Orientation::Vertical, 10);
        console_container.set_hexpand(true);
        console_container.set_vexpand(true);
        console_container.set_margin_top(12);
        console_container.set_margin_bottom(12);
        console_container.set_margin_start(12);
        console_container.set_margin_end(12);

        // Device Console header (clear/copy + collapse)
        let console_header = Box::new(Orientation::Horizontal, 6);
        console_header.set_hexpand(true);

        let console_title = Label::new(Some(&t!("Device Console")));
        console_title.add_css_class("mc-section-title");
        console_title.set_halign(Align::Start);
        console_title.set_hexpand(true);

        let console_clear_btn = Button::from_icon_name("user-trash-symbolic");
        console_clear_btn.set_tooltip_text(Some(&t!("Clear console")));
        console_clear_btn.update_property(&[AccessibleProperty::Label(&t!("Clear console"))]);

        let console_copy_err_btn = Button::from_icon_name("edit-copy-symbolic");
        console_copy_err_btn.set_tooltip_text(Some(&t!("Copy last error")));
        console_copy_err_btn.update_property(&[AccessibleProperty::Label(&t!("Copy last error"))]);

        let help_btn = help_browser::make_help_button("machine_control");

        console_header.append(&console_title);
        console_header.append(&console_clear_btn);
        console_header.append(&console_copy_err_btn);
        console_header.append(&help_btn);
        console_container.append(&console_header);

        // Embed Device Console if present
        if let Some(ref console_view) = device_console {
            // Guard against accidentally parenting the same widget twice.
            if console_view.widget.parent().is_none() {
                console_container.append(&console_view.widget);
            }
        } else {
            let placeholder = Label::new(Some(&t!("Device Console not available")));
            placeholder.set_halign(Align::Center);
            console_container.append(&placeholder);
        }

        let console_scroller = ScrolledWindow::new();
        console_scroller.set_hexpand(true);
        console_scroller.set_vexpand(true);
        console_scroller.set_policy(PolicyType::Never, PolicyType::Automatic);
        console_scroller.set_child(Some(&console_container));

        inner_paned.set_end_child(Some(&console_scroller));

        // Header actions (DeviceConsoleView toolbar is intentionally minimal)
        {
            let console_view = device_console.clone();
            console_clear_btn.connect_clicked(move |_| {
                if let Some(c) = console_view.as_ref() {
                    c.clear_log();
                }
            });
        }

        {
            let console_view = device_console.clone();
            let console_container = console_container.clone();
            console_copy_err_btn.connect_clicked(move |_| {
                let Some(c) = console_view.as_ref() else {
                    return;
                };
                let text = c.get_log_text();
                let mut last = None;
                for line in text.lines().rev() {
                    let l = line.trim();
                    if l.is_empty() || l == "ok" || l.starts_with('<') {
                        continue;
                    }
                    let low = l.to_lowercase();
                    if low.contains("error") || low.contains("alarm") {
                        last = Some(l.to_string());
                        break;
                    }
                }
                if let Some(line) = last {
                    let clipboard = console_container.display().clipboard();
                    clipboard.set_text(&line);
                }
            });
        }

        // Console hide/show
        // - Hide button lives in the console header.
        // - When hidden, the console collapses to 0 width and a small floating "Show" panel appears.
        let console_collapsed = Rc::new(Cell::new(false));
        let console_last_pos = Rc::new(Cell::new(0));

        let console_hide_btn = make_icon_label_button("view-conceal-symbolic", &t!("Hide"));
        console_hide_btn.set_tooltip_text(Some(&t!("Hide device console")));
        console_header.append(&console_hide_btn);

        let console_show_btn = make_icon_label_button("view-reveal-symbolic", &t!("Show Console"));
        console_show_btn.set_tooltip_text(Some(&t!("Show device console")));

        let console_show_panel = Box::new(Orientation::Horizontal, 0);
        console_show_panel.add_css_class("osd");
        console_show_panel.set_halign(Align::End);
        console_show_panel.set_valign(Align::Start);
        console_show_panel.set_margin_end(12);
        console_show_panel.set_margin_top(12);
        console_show_panel.append(&console_show_btn);
        console_show_panel.set_visible(false);

        let inner_overlay = Overlay::new();
        inner_overlay.set_child(Some(&inner_paned));
        inner_overlay.add_overlay(&console_show_panel);
        inner_overlay.set_hexpand(true);
        inner_overlay.set_vexpand(true);

        {
            let inner_paned = inner_paned.clone();
            let hide_btn = console_hide_btn.clone();
            let console_collapsed = console_collapsed.clone();
            let console_last_pos = console_last_pos.clone();
            let show_panel = console_show_panel.clone();

            console_hide_btn.connect_clicked(move |_| {
                if console_collapsed.get() {
                    return;
                }
                console_last_pos.set(inner_paned.position());
                inner_paned.set_end_child(None::<&gtk4::Widget>);
                hide_btn.set_sensitive(false);
                console_collapsed.set(true);
                show_panel.set_visible(true);
            });
        }

        {
            let inner_paned = inner_paned.clone();
            let console_scroller = console_scroller.clone();
            let hide_btn = console_hide_btn.clone();
            let console_collapsed = console_collapsed.clone();
            let console_last_pos = console_last_pos.clone();
            let show_panel = console_show_panel.clone();

            console_show_btn.connect_clicked(move |_| {
                if !console_collapsed.get() {
                    return;
                }
                inner_paned.set_end_child(Some(&console_scroller));
                let pos = console_last_pos.get();
                if pos > 0 {
                    inner_paned.set_position(pos);
                }
                hide_btn.set_sensitive(true);
                console_collapsed.set(false);
                show_panel.set_visible(false);
            });
        }

        // Initial sizing (~59% main / ~41% console), then let the user resize.
        let inner_sized = Rc::new(Cell::new(false));
        inner_paned.add_tick_callback({
            let inner_sized = inner_sized.clone();
            move |paned, _clock| {
                if inner_sized.get() {
                    return glib::ControlFlow::Break;
                }
                let width = paned.width();
                if width <= 0 {
                    return glib::ControlFlow::Continue;
                }
                // Was 36% console; increase by ~15% => ~41.4% console.
                paned.set_position((width as f64 * 0.586) as i32);
                inner_sized.set(true);
                glib::ControlFlow::Break
            }
        });

        widget.set_start_child(Some(&sidebar_scroller));
        widget.set_end_child(Some(&inner_overlay));

        // Initial sizing (20% sidebar / 80% main), then let the user resize.
        let outer_sized = Rc::new(Cell::new(false));
        widget.add_tick_callback({
            let outer_sized = outer_sized.clone();
            move |paned, _clock| {
                if outer_sized.get() {
                    return glib::ControlFlow::Break;
                }
                let width = paned.width();
                if width <= 0 {
                    return glib::ControlFlow::Continue;
                }
                paned.set_position((width as f64 * 0.2) as i32);
                outer_sized.set(true);
                glib::ControlFlow::Break
            }
        });

        let communicator = Arc::new(Mutex::new(SerialCommunicator::new()));

        // Initialize units from settings if available
        let initial_units = if let Some(controller) = &settings_controller {
            controller
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system
        } else {
            MeasurementSystem::Metric
        };
        let current_units = Arc::new(Mutex::new(initial_units));

        let initial_feed_units = if let Some(controller) = &settings_controller {
            controller.persistence.borrow().config().ui.feed_rate_units
        } else {
            FeedRateUnits::default()
        };
        let current_feed_units = Arc::new(Mutex::new(initial_feed_units));

        // Populate jog UI from initial settings
        {
            let unit_label = get_unit_label(initial_units);
            step_label.set_text(&format!("Step ({unit_label}):"));
            let presets_mm: [f32; 6] = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0];
            for mm in presets_mm {
                step_combo.append(
                    Some(&format!("{mm}")),
                    &format!("{} {unit_label}", format_length(mm, initial_units)),
                );
            }
            step_combo.set_active_id(Some("1"));
            *jog_step_mm.lock().unwrap_or_else(|p| p.into_inner()) = 1.0;

            jog_feed_units.set_text(&initial_feed_units.to_string());
            jog_feed_entry.set_text(&format_feed_rate(2000.0, initial_feed_units));
            *jog_feed_mm_per_min
                .lock()
                .unwrap_or_else(|p| p.into_inner()) = 2000.0;
        }

        // Listen for unit changes
        if let Some(controller) = &settings_controller {
            let units_clone = current_units.clone();
            let step_label_clone = step_label.clone();
            let step_combo_clone = step_combo.clone();
            let jog_step_mm_clone = jog_step_mm.clone();

            let feed_units_clone = current_feed_units.clone();
            let jog_feed_units_clone = jog_feed_units.clone();
            let jog_feed_entry_clone = jog_feed_entry.clone();
            let jog_feed_mm_per_min_clone = jog_feed_mm_per_min.clone();

            controller.on_setting_changed(move |key, value| {
                if key == "measurement_system" {
                    let new_units = match value {
                        "Imperial" => MeasurementSystem::Imperial,
                        _ => MeasurementSystem::Metric,
                    };
                    if let Ok(mut u) = units_clone.lock() {
                        *u = new_units;
                    }

                    let units = new_units;
                    let unit_label = get_unit_label(units);
                    step_label_clone.set_text(&format!("Step ({unit_label}):"));

                    let selected_mm = *jog_step_mm_clone.lock().unwrap_or_else(|p| p.into_inner());
                    step_combo_clone.remove_all();
                    let presets_mm: [f32; 6] = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0];
                    for mm in presets_mm {
                        step_combo_clone.append(
                            Some(&format!("{mm}")),
                            &format!("{} {unit_label}", format_length(mm, units)),
                        );
                    }
                    step_combo_clone.set_active_id(Some(&format!("{selected_mm}")));
                }

                if key == "feed_rate_units" {
                    let new_units = match value {
                        "mm/sec" => FeedRateUnits::MmPerSec,
                        "in/min" => FeedRateUnits::InPerMin,
                        "in/sec" => FeedRateUnits::InPerSec,
                        _ => FeedRateUnits::MmPerMin,
                    };
                    *feed_units_clone.lock().unwrap_or_else(|p| p.into_inner()) = new_units;
                    jog_feed_units_clone.set_text(&new_units.to_string());
                    let current_mm_per_min = *jog_feed_mm_per_min_clone
                        .lock()
                        .unwrap_or_else(|p| p.into_inner());
                    jog_feed_entry_clone.set_text(&format_feed_rate(current_mm_per_min, new_units));
                }
            });
        }

        let view = Self {
            widget,
            port_combo,
            connect_btn,
            refresh_btn,
            send_btn,
            stop_btn,
            pause_btn,
            resume_btn,

            conn_status_port,
            conn_status_baud,
            conn_status_state,
            disabled_reason_label,

            state_label,
            state_feed_label,
            state_spindle_label,
            state_buffer_label,

            home_btn,
            unlock_btn,
            wcs_btns,

            feed_value,
            feed_dec10,
            feed_dec1,
            feed_reset,
            feed_inc1,
            feed_inc10,
            spindle_value,
            spindle_dec10,
            spindle_dec1,
            spindle_reset,
            spindle_stop,
            spindle_inc1,
            spindle_inc10,

            x_dro,
            y_dro,
            z_dro,
            x_zero_btn,
            y_zero_btn,
            z_zero_btn,
            zero_all_btn,
            goto_zero_btn,
            goto_zero_include_z,
            world_x,
            world_y,
            world_z,
            step_combo,
            step_label,
            jog_feed_entry,
            jog_feed_units,
            jog_step_mm: jog_step_mm.clone(),
            jog_feed_mm_per_min: jog_feed_mm_per_min.clone(),
            current_feed_units: current_feed_units.clone(),
            jog_x_pos,
            jog_x_neg,
            jog_y_pos,
            jog_y_neg,
            jog_z_pos,
            jog_z_neg,
            estop_btn,
            communicator,
            status_bar: status_bar.clone(),
            device_console: device_console.clone(),
            editor,
            visualizer,
            send_queue: Arc::new(Mutex::new(VecDeque::new())),
            total_lines: Arc::new(Mutex::new(0)),
            is_streaming: Arc::new(Mutex::new(false)),
            is_paused: Arc::new(Mutex::new(false)),
            waiting_for_ack: Arc::new(Mutex::new(false)),
            current_units,
            last_overrides: Arc::new(Mutex::new(OverrideState {
                feed: 100,
                rapid: 100,
                spindle: 100,
            })),
            job_start_time: Arc::new(Mutex::new(None)),
        };

        // Keep internal jog values in base units (mm, mm/min)
        {
            let jog_step_mm = view.jog_step_mm.clone();
            view.step_combo.connect_changed(move |c| {
                if let Some(id) = c.active_id() {
                    if let Ok(v) = id.parse::<f32>() {
                        *jog_step_mm.lock().unwrap_or_else(|p| p.into_inner()) = v;
                    }
                }
            });
        }

        {
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let current_feed_units = view.current_feed_units.clone();
            view.jog_feed_entry.connect_changed(move |e| {
                let units = *current_feed_units.lock().unwrap_or_else(|p| p.into_inner());
                if let Ok(v) = parse_feed_rate(&e.text(), units) {
                    *jog_feed_mm_per_min
                        .lock()
                        .unwrap_or_else(|p| p.into_inner()) = v;
                }
            });
        }

        view.refresh_ports();

        // Disable all controls initially (until connected)
        set_controls_enabled(
            &view.send_btn,
            &view.stop_btn,
            &view.pause_btn,
            &view.resume_btn,
            &view.home_btn,
            &view.unlock_btn,
            &view.wcs_btns,
            &view.x_zero_btn,
            &view.y_zero_btn,
            &view.z_zero_btn,
            &view.zero_all_btn,
            &view.goto_zero_btn,
            &view.step_combo,
            &view.jog_feed_entry,
            &view.jog_x_pos,
            &view.jog_x_neg,
            &view.jog_y_pos,
            &view.jog_y_neg,
            &view.jog_z_pos,
            &view.jog_z_neg,
            &view.estop_btn,
            false,
        );

        // Setup jog button handlers
        fn send_jog(
            axis: char,
            delta: f32,
            communicator: &Arc<Mutex<SerialCommunicator>>,
            feed_mm_per_min: f32,
            console: &Option<Rc<DeviceConsoleView>>,
        ) {
            let jog_cmd = format!("$J=G91 {axis}{delta} F{feed_mm_per_min}\n");
            if let Some(c) = console {
                c.append_log(&format!("> {}\n", jog_cmd.trim()));
            }
            if let Ok(mut comm) = communicator.lock() {
                let _ = comm.send(jog_cmd.as_bytes());
            }
        }

        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            view.jog_x_pos.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap_or_else(|p| p.into_inner());
                let feed = *jog_feed_mm_per_min
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                send_jog('X', step, &communicator, feed, &console);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            view.jog_x_neg.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap_or_else(|p| p.into_inner());
                let feed = *jog_feed_mm_per_min
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                send_jog('X', -step, &communicator, feed, &console);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            view.jog_y_pos.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap_or_else(|p| p.into_inner());
                let feed = *jog_feed_mm_per_min
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                send_jog('Y', step, &communicator, feed, &console);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            view.jog_y_neg.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap_or_else(|p| p.into_inner());
                let feed = *jog_feed_mm_per_min
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                send_jog('Y', -step, &communicator, feed, &console);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            view.jog_z_pos.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap_or_else(|p| p.into_inner());
                let feed = *jog_feed_mm_per_min
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                send_jog('Z', step, &communicator, feed, &console);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            view.jog_z_neg.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap_or_else(|p| p.into_inner());
                let feed = *jog_feed_mm_per_min
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                send_jog('Z', -step, &communicator, feed, &console);
            });
        }

        // Keyboard jog shortcuts (when console entry is not focused)
        {
            let controller = EventControllerKey::new();
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console_entry = view
                .device_console
                .as_ref()
                .map(|c| c.command_entry.clone());
            let console = view.device_console.clone();

            controller.connect_key_pressed(move |_, key, _, _| {
                if let Some(entry) = console_entry.as_ref() {
                    if entry.has_focus() {
                        return glib::Propagation::Proceed;
                    }
                }

                let Some(ch) = key.to_unicode() else {
                    return glib::Propagation::Proceed;
                };

                let step = *jog_step_mm.lock().unwrap_or_else(|p| p.into_inner());
                let feed = *jog_feed_mm_per_min
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());

                match ch {
                    '8' => send_jog('Y', step, &communicator, feed, &console),
                    '2' => send_jog('Y', -step, &communicator, feed, &console),
                    '4' => send_jog('X', -step, &communicator, feed, &console),
                    '6' => send_jog('X', step, &communicator, feed, &console),
                    '9' => send_jog('Z', step, &communicator, feed, &console),
                    '3' => send_jog('Z', -step, &communicator, feed, &console),
                    _ => return glib::Propagation::Proceed,
                }

                glib::Propagation::Stop
            });

            view.widget.add_controller(controller);
        }

        let view_clone = view.clone();
        view.refresh_btn.connect_clicked(move |_| {
            view_clone.refresh_ports();
        });

        // Transmission Controls
        {
            let communicator = view.communicator.clone();
            let is_paused = view.is_paused.clone();
            let console = view.device_console.clone();
            view.pause_btn.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> ! (Pause)\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(b"!");
                }
                *is_paused.lock().unwrap_or_else(|p| p.into_inner()) = true;
            });
        }
        {
            let communicator = view.communicator.clone();
            let is_paused = view.is_paused.clone();
            let is_streaming = view.is_streaming.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let send_queue = view.send_queue.clone();
            let console = view.device_console.clone();

            view.resume_btn.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> ~ (Resume)\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(b"~");
                }
                *is_paused.lock().unwrap_or_else(|p| p.into_inner()) = false;

                // Kickstart if stalled (streaming, not waiting for ack, and has commands)
                if *is_streaming.lock().unwrap_or_else(|p| p.into_inner())
                    && !*waiting_for_ack.lock().unwrap_or_else(|p| p.into_inner())
                {
                    let mut queue = send_queue.lock().unwrap_or_else(|p| p.into_inner());
                    if let Some(cmd) = queue.pop_front() {
                        if let Some(c) = console.as_ref() {
                            c.append_log(&format!("> {}\n", cmd));
                        }
                        if let Ok(mut comm) = communicator.lock() {
                            let _ = comm.send_command(&cmd);
                            *waiting_for_ack.lock().unwrap_or_else(|p| p.into_inner()) = true;
                        }
                    }
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let is_streaming = view.is_streaming.clone();
            let is_paused = view.is_paused.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let send_queue = view.send_queue.clone();
            let status_bar = view.status_bar.clone();
            let job_start_time = view.job_start_time.clone();
            let console = view.device_console.clone();
            view.stop_btn.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> 0x18 (Stop)\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    // 0x18 = Ctrl-x = Soft Reset
                    let _ = comm.send(&[0x18]);
                }
                *is_streaming.lock().unwrap_or_else(|p| p.into_inner()) = false;
                *is_paused.lock().unwrap_or_else(|p| p.into_inner()) = false;
                *waiting_for_ack.lock().unwrap_or_else(|p| p.into_inner()) = false;
                *job_start_time.lock().unwrap_or_else(|p| p.into_inner()) = None;
                send_queue.lock().unwrap_or_else(|p| p.into_inner()).clear();

                // Reset progress
                if let Some(sb) = status_bar.as_ref() {
                    sb.set_progress(0.0, "", "");
                }
            });
        }
        {
            let editor = view.editor.clone();
            let widget_for_dialog = view.widget.clone();
            let view_clone = view.clone();

            view.send_btn.connect_clicked(move |_| {
                let mut content = String::new();
                if let Some(ed) = editor.as_ref() {
                    content = ed.get_text();
                }

                if content.trim().is_empty() {
                    let dialog = gtk4::MessageDialog::builder()
                        .message_type(gtk4::MessageType::Error)
                        .buttons(gtk4::ButtonsType::Ok)
                        .text(t!("No G-Code to Send"))
                        .secondary_text(t!("Please load or type G-Code into the editor first."))
                        .build();

                    // Set transient parent if possible
                    if let Some(root) = widget_for_dialog.root() {
                        if let Ok(win) = root.downcast::<gtk4::Window>() {
                            dialog.set_transient_for(Some(&win));
                            dialog.set_modal(true);
                        }
                    }

                    dialog.connect_response(|d, _| d.close());
                    dialog.show();
                    return;
                }

                view_clone.start_job(&content);
            });
        }

        // Machine State Controls
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.home_btn.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> $H\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("$H");
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.unlock_btn.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> $X\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("$X");
                }
            });
        }

        // WCS Controls
        for (i, btn) in view.wcs_btns.iter().enumerate() {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            let cmd = format!("G{}", 54 + i);
            btn.connect_toggled(move |b| {
                if !b.is_active() {
                    return;
                }
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(&cmd);
                }
            });
        }

        // Zero Controls
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            let console = view.device_console.clone();
            view.x_zero_btn.connect_clicked(move |_| {
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} X0");
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(&cmd);
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            let console = view.device_console.clone();
            view.y_zero_btn.connect_clicked(move |_| {
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} Y0");
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(&cmd);
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            let console = view.device_console.clone();
            view.z_zero_btn.connect_clicked(move |_| {
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} Z0");
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(&cmd);
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            let widget_for_dialog = view.widget.clone();
            let console = view.device_console.clone();
            view.zero_all_btn.connect_clicked(move |_| {
                let dialog = gtk4::MessageDialog::builder()
                    .message_type(gtk4::MessageType::Question)
                    .buttons(gtk4::ButtonsType::YesNo)
                    .text(t!("Zero all work axes?"))
                    .secondary_text(
                        t!("This sets the active work coordinate system so the current X/Y/Z becomes 0."),
                    )
                    .build();

                // Set transient parent if possible
                if let Some(root) = widget_for_dialog.root() {
                    if let Ok(win) = root.downcast::<gtk4::Window>() {
                        dialog.set_transient_for(Some(&win));
                        dialog.set_modal(true);
                    }
                }

                let communicator = communicator.clone();
                let wcs_btns = wcs_btns.clone();
                let console = console.clone();
                dialog.connect_response(move |d, resp| {
                    if resp == gtk4::ResponseType::Yes {
                        let p = wcs_btns
                            .iter()
                            .position(|b| b.is_active())
                            .map(|i| i + 1)
                            .unwrap_or(1);
                        let cmd = format!("G10 L20 P{p} X0 Y0 Z0");
                        if let Some(c) = console.as_ref() { c.append_log(&format!("> {}\n", cmd)); }
                        if let Ok(mut comm) = communicator.lock() {
                            let _ = comm.send_command(&cmd);
                        }
                    }
                    d.close();
                });

                dialog.show();
            });
        }

        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            let include_z_check = view.goto_zero_include_z.clone();
            view.goto_zero_btn.connect_clicked(move |_| {
                let cmd = if include_z_check.is_active() {
                    "G0 X0 Y0 Z0"
                } else {
                    "G0 X0 Y0"
                };
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(cmd);
                }
            });
        }

        // E-Stop
        {
            let view_clone = view.clone();
            view.estop_btn.connect_clicked(move |_| {
                view_clone.emergency_stop();
            });
        }

        // Console Command Send
        if let Some(console) = view.device_console.as_ref() {
            let communicator = view.communicator.clone();
            let console_clone = console.clone();
            let entry_clone = console.command_entry.clone();

            // Helper function to send command from console
            let send_console_command = move || {
                let cmd = entry_clone.text().to_string().trim().to_string();
                if cmd.is_empty() {
                    return;
                }

                // Add to history
                console_clone.add_to_history(cmd.clone());
                console_clone.reset_history_navigation();

                // Log to console
                console_clone.append_log(&format!("> {}\n", cmd));

                // Send command
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(&cmd);
                }

                // Clear entry
                entry_clone.set_text("");
            };

            // Connect send button
            let send_fn = send_console_command.clone();
            console.send_btn.connect_clicked(move |_| {
                send_fn();
            });

            // Connect Enter key on entry
            console.command_entry.connect_activate(move |_| {
                send_console_command();
            });
        }

        let view_clone = view.clone();
        view.connect_btn.connect_clicked(move |_| {
            let is_connected = view_clone.communicator.lock().unwrap_or_else(|p| p.into_inner()).is_connected();

            if is_connected {
                // Disconnect
                let mut comm = view_clone.communicator.lock().unwrap_or_else(|p| p.into_inner());
                match comm.disconnect() {
                    Ok(_) => {
                        set_button_icon_label(&view_clone.connect_btn, "network-wired-symbolic", &t!("Connect"));
                        view_clone.connect_btn.remove_css_class("destructive-action");
                        view_clone.connect_btn.add_css_class("suggested-action");
                        view_clone.port_combo.set_sensitive(true);
                        view_clone.refresh_btn.set_sensitive(true);

                        view_clone.conn_status_port.set_text(&t!("Port: -"));
                        view_clone.conn_status_baud.set_text(&t!("Baud: 115200"));
                        view_clone.conn_status_state.set_text(&t!("State: Disconnected"));
                        view_clone.state_label.set_text(&t!("Disconnected"));
                        view_clone.disabled_reason_label.set_text(&t!("Connect to enable controls."));

                        // Update global device status
                        device_status::update_connection_status(false, None);

                        // Disable all controls on disconnect
                        set_controls_enabled(
                            &view_clone.send_btn,
                            &view_clone.stop_btn,
                            &view_clone.pause_btn,
                            &view_clone.resume_btn,
                            &view_clone.home_btn,
                            &view_clone.unlock_btn,
                            &view_clone.wcs_btns,
                            &view_clone.x_zero_btn,
                            &view_clone.y_zero_btn,
                            &view_clone.z_zero_btn,
                            &view_clone.zero_all_btn,
                            &view_clone.goto_zero_btn,
                            &view_clone.step_combo,
                            &view_clone.jog_feed_entry,
                            &view_clone.jog_x_pos,
                            &view_clone.jog_x_neg,
                            &view_clone.jog_y_pos,
                            &view_clone.jog_y_neg,
                            &view_clone.jog_z_pos,
                            &view_clone.jog_z_neg,
                            &view_clone.estop_btn,
                            false,
                        );

                        // Update StatusBar
                        if let Some(ref status_bar) = view_clone.status_bar {
                            status_bar.set_connected(false, "");
                        }

                        // Log to device console
                        if let Some(ref console) = view_clone.device_console {
                            console.append_log(&format!("{}\n", t!("Disconnected")));
                        }
                    }
                    Err(e) => {
                        // Log error to device console
                        if let Some(ref console) = view_clone.device_console {
                            console.append_log(&format!("{}: {}\n", t!("Error disconnecting"), e));
                        }
                    }
                }
            } else {
                // Connect
                if view_clone.port_combo.active_id().as_deref() == Some("none") {
                    return;
                }
                if let Some(port_name) = view_clone.port_combo.active_text() {
                    // Disable connect button while connecting
                    view_clone.connect_btn.set_sensitive(false);
                    view_clone.state_label.set_text(&t!("Connecting…"));
                    view_clone.conn_status_state.set_text(&t!("State: Connecting…"));

                    let params = ConnectionParams {
                        driver: ConnectionDriver::Serial,
                        port: port_name.to_string(),
                        baud_rate: 115200,
                        ..Default::default()
                    };

                    // Perform synchronous connection (it's fast)
                    let result = view_clone.communicator.lock().unwrap_or_else(|p| p.into_inner()).connect(&params);

                    match result {
                        Ok(_) => {
                            set_button_icon_label(&view_clone.connect_btn, "network-offline-symbolic", &t!("Disconnect"));
                            view_clone.connect_btn.remove_css_class("suggested-action");
                            view_clone.connect_btn.add_css_class("destructive-action");
                            view_clone.connect_btn.set_sensitive(true);
                            view_clone.port_combo.set_sensitive(false);
                            view_clone.refresh_btn.set_sensitive(false);

                            view_clone.conn_status_port.set_text(&format!("{} {}", t!("Port:"), port_name));
                            view_clone.conn_status_baud.set_text(&t!("Baud: 115200"));
                            view_clone.conn_status_state.set_text(&t!("State: Connected"));
                            view_clone.state_label.set_text(&t!("Connected"));
                            view_clone.disabled_reason_label.set_text("");


                            // Update global device status
                            device_status::update_connection_status(true, Some(port_name.to_string()));

                            // Update StatusBar
                            if let Some(ref sb) = view_clone.status_bar {
                                sb.set_connected(true, port_name.as_ref());
                            }

                            // Log to device console
                            if let Some(ref console) = view_clone.device_console {
                                console.append_log(&format!(
                                    "{} {}\n",
                                    t!("Connected to"),
                                    port_name
                                ));
                            }

                            // Enable all controls on successful connection
                            set_controls_enabled(
                                &view_clone.send_btn,
                                &view_clone.stop_btn,
                                &view_clone.pause_btn,
                                &view_clone.resume_btn,
                                &view_clone.home_btn,
                                &view_clone.unlock_btn,
                                &view_clone.wcs_btns,
                                &view_clone.x_zero_btn,
                                &view_clone.y_zero_btn,
                                &view_clone.z_zero_btn,
                                &view_clone.zero_all_btn,
                                &view_clone.goto_zero_btn,
                                &view_clone.step_combo,
                                &view_clone.jog_feed_entry,
                                &view_clone.jog_x_pos,
                                &view_clone.jog_x_neg,
                                &view_clone.jog_y_pos,
                                &view_clone.jog_y_neg,
                                &view_clone.jog_z_pos,
                                &view_clone.jog_z_neg,
                                &view_clone.estop_btn,
                                true,
                            );

                            // Unlock button should initially be disabled until ALARM state is detected
                            view_clone.unlock_btn.set_sensitive(false);

                            // Trigger startup banner (some firmwares only emit it after reset) and then query firmware + settings.
                            // Important: some controllers ignore commands sent immediately after Ctrl-X.
                            if let Ok(mut comm) = view_clone.communicator.lock() {
                                if let Some(c) = view_clone.device_console.as_ref() { c.append_log("> 0x18 (Reset)\n"); }
                                let _ = comm.send(&[0x18]); // Ctrl-X (soft reset)
                            }

                            // Delay info/settings queries a bit to let the controller reboot.
                            {
                                let communicator_init = view_clone.communicator.clone();
                                let console = view_clone.device_console.clone();
                                glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
                                    if let Ok(mut comm) = communicator_init.try_lock() {
                                        if comm.is_connected() {
                                            if let Some(c) = console.as_ref() { c.append_log("> $I\n"); }
                                            let _ = comm.send_command("$I");
                                        }
                                    }
                                    glib::ControlFlow::Break
                                });
                            }

                            {
                                let communicator_init = view_clone.communicator.clone();
                                let console = view_clone.device_console.clone();
                                glib::timeout_add_local(std::time::Duration::from_millis(800), move || {
                                    if let Ok(mut comm) = communicator_init.try_lock() {
                                        if comm.is_connected() {
                                            if let Some(c) = console.as_ref() { c.append_log("> $$\n"); }
                                            let _ = comm.send_command("$$");
                                            // Ensure status report mask includes Overrides (32) and Feed/Speed (8)
                                            // $10=47 (1+2+4+8+32) = WPos | Buf | Ln | FS | Ov
                                            if let Some(c) = console.as_ref() { c.append_log("> $10=47\n"); }
                                            let _ = comm.send_command("$10=47");
                                        }
                                    }
                                    glib::ControlFlow::Break
                                });
                            }

                            // Simple polling using glib::timeout_add_local - runs on main thread, no blocking
                            let state_label_poll = view_clone.state_label.clone();
                            let state_feed_label_poll = view_clone.state_feed_label.clone();
                            let state_spindle_label_poll = view_clone.state_spindle_label.clone();
                            let state_buffer_label_poll = view_clone.state_buffer_label.clone();
                            let conn_status_state_poll = view_clone.conn_status_state.clone();
                            let disabled_reason_label_poll = view_clone.disabled_reason_label.clone();

                            let x_dro_poll = view_clone.x_dro.clone();
                            let y_dro_poll = view_clone.y_dro.clone();
                            let z_dro_poll = view_clone.z_dro.clone();
                            let world_x_poll = view_clone.world_x.clone();
                            let world_y_poll = view_clone.world_y.clone();
                            let world_z_poll = view_clone.world_z.clone();
                            let feed_value_poll = view_clone.feed_value.clone();
                            let spindle_value_poll = view_clone.spindle_value.clone();
                            let unlock_btn_poll = view_clone.unlock_btn.clone();
                            let communicator_poll = view_clone.communicator.clone();
                            let status_bar_poll = view_clone.status_bar.clone();
                            let is_streaming_poll = view_clone.is_streaming.clone();
                            let is_paused_poll = view_clone.is_paused.clone();
                            let waiting_for_ack_poll = view_clone.waiting_for_ack.clone();
                            let send_queue_poll = view_clone.send_queue.clone();
                            let device_console_poll = view_clone.device_console.clone();
                            let visualizer_poll = view_clone.visualizer.clone();
                            let total_lines_poll = view_clone.total_lines.clone();
                            let current_units_poll = view_clone.current_units.clone();
                            let current_feed_units_poll = view_clone.current_feed_units.clone();
                            let last_overrides_poll = view_clone.last_overrides.clone();
                            let widget_poll = view_clone.widget.clone();
                            let job_start_time_poll = view_clone.job_start_time.clone();

                            let mut query_counter = 0u32;
                            let mut response_buffer = String::new();
                            let mut firmware_detected = false;

                            // Cache the last known Work Coordinate Offset (WCO)
                            // This allows us to derive WPos from MPos even when WCO isn't in every status report
                            let mut last_wco: Option<gcodekit5_communication::firmware::grbl::status_parser::WorkCoordinateOffset> = None;

                            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                                query_counter += 1;

                                // Check if still connected
                                let is_connected = {
                                    if let Ok(comm) = communicator_poll.try_lock() {
                                        comm.is_connected()
                                    } else {
                                        return glib::ControlFlow::Continue;
                                    }
                                };

                                if !is_connected {
                                    return glib::ControlFlow::Break;
                                }

                                // Try to read data (non-blocking, quick)
                                if let Ok(mut comm) = communicator_poll.try_lock() {
                                    if let Ok(response_bytes) = comm.receive() {
                                        if !response_bytes.is_empty() {
                                            let s = String::from_utf8_lossy(&response_bytes);

                                            response_buffer.push_str(&s);

                                            // Process complete lines (support both \n and \r line endings)
                                            while let Some(idx) = {
                                                let idx_n = response_buffer.find('\n');
                                                let idx_r = response_buffer.find('\r');
                                                match (idx_n, idx_r) {
                                                    (Some(n), Some(r)) => Some(n.min(r)),
                                                    (Some(n), None) => Some(n),
                                                    (None, Some(r)) => Some(r),
                                                    (None, None) => None,
                                                }
                                            } {
                                                let line = response_buffer[..idx].trim().to_string();
                                                response_buffer.drain(..idx + 1);

                                                if line.is_empty() { continue; }

                                                // Detect firmware version info
                                                if !firmware_detected && (line.starts_with("[VER:") || line.contains("Grbl")) {
                                                    use gcodekit5_communication::firmware::firmware_detector::FirmwareDetector;
                                                    if let Ok(detection) = FirmwareDetector::parse_response(&line) {
                                                        let fw_type = format!("{:?}", detection.firmware_type);
                                                        let fw_version = detection.version_string.clone();
                                                        device_status::update_firmware_info(fw_type, fw_version, None);
                                                        firmware_detected = true;
                                                    }
                                                }

                                                // Capture GRBL $$ settings lines into global state (for Device Config / CAM).
                                                if line.starts_with('$') && line.contains('=') {
                                                    if let Some((n, v)) = gcodekit5_communication::firmware::grbl::settings::SettingsManager::parse_setting_line(&line) {
                                                        device_status::update_grbl_setting(n, v);
                                                    }
                                                }

                                                // Handle 'ok' or 'error' for streaming
                                                let is_ack = line == "ok";
                                                let lower = line.to_ascii_lowercase();
                                                let lower_trim = lower.trim_start();
                                                let is_error = lower_trim.starts_with("error:");

                                                // If this is an `error:n`, decode it and append the decoded text to the console log line.
                                                let mut line_for_log = line.clone();
                                                if is_error {
                                                    let rest = lower_trim.trim_start_matches("error:").trim();
                                                    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                                                    if let Ok(code_u16) = digits.parse::<u16>() {
                                                        if let Ok(code) = u8::try_from(code_u16) {
                                                            let decoded = gcodekit5_communication::firmware::grbl::decode_error(code);
                                                            line_for_log = format!("{} - {}", line, decoded);

                                                            // Only show dialog if NOT streaming, otherwise just log
                                                            if !*is_streaming_poll.lock().unwrap_or_else(|p| p.into_inner()) {
                                                                let secondary = format!("error:{} - {}", code, decoded);
                                                                let dialog = gtk4::MessageDialog::builder()
                                                                    .message_type(gtk4::MessageType::Error)
                                                                    .buttons(gtk4::ButtonsType::Ok)
                                                                    .text(t!("Controller error"))
                                                                    .secondary_text(&secondary)
                                                                    .build();

                                                                dialog.connect_response(|d, _| d.close());

                                                                // Best-effort parent association.
                                                                if let Some(root) = widget_poll.root() {
                                                                    if let Ok(win) = root.downcast::<gtk4::Window>() {
                                                                        dialog.set_transient_for(Some(&win));
                                                                        dialog.set_modal(true);
                                                                    }
                                                                }
                                                                dialog.show();
                                                            }
                                                        }
                                                    }
                                                }

                                                // Log to console, filtering out status reports and 'ok' acks to avoid spam
                                                if !line.starts_with('<') && line != "ok" {
                                                    if let Some(c) = device_console_poll.as_ref() {
                                                        c.append_log(&format!("{}\n", line_for_log));
                                                    }
                                                }

                                                if is_ack || is_error {
                                                     *waiting_for_ack_poll.lock().unwrap_or_else(|p| p.into_inner()) = false;

                                                     // If error, we might want to stop, but for now we continue
                                                     // if is_error { ... logic to stop ... }

                                                     if *is_streaming_poll.lock().unwrap_or_else(|p| p.into_inner())
                                                         && !*is_paused_poll.lock().unwrap_or_else(|p| p.into_inner()) {
                                                              let mut queue = send_queue_poll.lock().unwrap_or_else(|p| p.into_inner());
                                                              let total_lines_val = *total_lines_poll.lock().unwrap_or_else(|p| p.into_inner());
                                                              let remaining = queue.len();
                                                              let sent = total_lines_val - remaining;

                                                              // Update progress bar
                                                              if let Some(sb) = status_bar_poll.as_ref() {
                                                                  let progress = if total_lines_val > 0 {
                                                                      (sent as f64 / total_lines_val as f64) * 100.0
                                                                  } else {
                                                                      0.0
                                                                  };

                                                                  // Calculate actual elapsed time
                                                                  let elapsed_secs = if let Some(start) = *job_start_time_poll.lock().unwrap_or_else(|p| p.into_inner()) {
                                                                      start.elapsed().as_secs_f64()
                                                                  } else {
                                                                      0.0
                                                                  };

                                                                  // Estimate remaining time based on average time per line so far
                                                                  let remaining_secs = if sent > 0 && elapsed_secs > 0.0 {
                                                                      let avg_per_line = elapsed_secs / sent as f64;
                                                                      remaining as f64 * avg_per_line
                                                                  } else {
                                                                      0.0
                                                                  };

                                                                  let format_time = |secs: f64| {
                                                                      let h = (secs / 3600.0).floor();
                                                                      let m = ((secs % 3600.0) / 60.0).floor();
                                                                      let s = (secs % 60.0).floor();
                                                                      format!("{:02}:{:02}:{:02}", h, m, s)
                                                                  };

                                                                  sb.set_progress(
                                                                      progress,
                                                                      &format_time(elapsed_secs),
                                                                      &format_time(remaining_secs)
                                                                  );
                                                              }

                                                              if let Some(next_cmd) = queue.pop_front() {
                                                                   if let Some(c) = device_console_poll.as_ref() {
                                                                       c.append_log(&format!("> {}\n", next_cmd));
                                                                   }
                                                                   let _ = comm.send_command(&next_cmd);
                                                                    *waiting_for_ack_poll.lock().unwrap_or_else(|p| p.into_inner()) = true;
                                                              } else {
                                                                   // Done streaming
                                                                   *is_streaming_poll.lock().unwrap_or_else(|p| p.into_inner()) = false;
                                                                   *is_paused_poll.lock().unwrap_or_else(|p| p.into_inner()) = false;

                                                                   // Don't clear job_start_time yet - wait for machine to be Idle
                                                                   // *job_start_time_poll.lock().unwrap_or_else(|p| p.into_inner()) = None;

                                                                   if let Some(c) = device_console_poll.as_ref() {
                                                                       c.append_log(&format!("{}\n", t!("Streaming Completed.")));
                                                                   }
                                                                   // Don't reset progress yet
                                                                   // if let Some(sb) = status_bar_poll.as_ref() {
                                                                   //    sb.set_progress(0.0, "", "");
                                                                   // }
                                                              }
                                                         }
                                                }

                                                // Parse GRBL status: <Idle|MPos:0.000,0.000,0.000|...>
                                                if line.starts_with('<') && line.ends_with('>') {
                                                    // Update machine state
                                                    if let Some(state) = StatusParser::parse_machine_state(&line) {
                                                        conn_status_state_poll.set_text(&format!(
                                                            "{} {}",
                                                            t!("State:"),
                                                            state
                                                        ));
                                                        state_label_poll.set_text(&state);

                                                        for cls in [
                                                            "state-idle",
                                                            "state-run",
                                                            "state-hold",
                                                            "state-alarm",
                                                        ] {
                                                            state_label_poll.remove_css_class(cls);
                                                        }
                                                        let low = state.to_lowercase();
                                                        let cls = if low.starts_with("alarm") {
                                                            "state-alarm"
                                                        } else if low.starts_with("run") {
                                                            "state-run"
                                                        } else if low.starts_with("hold") {
                                                            "state-hold"
                                                        } else {
                                                            "state-idle"
                                                        };
                                                        state_label_poll.add_css_class(cls);

                                                        device_status::update_state(state.clone());

                                                        // Update StatusBar with state
                                                        if let Some(sb) = status_bar_poll.as_ref() {
                                                            sb.set_state(&state);
                                                        }

                                                        // If machine is Idle and not streaming, clear job timer
                                                        if state == "Idle" && !*is_streaming_poll.lock().unwrap_or_else(|p| p.into_inner()) {
                                                            let mut start_time = job_start_time_poll.lock().unwrap_or_else(|p| p.into_inner());
                                                            if start_time.is_some() {
                                                                *start_time = None;
                                                                if let Some(sb) = status_bar_poll.as_ref() {
                                                                    sb.set_progress(0.0, "", "");
                                                                }
                                                                if let Some(c) = device_console_poll.as_ref() {
                                                                    c.append_log(&format!("{}\n", t!("Job Finished.")));
                                                                }
                                                            }
                                                        }

                                                        // Unlock button only enabled in ALARM state
                                                        let is_alarm = low.starts_with("alarm");
                                                        unlock_btn_poll.set_sensitive(is_alarm);
                                                        if is_alarm {
                                                            disabled_reason_label_poll
                                                                .set_text(&t!("ALARM: Unlock required."));
                                                        } else {
                                                            disabled_reason_label_poll.set_text("");
                                                        }
                                                    }

                                                    let full_status = StatusParser::parse_full(&line);

                                                    // Update machine position (MPos)
                                                    if let Some(mpos) = full_status.mpos {
                                                        let units = *current_units_poll.lock().unwrap_or_else(|p| p.into_inner());
                                                        let unit_label = gcodekit5_core::units::get_unit_label(units);

                                                        world_x_poll.set_text(&format!(
                                                            "MX: {} {}",
                                                            format_length(mpos.x as f32, units),
                                                            unit_label
                                                        ));
                                                        world_y_poll.set_text(&format!(
                                                            "MY: {} {}",
                                                            format_length(mpos.y as f32, units),
                                                            unit_label
                                                        ));
                                                        world_z_poll.set_text(&format!(
                                                            "MZ: {} {}",
                                                            format_length(mpos.z as f32, units),
                                                            unit_label
                                                        ));

                                                        device_status::update_machine_position(mpos);

                                                        // Update StatusBar with position
                                                        if let Some(sb) = status_bar_poll.as_ref() {
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

                                                    // Update work position (WPos)
                                                    // WPos is either reported directly by GRBL or derived from MPos-WCO
                                                    // parse_full() automatically derives it when possible
                                                    if let Some(wpos) = full_status.wpos {
                                                        let units = *current_units_poll.lock().unwrap_or_else(|p| p.into_inner());
                                                        let unit_label = gcodekit5_core::units::get_unit_label(units);

                                                        // Update DRO (Digital ReadOut) with work coordinates
                                                        x_dro_poll.set_text(&format!(
                                                            "{} {}",
                                                            format_length(wpos.x as f32, units),
                                                            unit_label
                                                        ));
                                                        y_dro_poll.set_text(&format!(
                                                            "{} {}",
                                                            format_length(wpos.y as f32, units),
                                                            unit_label
                                                        ));
                                                        z_dro_poll.set_text(&format!(
                                                            "{} {}",
                                                            format_length(wpos.z as f32, units),
                                                            unit_label
                                                        ));

                                                        device_status::update_work_position(wpos);

                                                        // Update Visualizer with WORK position (not machine position!)
                                                        // Users work in work coordinates, so visualizer should show WPos
                                                        if let Some(vis) = visualizer_poll.as_ref() {
                                                            vis.set_current_position(wpos.x as f32, wpos.y as f32, wpos.z as f32);
                                                        }
                                                    }

                                                    // Update work coordinate offset - cache it for WPos calculation
                                                    if let Some(wco) = full_status.wco {
                                                        last_wco = Some(wco);
                                                        device_status::update_work_coordinate_offset(wco);
                                                    }

                                                    // If we didn't get WPos from GRBL, but we have MPos and cached WCO, derive it now
                                                    if full_status.wpos.is_none() && full_status.mpos.is_some() && last_wco.is_some() {
                                                        if let (Some(mpos), Some(wco)) = (full_status.mpos, last_wco) {
                                                            use gcodekit5_communication::firmware::grbl::status_parser::StatusParser;
                                                            let wpos = StatusParser::wpos_from_mpos_wco(mpos, wco);

                                                            let units = *current_units_poll.lock().unwrap_or_else(|p| p.into_inner());
                                                            let unit_label = gcodekit5_core::units::get_unit_label(units);

                                                            // Update DRO with derived work coordinates
                                                            x_dro_poll.set_text(&format!(
                                                                "{} {}",
                                                                format_length(wpos.x as f32, units),
                                                                unit_label
                                                            ));
                                                            y_dro_poll.set_text(&format!(
                                                                "{} {}",
                                                                format_length(wpos.y as f32, units),
                                                                unit_label
                                                            ));
                                                            z_dro_poll.set_text(&format!(
                                                                "{} {}",
                                                                format_length(wpos.z as f32, units),
                                                                unit_label
                                                            ));

                                                            device_status::update_work_position(wpos);

                                                            // Update Visualizer with derived work position
                                                            if let Some(vis) = visualizer_poll.as_ref() {
                                                                vis.set_current_position(wpos.x as f32, wpos.y as f32, wpos.z as f32);
                                                            }
                                                        }
                                                    }

                                                    // Update buffer state
                                                    if let Some(buffer) = full_status.buffer {
                                                        state_buffer_label_poll.set_text(&format!(
                                                            "{} {}/{}",
                                                            t!("Buffer:"),
                                                            buffer.plan,
                                                            buffer.rx
                                                        ));
                                                        device_status::update_buffer_state(buffer);
                                                    }

                                                    // Update overrides
                                                    let (feed_ov, spindle_ov) = if let Some(ov) = full_status.overrides {
                                                        if let Ok(mut last) = last_overrides_poll.lock() {
                                                            *last = ov;
                                                        }
                                                        (ov.feed, ov.spindle)
                                                    } else if let Ok(last) = last_overrides_poll.lock() {
                                                        (last.feed, last.spindle)
                                                    } else {
                                                        (100, 100)
                                                    };

                                                    // Update feed/spindle state
                                                    // Note: Feed rate and spindle speed are only reported if $10 bit 3 is set (FS field)
                                                    // Default $10 is often 1 or 3, which doesn't include FS. Users may need $10=15 for full status.

                                                    // Handle feed rate even if spindle is None
                                                    // Prefer commanded feed rate if available, otherwise use reported feed rate
                                                    let commanded_feed = device_status::DEVICE_STATUS.read().ok().and_then(|s| s.commanded_feed_rate);
                                                    let reported_feed = full_status.feed_rate.map(|f| f as f32);
                                                    // If we have a commanded feed rate, use it. If not, fall back to reported.
                                                    // However, if reported is 0 (idle), we might still want to show the last commanded?
                                                    // The user wants to see "Commanded" rather than "Actual" which fluctuates.
                                                    // So if we have a commanded value, use it.
                                                    let display_feed = commanded_feed.or(reported_feed);

                                                    if let Some(feed_rate) = display_feed {
                                                        let units = *current_feed_units_poll.lock().unwrap_or_else(|p| p.into_inner());
                                                        let feed = format_feed_rate(feed_rate, units);
                                                        state_feed_label_poll.set_text(&format!(
                                                            "{} {} {}",
                                                            t!("Feed:"),
                                                            feed,
                                                            units
                                                        ));

                                                        // Calculate adjusted feed
                                                        let adjusted_feed = feed_rate * (feed_ov as f32 / 100.0);
                                                        let adjusted_feed_str = format_feed_rate(adjusted_feed, units);

                                                        let mut text = format!("{} ({}%)", adjusted_feed_str, feed_ov);
                                                        if feed_ov >= 200 { text.push_str(" MAX"); }
                                                        if feed_ov <= 10 { text.push_str(" MIN"); }

                                                        feed_value_poll.set_text(&text);
                                                        if feed_ov >= 200 || feed_ov <= 10 {
                                                            feed_value_poll.add_css_class("error");
                                                        } else {
                                                            feed_value_poll.remove_css_class("error");
                                                        }
                                                    }

                                                    // Handle spindle speed even if feed is None
                                                    let commanded_spindle = device_status::DEVICE_STATUS.read().ok().and_then(|s| s.commanded_spindle_speed);
                                                    let reported_spindle = full_status.spindle_speed.map(|s| s as f32);
                                                    let display_spindle = commanded_spindle.or(reported_spindle);

                                                    if let Some(spindle_speed) = display_spindle {
                                                        state_spindle_label_poll.set_text(&format!(
                                                            "{} {} S",
                                                            t!("Spindle:"),
                                                            spindle_speed
                                                        ));

                                                        // Calculate adjusted spindle
                                                        let adjusted_spindle = spindle_speed * (spindle_ov as f32 / 100.0);

                                                        let mut text = format!("{:.0} S ({}%)", adjusted_spindle, spindle_ov);
                                                        if spindle_ov >= 200 { text.push_str(" MAX"); }
                                                        if spindle_ov <= 50 { text.push_str(" MIN"); }

                                                        spindle_value_poll.set_text(&text);
                                                        if spindle_ov >= 200 || spindle_ov <= 50 {
                                                            spindle_value_poll.add_css_class("error");
                                                        } else {
                                                            spindle_value_poll.remove_css_class("error");
                                                        }
                                                    }

                                                    // Update status bar and device_status if we have both
                                                    if let (Some(feed_rate), Some(spindle_speed)) = (display_feed, display_spindle) {
                                                        let units = *current_feed_units_poll.lock().unwrap_or_else(|p| p.into_inner());
                                                        let feed_spindle = FeedSpindleState {
                                                            feed_rate: feed_rate as f64,
                                                            spindle_speed: spindle_speed as u32,
                                                        };

                                                        // Update status bar with feed/spindle
                                                        if let Some(sb) = status_bar_poll.as_ref() {
                                                            sb.set_feed_spindle(feed_rate as f64, spindle_speed as u32, units);
                                                        }

                                                        device_status::update_feed_spindle_state(feed_spindle);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Send status query every ~250ms (every 5 cycles of 50ms)
                                    if query_counter.is_multiple_of(5) {
                                        let _ = comm.send(b"?");
                                    }
                                }

                                glib::ControlFlow::Continue
                            });
                        }
                        Err(e) => {
                            view_clone.connect_btn.set_sensitive(true);
                            view_clone.state_label.set_text(&t!("Disconnected"));
                            view_clone.conn_status_state.set_text(&t!("State: Disconnected"));
                            view_clone.disabled_reason_label.set_text(&t!("Connect to enable controls."));

                            // Log error to device console
                            if let Some(ref console) = view_clone.device_console {
                                console.append_log(&format!("{}: {}\n", t!("Error connecting"), e));
                            }
                        }
                    }
                }
            }
        });

        // Setup override handlers
        Self::setup_override_handlers(&view);

        view
    }
}

mod operations;
mod overrides;
