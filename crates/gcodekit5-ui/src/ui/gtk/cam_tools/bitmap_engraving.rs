//! Bitmap Image Engraving Tool

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, FileChooserAction, FileChooserDialog,
    Label, Orientation, Overlay, Paned, ResponseType, ScrolledWindow, Stack,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use super::common::{create_dimension_row, set_paned_initial_fraction};
use super::CamToolsView;
use crate::ui::gtk::help_browser;
use gcodekit5_camtools::laser_engraver::{
    BitmapImageEngraver, EngravingParameters, HalftoneMethod, ImageTransformations, RotationAngle,
    ScanDirection,
};
use gcodekit5_core::units;
use gcodekit5_settings::SettingsController;

struct BitmapEngravingWidgets {
    width_mm: Entry,
    feed_rate: Entry,
    travel_rate: Entry,
    min_power: Entry,
    max_power: Entry,
    pixels_per_mm: Entry,
    line_spacing: Entry,
    power_scale: Entry,
    offset_x: Entry,
    offset_y: Entry,
    scan_direction: ComboBoxText,
    bidirectional: CheckButton,
    invert: CheckButton,
    mirror_x: CheckButton,
    mirror_y: CheckButton,
    rotation: ComboBoxText,
    halftone: ComboBoxText,
    halftone_dot_size: Entry,
    halftone_threshold: Entry,
    image_path: Entry,
    preview_image: gtk4::Picture,
    preview_spinner: gtk4::Spinner,
    home_before: CheckButton,
}

pub struct BitmapEngravingTool {
    content: Box,
}

