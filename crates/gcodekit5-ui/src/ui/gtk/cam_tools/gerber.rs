//! Gerber to G-Code Converter Tool

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, FileChooserAction, FileChooserDialog,
    Label, Orientation, Paned, ResponseType, ScrolledWindow, Stack,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use tracing::warn;

use super::common::{create_dimension_row, set_paned_initial_fraction};
use super::CamToolsView;
use crate::ui::gtk::help_browser;
use gcodekit5_camtools::gerber::{GerberConverter, GerberLayerType, GerberParameters};
use gcodekit5_core::units;
use gcodekit5_settings::SettingsController;

pub struct GerberTool {
    content: Box,
}

struct GerberWidgets {
    layer_type: ComboBoxText,
    feed_rate: Entry,
    spindle_speed: Entry,
    board_width: Entry,
    board_height: Entry,
    offset_x: Entry,
    offset_y: Entry,
    generate_alignment_holes: CheckButton,
    alignment_hole_diameter: Entry,
    alignment_hole_margin: Entry,
    cut_depth: Entry,
    safe_z: Entry,
    tool_diameter: Entry,
    isolation_width: Entry,
    rubout: CheckButton,
    use_board_outline: CheckButton,
    layer_files: Rc<RefCell<HashMap<GerberLayerType, PathBuf>>>,
    file_label: Label,
}

