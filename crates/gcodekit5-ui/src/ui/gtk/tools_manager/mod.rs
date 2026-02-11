mod event_handlers;
mod ui_builders;

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, ComboBoxText, Entry, Frame, Label, ListBox, ListBoxRow, Orientation, Paned,
    PolicyType, ScrolledWindow, SearchEntry, SelectionMode, SpinButton, Stack, StackSwitcher,
    TextView,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::gtk::help_browser;
use crate::ui::tools_manager_backend::{string_to_tool_material, ToolsManagerBackend};
use gcodekit5_core::data::tools::{ShankType, Tool, ToolCoating, ToolId, ToolMaterial, ToolType};
use gcodekit5_settings::manager::SettingsManager;
use gcodekit5_settings::SettingsController;

pub(crate) const ROW_TOOL_ID_KEY: &str = "gcodekit5-tool-id";

#[derive(Clone)]
pub struct ToolsManagerView {
    pub widget: Paned,
    pub(crate) backend: Rc<RefCell<ToolsManagerBackend>>,
    pub(crate) settings_controller: Rc<SettingsController>,

    // Left panel widgets
    pub(crate) tools_list: ListBox,
    pub(crate) search_entry: SearchEntry,
    pub(crate) type_filter: ComboBoxText,
    pub(crate) material_filter: ComboBoxText,
    pub(crate) dia_min: Entry,
    pub(crate) dia_max: Entry,
    // Right panel
    pub(crate) right_panel_stack: Stack,

    // Edit form widgets
    pub(crate) edit_id: Entry,
    pub(crate) edit_number: SpinButton,
    pub(crate) edit_name: Entry,
    pub(crate) edit_tool_type: ComboBoxText,
    pub(crate) edit_material: ComboBoxText,
    pub(crate) edit_coating: ComboBoxText,
    pub(crate) edit_shank: ComboBoxText,

    pub(crate) edit_diameter: Entry,
    pub(crate) edit_length: Entry,
    pub(crate) edit_flute_length: Entry,
    pub(crate) edit_shaft_diameter: Entry,
    pub(crate) edit_flutes: SpinButton,
    pub(crate) edit_corner_radius: Entry,
    pub(crate) edit_tip_angle: Entry,

    pub(crate) edit_manufacturer: Entry,
    pub(crate) edit_part_number: Entry,
    pub(crate) edit_description: TextView,
    pub(crate) edit_notes: TextView,

    pub(crate) error_label: Label,

    // State
    pub(crate) selected_tool: Rc<RefCell<Option<Tool>>>,
    pub(crate) is_creating: Rc<RefCell<bool>>,
    pub(crate) last_selected_tool_id: Rc<RefCell<Option<String>>>,

    // Action buttons
    pub(crate) save_btn: Button,
    pub(crate) cancel_btn: Button,
    pub(crate) delete_btn: Button,

    pub(crate) new_btn: Button,
    pub(crate) import_zip_btn: Button,
    pub(crate) import_json_btn: Button,
    pub(crate) export_btn: Button,
    pub(crate) reset_btn: Button,
}

impl ToolsManagerView {
    pub fn new(settings_controller: Rc<SettingsController>) -> Rc<Self> {
        let backend = Rc::new(RefCell::new(ToolsManagerBackend::new()));

        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        // LEFT SIDEBAR
        let sidebar = Box::new(Orientation::Vertical, 10);
        sidebar.add_css_class("sidebar");
        sidebar.set_width_request(280);
        sidebar.set_margin_top(10);
        sidebar.set_margin_bottom(10);
        sidebar.set_margin_start(10);
        sidebar.set_margin_end(10);

        let header_box = Box::new(Orientation::Horizontal, 10);
        header_box.set_margin_start(5);
        let title = Label::new(Some("CNC Tools"));
        title.add_css_class("title-4");
        title.set_halign(Align::Start);
        header_box.append(&title);

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header_box.append(&spacer);
        header_box.append(&help_browser::make_help_button("tools_manager"));

        sidebar.append(&header_box);

        // Search
        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search (name, id, type, Ø)…"));
        sidebar.append(&search_entry);

        // Type filter
        let type_filter = ComboBoxText::new();
        type_filter.append(Some("all"), "All Types");
        type_filter.append(Some("endmill_flat"), "Flat End Mill");
        type_filter.append(Some("endmill_ball"), "Ball End Mill");
        type_filter.append(Some("endmill_cr"), "Corner Radius End Mill");
        type_filter.append(Some("vbit"), "V-Bit");
        type_filter.append(Some("drill"), "Drill Bit");
        type_filter.append(Some("spot"), "Spot Drill");
        type_filter.append(Some("engraving"), "Engraving Bit");
        type_filter.append(Some("chamfer"), "Chamfer Tool");
        type_filter.append(Some("specialty"), "Specialty");
        type_filter.set_active_id(Some("all"));
        sidebar.append(&type_filter);

        // Material filter
        let material_filter = ComboBoxText::new();
        material_filter.append(Some("all"), "All Materials");
        material_filter.append(Some("HSS"), "HSS");
        material_filter.append(Some("Carbide"), "Carbide");
        material_filter.append(Some("Coated Carbide"), "Coated Carbide");
        material_filter.append(Some("Diamond"), "Diamond");
        material_filter.set_active_id(Some("all"));
        sidebar.append(&material_filter);

        // Extra filters (single-row layout)
        let filters_frame = Frame::new(Some("Filters"));
        let filters_row = Box::new(Orientation::Horizontal, 8);
        filters_row.set_margin_top(8);
        filters_row.set_margin_bottom(8);
        filters_row.set_margin_start(8);
        filters_row.set_margin_end(8);

        let dia_min = Entry::new();
        dia_min.set_placeholder_text(Some("Min Ø (mm)"));
        dia_min.set_width_chars(8);
        let dia_max = Entry::new();
        dia_max.set_placeholder_text(Some("Max Ø (mm)"));
        dia_max.set_width_chars(8);

        filters_row.append(&dia_min);
        filters_row.append(&dia_max);

        filters_frame.set_child(Some(&filters_row));
        sidebar.append(&filters_frame);

        // Tools list
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);