impl BitmapEngravingTool {
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
            .label("Laser Image Engraver")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);

        header.append(&help_browser::make_help_button("laser_image_engraver"));

        content_box.append(&header);

        // Paned Layout
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
            .label("Bitmap Engraving")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Convert bitmap images to G-code for laser engraving. Supports various halftoning methods and image transformations.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Preview Image with spinner overlay
        let preview_overlay = Overlay::new();
        let preview_image = gtk4::Picture::new();
        preview_image.set_can_shrink(true);
        preview_image.set_vexpand(true);
        preview_image.set_hexpand(true);
        preview_overlay.set_child(Some(&preview_image));

        // Loading spinner
        let preview_spinner = gtk4::Spinner::new();
        preview_spinner.set_halign(Align::Center);
        preview_spinner.set_valign(Align::Center);
        preview_spinner.set_size_request(48, 48);
        preview_spinner.set_visible(false);
        preview_overlay.add_overlay(&preview_spinner);

        sidebar.append(&preview_overlay);

        // Content Area
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Create Widgets
        let image_path = Entry::builder()
            .placeholder_text("No image selected")
            .valign(Align::Center)
            .build();
        let (width_mm_row, width_mm, width_mm_unit) =
            create_dimension_row("Width:", 100.0, &settings);
        let feed_rate = Entry::builder().text("1000").valign(Align::Center).build();
        let travel_rate = Entry::builder().text("3000").valign(Align::Center).build();
        let min_power = Entry::builder().text("0").valign(Align::Center).build();
        let max_power = Entry::builder().text("100").valign(Align::Center).build();
        let pixels_per_mm = Entry::builder().text("10").valign(Align::Center).build();
        let line_spacing = Entry::builder().text("1.0").valign(Align::Center).build();
        let power_scale = Entry::builder().text("1000").valign(Align::Center).build();
        let (offset_x_row, offset_x, offset_x_unit) =
            create_dimension_row("Offset X:", 10.0, &settings);
        let (offset_y_row, offset_y, offset_y_unit) =
            create_dimension_row("Offset Y:", 10.0, &settings);

        let scan_direction = ComboBoxText::new();
        scan_direction.append(Some("0"), "Horizontal");
        scan_direction.append(Some("1"), "Vertical");
        scan_direction.set_active_id(Some("0"));
        scan_direction.set_valign(Align::Center);

        let bidirectional = CheckButton::builder()
            .active(true)
            .valign(Align::Center)
            .build();
        let invert = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let mirror_x = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let mirror_y = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        let rotation = ComboBoxText::new();
        rotation.append(Some("0"), "0°");
        rotation.append(Some("90"), "90°");
        rotation.append(Some("180"), "180°");
        rotation.append(Some("270"), "270°");
        rotation.set_active_id(Some("0"));
        rotation.set_valign(Align::Center);

        let halftone = ComboBoxText::new();
        halftone.append(Some("none"), "None");
        halftone.append(Some("threshold"), "Threshold");
        halftone.append(Some("bayer"), "Bayer 4x4");
        halftone.append(Some("floyd"), "Floyd-Steinberg");
        halftone.append(Some("atkinson"), "Atkinson");
        halftone.set_active_id(Some("none"));
        halftone.set_valign(Align::Center);

        let halftone_dot_size = Entry::builder().text("4").valign(Align::Center).build();
        let halftone_threshold = Entry::builder().text("127").valign(Align::Center).build();
        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let image_group = PreferencesGroup::builder().title("Image File").build();
        let image_row = ActionRow::builder().title("Image Path:").build();
        let image_box = Box::new(Orientation::Horizontal, 6);
        image_box.append(&image_path);
        let load_image_btn = Button::builder().label("Browse...").build();
        image_box.append(&load_image_btn);
        image_row.add_suffix(&image_box);
        image_group.add(&image_row);
        scroll_content.append(&image_group);

        let output_group = PreferencesGroup::builder().title("Output Settings").build();
        output_group.add(&width_mm_row);
        output_group.add(&Self::create_row("Feed Rate:", &feed_rate));
        output_group.add(&Self::create_row("Travel Rate:", &travel_rate));
        scroll_content.append(&output_group);

        let power_group = PreferencesGroup::builder().title("Laser Power").build();
        power_group.add(&Self::create_row("Min Power (%):", &min_power));
        power_group.add(&Self::create_row("Max Power (%):", &max_power));
        power_group.add(&Self::create_row("Power Scale (S):", &power_scale));
        scroll_content.append(&power_group);

        let scan_group = PreferencesGroup::builder().title("Scanning").build();
        scan_group.add(&Self::create_row("Scan Direction:", &scan_direction));
        scan_group.add(&Self::create_row("Pixels per mm:", &pixels_per_mm));
        scan_group.add(&Self::create_row("Line Spacing:", &line_spacing));
        let bid_row = ActionRow::builder().title("Bidirectional:").build();
        bid_row.add_suffix(&bidirectional);
        scan_group.add(&bid_row);
        scroll_content.append(&scan_group);

        let transform_group = PreferencesGroup::builder()
            .title("Image Transformations")
            .build();
        let invert_row = ActionRow::builder().title("Invert:").build();
        invert_row.add_suffix(&invert);
        transform_group.add(&invert_row);
        let mirror_x_row = ActionRow::builder().title("Mirror X:").build();
        mirror_x_row.add_suffix(&mirror_x);
        transform_group.add(&mirror_x_row);
        let mirror_y_row = ActionRow::builder().title("Mirror Y:").build();
        mirror_y_row.add_suffix(&mirror_y);
        transform_group.add(&mirror_y_row);
        transform_group.add(&Self::create_row("Rotation:", &rotation));
        scroll_content.append(&transform_group);

        let halftone_group = PreferencesGroup::builder().title("Halftoning").build();
        halftone_group.add(&Self::create_row("Method:", &halftone));
        halftone_group.add(&Self::create_row("Dot Size:", &halftone_dot_size));
        halftone_group.add(&Self::create_row("Threshold:", &halftone_threshold));
        scroll_content.append(&halftone_group);

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

        let widgets = Rc::new(BitmapEngravingWidgets {
            width_mm,
            feed_rate,
            travel_rate,
            min_power,
            max_power,
            pixels_per_mm,
            line_spacing,
            power_scale,
            offset_x,
            offset_y,
            scan_direction,
            bidirectional,
            invert,
            mirror_x,
            mirror_y,
            rotation,
            halftone,
            halftone_dot_size,
            halftone_threshold,
            image_path,
            preview_image: preview_image.clone(),
            preview_spinner: preview_spinner.clone(),
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let width_mm_unit = width_mm_unit.clone();
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

                        update_entry(&w.width_mm, &width_mm_unit);
                        update_entry(&w.offset_x, &offset_x_unit);
                        update_entry(&w.offset_y, &offset_y_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Load Image Button
        let w_load_image = widgets.clone();
        load_image_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Select Image"),
                None::<&gtk4::Window>,
                FileChooserAction::Open,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Open", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(900, 700);

            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Image Files"));
            filter.add_mime_type("image/png");
            filter.add_mime_type("image/jpeg");
            filter.add_mime_type("image/bmp");
            filter.add_mime_type("image/gif");
            filter.add_mime_type("image/tiff");
            dialog.add_filter(&filter);

            let w_clone = w_load_image.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            w_clone.image_path.set_text(&path.display().to_string());

                            // Show spinner and load preview in background
                            w_clone.preview_spinner.set_visible(true);
                            w_clone.preview_spinner.start();

                            let preview_img = w_clone.preview_image.clone();
                            let spinner = w_clone.preview_spinner.clone();
                            let path_clone = path.clone();

                            let (tx, rx) = std::sync::mpsc::channel();

                            std::thread::spawn(move || {
                                let file = gtk4::gio::File::for_path(&path_clone);
                                let texture_result = gtk4::gdk::Texture::from_file(&file);
                                let _ = tx.send(texture_result);
                            });

                            glib::timeout_add_local(
                                std::time::Duration::from_millis(50),
                                move || {
                                    if let Ok(texture_result) = rx.try_recv() {
                                        spinner.stop();
                                        spinner.set_visible(false);

                                        if let Ok(texture) = texture_result {
                                            preview_img.set_paintable(Some(&texture));
                                        }
                                        glib::ControlFlow::Break
                                    } else {
                                        glib::ControlFlow::Continue
                                    }
                                },
                            );
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
        generate_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&w_gen);
            let image_path = w_gen.image_path.text().to_string();
            let home_before = w_gen.home_before.is_active();

            if image_path.is_empty() {
                CamToolsView::show_error_dialog(
                    "No Image Selected",
                    "Please select an image file first.",
                );
                return;
            }

            // Create progress dialog with progress bar and cancel button
            let progress_window = gtk4::Window::builder()
                .title("Generating Engraving")
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

            let status_label = Label::new(Some("Processing image..."));
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

            // Clone what we need
            let image_path_thread = image_path.clone();
            let on_gen_clone = on_gen.clone();
            let progress_window_clone = progress_window.clone();
            let progress_bar_clone = progress_bar.clone();
            let status_label_clone = status_label.clone();

            // Create channels for progress and result
            let (progress_tx, progress_rx) = std::sync::mpsc::channel();
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();

            // Cancel button handler
            let cancel_tx_clone = cancel_tx.clone();
            cancel_button.connect_clicked(move |_| {
                let _ = cancel_tx_clone.send(());
            });

            // Spawn background thread for generation
            std::thread::spawn(move || {
                let result = BitmapImageEngraver::from_file(&image_path_thread, params)
                    .and_then(|engraver| {
                        engraver.generate_gcode_with_progress(|progress| {
                            // Check for cancellation
                            if cancel_rx.try_recv().is_ok() {
                                return; // Abort
                            }
                            // Send progress update
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

                // Send result back
                let _ = result_tx.send(result);
            });

            // Poll for progress and result on main thread
            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                // Check for progress updates
                while let Ok(progress) = progress_rx.try_recv() {
                    progress_bar_clone.set_fraction(progress as f64);
                    progress_bar_clone.set_text(Some(&format!("{:.0}%", progress * 100.0)));

                    // Update status label based on progress
                    if progress < 0.1 {
                        status_label_clone.set_text("Loading image...");
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
                                "Engraving Generation Failed",
                                &format!("Failed to generate engraving: {}", e),
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
            dialog.set_current_name("bitmap_params.json");

            let w_clone = w_save.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let params = Self::collect_params_for_save(&w_clone);
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
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            if let Ok(content) = fs::read_to_string(path) {
                                if let Ok(params) =
                                    serde_json::from_str::<serde_json::Value>(&content)
                                {
                                    Self::apply_params(&w_clone, &params);
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

    fn collect_params(w: &BitmapEngravingWidgets) -> EngravingParameters {
        let rotation = match w.rotation.active_id().as_ref().map(|s| s.as_str()) {
            Some("90") => RotationAngle::Degrees90,
            Some("180") => RotationAngle::Degrees180,
            Some("270") => RotationAngle::Degrees270,
            _ => RotationAngle::Degrees0,
        };

        let halftone = match w.halftone.active_id().as_ref().map(|s| s.as_str()) {
            Some("threshold") => HalftoneMethod::Threshold,
            Some("bayer") => HalftoneMethod::Bayer4x4,
            Some("floyd") => HalftoneMethod::FloydSteinberg,
            Some("atkinson") => HalftoneMethod::Atkinson,
            _ => HalftoneMethod::None,
        };

        let scan_direction =
            if w.scan_direction.active_id().as_ref().map(|s| s.as_str()) == Some("1") {
                ScanDirection::Vertical
            } else {
                ScanDirection::Horizontal
            };

        EngravingParameters {
            width_mm: w.width_mm.text().parse().unwrap_or(100.0),
            height_mm: None,
            feed_rate: w.feed_rate.text().parse().unwrap_or(1000.0),
            travel_rate: w.travel_rate.text().parse().unwrap_or(3000.0),
            min_power: w.min_power.text().parse().unwrap_or(0.0),
            max_power: w.max_power.text().parse().unwrap_or(100.0),
            pixels_per_mm: w.pixels_per_mm.text().parse().unwrap_or(10.0),
            scan_direction,
            bidirectional: w.bidirectional.is_active(),
            line_spacing: w.line_spacing.text().parse().unwrap_or(1.0),
            power_scale: w.power_scale.text().parse().unwrap_or(1000.0),
            transformations: ImageTransformations {
                mirror_x: w.mirror_x.is_active(),
                mirror_y: w.mirror_y.is_active(),
                rotation,
                halftone,
                halftone_dot_size: w.halftone_dot_size.text().parse().unwrap_or(4),
                halftone_threshold: w.halftone_threshold.text().parse().unwrap_or(127),
                invert: w.invert.is_active(),
            },
            offset_x: w.offset_x.text().parse().unwrap_or(10.0),
            offset_y: w.offset_y.text().parse().unwrap_or(10.0),
            num_axes: crate::device_status::get_active_num_axes(),
        }
    }

    fn collect_params_for_save(w: &BitmapEngravingWidgets) -> serde_json::Value {
        serde_json::json!({
            "image_path": w.image_path.text().to_string(),
            "width_mm": w.width_mm.text().to_string(),
            "feed_rate": w.feed_rate.text().to_string(),
            "travel_rate": w.travel_rate.text().to_string(),
            "min_power": w.min_power.text().to_string(),
            "max_power": w.max_power.text().to_string(),
            "pixels_per_mm": w.pixels_per_mm.text().to_string(),
            "line_spacing": w.line_spacing.text().to_string(),
            "power_scale": w.power_scale.text().to_string(),
            "offset_x": w.offset_x.text().to_string(),
            "offset_y": w.offset_y.text().to_string(),
            "scan_direction": w.scan_direction.active_id().unwrap_or_default().to_string(),
            "bidirectional": w.bidirectional.is_active(),
            "invert": w.invert.is_active(),
            "mirror_x": w.mirror_x.is_active(),
            "mirror_y": w.mirror_y.is_active(),
            "rotation": w.rotation.active_id().unwrap_or_default().to_string(),
            "halftone": w.halftone.active_id().unwrap_or_default().to_string(),
            "halftone_dot_size": w.halftone_dot_size.text().to_string(),
            "halftone_threshold": w.halftone_threshold.text().to_string(),
        })
    }

    fn apply_params(w: &BitmapEngravingWidgets, params: &serde_json::Value) {
        if let Some(image_path) = params.get("image_path").and_then(|v| v.as_str()) {
            w.image_path.set_text(image_path);

            // Show spinner and load preview in background
            w.preview_spinner.set_visible(true);
            w.preview_spinner.start();

            let preview_img = w.preview_image.clone();
            let spinner = w.preview_spinner.clone();
            let path = image_path.to_string();

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let file = gtk4::gio::File::for_path(&path);
                let texture_result = gtk4::gdk::Texture::from_file(&file);
                let _ = tx.send(texture_result);
            });

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                if let Ok(texture_result) = rx.try_recv() {
                    spinner.stop();
                    spinner.set_visible(false);

                    if let Ok(texture) = texture_result {
                        preview_img.set_paintable(Some(&texture));
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        }
        if let Some(v) = params.get("width_mm").and_then(|v| v.as_str()) {
            w.width_mm.set_text(v);
        }
        if let Some(v) = params.get("feed_rate").and_then(|v| v.as_str()) {
            w.feed_rate.set_text(v);
        }
        if let Some(v) = params.get("travel_rate").and_then(|v| v.as_str()) {
            w.travel_rate.set_text(v);
        }
        if let Some(v) = params.get("min_power").and_then(|v| v.as_str()) {
            w.min_power.set_text(v);
        }
        if let Some(v) = params.get("max_power").and_then(|v| v.as_str()) {
            w.max_power.set_text(v);
        }
        if let Some(v) = params.get("pixels_per_mm").and_then(|v| v.as_str()) {
            w.pixels_per_mm.set_text(v);
        }
        if let Some(v) = params.get("line_spacing").and_then(|v| v.as_str()) {
            w.line_spacing.set_text(v);
        }
        if let Some(v) = params.get("power_scale").and_then(|v| v.as_str()) {
            w.power_scale.set_text(v);
        }
        if let Some(v) = params.get("offset_x").and_then(|v| v.as_str()) {
            w.offset_x.set_text(v);
        }
        if let Some(v) = params.get("offset_y").and_then(|v| v.as_str()) {
            w.offset_y.set_text(v);
        }
        if let Some(v) = params.get("scan_direction").and_then(|v| v.as_str()) {
            w.scan_direction.set_active_id(Some(v));
        }
        if let Some(v) = params.get("bidirectional").and_then(|v| v.as_bool()) {
            w.bidirectional.set_active(v);
        }
        if let Some(v) = params.get("invert").and_then(|v| v.as_bool()) {
            w.invert.set_active(v);
        }
        if let Some(v) = params.get("mirror_x").and_then(|v| v.as_bool()) {
            w.mirror_x.set_active(v);
        }
        if let Some(v) = params.get("mirror_y").and_then(|v| v.as_bool()) {
            w.mirror_y.set_active(v);
        }
        if let Some(v) = params.get("rotation").and_then(|v| v.as_str()) {
            w.rotation.set_active_id(Some(v));
        }
        if let Some(v) = params.get("halftone").and_then(|v| v.as_str()) {
            w.halftone.set_active_id(Some(v));
        }
        if let Some(v) = params.get("halftone_dot_size").and_then(|v| v.as_str()) {
            w.halftone_dot_size.set_text(v);
        }
        if let Some(v) = params.get("halftone_threshold").and_then(|v| v.as_str()) {
            w.halftone_threshold.set_text(v);
        }
    }
}

// Vector Engraving Tool
