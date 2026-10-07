//! Speeds and Feeds Calculator Tool

use gtk4::prelude::*;
use gtk4::{Align, Box, Button, ComboBoxText, Label, Orientation, Paned, ScrolledWindow, Stack};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::rc::Rc;

use gcodekit5_core::data::materials::{Material, MaterialCategory, MaterialId, CuttingParameters};
use gcodekit5_core::data::tools::{Tool, ToolType, ToolCuttingParams, ToolId};
use gcodekit5_devicedb::model::DeviceProfile;
use gcodekit5_camtools::speeds_feeds::SpeedsFeedsCalculator;

use super::common::set_paned_initial_fraction;
use crate::ui::gtk::help_browser;
use gcodekit5_settings::SettingsController;

pub struct SpeedsFeedsTool {
    content: Box,
}

impl SpeedsFeedsTool {
    pub fn new(stack: &Stack, _settings: Rc<SettingsController>) -> Self {
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
            .label("Speeds and Feeds Calculator")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);
        header.append(&help_browser::make_help_button("speeds_feeds_calculator"));
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
            .label("Speeds and Feeds")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Calculate optimal cutting speeds and feed rates based on material properties and tool specifications. Uses standard machining formulas.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Results display area
        let results_box = Box::new(Orientation::Vertical, 6);
        results_box.set_vexpand(true);

        let results_frame = gtk4::Frame::new(Some("Calculated Results"));
        results_frame.set_margin_top(12);

        let results_content = Box::new(Orientation::Vertical, 6);
        results_content.set_margin_top(12);
        results_content.set_margin_bottom(12);
        results_content.set_margin_start(12);
        results_content.set_margin_end(12);

        let rpm_label = Label::builder()
            .label("RPM: --")
            .halign(Align::Start)
            .build();
        let feed_label = Label::builder()
            .label("Feed Rate: --")
            .halign(Align::Start)
            .build();
        let source_label = Label::builder()
            .label("")
            .css_classes(vec!["caption", "dim-label"])
            .halign(Align::Start)
            .wrap(true)
            .build();
        let warnings_label = Label::builder()
            .label("")
            .css_classes(vec!["caption", "warning"])
            .halign(Align::Start)
            .wrap(true)
            .build();

        results_content.append(&rpm_label);
        results_content.append(&feed_label);
        results_content.append(&source_label);
        results_content.append(&warnings_label);
        results_frame.set_child(Some(&results_content));
        results_box.append(&results_frame);
        sidebar.append(&results_box);

        // Content Area
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Material Selection
        let material_group = PreferencesGroup::builder().title("Material").build();
        let material_combo = ComboBoxText::new();
        material_combo.append(Some("aluminum"), "Aluminum");
        material_combo.append(Some("wood"), "Wood (Softwood)");
        material_combo.append(Some("acrylic"), "Acrylic");
        material_combo.append(Some("steel"), "Steel (Mild)");
        material_combo.set_active_id(Some("aluminum"));
        let material_row = ActionRow::builder().title("Material Type:").build();
        material_row.add_suffix(&material_combo);
        material_group.add(&material_row);
        scroll_content.append(&material_group);

        // Tool Selection
        let tool_group = PreferencesGroup::builder().title("Tool").build();
        let tool_combo = ComboBoxText::new();
        tool_combo.append(Some("endmill_6mm"), "End Mill - 6mm");
        tool_combo.append(Some("endmill_3mm"), "End Mill - 3mm");
        tool_combo.append(Some("vbit_30deg"), "V-Bit - 30°");
        tool_combo.set_active_id(Some("endmill_6mm"));
        let tool_row = ActionRow::builder().title("Tool Type:").build();
        tool_row.add_suffix(&tool_combo);
        tool_group.add(&tool_row);
        scroll_content.append(&tool_group);

