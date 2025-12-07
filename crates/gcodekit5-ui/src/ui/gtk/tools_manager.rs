use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, ComboBoxText, Entry, Frame, Grid, Label, ListBox,
    Orientation, Paned, PolicyType, ScrolledWindow, SearchEntry, SpinButton, Stack,
    StackSwitcher, TextView, WrapMode,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::tools_manager_backend::ToolsManagerBackend;
use gcodekit5_core::data::tools::{Tool, ToolId, ToolType};

#[derive(Clone)]
pub struct ToolsManagerView {
    pub widget: Paned,
    backend: Rc<RefCell<ToolsManagerBackend>>,
    tools_list: ListBox,
    search_entry: SearchEntry,
    type_filter: ComboBoxText,
    right_panel_stack: Stack,
    
    // Edit form widgets
    edit_id: Entry,
    edit_number: SpinButton,
    edit_name: Entry,
    edit_tool_type: ComboBoxText,
    edit_material: ComboBoxText,
    edit_diameter: Entry,
    edit_length: Entry,
    edit_flute_length: Entry,
    edit_shaft_diameter: Entry,
    edit_flutes: SpinButton,
    edit_coating: ComboBoxText,
    edit_manufacturer: Entry,
    edit_part_number: Entry,
    edit_description: TextView,
    edit_notes: TextView,
    
    // State
    selected_tool: Rc<RefCell<Option<Tool>>>,
    is_creating: Rc<RefCell<bool>>,
    
    // Action buttons
    save_btn: Button,
    cancel_btn: Button,
    delete_btn: Button,
    new_btn: Button,
}

impl ToolsManagerView {
    pub fn new() -> Rc<Self> {
        let backend = Rc::new(RefCell::new(ToolsManagerBackend::new()));
        
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        // LEFT SIDEBAR - Tools List
        let sidebar = Box::new(Orientation::Vertical, 10);
        sidebar.add_css_class("sidebar");
        sidebar.set_width_request(250);
        sidebar.set_margin_top(10);
        sidebar.set_margin_bottom(10);
        sidebar.set_margin_start(10);
        sidebar.set_margin_end(10);

        // Header
        let header_box = Box::new(Orientation::Horizontal, 10);
        header_box.set_margin_start(5);
        let title = Label::new(Some("CNC Tools"));
        title.add_css_class("title-4");
        title.set_halign(Align::Start);
        header_box.append(&title);
        sidebar.append(&header_box);

        // Search
        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search tools..."));
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
        type_filter.set_active(Some(0));
        sidebar.append(&type_filter);

        // Tools list
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);

        let tools_list = ListBox::new();
        tools_list.set_activate_on_single_click(true);
        tools_list.add_css_class("boxed-list");
        scroll.set_child(Some(&tools_list));
        sidebar.append(&scroll);

        // New tool button
        let new_btn = Button::with_label("➕ New Tool");
        new_btn.add_css_class("suggested-action");
        sidebar.append(&new_btn);

        widget.set_start_child(Some(&sidebar));

        // RIGHT PANEL - Tool Details/Edit Form
        let right_panel_stack = Stack::new();
        right_panel_stack.set_hexpand(true);
        right_panel_stack.set_vexpand(true);
        
        // Empty state
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

        // Action buttons bar
        let action_bar = Box::new(Orientation::Horizontal, 10);
        let save_btn = Button::with_label("💾 Save");
        save_btn.add_css_class("suggested-action");
        save_btn.set_sensitive(false);
        let cancel_btn = Button::with_label("❌ Cancel");
        cancel_btn.set_sensitive(false);
        let delete_btn = Button::with_label("🗑️ Delete");
        delete_btn.add_css_class("destructive-action");
        delete_btn.set_sensitive(false);
        
        action_bar.append(&save_btn);
        action_bar.append(&cancel_btn);
        action_bar.append(&delete_btn);
        
        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        action_bar.append(&spacer);
        
