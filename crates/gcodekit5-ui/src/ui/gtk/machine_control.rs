use gcodekit5_communication::firmware::grbl::status_parser::{FeedSpindleState, StatusParser};
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
    ComboBoxText, EventControllerKey, Grid, Image, Label, Orientation, Overlay, Paned, PolicyType,
    ScrolledWindow, SizeGroup, SizeGroupMode, ToggleButton,
};
use std::cell::Cell;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::device_status;
use crate::t;
use crate::ui::gtk::device_console::DeviceConsoleView;
use crate::ui::gtk::editor::GcodeEditor;
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
    pub reset_g53_btn: Button,
    pub wcs_btns: Vec<ToggleButton>,
    pub x_dro: Label,
    pub y_dro: Label,
    pub z_dro: Label,
    pub x_zero_btn: Button,
    pub y_zero_btn: Button,
    pub z_zero_btn: Button,
    pub zero_all_btn: Button,
    pub goto_zero_btn: Button,
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
        fn set_controls_enabled(
            send_btn: &Button,
            stop_btn: &Button,
            pause_btn: &Button,
            resume_btn: &Button,
            home_btn: &Button,
            unlock_btn: &Button,
            reset_g53_btn: &Button,
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
            reset_g53_btn.set_sensitive(enabled);
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

        // Setup / Job grouping
        let setup_title = Label::new(Some(&t!("Setup")));
        setup_title.add_css_class("mc-group-title");
        setup_title.set_halign(Align::Start);
        setup_title.set_margin_top(4);
        sidebar.append(&setup_title);

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

        let reset_g53_btn =
            make_icon_label_button("bookmark-new-symbolic", &t!("Use G53 (Machine Coords)"));
        reset_g53_btn.set_tooltip_text(Some(&t!(
            "Send G53: use machine coordinates for the next move (non-modal)"
        )));
        wcs_box.append(&reset_g53_btn);

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
            wcs_grid.attach(&btn, (i % 3) as i32, (i / 3) as i32, 1, 1);
        }
        if let Some(first) = wcs_btns.first() {
            first.set_active(true);
        }
        wcs_box.append(&wcs_grid);
        sidebar.append(&make_section(&t!("Work Coordinates"), &wcs_box));

        let job_title = Label::new(Some(&t!("Job")));
        job_title.add_css_class("mc-group-title");
        job_title.set_halign(Align::Start);
        job_title.set_margin_top(6);
        sidebar.append(&job_title);

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

        // DRO Section
        let dro_box = Box::new(Orientation::Vertical, 4);
        dro_box.set_hexpand(true);
        dro_box.set_halign(Align::Center);

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

        let zero_actions = Box::new(Orientation::Horizontal, 8);
        zero_actions.set_halign(Align::Center);
        zero_actions.set_margin_top(6);

        let zero_all_btn = make_icon_label_button("edit-clear-symbolic", &t!("Zero All Axes"));
        zero_all_btn.set_tooltip_text(Some(&t!("Set active work position to 0 for X/Y/Z")));
        zero_all_btn.add_css_class("destructive-action");
        zero_actions.append(&zero_all_btn);

        let goto_zero_btn = make_icon_label_button("go-jump-symbolic", &t!("Go to Work Zero"));
        goto_zero_btn.set_tooltip_text(Some(&t!("Rapid move to work origin (G0 X0 Y0)")));
        zero_actions.append(&goto_zero_btn);

        dro_box.append(&zero_actions);

        // Machine Coordinates
        let world_box = Box::new(Orientation::Vertical, 4);
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
        dro_box.append(&world_box);

        main_area.append(&dro_box);

        // Jog Controls
        let jog_area = Box::new(Orientation::Vertical, 12);
        jog_area.set_halign(Align::Center);
        jog_area.set_margin_top(16);

        let jog_step_mm = Arc::new(Mutex::new(1.0_f32));
        let jog_feed_mm_per_min = Arc::new(Mutex::new(2000.0_f32));

        let step_box = Box::new(Orientation::Horizontal, 10);
        step_box.set_halign(Align::Center);

        let step_label = Label::new(None);
        step_label.add_css_class("dim-label");
        step_box.append(&step_label);

        let step_combo = ComboBoxText::new();
        step_combo.set_width_request(180);
        step_box.append(&step_combo);

        let feed_box = Box::new(Orientation::Horizontal, 10);
        feed_box.set_halign(Align::Center);

        let feed_label = Label::new(Some(&t!("Jog Feed:")));
        feed_label.add_css_class("dim-label");
        feed_box.append(&feed_label);

        let jog_feed_entry = gtk4::Entry::new();
        jog_feed_entry.set_width_chars(8);
        feed_box.append(&jog_feed_entry);

        let jog_feed_units = Label::new(None);
        jog_feed_units.add_css_class("dim-label");
        feed_box.append(&jog_feed_units);

        jog_area.append(&step_box);
        jog_area.append(&feed_box);

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

        console_header.append(&console_title);
        console_header.append(&console_clear_btn);
        console_header.append(&console_copy_err_btn);
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
            *jog_step_mm.lock().unwrap() = 1.0;

            jog_feed_units.set_text(&initial_feed_units.to_string());
            jog_feed_entry.set_text(&format_feed_rate(2000.0, initial_feed_units));
            *jog_feed_mm_per_min.lock().unwrap() = 2000.0;
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

                    let selected_mm = *jog_step_mm_clone.lock().unwrap();
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
                    *feed_units_clone.lock().unwrap() = new_units;
                    jog_feed_units_clone.set_text(&new_units.to_string());
                    let current_mm_per_min = *jog_feed_mm_per_min_clone.lock().unwrap();
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
            reset_g53_btn,
            wcs_btns,
            x_dro,
            y_dro,
            z_dro,
            x_zero_btn,
            y_zero_btn,
            z_zero_btn,
            zero_all_btn,
            goto_zero_btn,
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
        };

        // Keep internal jog values in base units (mm, mm/min)
        {
            let jog_step_mm = view.jog_step_mm.clone();
            view.step_combo.connect_changed(move |c| {
                if let Some(id) = c.active_id() {
                    if let Ok(v) = id.parse::<f32>() {
                        *jog_step_mm.lock().unwrap() = v;
                    }
                }
            });
        }

        {
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let current_feed_units = view.current_feed_units.clone();
            view.jog_feed_entry.connect_changed(move |e| {
                let units = *current_feed_units.lock().unwrap();
                if let Ok(v) = parse_feed_rate(&e.text(), units) {
                    *jog_feed_mm_per_min.lock().unwrap() = v;
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
            &view.reset_g53_btn,
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
        ) {
            let jog_cmd = format!("$J=G91 {axis}{delta} F{feed_mm_per_min}\n");
            if let Ok(mut comm) = communicator.lock() {
                let _ = comm.send(jog_cmd.as_bytes());
            }
        }

        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            view.jog_x_pos.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap();
                let feed = *jog_feed_mm_per_min.lock().unwrap();
                send_jog('X', step, &communicator, feed);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            view.jog_x_neg.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap();
                let feed = *jog_feed_mm_per_min.lock().unwrap();
                send_jog('X', -step, &communicator, feed);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            view.jog_y_pos.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap();
                let feed = *jog_feed_mm_per_min.lock().unwrap();
                send_jog('Y', step, &communicator, feed);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            view.jog_y_neg.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap();
                let feed = *jog_feed_mm_per_min.lock().unwrap();
                send_jog('Y', -step, &communicator, feed);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            view.jog_z_pos.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap();
                let feed = *jog_feed_mm_per_min.lock().unwrap();
                send_jog('Z', step, &communicator, feed);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            view.jog_z_neg.connect_clicked(move |_| {
                let step = *jog_step_mm.lock().unwrap();
                let feed = *jog_feed_mm_per_min.lock().unwrap();
                send_jog('Z', -step, &communicator, feed);
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

            controller.connect_key_pressed(move |_, key, _, _| {
                if let Some(entry) = console_entry.as_ref() {
                    if entry.has_focus() {
                        return glib::Propagation::Proceed;
                    }
                }

                let Some(ch) = key.to_unicode() else {
                    return glib::Propagation::Proceed;
                };

                let step = *jog_step_mm.lock().unwrap();
                let feed = *jog_feed_mm_per_min.lock().unwrap();

                match ch {
                    '8' => send_jog('Y', step, &communicator, feed),
                    '2' => send_jog('Y', -step, &communicator, feed),
                    '4' => send_jog('X', -step, &communicator, feed),
                    '6' => send_jog('X', step, &communicator, feed),
                    '9' => send_jog('Z', step, &communicator, feed),
                    '3' => send_jog('Z', -step, &communicator, feed),
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
            view.pause_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(b"!");
                }
                *is_paused.lock().unwrap() = true;
            });
        }
        {
            let communicator = view.communicator.clone();
            let is_paused = view.is_paused.clone();
            let is_streaming = view.is_streaming.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let send_queue = view.send_queue.clone();

            view.resume_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(b"~");
                }
                *is_paused.lock().unwrap() = false;

                // Kickstart if stalled (streaming, not waiting for ack, and has commands)
                if *is_streaming.lock().unwrap() && !*waiting_for_ack.lock().unwrap() {
                    let mut queue = send_queue.lock().unwrap();
                    if let Some(cmd) = queue.pop_front() {
                        if let Ok(mut comm) = communicator.lock() {
                            let _ = comm.send_command(&cmd);
                            *waiting_for_ack.lock().unwrap() = true;
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
            view.stop_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    // 0x18 = Ctrl-x = Soft Reset
                    let _ = comm.send(&[0x18]);
                }
                *is_streaming.lock().unwrap() = false;
                *is_paused.lock().unwrap() = false;
                *waiting_for_ack.lock().unwrap() = false;
                send_queue.lock().unwrap().clear();

                // Reset progress
                if let Some(sb) = status_bar.as_ref() {
                    sb.set_progress(0.0, "", "");
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let editor = view.editor.clone();
            let send_queue = view.send_queue.clone();
            let is_streaming = view.is_streaming.clone();
            let is_paused = view.is_paused.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let total_lines = view.total_lines.clone();
            let console = view.device_console.clone();

            view.send_btn.connect_clicked(move |_| {
                if *is_streaming.lock().unwrap() {
                    return;
                }

                let mut content = String::new();
                if let Some(ed) = editor.as_ref() {
                    content = ed.get_text();
                } else {
                }

                if content.trim().is_empty() {
                    let dialog = gtk4::MessageDialog::builder()
                        .message_type(gtk4::MessageType::Error)
                        .buttons(gtk4::ButtonsType::Ok)
                        .text(&t!("No G-Code to Send"))
                        .secondary_text(&t!("Please load or type G-Code into the editor first."))
                        .build();
                    dialog.connect_response(|d, _| d.close());
                    dialog.show();
                    return;
                }

                let lines: Vec<String> = content
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty() && !s.starts_with(';') && !s.starts_with('('))
                    .collect();

                if lines.is_empty() {
                    if let Some(c) = console.as_ref() {
                        c.append_log(&format!("{}\n", t!("No valid G-Code lines found.")));
                    }
                    return;
                }

                {
                    let mut queue = send_queue.lock().unwrap();
                    queue.clear();
                    for line in lines.iter() {
                        queue.push_back(line.clone());
                    }
                    *total_lines.lock().unwrap() = queue.len();
                }

                *is_streaming.lock().unwrap() = true;
                *is_paused.lock().unwrap() = false;
                *waiting_for_ack.lock().unwrap() = false;

                // Kickstart
                if let Ok(mut comm) = communicator.lock() {
                    let mut queue = send_queue.lock().unwrap();
                    if let Some(cmd) = queue.pop_front() {
                        let _ = comm.send_command(&cmd);
                        *waiting_for_ack.lock().unwrap() = true;
                    } else {
                    }
                } else {
                }
            });
        }

        // Machine State Controls
        {
            let communicator = view.communicator.clone();
            view.home_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("$H");
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            view.unlock_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("$X");
                }
            });
        }

        // WCS Controls
        {
            let communicator = view.communicator.clone();
            view.reset_g53_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("G53");
                }
            });
        }

        for (i, btn) in view.wcs_btns.iter().enumerate() {
            let communicator = view.communicator.clone();
            let cmd = format!("G{}", 54 + i);
            btn.connect_toggled(move |b| {
                if !b.is_active() {
                    return;
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
            view.x_zero_btn.connect_clicked(move |_| {
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} X0");
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(&cmd);
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            view.y_zero_btn.connect_clicked(move |_| {
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} Y0");
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(&cmd);
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            view.z_zero_btn.connect_clicked(move |_| {
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} Z0");
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(&cmd);
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            view.zero_all_btn.connect_clicked(move |_| {
                let dialog = gtk4::MessageDialog::builder()
                    .message_type(gtk4::MessageType::Question)
                    .buttons(gtk4::ButtonsType::YesNo)
                    .text(&t!("Zero all work axes?"))
                    .secondary_text(
                        &t!("This sets the active work coordinate system so the current X/Y/Z becomes 0."),
                    )
                    .build();

                let communicator = communicator.clone();
                let wcs_btns = wcs_btns.clone();
                dialog.connect_response(move |d, resp| {
                    if resp == gtk4::ResponseType::Yes {
                        let p = wcs_btns
                            .iter()
                            .position(|b| b.is_active())
                            .map(|i| i + 1)
                            .unwrap_or(1);
                        let cmd = format!("G10 L20 P{p} X0 Y0 Z0");
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
            view.goto_zero_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("G0 X0 Y0");
                }
            });
        }

        // E-Stop
        {
            let communicator = view.communicator.clone();
            let is_streaming = view.is_streaming.clone();
            let is_paused = view.is_paused.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let send_queue = view.send_queue.clone();
            let status_bar = view.status_bar.clone();
            let device_console = view.device_console.clone();

            view.estop_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x18]);
                }

                *is_streaming.lock().unwrap() = false;
                *is_paused.lock().unwrap() = false;
                *waiting_for_ack.lock().unwrap() = false;
                send_queue.lock().unwrap().clear();

                // Reset progress
                if let Some(sb) = status_bar.as_ref() {
                    sb.set_progress(0.0, "", "");
                }

                // Match StatusBar eStop behavior
                if let Some(c) = device_console.as_ref() {
                    c.append_log(&format!("{}\n", t!("Emergency stop (Ctrl-X)")));
                }
            });
        }

        let view_clone = view.clone();
        view.connect_btn.connect_clicked(move |_| {
            let is_connected = view_clone.communicator.lock().unwrap().is_connected();

            if is_connected {
                // Disconnect
                let mut comm = view_clone.communicator.lock().unwrap();
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
                            &view_clone.reset_g53_btn,
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
                    let result = view_clone.communicator.lock().unwrap().connect(&params);

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
                                sb.set_connected(true, &port_name.to_string());
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
                                &view_clone.reset_g53_btn,
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
                                let _ = comm.send(&[0x18]); // Ctrl-X (soft reset)
                            }

                            // Delay info/settings queries a bit to let the controller reboot.
                            {
                                let communicator_init = view_clone.communicator.clone();
                                glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
                                    if let Ok(mut comm) = communicator_init.try_lock() {
                                        if comm.is_connected() {
                                            let _ = comm.send_command("$I");
                                        }
                                    }
                                    glib::ControlFlow::Break
                                });
                            }

                            {
                                let communicator_init = view_clone.communicator.clone();
                                glib::timeout_add_local(std::time::Duration::from_millis(800), move || {
                                    if let Ok(mut comm) = communicator_init.try_lock() {
                                        if comm.is_connected() {
                                            let _ = comm.send_command("$$");
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
                            let widget_poll = view_clone.widget.clone();

                            let mut query_counter = 0u32;
                            let mut response_buffer = String::new();
                            let mut firmware_detected = false;

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

                                                            let secondary = format!("error:{} - {}", code, decoded);

                                                            let dialog = gtk4::MessageDialog::builder()
                                                                .message_type(gtk4::MessageType::Error)
                                                                .buttons(gtk4::ButtonsType::Ok)
                                                                .text(&t!("Controller error"))
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

                                                // Log to console, filtering out status reports and 'ok' acks to avoid spam
                                                if !line.starts_with('<') && line != "ok" {
                                                    if let Some(c) = device_console_poll.as_ref() {
                                                        c.append_log(&format!("{}\n", line_for_log));
                                                    }
                                                }

                                                if is_ack || is_error {
                                                     *waiting_for_ack_poll.lock().unwrap() = false;

                                                     // If error, we might want to stop, but for now we continue
                                                     // if is_error { ... logic to stop ... }

                                                     if *is_streaming_poll.lock().unwrap() {
                                                         if !*is_paused_poll.lock().unwrap() {
                                                              let mut queue = send_queue_poll.lock().unwrap();
                                                              let total_lines_val = *total_lines_poll.lock().unwrap();
                                                              let remaining = queue.len();
                                                              let sent = total_lines_val - remaining;

                                                              // Update progress bar
                                                              if let Some(sb) = status_bar_poll.as_ref() {
                                                                  let progress = if total_lines_val > 0 {
                                                                      (sent as f64 / total_lines_val as f64) * 100.0
                                                                  } else {
                                                                      0.0
                                                                  };

                                                                  // Simple time estimation (very rough)
                                                                  // Assuming average 0.1s per command for now
                                                                  let elapsed_secs = sent as f64 * 0.1;
                                                                  let remaining_secs = remaining as f64 * 0.1;

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

                                                                   let _ = comm.send_command(&next_cmd);
                                                                    *waiting_for_ack_poll.lock().unwrap() = true;
                                                              } else {
                                                                   // Done
                                                                   *is_streaming_poll.lock().unwrap() = false;
                                                                   *is_paused_poll.lock().unwrap() = false;
                                                                   if let Some(c) = device_console_poll.as_ref() {
                                                                       c.append_log(&format!("{}\n", t!("Job Completed.")));
                                                                   }
                                                                   // Reset progress
                                                                   if let Some(sb) = status_bar_poll.as_ref() {
                                                                       sb.set_progress(0.0, "", "");
                                                                   }
                                                              }
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
                                                        let units = *current_units_poll.lock().unwrap();
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

                                                        // Update Visualizer with position
                                                        if let Some(vis) = visualizer_poll.as_ref() {
                                                            vis.set_current_position(mpos.x as f32, mpos.y as f32, mpos.z as f32);
                                                        }
                                                    }

                                                    // Update work position (WPos). This may be derived from MPos/WCO when GRBL isn't reporting WPos.
                                                    if let Some(wpos) = full_status.wpos {
                                                        let units = *current_units_poll.lock().unwrap();
                                                        let unit_label = gcodekit5_core::units::get_unit_label(units);

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
                                                    }

                                                    // Update work coordinate offset
                                                    if let Some(wco) = full_status.wco {
                                                        device_status::update_work_coordinate_offset(wco);
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

                                                    // Update feed/spindle state
                                                    if let (Some(feed_rate), Some(spindle_speed)) =
                                                        (full_status.feed_rate, full_status.spindle_speed)
                                                    {
                                                        let feed_spindle = FeedSpindleState {
                                                            feed_rate,
                                                            spindle_speed,
                                                        };
                                                        let units = *current_feed_units_poll.lock().unwrap();
                                                        let feed = format_feed_rate(feed_spindle.feed_rate as f32, units);
                                                        state_feed_label_poll.set_text(&format!(
                                                            "{} {} {}",
                                                            t!("Feed:"),
                                                            feed,
                                                            units
                                                        ));
                                                        state_spindle_label_poll.set_text(&format!(
                                                            "{} {} RPM",
                                                            t!("Spindle:"),
                                                            feed_spindle.spindle_speed
                                                        ));
                                                        device_status::update_feed_spindle_state(feed_spindle);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Send status query every ~250ms (every 5 cycles of 50ms)
                                    if query_counter % 5 == 0 {
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

        view
    }

    pub fn refresh_ports(&self) {
        self.port_combo.remove_all();

        match gcodekit5_communication::list_ports() {
            Ok(ports) if !ports.is_empty() => {
                for port in ports {
                    self.port_combo
                        .append(Some(&port.port_name), &port.port_name);
                }
                // Select the first port
                self.port_combo.set_active(Some(0));
            }
            _ => {
                self.port_combo
                    .append(Some("none"), &t!("No ports available"));
                self.port_combo.set_active_id(Some("none"));
            }
        }
    }

    pub fn get_step_size(&self) -> f64 {
        *self.jog_step_mm.lock().unwrap() as f64
    }
}
