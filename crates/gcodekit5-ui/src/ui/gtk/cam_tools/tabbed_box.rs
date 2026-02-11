//! Tabbed Box Maker Tool

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, FileChooserAction, FileChooserDialog,
    Label, Orientation, Paned, ResponseType, ScrolledWindow, Stack,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use super::common::{create_dimension_row, set_paned_initial_fraction};
use super::CamToolsView;
use crate::ui::gtk::help_browser;
use gcodekit5_camtools::tabbed_box::{
    BoxParameters, BoxType, KeyDividerType, TabbedBoxMaker as Generator,
};
use gcodekit5_core::units;
use gcodekit5_designer::{PathShape, Point, Shape};
use gcodekit5_settings::SettingsController;

struct TabbedBoxWidgets {
    width: Entry,
    depth: Entry,
    height: Entry,
    outside: CheckButton,
    thickness: Entry,
    burn: Entry,
    finger_width: Entry,
    space_width: Entry,
    surrounding_spaces: Entry,
    play: Entry,
    extra_length: Entry,
    // New controls
    box_type: ComboBoxText,
    dividers_x: Entry,
    dividers_y: Entry,
    divider_keying: ComboBoxText,
    optimize_layout: CheckButton,
    passes: Entry,
    power: Entry,
    feed_rate: Entry,
    z_step_down: Entry,
    offset_x: Entry,
    offset_y: Entry,
    home_before: CheckButton,
}

pub struct TabbedBoxMaker {
    pub content: Box,
}

