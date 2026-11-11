//! Bitmap Image Engraving Tool

use crate::ui::gtk::common::text_dropdown::{TextDropDownExt, new_text_dropdown};
use crate::ui::gtk::file_dialog::FileDialogHelpers;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, DropDown, Entry, Label, Orientation, Overlay, Paned,
    ScrolledWindow, Stack,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use super::CamToolsView;
use super::common::{
    create_dimension_row, set_paned_initial_fraction, show_load_warnings, write_preset_file,
};
use crate::t;
use crate::ui::gtk::help_browser;
use gcodekit5_camtools::cam_presets::BitmapEngravingPreset;
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
    scan_direction: DropDown,
    bidirectional: CheckButton,
    invert: CheckButton,
    mirror_x: CheckButton,
    mirror_y: CheckButton,
    rotation: DropDown,
    halftone: DropDown,
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
            .label(t!("Laser Image Engraver"))
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
            .label(t!("Convert bitmap images to G-code for laser engraving. Supports various halftoning methods and image transformations."))
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
            .placeholder_text(t!("No image selected"))
            .valign(Align::Center)
            .build();
        let (width_mm_row, width_mm, width_mm_unit) =
            create_dimension_row(&t!("Width:"), 100.0, &settings);
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

        let scan_direction = new_text_dropdown();
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

        let rotation = new_text_dropdown();
        rotation.append(Some("0"), "0°");
        rotation.append(Some("90"), "90°");
        rotation.append(Some("180"), "180°");
        rotation.append(Some("270"), "270°");
        rotation.set_active_id(Some("0"));
        rotation.set_valign(Align::Center);

        let halftone = new_text_dropdown();
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
        let image_group = PreferencesGroup::builder().title(t!("Image File")).build();
        let image_row = ActionRow::builder().title(t!("Image Path:")).build();
        let image_box = Box::new(Orientation::Horizontal, 6);
        image_box.append(&image_path);
        let load_image_btn = Button::builder().label(t!("Browse...")).build();
        image_box.append(&load_image_btn);
        image_row.add_suffix(&image_box);
        image_group.add(&image_row);
        scroll_content.append(&image_group);

        let output_group = PreferencesGroup::builder()
            .title(t!("Output Settings"))
            .build();
        output_group.add(&width_mm_row);
        output_group.add(&Self::create_row(&t!("Feed Rate:"), &feed_rate));
        output_group.add(&Self::create_row(&t!("Travel Rate:"), &travel_rate));
        scroll_content.append(&output_group);

        let power_group = PreferencesGroup::builder().title(t!("Laser Power")).build();
        power_group.add(&Self::create_row(&t!("Min Power (%):"), &min_power));
        power_group.add(&Self::create_row(&t!("Max Power (%):"), &max_power));
        power_group.add(&Self::create_row(&t!("Power Scale (S):"), &power_scale));
        scroll_content.append(&power_group);

        let scan_group = PreferencesGroup::builder().title(t!("Scanning")).build();
        scan_group.add(&Self::create_row(&t!("Scan Direction:"), &scan_direction));
        scan_group.add(&Self::create_row(&t!("Pixels per mm:"), &pixels_per_mm));
        scan_group.add(&Self::create_row(&t!("Line Spacing:"), &line_spacing));
        let bid_row = ActionRow::builder().title(t!("Bidirectional:")).build();
        bid_row.add_suffix(&bidirectional);
        scan_group.add(&bid_row);
        scroll_content.append(&scan_group);

        let transform_group = PreferencesGroup::builder()
            .title(t!("Image Transformations"))
            .build();
        invert.set_tooltip_text(Some(&t!(
            "Off (default): dark areas get more laser power, so a photo engraves as a positive. \
             On: engrave the light areas instead (negative)."
        )));
        let invert_row = ActionRow::builder()
            .title(t!("Invert:"))
            .subtitle(t!("Engrave light areas instead of dark ones"))
            .build();
        invert_row.add_suffix(&invert);
        transform_group.add(&invert_row);
        let mirror_x_row = ActionRow::builder().title(t!("Mirror X:")).build();
        mirror_x_row.add_suffix(&mirror_x);
        transform_group.add(&mirror_x_row);
        let mirror_y_row = ActionRow::builder().title(t!("Mirror Y:")).build();
        mirror_y_row.add_suffix(&mirror_y);
        transform_group.add(&mirror_y_row);
        transform_group.add(&Self::create_row(&t!("Rotation:"), &rotation));
        scroll_content.append(&transform_group);

        let halftone_group = PreferencesGroup::builder().title(t!("Halftoning")).build();
        halftone_group.add(&Self::create_row(&t!("Method:"), &halftone));
        halftone_group.add(&Self::create_row(&t!("Dot Size:"), &halftone_dot_size));
        halftone_group.add(&Self::create_row(&t!("Threshold:"), &halftone_threshold));
        scroll_content.append(&halftone_group);

        let offset_group = PreferencesGroup::builder()
            .title(t!("Work Offsets"))
            .build();
        offset_group.add(&offset_x_row);
        offset_group.add(&offset_y_row);

        let home_row = ActionRow::builder()
            .title(t!("Home Device Before Start"))
            .subtitle(t!(
                "Runs $H, then engraves from the G54 work zero you set. The engraver never moves Z."
            ))
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

        let load_btn = Button::with_label(&t!("Load"));
        let save_btn = Button::with_label(&t!("Save"));
        let cancel_btn = Button::with_label(&t!("Cancel"));
        let generate_btn = Button::with_label(&t!("Generate"));
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
            let w = widgets.clone();
            let width_mm_unit = width_mm_unit.clone();
            let offset_x_unit = offset_x_unit.clone();
            let offset_y_unit = offset_y_unit.clone();

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
            let dialog = crate::ui::gtk::file_dialog::new_file_dialog(&t!("Select Image"));

            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Image Files"));
            filter.add_mime_type("image/png");
            filter.add_mime_type("image/jpeg");
            filter.add_mime_type("image/bmp");
            filter.add_mime_type("image/gif");
            filter.add_mime_type("image/tiff");
            dialog.add_filter(&filter);

            let w_clone = w_load_image.clone();
            dialog.pick_open(None::<&gtk4::Window>, move |file| {
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

                    glib::timeout_add_local(std::time::Duration::from_millis(50), move || match rx
                        .try_recv()
                    {
                        Ok(texture_result) => {
                            spinner.stop();
                            spinner.set_visible(false);

                            if let Ok(texture) = texture_result {
                                preview_img.set_paintable(Some(&texture));
                            }
                            glib::ControlFlow::Break
                        }
                        _ => glib::ControlFlow::Continue,
                    });
                }
            });
        });

        // Connect Generate
        let w_gen = widgets.clone();
        let on_gen = on_generate.clone();
        let settings_gen = settings.clone();
        generate_btn.connect_clicked(move |_| {
            let system = settings_gen
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system;
            let params = Self::collect_params(&w_gen, system);
            let image_path = w_gen.image_path.text().to_string();

            if image_path.is_empty() {
                CamToolsView::show_error_dialog(
                    &t!("No Image Selected"),
                    &t!("Please select an image file first."),
                );
                return;
            }

            // Create progress dialog with progress bar and cancel button
            let progress_window = gtk4::Window::builder()
                .title(t!("Generating Engraving"))
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

            let status_label = Label::new(Some(&t!("Processing image...")));
            vbox.append(&status_label);

            let progress_bar = gtk4::ProgressBar::new();
            progress_bar.set_show_text(true);
            vbox.append(&progress_bar);

            let button_box = Box::new(Orientation::Horizontal, 6);
            button_box.set_halign(Align::End);
            let cancel_button = Button::with_label(&t!("Cancel"));
            button_box.append(&cancel_button);
            vbox.append(&button_box);

            progress_window.set_child(Some(&vbox));
            progress_window.present();

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
            // Cancelling (button or window close) discards the result; the worker
            // thread cannot be interrupted mid-generation, so its output is ignored.
            let cancelled = Rc::new(Cell::new(false));
            {
                let cancelled = cancelled.clone();
                progress_window.connect_close_request(move |_| {
                    cancelled.set(true);
                    glib::Propagation::Proceed
                });
            }
            {
                let progress_window = progress_window.clone();
                cancel_button.connect_clicked(move |_| {
                    let _ = cancel_tx_clone.send(());
                    progress_window.close();
                });
            }

            // Spawn background thread for generation
            std::thread::spawn(move || {
                let result = BitmapImageEngraver::from_file(&image_path_thread, params).and_then(
                    |engraver| {
                        engraver.generate_gcode_with_progress(|progress| {
                            if cancel_rx.try_recv().is_ok() {
                                return; // Abort
                            }
                            let _ = progress_tx.send(progress);
                        })
                    },
                );

                // Send result back
                let _ = result_tx.send(result);
            });

            // Poll for progress and result on main thread
            let cancelled_poll = cancelled.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                if cancelled_poll.get() {
                    return glib::ControlFlow::Break;
                }
                // Check for progress updates
                while let Ok(progress) = progress_rx.try_recv() {
                    progress_bar_clone.set_fraction(progress as f64);
                    progress_bar_clone.set_text(Some(&format!("{:.0}%", progress * 100.0)));

                    // Update status label based on progress
                    if progress < 0.1 {
                        status_label_clone.set_text(&t!("Loading image..."));
                    } else if progress < 0.9 {
                        status_label_clone.set_text(&t!("Generating G-code..."));
                    } else {
                        status_label_clone.set_text(&t!("Finalizing..."));
                    }
                }

                // Check for result
                match result_rx.try_recv() {
                    Ok(result) => {
                        progress_window_clone.close();

                        match result {
                            Ok(gcode) => {
                                on_gen_clone(gcode);
                            }
                            Err(e) => {
                                CamToolsView::show_error_dialog(
                                    &t!("Engraving Generation Failed"),
                                    &format!("{}: {}", t!("Failed to generate engraving"), e),
                                );
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    _ => glib::ControlFlow::Continue,
                }
            });
        });

        // Save params
        let w_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            let system = settings_save
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system;
            let json = match Self::collect_preset(&w_save, system)
                .and_then(|p| p.to_json().map_err(|e| e.to_string()))
            {
                Ok(json) => json,
                Err(e) => {
                    CamToolsView::show_error_dialog(&t!("Save Error"), &e);
                    return;
                }
            };
            let dialog = crate::ui::gtk::file_dialog::new_file_dialog(&t!("Save Parameters"));
            dialog.set_initial_name(Some("bitmap_params.json"));

            dialog.pick_save(None::<&gtk4::Window>, move |file| {
                if let Some(path) = file.path() {
                    write_preset_file(&path, &json);
                }
            });
        });

        // Load params
        let w_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            let dialog = crate::ui::gtk::file_dialog::new_file_dialog(&t!("Load Parameters"));

            let w_clone = w_load.clone();
            let settings_clone = settings_load.clone();
            dialog.pick_open(None::<&gtk4::Window>, move |file| {
                if let Some(path) = file.path() {
                    // Old files stored display text without a unit; they are read in the
                    // current measurement system (as before) and flagged in a warning.
                    let system = settings_clone
                        .persistence
                        .borrow()
                        .config()
                        .ui
                        .measurement_system;
                    let loaded = fs::read_to_string(&path)
                        .map_err(anyhow::Error::from)
                        .and_then(|json| BitmapEngravingPreset::from_json(&json, system));
                    match loaded {
                        Ok(loaded) => {
                            Self::apply_preset(&w_clone, &loaded.preset, system);
                            show_load_warnings(&loaded.warnings);
                        }
                        Err(e) => {
                            tracing::warn!(
                                path = %path.display(),
                                error = %e,
                                "Failed to load bitmap preset"
                            );
                            CamToolsView::show_error_dialog(
                                &t!("Load Error"),
                                &format!("{}: {}", path.display(), e),
                            );
                        }
                    }
                }
            });
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
        w: &BitmapEngravingWidgets,
        system: units::MeasurementSystem,
    ) -> EngravingParameters {
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
            // Dimension entries are shown in the current measurement system.
            width_mm: units::parse_length(&w.width_mm.text(), system).unwrap_or(100.0),
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
            offset_x: units::parse_length(&w.offset_x.text(), system).unwrap_or(10.0),
            offset_y: units::parse_length(&w.offset_y.text(), system).unwrap_or(10.0),
            num_axes: crate::device_status::get_active_num_axes(),
            home_before_job: w.home_before.is_active(),
        }
    }

    /// Read the form into a preset with all lengths converted to mm.
    ///
    /// # Returns
    /// The preset, or the list of fields that could not be parsed.
    fn collect_preset(
        w: &BitmapEngravingWidgets,
        system: units::MeasurementSystem,
    ) -> Result<BitmapEngravingPreset, String> {
        let mut invalid: Vec<&str> = Vec::new();
        let mut len = |e: &Entry, name: &'static str| match units::parse_length(&e.text(), system) {
            Ok(v) if v.is_finite() => v,
            _ => {
                invalid.push(name);
                0.0
            }
        };
        let width_mm = len(&w.width_mm, "Width");
        let offset_x = len(&w.offset_x, "Offset X");
        let offset_y = len(&w.offset_y, "Offset Y");
        fn num<T: std::str::FromStr + Default>(
            e: &Entry,
            name: &'static str,
            invalid: &mut Vec<&'static str>,
        ) -> T {
            e.text().trim().parse::<T>().unwrap_or_else(|_| {
                invalid.push(name);
                T::default()
            })
        }
        let preset = BitmapEngravingPreset {
            image_path: w.image_path.text().to_string(),
            width_mm,
            feed_rate: num(&w.feed_rate, "Feed Rate", &mut invalid),
            travel_rate: num(&w.travel_rate, "Travel Rate", &mut invalid),
            min_power: num(&w.min_power, "Min Power", &mut invalid),
            max_power: num(&w.max_power, "Max Power", &mut invalid),
            pixels_per_mm: num(&w.pixels_per_mm, "Pixels per mm", &mut invalid),
            line_spacing: num(&w.line_spacing, "Line Spacing", &mut invalid),
            power_scale: num(&w.power_scale, "Power Scale", &mut invalid),
            offset_x,
            offset_y,
            scan_direction: w.scan_direction.active_id().unwrap_or_default().to_string(),
            bidirectional: w.bidirectional.is_active(),
            invert: w.invert.is_active(),
            mirror_x: w.mirror_x.is_active(),
            mirror_y: w.mirror_y.is_active(),
            rotation: w.rotation.active_id().unwrap_or_default().to_string(),
            halftone: w.halftone.active_id().unwrap_or_default().to_string(),
            halftone_dot_size: num(&w.halftone_dot_size, "Dot Size", &mut invalid),
            halftone_threshold: num(&w.halftone_threshold, "Threshold", &mut invalid),
            home_before_job: w.home_before.is_active(),
            ..BitmapEngravingPreset::default()
        };
        if invalid.is_empty() {
            Ok(preset)
        } else {
            Err(format!("Please correct: {}", invalid.join(", ")))
        }
    }

    /// Fill the form from a preset (lengths in mm, shown in `system`).
    fn apply_preset(
        w: &BitmapEngravingWidgets,
        p: &BitmapEngravingPreset,
        system: units::MeasurementSystem,
    ) {
        if !p.image_path.is_empty() {
            Self::load_image_preview(w, &p.image_path);
        }
        w.width_mm
            .set_text(&units::format_length(p.width_mm, system));
        w.feed_rate.set_text(&p.feed_rate.to_string());
        w.travel_rate.set_text(&p.travel_rate.to_string());
        w.min_power.set_text(&p.min_power.to_string());
        w.max_power.set_text(&p.max_power.to_string());
        w.pixels_per_mm.set_text(&p.pixels_per_mm.to_string());
        w.line_spacing.set_text(&p.line_spacing.to_string());
        w.power_scale.set_text(&p.power_scale.to_string());
        w.offset_x
            .set_text(&units::format_length(p.offset_x, system));
        w.offset_y
            .set_text(&units::format_length(p.offset_y, system));
        w.scan_direction.set_active_id(Some(&p.scan_direction));
        w.bidirectional.set_active(p.bidirectional);
        w.invert.set_active(p.invert);
        w.mirror_x.set_active(p.mirror_x);
        w.mirror_y.set_active(p.mirror_y);
        w.rotation.set_active_id(Some(&p.rotation));
        w.halftone.set_active_id(Some(&p.halftone));
        w.halftone_dot_size
            .set_text(&p.halftone_dot_size.to_string());
        w.halftone_threshold
            .set_text(&p.halftone_threshold.to_string());
        w.home_before.set_active(p.home_before_job);
    }

    /// Show `path` in the preview, loading it on a worker thread.
    fn load_image_preview(w: &BitmapEngravingWidgets, image_path: &str) {
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
            match rx.try_recv() {
                Ok(texture_result) => {
                    spinner.stop();
                    spinner.set_visible(false);

                    if let Ok(texture) = texture_result {
                        preview_img.set_paintable(Some(&texture));
                    }
                    glib::ControlFlow::Break
                }
                _ => glib::ControlFlow::Continue,
            }
        });
    }
}
