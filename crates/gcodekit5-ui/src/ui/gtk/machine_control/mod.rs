//! # Machine Control Panel
//!
//! UI panel for real-time CNC machine control including jogging,
//! homing, work coordinate setting, spindle/laser control,
//! and job execution (start/pause/stop).

use gcodekit5_communication::firmware::grbl::status_parser::{
    FeedSpindleState, OverrideState, StatusParser, WorkCoordinateOffset,
};
use gcodekit5_communication::{
    Communicator, ConnectionDriver, ConnectionParams, SerialCommunicator,
};
use gcodekit5_core::units::{
    format_feed_rate, format_length, get_unit_label, FeedRateUnits, MeasurementSystem,
};
use gcodekit5_settings::controller::SettingsController;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    accessible::Property as AccessibleProperty, pango::EllipsizeMode, Align, Box, Button,
    CheckButton, ComboBoxText, EventControllerKey, Grid, Image, Label, Orientation, Overlay, Paned,
    PolicyType, ScrolledWindow, SizeGroup, SizeGroupMode, SpinButton, ToggleButton,
};
use std::cell::Cell;

use crate::device_status;
use crate::t;
use crate::ui::gtk::device_console::DeviceConsoleView;
use crate::ui::gtk::editor::GcodeEditor;
use crate::ui::gtk::help_browser;
use crate::ui::gtk::machine_control::continuous_jog::{numpad_jog, ContinuousJogState};
use crate::ui::gtk::machine_control::direct_sender::{DirectSender, JobOutcome, SenderEvent};
use crate::ui::gtk::machine_control::grbl_stream::{
    format_probe_move, jog_step_presets, nearest_jog_step_preset, parse_jog_feed,
};
use crate::ui::gtk::machine_control::status_report::QuickProbeState;
use crate::ui::gtk::status_bar::StatusBar;
use crate::ui::gtk::visualizer::GcodeVisualizer;
use gcodekit5_core::{
    thread_safe, thread_safe_deque, thread_safe_none, ThreadSafe, ThreadSafeDeque, ThreadSafeOption,
};
use std::rc::Rc;

mod connection;
mod continuous_jog;
mod direct_sender;
pub mod grbl_stream;
mod state;
mod status_report;
mod ui_builders;

/// Quick Z probe: signed distance (mm) along Z.
const PROBE_Z_MOVE: (char, f32) = ('Z', -20.0);
/// Quick edge-find probe distance (mm).
const PROBE_EDGE_DISTANCE_MM: f32 = 10.0;
/// Quick probe feed (mm/min).
const PROBE_FEED_MM_PER_MIN: f32 = 100.0;

pub use ui_builders::{create_dro, make_section};

fn set_button_icon_label(btn: &Button, icon: &str, label: &str) {
    ui_builders::set_button_icon_label(btn, icon, label);
}

fn make_icon_label_button(icon: &str, label: &str) -> Button {
    ui_builders::make_icon_label_button(icon, label)
}