impl TabbedBoxMaker {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        on_generate: Rc<F>,
        designer_view: Option<Rc<crate::ui::gtk::designer::DesignerView>>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header with Back Button
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
        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);
        header.append(&help_browser::make_help_button("tabbed_box_maker"));
        content_box.append(&header);

        // Split View
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar (40% width)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_lbl = Label::builder()
            .label("Tabbed Box Maker")
            .css_classes(vec!["title-3"])
            .wrap(true)
            .halign(Align::Start)
            .build();

        let desc_lbl = Label::builder()
            .label("Generate G-code for laser/CNC cut boxes with finger joints based on the boxes.py algorithm.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();

        sidebar.append(&title_lbl);
        sidebar.append(&desc_lbl);

        // Right Panel
        let right_panel = Box::new(Orientation::Vertical, 0);

        // Scrollable Content
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Widgets
        let (width_row, width, width_unit) = create_dimension_row("X (Width):", 100.0, &settings);
        let (depth_row, depth, depth_unit) = create_dimension_row("Y (Depth):", 100.0, &settings);
        let (height_row, height, height_unit) =
            create_dimension_row("H (Height):", 100.0, &settings);
        let outside = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let (thickness_row, thickness, thickness_unit) =
            create_dimension_row("Thickness:", 3.0, &settings);
        let (burn_row, burn, burn_unit) = create_dimension_row("Burn / Tool Dia:", 0.1, &settings);
        let finger_width = Entry::builder().text("2").valign(Align::Center).build();
        let space_width = Entry::builder().text("2").valign(Align::Center).build();
        let surrounding_spaces = Entry::builder().text("2").valign(Align::Center).build();
        let (play_row, play, play_unit) =
            create_dimension_row("Play (fit tolerance):", 0.0, &settings);
        let (extra_length_row, extra_length, extra_length_unit) =
            create_dimension_row("Extra Length:", 0.0, &settings);

        // New Widgets
        let box_type = ComboBoxText::new();
        box_type.append(Some("0"), "Full Box");
        box_type.append(Some("1"), "No Top");
        box_type.append(Some("2"), "No Bottom");
        box_type.append(Some("3"), "No Sides");
        box_type.append(Some("4"), "No Front/Back");
        box_type.append(Some("5"), "No Left/Right");
        box_type.set_active_id(Some("0"));
        box_type.set_valign(Align::Center);

        let dividers_x = Entry::builder().text("0").valign(Align::Center).build();
        let dividers_y = Entry::builder().text("0").valign(Align::Center).build();

        let divider_keying = ComboBoxText::new();
        divider_keying.append(Some("0"), "Walls and Floor");
        divider_keying.append(Some("1"), "Walls Only");
        divider_keying.append(Some("2"), "Floor Only");
        divider_keying.append(Some("3"), "None");
        divider_keying.set_active_id(Some("0"));
        divider_keying.set_valign(Align::Center);

        let optimize_layout = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        let passes = Entry::builder().text("3").valign(Align::Center).build();
        let power = Entry::builder().text("1000").valign(Align::Center).build();
        let feed_rate = Entry::builder().text("500").valign(Align::Center).build();

        let (z_step_down_row, z_step_down, z_step_down_unit) =
            create_dimension_row("Z Step Down:", 0.1, &settings);

        let (offset_x_row, offset_x, offset_x_unit) =
            create_dimension_row("Offset X:", 10.0, &settings);
        let (offset_y_row, offset_y, offset_y_unit) =
            create_dimension_row("Offset Y:", 10.0, &settings);
        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Box Dimensions
        let dim_group = PreferencesGroup::builder().title("Box Dimensions").build();
        dim_group.add(&width_row);
        dim_group.add(&depth_row);
        dim_group.add(&height_row);

        let outside_row = ActionRow::builder().title("Outside Dims:").build();
        outside_row.add_suffix(&outside);
        dim_group.add(&outside_row);

        scroll_content.append(&dim_group);

        // Box Configuration
        let config_group = PreferencesGroup::builder()
            .title("Box Configuration")
            .build();
        config_group.add(&Self::create_row("Box Type:", &box_type));
        config_group.add(&Self::create_row("Dividers X:", &dividers_x));
        config_group.add(&Self::create_row("Dividers Y:", &dividers_y));
        config_group.add(&Self::create_row("Divider Keying:", &divider_keying));

        let optimize_row = ActionRow::builder().title("Optimize Layout:").build();
        optimize_row.add_suffix(&optimize_layout);
        config_group.add(&optimize_row);

        scroll_content.append(&config_group);

        // Material Settings
        let mat_group = PreferencesGroup::builder()
            .title("Material Settings")
            .build();
        mat_group.add(&thickness_row);
        mat_group.add(&burn_row);
        scroll_content.append(&mat_group);

        // Finger Joint Settings
        let finger_group = PreferencesGroup::builder()
            .title("Finger Joint Settings (multiples of thickness)")
            .build();
        finger_group.add(&Self::create_row("Finger Width:", &finger_width));
        finger_group.add(&Self::create_row("Space Width:", &space_width));
        finger_group.add(&Self::create_row(
            "Surrounding Spaces:",
            &surrounding_spaces,
        ));
        finger_group.add(&play_row);
        finger_group.add(&extra_length_row);
        scroll_content.append(&finger_group);

        // Laser Settings
        let laser_group = PreferencesGroup::builder().title("Laser Settings").build();
        laser_group.add(&Self::create_row("Passes:", &passes));
        laser_group.add(&Self::create_row("Power (S):", &power));
        laser_group.add(&Self::create_row("Feed Rate:", &feed_rate));
        laser_group.add(&z_step_down_row);
        scroll_content.append(&laser_group);

        // Work Origin Offsets
        let offset_group = PreferencesGroup::builder()
            .title("Work Origin Offsets")
            .build();
        offset_group.add(&offset_x_row);
        offset_group.add(&offset_y_row);

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        offset_group.add(&home_row);

        scroll_content.append(&offset_group);

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
        let generate_gcode_btn = Button::with_label("Generate G-code");
        let generate_shapes_btn = Button::with_label("Generate Shapes");
        generate_gcode_btn.add_css_class("suggested-action");

        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_gcode_btn);
        action_box.append(&generate_shapes_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));

        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        content_box.append(&paned);

        let widgets = Rc::new(TabbedBoxWidgets {
            width,
            depth,
            height,
            outside,
            thickness,
            burn,
            finger_width,
            space_width,
            surrounding_spaces,
            play,
            extra_length,
            box_type,
            dividers_x,
            dividers_y,
            divider_keying,
            optimize_layout,
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
            let depth_unit = depth_unit.clone();
            let height_unit = height_unit.clone();
            let thickness_unit = thickness_unit.clone();
            let burn_unit = burn_unit.clone();
            let play_unit = play_unit.clone();
            let extra_length_unit = extra_length_unit.clone();
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
                        update_entry(&w.depth, &depth_unit);
                        update_entry(&w.height, &height_unit);
                        update_entry(&w.thickness, &thickness_unit);
                        update_entry(&w.burn, &burn_unit);
                        update_entry(&w.play, &play_unit);
                        update_entry(&w.extra_length, &extra_length_unit);
                        update_entry(&w.z_step_down, &z_step_down_unit);
                        update_entry(&w.offset_x, &offset_x_unit);
                        update_entry(&w.offset_y, &offset_y_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Connect Signals
        let widgets_gen = widgets.clone();
        let on_generate = on_generate.clone();
        let settings_gen = settings.clone();
        generate_gcode_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&widgets_gen, &settings_gen);
            let home_before = widgets_gen.home_before.is_active();

            // Create progress dialog
            let progress_window = gtk4::Window::builder()
                .title("Generating Box")
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

            let status_label = Label::new(Some("Generating box panels..."));
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

            let on_generate_clone = on_generate.clone();
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
                    let mut generator = Generator::new(params)?;
                    generator.generate()?;
                    let mut gcode = generator.to_gcode();

                    gcode = gcode.replace("$H\n", "").replace("$H", "");
                    if home_before {
                        gcode = format!("$H\n{}", gcode);
                    }
                    Ok(gcode)
                })();

                let _ = result_tx.send(result);
            });

            // Simulate progress
            let mut progress = 0.0;
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                // Check for result
                if let Ok(result) = result_rx.try_recv() {
                    progress_window_clone.close();

                    match result {
                        Ok(gcode) => {
                            on_generate_clone(gcode);
                        }
                        Err(e) => {
                            CamToolsView::show_error_dialog(
                                "Box Generation Failed",
                                &format!("Failed to generate box: {}", e),
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

        // Generate Shapes button
        let widgets_shapes = widgets.clone();
        let settings_shapes = settings.clone();
        let designer_view_shapes = designer_view.clone();
        generate_shapes_btn.connect_clicked(move |_| {
            if let Some(designer_view) = &designer_view_shapes {
                let params = Self::collect_params(&widgets_shapes, &settings_shapes);

                // Create progress dialog
                let progress_window = gtk4::Window::builder()
                    .title("Generating Box Shapes")
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

                let status_label = Label::new(Some("Generating box panel shapes..."));
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

                let designer_view_clone = designer_view.clone();
                let progress_window_clone = progress_window.clone();
                let progress_bar_clone = progress_bar.clone();

                let (result_tx, result_rx) = std::sync::mpsc::channel();
                let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();

                let cancel_tx_clone = cancel_tx.clone();
                cancel_button.connect_clicked(move |_| {
                    let _ = cancel_tx_clone.send(());
                });

                // Spawn background thread for shape generation
                std::thread::spawn(move || {
                    let result = (|| -> Result<Vec<Shape>, String> {
                        if cancel_rx.try_recv().is_ok() {
                            return Err("Cancelled by user".to_string());
                        }

                        let mut generator = Generator::new(params)?;
                        generator.generate()?;

                        // Convert paths to shapes
                        let shapes: Vec<Shape> = generator
                            .paths()
                            .iter()
                            .map(|path| {
                                // Convert Vec<Point> to Vec<gcodekit5_designer::Point>
                                let points: Vec<Point> = path
                                    .iter()
                                    .map(|p| Point::new(p.x as f64, p.y as f64))
                                    .collect();

                                // Create PathShape from points (closed paths for box panels)
                                let path_shape = PathShape::from_points(&points, true);
                                Shape::Path(path_shape)
                            })
                            .collect();

                        Ok(shapes)
                    })();

                    let _ = result_tx.send(result);
                });

                // Simulate progress and handle result
                let mut progress = 0.0;
                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    // Check for result
                    if let Ok(result) = result_rx.try_recv() {
                        progress_window_clone.close();

                        match result {
                            Ok(shapes) => {
                                // Add shapes to designer canvas
                                for shape in shapes {
                                    designer_view_clone.add_shape(shape);
                                }

                                // Switch to designer tab
                                if let Some(parent) = designer_view_clone.widget.parent() {
                                    if let Ok(stack) = parent.downcast::<gtk4::Stack>() {
                                        stack.set_visible_child_name("designer");
                                    }
                                }
                            }
                            Err(e) => {
                                CamToolsView::show_error_dialog(
                                    "Shape Generation Failed",
                                    &format!("Failed to generate box shapes: {}", e),
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
            } else {
                CamToolsView::show_error_dialog(
                    "Error",
                    "Designer view not available for shape generation.",
                );
            }
        });

        let widgets_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&widgets_save, &settings_save);
            Self::save_params(&params);
        });

        let widgets_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            Self::load_params(&widgets_load, &settings_load);
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

    fn collect_params(w: &TabbedBoxWidgets, settings: &Rc<SettingsController>) -> BoxParameters {
        let mut params = BoxParameters::default();
        let system = settings.persistence.borrow().config().ui.measurement_system;

        params.x = units::parse_length(&w.width.text(), system).unwrap_or(100.0);
        params.y = units::parse_length(&w.depth.text(), system).unwrap_or(100.0);
        params.h = units::parse_length(&w.height.text(), system).unwrap_or(100.0);
        params.outside = w.outside.is_active();
        params.thickness = units::parse_length(&w.thickness.text(), system).unwrap_or(3.0);
        params.burn = units::parse_length(&w.burn.text(), system).unwrap_or(0.1);

        params.finger_joint.finger = w.finger_width.text().parse().unwrap_or(2.0);
        params.finger_joint.space = w.space_width.text().parse().unwrap_or(2.0);
        params.finger_joint.surrounding_spaces = w.surrounding_spaces.text().parse().unwrap_or(2.0);
        params.finger_joint.play = units::parse_length(&w.play.text(), system).unwrap_or(0.0);
        params.finger_joint.extra_length =
            units::parse_length(&w.extra_length.text(), system).unwrap_or(0.0);

        // New params
        if let Some(id) = w.box_type.active_id() {
            params.box_type = BoxType::from(id.parse::<i32>().unwrap_or(0));
        }
        params.dividers_x = w.dividers_x.text().parse().unwrap_or(0);
        params.dividers_y = w.dividers_y.text().parse().unwrap_or(0);
        if let Some(id) = w.divider_keying.active_id() {
            params.key_divider_type = KeyDividerType::from(id.parse::<i32>().unwrap_or(0));
        }
        params.optimize_layout = w.optimize_layout.is_active();

        params.laser_passes = w.passes.text().parse().unwrap_or(3);
        params.laser_power = w.power.text().parse().unwrap_or(1000);
        params.feed_rate = w.feed_rate.text().parse().unwrap_or(500.0);
        params.z_step_down = units::parse_length(&w.z_step_down.text(), system).unwrap_or(0.1);

        params.offset_x = units::parse_length(&w.offset_x.text(), system).unwrap_or(10.0);
        params.offset_y = units::parse_length(&w.offset_y.text(), system).unwrap_or(10.0);
        params.num_axes = crate::device_status::get_active_num_axes();

        params
    }

    fn save_params(params: &BoxParameters) {
        let dialog = FileChooserDialog::new(
            Some("Save Box Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        dialog.set_current_name("box_params.json");

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

    fn load_params(w: &Rc<TabbedBoxWidgets>, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Load Box Parameters"),
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
                            if let Ok(params) = serde_json::from_str::<BoxParameters>(&content) {
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

    fn apply_params(w: &TabbedBoxWidgets, p: &BoxParameters, settings: &Rc<SettingsController>) {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        w.width.set_text(&units::format_length(p.x, system));
        w.depth.set_text(&units::format_length(p.y, system));
        w.height.set_text(&units::format_length(p.h, system));
        w.outside.set_active(p.outside);
        w.thickness
            .set_text(&units::format_length(p.thickness, system));
        w.burn.set_text(&units::format_length(p.burn, system));

        w.finger_width.set_text(&p.finger_joint.finger.to_string());
        w.space_width.set_text(&p.finger_joint.space.to_string());
        w.surrounding_spaces
            .set_text(&p.finger_joint.surrounding_spaces.to_string());
        w.play
            .set_text(&units::format_length(p.finger_joint.play, system));
        w.extra_length
            .set_text(&units::format_length(p.finger_joint.extra_length, system));

        // New params
        w.box_type
            .set_active_id(Some(&(p.box_type as i32).to_string()));
        w.dividers_x.set_text(&p.dividers_x.to_string());
        w.dividers_y.set_text(&p.dividers_y.to_string());
        w.divider_keying
            .set_active_id(Some(&(p.key_divider_type as i32).to_string()));
        w.optimize_layout.set_active(p.optimize_layout);

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

// Speeds & Feeds Calculator
