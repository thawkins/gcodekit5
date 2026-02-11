//! Jigsaw Puzzle Generator Tool

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, Entry, FileChooserAction, FileChooserDialog, Label,
    Orientation, Paned, ResponseType, ScrolledWindow, Stack,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use super::common::{create_dimension_row, set_paned_initial_fraction};
use super::CamToolsView;
use crate::ui::gtk::help_browser;
use gcodekit5_camtools::jigsaw_puzzle::{JigsawPuzzleMaker, PuzzleParameters};
use gcodekit5_core::units;
use gcodekit5_settings::SettingsController;

struct JigsawWidgets {
    width: Entry,
    height: Entry,
    pieces_across: Entry,
    pieces_down: Entry,
    kerf: Entry,
    seed: Entry,
    tab_size: Entry,
    jitter: Entry,
    corner_radius: Entry,
    passes: Entry,
    power: Entry,
    feed_rate: Entry,
    z_step_down: Entry,
    offset_x: Entry,
    offset_y: Entry,
    home_before: CheckButton,
}

pub struct JigsawTool {
    content: Box,
}

impl JigsawTool {
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
            .label("Jigsaw Puzzle Generator")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);

        header.append(&help_browser::make_help_button("jigsaw_puzzle"));

        content_box.append(&header);

        // Paned Layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);
        content_box.append(&paned);

        // Sidebar (40%)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_label = Label::builder()
            .label("Jigsaw Puzzle Generator")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Create custom jigsaw puzzle patterns from images or blank material. Features Draradech's algorithm for unique pieces.")
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

        // Widgets
        let (width_row, width, width_unit) = create_dimension_row("Width:", 200.0, &settings);
        let (height_row, height, height_unit) = create_dimension_row("Height:", 150.0, &settings);
        let pieces_across = Entry::builder().text("4").valign(Align::Center).build();
        let pieces_down = Entry::builder().text("3").valign(Align::Center).build();
        let (kerf_row, kerf, kerf_unit) = create_dimension_row("Kerf:", 0.5, &settings);
        let seed = Entry::builder().text("42").valign(Align::Center).build();
        let tab_size = Entry::builder().text("20").valign(Align::Center).build();
        let jitter = Entry::builder().text("4").valign(Align::Center).build();
        let (corner_radius_row, corner_radius, corner_radius_unit) =
            create_dimension_row("Corner Radius:", 2.0, &settings);
        let passes = Entry::builder().text("3").valign(Align::Center).build();
        let power = Entry::builder().text("1000").valign(Align::Center).build();
        let feed_rate = Entry::builder().text("500").valign(Align::Center).build();
        let (z_step_down_row, z_step_down, z_step_down_unit) =
            create_dimension_row("Z Step Down:", 0.5, &settings);
        let (offset_x_row, offset_x, offset_x_unit) =
            create_dimension_row("Offset X:", 10.0, &settings);
        let (offset_y_row, offset_y, offset_y_unit) =
            create_dimension_row("Offset Y:", 10.0, &settings);
        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let dim_group = PreferencesGroup::builder()
            .title("Puzzle Dimensions")
            .build();
        dim_group.add(&width_row);
        dim_group.add(&height_row);
        dim_group.add(&corner_radius_row);
        scroll_content.append(&dim_group);

        let grid_group = PreferencesGroup::builder()
            .title("Grid Configuration")
            .build();
        grid_group.add(&Self::create_row("Pieces Across:", &pieces_across));
        grid_group.add(&Self::create_row("Pieces Down:", &pieces_down));
        scroll_content.append(&grid_group);

        let param_group = PreferencesGroup::builder()
            .title("Puzzle Parameters")
            .build();
        param_group.add(&kerf_row);
        param_group.add(&Self::create_row("Tab Size (%):", &tab_size));
        param_group.add(&Self::create_row("Jitter (%):", &jitter));

        let seed_row = ActionRow::builder().title("Random Seed:").build();
        let seed_box = Box::new(Orientation::Horizontal, 6);
        seed_box.append(&seed);
        let rand_btn = Button::builder()
            .icon_name("media-playlist-shuffle-symbolic")
            .build();
        seed_box.append(&rand_btn);
        seed_row.add_suffix(&seed_box);
        param_group.add(&seed_row);

        scroll_content.append(&param_group);

        let laser_group = PreferencesGroup::builder().title("Laser Settings").build();
        laser_group.add(&Self::create_row("Passes:", &passes));
        laser_group.add(&Self::create_row("Power (S):", &power));
        laser_group.add(&Self::create_row("Feed Rate:", &feed_rate));
        laser_group.add(&z_step_down_row);
        scroll_content.append(&laser_group);

        let offset_group = PreferencesGroup::builder().title("Work Offsets").build();
        offset_group.add(&offset_x_row);
        offset_group.add(&offset_y_row);

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        offset_group.add(&home_row);

        scroll_content.append(&offset_group);

        right_panel.append(&scrolled);

        // Actions
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

        let widgets = Rc::new(JigsawWidgets {
            width,
            height,
            pieces_across,
            pieces_down,
            kerf,
            seed,
            tab_size,
            jitter,
            corner_radius,
            passes,
            power,
            feed_rate,
            z_step_down,
            offset_x,
            offset_y,
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let width_unit = width_unit.clone();
            let height_unit = height_unit.clone();
            let kerf_unit = kerf_unit.clone();
            let corner_radius_unit = corner_radius_unit.clone();
            let z_step_down_unit = z_step_down_unit.clone();
            let offset_x_unit = offset_x_unit.clone();
            let offset_y_unit = offset_y_unit.clone();

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

                        update_entry(&w.width, &width_unit);
                        update_entry(&w.height, &height_unit);
                        update_entry(&w.kerf, &kerf_unit);
                        update_entry(&w.corner_radius, &corner_radius_unit);
                        update_entry(&w.z_step_down, &z_step_down_unit);
                        update_entry(&w.offset_x, &offset_x_unit);
                        update_entry(&w.offset_y, &offset_y_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Connect Generate
        let w_gen = widgets.clone();
        let on_gen = on_generate.clone();
        let settings_gen = settings.clone();
        generate_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&w_gen, &settings_gen);
            let home_before = w_gen.home_before.is_active();

            // Create progress dialog
            let progress_window = gtk4::Window::builder()
                .title("Generating Puzzle")
                .modal(true)
                .default_width(400)
                .default_height(150)
                .resizable(false)
                .build();

            let vbox = Box::new(Orientation::Vertical, 12);
            vbox.set_margin_top(24);
            vbox.set_margin_bottom(24);
            vbox.set_margin_start(24);
            vbox.set_margin_end(24);

            let status_label = Label::new(Some("Generating puzzle pieces..."));
            vbox.append(&status_label);

            let progress_bar = gtk4::ProgressBar::new();
            progress_bar.set_show_text(true);
            progress_bar.set_fraction(0.0);
            vbox.append(&progress_bar);

            let button_box = Box::new(Orientation::Horizontal, 6);
            button_box.set_halign(Align::End);
            let cancel_button = Button::with_label("Cancel");
            button_box.append(&cancel_button);
            vbox.append(&button_box);

            progress_window.set_child(Some(&vbox));
            progress_window.show();

            let on_gen_clone = on_gen.clone();
            let progress_window_clone = progress_window.clone();
            let progress_bar_clone = progress_bar.clone();

            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();

            let cancel_tx_clone = cancel_tx.clone();
            cancel_button.connect_clicked(move |_| {
                let _ = cancel_tx_clone.send(());
            });

            // Spawn background thread
            std::thread::spawn(move || {
                let result = (|| -> Result<String, String> {
                    if cancel_rx.try_recv().is_ok() {
                        return Err("Cancelled by user".to_string());
                    }
                    let mut maker = JigsawPuzzleMaker::new(params)?;
                    maker.generate()?;
                    let mut gcode = maker.to_gcode(500.0, 1.0);

                    // Handle homing
                    gcode = gcode.replace("$H\n", "").replace("$H", "");
                    if home_before {
                        gcode = format!("$H\n{}", gcode);
                    }

                    Ok(gcode)
                })();

                let _ = result_tx.send(result);
            });

            // Simulate progress since JigsawPuzzleMaker doesn't have progress callback yet
            let mut progress = 0.0;
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                // Check for result
                if let Ok(result) = result_rx.try_recv() {
                    progress_window_clone.close();

                    match result {
                        Ok(gcode) => {
                            on_gen_clone(gcode);
                        }
                        Err(e) => {
                            CamToolsView::show_error_dialog(
                                "Puzzle Generation Failed",
                                &format!("Failed to generate puzzle: {}", e),
                            );
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    // Simulate progress
                    progress += 0.05;
                    if progress > 0.95 {
                        progress = 0.95;
                    }
                    progress_bar_clone.set_fraction(progress);
                    progress_bar_clone.set_text(Some(&format!("{:.0}%", progress * 100.0)));
                    glib::ControlFlow::Continue
                }
            });
        });

        // Seed Randomizer
        let s_gen = widgets.clone();
        rand_btn.connect_clicked(move |_| {
            let now = std::time::SystemTime::now();
            let seed = now
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            let new_seed = seed % 100000;
            s_gen.seed.set_text(&new_seed.to_string());
        });

        // Save
        let w_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&w_save, &settings_save);
            Self::save_params(&params);
        });

        // Load
        let w_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            Self::load_params(&w_load, &settings_load);
        });

        // Cancel
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

    fn collect_params(w: &JigsawWidgets, settings: &Rc<SettingsController>) -> PuzzleParameters {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        PuzzleParameters {
            width: units::parse_length(&w.width.text(), system).unwrap_or(200.0),
            height: units::parse_length(&w.height.text(), system).unwrap_or(150.0),
            pieces_across: w.pieces_across.text().parse().unwrap_or(4),
            pieces_down: w.pieces_down.text().parse().unwrap_or(3),
            kerf: units::parse_length(&w.kerf.text(), system).unwrap_or(0.5),
            seed: w.seed.text().parse::<u32>().unwrap_or(42), // Handles empty or invalid
            tab_size_percent: w.tab_size.text().parse().unwrap_or(20.0),
            jitter_percent: w.jitter.text().parse().unwrap_or(4.0),
            corner_radius: units::parse_length(&w.corner_radius.text(), system).unwrap_or(2.0),
            laser_passes: w.passes.text().parse().unwrap_or(3),
            laser_power: w.power.text().parse().unwrap_or(1000),
            feed_rate: w.feed_rate.text().parse().unwrap_or(500.0),
            z_step_down: units::parse_length(&w.z_step_down.text(), system).unwrap_or(0.5),
            offset_x: units::parse_length(&w.offset_x.text(), system).unwrap_or(10.0),
            offset_y: units::parse_length(&w.offset_y.text(), system).unwrap_or(10.0),
            num_axes: crate::device_status::get_active_num_axes(),
        }
    }

    fn save_params(params: &PuzzleParameters) {
        let dialog = FileChooserDialog::new(
            Some("Save Puzzle Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        dialog.set_current_name("puzzle_params.json");

        let params_clone = params.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(json) = serde_json::to_string_pretty(&params_clone) {
                            let _ = fs::write(path, json);
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn load_params(w: &Rc<JigsawWidgets>, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Load Puzzle Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Open,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        let w_clone = w.clone();
        let settings_clone = settings.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(content) = fs::read_to_string(path) {
                            if let Ok(params) = serde_json::from_str::<PuzzleParameters>(&content) {
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

    fn apply_params(w: &JigsawWidgets, p: &PuzzleParameters, settings: &Rc<SettingsController>) {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        w.width.set_text(&units::format_length(p.width, system));
        w.height.set_text(&units::format_length(p.height, system));
        w.pieces_across.set_text(&p.pieces_across.to_string());
        w.pieces_down.set_text(&p.pieces_down.to_string());
        w.kerf.set_text(&units::format_length(p.kerf, system));
        w.seed.set_text(&p.seed.to_string());
        w.tab_size.set_text(&p.tab_size_percent.to_string());
        w.jitter.set_text(&p.jitter_percent.to_string());
        w.corner_radius
            .set_text(&units::format_length(p.corner_radius, system));
        w.passes.set_text(&p.laser_passes.to_string());
        w.power.set_text(&p.laser_power.to_string());
        w.feed_rate.set_text(&p.feed_rate.to_string());
        w.z_step_down
            .set_text(&units::format_length(p.z_step_down, system));
        w.offset_x
            .set_text(&units::format_length(p.offset_x, system));
        w.offset_y
            .set_text(&units::format_length(p.offset_y, system));
    }
}

// Bitmap Engraving Tool
