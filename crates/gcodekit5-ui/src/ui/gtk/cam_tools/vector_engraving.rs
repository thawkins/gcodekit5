//! Vector Engraving Tool (SVG/DXF to G-code)

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, Entry, FileChooserAction, FileChooserDialog, Label,
    Orientation, Overlay, Paned, ResponseType, ScrolledWindow, Stack,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use super::common::{create_dimension_row, set_paned_initial_fraction};
use super::CamToolsView;
use crate::ui::gtk::help_browser;
use gcodekit5_camtools::vector_engraver::{VectorEngraver, VectorEngravingParameters};
use gcodekit5_core::units;
use gcodekit5_settings::SettingsController;

struct VectorEngravingWidgets {
    feed_rate: Entry,
    travel_rate: Entry,
    cut_power: Entry,
    engrave_power: Entry,
    power_scale: Entry,
    multi_pass: CheckButton,
    num_passes: Entry,
    z_step_down: Entry,
    invert_power: CheckButton,
    desired_width: Entry,
    offset_x: Entry,
    offset_y: Entry,
    enable_hatch: CheckButton,
    hatch_angle: Entry,
    hatch_spacing: Entry,
    hatch_tolerance: Entry,
    cross_hatch: CheckButton,
    enable_dwell: CheckButton,
    dwell_time: Entry,
    vector_path: Entry,
    preview_image: gtk4::Picture,
    preview_spinner: gtk4::Spinner,
    info_label: Label,
    home_before: CheckButton,
}

pub struct VectorEngravingTool {
    content: Box,
}