        let tools_list = ListBox::new();
        tools_list.set_selection_mode(SelectionMode::Single);
        tools_list.add_css_class("boxed-list");
        scroll.set_child(Some(&tools_list));
        sidebar.append(&scroll);

        // Actions (moved to right-hand sidebar)
        let new_btn = Button::with_label("➕ New Tool");
        new_btn.add_css_class("suggested-action");
        new_btn.set_tooltip_text(Some("Create a new custom tool"));

        let actions_frame = Frame::new(Some("Library"));
        let actions_box = Box::new(Orientation::Vertical, 8);
        actions_box.set_margin_top(8);
        actions_box.set_margin_bottom(8);
        actions_box.set_margin_start(8);
        actions_box.set_margin_end(8);

        let import_zip_btn = Button::with_label("Import GTC (.zip)…");
        import_zip_btn.set_tooltip_text(Some("Import tool catalog from a GTC zip package"));
        let import_json_btn = Button::with_label("Import GTC (.json)…");
        import_json_btn.set_tooltip_text(Some("Import tool catalog from a GTC JSON file"));
        let export_btn = Button::with_label("Export Custom Tools…");
        export_btn.set_tooltip_text(Some("Export custom tools to a JSON file"));
        let reset_btn = Button::with_label("Reset Custom Tools…");
        reset_btn.add_css_class("destructive-action");
        reset_btn.set_tooltip_text(Some(
            "Remove all custom/imported tools and delete local storage",
        ));

        actions_box.append(&import_zip_btn);
        actions_box.append(&import_json_btn);
        actions_box.append(&export_btn);
        actions_box.append(&reset_btn);
        actions_frame.set_child(Some(&actions_box));

        widget.set_start_child(Some(&sidebar));

        // RIGHT PANEL (editor + library sidebar)
        let right_panel_stack = Stack::new();
        right_panel_stack.set_hexpand(true);
        right_panel_stack.set_vexpand(true);

        let empty_state = Box::new(Orientation::Vertical, 0);
        empty_state.set_valign(Align::Center);
        empty_state.set_halign(Align::Center);
        empty_state.set_vexpand(true);
        empty_state.set_hexpand(true);

        let empty_label = Label::new(Some("Please select or create a tool to edit"));
        empty_label.add_css_class("title-2");
        empty_label.add_css_class("dim-label");
        empty_state.append(&empty_label);

        right_panel_stack.add_named(&empty_state, Some("empty"));

        // Edit form container
        let main_content = Box::new(Orientation::Vertical, 10);
        main_content.set_margin_top(20);
        main_content.set_margin_bottom(20);
        main_content.set_margin_start(20);
        main_content.set_margin_end(20);

        let action_bar = Box::new(Orientation::Horizontal, 10);
        let save_btn = Button::with_label("💾 Save");
        save_btn.add_css_class("suggested-action");
        save_btn.set_sensitive(false);
        save_btn.set_tooltip_text(Some("Save changes"));

        let cancel_btn = Button::with_label("Cancel");
        cancel_btn.set_sensitive(false);
        cancel_btn.set_tooltip_text(Some("Discard changes"));

        let delete_btn = Button::with_label("🗑️ Delete");
        delete_btn.add_css_class("destructive-action");
        delete_btn.set_sensitive(false);
        delete_btn.set_tooltip_text(Some("Delete this tool"));

        action_bar.append(&save_btn);
        action_bar.append(&cancel_btn);
        action_bar.append(&delete_btn);

        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        action_bar.append(&spacer);

        main_content.append(&action_bar);

        let error_label = Label::new(None);
        error_label.set_halign(Align::Start);
        error_label.set_xalign(0.0);
        error_label.add_css_class("error");
        error_label.set_visible(false);
        main_content.append(&error_label);

        // Tabs
        let stack = Stack::new();
        stack.set_vexpand(true);

