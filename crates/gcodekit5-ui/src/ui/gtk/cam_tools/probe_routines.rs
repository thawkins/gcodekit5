//! Probe Routines Tool - Touch probe CAM tool dashboard
//!
//! Provides a UI for configuring and running probe routines including:
//! - Z-Touch (surface probe)
//! - Edge Find (single axis)
//! - Corner Find (two-axis)
//! - Bore Center (4-point internal)
//! - Boss Center (4-point external)
//! - Tool Length measurement

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, FileChooserAction, Label, Orientation,
    Paned, ResponseType, ScrolledWindow, Stack, TextView, WrapMode,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::common::{
    poll_connection_status, set_paned_initial_fraction, show_load_warnings, write_preset_file,
};
use super::CamToolsView;
use crate::device_status;
use crate::t;
use crate::ui::gtk::machine_control::MachineControlView;
use gcodekit5_camtools::cam_presets::ProbePreset;
use gcodekit5_camtools::comment_processor::strip_comments;
use gcodekit5_camtools::probe_routines::{
    defaults as probe_defaults, ProbeOptions, ProbeRoutineEngine,
};
use gcodekit5_camtools::wcs_service::{WcsUpdateConfig, WcsUpdateService};
use gcodekit5_communication::firmware::grbl::controller::parse_probe_report;
use gcodekit5_communication::Communicator;
use gcodekit5_core::{
    Corner, Position, ProbeAxis as Axis, ProbeDirection as Direction, ProbeReport, ProbeRoutine,
};
use gcodekit5_settings::SettingsController;

/// State for the currently selected probe routine
type RoutineType = usize;
const ROUTINE_Z_TOUCH: RoutineType = 0;
const ROUTINE_EDGE_FIND: RoutineType = 1;
const ROUTINE_CORNER_FIND: RoutineType = 2;
const ROUTINE_BORE_CENTER: RoutineType = 3;
const ROUTINE_BOSS_CENTER: RoutineType = 4;
const ROUTINE_TOOL_LENGTH: RoutineType = 5;

/// Routine info for sidebar display
struct RoutineInfo {
    #[allow(dead_code)]
    id: RoutineType,
    #[allow(dead_code)]
    name: &'static str,
    #[allow(dead_code)]
    description: &'static str,
}

/// All input widgets of the probe tool.
struct ProbeWidgets {
    routine: ComboBoxText,
    safe_height: Entry,
    fast_feed: Entry,
    slow_feed: Entry,
    backoff: Entry,
    max_depth: Entry,
    wcs: ComboBoxText,
    auto_update: CheckButton,
    edge_axis: ComboBoxText,
    edge_dir: ComboBoxText,
    edge_dist: Entry,
    corner: ComboBoxText,
    diameter: Entry,
    setter_x: Entry,
    setter_y: Entry,
    setter_z: Entry,
    probe_depth: Entry,
    tip_radius: Entry,
    plate_thickness: Entry,
}

/// Interval at which a running probe program is checked for completion.
const PROBE_MONITOR_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// State of probe runs started from this tool (main-thread only).
#[derive(Default)]
struct ProbeSession {
    /// A probe program is running, or a failed run is waiting to restore G90.
    busy: Cell<bool>,
    /// Report and options of the last successful run, for "Apply to WCS".
    last: RefCell<Option<(ProbeReport, ProbeOptions)>>,
}

/// Outcome of reading the device-console output of a probe run.
struct ProbeRunLog {
    /// Probe reports in the order received (one per `G38.x` cycle).
    reports: Vec<gcodekit5_core::ProbeResult>,
    /// First alarm or error line, if any.
    failure: Option<String>,
}

impl ProbeRunLog {
    /// Collect `[PRB:...]` reports and the first `ALARM:`/`error:` from console text.
    fn parse(text: &str) -> Self {
        let mut reports = Vec::new();
        let mut failure = None;
        for line in text.lines().map(str::trim) {
            if let Some(report) = parse_probe_report(line) {
                reports.push(report);
            } else if failure.is_none() {
                let lower = line.to_ascii_lowercase();
                if lower.starts_with("alarm:") || lower.starts_with("error:") {
                    failure = Some(line.to_string());
                }
            }
        }
        Self { reports, failure }
    }
}

pub struct ProbeRoutinesTool {
    content: Box,
    #[allow(dead_code)]
    current_routine: Cell<RoutineType>,
}