fn make_icon_label_toggle(icon: &str, label: &str) -> ToggleButton {
    ui_builders::make_icon_label_toggle(icon, label)
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
    /// Target feed override (%) applied by `feed_set`.
    pub feed_target: SpinButton,
    /// Sets the feed override to exactly `feed_target`, one confirmed step at a time.
    pub feed_set: Button,
    pub spindle_value: Label,
    pub spindle_dec10: Button,
    pub spindle_dec1: Button,
    pub spindle_reset: Button,
    pub spindle_stop: Button,
    pub spindle_inc1: Button,
    pub spindle_inc10: Button,
    /// Target spindle override (%) applied by `spindle_set`.
    pub spindle_target: SpinButton,
    /// Sets the spindle override to exactly `spindle_target`, one confirmed step at a time.
    pub spindle_set: Button,

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
    // Rotary axis DRO labels (A, B, C)
    pub a_dro: Option<Label>,
    pub b_dro: Option<Label>,
    pub c_dro: Option<Label>,
    pub world_a: Option<Label>,
    pub world_b: Option<Label>,
    pub world_c: Option<Label>,
    // Rotary axis DRO containers (for visibility control)
    pub a_dro_container: Option<gtk4::Box>,
    pub b_dro_container: Option<gtk4::Box>,
    pub c_dro_container: Option<gtk4::Box>,
    // Rotary axis jog buttons
    pub jog_a_pos: Option<Button>,
    pub jog_a_neg: Option<Button>,
    pub jog_b_pos: Option<Button>,
    pub jog_b_neg: Option<Button>,
    pub jog_c_pos: Option<Button>,
    pub jog_c_neg: Option<Button>,
    // Rotary axis zero buttons
    pub a_zero_btn: Option<Button>,
    pub b_zero_btn: Option<Button>,
    pub c_zero_btn: Option<Button>,
    // Machine limits display labels
    pub limits_x: Option<Label>,
    pub limits_y: Option<Label>,
    pub limits_z: Option<Label>,
    pub limits_a: Option<Label>,
    pub limits_b: Option<Label>,
    pub limits_c: Option<Label>,
    // Rotary axis limit rows (label + value), shown with their axis
    pub limits_a_row: Option<gtk4::Box>,
    pub limits_b_row: Option<gtk4::Box>,
    pub limits_c_row: Option<gtk4::Box>,
    // Axis count for dynamic UI
    pub axis_count: std::rc::Rc<std::cell::Cell<u8>>,
    // Coolant controls
    pub coolant_mist_btn: Option<Button>,
    pub coolant_flood_btn: Option<Button>,
    pub coolant_off_btn: Option<Button>,
    // Probe controls
    pub probe_single_btn: Option<Button>,
    pub probe_continuous_btn: Option<Button>,
    pub probe_to_surface_btn: Option<Button>,
    // Quick probe buttons (Milestone 6)
    pub probe_z_btn: Option<Button>,
    pub probe_x_neg_btn: Option<Button>,
    pub probe_x_pos_btn: Option<Button>,
    pub probe_y_neg_btn: Option<Button>,
    pub probe_y_pos_btn: Option<Button>,
    pub step_combo: ComboBoxText,
    pub step_label: Label,
    /// When active, holding a jog button or numpad key jogs continuously.
    pub continuous_jog_check: CheckButton,
    /// State of the continuous jog in progress, shared with the status poller.
    pub continuous_jog: ThreadSafe<ContinuousJogState>,
    pub jog_feed_entry: gtk4::Entry,
    pub jog_feed_units: Label,
    pub jog_step_mm: ThreadSafe<f32>,
    pub jog_feed_mm_per_min: ThreadSafe<f32>,
    pub current_feed_units: ThreadSafe<FeedRateUnits>,
    pub jog_x_pos: Button,
    pub jog_x_neg: Button,
    pub jog_y_pos: Button,
    pub jog_y_neg: Button,
    pub jog_z_pos: Button,
    pub jog_z_neg: Button,
    pub estop_btn: Button,
    pub communicator: ThreadSafe<SerialCommunicator>,
    pub status_bar: Option<StatusBar>,
    pub device_console: Option<Rc<DeviceConsoleView>>,
    pub editor: Option<Rc<GcodeEditor>>,
    pub visualizer: Option<Rc<GcodeVisualizer>>,
    pub send_queue: ThreadSafeDeque<String>,
    pub total_lines: ThreadSafe<usize>,
    pub is_streaming: ThreadSafe<bool>,
    pub is_paused: ThreadSafe<bool>,
    pub waiting_for_ack: ThreadSafe<bool>,
    pub current_units: ThreadSafe<MeasurementSystem>,
    pub last_overrides: ThreadSafe<OverrideState>,
    pub job_start_time: ThreadSafeOption<std::time::Instant>,
    pub direct_sender: ThreadSafeOption<DirectSender>,
    /// Last Work Coordinate Offset reported (GRBL only sends WCO in some reports).
    pub(crate) last_wco: Rc<Cell<Option<WorkCoordinateOffset>>>,
    /// Progress of a quick probe started from the panel.
    pub(crate) quick_probe: Rc<Cell<QuickProbeState>>,
    /// Bumped on every connect so a stale status poller stops itself.
    pub(crate) poll_generation: Rc<Cell<u64>>,
    /// Index (0 = G54) of the work coordinate system last selected.
    pub(crate) active_wcs: Rc<Cell<usize>>,
    /// Set while a WCS toggle is being reverted, so the revert sends nothing.
    pub(crate) wcs_reverting: Rc<Cell<bool>>,
    /// Device profiles; the active profile chooses serial vs. network connection.
    pub(crate) device_manager:
        Rc<std::cell::RefCell<Option<std::sync::Arc<gcodekit5_devicedb::DeviceManager>>>>,
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

        use crate::ui::gtk::machine_control::ui_builders::make_section;

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

        // Machine Limits Section (compact display for 6 axes)
        let limits_box = Box::new(Orientation::Vertical, 4);

        // Axis limits grid - shows min/max for each configured axis
        let limits_grid = Grid::new();
        limits_grid.set_column_spacing(8);
        limits_grid.set_row_spacing(4);
        limits_grid.set_halign(Align::Start);

        // Create limit labels for all axes
        let limits_title = Label::new(Some(&t!("Machine Limits:")));
        limits_title.add_css_class("dim-label");
        limits_title.set_halign(Align::Start);
        limits_box.append(&limits_title);

        // X axis limits
        let limits_x_row = Box::new(Orientation::Horizontal, 8);
        let limits_x_label = Label::new(Some("X:"));
        limits_x_label.add_css_class("dim-label");
        let limits_x = Label::new(Some("0.0 - 200.0"));
        limits_x.add_css_class("mc-limits-value");
        limits_x_row.append(&limits_x_label);
        limits_x_row.append(&limits_x);
        limits_grid.attach(&limits_x_row, 0, 0, 1, 1);

        // Y axis limits
        let limits_y_row = Box::new(Orientation::Horizontal, 8);
        let limits_y_label = Label::new(Some("Y:"));
        limits_y_label.add_css_class("dim-label");
        let limits_y = Label::new(Some("0.0 - 200.0"));
        limits_y.add_css_class("mc-limits-value");
        limits_y_row.append(&limits_y_label);
        limits_y_row.append(&limits_y);
        limits_grid.attach(&limits_y_row, 0, 1, 1, 1);

        // Z axis limits
        let limits_z_row = Box::new(Orientation::Horizontal, 8);
        let limits_z_label = Label::new(Some("Z:"));
        limits_z_label.add_css_class("dim-label");
        let limits_z = Label::new(Some("0.0 - 100.0"));
        limits_z.add_css_class("mc-limits-value");
        limits_z_row.append(&limits_z_label);
        limits_z_row.append(&limits_z);
        limits_grid.attach(&limits_z_row, 0, 2, 1, 1);

        // A axis limits (rotary - hidden by default)
        let limits_a_row = Box::new(Orientation::Horizontal, 8);
        let limits_a_label = Label::new(Some("A:"));
        limits_a_label.add_css_class("dim-label");
        let limits_a = Label::new(Some("0.0° - 360.0°"));
        limits_a.add_css_class("mc-limits-value");
        limits_a_row.append(&limits_a_label);
        limits_a_row.append(&limits_a);
        limits_a_row.set_visible(false); // Hidden until axis_count >= 4
        limits_grid.attach(&limits_a_row, 1, 0, 1, 1);

        // B axis limits (rotary - hidden by default)
        let limits_b_row = Box::new(Orientation::Horizontal, 8);
        let limits_b_label = Label::new(Some("B:"));
        limits_b_label.add_css_class("dim-label");
        let limits_b = Label::new(Some("0.0° - 360.0°"));
        limits_b.add_css_class("mc-limits-value");
        limits_b_row.append(&limits_b_label);
        limits_b_row.append(&limits_b);
        limits_b_row.set_visible(false); // Hidden until axis_count >= 5
        limits_grid.attach(&limits_b_row, 1, 1, 1, 1);

        // C axis limits (rotary - hidden by default)
        let limits_c_row = Box::new(Orientation::Horizontal, 8);
        let limits_c_label = Label::new(Some("C:"));
        limits_c_label.add_css_class("dim-label");
        let limits_c = Label::new(Some("0.0° - 360.0°"));
        limits_c.add_css_class("mc-limits-value");
        limits_c_row.append(&limits_c_label);
        limits_c_row.append(&limits_c);
        limits_c_row.set_visible(false); // Hidden until axis_count >= 6
        limits_grid.attach(&limits_c_row, 1, 2, 1, 1);

        limits_box.append(&limits_grid);
        sidebar.append(&make_section(&t!("Machine Limits"), &limits_box));

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
        main_area.set_hexpand(false); // false - No reducir del mínimo
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

        let (feed_target, feed_set) = Self::override_target_widgets(
            &t!("Target feed override (%)"),
            &t!("Set the feed override to exactly the target percentage"),
        );

        feed_controls.append(&feed_dec10);
        feed_controls.append(&feed_dec1);
        feed_controls.append(&feed_reset);
        feed_controls.append(&feed_inc1);
        feed_controls.append(&feed_inc10);
        feed_controls.append(&feed_target);
        feed_controls.append(&feed_set);

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

        let (spindle_target, spindle_set) = Self::override_target_widgets(
            &t!("Target spindle override (%)"),
            &t!("Set the spindle override to exactly the target percentage"),
        );
        spindle_controls.append(&spindle_target);
        spindle_controls.append(&spindle_set);

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

        let create_dro = crate::ui::gtk::machine_control::ui_builders::create_dro;

        let (x_box, x_dro, x_zero_btn) = create_dro("X", "WX");
        let (y_box, y_dro, y_zero_btn) = create_dro("Y", "WY");
        let (z_box, z_dro, z_zero_btn) = create_dro("Z", "WZ");

        dro_box.append(&x_box);
        dro_box.append(&y_box);
        dro_box.append(&z_box);

        // Rotary axis DRO displays (initially hidden, shown based on device capabilities)
        let (a_box, a_dro, a_zero_btn) = create_dro("A", "WA");
        let (b_box, b_dro, b_zero_btn) = create_dro("B", "WB");
        let (c_box, c_dro, c_zero_btn) = create_dro("C", "WC");

        // Hide rotary axis DROs initially
        a_box.set_visible(false);
        b_box.set_visible(false);
        c_box.set_visible(false);

        dro_box.append(&a_box);
        dro_box.append(&b_box);
        dro_box.append(&c_box);
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

        // Quick Probe Z button (Milestone 6)
        let probe_z_btn = make_icon_label_button("input-touchpad-symbolic", &t!("Probe Z"));
        probe_z_btn.set_tooltip_text(Some(&t!("Quick Z-touch probe at current XY position")));
        probe_z_btn.add_css_class("suggested-action");
        zero_actions.append(&probe_z_btn);

        // Quick Edge Find buttons row (Milestone 6)
        let edge_probe_row = Box::new(Orientation::Horizontal, 4);
        edge_probe_row.set_halign(Align::Center);
        edge_probe_row.set_tooltip_text(Some(&t!("Quick edge find in direction")));

        let probe_x_neg_btn = Button::from_icon_name("go-previous-symbolic");
        probe_x_neg_btn.set_tooltip_text(Some(&t!("Probe X- edge")));
        let probe_x_pos_btn = Button::from_icon_name("go-next-symbolic");
        probe_x_pos_btn.set_tooltip_text(Some(&t!("Probe X+ edge")));
        let probe_y_neg_btn = Button::from_icon_name("go-down-symbolic");
        probe_y_neg_btn.set_tooltip_text(Some(&t!("Probe Y- edge")));
        let probe_y_pos_btn = Button::from_icon_name("go-up-symbolic");
        probe_y_pos_btn.set_tooltip_text(Some(&t!("Probe Y+ edge")));

        for btn in [
            &probe_x_neg_btn,
            &probe_x_pos_btn,
            &probe_y_neg_btn,
            &probe_y_pos_btn,
        ] {
            btn.set_size_request(32, 32);
            btn.add_css_class("flat");
        }

        let edge_probe_label = Label::new(Some(&t!("Edge:")));
        edge_probe_label.add_css_class("dim-label");
        edge_probe_label.set_margin_end(4);
        edge_probe_row.append(&edge_probe_label);
        edge_probe_row.append(&probe_x_neg_btn);
        edge_probe_row.append(&probe_x_pos_btn);
        edge_probe_row.append(&probe_y_neg_btn);
        edge_probe_row.append(&probe_y_pos_btn);
        zero_actions.append(&edge_probe_row);
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

        // Rotary axis machine coordinates (initially hidden)
        let world_a = Label::new(Some("MA: 0.000"));
        let world_b = Label::new(Some("MB: 0.000"));
        let world_c = Label::new(Some("MC: 0.000"));
        world_a.set_visible(false);
        world_b.set_visible(false);
        world_c.set_visible(false);
        world_vals.append(&world_a);
        world_vals.append(&world_b);
        world_vals.append(&world_c);
        main_area.append(&dro_container);
        main_area.append(&world_box);

        // Jog Controls (step and feed on same line)
        let jog_area = Box::new(Orientation::Vertical, 8);
        jog_area.set_halign(Align::Center);
        jog_area.set_margin_top(12);

        let jog_step_mm = thread_safe(1.0_f32);
        let jog_feed_mm_per_min = thread_safe(2000.0_f32);

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

        let continuous_jog_check = CheckButton::with_label(&t!("Continuous"));
        continuous_jog_check.set_tooltip_text(Some(&t!(
            "Hold a jog button or numpad key to jog continuously; release to stop. \
             Moves in short increments, so a soft limit stops the jog safely."
        )));
        step_box.append(&continuous_jog_check);

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
        // Layout: [sidebar] | [editor (top) + console (bottom)]
        let main_scroller = ScrolledWindow::new();
        main_scroller.set_hexpand(true);
        main_scroller.set_vexpand(true);
        main_scroller.set_policy(PolicyType::Never, PolicyType::Automatic);
        main_scroller.set_child(Some(&main_area));

        let inner_paned = Paned::new(Orientation::Horizontal);
        inner_paned.set_start_child(Some(&main_scroller));

        // ==========================================
        // PANEL DERECHO DIVIDIDO VERTICALMENTE
        // ==========================================
        let right_vpaned = Paned::new(Orientation::Vertical);
        right_vpaned.set_vexpand(true);
        right_vpaned.set_hexpand(true);

        // ---- Editor de G-code (PARTE SUPERIOR) ----
        let editor_scroller = ScrolledWindow::new();
        editor_scroller.set_policy(PolicyType::Automatic, PolicyType::Automatic);
        editor_scroller.set_vexpand(true);
        editor_scroller.set_hexpand(true);

        if let Some(editor) = &editor {
            editor.widget.set_vexpand(true);
            editor.widget.set_hexpand(true);
            editor_scroller.set_child(Some(&editor.widget));
        } else {
            let placeholder = Label::new(Some(&t!("Editor not available")));
            placeholder.set_halign(Align::Center);
            placeholder.set_valign(Align::Center);
            editor_scroller.set_child(Some(&placeholder));
        }

        // Toolbar para el editor
        let editor_header = Box::new(Orientation::Horizontal, 6);
        editor_header.set_margin_start(6);
        editor_header.set_margin_end(6);
        editor_header.set_margin_top(6);
        let editor_title = Label::new(Some(&t!("G-Code Editor")));
        editor_title.add_css_class("dim-label");
        editor_title.set_halign(Align::Start);
        editor_title.set_hexpand(true);
        editor_header.append(&editor_title);

        let clear_editor_btn = Button::from_icon_name("edit-clear-symbolic");
        clear_editor_btn.set_tooltip_text(Some(&t!("Clear editor")));
        editor_header.append(&clear_editor_btn);

        let save_editor_btn = Button::from_icon_name("document-save-symbolic");
        save_editor_btn.set_tooltip_text(Some(&t!("Save G-code")));
        editor_header.append(&save_editor_btn);

        let editor_container = Box::new(Orientation::Vertical, 0);
        editor_container.set_vexpand(true);
        editor_container.append(&editor_header);
        editor_container.append(&editor_scroller);

        right_vpaned.set_start_child(Some(&editor_container));

        // ---- Consola del dispositivo (PARTE INFERIOR) ----
        let console_container = Box::new(Orientation::Vertical, 10);
        console_container.set_hexpand(true);
        console_container.set_vexpand(true);
        console_container.set_margin_top(12);
        console_container.set_margin_bottom(12);
        console_container.set_margin_start(12);
        console_container.set_margin_end(12);

        // Device Console header
        let console_header = Box::new(Orientation::Horizontal, 6);
        console_header.set_hexpand(true);

        let console_title = Label::new(Some(&t!("Device Console")));
        console_title.add_css_class("mc-section-title");
        console_title.set_halign(Align::Start);
        console_title.set_hexpand(true);

        let console_clear_btn = Button::from_icon_name("user-trash-symbolic");
        console_clear_btn.set_tooltip_text(Some(&t!("Clear console")));
        let console_copy_err_btn = Button::from_icon_name("edit-copy-symbolic");
        console_copy_err_btn.set_tooltip_text(Some(&t!("Copy last error")));
        let help_btn = help_browser::make_help_button("machine_control");

        console_header.append(&console_title);
        console_header.append(&console_clear_btn);
        console_header.append(&console_copy_err_btn);
        console_header.append(&help_btn);

        console_container.append(&console_header);

        // Embed Device Console
        if let Some(ref console_view) = device_console {
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

        right_vpaned.set_end_child(Some(&console_scroller));

        // Configurar porcentaje (70% editor, 30% consola)
        let right_vpaned_sized = Rc::new(Cell::new(false));
        right_vpaned.add_tick_callback({
            let right_vpaned_sized = right_vpaned_sized.clone();
            let right_vpaned = right_vpaned.clone();
            move |_, _| {
                if right_vpaned_sized.get() {
                    return glib::ControlFlow::Break;
                }
                let height = right_vpaned.height();
                if height > 0 {
                    // 70% para el editor (arriba), 30% para la consola (abajo)
                    right_vpaned.set_position((height as f64 * 0.7) as i32);
                    right_vpaned_sized.set(true);
                }
                glib::ControlFlow::Continue
            }
        });

        // Conectar botones del editor
        if let Some(editor) = &editor {
            let editor_clone = editor.clone();
            clear_editor_btn.connect_clicked(move |btn| {
                if editor_clone.get_text().trim().is_empty() {
                    editor_clone.set_text("");
                    return;
                }
                let dialog = gtk4::MessageDialog::builder()
                    .message_type(gtk4::MessageType::Question)
                    .buttons(gtk4::ButtonsType::YesNo)
                    .text(t!("Clear the editor?"))
                    .secondary_text(t!("All G-code in the editor will be removed."))
                    .build();
                if let Some(win) = btn.root().and_downcast::<gtk4::Window>() {
                    dialog.set_transient_for(Some(&win));
                    dialog.set_modal(true);
                }
                let editor = editor_clone.clone();
                dialog.connect_response(move |d, resp| {
                    if resp == gtk4::ResponseType::Yes {
                        editor.set_text("");
                    }
                    d.close();
                });
                dialog.show();
            });

            let editor_clone_save = editor.clone();
            save_editor_btn.connect_clicked(move |_| {
                editor_clone_save.save_file();
            });
        }

        // Conectar botones de consola
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

        inner_paned.set_end_child(Some(&right_vpaned));

        let console_show_panel = Box::new(Orientation::Horizontal, 0);
        console_show_panel.add_css_class("osd");
        console_show_panel.set_halign(Align::End);
        console_show_panel.set_valign(Align::Start);
        console_show_panel.set_margin_end(12);
        console_show_panel.set_margin_top(12);
        console_show_panel.set_visible(false);

        let inner_overlay = Overlay::new();
        inner_overlay.set_child(Some(&inner_paned));
        inner_overlay.add_overlay(&console_show_panel);
        inner_overlay.set_hexpand(true);
        inner_overlay.set_vexpand(true);

        widget.set_start_child(Some(&sidebar_scroller));
        widget.set_end_child(Some(&inner_overlay));

        // Initial sizing (20% left sidebar / 80% main), then let the user resize.
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
                paned.set_position((width as f64 * 0.20) as i32);
                outer_sized.set(true);
                glib::ControlFlow::Break
            }
        });

        // No reducir del tamaño menor definido
        widget.set_shrink_start_child(false);

        let communicator = thread_safe(SerialCommunicator::new());

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
        let current_units = thread_safe(initial_units);

        let initial_feed_units = if let Some(controller) = &settings_controller {
            controller.persistence.borrow().config().ui.feed_rate_units
        } else {
            FeedRateUnits::default()
        };
        let current_feed_units = thread_safe(initial_feed_units);

        // Populate jog UI from initial settings
        {
            let unit_label = get_unit_label(initial_units);
            step_label.set_text(&format!("{} ({unit_label}):", t!("Step")));
            let step_mm = Self::populate_step_presets(&step_combo, initial_units, 1.0);
            *jog_step_mm.lock() = step_mm;

            jog_feed_units.set_text(&initial_feed_units.to_string());
            jog_feed_entry.set_text(&format_feed_rate(2000.0, initial_feed_units));
            *jog_feed_mm_per_min.lock() = 2000.0;
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
            let probe_btns = [
                probe_z_btn.clone(),
                probe_x_neg_btn.clone(),
                probe_x_pos_btn.clone(),
                probe_y_neg_btn.clone(),
                probe_y_pos_btn.clone(),
            ];

            controller.on_setting_changed(move |key, value| {
                if key == "measurement_system" {
                    let new_units = match value {
                        "Imperial" => MeasurementSystem::Imperial,
                        _ => MeasurementSystem::Metric,
                    };
                    {
                        let mut u = units_clone.lock();
                        *u = new_units;
                    }

                    let units = new_units;
                    let unit_label = get_unit_label(units);
                    step_label_clone.set_text(&format!("{} ({unit_label}):", t!("Step")));

                    // Inch mode offers inch presets; the nearest one replaces the
                    // current step. The value stays in mm internally.
                    let selected_mm = *jog_step_mm_clone.lock();
                    let step_mm =
                        Self::populate_step_presets(&step_combo_clone, units, selected_mm);
                    *jog_step_mm_clone.lock() = step_mm;
                    Self::update_probe_tooltips(&probe_btns, units, *feed_units_clone.lock());
                }

                if key == "feed_rate_units" {
                    let new_units = match value {
                        "mm/sec" => FeedRateUnits::MmPerSec,
                        "in/min" => FeedRateUnits::InPerMin,
                        "in/sec" => FeedRateUnits::InPerSec,
                        _ => FeedRateUnits::MmPerMin,
                    };
                    *feed_units_clone.lock() = new_units;
                    Self::update_probe_tooltips(&probe_btns, *units_clone.lock(), new_units);
                    jog_feed_units_clone.set_text(&new_units.to_string());
                    let current_mm_per_min = *jog_feed_mm_per_min_clone.lock();
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
            feed_target,
            feed_set,
            spindle_value,
            spindle_dec10,
            spindle_dec1,
            spindle_reset,
            spindle_stop,
            spindle_inc1,
            spindle_inc10,
            spindle_target,
            spindle_set,

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
            // Rotary axis DRO and machine coordinates (initially None)
            a_dro: Some(a_dro),
            b_dro: Some(b_dro),
            c_dro: Some(c_dro),
            world_a: Some(world_a),
            world_b: Some(world_b),
            world_c: Some(world_c),
            // Rotary axis DRO containers (for visibility control)
            a_dro_container: Some(a_box),
            b_dro_container: Some(b_box),
            c_dro_container: Some(c_box),
            // Rotary axis jog buttons (initially None)
            jog_a_pos: None,
            jog_a_neg: None,
            jog_b_pos: None,
            jog_b_neg: None,
            jog_c_pos: None,
            jog_c_neg: None,
            // Rotary axis zero buttons
            a_zero_btn: Some(a_zero_btn),
            b_zero_btn: Some(b_zero_btn),
            c_zero_btn: Some(c_zero_btn),
            // Machine limits display labels (initially None)
            limits_x: Some(limits_x),
            limits_y: Some(limits_y),
            limits_z: Some(limits_z),
            limits_a: Some(limits_a),
            limits_b: Some(limits_b),
            limits_c: Some(limits_c),
            limits_a_row: Some(limits_a_row),
            limits_b_row: Some(limits_b_row),
            limits_c_row: Some(limits_c_row),
            // Axis count for dynamic UI
            axis_count: std::rc::Rc::new(std::cell::Cell::new(3)),
            // Coolant controls (initially None)
            coolant_mist_btn: None,
            coolant_flood_btn: None,
            coolant_off_btn: None, // Probe controls (initially None)
            probe_single_btn: None,
            probe_continuous_btn: None,
            probe_to_surface_btn: None,
            // Quick probe buttons (Milestone 6)
            probe_z_btn: Some(probe_z_btn.clone()),
            probe_x_neg_btn: Some(probe_x_neg_btn.clone()),
            probe_x_pos_btn: Some(probe_x_pos_btn.clone()),
            probe_y_neg_btn: Some(probe_y_neg_btn.clone()),
            probe_y_pos_btn: Some(probe_y_pos_btn.clone()),
            step_combo,
            step_label,
            continuous_jog_check,
            continuous_jog: thread_safe(ContinuousJogState::default()),
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
            send_queue: thread_safe_deque(),
            total_lines: thread_safe(0),
            is_streaming: thread_safe(false),
            is_paused: thread_safe(false),
            waiting_for_ack: thread_safe(false),
            current_units,
            last_overrides: thread_safe(OverrideState {
                feed: 100,
                rapid: 100,
                spindle: 100,
            }),
            job_start_time: thread_safe_none(),
            direct_sender: thread_safe_none(),
            last_wco: Rc::new(Cell::new(None)),
            quick_probe: Rc::new(Cell::new(QuickProbeState::Idle)),
            poll_generation: Rc::new(Cell::new(0)),
            active_wcs: Rc::new(Cell::new(0)),
            wcs_reverting: Rc::new(Cell::new(false)),
            device_manager: Rc::new(std::cell::RefCell::new(None)),
        };

        Self::update_probe_tooltips(
            &[
                probe_z_btn.clone(),
                probe_x_neg_btn.clone(),
                probe_x_pos_btn.clone(),
                probe_y_neg_btn.clone(),
                probe_y_pos_btn.clone(),
            ],
            initial_units,
            initial_feed_units,
        );
        view.update_axis_visibility(device_status::get_active_num_axes());

        // Keep internal jog values in base units (mm, mm/min)
        {
            let jog_step_mm = view.jog_step_mm.clone();
            view.step_combo.connect_changed(move |c| {
                if let Some(id) = c.active_id() {
                    if let Ok(v) = id.parse::<f32>() {
                        *jog_step_mm.lock() = v;
                    }
                }
            });
        }

        {
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let current_feed_units = view.current_feed_units.clone();
            view.jog_feed_entry.connect_changed(move |e| {
                let units = *current_feed_units.lock();
                match parse_jog_feed(&e.text(), units) {
                    Ok(v) => {
                        *jog_feed_mm_per_min.lock() = v;
                        e.remove_css_class("error");
                        e.set_tooltip_text(None);
                    }
                    Err(err) => {
                        // Keep the last valid feed, but make the problem visible.
                        if !e.has_css_class("error") {
                            tracing::warn!(
                                entry = %e.text(),
                                kept_mm_per_min = *jog_feed_mm_per_min.lock(),
                                "Invalid jog feed entry: {err}"
                            );
                        }
                        e.add_css_class("error");
                        e.set_tooltip_text(Some(&format!(
                            "{} {:.2} {}",
                            t!("Invalid jog feed; still using"),
                            format_feed_rate(*jog_feed_mm_per_min.lock(), units)
                                .parse::<f32>()
                                .unwrap_or_default(),
                            units
                        )));
                    }
                }
            });
        }

        view.refresh_ports();

        // Disable all controls initially (until connected)
        view.set_connection_controls(false);
        // Setup jog button handlers
        /// Returns true (and logs why) when a job is streaming, so manual motion /
        /// offset commands are not interleaved with the job's G-code.
        fn job_running(
            is_streaming: &ThreadSafe<bool>,
            console: &Option<Rc<DeviceConsoleView>>,
        ) -> bool {
            if *is_streaming.lock() {
                if let Some(c) = console {
                    c.append_log(&format!("{}\n", t!("Command ignored: a job is running.")));
                }
                return true;
            }
            false
        }

        fn send_jog(
            axis: char,
            delta: f32,
            communicator: &ThreadSafe<SerialCommunicator>,
            feed_mm_per_min: f32,
            console: &Option<Rc<DeviceConsoleView>>,
            is_streaming: &ThreadSafe<bool>,
        ) {
            // Jogging (buttons or numpad shortcuts) must never inject motion into a running
            // job, and must not log commands that were never sent while disconnected.
            if *is_streaming.lock() {
                if let Some(c) = console {
                    c.append_log(&format!("{}\n", t!("Jog ignored: a job is running.")));
                }
                return;
            }
            let mut comm = communicator.lock();
            if !comm.is_connected() {
                return;
            }
            // G21 makes the jog explicitly metric: internal jog values are always mm and
            // mm/min, regardless of the controller's current G20/G21 modal state.
            let jog_cmd = format!("$J=G21 G91 {axis}{delta} F{feed_mm_per_min}\n");
            if let Some(c) = console {
                c.append_log(&format!("> {}\n", jog_cmd.trim()));
            }
            let _ = comm.send(jog_cmd.as_bytes());
        }

        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            let continuous = view.continuous_jog_check.clone();
            view.jog_x_pos.connect_clicked(move |_| {
                if continuous.is_active() {
                    return;
                }
                let step = *jog_step_mm.lock();
                let feed = *jog_feed_mm_per_min.lock();
                send_jog('X', step, &communicator, feed, &console, &is_streaming);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            let continuous = view.continuous_jog_check.clone();
            view.jog_x_neg.connect_clicked(move |_| {
                if continuous.is_active() {
                    return;
                }
                let step = *jog_step_mm.lock();
                let feed = *jog_feed_mm_per_min.lock();
                send_jog('X', -step, &communicator, feed, &console, &is_streaming);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            let continuous = view.continuous_jog_check.clone();
            view.jog_y_pos.connect_clicked(move |_| {
                if continuous.is_active() {
                    return;
                }
                let step = *jog_step_mm.lock();
                let feed = *jog_feed_mm_per_min.lock();
                send_jog('Y', step, &communicator, feed, &console, &is_streaming);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            let continuous = view.continuous_jog_check.clone();
            view.jog_y_neg.connect_clicked(move |_| {
                if continuous.is_active() {
                    return;
                }
                let step = *jog_step_mm.lock();
                let feed = *jog_feed_mm_per_min.lock();
                send_jog('Y', -step, &communicator, feed, &console, &is_streaming);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            let continuous = view.continuous_jog_check.clone();
            view.jog_z_pos.connect_clicked(move |_| {
                if continuous.is_active() {
                    return;
                }
                let step = *jog_step_mm.lock();
                let feed = *jog_feed_mm_per_min.lock();
                send_jog('Z', step, &communicator, feed, &console, &is_streaming);
            });
        }
        {
            let communicator = view.communicator.clone();
            let jog_step_mm = view.jog_step_mm.clone();
            let jog_feed_mm_per_min = view.jog_feed_mm_per_min.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            let continuous = view.continuous_jog_check.clone();
            view.jog_z_neg.connect_clicked(move |_| {
                if continuous.is_active() {
                    return;
                }
                let step = *jog_step_mm.lock();
                let feed = *jog_feed_mm_per_min.lock();
                send_jog('Z', -step, &communicator, feed, &console, &is_streaming);
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
            let is_streaming = view.is_streaming.clone();

            let view_keys = view.clone();
            let console_entry_release = console_entry.clone();
            let view_release = view.clone();
            controller.connect_key_released(move |_, key, _, _| {
                if let Some(entry) = console_entry_release.as_ref() {
                    if entry.has_focus() {
                        return;
                    }
                }
                if key.to_unicode().and_then(numpad_jog).is_some() {
                    view_release.stop_continuous_jog();
                }
            });
            controller.connect_key_pressed(move |_, key, _, _| {
                if let Some(entry) = console_entry.as_ref() {
                    if entry.has_focus() {
                        return glib::Propagation::Proceed;
                    }
                }

                let Some(ch) = key.to_unicode() else {
                    return glib::Propagation::Proceed;
                };

                if view_keys.continuous_jog_enabled() {
                    let Some((axis, direction)) = numpad_jog(ch) else {
                        return glib::Propagation::Proceed;
                    };
                    view_keys.start_continuous_jog(axis, direction);
                    return glib::Propagation::Stop;
                }

                let step = *jog_step_mm.lock();
                let feed = *jog_feed_mm_per_min.lock();

                match ch {
                    '8' => send_jog('Y', step, &communicator, feed, &console, &is_streaming),
                    '2' => send_jog('Y', -step, &communicator, feed, &console, &is_streaming),
                    '4' => send_jog('X', -step, &communicator, feed, &console, &is_streaming),
                    '6' => send_jog('X', step, &communicator, feed, &console, &is_streaming),
                    '9' => send_jog('Z', step, &communicator, feed, &console, &is_streaming),
                    '3' => send_jog('Z', -step, &communicator, feed, &console, &is_streaming),
                    _ => return glib::Propagation::Proceed,
                }

                glib::Propagation::Stop
            });

            view.widget.add_controller(controller);

            // A key release can be missed if focus leaves the panel mid-jog: stop then too.
            let focus = gtk4::EventControllerFocus::new();
            let view_focus = view.clone();
            focus.connect_leave(move |_| view_focus.stop_continuous_jog());
            view.widget.add_controller(focus);
        }

        // Continuous (hold-to-jog) mode for the jog buttons.
        view.attach_continuous_jog(&view.jog_x_pos, 'X', 1.0);
        view.attach_continuous_jog(&view.jog_x_neg, 'X', -1.0);
        view.attach_continuous_jog(&view.jog_y_pos, 'Y', 1.0);
        view.attach_continuous_jog(&view.jog_y_neg, 'Y', -1.0);
        view.attach_continuous_jog(&view.jog_z_pos, 'Z', 1.0);
        view.attach_continuous_jog(&view.jog_z_neg, 'Z', -1.0);
        {
            let view_toggle = view.clone();
            view.continuous_jog_check.connect_toggled(move |c| {
                if !c.is_active() {
                    view_toggle.stop_continuous_jog();
                }
            });
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
            let view_pause = view.clone();

            // Boton de pausa
            view.pause_btn.connect_clicked(move |_| {
                if !view_pause.job_active() {
                    return;
                }
                if let Some(c) = console.as_ref() {
                    c.append_log("> ! (Pause)\n");
                }

                let already_paused = *is_paused.lock();

                // Detectar modo láser: leer $32, si no disponible → CNC (false)
                let is_laser = device_status::get_grbl_setting(32)
                    .and_then(|v| v.parse::<i32>().ok())
                    .map(|v| v == 1)
                    .unwrap_or(false); // ← Default: CNC

                let sender = view_pause.direct_sender.lock().clone();
                match sender.as_ref().filter(|s| s.is_running()) {
                    // DirectSender sends the raw feed-hold byte itself.
                    Some(sender) => sender.pause(),
                    None => {
                        let mut comm = communicator.lock();
                        let _ = comm.send(b"!");
                    }
                }

                // CNC: stop the spindle with the realtime spindle-stop override (0x9E).
                // A queued `M5` line would only execute after the buffered moves, i.e.
                // *after* resume, and the job would continue cutting with the spindle off.
                // GRBL restores the spindle automatically on resume (~). Only send it on the
                // first pause, because 0x9E toggles and a second press would restore it.
                // Láser: solo enviar ! (GRBL apaga el láser automáticamente)
                if !is_laser && !already_paused {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x9E]);
                    if let Some(c) = console.as_ref() {
                        c.append_log("> 0x9E (Spindle stop)\n");
                    }
                }

                *is_paused.lock() = true;
                view_pause.update_job_controls();
            });
        }

        {
            let communicator = view.communicator.clone();
            let is_paused = view.is_paused.clone();
            let is_streaming = view.is_streaming.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let send_queue = view.send_queue.clone();
            let console = view.device_console.clone();
            let view_resume = view.clone();

            view.resume_btn.connect_clicked(move |_| {
                if !view_resume.job_active() {
                    return;
                }
                if let Some(c) = console.as_ref() {
                    c.append_log("> ~ (Resume)\n");
                }
                // Send the cycle-start byte exactly once (DirectSender sends it itself).
                let sender = view_resume.direct_sender.lock().clone();
                match sender.as_ref().filter(|s| s.is_running()) {
                    Some(sender) => sender.resume(),
                    None => {
                        let mut comm = communicator.lock();
                        let _ = comm.send(b"~");
                    }
                }
                *is_paused.lock() = false;
                view_resume.update_job_controls();

                // Kickstart if stalled
                if *is_streaming.lock() && !*waiting_for_ack.lock() {
                    let mut queue = send_queue.lock();
                    if let Some(cmd) = queue.pop_front() {
                        if let Some(c) = console.as_ref() {
                            c.append_log(&format!("> {}\n", cmd));
                        }
                        {
                            let mut comm = communicator.lock();
                            let _ = comm.send_command(&cmd);
                            *waiting_for_ack.lock() = true;
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
            let view_stop = view.clone();

            view.stop_btn.connect_clicked(move |_| {
                // Stop DirectSender before the reset so it cannot write lines after it
                // (the worker re-checks this flag while holding the communicator lock).
                if let Some(sender) = view_stop.direct_sender.lock().as_ref() {
                    sender.stop();
                }

                if let Some(c) = console.as_ref() {
                    c.append_log("> 0x18 (Stop)\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send(&[0x18]);
                }

                *is_streaming.lock() = false;
                *is_paused.lock() = false;
                *waiting_for_ack.lock() = false;
                *job_start_time.lock() = None;
                send_queue.lock().clear();

                if let Some(sb) = status_bar.as_ref() {
                    sb.set_progress(0.0, "", "");
                }
                // Re-enable Send; Pause/Resume/Stop follow the (now ended) job state.
                view_stop.send_btn.set_sensitive(true);
                view_stop.update_job_controls();
            });
        }

        {
            let editor = view.editor.clone();
            let widget_for_dialog = view.widget.clone();
            let view_clone = view.clone();
            let communicator_clone = view.communicator.clone();

            view.send_btn.connect_clicked(move |_| {
                // Never start a second stream on top of a running one (DirectSender::new
                // would otherwise reset the shared streaming flag mid-job).
                let previous_still_running = view_clone
                    .direct_sender
                    .lock()
                    .as_ref()
                    .is_some_and(|s| s.is_running());
                if *view_clone.is_streaming.lock() || previous_still_running {
                    if let Some(c) = view_clone.device_console.as_ref() {
                        c.append_log(&format!("{}\n", t!("A job is already running.")));
                    }
                    return;
                }

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
                let content = view_clone.prepare_job_content(content);

                // Send is disabled while the job runs; Pause/Stop follow the job state.
                view_clone.send_btn.set_sensitive(false);

                let (sender, receiver) =
                    DirectSender::new(communicator_clone.clone(), view_clone.is_streaming.clone());
                // Keep the sender for pause/stop/E-stop.
                *view_clone.direct_sender.lock() = Some(sender.clone());
                *view_clone.is_paused.lock() = false;

                let view_timeout = view_clone.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    while let Ok(event) = receiver.try_recv() {
                        match event {
                            SenderEvent::Log(msg) => view_timeout.log(&msg),
                            SenderEvent::Response(line) => {
                                view_timeout.log(&Self::decode_response_line(&line));
                            }
                            SenderEvent::Status(line) => view_timeout.apply_status_report(&line),
                            SenderEvent::Progress(percent) => {
                                Self::update_progress(
                                    &view_timeout.status_bar,
                                    &view_timeout.job_start_time,
                                    f64::from(percent),
                                    None,
                                    None,
                                    None,
                                );
                            }
                            SenderEvent::Finished(outcome) => {
                                view_timeout.finish_direct_job(&outcome);
                                // The worker has finished: stop polling, otherwise every
                                // Send adds another 100 ms timer that runs forever.
                                return glib::ControlFlow::Break;
                            }
                        }
                    }
                    glib::ControlFlow::Continue
                });

                *view_clone.job_start_time.lock() = Some(std::time::Instant::now());
                view_clone.update_job_controls();
                sender.send_gcode(&content);
            });
        }

        // Machine State Controls
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            view.home_btn.connect_clicked(move |_| {
                if job_running(&is_streaming, &console) {
                    return;
                }
                if let Some(c) = console.as_ref() {
                    c.append_log("> $H\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send_command("$H");
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            let view_unlock = view.clone();

            view.unlock_btn.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> $X\n");
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send_command("$X");
                }
                view_unlock.update_job_controls();
            });
        }

        // WCS Controls
        for (i, btn) in view.wcs_btns.iter().enumerate() {
            let view_wcs = view.clone();
            let cmd = format!("G{}", 54 + i);
            btn.connect_toggled(move |b| {
                if !b.is_active() || view_wcs.wcs_reverting.get() {
                    return;
                }
                if *view_wcs.is_streaming.lock() {
                    // Changing the WCS mid-job would shift every following move: put the
                    // toggle back on the WCS that is really active and send nothing.
                    let previous = view_wcs.active_wcs.get();
                    if let Some(prev_btn) = view_wcs.wcs_btns.get(previous) {
                        view_wcs.wcs_reverting.set(true);
                        prev_btn.set_active(true);
                        view_wcs.wcs_reverting.set(false);
                    }
                    tracing::info!(requested = %cmd, "WCS change ignored while a job is running");
                    view_wcs.log(&format!(
                        "{} ({cmd})",
                        t!("Work coordinate change ignored: a job is running.")
                    ));
                    return;
                }
                view_wcs.log(&format!("> {cmd}"));
                let result = view_wcs.communicator.lock().send_command(&cmd);
                match result {
                    Ok(()) => view_wcs.active_wcs.set(i),
                    Err(e) => {
                        tracing::warn!(command = %cmd, error = %e, "Failed to select WCS");
                        view_wcs.log(&format!("{}: {e}", t!("Error sending")));
                    }
                }
            });
        }

        // Zero Controls
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            view.x_zero_btn.connect_clicked(move |_| {
                if job_running(&is_streaming, &console) {
                    return;
                }
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} X0");
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send_command(&cmd);
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            view.y_zero_btn.connect_clicked(move |_| {
                if job_running(&is_streaming, &console) {
                    return;
                }
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} Y0");
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send_command(&cmd);
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            view.z_zero_btn.connect_clicked(move |_| {
                if job_running(&is_streaming, &console) {
                    return;
                }
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} Z0");
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                {
                    let mut comm = communicator.lock();
                    let _ = comm.send_command(&cmd);
                }
            });
        }
        // Rotary axis zero buttons (shown when the machine has A/B/C axes)
        for (btn, axis) in [
            (view.a_zero_btn.as_ref(), 'A'),
            (view.b_zero_btn.as_ref(), 'B'),
            (view.c_zero_btn.as_ref(), 'C'),
        ] {
            let Some(btn) = btn else {
                continue;
            };
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            btn.connect_clicked(move |_| {
                if job_running(&is_streaming, &console) {
                    return;
                }
                let p = wcs_btns
                    .iter()
                    .position(|b| b.is_active())
                    .map(|i| i + 1)
                    .unwrap_or(1);
                let cmd = format!("G10 L20 P{p} {axis}0");
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                let mut comm = communicator.lock();
                let _ = comm.send_command(&cmd);
            });
        }
        {
            let communicator = view.communicator.clone();
            let wcs_btns = view.wcs_btns.clone();
            let widget_for_dialog = view.widget.clone();
            let console = view.device_console.clone();
            let is_streaming = view.is_streaming.clone();
            view.zero_all_btn.connect_clicked(move |_| {
                if job_running(&is_streaming, &console) {
                    return;
                }
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
                        { let mut comm = communicator.lock();
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
            let is_streaming = view.is_streaming.clone();
            let include_z_check = view.goto_zero_include_z.clone();
            view.goto_zero_btn.connect_clicked(move |_| {
                if job_running(&is_streaming, &console) {
                    return;
                }
                // Explicit G90: in G91 (e.g. left over from a console command) "G0 X0 Y0"
                // would be a zero-length relative move instead of going to work zero.
                let cmd = if include_z_check.is_active() {
                    "G90 G0 X0 Y0 Z0"
                } else {
                    "G90 G0 X0 Y0"
                };
                if let Some(c) = console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                {
                    let mut comm = communicator.lock();
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

        // Quick probe buttons (Milestone 6). Distances/feeds are fixed in mm (sent with
        // G21); the log and tooltips show them in the display units.
        for (btn, name, axis_move) in [
            (&probe_z_btn, t!("Quick Z-touch probe"), PROBE_Z_MOVE),
            (
                &probe_x_neg_btn,
                t!("Probe X- edge"),
                ('X', -PROBE_EDGE_DISTANCE_MM),
            ),
            (
                &probe_x_pos_btn,
                t!("Probe X+ edge"),
                ('X', PROBE_EDGE_DISTANCE_MM),
            ),
            (
                &probe_y_neg_btn,
                t!("Probe Y- edge"),
                ('Y', -PROBE_EDGE_DISTANCE_MM),
            ),
            (
                &probe_y_pos_btn,
                t!("Probe Y+ edge"),
                ('Y', PROBE_EDGE_DISTANCE_MM),
            ),
        ] {
            let view_probe = view.clone();
            btn.connect_clicked(move |_| view_probe.start_quick_probe(&name, axis_move));
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

                // Send command (only log it as sent if it actually went out)
                {
                    let mut comm = communicator.lock();
                    if !comm.is_connected() {
                        console_clone.append_log(&format!("{}\n", t!("Not connected")));
                        return;
                    }
                    match comm.send_command(&cmd) {
                        Ok(_) => console_clone.append_log(&format!("> {}\n", cmd)),
                        Err(e) => {
                            console_clone.append_log(&format!("{}: {}\n", t!("Error sending"), e))
                        }
                    }
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
        view.connect_btn
            .connect_clicked(move |_| view_clone.on_connect_clicked());

        // Setup override handlers
        Self::setup_override_handlers(&view);

        view
    }

    // Update progress
    fn update_progress(
        status_bar: &Option<StatusBar>,
        job_start_time: &ThreadSafeOption<std::time::Instant>,
        percentage: f64,
        sent_lines: Option<usize>,
        total_lines: Option<usize>,
        remaining_lines: Option<usize>,
    ) {
        let Some(sb) = status_bar.as_ref() else {
            return;
        };
        let start_opt = *job_start_time.lock();
        let Some(start) = start_opt else {
            sb.set_progress(percentage, "", "");
            return;
        };

        let elapsed = start.elapsed().as_secs_f64();

        let remaining = if let (Some(sent), Some(total)) = (sent_lines, total_lines) {
            if sent > 0 && elapsed > 0.0 {
                let avg_per_line = elapsed / sent as f64;
                let remaining_lines = total - sent;
                remaining_lines as f64 * avg_per_line
            } else {
                0.0
            }
        } else if let Some(_rem_lines) = remaining_lines {
            if percentage > 0.0 && percentage < 100.0 {
                (elapsed / percentage) * (100.0 - percentage)
            } else {
                0.0
            }
        } else if percentage > 0.0 && percentage < 100.0 {
            (elapsed / percentage) * (100.0 - percentage)
        } else {
            0.0
        };

        let format_time = |secs: f64| {
            let h = (secs / 3600.0).floor() as u32;
            let m = ((secs % 3600.0) / 60.0).floor() as u32;
            let s = (secs % 60.0).floor() as u32;
            format!("{:02}:{:02}:{:02}", h, m, s)
        };

        sb.set_progress(percentage, &format_time(elapsed), &format_time(remaining));
    }

    /// Update axis visibility based on device axis count
    /// Shows/hides rotary axis controls (A, B, C) based on the configured axis count
    pub fn update_axis_visibility(&self, count: u8) {
        let count = count.clamp(3, 6);
        self.axis_count.set(count);

        // Show/hide A axis (4th axis)
        let show_a = count >= 4;
        if let Some(ref container) = self.a_dro_container {
            container.set_visible(show_a);
        }
        if let Some(ref btn) = self.a_zero_btn {
            btn.set_visible(show_a);
        }

        // Show/hide B axis (5th axis)
        let show_b = count >= 5;
        if let Some(ref container) = self.b_dro_container {
            container.set_visible(show_b);
        }
        if let Some(ref btn) = self.b_zero_btn {
            btn.set_visible(show_b);
        }

        // Show/hide C axis (6th axis)
        let show_c = count >= 6;
        if let Some(ref container) = self.c_dro_container {
            container.set_visible(show_c);
        }
        if let Some(ref btn) = self.c_zero_btn {
            btn.set_visible(show_c);
        }

        // Show/hide machine coordinate labels
        if let Some(ref label) = self.world_a {
            label.set_visible(show_a);
        }
        if let Some(ref label) = self.world_b {
            label.set_visible(show_b);
        }
        if let Some(ref label) = self.world_c {
            label.set_visible(show_c);
        }

        // Show/hide the whole machine limits rows (axis label and value)
        for (row, show) in [
            (&self.limits_a_row, show_a),
            (&self.limits_b_row, show_b),
            (&self.limits_c_row, show_c),
        ] {
            if let Some(row) = row {
                row.set_visible(show);
            }
        }
        for (label, show) in [
            (&self.limits_a, show_a),
            (&self.limits_b, show_b),
            (&self.limits_c, show_c),
        ] {
            if let Some(label) = label {
                label.set_visible(show);
            }
        }
    }

    /// Fills the jog step combo with the presets for `units` and selects the one
    /// closest to `step_mm`.
    ///
    /// # Arguments
    /// * `combo` - The step combo box; its ids are step sizes in mm.
    /// * `units` - Display measurement system.
    /// * `step_mm` - The step to keep (or approximate).
    ///
    /// # Returns
    /// The selected step in mm.
    fn populate_step_presets(combo: &ComboBoxText, units: MeasurementSystem, step_mm: f32) -> f32 {
        let presets = jog_step_presets(units);
        combo.remove_all();
        for preset in &presets {
            combo.append(Some(&format!("{}", preset.mm)), &preset.label);
        }
        match nearest_jog_step_preset(&presets, step_mm).and_then(|i| presets.get(i)) {
            Some(preset) => {
                combo.set_active_id(Some(&format!("{}", preset.mm)));
                preset.mm
            }
            None => step_mm,
        }
    }

    /// Updates the quick probe tooltips with distances and feed in the display units.
    ///
    /// # Arguments
    /// * `buttons` - Probe Z, X-, X+, Y-, Y+ buttons.
    /// * `units` - Display length unit.
    /// * `feed_units` - Display feed unit.
    fn update_probe_tooltips(
        buttons: &[Button; 5],
        units: MeasurementSystem,
        feed_units: FeedRateUnits,
    ) {
        let texts = [
            (
                t!("Quick Z-touch probe at current XY position"),
                PROBE_Z_MOVE,
            ),
            (t!("Probe X- edge"), ('X', -PROBE_EDGE_DISTANCE_MM)),
            (t!("Probe X+ edge"), ('X', PROBE_EDGE_DISTANCE_MM)),
            (t!("Probe Y- edge"), ('Y', -PROBE_EDGE_DISTANCE_MM)),
            (t!("Probe Y+ edge"), ('Y', PROBE_EDGE_DISTANCE_MM)),
        ];
        for (btn, (text, axis_move)) in buttons.iter().zip(texts) {
            let detail = format_probe_move(axis_move, PROBE_FEED_MM_PER_MIN, units, feed_units);
            btn.set_tooltip_text(Some(&format!("{text} ({detail})")));
        }
    }

    /// Starts a quick probe (G38.2) along one axis, relative to the current position.
    ///
    /// The command is always metric (G21); the log shows it in the display units. Only
    /// the probe line is sent: `G90` is restored once the result is known (see the
    /// status poller), so a failed probe no longer triggers a "G-code locked" error.
    ///
    /// # Arguments
    /// * `name` - Human-readable probe name for the log.
    /// * `axis_move` - Axis letter and signed distance in mm.
    pub(crate) fn start_quick_probe(&self, name: &str, axis_move: (char, f32)) {
        if *self.is_streaming.lock() {
            self.log(&t!("Command ignored: a job is running."));
            return;
        }
        if self.quick_probe.get() == QuickProbeState::AwaitingResult {
            self.log(&t!("A probe is already in progress."));
            return;
        }
        let units = *self.current_units.lock();
        let feed_units = *self.current_feed_units.lock();
        let detail = format_probe_move(axis_move, PROBE_FEED_MM_PER_MIN, units, feed_units);
        let (axis, distance) = axis_move;
        // G38.2 targets are absolute in G90: probe *relative* to the current position.
        let cmd = format!("G21 G91 G38.2 {axis}{distance} F{PROBE_FEED_MM_PER_MIN}");

        let mut comm = self.communicator.lock();
        if !comm.is_connected() {
            return;
        }
        self.log(&format!("> {name}: {detail}"));
        self.log(&format!("> {cmd}"));
        match comm.send_command(&cmd) {
            Ok(()) => self.quick_probe.set(QuickProbeState::AwaitingResult),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to send quick probe");
                self.log(&format!("{}: {e}", t!("Error sending")));
            }
        }
    }

    /// Updates the panel when a DirectSender job ends.
    ///
    /// # Arguments
    /// * `outcome` - How the job ended.
    fn finish_direct_job(&self, outcome: &JobOutcome) {
        *self.is_streaming.lock() = false;
        *self.is_paused.lock() = false;
        match outcome {
            JobOutcome::Completed => {
                // Buffered moves may still be executing; the job timer (and Stop/Pause)
                // end once the controller reports Idle.
                tracing::info!("Job completed");
                self.log(&t!("Job Completed"));
            }
            JobOutcome::Cancelled => {
                *self.job_start_time.lock() = None;
                self.log(&format!("{} {}", t!("Job stopped:"), t!("stopped by user")));
            }
            JobOutcome::Stopped(reason) => {
                // After an `error:` the controller keeps executing the moves it already
                // buffered, so keep the job active until it reports Idle.
                if !reason.starts_with("error:") {
                    *self.job_start_time.lock() = None;
                }
                tracing::warn!(reason = %reason, "Job stopped");
                let text = format!(
                    "{} {}",
                    t!("Job stopped:"),
                    Self::decode_response_line(reason)
                );
                self.log(&text);
                self.show_controller_error(&text);
            }
        }
        if let Some(sb) = self.status_bar.as_ref() {
            sb.set_progress(0.0, "", "");
        }
        self.send_btn
            .set_sensitive(device_status::DEVICE_STATUS.read().is_connected);
        self.update_job_controls();
    }

    /// Returns the job content to stream, prefixed with `G90` when a failed quick probe
    /// left the controller in incremental mode.
    fn prepare_job_content(&self, content: String) -> String {
        if self.quick_probe.get() == QuickProbeState::NeedsAbsoluteRestore {
            self.quick_probe.set(QuickProbeState::Idle);
            self.log(&t!(
                "Restoring absolute mode (G90) before the job (a probe failed)."
            ));
            format!("G90\n{content}")
        } else {
            content
        }
    }

    /// Get the current axis count
    pub fn get_axis_count(&self) -> u8 {
        self.axis_count.get()
    }
}
mod operations;
mod overrides;