        let (
            basic_page,
            edit_id,
            edit_number,
            edit_name,
            edit_tool_type,
            edit_material,
            edit_coating,
            edit_shank,
        ) = Self::create_basic_tab();
        let (
            geometry_page,
            edit_diameter,
            edit_length,
            edit_flute_length,
            edit_shaft_diameter,
            edit_flutes,
            edit_corner_radius,
            edit_tip_angle,
        ) = Self::create_geometry_tab();
        let (mfg_page, edit_manufacturer, edit_part_number, edit_description) =
            Self::create_manufacturer_tab();
        let (notes_page, edit_notes) = Self::create_notes_tab();

        stack.add_titled(&basic_page, Some("basic"), "Basic Info");
        stack.add_titled(&geometry_page, Some("geometry"), "Geometry");
        stack.add_titled(&mfg_page, Some("manufacturer"), "Manufacturer");
        stack.add_titled(&notes_page, Some("notes"), "Notes");

        let switcher = StackSwitcher::new();
        switcher.set_stack(Some(&stack));
        switcher.set_halign(Align::Center);

        main_content.append(&switcher);
        main_content.append(&stack);

        right_panel_stack.add_named(&main_content, Some("edit"));
        right_panel_stack.set_visible_child_name("empty");

        let right_sidebar = Box::new(Orientation::Vertical, 10);
        right_sidebar.add_css_class("sidebar");
        right_sidebar.set_width_request(280);
        right_sidebar.set_margin_top(10);
        right_sidebar.set_margin_bottom(10);
        right_sidebar.set_margin_start(10);
        right_sidebar.set_margin_end(10);

        right_sidebar.append(&new_btn);
        right_sidebar.append(&actions_frame);

        let right_container = Box::new(Orientation::Horizontal, 0);
        right_container.set_hexpand(true);
        right_container.set_vexpand(true);
        right_container.append(&right_panel_stack);
        right_container.append(&right_sidebar);

        widget.set_end_child(Some(&right_container));

        let view = Rc::new(Self {
            widget,
            backend: backend.clone(),
            settings_controller,
            tools_list,
            search_entry,
            type_filter,
            material_filter,
            dia_min,
            dia_max,
            right_panel_stack: right_panel_stack.clone(),
            edit_id,
            edit_number,
            edit_name,
            edit_tool_type,
            edit_material,
            edit_coating,
            edit_shank,
            edit_diameter,
            edit_length,
            edit_flute_length,
            edit_shaft_diameter,
            edit_flutes,
            edit_corner_radius,
            edit_tip_angle,
            edit_manufacturer,
            edit_part_number,
            edit_description,
            edit_notes,
            error_label,
            selected_tool: Rc::new(RefCell::new(None)),
            is_creating: Rc::new(RefCell::new(false)),
            last_selected_tool_id: Rc::new(RefCell::new(None)),
            save_btn,
            cancel_btn,
            delete_btn,
            new_btn,
            import_zip_btn,
            import_json_btn,
            export_btn,
            reset_btn,
        });

        view.restore_ui_state();
        view.setup_splitter_persistence();
        view.setup_event_handlers();
        view.load_tools();