impl ProbeRoutinesTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        _settings: Rc<SettingsController>,
        machine_control: Option<MachineControlView>,
        on_generate: Rc<F>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });
        header.append(&back_btn);

        let title = Label::builder()
            .label(t!("Probe Tools"))
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);
        content_box.append(&header);

        // Paned Layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar (40%)
        let sidebar = Box::new(Orientation::Vertical, 6);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        // Title and description
        let title_label = Label::builder()
            .label(t!("Probe Tools"))
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label(t!("Configure and run touch probe routines for work coordinate setup. Includes Z-touch, edge find, corner find, bore/boss center, and tool length measurement."))
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Routine selector
        let routine_list = Box::new(Orientation::Vertical, 6);
        routine_list.set_margin_top(24);

        let _routines = [
            RoutineInfo {
                id: ROUTINE_Z_TOUCH,
                name: "Z-Touch",
                description: "Surface probe to set Z=0",
            },
            RoutineInfo {
                id: ROUTINE_EDGE_FIND,
                name: "Edge Find",
                description: "Single-axis edge detection",
            },
            RoutineInfo {
                id: ROUTINE_CORNER_FIND,
                name: "Corner Find",
                description: "Two-axis corner location",
            },
            RoutineInfo {
                id: ROUTINE_BORE_CENTER,
                name: "Bore Center",
                description: "Internal diameter center",
            },
            RoutineInfo {
                id: ROUTINE_BOSS_CENTER,
                name: "Boss Center",
                description: "External diameter center",
            },
            RoutineInfo {
                id: ROUTINE_TOOL_LENGTH,
                name: "Tool Length",
                description: "Measure tool using setter plate",
            },
        ];

        // Status indicators
        let status_box = Box::new(Orientation::Horizontal, 12);
        status_box.set_margin_top(24);

        let status_label = Label::builder()
            .label("Status: ● Idle")
            .css_classes(vec!["body"])
            .halign(Align::Start)
            .build();

        let probe_pin_label = Label::builder()
            .label("Probe: ○ Open")
            .css_classes(vec!["body"])
            .halign(Align::Start)
            .build();

        status_box.append(&status_label);
        status_box.append(&probe_pin_label);
        sidebar.append(&status_box);

        // Offline message
        let offline_msg = Label::builder()
            .label(t!("Device connection required to use Probe Tools"))
            .css_classes(vec!["title-3", "error"])
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .halign(Align::Center)
            .valign(Align::Center)
            .vexpand(true)
            .build();
        sidebar.append(&offline_msg);

        // Connection status polling: holds only a weak ref, stopped when the page is destroyed.
        poll_connection_status(&offline_msg, |offline_msg, connected| {
            offline_msg.set_visible(!connected);
        });

        // Content Area (60%)
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 12);
        scroll_content.set_margin_top(24);
        scroll_content.set_margin_bottom(24);
        scroll_content.set_margin_start(24);
        scroll_content.set_margin_end(24);

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Routine selector dropdown
        let routine_row = ActionRow::builder()
            .title("Routine")
            .subtitle("Select probe routine type")
            .build();
        let routine_combo = ComboBoxText::new();
        routine_combo.append(Some("z_touch"), "Z-Touch");
        routine_combo.append(Some("edge_find"), "Edge Find");
        routine_combo.append(Some("corner_find"), "Corner Find");
        routine_combo.append(Some("bore_center"), "Bore Center");
        routine_combo.append(Some("boss_center"), "Boss Center");
        routine_combo.append(Some("tool_length"), "Tool Length");
        routine_combo.set_active(Some(0));
        routine_row.add_suffix(&routine_combo);

        let routine_group = PreferencesGroup::builder()
            .title("Routine Selection")
            .build();
        routine_group.add(&routine_row);
        scroll_content.append(&routine_group);

        // Parameters section
        let params_group = PreferencesGroup::builder().title("Parameters").build();

        // Common parameters
        // All probe parameters are entered and parsed in mm (mm/min for feeds), like
        // the routine-specific fields below. Using measurement-system-aware rows here
        // made imperial users' values (e.g. 0.197 in safe height) be parsed as mm.
        let (safe_height_row, safe_height) =
            Self::create_mm_row("Safe Height:", probe_defaults::SAFE_HEIGHT, "mm");
        let (fast_feed_row, fast_feed) =
            Self::create_mm_row("Fast Feed Rate:", probe_defaults::FAST_FEED, "mm/min");
        let (slow_feed_row, slow_feed) =
            Self::create_mm_row("Slow Feed Rate:", probe_defaults::SLOW_FEED, "mm/min");
        let (backoff_row, backoff) =
            Self::create_mm_row("Backoff Distance:", probe_defaults::BACKOFF, "mm");
        safe_height_row.set_subtitle(
            "Retract above the surface after Z-touch / tool length. Edge, corner, bore and \
             boss probes start where the probe is and return to that height.",
        );

        params_group.add(&safe_height_row);
        params_group.add(&fast_feed_row);
        params_group.add(&slow_feed_row);
        params_group.add(&backoff_row);

        // Max probe depth
        let max_depth_row = ActionRow::builder().title("Max Probe Depth").build();
        let max_depth_entry = Entry::new();
        max_depth_entry.set_text(&format!("{:.1}", probe_defaults::MAX_PROBE_DEPTH));
        max_depth_entry.set_width_request(80);
        max_depth_entry.set_halign(Align::End);
        max_depth_row.add_suffix(&max_depth_entry);
        params_group.add(&max_depth_row);

        // WCS selection
        let wcs_row = ActionRow::builder()
            .title("Target WCS")
            .subtitle("Work coordinate system to update")
            .build();
        let wcs_combo = ComboBoxText::new();
        for i in 54..=59 {
            wcs_combo.append(Some(&format!("G{}", i)), &format!("G{}", i));
        }
        wcs_combo.set_active_id(Some("G54"));
        wcs_row.add_suffix(&wcs_combo);
        params_group.add(&wcs_row);

        // Auto-update WCS checkbox
        let auto_update_row = ActionRow::builder()
            .title("Auto-Update WCS")
            .subtitle(
                "Write the result into the selected WCS (G10 L20) from the generated G-code. \
                 Z-touch, edge and corner only; bore/boss centres are not set automatically.",
            )
            .build();
        let auto_update_check = CheckButton::builder().active(false).build();
        auto_update_row.add_suffix(&auto_update_check);
        params_group.add(&auto_update_row);

        scroll_content.append(&params_group);

        // Routine-specific parameters (hidden by default, shown based on selection)
        let specific_params = PreferencesGroup::builder()
            .title("Routine Settings")
            .build();

        // Edge find: axis and direction
        let edge_axis_row = ActionRow::builder().title("Probe Axis").build();
        let edge_axis_combo = ComboBoxText::new();
        edge_axis_combo.append(Some("X"), "X Axis");
        edge_axis_combo.append(Some("Y"), "Y Axis");
        edge_axis_combo.append(Some("Z"), "Z Axis");
        edge_axis_row.add_suffix(&edge_axis_combo);
        specific_params.add(&edge_axis_row);

        let edge_dir_row = ActionRow::builder().title("Probe Direction").build();
        let edge_dir_combo = ComboBoxText::new();
        edge_dir_combo.append(Some("negative"), "Negative");
        edge_dir_combo.append(Some("positive"), "Positive");
        edge_dir_row.add_suffix(&edge_dir_combo);
        specific_params.add(&edge_dir_row);

        // Edge find: probe distance
        let edge_dist_row = ActionRow::builder().title("Probe Distance").build();
        let edge_dist_entry = Entry::new();
        edge_dist_entry.set_text(&format!("{:.1}", probe_defaults::PROBE_DISTANCE));
        edge_dist_entry.set_width_request(80);
        edge_dist_row.add_suffix(&edge_dist_entry);
        specific_params.add(&edge_dist_row);

        // Corner find: corner selection
        let corner_row = ActionRow::builder().title("Corner").build();
        let corner_combo = ComboBoxText::new();
        corner_combo.append(Some("xmin_ymin"), "X-min / Y-min");
        corner_combo.append(Some("xmax_ymin"), "X-max / Y-min");
        corner_combo.append(Some("xmin_ymax"), "X-min / Y-max");
        corner_combo.append(Some("xmax_ymax"), "X-max / Y-max");
        corner_row.add_suffix(&corner_combo);
        specific_params.add(&corner_row);

        // Bore/Boss diameter
        let diameter_row = ActionRow::builder().title("Diameter (mm)").build();
        let diameter_entry = Entry::new();
        diameter_entry.set_text("20.0");
        diameter_entry.set_width_request(80);
        diameter_row.add_suffix(&diameter_entry);
        specific_params.add(&diameter_row);

        // Tool length setter plate position
        let setter_x_row = ActionRow::builder().title("Setter Plate X").build();
        let setter_x_entry = Entry::new();
        setter_x_entry.set_text("100.0");
        setter_x_entry.set_width_request(80);
        setter_x_row.add_suffix(&setter_x_entry);
        specific_params.add(&setter_x_row);

        let setter_y_row = ActionRow::builder().title("Setter Plate Y").build();
        let setter_y_entry = Entry::new();
        setter_y_entry.set_text("100.0");
        setter_y_entry.set_width_request(80);
        setter_y_row.add_suffix(&setter_y_entry);
        specific_params.add(&setter_y_row);

        let setter_z_row = ActionRow::builder().title("Setter Plate Z").build();
        let setter_z_entry = Entry::new();
        setter_z_entry.set_text("-50.0");
        setter_z_entry.set_width_request(80);
        setter_z_row.add_suffix(&setter_z_entry);
        specific_params.add(&setter_z_row);

        // Sideways probing depth and compensation
        let (probe_depth_row, probe_depth_entry) =
            Self::create_mm_row("Probe Depth:", probe_defaults::PROBE_DEPTH, "mm");
        probe_depth_row.set_subtitle(
            "How far below the start height to lower (protected G38.3) before probing sideways",
        );
        specific_params.add(&probe_depth_row);

        let (tip_radius_row, tip_radius_entry) =
            Self::create_mm_row("Probe Tip Radius:", 0.0, "mm");
        tip_radius_row.set_subtitle("Stylus ball radius; compensates edge and corner results");
        specific_params.add(&tip_radius_row);

        let (plate_row, plate_entry) = Self::create_mm_row("Plate Thickness:", 0.0, "mm");
        plate_row.set_subtitle("Touch plate laid on the work; Z0 is set this far below the touch");
        specific_params.add(&plate_row);

        scroll_content.append(&specific_params);

        // Results section (initially hidden)
        let results_group = PreferencesGroup::builder().title("Results").build();

        let result_text = TextView::new();
        result_text.set_editable(false);
        result_text.set_wrap_mode(WrapMode::Word);
        result_text.set_buffer(Some(&gtk4::TextBuffer::new(None)));
        results_group.add(&result_text);

        let apply_btn = Button::with_label("Apply to WCS");
        apply_btn.add_css_class("suggested-action");
        let apply_row = ActionRow::new();
        apply_row.add_suffix(&apply_btn);
        results_group.add(&apply_row);

        results_group.set_visible(false);
        scroll_content.append(&results_group);

        right_panel.append(&scrolled);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label(&t!("Load"));
        let save_btn = Button::with_label(&t!("Save"));
        let generate_btn = Button::with_label(&t!("Generate G-code"));
        let probe_btn = Button::with_label(&t!("Probe"));
        probe_btn.add_css_class("suggested-action");

        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&generate_btn);
        action_box.append(&probe_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        set_paned_initial_fraction(&paned, 0.40);

        content_box.append(&paned);

        let widgets = Rc::new(ProbeWidgets {
            routine: routine_combo.clone(),
            safe_height,
            fast_feed,
            slow_feed,
            backoff,
            max_depth: max_depth_entry,
            wcs: wcs_combo,
            auto_update: auto_update_check,
            edge_axis: edge_axis_combo,
            edge_dir: edge_dir_combo,
            edge_dist: edge_dist_entry,
            corner: corner_combo,
            diameter: diameter_entry,
            setter_x: setter_x_entry,
            setter_y: setter_y_entry,
            setter_z: setter_z_entry,
            probe_depth: probe_depth_entry,
            tip_radius: tip_radius_entry,
            plate_thickness: plate_entry,
        });

        // Wire up routine selection visibility
        let (edge_axis_row, edge_dir_row, edge_dist_row) = (
            edge_axis_row.clone(),
            edge_dir_row.clone(),
            edge_dist_row.clone(),
        );
        let (corner_row, diameter_row) = (corner_row.clone(), diameter_row.clone());
        let (setter_x_row, setter_y_row, setter_z_row) = (
            setter_x_row.clone(),
            setter_y_row.clone(),
            setter_z_row.clone(),
        );
        let (probe_depth_row, tip_radius_row, plate_row) = (
            probe_depth_row.clone(),
            tip_radius_row.clone(),
            plate_row.clone(),
        );
        let rows = [
            edge_axis_row.clone(),
            edge_dir_row.clone(),
            edge_dist_row.clone(),
            corner_row.clone(),
            diameter_row.clone(),
            setter_x_row.clone(),
            setter_y_row.clone(),
            setter_z_row.clone(),
            probe_depth_row.clone(),
            tip_radius_row.clone(),
            plate_row.clone(),
        ];
        routine_combo.connect_changed(move |combo| {
            let id = combo.active_id().map(|s| s.as_str().to_string());

            for row in &rows {
                row.set_visible(false);
            }

            match id.as_deref() {
                Some("z_touch") => {
                    plate_row.set_visible(true);
                }
                Some("edge_find") => {
                    edge_axis_row.set_visible(true);
                    edge_dir_row.set_visible(true);
                    edge_dist_row.set_visible(true);
                    probe_depth_row.set_visible(true);
                    tip_radius_row.set_visible(true);
                    plate_row.set_visible(true);
                }
                Some("corner_find") => {
                    corner_row.set_visible(true);
                    edge_dist_row.set_visible(true);
                    probe_depth_row.set_visible(true);
                    tip_radius_row.set_visible(true);
                }
                Some("bore_center") | Some("boss_center") => {
                    diameter_row.set_visible(true);
                    probe_depth_row.set_visible(true);
                    tip_radius_row.set_visible(true);
                }
                Some("tool_length") => {
                    setter_x_row.set_visible(true);
                    setter_y_row.set_visible(true);
                    setter_z_row.set_visible(true);
                }
                _ => {}
            }
        });
        // Apply the initial visibility.
        routine_combo.set_active(Some(1));
        routine_combo.set_active(Some(0));

        // Generate G-code handler
        let w_gen = widgets.clone();
        let on_generate_clone = on_generate.clone();
        generate_btn.connect_clicked(move |_| {
            let built = Self::build_routine(&w_gen)
                .and_then(|r| Self::build_options(&w_gen).map(|o| (r, o)));
            let (routine, options) = match built {
                Ok(v) => v,
                Err(e) => {
                    CamToolsView::show_error_dialog(&t!("Invalid Parameters"), &format!("{}", e));
                    return;
                }
            };
            // The routines move relative to wherever the probe is when they run; the
            // position is only recorded in the program header.
            let current_pos = device_status::get_status()
                .machine_position
                .map(|p| Position::new(p.x as f32, p.y as f32, p.z as f32))
                .unwrap_or_default();
            match ProbeRoutineEngine::with_options(routine, options).generate(current_pos) {
                Ok(output) => on_generate_clone(output.gcode),
                Err(e) => CamToolsView::show_error_dialog(
                    &t!("Probe Generation Error"),
                    &format!("{}", e),
                ),
            }
        });

        // Wire up Save button
        let w_save = widgets.clone();
        save_btn.clone().connect_clicked(move |_| {
            let json = match Self::collect_preset(&w_save).and_then(|p| p.to_json()) {
                Ok(json) => json,
                Err(e) => {
                    CamToolsView::show_error_dialog(&t!("Save Error"), &format!("{}", e));
                    return;
                }
            };

            let window = save_btn
                .root()
                .and_then(|r| r.downcast::<gtk4::Window>().ok());

            let dialog = gtk4::FileChooserDialog::builder()
                .title(t!("Save Probe Preset"))
                .action(FileChooserAction::Save)
                .modal(true)
                .build();
            dialog.add_button(&t!("Cancel"), ResponseType::Cancel);
            dialog.add_button(&t!("Save"), ResponseType::Accept);
            dialog.set_current_name("probe_preset.json");

            if let Some(ref w) = window {
                dialog.set_transient_for(Some(w));
            }

            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(path) = d.file().and_then(|f| f.path()) {
                        write_preset_file(&path, &json);
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // Wire up Load button
        let w_load = widgets.clone();
        load_btn.clone().connect_clicked(move |_| {
            let window = load_btn
                .root()
                .and_then(|r| r.downcast::<gtk4::Window>().ok());

            let dialog = gtk4::FileChooserDialog::builder()
                .title(t!("Load Probe Preset"))
                .action(FileChooserAction::Open)
                .modal(true)
                .build();
            dialog.add_button(&t!("Cancel"), ResponseType::Cancel);
            dialog.add_button(&t!("Open"), ResponseType::Accept);

            if let Some(ref w) = window {
                dialog.set_transient_for(Some(w));
            }

            // Filter for JSON files
            let filter = gtk4::FileFilter::new();
            filter.add_pattern("*.json");
            filter.set_name(Some("JSON files"));
            dialog.add_filter(&filter);

            let w = w_load.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(path) = d.file().and_then(|f| f.path()) {
                        let loaded = std::fs::read_to_string(&path)
                            .map_err(anyhow::Error::from)
                            .and_then(|json| ProbePreset::from_json(&json));
                        match loaded {
                            Ok(loaded) => {
                                Self::apply_preset(&w, &loaded.preset);
                                show_load_warnings(&loaded.warnings);
                            }
                            Err(e) => {
                                tracing::warn!(
                                    path = %path.display(),
                                    error = %e,
                                    "Failed to load probe preset"
                                );
                                CamToolsView::show_error_dialog(
                                    &t!("Load Error"),
                                    &format!("{}: {}", path.display(), e),
                                );
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });
        // Probe / Apply to WCS
        let session = Rc::new(ProbeSession::default());
        apply_btn.set_sensitive(false);
        {
            let w = widgets.clone();
            let session = session.clone();
            let mc = machine_control.clone();
            let results_group = results_group.clone();
            let result_text = result_text.clone();
            let apply_btn = apply_btn.clone();
            let status_label = status_label.clone();
            probe_btn.connect_clicked(move |probe_btn| {
                Self::start_probe_run(
                    &w,
                    &session,
                    mc.as_ref(),
                    probe_btn,
                    &apply_btn,
                    &results_group,
                    &result_text,
                    &status_label,
                );
            });
        }
        {
            let w = widgets.clone();
            let session = session.clone();
            let mc = machine_control.clone();
            apply_btn.connect_clicked(move |_| {
                Self::apply_last_result(&w, &session, mc.as_ref());
            });
        }

        Self {
            content: content_box,
            current_routine: Cell::new(ROUTINE_Z_TOUCH),
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    /// Why the machine cannot accept a probe or WCS command now, if it cannot.
    fn machine_busy_reason(mc: &MachineControlView, session: &ProbeSession) -> Option<String> {
        let status = device_status::get_status();
        if !status.is_connected || !mc.communicator.lock().is_connected() {
            return Some(t!("The machine is not connected."));
        }
        if session.busy.get() {
            return Some(t!("A probe run is still in progress."));
        }
        let direct_running = mc
            .direct_sender
            .lock()
            .as_ref()
            .is_some_and(|s| s.is_running());
        if *mc.is_streaming.lock() || direct_running {
            return Some(t!("A job is running."));
        }
        if mc.quick_probe.get() != Default::default() {
            return Some(t!("A quick probe from the machine panel is in progress."));
        }
        if status.state != "Idle" {
            return Some(format!(
                "{} ({})",
                t!("The machine must be Idle"),
                status.state
            ));
        }
        None
    }

    fn set_result_text(result_text: &TextView, text: &str) {
        result_text.buffer().set_text(text);
    }

    /// Send the generated probe program and monitor it without blocking the main loop.
    #[allow(clippy::too_many_arguments)]
    fn start_probe_run(
        w: &ProbeWidgets,
        session: &Rc<ProbeSession>,
        mc: Option<&MachineControlView>,
        probe_btn: &Button,
        apply_btn: &Button,
        results_group: &PreferencesGroup,
        result_text: &TextView,
        status_label: &Label,
    ) {
        let Some(mc) = mc else {
            CamToolsView::show_error_dialog(&t!("Probe"), &t!("Machine control is not available."));
            return;
        };
        let Some(console) = mc.device_console.clone() else {
            CamToolsView::show_error_dialog(
                &t!("Probe"),
                &t!("The device console is needed to read probe reports."),
            );
            return;
        };
        if let Some(reason) = Self::machine_busy_reason(mc, session) {
            CamToolsView::show_error_dialog(&t!("Cannot Probe"), &reason);
            return;
        }
        let built = Self::build_routine(w).and_then(|r| Self::build_options(w).map(|o| (r, o)));
        let (routine, options) = match built {
            Ok(v) => v,
            Err(e) => {
                CamToolsView::show_error_dialog(&t!("Invalid Parameters"), &format!("{}", e));
                return;
            }
        };
        let current_pos = device_status::get_status()
            .machine_position
            .map(|p| Position::new(p.x as f32, p.y as f32, p.z as f32))
            .unwrap_or_default();
        let output = match ProbeRoutineEngine::with_options(routine.clone(), options)
            .generate(current_pos)
        {
            Ok(o) => o,
            Err(e) => {
                CamToolsView::show_error_dialog(&t!("Probe Generation Error"), &format!("{}", e));
                return;
            }
        };
        // Send code only: the streamer forwards lines as-is.
        let program: String = output
            .gcode
            .lines()
            .map(strip_comments)
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        let log_start = console.log_position();
        mc.start_job(&program);
        if !*mc.is_streaming.lock() {
            CamToolsView::show_error_dialog(
                &t!("Cannot Probe"),
                &t!("The probe program was not started."),
            );
            return;
        }
        tracing::info!(routine = %output.description, "Probe run started");

        session.busy.set(true);
        session.last.borrow_mut().take();
        probe_btn.set_sensitive(false);
        apply_btn.set_sensitive(false);
        results_group.set_visible(true);
        Self::set_result_text(
            result_text,
            &format!("{}: {}", t!("Probing"), output.description),
        );
        status_label.set_label("Status: ● Probing");

        // Main-loop monitor: weak widget refs, ends when the run is finished.
        let mc = mc.clone();
        let session = session.clone();
        let probe_btn = probe_btn.downgrade();
        let apply_btn = apply_btn.downgrade();
        let result_text = result_text.downgrade();
        let status_label = status_label.downgrade();
        let mut failed: Option<String> = None;
        glib::timeout_add_local(PROBE_MONITOR_INTERVAL, move || {
            let finish = |session: &ProbeSession| {
                session.busy.set(false);
                if let Some(b) = probe_btn.upgrade() {
                    b.set_sensitive(true);
                }
                if let Some(l) = status_label.upgrade() {
                    l.set_label("Status: ● Idle");
                }
            };
            let connected = mc.communicator.lock().is_connected();
            if !connected {
                if let Some(t) = result_text.upgrade() {
                    Self::set_result_text(&t, &t!("Probe aborted: the machine was disconnected."));
                }
                finish(&session);
                return glib::ControlFlow::Break;
            }

            if let Some(reason) = &failed {
                // The program stopped in G91; restore G90 once the user has cleared the
                // alarm and the controller is Idle again.
                if !*mc.is_streaming.lock() && device_status::get_status().state == "Idle" {
                    let _ = mc.communicator.lock().send_command("G90");
                    console.append_log("> G90\n");
                    tracing::warn!(reason = %reason, "Probe run failed; restored G90");
                    finish(&session);
                    return glib::ControlFlow::Break;
                }
                return glib::ControlFlow::Continue;
            }

            if *mc.is_streaming.lock() {
                return glib::ControlFlow::Continue;
            }

            let run = ProbeRunLog::parse(&console.log_text_since(log_start));
            let mut report = ProbeReport::new(routine.clone());
            for r in &run.reports {
                report.add_trigger(*r);
            }
            let outcome = match &run.failure {
                Some(line) => Err(format!("{}: {}", t!("Controller reported"), line)),
                None => ProbeRoutineEngine::compute_result_with(&report, &options)
                    .map_err(|e| e.to_string()),
            };
            match outcome {
                Ok(pos) => {
                    let text = Self::describe_result(&report, &options, pos);
                    if let Some(t) = result_text.upgrade() {
                        Self::set_result_text(&t, &text);
                    }
                    session.last.replace(Some((report, options)));
                    if let Some(b) = apply_btn.upgrade() {
                        b.set_sensitive(true);
                    }
                    finish(&session);
                    glib::ControlFlow::Break
                }
                Err(e) => {
                    if let Some(t) = result_text.upgrade() {
                        Self::set_result_text(
                            &t,
                            &format!(
                                "{}: {}\n{}",
                                t!("Probe failed"),
                                e,
                                t!("Clear any alarm ($X); absolute mode (G90) is \
                                     restored once the machine is Idle.")
                            ),
                        );
                    }
                    failed = Some(e);
                    glib::ControlFlow::Continue
                }
            }
        });
    }

    /// Human-readable result of a successful probe run.
    fn describe_result(report: &ProbeReport, options: &ProbeOptions, pos: Position) -> String {
        let mut text = match report.routine {
            ProbeRoutine::ZTouch { .. } => format!("Surface at machine Z{:.3}", pos.z),
            ProbeRoutine::EdgeFind { axis, .. } => {
                let v = match axis {
                    Axis::X => pos.x,
                    Axis::Y => pos.y,
                    Axis::Z => pos.z,
                };
                format!("Edge at machine {:?}{:.3}", axis, v)
            }
            ProbeRoutine::CornerFind { .. } => {
                format!("Corner at machine X{:.3} Y{:.3}", pos.x, pos.y)
            }
            ProbeRoutine::BoreCenter { .. } | ProbeRoutine::BossCenter { .. } => {
                let mut t = format!("Centre at machine X{:.3} Y{:.3}", pos.x, pos.y);
                if let Ok(d) = ProbeRoutineEngine::compute_diameter(report, options) {
                    t.push_str(&format!("\nMeasured diameter: {:.3} mm", d));
                }
                t
            }
            ProbeRoutine::ToolLength { .. } => {
                format!("Tool length offset: {:.3} mm", pos.z)
            }
        };
        text.push_str(&format!(
            "\n{} probe report(s) received.",
            report.triggers.len()
        ));
        text
    }

    /// Send the WCS (or tool-length) command for the last successful probe run.
    fn apply_last_result(
        w: &ProbeWidgets,
        session: &ProbeSession,
        mc: Option<&MachineControlView>,
    ) {
        let Some(mc) = mc else {
            CamToolsView::show_error_dialog(
                &t!("Apply to WCS"),
                &t!("Machine control is not available."),
            );
            return;
        };
        let Some((report, options)) = session.last.borrow().clone() else {
            CamToolsView::show_error_dialog(&t!("Apply to WCS"), &t!("Run a probe first."));
            return;
        };
        if let Some(reason) = Self::machine_busy_reason(mc, session) {
            CamToolsView::show_error_dialog(&t!("Cannot Apply"), &reason);
            return;
        }
        let target_wcs = w
            .wcs
            .active_id()
            .and_then(|id| id.trim_start_matches('G').parse::<u8>().ok())
            .unwrap_or(54);
        let mut service = WcsUpdateService::new(WcsUpdateConfig {
            target_wcs,
            probe_options: options,
            ..WcsUpdateConfig::default()
        });
        // G10 L2 with the probed machine coordinates: only the axes the routine measured.
        let result = match service.process_probe_report(&report) {
            Ok(r) => r,
            Err(e) => {
                CamToolsView::show_error_dialog(&t!("Cannot Apply"), &format!("{}", e));
                return;
            }
        };
        if let Err(e) = mc.communicator.lock().send_command(&result.command) {
            CamToolsView::show_error_dialog(&t!("Cannot Apply"), &format!("{}", e));
            return;
        }
        if let Some(c) = mc.device_console.as_ref() {
            c.append_log(&format!("> {}\n", result.command));
        }
        tracing::info!(command = %result.command, "Applied probe result");
    }

    /// Create an entry row whose value is always expressed in fixed (metric) units.
    fn create_mm_row(title: &str, value: f64, unit: &str) -> (ActionRow, Entry) {
        let row = ActionRow::builder().title(title).build();
        let box_ = Box::new(Orientation::Horizontal, 6);
        let entry = Entry::builder()
            .text(format!("{:.2}", value))
            .valign(Align::Center)
            .width_chars(8)
            .build();
        let label = Label::new(Some(unit));
        label.add_css_class("dim-label");
        box_.append(&entry);
        box_.append(&label);
        row.add_suffix(&box_);
        (row, entry)
    }

    /// Parse a millimetre (or mm/min) entry, naming the field on error.
    fn parse_mm(entry: &Entry, name: &str) -> anyhow::Result<f64> {
        let text = entry.text();
        text.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| anyhow::anyhow!("Invalid {}: '{}'", name, text))
    }

    fn build_options(w: &ProbeWidgets) -> anyhow::Result<ProbeOptions> {
        let set_wcs = if w.auto_update.is_active() {
            w.wcs
                .active_id()
                .and_then(|id| id.trim_start_matches('G').parse::<u8>().ok())
        } else {
            None
        };
        Ok(ProbeOptions {
            probe_depth: Self::parse_mm(&w.probe_depth, "probe depth")?,
            tip_radius: Self::parse_mm(&w.tip_radius, "probe tip radius")?,
            plate_thickness: Self::parse_mm(&w.plate_thickness, "plate thickness")?,
            set_wcs,
        })
    }

    fn build_routine(w: &ProbeWidgets) -> anyhow::Result<ProbeRoutine> {
        let routine_id = w.routine.active_id().unwrap_or_default();
        let safe_height = Self::parse_mm(&w.safe_height, "safe height")?;
        let fast_feed = Self::parse_mm(&w.fast_feed, "fast feed")?;
        let slow_feed = Self::parse_mm(&w.slow_feed, "slow feed")?;
        let backoff = Self::parse_mm(&w.backoff, "backoff")?;

        match routine_id.as_str() {
            "z_touch" => Ok(ProbeRoutine::ZTouch {
                safe_height,
                max_depth: Self::parse_mm(&w.max_depth, "max depth")?,
                fast_feed,
                slow_feed,
                backoff,
            }),
            "edge_find" => {
                let axis = match w.edge_axis.active_id().as_deref() {
                    Some("Y") => Axis::Y,
                    Some("Z") => Axis::Z,
                    _ => Axis::X,
                };
                let direction = match w.edge_dir.active_id().as_deref() {
                    Some("positive") => Direction::Positive,
                    _ => Direction::Negative,
                };
                Ok(ProbeRoutine::EdgeFind {
                    axis,
                    direction,
                    safe_height,
                    probe_distance: Self::parse_mm(&w.edge_dist, "probe distance")?,
                    fast_feed,
                    slow_feed,
                    backoff,
                })
            }
            "corner_find" => {
                let corner = match w.corner.active_id().as_deref() {
                    Some("xmax_ymin") => Corner::XmaxYmin,
                    Some("xmin_ymax") => Corner::XminYmax,
                    Some("xmax_ymax") => Corner::XmaxYmax,
                    _ => Corner::XminYmin,
                };
                Ok(ProbeRoutine::CornerFind {
                    corner,
                    safe_height,
                    probe_distance: Self::parse_mm(&w.edge_dist, "probe distance")?,
                    fast_feed,
                    slow_feed,
                    backoff,
                })
            }
            "bore_center" => Ok(ProbeRoutine::BoreCenter {
                diameter: Self::parse_mm(&w.diameter, "diameter")?,
                safe_height,
                fast_feed,
                slow_feed,
            }),
            "boss_center" => Ok(ProbeRoutine::BossCenter {
                diameter: Self::parse_mm(&w.diameter, "diameter")?,
                safe_height,
                fast_feed,
                slow_feed,
            }),
            "tool_length" => Ok(ProbeRoutine::ToolLength {
                plate_xy: (
                    Self::parse_mm(&w.setter_x, "setter X")?,
                    Self::parse_mm(&w.setter_y, "setter Y")?,
                ),
                plate_z: Self::parse_mm(&w.setter_z, "setter Z")?,
                safe_height,
                max_depth: Self::parse_mm(&w.max_depth, "max depth")?,
                feed_rate: fast_feed,
            }),
            _ => Err(anyhow::anyhow!("Unknown routine")),
        }
    }

    /// Read all fields into a preset (values in mm); fails on the first invalid field.
    fn collect_preset(w: &ProbeWidgets) -> anyhow::Result<ProbePreset> {
        let id = |c: &ComboBoxText| c.active_id().map(|s| s.to_string());
        Ok(ProbePreset {
            routine: id(&w.routine),
            safe_height: Self::parse_mm(&w.safe_height, "safe height")?,
            fast_feed: Self::parse_mm(&w.fast_feed, "fast feed")?,
            slow_feed: Self::parse_mm(&w.slow_feed, "slow feed")?,
            backoff: Self::parse_mm(&w.backoff, "backoff")?,
            max_depth: Self::parse_mm(&w.max_depth, "max depth")?,
            edge_axis: id(&w.edge_axis),
            edge_direction: id(&w.edge_dir),
            edge_distance: Self::parse_mm(&w.edge_dist, "probe distance")?,
            corner: id(&w.corner),
            diameter: Self::parse_mm(&w.diameter, "diameter")?,
            setter_x: Self::parse_mm(&w.setter_x, "setter X")?,
            setter_y: Self::parse_mm(&w.setter_y, "setter Y")?,
            setter_z: Self::parse_mm(&w.setter_z, "setter Z")?,
            probe_depth: Self::parse_mm(&w.probe_depth, "probe depth")?,
            tip_radius: Self::parse_mm(&w.tip_radius, "probe tip radius")?,
            plate_thickness: Self::parse_mm(&w.plate_thickness, "plate thickness")?,
            target_wcs: id(&w.wcs),
            auto_update_wcs: w.auto_update.is_active(),
            ..ProbePreset::default()
        })
    }

    fn apply_preset(w: &ProbeWidgets, p: &ProbePreset) {
        let mm = |e: &Entry, v: f64| e.set_text(&format!("{:.3}", v));
        if let Some(r) = &p.routine {
            w.routine.set_active_id(Some(r));
        }
        mm(&w.safe_height, p.safe_height);
        mm(&w.fast_feed, p.fast_feed);
        mm(&w.slow_feed, p.slow_feed);
        mm(&w.backoff, p.backoff);
        mm(&w.max_depth, p.max_depth);
        if let Some(v) = &p.edge_axis {
            w.edge_axis.set_active_id(Some(v));
        }
        if let Some(v) = &p.edge_direction {
            w.edge_dir.set_active_id(Some(v));
        }
        mm(&w.edge_dist, p.edge_distance);
        if let Some(v) = &p.corner {
            w.corner.set_active_id(Some(v));
        }
        mm(&w.diameter, p.diameter);
        mm(&w.setter_x, p.setter_x);
        mm(&w.setter_y, p.setter_y);
        mm(&w.setter_z, p.setter_z);
        mm(&w.probe_depth, p.probe_depth);
        mm(&w.tip_radius, p.tip_radius);
        mm(&w.plate_thickness, p.plate_thickness);
        if let Some(v) = &p.target_wcs {
            w.wcs.set_active_id(Some(v));
        }
        w.auto_update.set_active(p.auto_update_wcs);
    }
}
