//! Spoilboard Grid Tool

use crate::ui::gtk::common::text_dropdown::{new_text_dropdown, TextDropDownExt};
use crate::ui::gtk::file_dialog::FileDialogHelpers;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, DropDown, Entry, Label, Orientation, Paned, ScrolledWindow,
    Stack,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use serde_json;
use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use super::common::{create_dimension_row, set_paned_initial_fraction};
use super::CamToolsView;
use crate::ui::gtk::help_browser;
use gcodekit5_camtools::spoilboard_grid::{SpoilboardGridGenerator, SpoilboardGridParameters};
use gcodekit5_core::units;
use gcodekit5_settings::SettingsController;

struct SpoilboardGridWidgets {
    width: Entry,
    height: Entry,
    grid_spacing: Entry,
    feed_rate: Entry,
    laser_power: Entry,
    laser_mode: DropDown,
    home_before: CheckButton,
}

pub struct SpoilboardGridTool {
    content: Box,
}

impl SpoilboardGridTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
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
            .label("Spoilboard Grid")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);
        header.append(&help_browser::make_help_button("spoilboard_grid"));
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
            .label("Spoilboard Grid")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Create a grid pattern on your spoilboard for easy workpiece alignment and fixturing.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Content Area
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Create widgets
        let (width_row, width, width_unit) = create_dimension_row("Width:", 400.0, &settings);
        let (height_row, height, height_unit) = create_dimension_row("Height:", 300.0, &settings);
        let (grid_spacing_row, grid_spacing, grid_spacing_unit) =
            create_dimension_row("Grid Spacing:", 10.0, &settings);
        let feed_rate = Entry::builder().text("1000").valign(Align::Center).build();
        let laser_power = Entry::builder().text("1000").valign(Align::Center).build();

        let laser_mode = new_text_dropdown();
        laser_mode.append(Some("M3"), "M3 (Constant Power)");
        laser_mode.append(Some("M4"), "M4 (Dynamic Power)");
        laser_mode.set_active_id(Some("M4"));
        laser_mode.set_valign(Align::Center);

        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let dim_group = PreferencesGroup::builder()
            .title("Spoilboard Dimensions")
            .build();
        dim_group.add(&width_row);
        dim_group.add(&height_row);
        dim_group.add(&grid_spacing_row);
        scroll_content.append(&dim_group);

        let laser_group = PreferencesGroup::builder().title("Laser Settings").build();
        laser_group.add(&Self::create_row("Laser Power (S):", &laser_power));
        laser_group.add(&Self::create_row("Feed Rate (mm/min):", &feed_rate));
        laser_group.add(&Self::create_row("Laser Mode:", &laser_mode));

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        laser_group.add(&home_row);

        scroll_content.append(&laser_group);

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
        let generate_btn = Button::with_label("Generate");
        generate_btn.add_css_class("suggested-action");
        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        content_box.append(&paned);

        let widgets = Rc::new(SpoilboardGridWidgets {
            width,
            height,
            grid_spacing,
            feed_rate,
            laser_power,
            laser_mode,
            home_before,
        });

        // Unit update listener
        {
            let w = widgets.clone();
            let width_unit = width_unit.clone();
            let height_unit = height_unit.clone();
            let grid_spacing_unit = grid_spacing_unit.clone();

            let last_system = Rc::new(Cell::new(
                settings.persistence.borrow().config().ui.measurement_system,
            ));

            settings.on_setting_changed(move |key, value| {
                if key == "measurement_system" {
                    // Listeners fire when the setting is edited, before it is saved to
                    // persistence, so the new system must be taken from `value`.
                    let Ok(new_system) = value.parse::<units::MeasurementSystem>() else {
                        return;
                    };
                    let old_system = last_system.get();

                    if new_system != old_system {
                        let unit_label = units::get_unit_label(new_system);

                        let update_entry = |entry: &Entry, label: &Label| {
                            if let Ok(val_mm) = units::parse_length(&entry.text(), old_system) {
                                entry.set_text(&units::format_length(val_mm, new_system));
                            }
                            label.set_text(unit_label);
                        };

                        update_entry(&w.width, &width_unit);
                        update_entry(&w.height, &height_unit);
                        update_entry(&w.grid_spacing, &grid_spacing_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Generate button
        let w_gen = widgets.clone();
        let settings_gen = settings.clone();
        generate_btn.connect_clicked(move |_| {
            let home_before = w_gen.home_before.is_active();
            let laser_mode_str = w_gen
                .laser_mode
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "M4".to_string());

            let system = settings_gen
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system;

            let params = SpoilboardGridParameters {
                width: units::parse_length(&w_gen.width.text(), system).unwrap_or(400.0) as f64,
                height: units::parse_length(&w_gen.height.text(), system).unwrap_or(300.0) as f64,
                grid_spacing: units::parse_length(&w_gen.grid_spacing.text(), system)
                    .unwrap_or(10.0) as f64,
                feed_rate: w_gen.feed_rate.text().parse().unwrap_or(1000.0),
                laser_power: w_gen.laser_power.text().parse().unwrap_or(1000.0),
                laser_mode: laser_mode_str,
                home_before_job: home_before,
            };

            let generator = SpoilboardGridGenerator::new(params);
            match generator.generate() {
                Ok(gcode) => {
                    on_generate(gcode);
                }
                Err(e) => {
                    CamToolsView::show_error_dialog(
                        "Generation Failed",
                        &format!("Failed to generate grid pattern: {}", e),
                    );
                }
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

    fn save_params(w: &SpoilboardGridWidgets, settings: &Rc<SettingsController>) {
        let dialog = crate::ui::gtk::file_dialog::new_file_dialog("Save Parameters");
        dialog.set_initial_name(Some("grid_params.json"));

        let system = settings.persistence.borrow().config().ui.measurement_system;
        let w_clone = Rc::new((
            units::parse_length(&w.width.text(), system).unwrap_or(400.0),
            units::parse_length(&w.height.text(), system).unwrap_or(300.0),
            units::parse_length(&w.grid_spacing.text(), system).unwrap_or(10.0),
            w.feed_rate.text().to_string(),
            w.laser_power.text().to_string(),
            w.laser_mode
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "M4".to_string()),
        ));

        dialog.pick_save(None::<&gtk4::Window>, move |file| {
            if let Some(path) = file.path() {
                let json = serde_json::json!({
                    "width": w_clone.0,
                    "height": w_clone.1,
                    "grid_spacing": w_clone.2,
                    "feed_rate": w_clone.3,
                    "laser_power": w_clone.4,
                    "laser_mode": w_clone.5,
                });
                let _ = fs::write(
                    path,
                    serde_json::to_string_pretty(&json).unwrap_or_default(),
                );
            }
        });
    }

    fn load_params(w: &SpoilboardGridWidgets, settings: &Rc<SettingsController>) {
        let dialog = crate::ui::gtk::file_dialog::new_file_dialog("Load Parameters");

        let w_clone = Rc::new((
            w.width.clone(),
            w.height.clone(),
            w.grid_spacing.clone(),
            w.feed_rate.clone(),
            w.laser_power.clone(),
            w.laser_mode.clone(),
        ));
        let settings_clone = settings.clone();

        dialog.pick_open(None::<&gtk4::Window>, move |file| {
            if let Some(path) = file.path() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(params) = serde_json::from_str::<serde_json::Value>(&content) {
                        let system = settings_clone
                            .persistence
                            .borrow()
                            .config()
                            .ui
                            .measurement_system;
                        if let Some(v) = params.get("width").and_then(|v| v.as_f64()) {
                            w_clone.0.set_text(&units::format_length(v as f32, system));
                        }
                        if let Some(v) = params.get("height").and_then(|v| v.as_f64()) {
                            w_clone.1.set_text(&units::format_length(v as f32, system));
                        }
                        if let Some(v) = params.get("grid_spacing").and_then(|v| v.as_f64()) {
                            w_clone.2.set_text(&units::format_length(v as f32, system));
                        }
                        if let Some(v) = params.get("feed_rate").and_then(|v| v.as_str()) {
                            w_clone.3.set_text(v);
                        }
                        if let Some(v) = params.get("laser_power").and_then(|v| v.as_str()) {
                            w_clone.4.set_text(v);
                        }
                        if let Some(v) = params.get("laser_mode").and_then(|v| v.as_str()) {
                            w_clone.5.set_active_id(Some(v));
                        }
                    }
                }
            }
        });
    }
}