impl VectorEngravingTool {
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
            .label("Laser Vector Engraver")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);

        header.append(&help_browser::make_help_button("laser_vector_engraver"));

        content_box.append(&header);

        // Paned Layout (20% sidebar, 80% content)
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);
        content_box.append(&paned);

        // Sidebar with Preview (40%)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_label = Label::builder()
            .label("Vector Engraving")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Convert vector graphics (SVG, DXF) to G-code for laser cutting/engraving. Supports hatching, multi-pass, and path optimization.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Preview Area
        let preview_container = Box::new(Orientation::Vertical, 6);
        preview_container.set_vexpand(true);

        // Preview image with spinner overlay
        let preview_overlay = Overlay::new();

        // Add light gray background frame
        let preview_frame = gtk4::Frame::new(None::<&str>);
        preview_frame.add_css_class("vector-preview");
        preview_frame.set_vexpand(true);
        preview_frame.set_hexpand(true);

        let preview_image = gtk4::Picture::new();
        preview_image.set_can_shrink(true);
        preview_image.set_vexpand(true);
        preview_image.set_hexpand(true);
        preview_frame.set_child(Some(&preview_image));
        preview_overlay.set_child(Some(&preview_frame));

        // Loading spinner
        let preview_spinner = gtk4::Spinner::new();
        preview_spinner.set_halign(Align::Center);
        preview_spinner.set_valign(Align::Center);
        preview_spinner.set_size_request(48, 48);
        preview_overlay.add_overlay(&preview_spinner);

        preview_container.append(&preview_overlay);

        // Info overlay label
        let info_label = Label::builder()
            .label("No file selected")
            .css_classes(vec!["caption", "dim-label"])
            .halign(Align::Start)
            .wrap(true)
            .build();
        preview_container.append(&info_label);

        sidebar.append(&preview_container);

        // Content Area (80%)
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Create Widgets
        let vector_path = Entry::builder()
            .placeholder_text("No vector file selected")
            .valign(Align::Center)
            .build();
        let feed_rate = Entry::builder().text("600").valign(Align::Center).build();
        let travel_rate = Entry::builder().text("3000").valign(Align::Center).build();
        let cut_power = Entry::builder().text("100").valign(Align::Center).build();
        let engrave_power = Entry::builder().text("50").valign(Align::Center).build();
        let power_scale = Entry::builder().text("1000").valign(Align::Center).build();
        let multi_pass = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let num_passes = Entry::builder().text("1").valign(Align::Center).build();
        let (z_increment_row, z_step_down, z_increment_unit) =
            create_dimension_row("Z Step Down:", 0.5, &settings);
        let invert_power = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let (desired_width_row, desired_width, desired_width_unit) =
            create_dimension_row("Desired Width:", 100.0, &settings);
        let (offset_x_row, offset_x, offset_x_unit) =
            create_dimension_row("Offset X:", 10.0, &settings);
        let (offset_y_row, offset_y, offset_y_unit) =
            create_dimension_row("Offset Y:", 10.0, &settings);
        let enable_hatch = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let hatch_angle = Entry::builder().text("45").valign(Align::Center).build();
        let (hatch_spacing_row, hatch_spacing, hatch_spacing_unit) =
            create_dimension_row("Hatch Spacing:", 1.0, &settings);
        let (hatch_tolerance_row, hatch_tolerance, hatch_tolerance_unit) =
            create_dimension_row("Hatch Tolerance:", 0.1, &settings);
        let cross_hatch = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let enable_dwell = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let dwell_time = Entry::builder().text("0.1").valign(Align::Center).build();
        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let file_group = PreferencesGroup::builder().title("Vector File").build();
        let file_row = ActionRow::builder().title("File Path:").build();
        let file_box = Box::new(Orientation::Horizontal, 6);
        file_box.append(&vector_path);
        let load_file_btn = Button::builder().label("Browse...").build();
        file_box.append(&load_file_btn);
        file_row.add_suffix(&file_box);
        file_group.add(&file_row);
        scroll_content.append(&file_group);

        let output_group = PreferencesGroup::builder().title("Output Settings").build();
        output_group.add(&desired_width_row);
        output_group.add(&Self::create_row("Feed Rate:", &feed_rate));
        output_group.add(&Self::create_row("Travel Rate:", &travel_rate));
        scroll_content.append(&output_group);

        let power_group = PreferencesGroup::builder().title("Laser Power").build();
        power_group.add(&Self::create_row("Cut Power (%):", &cut_power));
        power_group.add(&Self::create_row("Engrave Power (%):", &engrave_power));
        power_group.add(&Self::create_row("Power Scale (S):", &power_scale));
        let invert_row = ActionRow::builder().title("Invert Power:").build();
        invert_row.add_suffix(&invert_power);
        power_group.add(&invert_row);
        scroll_content.append(&power_group);

        let multipass_group = PreferencesGroup::builder()
            .title("Multi-Pass Settings")
            .build();
        let multi_row = ActionRow::builder().title("Multi-Pass:").build();
        multi_row.add_suffix(&multi_pass);
        multipass_group.add(&multi_row);
        multipass_group.add(&Self::create_row("Number of Passes:", &num_passes));
        multipass_group.add(&z_increment_row);
        scroll_content.append(&multipass_group);

        let hatch_group = PreferencesGroup::builder().title("Hatching").build();
        let hatch_row = ActionRow::builder().title("Enable Hatch:").build();
        hatch_row.add_suffix(&enable_hatch);
        hatch_group.add(&hatch_row);
        hatch_group.add(&Self::create_row("Hatch Angle (°):", &hatch_angle));
        hatch_group.add(&hatch_spacing_row);
        hatch_group.add(&hatch_tolerance_row);
        let cross_row = ActionRow::builder().title("Cross Hatch:").build();
        cross_row.add_suffix(&cross_hatch);
        hatch_group.add(&cross_row);
        scroll_content.append(&hatch_group);

        let dwell_group = PreferencesGroup::builder().title("Dwell Settings").build();
        let dwell_row = ActionRow::builder().title("Enable Dwell:").build();
        dwell_row.add_suffix(&enable_dwell);
        dwell_group.add(&dwell_row);
        dwell_group.add(&Self::create_row("Dwell Time (s):", &dwell_time));
        scroll_content.append(&dwell_group);

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

        let widgets = Rc::new(VectorEngravingWidgets {
            feed_rate,
            travel_rate,
            cut_power,
            engrave_power,
            power_scale,
            multi_pass,
            num_passes,
            z_step_down,
            invert_power,
            desired_width,
            offset_x,
            offset_y,
            enable_hatch,
            hatch_angle,
            hatch_spacing,
            hatch_tolerance,
            cross_hatch,
            enable_dwell,
            dwell_time,
            vector_path,
            preview_image: preview_image.clone(),
            preview_spinner: preview_spinner.clone(),
            info_label: info_label.clone(),
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let z_increment_unit = z_increment_unit.clone();
            let desired_width_unit = desired_width_unit.clone();
            let offset_x_unit = offset_x_unit.clone();
            let offset_y_unit = offset_y_unit.clone();
            let hatch_spacing_unit = hatch_spacing_unit.clone();
            let hatch_tolerance_unit = hatch_tolerance_unit.clone();

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

                        update_entry(&w.z_step_down, &z_increment_unit);
                        update_entry(&w.desired_width, &desired_width_unit);
                        update_entry(&w.offset_x, &offset_x_unit);
                        update_entry(&w.offset_y, &offset_y_unit);
                        update_entry(&w.hatch_spacing, &hatch_spacing_unit);
                        update_entry(&w.hatch_tolerance, &hatch_tolerance_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Load File Button
        let w_load_file = widgets.clone();
        load_file_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Select Vector File"),
                None::<&gtk4::Window>,
                FileChooserAction::Open,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Open", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(900, 700);

            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Vector Files"));
            filter.add_pattern("*.svg");
            filter.add_pattern("*.dxf");
            dialog.add_filter(&filter);

            let w_clone = w_load_file.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            w_clone.vector_path.set_text(&path.display().to_string());
                            Self::load_vector_preview(&w_clone, &path);
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // Connect Generate
        let w_gen = widgets.clone();
        let on_gen = on_generate.clone();
        let settings_gen = settings.clone();
        generate_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&w_gen, &settings_gen);
            let vector_path = w_gen.vector_path.text().to_string();
            let home_before = w_gen.home_before.is_active();

            if vector_path.is_empty() {
                CamToolsView::show_error_dialog(
                    "No Vector File Selected",
                    "Please select a vector file first.",
                );
                return;
            }

            // Create progress dialog
            let progress_window = gtk4::Window::builder()
                .title("Generating Vector Engraving")
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

            let status_label = Label::new(Some("Loading vector file..."));
            vbox.append(&status_label);

            let progress_bar = gtk4::ProgressBar::new();
            progress_bar.set_show_text(true);
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
            let status_label_clone = status_label.clone();

            let (progress_tx, progress_rx) = std::sync::mpsc::channel();
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();

            let cancel_tx_clone = cancel_tx.clone();
            cancel_button.connect_clicked(move |_| {
                let _ = cancel_tx_clone.send(());
            });

            // Spawn background thread
            std::thread::spawn(move || {
                let result = VectorEngraver::from_file(&vector_path, params)
                    .and_then(|engraver| {
                        engraver.generate_gcode_with_progress(|progress| {
                            if cancel_rx.try_recv().is_ok() {
                                return;
                            }
                            let _ = progress_tx.send(progress);
                        })
                    })
                    .map(|mut gcode| {
                        gcode = gcode.replace("$H\n", "").replace("$H", "");
                        if home_before {
                            format!("$H\n{}", gcode)
                        } else {
                            gcode
                        }
                    });

                let _ = result_tx.send(result);
            });

            // Poll for progress and result
            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                // Check for progress updates
                while let Ok(progress) = progress_rx.try_recv() {
                    progress_bar_clone.set_fraction(progress as f64);
                    progress_bar_clone.set_text(Some(&format!("{:.0}%", progress * 100.0)));

                    if progress < 0.1 {
                        status_label_clone.set_text("Loading vector file...");
                    } else if progress < 0.9 {
                        status_label_clone.set_text("Generating G-code...");
                    } else {
                        status_label_clone.set_text("Finalizing...");
                    }
                }

                // Check for result
                if let Ok(result) = result_rx.try_recv() {
                    progress_window_clone.close();

                    match result {
                        Ok(gcode) => {
                            on_gen_clone(gcode);
                        }
                        Err(e) => {
                            CamToolsView::show_error_dialog(
                                "Vector Engraving Generation Failed",
                                &format!("Failed to generate vector engraving: {}", e),
                            );
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        });

        // Save params
        let w_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
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
            dialog.set_current_name("vector_params.json");

            let w_clone = w_save.clone();
            let settings_clone = settings_save.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let params = Self::collect_params_for_save(&w_clone, &settings_clone);
                            if let Ok(json) = serde_json::to_string_pretty(&params) {
                                let _ = fs::write(path, json);
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // Load params
        let w_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
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

            let w_clone = w_load.clone();
            let settings_clone = settings_load.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            if let Ok(content) = fs::read_to_string(path) {
                                if let Ok(params) =
                                    serde_json::from_str::<serde_json::Value>(&content)
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

    fn collect_params(
        w: &VectorEngravingWidgets,
        settings: &Rc<SettingsController>,
    ) -> VectorEngravingParameters {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        VectorEngravingParameters {
            feed_rate: w.feed_rate.text().parse().unwrap_or(600.0),
            travel_rate: w.travel_rate.text().parse().unwrap_or(3000.0),
            cut_power: w.cut_power.text().parse().unwrap_or(100.0),
            engrave_power: w.engrave_power.text().parse().unwrap_or(50.0),
            power_scale: w.power_scale.text().parse().unwrap_or(1000.0),
            multi_pass: w.multi_pass.is_active(),
            num_passes: w.num_passes.text().parse().unwrap_or(1),
            z_step_down: units::parse_length(&w.z_step_down.text(), system).unwrap_or(0.5),
            invert_power: w.invert_power.is_active(),
            desired_width: units::parse_length(&w.desired_width.text(), system).unwrap_or(100.0),
            offset_x: units::parse_length(&w.offset_x.text(), system).unwrap_or(10.0),
            offset_y: units::parse_length(&w.offset_y.text(), system).unwrap_or(10.0),
            enable_hatch: w.enable_hatch.is_active(),
            hatch_angle: w.hatch_angle.text().parse().unwrap_or(45.0),
            hatch_spacing: units::parse_length(&w.hatch_spacing.text(), system).unwrap_or(1.0),
            hatch_tolerance: units::parse_length(&w.hatch_tolerance.text(), system).unwrap_or(0.1),
            enable_dwell: w.enable_dwell.is_active(),
            dwell_time: w.dwell_time.text().parse().unwrap_or(0.1),
            cross_hatch: w.cross_hatch.is_active(),
            num_axes: crate::device_status::get_active_num_axes(),
        }
    }

    fn collect_params_for_save(
        w: &VectorEngravingWidgets,
        settings: &Rc<SettingsController>,
    ) -> serde_json::Value {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        serde_json::json!({
            "feed_rate": w.feed_rate.text().to_string(),
            "travel_rate": w.travel_rate.text().to_string(),
            "cut_power": w.cut_power.text().to_string(),
            "engrave_power": w.engrave_power.text().to_string(),
            "power_scale": w.power_scale.text().to_string(),
            "multi_pass": w.multi_pass.is_active(),
            "num_passes": w.num_passes.text().to_string(),
            "z_step_down": units::parse_length(&w.z_step_down.text(), system).unwrap_or(0.5),
            "invert_power": w.invert_power.is_active(),
            "desired_width": units::parse_length(&w.desired_width.text(), system).unwrap_or(100.0),
            "offset_x": units::parse_length(&w.offset_x.text(), system).unwrap_or(10.0),
            "offset_y": units::parse_length(&w.offset_y.text(), system).unwrap_or(10.0),
            "enable_hatch": w.enable_hatch.is_active(),
            "hatch_angle": w.hatch_angle.text().to_string(),
            "hatch_spacing": units::parse_length(&w.hatch_spacing.text(), system).unwrap_or(1.0),
            "hatch_tolerance": units::parse_length(&w.hatch_tolerance.text(), system).unwrap_or(0.1),
            "enable_dwell": w.enable_dwell.is_active(),
            "dwell_time": w.dwell_time.text().to_string(),
            "cross_hatch": w.cross_hatch.is_active(),
            "vector_path": w.vector_path.text().to_string(),
        })
    }

    fn apply_params(
        w: &VectorEngravingWidgets,
        params: &serde_json::Value,
        settings: &Rc<SettingsController>,
    ) {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        if let Some(v) = params.get("feed_rate").and_then(|v| v.as_str()) {
            w.feed_rate.set_text(v);
        }
        if let Some(v) = params.get("travel_rate").and_then(|v| v.as_str()) {
            w.travel_rate.set_text(v);
        }
        if let Some(v) = params.get("cut_power").and_then(|v| v.as_str()) {
            w.cut_power.set_text(v);
        }
        if let Some(v) = params.get("engrave_power").and_then(|v| v.as_str()) {
            w.engrave_power.set_text(v);
        }
        if let Some(v) = params.get("power_scale").and_then(|v| v.as_str()) {
            w.power_scale.set_text(v);
        }
        if let Some(v) = params.get("multi_pass").and_then(|v| v.as_bool()) {
            w.multi_pass.set_active(v);
        }
        if let Some(v) = params.get("num_passes").and_then(|v| v.as_str()) {
            w.num_passes.set_text(v);
        }
        if let Some(v) = params.get("z_step_down").and_then(|v| v.as_f64()) {
            w.z_step_down
                .set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("invert_power").and_then(|v| v.as_bool()) {
            w.invert_power.set_active(v);
        }
        if let Some(v) = params.get("desired_width").and_then(|v| v.as_f64()) {
            w.desired_width
                .set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("offset_x").and_then(|v| v.as_f64()) {
            w.offset_x.set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("offset_y").and_then(|v| v.as_f64()) {
            w.offset_y.set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("enable_hatch").and_then(|v| v.as_bool()) {
            w.enable_hatch.set_active(v);
        }
        if let Some(v) = params.get("hatch_angle").and_then(|v| v.as_str()) {
            w.hatch_angle.set_text(v);
        }
        if let Some(v) = params.get("hatch_spacing").and_then(|v| v.as_f64()) {
            w.hatch_spacing
                .set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("hatch_tolerance").and_then(|v| v.as_f64()) {
            w.hatch_tolerance
                .set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("enable_dwell").and_then(|v| v.as_bool()) {
            w.enable_dwell.set_active(v);
        }
        if let Some(v) = params.get("dwell_time").and_then(|v| v.as_str()) {
            w.dwell_time.set_text(v);
        }
        if let Some(v) = params.get("cross_hatch").and_then(|v| v.as_bool()) {
            w.cross_hatch.set_active(v);
        }
        if let Some(v) = params.get("vector_path").and_then(|v| v.as_str()) {
            w.vector_path.set_text(v);
            if !v.is_empty() {
                Self::load_vector_preview(w, std::path::Path::new(v));
            }
        }
    }

    fn load_vector_preview(w: &VectorEngravingWidgets, path: &std::path::Path) {
        // Show spinner
        w.preview_spinner.start();
        w.preview_spinner.set_visible(true);

        let path_clone = path.to_path_buf();
        let preview_image = w.preview_image.clone();
        let spinner = w.preview_spinner.clone();
        let info_label = w.info_label.clone();

        // Use channel to communicate with main thread
        let (tx, rx) = std::sync::mpsc::channel();

        // Load in background thread
        std::thread::spawn(move || {
            let result = Self::render_vector_file(&path_clone);
            let _ = tx.send(result);
        });

        // Poll for result on main thread
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            if let Ok(result) = rx.try_recv() {
                spinner.stop();
                spinner.set_visible(false);

                match result {
                    Ok((texture, info)) => {
                        preview_image.set_paintable(Some(&texture));
                        info_label.set_text(&info);
                    }
                    Err(e) => {
                        preview_image.set_paintable(None::<&gtk4::gdk::Texture>);
                        info_label.set_text(&format!("Error: {}", e));
                    }
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    fn render_vector_file(path: &std::path::Path) -> Result<(gtk4::gdk::Texture, String), String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or("Unknown file extension")?;

        match ext.to_lowercase().as_str() {
            "svg" => Self::render_svg(path),
            "dxf" => Self::render_dxf(path),
            _ => Err(format!("Unsupported file format: {}", ext)),
        }
    }

    fn render_svg(path: &std::path::Path) -> Result<(gtk4::gdk::Texture, String), String> {
        let file = gtk4::gio::File::for_path(path);
        let texture = gtk4::gdk::Texture::from_file(&file)
            .map_err(|e| format!("Failed to load SVG: {}", e))?;

        let width = texture.intrinsic_width();
        let height = texture.intrinsic_height();

        let info = format!("SVG: {}x{} pixels", width, height);
        Ok((texture, info))
    }

    fn render_dxf(path: &std::path::Path) -> Result<(gtk4::gdk::Texture, String), String> {
        // Load DXF using the vector engraver
        let params = VectorEngravingParameters::default();
        let engraver = VectorEngraver::from_file(path, params)
            .map_err(|e| format!("Failed to load DXF: {}", e))?;

        // Render paths to a raster image
        let (width, height) = (400, 400);
        let mut img = image::RgbImage::new(width, height);

        // Fill with white background
        for pixel in img.pixels_mut() {
            *pixel = image::Rgb([255, 255, 255]);
        }

        // Calculate bounds
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut path_count = 0;

        for path in &engraver.paths {
            path_count += 1;
            for event in path.iter() {
                use lyon::path::Event;
                match event {
                    Event::Begin { at }
                    | Event::Line { to: at, .. }
                    | Event::End { last: at, .. } => {
                        min_x = min_x.min(at.x);
                        min_y = min_y.min(at.y);
                        max_x = max_x.max(at.x);
                        max_y = max_y.max(at.y);
                    }
                    Event::Quadratic { to, .. } | Event::Cubic { to, .. } => {
                        min_x = min_x.min(to.x);
                        min_y = min_y.min(to.y);
                        max_x = max_x.max(to.x);
                        max_y = max_y.max(to.y);
                    }
                }
            }
        }

        let bounds_width = max_x - min_x;
        let bounds_height = max_y - min_y;

        if bounds_width > 0.0 && bounds_height > 0.0 {
            let scale = (width as f32 / bounds_width).min(height as f32 / bounds_height) * 0.9;
            let offset_x = (width as f32 - bounds_width * scale) / 2.0;
            let offset_y = (height as f32 - bounds_height * scale) / 2.0;

            // Draw paths
            for path in &engraver.paths {
                let mut prev_x = 0;
                let mut prev_y = 0;

                for event in path.iter() {
                    use lyon::path::Event;
                    match event {
                        Event::Begin { at } => {
                            let x = ((at.x - min_x) * scale + offset_x) as i32;
                            let y = ((at.y - min_y) * scale + offset_y) as i32;
                            prev_x = x.clamp(0, width as i32 - 1);
                            prev_y = y.clamp(0, height as i32 - 1);
                        }
                        Event::Line { to, .. } => {
                            let x = ((to.x - min_x) * scale + offset_x) as i32;
                            let y = ((to.y - min_y) * scale + offset_y) as i32;
                            let x = x.clamp(0, width as i32 - 1);
                            let y = y.clamp(0, height as i32 - 1);

                            // Draw line using Bresenham
                            Self::draw_line(&mut img, prev_x, prev_y, x, y);
                            prev_x = x;
                            prev_y = y;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Convert to texture
        let buffer = glib::Bytes::from(&img.into_raw());
        let texture = gtk4::gdk::MemoryTexture::new(
            width as i32,
            height as i32,
            gtk4::gdk::MemoryFormat::R8g8b8,
            &buffer,
            width as usize * 3,
        );

        let info = format!(
            "DXF: {:.1}x{:.1} mm, {} paths",
            bounds_width, bounds_height, path_count
        );
        Ok((texture.upcast(), info))
    }

    fn draw_line(img: &mut image::RgbImage, x0: i32, y0: i32, x1: i32, y1: i32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && x < img.width() as i32 && y >= 0 && y < img.height() as i32 {
                img.put_pixel(x as u32, y as u32, image::Rgb([0, 0, 0]));
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
}