        main_content.append(&action_bar);

        // Stack with tabs
        let stack = Stack::new();
        stack.set_vexpand(true);

        // Create tab pages
        let (basic_page, edit_id, edit_number, edit_name, edit_tool_type, edit_material) =
            Self::create_basic_tab();
        let (geometry_page, edit_diameter, edit_length, edit_flute_length, edit_shaft_diameter, edit_flutes) =
            Self::create_geometry_tab();
        let (mfg_page, edit_coating, edit_manufacturer, edit_part_number, edit_description) =
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

        widget.set_end_child(Some(&right_panel_stack));

        // Set initial position
        widget.add_tick_callback(|paned, _clock| {
            let width = paned.width();
            let target = (width as f64 * 0.2) as i32;
            if (paned.position() - target).abs() > 2 {
                paned.set_position(target);
            }
            gtk4::glib::ControlFlow::Continue
        });

        let view = Rc::new(Self {
            widget,
            backend: backend.clone(),
            tools_list,
            search_entry,
            type_filter,
            right_panel_stack: right_panel_stack.clone(),
            edit_id,
            edit_number,
            edit_name,
            edit_tool_type,
            edit_material,
            edit_diameter,
            edit_length,
            edit_flute_length,
            edit_shaft_diameter,
            edit_flutes,
            edit_coating,
            edit_manufacturer,
            edit_part_number,
            edit_description,
            edit_notes,
            selected_tool: Rc::new(RefCell::new(None)),
            is_creating: Rc::new(RefCell::new(false)),
            save_btn,
            cancel_btn,
            delete_btn,
            new_btn,
        });

        view.setup_event_handlers();
        view.load_tools();

