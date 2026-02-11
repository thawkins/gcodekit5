//! Drill Press Tool

use gtk4::prelude::*;
use gtk4::{
    accessible::Property as AccessibleProperty, Align, Box, Button, CheckButton, Entry,
    FileChooserAction, FileChooserDialog, Image, Label, Orientation, Paned, ResponseType,
    ScrolledWindow, Stack,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use serde_json;
use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use super::common::{create_dimension_row, set_paned_initial_fraction};
use super::CamToolsView;
use crate::device_status;
use crate::t;
use crate::ui::gtk::help_browser;
use crate::ui::gtk::machine_control::MachineControlView;
use gcodekit5_camtools::drill_press::{DrillPressGenerator, DrillPressParameters};
use gcodekit5_core::units;
use gcodekit5_settings::SettingsController;

struct DrillPressWidgets {
    hole_diameter: Entry,
    tool_diameter: Entry,
    top_z: Entry,
    bottom_z: Entry,
    peck_depth: Entry,
    plunge_rate: Entry,
    feed_rate: Entry,
    spindle_speed: Entry,
    safe_z: Entry,
    x: Entry,
    y: Entry,
    home_before: CheckButton,
}

pub struct DrillPressTool {
    content: Box,
}

impl DrillPressTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        machine_control: Option<MachineControlView>,
        _on_generate: Rc<F>,
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
            .label("Drill Press")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);
        header.append(&help_browser::make_help_button("drill_press"));
        content_box.append(&header);

        // Paned Layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar (40%)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_label = Label::builder()
            .label("Drill Press")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Emulate a drill press on your CNC. Supports standard drilling, peck drilling for chip clearing, and helical interpolation for holes larger than the tool.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Sidebar Actions
        let sidebar_actions = Box::new(Orientation::Horizontal, 12);
        sidebar_actions.set_margin_top(24);
        sidebar_actions.set_halign(Align::Center);

        let drill_btn = Button::with_label("Drill");
        drill_btn.add_css_class("suggested-action");
        drill_btn.set_width_request(112);
        drill_btn.set_height_request(80);

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
        estop_btn.set_width_request(112);
        estop_btn.set_height_request(80);

        sidebar_actions.append(&drill_btn);
        sidebar_actions.append(&estop_btn);
        sidebar.append(&sidebar_actions);

        // Offline Message
        let offline_msg = Label::builder()
            .label("Device connection required to use Drill Press")
            .css_classes(vec!["title-3", "error"])
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .halign(Align::Center)
            .valign(Align::Center)
            .vexpand(true)
            .build();
        sidebar.append(&offline_msg);

        // Initial visibility
        let connected = device_status::get_status().is_connected;
        sidebar_actions.set_visible(connected);
        offline_msg.set_visible(!connected);

        // Connection status polling
        let sidebar_actions_clone = sidebar_actions.clone();
        let offline_msg_clone = offline_msg.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            let connected = device_status::get_status().is_connected;
            sidebar_actions_clone.set_visible(connected);
            offline_msg_clone.set_visible(!connected);
            glib::ControlFlow::Continue
        });

        // Content Area
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Create widgets
        let (hole_dia_row, hole_diameter, hole_dia_unit) =
            create_dimension_row("Hole Diameter:", 10.0, &settings);
        let (tool_dia_row, tool_diameter, tool_dia_unit) =
            create_dimension_row("Tool Diameter:", 6.0, &settings);
        let (top_z_row, top_z, top_z_unit) =
            create_dimension_row("Top Z (Surface):", 0.0, &settings);
        let (bottom_z_row, bottom_z, bottom_z_unit) =
            create_dimension_row("Bottom Z (Depth):", -10.0, &settings);
        let (peck_row, peck_depth, peck_unit) =
            create_dimension_row("Peck Depth (0 for none):", 2.0, &settings);
        let (safe_z_row, safe_z, safe_z_unit) = create_dimension_row("Safe Z:", 5.0, &settings);

        // Default X/Y to device center if available
        let (center_x, center_y) = {
            let x_max = device_status::get_grbl_setting_numeric(130).unwrap_or(200.0);
            let y_max = device_status::get_grbl_setting_numeric(131).unwrap_or(200.0);
            (x_max / 2.0, y_max / 2.0)
        };

        let (x_row, x, x_unit) = create_dimension_row("Center X:", center_x, &settings);
        let (y_row, y, y_unit) = create_dimension_row("Center Y:", center_y, &settings);

        let plunge_rate = Entry::builder().text("100").valign(Align::Center).build();
        let feed_rate = Entry::builder().text("500").valign(Align::Center).build();
        let spindle_speed = Entry::builder().text("10000").valign(Align::Center).build();

        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let hole_group = PreferencesGroup::builder().title("Hole Geometry").build();
        hole_group.add(&hole_dia_row);
        hole_group.add(&tool_dia_row);
        hole_group.add(&x_row);
        hole_group.add(&y_row);
        scroll_content.append(&hole_group);

        let depth_group = PreferencesGroup::builder()
            .title("Depth and Pecking")
            .build();
        depth_group.add(&top_z_row);
        depth_group.add(&bottom_z_row);
        depth_group.add(&peck_row);
        scroll_content.append(&depth_group);

        let machine_group = PreferencesGroup::builder()
            .title("Machine Settings")
            .build();
        machine_group.add(&Self::create_row("Plunge Rate (mm/min):", &plunge_rate));
        machine_group.add(&Self::create_row("Feed Rate (mm/min):", &feed_rate));
        machine_group.add(&Self::create_row("Spindle Speed (RPM):", &spindle_speed));
        machine_group.add(&safe_z_row);

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        machine_group.add(&home_row);

        scroll_content.append(&machine_group);

        right_panel.append(&scrolled);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label("Load");
        let save_btn = Button::with_label("Save");
        let cancel_btn = Button::with_label("Cancel");
        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        set_paned_initial_fraction(&paned, 0.40);

        content_box.append(&paned);

        let widgets = Rc::new(DrillPressWidgets {
            hole_diameter,
            tool_diameter,
            top_z,
            bottom_z,
            peck_depth,
            plunge_rate,
            feed_rate,
            spindle_speed,
            safe_z,
            x,
            y,
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let hole_dia_unit = hole_dia_unit.clone();
            let tool_dia_unit = tool_dia_unit.clone();
            let top_z_unit = top_z_unit.clone();
            let bottom_z_unit = bottom_z_unit.clone();
            let peck_unit = peck_unit.clone();
            let safe_z_unit = safe_z_unit.clone();
            let x_unit = x_unit.clone();
            let y_unit = y_unit.clone();

            let last_system = Rc::new(Cell::new(
                settings.persistence.borrow().config().ui.measurement_system,
            ));

            settings.on_setting_changed(move |key, _| {
                if key == "measurement_system" {
                    let new_system = settings_clone
                        .persistence
                        .borrow()
                        .config()
                        .ui
                        .measurement_system;
                    let old_system = last_system.get();

                    if new_system != old_system {
                        let unit_label = units::get_unit_label(new_system);

                        let update_entry = |entry: &Entry, label: &Label| {
                            if let Ok(val_mm) = units::parse_length(&entry.text(), old_system) {
                                entry.set_text(&units::format_length(val_mm, new_system));
                            }
                            label.set_text(unit_label);
                        };

                        update_entry(&w.hole_diameter, &hole_dia_unit);
                        update_entry(&w.tool_diameter, &tool_dia_unit);
                        update_entry(&w.top_z, &top_z_unit);
                        update_entry(&w.bottom_z, &bottom_z_unit);
                        update_entry(&w.peck_depth, &peck_unit);
                        update_entry(&w.safe_z, &safe_z_unit);
                        update_entry(&w.x, &x_unit);
                        update_entry(&w.y, &y_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Drill button
        let w_run = widgets.clone();
        let settings_run = settings.clone();
        let mc_run = machine_control.clone();
        drill_btn.connect_clicked(move |_| {
            let status = device_status::get_status();
            if !status.is_connected {
                CamToolsView::show_error_dialog(
                    "Device Offline",
                    "Please connect to a CNC machine before running the drill press tool.",
                );
                return;
            }

            // Check if laser mode is enabled ($32=1)
            if device_status::get_grbl_setting_numeric(32).unwrap_or(0.0) > 0.5 {
                CamToolsView::show_error_dialog(
                    "Laser Mode Enabled",
                    "The device is currently in Laser Mode ($32=1). Please switch to CNC mode to use the Drill Press tool.",
                );
                return;
            }

            let home_before = w_run.home_before.is_active();
            let system = settings_run
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system;

            // Drill press requires Z axis
            if device_status::get_active_num_axes() < 3 {
                CamToolsView::show_error_dialog(
                    "Insufficient Axes",
                    "The Drill Press tool requires at least 3 axes (X, Y, Z). The active device has fewer than 3 axes configured.",
                );
                return;
            }

            let params = DrillPressParameters {
                hole_diameter: units::parse_length(&w_run.hole_diameter.text(), system)
                    .unwrap_or(10.0) as f64,
                tool_diameter: units::parse_length(&w_run.tool_diameter.text(), system)
                    .unwrap_or(6.0) as f64,
                top_z: units::parse_length(&w_run.top_z.text(), system).unwrap_or(0.0) as f64,
                bottom_z: units::parse_length(&w_run.bottom_z.text(), system).unwrap_or(-10.0)
                    as f64,
                peck_depth: units::parse_length(&w_run.peck_depth.text(), system).unwrap_or(2.0)
                    as f64,
                plunge_rate: w_run.plunge_rate.text().parse().unwrap_or(100.0),
                feed_rate: w_run.feed_rate.text().parse().unwrap_or(500.0),
                spindle_speed: w_run.spindle_speed.text().parse().unwrap_or(10000.0),
                safe_z: units::parse_length(&w_run.safe_z.text(), system).unwrap_or(5.0) as f64,
                x: units::parse_length(&w_run.x.text(), system).unwrap_or(0.0) as f64,
                y: units::parse_length(&w_run.y.text(), system).unwrap_or(0.0) as f64,
            };

            let generator = DrillPressGenerator::new(params);
            match generator.generate() {
                Ok(mut gcode) => {
                    gcode = gcode.replace("$H\n", "").replace("$H", "");
                    if home_before {
                        gcode = format!("$H\n{}", gcode);
                    }

                    if let Some(mc) = mc_run.as_ref() {
                        mc.start_job(&gcode);
                    } else {
                        CamToolsView::show_error_dialog(
                            "Execution Failed",
                            "Machine control interface not available.",
                        );
                    }
                }
                Err(e) => {
                    CamToolsView::show_error_dialog(
                        "Generation Failed",
                        &format!("Failed to generate drill press toolpath: {}", e),
                    );
                }
            }
        });

        // eStop button
        let mc_estop = machine_control.clone();
        estop_btn.connect_clicked(move |_| {
            if let Some(mc) = mc_estop.as_ref() {
                mc.emergency_stop();
            }
        });

        // Save/Load/Cancel
        let w_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            Self::save_params(&w_save, &settings_save);
        });

        let w_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            Self::load_params(&w_load, &settings_load);
        });

        let stack_clone_cancel = stack.clone();
        cancel_btn.connect_clicked(move |_| {
            stack_clone_cancel.set_visible_child_name("dashboard");
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn create_row(title: &str, widget: &impl IsA<gtk4::Widget>) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        row.add_suffix(widget);
        row
    }

    fn save_params(w: &DrillPressWidgets, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Save Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);
        dialog.set_current_name("drill_params.json");

        let system = settings.persistence.borrow().config().ui.measurement_system;
        let params = DrillPressParameters {
            hole_diameter: units::parse_length(&w.hole_diameter.text(), system).unwrap_or(10.0)
                as f64,
            tool_diameter: units::parse_length(&w.tool_diameter.text(), system).unwrap_or(6.0)
                as f64,
            top_z: units::parse_length(&w.top_z.text(), system).unwrap_or(0.0) as f64,
            bottom_z: units::parse_length(&w.bottom_z.text(), system).unwrap_or(-10.0) as f64,
            peck_depth: units::parse_length(&w.peck_depth.text(), system).unwrap_or(2.0) as f64,
            plunge_rate: w.plunge_rate.text().parse().unwrap_or(100.0),
            feed_rate: w.feed_rate.text().parse().unwrap_or(500.0),
            spindle_speed: w.spindle_speed.text().parse().unwrap_or(10000.0),
            safe_z: units::parse_length(&w.safe_z.text(), system).unwrap_or(5.0) as f64,
            x: units::parse_length(&w.x.text(), system).unwrap_or(0.0) as f64,
            y: units::parse_length(&w.y.text(), system).unwrap_or(0.0) as f64,
        };

        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(json) = serde_json::to_string_pretty(&params) {
                            let _ = fs::write(path, json);
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn load_params(w: &DrillPressWidgets, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Load Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Open,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        let w_clone = Rc::new((
            w.hole_diameter.clone(),
            w.tool_diameter.clone(),
            w.top_z.clone(),
            w.bottom_z.clone(),
            w.peck_depth.clone(),
            w.plunge_rate.clone(),
            w.feed_rate.clone(),
            w.spindle_speed.clone(),
            w.safe_z.clone(),
            w.x.clone(),
            w.y.clone(),
        ));
        let settings_clone = settings.clone();

        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(content) = fs::read_to_string(path) {
                            if let Ok(params) =
                                serde_json::from_str::<DrillPressParameters>(&content)
                            {
                                Self::apply_params(&w_clone, &params, &settings_clone);
                            }
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn apply_params(
        w: &(
            Entry,
            Entry,
            Entry,
            Entry,
            Entry,
            Entry,
            Entry,
            Entry,
            Entry,
            Entry,
            Entry,
        ),
        p: &DrillPressParameters,
        settings: &Rc<SettingsController>,
    ) {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        w.0.set_text(&units::format_length(p.hole_diameter as f32, system));
        w.1.set_text(&units::format_length(p.tool_diameter as f32, system));
        w.2.set_text(&units::format_length(p.top_z as f32, system));
        w.3.set_text(&units::format_length(p.bottom_z as f32, system));
        w.4.set_text(&units::format_length(p.peck_depth as f32, system));
        w.5.set_text(&p.plunge_rate.to_string());
        w.6.set_text(&p.feed_rate.to_string());
        w.7.set_text(&p.spindle_speed.to_string());
        w.8.set_text(&units::format_length(p.safe_z as f32, system));
        w.9.set_text(&units::format_length(p.x as f32, system));
        w.10.set_text(&units::format_length(p.y as f32, system));
    }
}
