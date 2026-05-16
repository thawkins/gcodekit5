//! Probe Routines Tool - Touch probe CAM tool dashboard
//!
//! Provides a UI for configuring and running probe routines including:
//! - Z-Touch (surface probe)
//! - Edge Find (single axis)
//! - Corner Find (two-axis)
//! - Bore Center (4-point internal)
//! - Boss Center (4-point external)
//! - Tool Length measurement

use glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, FileChooserAction, Label, Orientation,
    Paned, ResponseType, ScrolledWindow, Stack, TextView, WrapMode,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::Cell;
use std::rc::Rc;

use super::common::{create_dimension_row, set_paned_initial_fraction};
use super::CamToolsView;
use crate::device_status;
use crate::t;
use crate::ui::gtk::machine_control::MachineControlView;
use gcodekit5_camtools::probe_routines::{defaults as probe_defaults, ProbeRoutineEngine};
// WCS service module available when needed
use gcodekit5_core::{
    Corner, Position, ProbeAxis as Axis, ProbeDirection as Direction, ProbeRoutine,
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

pub struct ProbeRoutinesTool {
    content: Box,
    #[allow(dead_code)]
    current_routine: Cell<RoutineType>,
}

impl ProbeRoutinesTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        _machine_control: Option<MachineControlView>,
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

        // Connection status polling
        let offline_msg_clone = offline_msg.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            let connected = device_status::get_status().is_connected;
            offline_msg_clone.set_visible(!connected);
            glib::ControlFlow::Continue
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
        let (safe_height_row, safe_height, _safe_unit) =
            create_dimension_row("Safe Height:", probe_defaults::SAFE_HEIGHT, &settings);
        let (fast_feed_row, fast_feed, _fast_unit) =
            create_dimension_row("Fast Feed Rate:", probe_defaults::FAST_FEED, &settings);
        let (slow_feed_row, slow_feed, _slow_unit) =
            create_dimension_row("Slow Feed Rate:", probe_defaults::SLOW_FEED, &settings);
        let (backoff_row, backoff, _backoff_unit) =
            create_dimension_row("Backoff Distance:", probe_defaults::BACKOFF, &settings);

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
            .subtitle("Automatically apply probe result to selected WCS")
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

        // Wire up routine selection visibility
        let _specific_params_clone = specific_params.clone();
        let edge_axis_row_clone = edge_axis_row.clone();
        let edge_dir_row_clone = edge_dir_row.clone();
        let edge_dist_row_clone = edge_dist_row.clone();
        let corner_row_clone = corner_row.clone();
        let diameter_row_clone = diameter_row.clone();
        let setter_x_row_clone = setter_x_row.clone();
        let setter_y_row_clone = setter_y_row.clone();
        let setter_z_row_clone = setter_z_row.clone();

        routine_combo.connect_changed(move |combo| {
            let id = combo.active_id().map(|s| s.as_str().to_string());

            // Hide all specific rows first
            edge_axis_row_clone.set_visible(false);
            edge_dir_row_clone.set_visible(false);
            edge_dist_row_clone.set_visible(false);
            corner_row_clone.set_visible(false);
            diameter_row_clone.set_visible(false);
            setter_x_row_clone.set_visible(false);
            setter_y_row_clone.set_visible(false);
            setter_z_row_clone.set_visible(false);

            // Show relevant rows based on selection
            match id.as_deref() {
                Some("edge_find") => {
                    edge_axis_row_clone.set_visible(true);
                    edge_dir_row_clone.set_visible(true);
                    edge_dist_row_clone.set_visible(true);
                }
                Some("corner_find") => {
                    corner_row_clone.set_visible(true);
                }
                Some("bore_center") | Some("boss_center") => {
                    diameter_row_clone.set_visible(true);
                }
                Some("tool_length") => {
                    setter_x_row_clone.set_visible(true);
                    setter_y_row_clone.set_visible(true);
                    setter_z_row_clone.set_visible(true);
                }
                _ => {}
            }
        });

        // Generate G-code handler
        let _generate_btn_clone = generate_btn.clone();
        let safe_height_clone = safe_height.clone();
        let fast_feed_clone = fast_feed.clone();
        let slow_feed_clone = slow_feed.clone();
        let backoff_clone = backoff.clone();
        let max_depth_clone = max_depth_entry.clone();
        let routine_combo_clone = routine_combo.clone();
        let edge_axis_clone = edge_axis_combo.clone();
        let edge_dir_clone = edge_dir_combo.clone();
        let edge_dist_clone = edge_dist_entry.clone();
        let corner_clone = corner_combo.clone();
        let diameter_clone = diameter_entry.clone();
        let setter_x_clone = setter_x_entry.clone();
        let setter_y_clone = setter_y_entry.clone();
        let setter_z_clone = setter_z_entry.clone();
        let on_generate_clone = on_generate.clone();

        generate_btn.clone().connect_clicked(move |_| {
            let routine = Self::build_routine(
                &routine_combo_clone,
                &safe_height_clone,
                &fast_feed_clone,
                &slow_feed_clone,
                &backoff_clone,
                &max_depth_clone,
                &edge_axis_clone,
                &edge_dir_clone,
                &edge_dist_clone,
                &corner_clone,
                &diameter_clone,
                &setter_x_clone,
                &setter_y_clone,
                &setter_z_clone,
            );

            match routine {
                Ok(r) => {
                    let engine = ProbeRoutineEngine::new(r);
                    let current_pos = Position::new(0.0, 0.0, 5.0); // Default position

                    match engine.generate(current_pos) {
                        Ok(output) => {
                            on_generate_clone(output.gcode);
                        }
                        Err(e) => {
                            CamToolsView::show_error_dialog(
                                &t!("Probe Generation Error"),
                                &format!("{}", e),
                            );
                        }
                    }
                }
                Err(e) => {
                    CamToolsView::show_error_dialog(&t!("Invalid Parameters"), &format!("{}", e));
                }
            }
        });

        // Wire up Save button
        let safe_height_save = safe_height.clone();
        let fast_feed_save = fast_feed.clone();
        let slow_feed_save = slow_feed.clone();
        let backoff_save = backoff.clone();
        let max_depth_save = max_depth_entry.clone();
        let routine_combo_save = routine_combo.clone();
        let edge_axis_save = edge_axis_combo.clone();
        let edge_dir_save = edge_dir_combo.clone();
        let edge_dist_save = edge_dist_entry.clone();
        let corner_save = corner_combo.clone();
        let diameter_save = diameter_entry.clone();
        let setter_x_save = setter_x_entry.clone();
        let setter_y_save = setter_y_entry.clone();
        let setter_z_save = setter_z_entry.clone();

        save_btn.clone().connect_clicked(move |_| {
            let params = Self::build_params_json(
                &routine_combo_save,
                &safe_height_save,
                &fast_feed_save,
                &slow_feed_save,
                &backoff_save,
                &max_depth_save,
                &edge_axis_save,
                &edge_dir_save,
                &edge_dist_save,
                &corner_save,
                &diameter_save,
                &setter_x_save,
                &setter_y_save,
                &setter_z_save,
            );

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

            if let Some(ref w) = window {
                dialog.set_transient_for(Some(w));
            }

            let params_clone = params;
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            if let Err(e) = std::fs::write(&path, &params_clone) {
                                CamToolsView::show_error_dialog(
                                    &t!("Save Error"),
                                    &format!("{}", e),
                                );
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        }); // Wire up Load button
        let safe_height_load = safe_height.clone();
        let fast_feed_load = fast_feed.clone();
        let slow_feed_load = slow_feed.clone();
        let backoff_load = backoff.clone();
        let max_depth_load = max_depth_entry.clone();
        let routine_combo_load = routine_combo.clone();
        let edge_axis_load = edge_axis_combo.clone();
        let edge_dir_load = edge_dir_combo.clone();
        let edge_dist_load = edge_dist_entry.clone();
        let corner_load = corner_combo.clone();
        let diameter_load = diameter_entry.clone();
        let setter_x_load = setter_x_entry.clone();
        let setter_y_load = setter_y_entry.clone();
        let setter_z_load = setter_z_entry.clone();

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

            let routine_combo_c = routine_combo_load.clone();
            let safe_height_c = safe_height_load.clone();
            let fast_feed_c = fast_feed_load.clone();
            let slow_feed_c = slow_feed_load.clone();
            let backoff_c = backoff_load.clone();
            let max_depth_c = max_depth_load.clone();
            let edge_axis_c = edge_axis_load.clone();
            let edge_dir_c = edge_dir_load.clone();
            let edge_dist_c = edge_dist_load.clone();
            let corner_c = corner_load.clone();
            let diameter_c = diameter_load.clone();
            let setter_x_c = setter_x_load.clone();
            let setter_y_c = setter_y_load.clone();
            let setter_z_c = setter_z_load.clone();

            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            match std::fs::read_to_string(&path) {
                                Ok(json) => {
                                    Self::apply_params_json(
                                        &json,
                                        &routine_combo_c,
                                        &safe_height_c,
                                        &fast_feed_c,
                                        &slow_feed_c,
                                        &backoff_c,
                                        &max_depth_c,
                                        &edge_axis_c,
                                        &edge_dir_c,
                                        &edge_dist_c,
                                        &corner_c,
                                        &diameter_c,
                                        &setter_x_c,
                                        &setter_y_c,
                                        &setter_z_c,
                                    );
                                }
                                Err(e) => {
                                    CamToolsView::show_error_dialog(
                                        &t!("Load Error"),
                                        &format!("{}", e),
                                    );
                                }
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });
        Self {
            content: content_box,
            current_routine: Cell::new(ROUTINE_Z_TOUCH),
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    #[allow(clippy::too_many_arguments)]
    fn build_routine(
        routine_combo: &ComboBoxText,
        safe_height: &Entry,
        fast_feed: &Entry,
        slow_feed: &Entry,
        backoff: &Entry,
        max_depth: &Entry,
        edge_axis: &ComboBoxText,
        edge_dir: &ComboBoxText,
        edge_dist: &Entry,
        corner: &ComboBoxText,
        diameter: &Entry,
        setter_x: &Entry,
        setter_y: &Entry,
        setter_z: &Entry,
    ) -> anyhow::Result<ProbeRoutine> {
        let routine_id = routine_combo.active_id().unwrap_or_default();
        let safe_height_val = safe_height
            .text()
            .parse::<f64>()
            .map_err(|_| anyhow::anyhow!("Invalid safe height"))?;
        let fast_feed_val = fast_feed
            .text()
            .parse::<f64>()
            .map_err(|_| anyhow::anyhow!("Invalid fast feed"))?;
        let slow_feed_val = slow_feed
            .text()
            .parse::<f64>()
            .map_err(|_| anyhow::anyhow!("Invalid slow feed"))?;
        let backoff_val = backoff
            .text()
            .parse::<f64>()
            .map_err(|_| anyhow::anyhow!("Invalid backoff"))?;

        match routine_id.as_str() {
            "z_touch" => Ok(ProbeRoutine::ZTouch {
                safe_height: safe_height_val,
                max_depth: max_depth
                    .text()
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("Invalid max depth"))?,
                fast_feed: fast_feed_val,
                slow_feed: slow_feed_val,
                backoff: backoff_val,
            }),
            "edge_find" => {
                let axis = match edge_axis.active_id().as_deref() {
                    Some("Y") => Axis::Y,
                    Some("Z") => Axis::Z,
                    _ => Axis::X,
                };
                let direction = match edge_dir.active_id().as_deref() {
                    Some("positive") => Direction::Positive,
                    _ => Direction::Negative,
                };
                Ok(ProbeRoutine::EdgeFind {
                    axis,
                    direction,
                    safe_height: safe_height_val,
                    probe_distance: edge_dist
                        .text()
                        .parse::<f64>()
                        .map_err(|_| anyhow::anyhow!("Invalid probe distance"))?,
                    fast_feed: fast_feed_val,
                    slow_feed: slow_feed_val,
                    backoff: backoff_val,
                })
            }
            "corner_find" => {
                let corner = match corner.active_id().as_deref() {
                    Some("xmax_ymin") => Corner::XmaxYmin,
                    Some("xmin_ymax") => Corner::XminYmax,
                    Some("xmax_ymax") => Corner::XmaxYmax,
                    _ => Corner::XminYmin,
                };
                Ok(ProbeRoutine::CornerFind {
                    corner,
                    safe_height: safe_height_val,
                    probe_distance: edge_dist
                        .text()
                        .parse::<f64>()
                        .map_err(|_| anyhow::anyhow!("Invalid probe distance"))?,
                    fast_feed: fast_feed_val,
                    slow_feed: slow_feed_val,
                    backoff: backoff_val,
                })
            }
            "bore_center" => Ok(ProbeRoutine::BoreCenter {
                diameter: diameter
                    .text()
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("Invalid diameter"))?,
                safe_height: safe_height_val,
                fast_feed: fast_feed_val,
                slow_feed: slow_feed_val,
            }),
            "boss_center" => Ok(ProbeRoutine::BossCenter {
                diameter: diameter
                    .text()
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("Invalid diameter"))?,
                safe_height: safe_height_val,
                fast_feed: fast_feed_val,
                slow_feed: slow_feed_val,
            }),
            "tool_length" => Ok(ProbeRoutine::ToolLength {
                plate_xy: (
                    setter_x
                        .text()
                        .parse::<f64>()
                        .map_err(|_| anyhow::anyhow!("Invalid setter X"))?,
                    setter_y
                        .text()
                        .parse::<f64>()
                        .map_err(|_| anyhow::anyhow!("Invalid setter Y"))?,
                ),
                plate_z: setter_z
                    .text()
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("Invalid setter Z"))?,
                safe_height: safe_height_val,
                max_depth: max_depth
                    .text()
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("Invalid max depth"))?,
                feed_rate: fast_feed_val,
            }),
            _ => Err(anyhow::anyhow!("Unknown routine")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_params_json(
        routine_combo: &ComboBoxText,
        safe_height: &Entry,
        fast_feed: &Entry,
        slow_feed: &Entry,
        backoff: &Entry,
        max_depth: &Entry,
        edge_axis: &ComboBoxText,
        edge_dir: &ComboBoxText,
        edge_dist: &Entry,
        corner: &ComboBoxText,
        diameter: &Entry,
        setter_x: &Entry,
        setter_y: &Entry,
        setter_z: &Entry,
    ) -> String {
        let routine = routine_combo.active_id().map(|s| s.to_string());
        let edge_axis = edge_axis.active_id().map(|s| s.to_string());
        let edge_dir = edge_dir.active_id().map(|s| s.to_string());
        let corner_id = corner.active_id().map(|s| s.to_string());

        serde_json::json!({
            "routine": routine,
            "safe_height": safe_height.text().to_string(),
            "fast_feed": fast_feed.text().to_string(),
            "slow_feed": slow_feed.text().to_string(),
            "backoff": backoff.text().to_string(),
            "max_depth": max_depth.text().to_string(),
            "edge_axis": edge_axis,
            "edge_direction": edge_dir,
            "edge_distance": edge_dist.text().to_string(),
            "corner": corner_id,
            "diameter": diameter.text().to_string(),
            "setter_x": setter_x.text().to_string(),
            "setter_y": setter_y.text().to_string(),
            "setter_z": setter_z.text().to_string(),
        })
        .to_string()
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_params_json(
        json: &str,
        routine_combo: &ComboBoxText,
        safe_height: &Entry,
        fast_feed: &Entry,
        slow_feed: &Entry,
        backoff: &Entry,
        max_depth: &Entry,
        edge_axis: &ComboBoxText,
        edge_dir: &ComboBoxText,
        edge_dist: &Entry,
        corner: &ComboBoxText,
        diameter: &Entry,
        setter_x: &Entry,
        setter_y: &Entry,
        setter_z: &Entry,
    ) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(routine) = value.get("routine").and_then(|v| v.as_str()) {
                routine_combo.set_active_id(Some(routine));
            }
            if let Some(v) = value.get("safe_height").and_then(|v| v.as_str()) {
                safe_height.set_text(v);
            }
            if let Some(v) = value.get("fast_feed").and_then(|v| v.as_str()) {
                fast_feed.set_text(v);
            }
            if let Some(v) = value.get("slow_feed").and_then(|v| v.as_str()) {
                slow_feed.set_text(v);
            }
            if let Some(v) = value.get("backoff").and_then(|v| v.as_str()) {
                backoff.set_text(v);
            }
            if let Some(v) = value.get("max_depth").and_then(|v| v.as_str()) {
                max_depth.set_text(v);
            }
            if let Some(v) = value.get("edge_axis").and_then(|v| v.as_str()) {
                edge_axis.set_active_id(Some(v));
            }
            if let Some(v) = value.get("edge_direction").and_then(|v| v.as_str()) {
                edge_dir.set_active_id(Some(v));
            }
            if let Some(v) = value.get("edge_distance").and_then(|v| v.as_str()) {
                edge_dist.set_text(v);
            }
            if let Some(v) = value.get("corner").and_then(|v| v.as_str()) {
                corner.set_active_id(Some(v));
            }
            if let Some(v) = value.get("diameter").and_then(|v| v.as_str()) {
                diameter.set_text(v);
            }
            if let Some(v) = value.get("setter_x").and_then(|v| v.as_str()) {
                setter_x.set_text(v);
            }
            if let Some(v) = value.get("setter_y").and_then(|v| v.as_str()) {
                setter_y.set_text(v);
            }
            if let Some(v) = value.get("setter_z").and_then(|v| v.as_str()) {
                setter_z.set_text(v);
            }
        }
    }
}