        view
    }

    fn create_basic_tab() -> (ScrolledWindow, Entry, SpinButton, Entry, ComboBoxText, ComboBoxText) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let mut row = 0;

        // ID
        let id_label = Label::new(Some("ID:"));
        id_label.set_halign(Align::Start);
        let edit_id = Entry::new();
        edit_id.set_placeholder_text(Some("tool_id"));
        grid.attach(&id_label, 0, row, 1, 1);
        grid.attach(&edit_id, 1, row, 1, 1);
        row += 1;

        // Tool Number
        let num_label = Label::new(Some("Tool Number:"));
        num_label.set_halign(Align::Start);
        let edit_number = SpinButton::with_range(1.0, 999.0, 1.0);
        edit_number.set_value(1.0);
        grid.attach(&num_label, 0, row, 1, 1);
        grid.attach(&edit_number, 1, row, 1, 1);
        row += 1;

        // Name
        let name_label = Label::new(Some("Name:"));
        name_label.set_halign(Align::Start);
        let edit_name = Entry::new();
        edit_name.set_hexpand(true);
        grid.attach(&name_label, 0, row, 1, 1);
        grid.attach(&edit_name, 1, row, 1, 1);
        row += 1;

        // Tool Type
        let type_label = Label::new(Some("Tool Type:"));
        type_label.set_halign(Align::Start);
        let edit_tool_type = ComboBoxText::new();
        edit_tool_type.append_text("Flat End Mill");
        edit_tool_type.append_text("Ball End Mill");
        edit_tool_type.append_text("Corner Radius End Mill");
        edit_tool_type.append_text("V-Bit");
        edit_tool_type.append_text("Drill Bit");
        edit_tool_type.append_text("Spot Drill");
        edit_tool_type.append_text("Engraving Bit");
        edit_tool_type.append_text("Chamfer Tool");
        edit_tool_type.append_text("Specialty");
        edit_tool_type.set_active(Some(0));
        grid.attach(&type_label, 0, row, 1, 1);
        grid.attach(&edit_tool_type, 1, row, 1, 1);
        row += 1;

        // Material
        let mat_label = Label::new(Some("Material:"));
        mat_label.set_halign(Align::Start);
        let edit_material = ComboBoxText::new();
        edit_material.append_text("HSS");
        edit_material.append_text("Carbide");
        edit_material.append_text("Coated Carbide");
        edit_material.append_text("Diamond");
        edit_material.set_active(Some(1));
        grid.attach(&mat_label, 0, row, 1, 1);
        grid.attach(&edit_material, 1, row, 1, 1);

        scroll.set_child(Some(&grid));
        (scroll, edit_id, edit_number, edit_name, edit_tool_type, edit_material)
    }

    fn create_geometry_tab() -> (ScrolledWindow, Entry, Entry, Entry, Entry, SpinButton) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let mut row = 0;

        // Diameter
        let dia_label = Label::new(Some("Diameter (mm):"));
        dia_label.set_halign(Align::Start);
        let edit_diameter = Entry::new();
        edit_diameter.set_text("6.35");
        grid.attach(&dia_label, 0, row, 1, 1);
        grid.attach(&edit_diameter, 1, row, 1, 1);
        row += 1;

        // Length
        let len_label = Label::new(Some("Length (mm):"));
        len_label.set_halign(Align::Start);
        let edit_length = Entry::new();
        edit_length.set_text("50.0");
        grid.attach(&len_label, 0, row, 1, 1);
        grid.attach(&edit_length, 1, row, 1, 1);
        row += 1;

        // Flute Length
        let flute_label = Label::new(Some("Flute Length (mm):"));
        flute_label.set_halign(Align::Start);
        let edit_flute_length = Entry::new();
        edit_flute_length.set_text("20.0");
        grid.attach(&flute_label, 0, row, 1, 1);
        grid.attach(&edit_flute_length, 1, row, 1, 1);
        row += 1;

        // Shaft Diameter
        let shaft_label = Label::new(Some("Shaft Diameter (mm):"));
        shaft_label.set_halign(Align::Start);
        let edit_shaft_diameter = Entry::new();
        edit_shaft_diameter.set_text("6.35");
        grid.attach(&shaft_label, 0, row, 1, 1);
        grid.attach(&edit_shaft_diameter, 1, row, 1, 1);
        row += 1;

        // Flutes
        let flutes_label = Label::new(Some("Number of Flutes:"));
        flutes_label.set_halign(Align::Start);
        let edit_flutes = SpinButton::with_range(1.0, 8.0, 1.0);
        edit_flutes.set_value(2.0);
        grid.attach(&flutes_label, 0, row, 1, 1);
        grid.attach(&edit_flutes, 1, row, 1, 1);

        scroll.set_child(Some(&grid));
        (scroll, edit_diameter, edit_length, edit_flute_length, edit_shaft_diameter, edit_flutes)
    }

    fn create_manufacturer_tab() -> (ScrolledWindow, ComboBoxText, Entry, Entry, TextView) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let vbox = Box::new(Orientation::Vertical, 10);
        vbox.set_margin_top(10);
        vbox.set_margin_bottom(10);
        vbox.set_margin_start(10);
        vbox.set_margin_end(10);

        // Coating
        let coating_grid = Grid::new();
        coating_grid.set_column_spacing(10);
        let coating_label = Label::new(Some("Coating:"));
        coating_label.set_halign(Align::Start);
        let edit_coating = ComboBoxText::new();
        edit_coating.append_text("None");
        edit_coating.append_text("TiN");
        edit_coating.append_text("TiAlN");
        edit_coating.append_text("DLC");
        edit_coating.append_text("AlOx");
        edit_coating.set_active(Some(0));
        coating_grid.attach(&coating_label, 0, 0, 1, 1);
        coating_grid.attach(&edit_coating, 1, 0, 1, 1);
        vbox.append(&coating_grid);

        // Manufacturer
        let mfg_grid = Grid::new();
        mfg_grid.set_column_spacing(10);
        let mfg_label = Label::new(Some("Manufacturer:"));
        mfg_label.set_halign(Align::Start);
        let edit_manufacturer = Entry::new();
        edit_manufacturer.set_hexpand(true);
        mfg_grid.attach(&mfg_label, 0, 0, 1, 1);
        mfg_grid.attach(&edit_manufacturer, 1, 0, 1, 1);
        vbox.append(&mfg_grid);

        // Part Number
        let pn_grid = Grid::new();
        pn_grid.set_column_spacing(10);
        let pn_label = Label::new(Some("Part Number:"));
        pn_label.set_halign(Align::Start);
        let edit_part_number = Entry::new();
        edit_part_number.set_hexpand(true);
        pn_grid.attach(&pn_label, 0, 0, 1, 1);
        pn_grid.attach(&edit_part_number, 1, 0, 1, 1);
        vbox.append(&pn_grid);

        // Description
        let desc_frame = Frame::new(Some("Description"));
        let edit_description = TextView::new();
        edit_description.set_wrap_mode(WrapMode::Word);
        edit_description.set_height_request(80);
        let desc_scroll = ScrolledWindow::new();
        desc_scroll.set_child(Some(&edit_description));
        desc_frame.set_child(Some(&desc_scroll));
        vbox.append(&desc_frame);

        scroll.set_child(Some(&vbox));
        (scroll, edit_coating, edit_manufacturer, edit_part_number, edit_description)
    }

    fn create_notes_tab() -> (ScrolledWindow, TextView) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);

        let vbox = Box::new(Orientation::Vertical, 10);
        vbox.set_margin_top(10);
        vbox.set_margin_bottom(10);
        vbox.set_margin_start(10);
        vbox.set_margin_end(10);

        let label = Label::new(Some("Additional Notes:"));
        label.set_halign(Align::Start);
        vbox.append(&label);

        let edit_notes = TextView::new();
        edit_notes.set_wrap_mode(WrapMode::Word);
        edit_notes.set_vexpand(true);
        vbox.append(&edit_notes);

        scroll.set_child(Some(&vbox));
        (scroll, edit_notes)
    }

    fn setup_event_handlers(self: &Rc<Self>) {
        // New tool button
        let view = self.clone();
        self.new_btn.connect_clicked(move |_| {
            view.start_create_new();
        });

        // Save button
        let view = self.clone();
        self.save_btn.connect_clicked(move |_| {
            view.save_tool();
        });

        // Cancel button
        let view = self.clone();
        self.cancel_btn.connect_clicked(move |_| {
            view.cancel_edit();
        });

        // Delete button
        let view = self.clone();
        self.delete_btn.connect_clicked(move |_| {
            view.delete_tool();
        });

        // Search
        let view = self.clone();
        self.search_entry.connect_search_changed(move |_| {
            view.load_tools();
        });

        // Type filter
        let view = self.clone();
        self.type_filter.connect_changed(move |_| {
            view.load_tools();
        });

        // List selection
        let view = self.clone();
        self.tools_list.connect_row_activated(move |_, row| {
            if let Some(row_box) = row.child().and_then(|w| w.downcast::<Box>().ok()) {
                let mut child = row_box.first_child();
                let mut id_label: Option<Label> = None;
                
                while let Some(widget) = child.clone() {
                    if let Ok(label) = widget.clone().downcast::<Label>() {
                        if !label.is_visible() {
                            id_label = Some(label.clone());
                            break;
                        }
                    }
                    child = widget.next_sibling();
                }
                
                if let Some(label) = id_label {
                    let tool_id = label.label().to_string();
                    view.load_tool_for_edit(&tool_id);
                }
            }
        });
    }

    fn load_tools(&self) {
        // Clear list
        while let Some(child) = self.tools_list.first_child() {
            self.tools_list.remove(&child);
        }

        let backend = self.backend.borrow();
        let search_query = self.search_entry.text().to_string();
        let mut tools = backend.search_tools(&search_query);
        
        // Apply type filter
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

        for tool in tools {
            let row_box = Box::new(Orientation::Vertical, 5);
            row_box.set_margin_top(5);
            row_box.set_margin_bottom(5);
            row_box.set_margin_start(10);
            row_box.set_margin_end(10);

            let name_label = Label::new(Some(&format!("#{} {}", tool.number, tool.name)));
            name_label.add_css_class("title-4");
            name_label.set_halign(Align::Start);
            name_label.set_xalign(0.0);
            name_label.set_wrap(true);
            name_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            name_label.set_max_width_chars(30);
            row_box.append(&name_label);

            let info = format!("{} - Ø{:.2}mm", tool.tool_type, tool.diameter);
            let info_label = Label::new(Some(&info));
            info_label.add_css_class("dim-label");
            info_label.set_halign(Align::Start);
            info_label.set_xalign(0.0);
            info_label.set_wrap(true);
            info_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            info_label.set_max_width_chars(30);
            row_box.append(&info_label);

            // Store tool ID as hidden label
            let id_label = Label::new(Some(&tool.id.0));
            id_label.set_visible(false);
            row_box.append(&id_label);

            self.tools_list.append(&row_box);
        }
    }

    fn start_create_new(&self) {
        *self.is_creating.borrow_mut() = true;
        *self.selected_tool.borrow_mut() = None;

        self.clear_form();
        self.edit_id.set_sensitive(true);
        self.right_panel_stack.set_visible_child_name("edit");
        self.save_btn.set_sensitive(true);
        self.cancel_btn.set_sensitive(true);
        self.delete_btn.set_sensitive(false);
    }

    fn load_tool_for_edit(&self, tool_id: &str) {
        // Load tool from backend
        let backend = self.backend.borrow();
        if let Some(tool) = backend.get_tool(&ToolId(tool_id.to_string())) {
            *self.is_creating.borrow_mut() = false;
            *self.selected_tool.borrow_mut() = Some(tool.clone());
            
            // Populate form with tool data
            self.edit_id.set_text(&tool.id.0);
            self.edit_id.set_sensitive(false);
            self.edit_number.set_value(tool.number as f64);
            self.edit_name.set_text(&tool.name);
            
            // Set tool type
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
            
            // Material - map string to combobox
            self.edit_material.set_active(Some(1)); // Default to Carbide
            
            self.edit_diameter.set_text(&format!("{:.2}", tool.diameter));
            self.edit_length.set_text(&format!("{:.2}", tool.length));
            self.edit_flute_length.set_text(&format!("{:.2}", tool.flute_length));
            
            if let Some(shaft_dia) = tool.shaft_diameter {
                self.edit_shaft_diameter.set_text(&format!("{:.2}", shaft_dia));
            } else {
                self.edit_shaft_diameter.set_text(&format!("{:.2}", tool.diameter));
            }
            
            self.edit_flutes.set_value(tool.flutes as f64);
            
            // Optional fields
            if let Some(ref coating) = tool.coating {
                // Set coating combobox
                self.edit_coating.set_active(Some(0));
            }
            if let Some(ref manufacturer) = tool.manufacturer {
                self.edit_manufacturer.set_text(manufacturer);
            }
            if let Some(ref part_number) = tool.part_number {
                self.edit_part_number.set_text(part_number);
            }
            
            // Description and notes are not Option types
            self.edit_description.buffer().set_text(&tool.description);
            self.edit_notes.buffer().set_text(&tool.notes);
            
            // Show edit form
            self.right_panel_stack.set_visible_child_name("edit");
            self.save_btn.set_sensitive(true);
            self.cancel_btn.set_sensitive(true);
            self.delete_btn.set_sensitive(true);
        }
    }

    fn save_tool(&self) {
        // Collect form data
        let tool_id = self.edit_id.text().to_string();
        let tool_number = self.edit_number.value() as u32;
        let tool_name = self.edit_name.text().to_string();
        
        // Get tool type from combobox
        let tool_type = match self.edit_tool_type.active() {
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
        
        // Parse numeric fields
        let diameter = self.edit_diameter.text().parse::<f32>().unwrap_or(6.35);
        let length = self.edit_length.text().parse::<f32>().unwrap_or(50.0);
        let flute_length = self.edit_flute_length.text().parse::<f32>().unwrap_or(20.0);
        let shaft_diameter = self.edit_shaft_diameter.text().parse::<f32>().ok();
        let flutes = self.edit_flutes.value() as u32;
        
        // Get optional fields
        let manufacturer = {
            let text = self.edit_manufacturer.text().to_string();
            if text.is_empty() { None } else { Some(text) }
        };
        
        let part_number = {
            let text = self.edit_part_number.text().to_string();
            if text.is_empty() { None } else { Some(text) }
        };
        
        let coating = None; // TODO: Parse from combobox
        
        let description = {
            let buffer = self.edit_description.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            buffer.text(&start, &end, true).to_string()
        };
        
        let notes = {
            let buffer = self.edit_notes.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            buffer.text(&start, &end, true).to_string()
        };
        
        // Create tool
        let tool = Tool {
            id: ToolId(tool_id.clone()),
            number: tool_number,
            name: tool_name,
            description,
            tool_type,
            diameter,
            shaft_diameter,
            length,
            flute_length,
            flutes,
            corner_radius: None,
            tip_angle: None,
            material: gcodekit5_core::data::tools::ToolMaterial::Carbide,
            coating,
            shank: gcodekit5_core::data::tools::ShankType::Straight(
                (shaft_diameter.unwrap_or(diameter) * 10.0) as u32
            ),
            params: gcodekit5_core::data::tools::ToolCuttingParams::default(),
            manufacturer,
            part_number,
            cost: None,
            notes,
            custom: true,
        };
        
        // Save to backend
        let mut backend = self.backend.borrow_mut();
        if *self.is_creating.borrow() {
            backend.add_tool(tool);
        } else {
            // For update, remove old and add new
            backend.remove_tool(&ToolId(tool_id.clone()));
            backend.add_tool(tool);
        }
        drop(backend);
        
        // Refresh list and return to empty state
        self.load_tools();
        self.cancel_edit();
    }

    fn delete_tool(&self) {
        // Get the selected tool
        if let Some(ref tool) = *self.selected_tool.borrow() {
            let tool_id = tool.id.clone();
            
            // Show confirmation dialog
            if let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() {
                let dialog = gtk4::MessageDialog::builder()
                    .transient_for(&window)
                    .modal(true)
                    .message_type(gtk4::MessageType::Warning)
                    .buttons(gtk4::ButtonsType::YesNo)
                    .text("Delete Tool?")
                    .secondary_text(&format!(
                        "Are you sure you want to delete tool #{} - {}?\n\nThis action cannot be undone.",
                        tool.number, tool.name
                    ))
                    .build();
                
                let backend = self.backend.clone();
                let view = Rc::new(self.clone());
                
                dialog.connect_response(move |dialog, response| {
                    if response == gtk4::ResponseType::Yes {
                        // Delete from backend
                        let mut backend_mut = backend.borrow_mut();
                        backend_mut.remove_tool(&tool_id);
                        drop(backend_mut);
                        
                        // Refresh list and return to empty state
                        view.load_tools();
                        view.cancel_edit();
                    }
                    dialog.close();
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
        self.edit_diameter.set_text("6.35");
        self.edit_length.set_text("50.0");
        self.edit_flute_length.set_text("20.0");
        self.edit_shaft_diameter.set_text("6.35");
        self.edit_flutes.set_value(2.0);
        self.edit_coating.set_active(Some(0));
        self.edit_manufacturer.set_text("");
        self.edit_part_number.set_text("");
        self.edit_description.buffer().set_text("");
        self.edit_notes.buffer().set_text("");
    }
}