impl GerberTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        on_generate: Rc<F>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);
        // content_box.set_hexpand(true);
        // content_box.set_vexpand(true);

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
            .label("Gerber to G-Code")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);

        header.append(&help_browser::make_help_button("gerber"));

        content_box.append(&header);

        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar
        let sidebar = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .min_content_width(300)
            .build();
        let sidebar_box = Box::new(Orientation::Vertical, 12);
        sidebar_box.set_margin_top(12);
        sidebar_box.set_margin_bottom(12);
        sidebar_box.set_margin_start(12);
        sidebar_box.set_margin_end(12);
        sidebar.set_child(Some(&sidebar_box));

        // File Selection
        let file_group = PreferencesGroup::new();
        file_group.set_title("Gerber Project");

        let file_row = ActionRow::builder().title("Select Directory").build();
        let file_btn = Button::with_label("Browse...");
        let file_label = Label::new(Some("No directory selected"));
        file_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        file_label.set_width_chars(20);

        let file_box = Box::new(Orientation::Horizontal, 6);
        file_box.append(&file_label);
        file_box.append(&file_btn);
        file_row.add_suffix(&file_box);
        file_group.add(&file_row);
        sidebar_box.append(&file_group);

        let layer_files = Rc::new(RefCell::new(HashMap::new()));
        let layer_files_clone = layer_files.clone();
        let file_label_clone = file_label.clone();

        // Helper to detect layers
        let detect_layers = |path: &std::path::Path| -> HashMap<GerberLayerType, PathBuf> {
            let mut map = HashMap::new();
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default()
                        .to_lowercase();
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();

                    // Simple heuristic matching
                    if ext == "gtl" || name.contains("f.cu") || name.contains("top.gbr") {
                        map.insert(GerberLayerType::TopCopper, path.clone());
                    } else if ext == "gbl" || name.contains("b.cu") || name.contains("bot.gbr") {
                        map.insert(GerberLayerType::BottomCopper, path.clone());
                    } else if ext == "gts" || name.contains("f.mask") || name.contains("smask_top")
                    {
                        map.insert(GerberLayerType::TopSolderMask, path.clone());
                    } else if ext == "gbs" || name.contains("b.mask") || name.contains("smask_bot")
                    {
                        map.insert(GerberLayerType::BottomSolderMask, path.clone());
                    } else if ext == "gto"
                        || name.contains("f.silks")
                        || name.contains("legend_top")
                    {
                        map.insert(GerberLayerType::TopScreenPrint, path.clone());
                    } else if ext == "gbo"
                        || name.contains("b.silks")
                        || name.contains("legend_bot")
                    {
                        map.insert(GerberLayerType::BottomScreenPrint, path.clone());
                    } else if ext == "drl" || ext == "txt" || name.contains("drill") {
                        map.insert(GerberLayerType::DrillHoles, path.clone());
                    } else if ext == "gko"
                        || ext == "gm1"
                        || name.contains("edge.cuts")
                        || name.contains("outline")
                    {
                        map.insert(GerberLayerType::BoardOutline, path.clone());
                    }
                }
            }
            map
        };

        file_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Open Gerber Directory"),
                None::<&gtk4::Window>,
                FileChooserAction::SelectFolder,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Select", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(800, 600);

            let lf = layer_files_clone.clone();
            let fl = file_label_clone.clone();

            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let map = detect_layers(&path);
                            *lf.borrow_mut() = map;
                            fl.set_text(
                                path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unnamed"),
                            );
                        }
                    }
                }
                d.close();
            });
            dialog.show();
        });

        // Parameters
        let params_group = PreferencesGroup::new();
        params_group.set_title("Parameters");

        let layer_type = ComboBoxText::new();
        layer_type.append(Some("TopCopper"), "Top Copper");
        layer_type.append(Some("BottomCopper"), "Bottom Copper");
        layer_type.append(Some("TopSolderMask"), "Top Solder Mask");
        layer_type.append(Some("BottomSolderMask"), "Bottom Solder Mask");
        layer_type.append(Some("TopScreenPrint"), "Top Screen Print");
        layer_type.append(Some("BottomScreenPrint"), "Bottom Screen Print");
        layer_type.append(Some("DrillHoles"), "Drill Holes");
        layer_type.append(Some("BoardOutline"), "Board Outline");
        layer_type.set_active_id(Some("TopCopper"));

        let layer_row = ActionRow::builder().title("Layer Type").build();
        layer_row.add_suffix(&layer_type);
        params_group.add(&layer_row);

        let (w_row, board_width, _) = create_dimension_row("Board Width", 100.0, &settings);
        params_group.add(&w_row);
        let (h_row, board_height, _) = create_dimension_row("Board Height", 100.0, &settings);
        params_group.add(&h_row);

        let (ox_row, offset_x, _) = create_dimension_row("Offset X", 0.0, &settings);
        params_group.add(&ox_row);
        let (oy_row, offset_y, _) = create_dimension_row("Offset Y", 0.0, &settings);
        params_group.add(&oy_row);

        let feed_rate = Entry::builder()
            .text("100.0")
            .width_chars(8)
            .valign(Align::Center)
            .build();
        let fr_row = ActionRow::builder().title("Feed Rate").build();
        fr_row.add_suffix(&feed_rate);
        params_group.add(&fr_row);

        let spindle_speed = Entry::builder()
            .text("10000.0")
            .width_chars(8)
            .valign(Align::Center)
            .build();
        let ss_row = ActionRow::builder().title("Spindle Speed (RPM)").build();
        ss_row.add_suffix(&spindle_speed);
        params_group.add(&ss_row);

        let (cd_row, cut_depth, _) = create_dimension_row("Cut Depth", -0.1, &settings);
        params_group.add(&cd_row);

        let (sz_row, safe_z, _) = create_dimension_row("Safe Z", 5.0, &settings);
        params_group.add(&sz_row);

        let (td_row, tool_diameter, _) = create_dimension_row("Tool Diameter", 0.1, &settings);
        params_group.add(&td_row);

        let (iw_row, isolation_width, _) = create_dimension_row("Isolation Width", 0.0, &settings);
        params_group.add(&iw_row);

        let rubout = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let rubout_row = ActionRow::builder().title("Remove Excess Copper").build();
        rubout_row.add_suffix(&rubout);
        params_group.add(&rubout_row);

        let use_board_outline = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let ubo_row = ActionRow::builder()
            .title("Use Board Outline for Rubout")
            .build();
        ubo_row.add_suffix(&use_board_outline);
        params_group.add(&ubo_row);

        sidebar_box.append(&params_group);

        // Alignment Holes
        let align_group = PreferencesGroup::new();
        align_group.set_title("Alignment Holes");

        let generate_alignment_holes = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let ah_row = ActionRow::builder()
            .title("Generate Alignment Holes")
            .build();
        ah_row.add_suffix(&generate_alignment_holes);
        align_group.add(&ah_row);

        let (ahd_row, alignment_hole_diameter, _) =
            create_dimension_row("Hole Diameter", 3.175, &settings);
        align_group.add(&ahd_row);

        let (ahm_row, alignment_hole_margin, _) = create_dimension_row("Margin", 5.0, &settings);
        align_group.add(&ahm_row);

        sidebar_box.append(&align_group);

        // Left Panel (Description)
        let left_panel = Box::new(Orientation::Vertical, 12);
        left_panel.add_css_class("sidebar");
        left_panel.set_margin_top(24);
        left_panel.set_margin_bottom(24);
        left_panel.set_margin_start(24);
        left_panel.set_margin_end(24);

        let title_label = Label::builder()
            .label("Gerber to G-Code")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        left_panel.append(&title_label);

        let desc = Label::builder()
            .label("Convert standard Gerber files (PCB layers) into G-Code for CNC isolation routing. Supports copper clearing (rubout), alignment holes, and custom tool parameters.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        left_panel.append(&desc);

        // Right Panel (Controls + Actions)
        let right_panel = Box::new(Orientation::Vertical, 0);
        sidebar.set_vexpand(true);
        right_panel.append(&sidebar);

        // Actions
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_start(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label("Load");
        let save_btn = Button::with_label("Save");
        let cancel_btn = Button::with_label("Cancel");
        let generate_btn = Button::with_label("Generate G-Code");
        generate_btn.add_css_class("suggested-action");

        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&left_panel));
        paned.set_end_child(Some(&right_panel));
        set_paned_initial_fraction(&paned, 0.40);
        content_box.append(&paned);

        let widgets = Rc::new(GerberWidgets {
            layer_type: layer_type.clone(),
            feed_rate,
            spindle_speed,
            board_width,
            board_height,
            offset_x,
            offset_y,
            generate_alignment_holes,
            alignment_hole_diameter,
            alignment_hole_margin,
            cut_depth,
            safe_z,
            tool_diameter,
            isolation_width,
            rubout,
            use_board_outline,
            layer_files,
            file_label,
        });

        // Update label when layer changes
        let w_layer = widgets.clone();
        layer_type.connect_changed(move |c| {
            if let Some(id) = c.active_id() {
                let layer_type = match id.as_str() {
                    "TopCopper" => GerberLayerType::TopCopper,
                    "BottomCopper" => GerberLayerType::BottomCopper,
                    "TopSolderMask" => GerberLayerType::TopSolderMask,
                    "BottomSolderMask" => GerberLayerType::BottomSolderMask,
                    "TopScreenPrint" => GerberLayerType::TopScreenPrint,
                    "BottomScreenPrint" => GerberLayerType::BottomScreenPrint,
                    "DrillHoles" => GerberLayerType::DrillHoles,
                    "BoardOutline" => GerberLayerType::BoardOutline,
                    _ => GerberLayerType::TopCopper,
                };

                let files = w_layer.layer_files.borrow();
                if let Some(path) = files.get(&layer_type) {
                    w_layer.file_label.set_text(
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unnamed"),
                    );
                } else {
                    w_layer.file_label.set_text("Layer not found in directory");
                }
            }
        });

        // Connect Generate
        let w_gen = widgets.clone();
        let settings_gen = settings.clone();
        let on_gen = on_generate.clone();

        generate_btn.connect_clicked(move |_| {
            let files = w_gen.layer_files.borrow();
            if files.is_empty() {
                CamToolsView::show_error_dialog("Error", "Please select a Gerber directory.");
                return;
            }

            let params = Self::collect_params(&w_gen, &settings_gen);

            let path = match files.get(&params.layer_type) {
                Some(p) => p,
                None => {
                    CamToolsView::show_error_dialog(
                        "Error",
                        "Selected layer not found in directory.",
                    );
                    return;
                }
            };

            warn!("Reading Gerber file: {:?}", path);

            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    CamToolsView::show_error_dialog(
                        "Error",
                        &format!("Failed to read file: {}", e),
                    );
                    return;
                }
            };

            warn!("Read {} bytes", content.len());
            warn!(
                "Generating G-Code for layer: {:?} with params: {:?}",
                params.layer_type, params
            );

            match GerberConverter::generate(&params, &content) {
                Ok(gcode) => {
                    warn!("Generated {} bytes of G-Code", gcode.len());
                    on_gen(gcode)
                }
                Err(e) => {
                    warn!("Generation failed: {}", e);
                    CamToolsView::show_error_dialog("Generation Failed", &e.to_string())
                }
            }
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

        let stack_clone = stack.clone();
        cancel_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn collect_params(w: &GerberWidgets, settings: &Rc<SettingsController>) -> GerberParameters {
        let system = settings.persistence.borrow().config().ui.measurement_system;

        let layer_type = match w.layer_type.active_id().as_deref() {
            Some("TopCopper") => GerberLayerType::TopCopper,
            Some("BottomCopper") => GerberLayerType::BottomCopper,
            Some("TopSolderMask") => GerberLayerType::TopSolderMask,
            Some("BottomSolderMask") => GerberLayerType::BottomSolderMask,
            Some("TopScreenPrint") => GerberLayerType::TopScreenPrint,
            Some("BottomScreenPrint") => GerberLayerType::BottomScreenPrint,
            Some("DrillHoles") => GerberLayerType::DrillHoles,
            Some("BoardOutline") => GerberLayerType::BoardOutline,
            _ => GerberLayerType::TopCopper,
        };

        GerberParameters {
            layer_type,
            feed_rate: w.feed_rate.text().parse().unwrap_or(100.0),
            spindle_speed: w.spindle_speed.text().parse().unwrap_or(10000.0),
            board_width: units::parse_length(&w.board_width.text(), system).unwrap_or(100.0) as f32,
            board_height: units::parse_length(&w.board_height.text(), system).unwrap_or(100.0)
                as f32,
            offset_x: units::parse_length(&w.offset_x.text(), system).unwrap_or(0.0) as f32,
            offset_y: units::parse_length(&w.offset_y.text(), system).unwrap_or(0.0) as f32,
            generate_alignment_holes: w.generate_alignment_holes.is_active(),
            alignment_hole_diameter: units::parse_length(&w.alignment_hole_diameter.text(), system)
                .unwrap_or(3.175) as f32,
            alignment_hole_margin: units::parse_length(&w.alignment_hole_margin.text(), system)
                .unwrap_or(5.0) as f32,
            cut_depth: units::parse_length(&w.cut_depth.text(), system).unwrap_or(-0.1) as f32,
            safe_z: units::parse_length(&w.safe_z.text(), system).unwrap_or(5.0) as f32,
            tool_diameter: units::parse_length(&w.tool_diameter.text(), system).unwrap_or(0.1)
                as f32,
            isolation_width: units::parse_length(&w.isolation_width.text(), system).unwrap_or(0.0)
                as f32,
            rubout: w.rubout.is_active(),
            use_board_outline: w.use_board_outline.is_active(),
            directory: {
                let files = w.layer_files.borrow();
                // Just grab the parent dir of the first file found, or None
                files
                    .values()
                    .next()
                    .and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string()))
            },
            num_axes: crate::device_status::get_active_num_axes(),
        }
    }

    fn save_params(params: &GerberParameters) {
        let dialog = FileChooserDialog::new(
            Some("Save Gerber Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);
        dialog.set_current_name("gerber_params.json");

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

    fn load_params(w: &Rc<GerberWidgets>, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Load Gerber Parameters"),
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
                            match serde_json::from_str::<GerberParameters>(&content) {
                                Ok(params) => {
                                    Self::apply_params(&w_clone, &params, &settings_clone)
                                }
                                Err(e) => warn!("Failed to load parameters: {}", e),
                            }
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn apply_params(w: &GerberWidgets, p: &GerberParameters, settings: &Rc<SettingsController>) {
        let system = settings.persistence.borrow().config().ui.measurement_system;

        let layer_id = match p.layer_type {
            GerberLayerType::TopCopper => "TopCopper",
            GerberLayerType::BottomCopper => "BottomCopper",
            GerberLayerType::TopSolderMask => "TopSolderMask",
            GerberLayerType::BottomSolderMask => "BottomSolderMask",
            GerberLayerType::TopScreenPrint => "TopScreenPrint",
            GerberLayerType::BottomScreenPrint => "BottomScreenPrint",
            GerberLayerType::DrillHoles => "DrillHoles",
            GerberLayerType::BoardOutline => "BoardOutline",
        };
        w.layer_type.set_active_id(Some(layer_id));

        w.feed_rate.set_text(&p.feed_rate.to_string());
        w.spindle_speed.set_text(&p.spindle_speed.to_string());
        w.board_width
            .set_text(&units::format_length(p.board_width, system));
        w.board_height
            .set_text(&units::format_length(p.board_height, system));
        w.offset_x
            .set_text(&units::format_length(p.offset_x, system));
        w.offset_y
            .set_text(&units::format_length(p.offset_y, system));
        w.generate_alignment_holes
            .set_active(p.generate_alignment_holes);
        w.alignment_hole_diameter
            .set_text(&units::format_length(p.alignment_hole_diameter, system));
        w.alignment_hole_margin
            .set_text(&units::format_length(p.alignment_hole_margin, system));
        w.cut_depth
            .set_text(&units::format_length(p.cut_depth, system));
        w.safe_z.set_text(&units::format_length(p.safe_z, system));
        w.tool_diameter
            .set_text(&units::format_length(p.tool_diameter, system));
        w.isolation_width
            .set_text(&units::format_length(p.isolation_width, system));
        w.rubout.set_active(p.rubout);
        w.use_board_outline.set_active(p.use_board_outline);

        if let Some(dir) = &p.directory {
            let path = PathBuf::from(dir);
            if path.exists() {
                // Re-run detection
                // We need access to detect_layers logic here, but it's inside new().
                // We can duplicate the logic or refactor. Duplication is easier for now as it's small.
                let mut map = HashMap::new();
                if let Ok(entries) = fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if !path.is_file() {
                            continue;
                        }

                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy())
                            .unwrap_or_default()
                            .to_lowercase();
                        let ext = path
                            .extension()
                            .map(|e| e.to_string_lossy().to_lowercase())
                            .unwrap_or_default();

                        if ext == "gtl" || name.contains("f.cu") || name.contains("top.gbr") {
                            map.insert(GerberLayerType::TopCopper, path.clone());
                        } else if ext == "gbl" || name.contains("b.cu") || name.contains("bot.gbr")
                        {
                            map.insert(GerberLayerType::BottomCopper, path.clone());
                        } else if ext == "gts"
                            || name.contains("f.mask")
                            || name.contains("smask_top")
                        {
                            map.insert(GerberLayerType::TopSolderMask, path.clone());
                        } else if ext == "gbs"
                            || name.contains("b.mask")
                            || name.contains("smask_bot")
                        {
                            map.insert(GerberLayerType::BottomSolderMask, path.clone());
                        } else if ext == "gto"
                            || name.contains("f.silks")
                            || name.contains("legend_top")
                        {
                            map.insert(GerberLayerType::TopScreenPrint, path.clone());
                        } else if ext == "gbo"
                            || name.contains("b.silks")
                            || name.contains("legend_bot")
                        {
                            map.insert(GerberLayerType::BottomScreenPrint, path.clone());
                        } else if ext == "drl" || ext == "txt" || name.contains("drill") {
                            map.insert(GerberLayerType::DrillHoles, path.clone());
                        } else if ext == "gko"
                            || ext == "gm1"
                            || name.contains("edge.cuts")
                            || name.contains("outline")
                        {
                            map.insert(GerberLayerType::BoardOutline, path.clone());
                        }
                    }
                }
                *w.layer_files.borrow_mut() = map;
                w.file_label.set_text(
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unnamed"),
                );

                // Update label for selected layer
                let files = w.layer_files.borrow();
                if let Some(path) = files.get(&p.layer_type) {
                    w.file_label.set_text(
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unnamed"),
                    );
                } else {
                    w.file_label.set_text("Layer not found in directory");
                }
            }
        }
    }
}

// Drill Press Tool