        view
    }

    fn persist_tools_sidebar_position(&self, pos: i32) {
        {
            let mut p = self.settings_controller.persistence.borrow_mut();
            p.config_mut().ui.tools_sidebar_position = Some(pos);
            if let Ok(path) = SettingsManager::config_file_path() {
                let _ = SettingsManager::ensure_config_dir();
                let _ = p.save_to_file(&path);
            }
        }
    }

    fn persist_ui_state(&self) {
        let search = self.search_entry.text().to_string();
        let type_id = self.type_filter.active_id().map(|s| s.to_string());
        let mat_id = self.material_filter.active_id().map(|s| s.to_string());
        let dia_min = self.dia_min.text().to_string();
        let dia_max = self.dia_max.text().to_string();

        let parsed_dia_min = dia_min.parse::<f32>().ok();
        let parsed_dia_max = dia_max.parse::<f32>().ok();
        {
            let mut p = self.settings_controller.persistence.borrow_mut();
            p.config_mut().ui.tools_manager_search = search;
            p.config_mut().ui.tools_manager_type_filter = type_id.unwrap_or_default();
            p.config_mut().ui.tools_manager_material_filter = mat_id.unwrap_or_default();
            p.config_mut().ui.tools_manager_dia_min = parsed_dia_min;
            p.config_mut().ui.tools_manager_dia_max = parsed_dia_max;
            p.config_mut().ui.tools_manager_selected_tool =
                self.last_selected_tool_id.borrow().clone();

            if let Ok(path) = SettingsManager::config_file_path() {
                let _ = SettingsManager::ensure_config_dir();
                let _ = p.save_to_file(&path);
            }
        }
    }

    fn select_row_by_tool_id(&self, list: &ListBox, tool_id: &str) {
        let mut child = list.first_child();
        while let Some(w) = child {
            let next = w.next_sibling();
            if let Ok(row) = w.downcast::<ListBoxRow>() {
                let stored = unsafe {
                    row.data::<String>(ROW_TOOL_ID_KEY)
                        .map(|p| p.as_ref().clone())
                };
                if stored.as_deref() == Some(tool_id) {
                    list.select_row(Some(&row));
                    return;
                }
            }
            child = next;
        }
    }

    fn load_tools(&self) {
        while let Some(child) = self.tools_list.first_child() {
            self.tools_list.remove(&child);
        }

        let backend = self.backend.borrow();
        let mut tools: Vec<&Tool> = backend.get_all_tools();

        let q = self.search_entry.text().to_string().to_lowercase();
        if !q.is_empty() {
            tools.retain(|t| {
                let dia = format!("{:.3}", t.diameter);
                t.name.to_lowercase().contains(&q)
                    || t.id.0.to_lowercase().contains(&q)
                    || t.tool_type.to_string().to_lowercase().contains(&q)
                    || dia.contains(&q)
            });
        }

        if let Some(type_id) = self.type_filter.active_id() {
            if type_id.as_str() != "all" {
                tools.retain(|tool| {
                    let tool_type_str = match tool.tool_type {
                        ToolType::EndMillFlat => "endmill_flat",
                        ToolType::EndMillBall => "endmill_ball",
                        ToolType::EndMillCornerRadius => "endmill_cr",
                        ToolType::VBit => "vbit",
                        ToolType::DrillBit => "drill",
                        ToolType::SpotDrill => "spot",
                        ToolType::EngravingBit => "engraving",
                        ToolType::ChamferTool => "chamfer",
                        ToolType::Specialty => "specialty",
                    };
                    tool_type_str == type_id.as_str()
                });
            }
        }

        if let Some(mat_id) = self.material_filter.active_id() {
            if mat_id.as_str() != "all" {
                tools.retain(|tool| tool.material.to_string() == mat_id.as_str());
            }
        }

        let dia_min_txt = self.dia_min.text().to_string();
        let dia_max_txt = self.dia_max.text().to_string();
        let dia_min = dia_min_txt.parse::<f32>().ok();
        let dia_max = dia_max_txt.parse::<f32>().ok();

        if !dia_min_txt.trim().is_empty() && dia_min.is_none() {
            self.dia_min.add_css_class("error");
        } else {
            self.dia_min.remove_css_class("error");
        }
        if !dia_max_txt.trim().is_empty() && dia_max.is_none() {
            self.dia_max.add_css_class("error");
        } else {
            self.dia_max.remove_css_class("error");
        }

        if let Some(min) = dia_min {
            tools.retain(|t| t.diameter >= min);
        }
        if let Some(max) = dia_max {
            tools.retain(|t| t.diameter <= max);
        }

        tools.sort_by(|a, b| a.id.0.cmp(&b.id.0).then(a.name.cmp(&b.name)));

        for tool in tools {
            let row = ListBoxRow::new();
            unsafe {
                row.set_data(ROW_TOOL_ID_KEY, tool.id.0.clone());
            }

            let row_box = Box::new(Orientation::Vertical, 4);
            row_box.set_margin_top(6);
            row_box.set_margin_bottom(6);
            row_box.set_margin_start(10);
            row_box.set_margin_end(10);

            let name_label = Label::new(Some(&tool.name));
            name_label.add_css_class("title-4");
            name_label.set_halign(Align::Start);
            name_label.set_xalign(0.0);
            name_label.set_wrap(true);
            name_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            name_label.set_max_width_chars(36);
            row_box.append(&name_label);

            let coating = tool
                .coating
                .map(|c| c.to_string())
                .unwrap_or("None".to_string());
            let shaft = tool.shaft_diameter.unwrap_or(tool.diameter);
            let info = format!(
                "{} • id:{} • Ø{:.3}mm • shaft Ø{:.3}mm • {}F • FL{:.3}mm • {} / {}",
                tool.tool_type,
                tool.id.0,
                tool.diameter,
                shaft,
                tool.flutes,
                tool.flute_length,
                tool.material,
                coating
            );
            let info_label = Label::new(Some(&info));
            info_label.add_css_class("dim-label");
            info_label.set_halign(Align::Start);
            info_label.set_xalign(0.0);
            info_label.set_wrap(true);
            info_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            info_label.set_max_width_chars(44);
            row_box.append(&info_label);

            row.set_child(Some(&row_box));
            self.tools_list.append(&row);
        }

        // Restore selection if possible.
        if self.right_panel_stack.visible_child_name().as_deref() != Some("edit") {
            // IMPORTANT: don't hold a RefCell borrow across select_row(), since that can emit
            // row-selected signals synchronously.
            let selected_tool_id = { self.last_selected_tool_id.borrow().clone() };
            if let Some(tool_id) = selected_tool_id {
                self.select_row_by_tool_id(&self.tools_list, &tool_id);
            }
        }
    }

    fn start_create_new(&self) {
        if self.is_dirty() {
            let view = self.clone();
            self.show_discard_dialog(move || {
                view.start_create_new_inner();
            });
            return;
        }

        self.start_create_new_inner();
    }

    fn start_create_new_inner(&self) {
        *self.is_creating.borrow_mut() = true;
        *self.selected_tool.borrow_mut() = None;

        self.clear_form();
        self.edit_id.set_sensitive(true);
        self.right_panel_stack.set_visible_child_name("edit");
        self.cancel_btn.set_sensitive(true);
        self.delete_btn.set_sensitive(false);
        self.update_type_dependent_fields();
        self.update_save_state();
    }

    fn load_tool_for_edit(&self, tool_id: &str) {
        let backend = self.backend.borrow();
        if let Some(tool) = backend.get_tool(&ToolId(tool_id.to_string())) {
            *self.is_creating.borrow_mut() = false;
            *self.selected_tool.borrow_mut() = Some(tool.clone());
            if let Ok(mut last) = self.last_selected_tool_id.try_borrow_mut() {
                *last = Some(tool.id.0.clone());
            } else {
                tracing::warn!("last_selected_tool_id already borrowed; skipping update");
            }

            self.edit_id.set_text(&tool.id.0);
            self.edit_id.set_sensitive(false);
            self.edit_number.set_value(tool.number as f64);
            self.edit_name.set_text(&tool.name);

            match tool.tool_type {
                ToolType::EndMillFlat => self.edit_tool_type.set_active(Some(0)),
                ToolType::EndMillBall => self.edit_tool_type.set_active(Some(1)),
                ToolType::EndMillCornerRadius => self.edit_tool_type.set_active(Some(2)),
                ToolType::VBit => self.edit_tool_type.set_active(Some(3)),
                ToolType::DrillBit => self.edit_tool_type.set_active(Some(4)),
                ToolType::SpotDrill => self.edit_tool_type.set_active(Some(5)),
                ToolType::EngravingBit => self.edit_tool_type.set_active(Some(6)),
                ToolType::ChamferTool => self.edit_tool_type.set_active(Some(7)),
                ToolType::Specialty => self.edit_tool_type.set_active(Some(8)),
            }

            self.edit_material.set_active(match tool.material {
                ToolMaterial::HSS => Some(0),
                ToolMaterial::Carbide => Some(1),
                ToolMaterial::CoatedCarbide => Some(2),
                ToolMaterial::Diamond => Some(3),
            });

            self.edit_coating.set_active(match tool.coating {
                None => Some(0),
                Some(ToolCoating::TiN) => Some(1),
                Some(ToolCoating::TiAlN) => Some(2),
                Some(ToolCoating::DLC) => Some(3),
                Some(ToolCoating::AlOx) => Some(4),
            });

            self.edit_shank.set_active(match tool.shank {
                ShankType::Straight(_) => Some(0),
                ShankType::Collet => Some(1),
                ShankType::Tapered => Some(2),
            });

            self.edit_diameter
                .set_text(&format!("{:.3}", tool.diameter));
            self.edit_length.set_text(&format!("{:.3}", tool.length));
            self.edit_flute_length
                .set_text(&format!("{:.3}", tool.flute_length));

            let shaft = tool.shaft_diameter.unwrap_or(tool.diameter);
            self.edit_shaft_diameter.set_text(&format!("{shaft:.3}"));
            self.edit_flutes.set_value(tool.flutes as f64);

            self.edit_corner_radius.set_text(
                &tool
                    .corner_radius
                    .map(|v| format!("{v:.3}"))
                    .unwrap_or_default(),
            );
            self.edit_tip_angle.set_text(
                &tool
                    .tip_angle
                    .map(|v| format!("{v:.3}"))
                    .unwrap_or_default(),
            );

            if let Some(ref manufacturer) = tool.manufacturer {
                self.edit_manufacturer.set_text(manufacturer);
            } else {
                self.edit_manufacturer.set_text("");
            }

            if let Some(ref part_number) = tool.part_number {
                self.edit_part_number.set_text(part_number);
            } else {
                self.edit_part_number.set_text("");
            }

            self.edit_description.buffer().set_text(&tool.description);
            self.edit_notes.buffer().set_text(&tool.notes);

            self.right_panel_stack.set_visible_child_name("edit");
            self.cancel_btn.set_sensitive(true);
            self.delete_btn.set_sensitive(true);

            self.update_type_dependent_fields();
            self.update_save_state();
        }
    }

    fn update_type_dependent_fields(&self) {
        let tt = match self.edit_tool_type.active() {
            Some(0) => ToolType::EndMillFlat,
            Some(1) => ToolType::EndMillBall,
            Some(2) => ToolType::EndMillCornerRadius,
            Some(3) => ToolType::VBit,
            Some(4) => ToolType::DrillBit,
            Some(5) => ToolType::SpotDrill,
            Some(6) => ToolType::EngravingBit,
            Some(7) => ToolType::ChamferTool,
            Some(8) => ToolType::Specialty,
            _ => ToolType::EndMillFlat,
        };

        let show_corner_radius = tt == ToolType::EndMillCornerRadius;
        let show_tip_angle = matches!(
            tt,
            ToolType::VBit | ToolType::DrillBit | ToolType::SpotDrill
        );

        self.edit_corner_radius.set_sensitive(show_corner_radius);
        self.edit_tip_angle.set_sensitive(show_tip_angle);

        if !show_corner_radius {
            self.edit_corner_radius.set_text("");
        }
        if !show_tip_angle {
            self.edit_tip_angle.set_text("");
        }
    }

    fn tool_type_from_active(active: Option<u32>) -> ToolType {
        match active {
            Some(0) => ToolType::EndMillFlat,
            Some(1) => ToolType::EndMillBall,
            Some(2) => ToolType::EndMillCornerRadius,
            Some(3) => ToolType::VBit,
            Some(4) => ToolType::DrillBit,
            Some(5) => ToolType::SpotDrill,
            Some(6) => ToolType::EngravingBit,
            Some(7) => ToolType::ChamferTool,
            Some(8) => ToolType::Specialty,
            _ => ToolType::EndMillFlat,
        }
    }

    fn coating_from_combo(text: &str) -> Option<ToolCoating> {
        match text {
            "TiN" => Some(ToolCoating::TiN),
            "TiAlN" => Some(ToolCoating::TiAlN),
            "DLC" => Some(ToolCoating::DLC),
            "AlOx" => Some(ToolCoating::AlOx),
            _ => None,
        }
    }

    fn shank_from_combo(idx: Option<u32>, shaft_diameter_mm: f32) -> ShankType {
        match idx {
            Some(1) => ShankType::Collet,
            Some(2) => ShankType::Tapered,
            _ => ShankType::Straight((shaft_diameter_mm * 10.0) as u32),
        }
    }

    fn read_text_view(tv: &TextView) -> String {
        let buffer = tv.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        buffer.text(&start, &end, true).to_string()
    }

    fn build_tool_from_form(&self) -> Result<Tool, String> {
        let tool_id = self.edit_id.text().trim().to_string();
        if tool_id.is_empty() {
            return Err("Tool ID is required".to_string());
        }

        let tool_name = self.edit_name.text().trim().to_string();
        if tool_name.is_empty() {
            return Err("Tool name is required".to_string());
        }

        let tool_type = Self::tool_type_from_active(self.edit_tool_type.active());

        let material_text = self
            .edit_material
            .active_text()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Carbide".to_string());
        let tool_material = string_to_tool_material(&material_text)
            .ok_or_else(|| "Invalid tool material".to_string())?;

        let diameter = self
            .edit_diameter
            .text()
            .trim()
            .parse::<f32>()
            .map_err(|_| "Invalid diameter".to_string())?;
        let length = self
            .edit_length
            .text()
            .trim()
            .parse::<f32>()
            .map_err(|_| "Invalid length".to_string())?;
        let flute_length = self
            .edit_flute_length
            .text()
            .trim()
            .parse::<f32>()
            .map_err(|_| "Invalid flute length".to_string())?;
        let shaft_diameter_mm = self
            .edit_shaft_diameter
            .text()
            .trim()
            .parse::<f32>()
            .map_err(|_| "Invalid shaft diameter".to_string())?;

        let flutes = self.edit_flutes.value() as u32;

        let corner_radius = if self.edit_corner_radius.is_sensitive() {
            let txt = self.edit_corner_radius.text().trim().to_string();
            if txt.is_empty() {
                None
            } else {
                Some(
                    txt.parse::<f32>()
                        .map_err(|_| "Invalid corner radius".to_string())?,
                )
            }
        } else {
            None
        };

        let tip_angle = if self.edit_tip_angle.is_sensitive() {
            let txt = self.edit_tip_angle.text().trim().to_string();
            if txt.is_empty() {
                None
            } else {
                Some(
                    txt.parse::<f32>()
                        .map_err(|_| "Invalid tip angle".to_string())?,
                )
            }
        } else {
            None
        };

        let coating = self
            .edit_coating
            .active_text()
            .map(|s| s.to_string())
            .and_then(|t| Self::coating_from_combo(&t));

        let shank = Self::shank_from_combo(self.edit_shank.active(), shaft_diameter_mm);

        let manufacturer = {
            let text = self.edit_manufacturer.text().trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        };

        let part_number = {
            let text = self.edit_part_number.text().trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        };

        let description = Self::read_text_view(&self.edit_description);
        let notes = Self::read_text_view(&self.edit_notes);

        let tool_number = self.edit_number.value() as u32;

        let mut tool = Tool::new(
            ToolId(tool_id),
            tool_number,
            tool_name,
            tool_type,
            diameter,
            length,
        );

        tool.custom = true;
        tool.description = description;
        tool.material = tool_material;
        tool.coating = coating;
        tool.shank = shank;
        tool.flute_length = flute_length;
        tool.shaft_diameter = Some(shaft_diameter_mm);
        tool.flutes = flutes;
        tool.corner_radius = corner_radius;
        tool.tip_angle = tip_angle;
        tool.manufacturer = manufacturer;
        tool.part_number = part_number;
        tool.notes = notes;

        Ok(tool)
    }

    fn tool_contents_equal(a: &Tool, b: &Tool) -> bool {
        // Helper for float comparison with tolerance matching UI precision (3 decimals)
        let eq_f32 = |x: f32, y: f32| (x - y).abs() < 0.0001;

        // Helper for Option<f32>
        let eq_opt_f32 = |x: Option<f32>, y: Option<f32>| match (x, y) {
            (Some(vx), Some(vy)) => eq_f32(vx, vy),
            (None, None) => true,
            _ => false,
        };

        // Helper for shaft diameter (None implies == diameter)
        let eq_shaft = |t: &Tool| t.shaft_diameter.unwrap_or(t.diameter);

        // Helper for strings (treat None and Some("") as equal)
        let eq_str = |x: &Option<String>, y: &Option<String>| {
            x.as_deref().unwrap_or("").trim() == y.as_deref().unwrap_or("").trim()
        };

        a.id.0 == b.id.0
            && a.name == b.name
            && a.description == b.description
            && a.tool_type == b.tool_type
            && eq_f32(a.diameter, b.diameter)
            && eq_f32(a.length, b.length)
            && eq_f32(a.flute_length, b.flute_length)
            && a.flutes == b.flutes
            && eq_opt_f32(a.corner_radius, b.corner_radius)
            && eq_opt_f32(a.tip_angle, b.tip_angle)
            && eq_f32(eq_shaft(a), eq_shaft(b))
            && a.material == b.material
            && a.coating == b.coating
            && a.shank == b.shank
            && eq_str(&a.manufacturer, &b.manufacturer)
            && eq_str(&a.part_number, &b.part_number)
            && a.notes == b.notes
    }

    fn is_dirty(&self) -> bool {
        if self.right_panel_stack.visible_child_name().as_deref() != Some("edit") {
            return false;
        }

        let current = match self.build_tool_from_form() {
            Ok(t) => t,
            Err(_) => return true,
        };

        if *self.is_creating.borrow() {
            return true;
        }

        let Some(orig) = self.selected_tool.borrow().clone() else {
            return true;
        };

        !Self::tool_contents_equal(&orig, &current)
    }

    fn show_discard_dialog<F: Fn() + 'static>(&self, on_yes: F) {
        let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() else {
            return;
        };

        let dialog = gtk4::MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(gtk4::MessageType::Question)
            .buttons(gtk4::ButtonsType::YesNo)
            .text("Discard changes?")
            .secondary_text("You have unsaved changes. Discard them?")
            .build();

        dialog.connect_response(move |d, resp| {
            if resp == gtk4::ResponseType::Yes {
                on_yes();
            }
            d.close();
        });

        dialog.show();
    }

    fn update_save_state(&self) {
        // Clear previous error
        self.error_label.set_visible(false);
        self.error_label.set_text("");

        let visible_edit = self.right_panel_stack.visible_child_name().as_deref() == Some("edit");
        if !visible_edit {
            self.save_btn.set_sensitive(false);
            return;
        }

        let dirty = self.is_dirty();
        let valid = self.build_tool_from_form().is_ok();

        self.save_btn.set_sensitive(dirty && valid);
    }

    fn cancel_edit_with_confirm(&self) {
        if self.is_dirty() {
            let view = self.clone();
            self.show_discard_dialog(move || {
                view.cancel_edit();
            });
            return;
        }
        self.cancel_edit();
    }

    fn save_tool(&self) {
        self.error_label.set_visible(false);
        self.error_label.set_text("");

        let tool = match self.build_tool_from_form() {
            Ok(t) => t,
            Err(e) => {
                self.error_label.set_text(&e);
                self.error_label.set_visible(true);
                return;
            }
        };

        // Validate uniqueness on create
        if *self.is_creating.borrow() {
            let backend = self.backend.borrow();
            if backend.get_tool(&tool.id).is_some() {
                self.error_label
                    .set_text("Tool ID already exists. Please choose a unique ID.");
                self.error_label.set_visible(true);
                return;
            }
        }

        let mut backend = self.backend.borrow_mut();

        if *self.is_creating.borrow() {
            backend.add_tool(tool.clone());
        } else {
            backend.remove_tool(&tool.id);
            backend.add_tool(tool.clone());
        }

        drop(backend);

        *self.last_selected_tool_id.borrow_mut() = Some(tool.id.0.clone());
        self.persist_ui_state();

        self.load_tools();
        self.load_tool_for_edit(&tool.id.0);
        self.update_save_state();
    }

    fn delete_tool(&self) {
        if let Some(ref tool) = *self.selected_tool.borrow() {
            let tool_id = tool.id.clone();

            if let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() {
                let dialog = gtk4::MessageDialog::builder()
                    .transient_for(&window)
                    .modal(true)
                    .message_type(gtk4::MessageType::Warning)
                    .buttons(gtk4::ButtonsType::YesNo)
                    .text("Delete Tool?")
                    .secondary_text(format!(
                        "Are you sure you want to delete tool '{}' (id: {})?\n\nThis action cannot be undone.",
                        tool.name, tool.id.0
                    ))
                    .build();

                let backend = self.backend.clone();
                let view = self.clone();

                dialog.connect_response(move |d, response| {
                    if response == gtk4::ResponseType::Yes {
                        let mut backend_mut = backend.borrow_mut();
                        backend_mut.remove_tool(&tool_id);
                        drop(backend_mut);

                        view.load_tools();
                        view.cancel_edit();
                    }
                    d.close();
                });

                dialog.show();
            }
        }
    }

    fn cancel_edit(&self) {
        *self.is_creating.borrow_mut() = false;
        *self.selected_tool.borrow_mut() = None;
        self.clear_form();
        self.right_panel_stack.set_visible_child_name("empty");
        self.save_btn.set_sensitive(false);
        self.cancel_btn.set_sensitive(false);
        self.delete_btn.set_sensitive(false);
    }

    fn clear_form(&self) {
        self.edit_id.set_text("");
        self.edit_number.set_value(1.0);
        self.edit_name.set_text("");
        self.edit_tool_type.set_active(Some(0));
        self.edit_material.set_active(Some(1));
        self.edit_coating.set_active(Some(0));
        self.edit_shank.set_active(Some(0));

        self.edit_diameter.set_text("6.350");
        self.edit_length.set_text("50.000");
        self.edit_flute_length.set_text("20.000");
        self.edit_shaft_diameter.set_text("6.350");
        self.edit_flutes.set_value(2.0);
        self.edit_corner_radius.set_text("");
        self.edit_tip_angle.set_text("");

        self.edit_manufacturer.set_text("");
        self.edit_part_number.set_text("");
        self.edit_description.buffer().set_text("");
        self.edit_notes.buffer().set_text("");

        self.error_label.set_text("");
        self.error_label.set_visible(false);

        self.update_type_dependent_fields();
    }

    fn import_gtc_zip(&self) {
        let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() else {
            return;
        };

        let dialog = super::file_dialog::open_dialog("Import GTC Package", Some(&window));

        let backend = self.backend.clone();
        let view = self.clone();

        dialog.connect_response(move |dialog, resp| {
            if resp == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let mut backend = backend.borrow_mut();
                        match backend.import_gtc_package(path) {
                            Ok(result) => {
                                view.load_tools();
                                view.show_info_dialog(
                                    "Import Complete",
                                    &format!(
                                        "Imported: {}\nSkipped: {}\nErrors: {}",
                                        result.imported_tools.len(),
                                        result.skipped_tools,
                                        result.errors.len()
                                    ),
                                );
                            }
                            Err(e) => {
                                view.show_error_dialog("Import Failed", &e.to_string());
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    fn import_gtc_json(&self) {
        let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() else {
            return;
        };

        let dialog = super::file_dialog::open_dialog("Import GTC JSON", Some(&window));

        let backend = self.backend.clone();
        let view = self.clone();

        dialog.connect_response(move |dialog, resp| {
            if resp == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let mut backend = backend.borrow_mut();
                        match backend.import_gtc_json(path) {
                            Ok(result) => {
                                view.load_tools();
                                view.show_info_dialog(
                                    "Import Complete",
                                    &format!(
                                        "Imported: {}\nSkipped: {}\nErrors: {}",
                                        result.imported_tools.len(),
                                        result.skipped_tools,
                                        result.errors.len()
                                    ),
                                );
                            }
                            Err(e) => {
                                view.show_error_dialog("Import Failed", &e.to_string());
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    fn export_custom_tools(&self) {
        let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() else {
            return;
        };

        let dialog = super::file_dialog::save_dialog("Export Custom Tools", Some(&window));

        let backend = self.backend.clone();
        let view = self.clone();

        dialog.connect_response(move |dialog, resp| {
            if resp == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let backend = backend.borrow();
                        match backend.export_custom_tools(path) {
                            Ok(_) => {
                                view.show_info_dialog("Export Complete", "Custom tools exported.")
                            }
                            Err(e) => view.show_error_dialog("Export Failed", &e.to_string()),
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    fn reset_custom_tools(&self) {
        let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() else {
            return;
        };

        let dialog = gtk4::MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(gtk4::MessageType::Warning)
            .buttons(gtk4::ButtonsType::YesNo)
            .text("Reset custom tools?")
            .secondary_text(
                "This will delete all custom/imported tools and remove local custom tools storage.\n\nContinue?",
            )
            .build();

        let backend = self.backend.clone();
        let view = self.clone();

        dialog.connect_response(move |d, resp| {
            if resp == gtk4::ResponseType::Yes {
                let mut backend = backend.borrow_mut();
                match backend.reset_custom_tools() {
                    Ok(_) => {
                        view.cancel_edit();
                        view.load_tools();
                        view.show_info_dialog("Reset Complete", "Custom tools removed.");
                    }
                    Err(e) => view.show_error_dialog("Reset Failed", &e.to_string()),
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn show_info_dialog(&self, title: &str, message: &str) {
        if let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() {
            let dialog = gtk4::MessageDialog::builder()
                .transient_for(&window)
                .modal(true)
                .message_type(gtk4::MessageType::Info)
                .buttons(gtk4::ButtonsType::Ok)
                .text(title)
                .secondary_text(message)
                .build();
            dialog.connect_response(|d, _| d.close());
            dialog.show();
        }
    }

    fn show_error_dialog(&self, title: &str, message: &str) {
        let parent = crate::ui::gtk::file_dialog::parent_window(&self.widget);
        crate::ui::gtk::file_dialog::show_error_dialog(title, message, parent.as_ref());
    }
}