        right_panel.append(&scrolled);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let calculate_btn = Button::with_label("Calculate");
        calculate_btn.add_css_class("suggested-action");
        action_box.append(&calculate_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        content_box.append(&paned);

        let rpm_label_calc = rpm_label.clone();
        let feed_label_calc = feed_label.clone();
        let source_label_calc = source_label.clone();
        let warnings_label_calc = warnings_label.clone();

        calculate_btn.connect_clicked(move |_| {
            // 1. Obtener los IDs seleccionados en la pantalla
            let selected_material = material_combo.active_id().unwrap_or_else(|| "aluminum".into());
            let selected_tool = tool_combo.active_id().unwrap_or_else(|| "endmill_6mm".into());

            // 2. Construir el objeto Material emulando la base de datos de manera limpia
            let (mat_id_str, mat_name, mat_cat) = match selected_material.as_str() {
                "aluminum" => ("metal_al_6061", "Aluminum 6061", MaterialCategory::NonFerrousMetal),
                "wood" => ("wood_oak_red", "Wood (Softwood)", MaterialCategory::Wood),
                "acrylic" => ("plastic_acrylic", "Acrylic", MaterialCategory::Plastic),
                "steel" => ("metal_steel_mild", "Steel (Mild)", MaterialCategory::FerrousMetal),
                _ => ("metal_al_6061", "Aluminum 6061", MaterialCategory::NonFerrousMetal),
            };

            let mut material = Material::new(
                MaterialId(mat_id_str.to_string()),
                mat_name.to_string(),
                mat_cat,
                "".to_string(),
            );

            // 3. Crear la herramienta nativa mapeando las geometrías de la interfaz
            let (t_id, t_name, t_type, t_dia, t_flutes) = match selected_tool.as_str() {
                "endmill_6mm" => ("endmill_6mm", "6mm Flat End Mill", ToolType::EndMillFlat, 6.0, 2),
                "endmill_3mm" => ("endmill_3mm", "3mm Ball End Mill", ToolType::EndMillBall, 3.0, 2),
                "vbit_30deg" => ("vbit_30deg", "V-Bit 30°", ToolType::VBit, 3.175, 1),
                _ => ("endmill_6mm", "6mm Flat End Mill", ToolType::EndMillFlat, 6.0, 2),
            };

            let mut tool = Tool::new(
                ToolId(t_id.to_string()),
                1,
                t_name.to_string(),
                t_type,
                t_dia,
                50.0,
            );
            tool.flutes = t_flutes;

            tool.params = ToolCuttingParams {
                rpm: 12000,
                rpm_range: (8000, 18000),
                feed_rate: 1500.0,
                plunge_rate: 750.0,
                stepover_percent: 50.0,
                depth_per_pass: 3.0,
            };

            let tool_type_key = match t_type {
                ToolType::EndMillFlat => "endmill_flat",
                ToolType::EndMillBall => "endmill_ball",
                ToolType::VBit => "vbit",
                ToolType::DrillBit => "drill",
                _ => "generic",
            };

            // 4. Inyectar los parámetros de corte correspondientes
            let cutting_params = match selected_material.as_str() {
                "aluminum" => CuttingParameters {
                    rpm_range: (8000, 12000),
                    feed_rate_range: (900.0, 2200.0),
                    surface_speed_m_min: Some(300.0),
                    chip_load_mm: Some(0.05),
                    ..Default::default()
                },
                "wood" => CuttingParameters {
                    rpm_range: (16000, 20000),
                    feed_rate_range: (1200.0, 2000.0),
                    ..Default::default()
                },
                "acrylic" => CuttingParameters {
                    rpm_range: (18000, 24000),
                    feed_rate_range: (1000.0, 1800.0),
                    ..Default::default()
                },
                "steel" => CuttingParameters {
                    rpm_range: (3000, 6000),
                    feed_rate_range: (300.0, 800.0),
                    ..Default::default()
                },
                _ => Default::default(),
            };
            material.set_cutting_params(tool_type_key.to_string(), cutting_params);

            // 5. Perfil de máquina por defecto
            let device = DeviceProfile::default();

            // 6. EJECUTAR EL CÁLCULO REAL
            let result = SpeedsFeedsCalculator::calculate(&material, &tool, &device);

            // 7. Pintar los resultados calculados dinámicamente en la interfaz
            rpm_label_calc.set_text(&format!("RPM: {}", result.rpm));
            feed_label_calc.set_text(&format!("Feed Rate: {:.0} mm/min", result.feed_rate));
            source_label_calc.set_text(&format!("Source: {}", result.source));

            if result.warnings.is_empty() {
                warnings_label_calc.set_text("");
            } else {
                warnings_label_calc.set_text(&format!("Warnings: {}", result.warnings.join(", ")));
            }
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }
}

// Spoilboard Surfacing Tool
