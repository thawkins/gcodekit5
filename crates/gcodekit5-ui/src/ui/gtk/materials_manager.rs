use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, ButtonsType, CheckButton, ComboBoxText, Entry, Frame, Grid, Image, Label,
    ListBox, ListBoxRow, MessageDialog, MessageType, Orientation, Paned, PolicyType, ResponseType,
    ScrolledWindow, SearchEntry, SelectionMode, SpinButton, Stack, StackSwitcher, TextView,
    WrapMode,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::ui::gtk::help_browser;
use crate::ui::materials_manager_backend;
use crate::ui::materials_manager_backend::MaterialsManagerBackend;
use gcodekit5_core::data::materials::{Material, MaterialCategory, MaterialId};

#[derive(Clone)]
pub struct MaterialsManagerView {
    pub widget: Paned,
    backend: Rc<RefCell<MaterialsManagerBackend>>,
    materials_list: ListBox,
    search_entry: SearchEntry,
    category_filter: ComboBoxText,
    right_panel_stack: Stack,

    // Edit form widgets
    edit_id_row: Grid,
    edit_id: Entry,
    edit_name: Entry,
    edit_category: ComboBoxText,
    edit_subcategory: Entry,
    edit_description: TextView,
    edit_density: SpinButton,
    edit_machinability: SpinButton,
    edit_tensile_strength: Entry,
    edit_melting_point: Entry,
    edit_chip_type: ComboBoxText,
    edit_heat_sensitivity: ComboBoxText,
    edit_abrasiveness: ComboBoxText,
    edit_surface_finish: ComboBoxText,
    edit_dust_hazard: ComboBoxText,
    edit_fume_hazard: ComboBoxText,
    edit_coolant_required: CheckButton,
    edit_notes: TextView,

    // State
    selected_material: Rc<RefCell<Option<Material>>>,
    is_creating: Rc<RefCell<bool>>,

    // Action buttons
    save_btn: Button,
    cancel_btn: Button,
    delete_btn: Button,
    new_btn: Button,
}

impl MaterialsManagerView {
    const DEFAULT_DENSITY: f64 = 750.0;
    const DEFAULT_MACHINABILITY: f64 = 7.0;

    pub fn new() -> Rc<Self> {
        let backend = Rc::new(RefCell::new(MaterialsManagerBackend::new()));

        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        // Set initial position once, then let the user control it.
        let did_set_position = Rc::new(Cell::new(false));
        {
            let did_set_position = did_set_position.clone();
            widget.connect_map(move |paned| {
                if did_set_position.get() {
                    return;
                }
                let width = paned.width();
                if width > 0 {
                    paned.set_position((width as f64 * 0.2) as i32);
                    did_set_position.set(true);
                }
            });
        }

        // ═══════════════════════════════════════════════
        // LEFT SIDEBAR - Materials List
        // ═══════════════════════════════════════════════
        let sidebar = Box::new(Orientation::Vertical, 10);
        sidebar.add_css_class("sidebar");
        sidebar.set_width_request(250);
        sidebar.set_margin_top(10);
        sidebar.set_margin_bottom(10);
        sidebar.set_margin_start(10);
        sidebar.set_margin_end(10);

        // Header
        let header_box = Box::new(Orientation::Horizontal, 10);
        let title = Label::new(Some("Materials"));
        title.add_css_class("title-4");
        title.set_halign(Align::Start);
        header_box.append(&title);

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header_box.append(&spacer);
        header_box.append(&help_browser::make_help_button("materials_manager"));

        sidebar.append(&header_box);

        // Search
        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search materials..."));
        sidebar.append(&search_entry);

        // Category filter
        let category_filter = ComboBoxText::new();
        category_filter.append(Some("all"), "All Categories");
        category_filter.append(Some("wood"), "Wood");
        category_filter.append(Some("eng_wood"), "Engineered Wood");
        category_filter.append(Some("plastic"), "Plastic");
        category_filter.append(Some("non_ferrous"), "Non-Ferrous Metal");
        category_filter.append(Some("ferrous"), "Ferrous Metal");
        category_filter.append(Some("composite"), "Composite");
        category_filter.append(Some("stone"), "Stone & Ceramic");
        category_filter.set_active_id(Some("all"));
        sidebar.append(&category_filter);

        // Materials list
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);

        let materials_list = ListBox::new();
        materials_list.set_activate_on_single_click(true);
        materials_list.set_selection_mode(SelectionMode::Single);
        materials_list.add_css_class("boxed-list");

        let no_results = Label::new(Some("No materials found"));
        no_results.add_css_class("dim-label");
        no_results.set_margin_top(12);
        no_results.set_margin_bottom(12);
        materials_list.set_placeholder(Some(&no_results));

        scroll.set_child(Some(&materials_list));
        sidebar.append(&scroll);

        // New material button
        let new_btn = Self::make_icon_label_button("list-add-symbolic", "New Material");
        new_btn.add_css_class("suggested-action");
        new_btn.set_tooltip_text(Some("Create a new custom material"));
        sidebar.append(&new_btn);

        widget.set_start_child(Some(&sidebar));

        // ═══════════════════════════════════════════════
        // RIGHT PANEL - Material Details/Edit Form
        // ═══════════════════════════════════════════════
        let right_panel_stack = Stack::new();
        right_panel_stack.set_hexpand(true);
        right_panel_stack.set_vexpand(true);

        // Empty state
        let empty_state = Box::new(Orientation::Vertical, 0);
        empty_state.set_valign(Align::Center);
        empty_state.set_halign(Align::Center);
        empty_state.set_vexpand(true);
        empty_state.set_hexpand(true);

        let empty_label = Label::new(Some("Please select or create a material to edit"));
        empty_label.add_css_class("title-2");
        empty_label.add_css_class("dim-label");
        empty_state.append(&empty_label);
        right_panel_stack.add_named(&empty_state, Some("empty"));

        // Edit form container
        let main_content = Box::new(Orientation::Vertical, 10);
        main_content.add_css_class("gk-page-padding");

        // Action buttons bar
        let action_bar = Box::new(Orientation::Horizontal, 10);

        let save_btn = Self::make_icon_label_button("document-save-symbolic", "Save");
        save_btn.add_css_class("suggested-action");
        save_btn.set_sensitive(false);
        save_btn.set_tooltip_text(Some("Save changes"));

        let cancel_btn = Self::make_icon_label_button("window-close-symbolic", "Cancel");
        cancel_btn.set_sensitive(false);
        cancel_btn.set_tooltip_text(Some("Discard changes"));

        let delete_btn = Self::make_icon_label_button("user-trash-symbolic", "Delete");
        delete_btn.add_css_class("destructive-action");
        delete_btn.set_sensitive(false);
        delete_btn.set_tooltip_text(Some("Delete selected custom material"));

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
        let (
            general_page,
            edit_id_row,
            edit_id,
            edit_name,
            edit_category,
            edit_subcategory,
            edit_description,
        ) = Self::create_general_tab();
        let (
            properties_page,
            edit_density,
            edit_machinability,
            edit_tensile_strength,
            edit_melting_point,
        ) = Self::create_properties_tab();
        let (
            machining_page,
            edit_chip_type,
            edit_heat_sensitivity,
            edit_abrasiveness,
            edit_surface_finish,
        ) = Self::create_machining_tab();
        let (safety_page, edit_dust_hazard, edit_fume_hazard, edit_coolant_required) =
            Self::create_safety_tab();
        let (notes_page, edit_notes) = Self::create_notes_tab();

        stack.add_titled(&general_page, Some("general"), "Basic Info");
        stack.add_titled(&properties_page, Some("properties"), "Properties");
        stack.add_titled(&machining_page, Some("machining"), "Machining");
        stack.add_titled(&safety_page, Some("safety"), "Safety");
        stack.add_titled(&notes_page, Some("notes"), "Notes");

        let switcher = StackSwitcher::new();
        switcher.set_stack(Some(&stack));
        switcher.set_halign(Align::Center);

        main_content.append(&switcher);
        main_content.append(&stack);

        right_panel_stack.add_named(&main_content, Some("edit"));
        right_panel_stack.set_visible_child_name("empty");

        widget.set_end_child(Some(&right_panel_stack));

        let view = Rc::new(Self {
            widget,
            backend: backend.clone(),
            materials_list,
            search_entry,
            category_filter,
            right_panel_stack: right_panel_stack.clone(),
            edit_id_row,
            edit_id,
            edit_name,
            edit_category,
            edit_subcategory,
            edit_description,
            edit_density,
            edit_machinability,
            edit_tensile_strength,
            edit_melting_point,
            edit_chip_type,
            edit_heat_sensitivity,
            edit_abrasiveness,
            edit_surface_finish,
            edit_dust_hazard,
            edit_fume_hazard,
            edit_coolant_required,
            edit_notes,
            selected_material: Rc::new(RefCell::new(None)),
            is_creating: Rc::new(RefCell::new(false)),
            save_btn,
            cancel_btn,
            delete_btn,
            new_btn,
        });

        view.setup_event_handlers();
        view.cancel_edit();
        view.load_materials();

        view
    }

    fn make_icon_label_button(icon: &str, label: &str) -> Button {
        let btn = Button::new();
        let content = Box::new(Orientation::Horizontal, 6);
        content.set_halign(Align::Center);
        content.set_valign(Align::Center);

        let img = Image::from_icon_name(icon);
        img.set_pixel_size(16);

        let lbl = Label::new(Some(label));
        lbl.set_valign(Align::Center);

        content.append(&img);
        content.append(&lbl);
        btn.set_child(Some(&content));
        btn
    }

    fn show_error_dialog(title: &str, message: &str) {
        let dialog = MessageDialog::builder()
            .message_type(MessageType::Error)
            .buttons(ButtonsType::Ok)
            .text(title)
            .secondary_text(message)
            .build();
        if let Some(win) = gtk4::Application::default().active_window() {
            dialog.set_transient_for(Some(&win));
            dialog.set_modal(true);
        }
        dialog.connect_response(|d, _| d.close());
        dialog.show();
    }

    fn create_general_tab() -> (
        ScrolledWindow,
        Grid,
        Entry,
        Entry,
        ComboBoxText,
        Entry,
        TextView,
    ) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let vbox = Box::new(Orientation::Vertical, 15);
        vbox.add_css_class("gk-page-padding-sm");

        // ID (shown only when creating)
        let id_grid = Grid::new();
        id_grid.set_column_spacing(10);
        id_grid.set_row_spacing(5);
        let id_label = Label::new(Some("ID:"));
        id_label.set_halign(Align::Start);
        let edit_id = Entry::new();
        edit_id.set_placeholder_text(Some("unique_id"));
        edit_id.set_hexpand(true);
        id_grid.attach(&id_label, 0, 0, 1, 1);
        id_grid.attach(&edit_id, 1, 0, 1, 1);
        vbox.append(&id_grid);

        // Name
        let name_grid = Grid::new();
        name_grid.set_column_spacing(10);
        name_grid.set_row_spacing(5);
        let name_label = Label::new(Some("Name:"));
        name_label.set_halign(Align::Start);
        let edit_name = Entry::new();
        edit_name.set_hexpand(true);
        name_grid.attach(&name_label, 0, 0, 1, 1);
        name_grid.attach(&edit_name, 1, 0, 1, 1);
        vbox.append(&name_grid);

        // Category
        let cat_grid = Grid::new();
        cat_grid.set_column_spacing(10);
        cat_grid.set_row_spacing(5);
        let cat_label = Label::new(Some("Category:"));
        cat_label.set_halign(Align::Start);
        let edit_category = ComboBoxText::new();
        edit_category.append(Some("wood"), "Wood");
        edit_category.append(Some("eng_wood"), "Engineered Wood");
        edit_category.append(Some("plastic"), "Plastic");
        edit_category.append(Some("non_ferrous"), "Non-Ferrous Metal");
        edit_category.append(Some("ferrous"), "Ferrous Metal");
        edit_category.append(Some("composite"), "Composite");
        edit_category.append(Some("stone"), "Stone & Ceramic");
        edit_category.set_active_id(Some("wood"));
        cat_grid.attach(&cat_label, 0, 0, 1, 1);
        cat_grid.attach(&edit_category, 1, 0, 1, 1);
        vbox.append(&cat_grid);

        // Subcategory
        let subcat_grid = Grid::new();
        subcat_grid.set_column_spacing(10);
        subcat_grid.set_row_spacing(5);
        let subcat_label = Label::new(Some("Subcategory:"));
        subcat_label.set_halign(Align::Start);
        let edit_subcategory = Entry::new();
        edit_subcategory.set_placeholder_text(Some("e.g., Hardwood, Alloy"));
        edit_subcategory.set_hexpand(true);
        subcat_grid.attach(&subcat_label, 0, 0, 1, 1);
        subcat_grid.attach(&edit_subcategory, 1, 0, 1, 1);
        vbox.append(&subcat_grid);

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
        (
            scroll,
            id_grid,
            edit_id,
            edit_name,
            edit_category,
            edit_subcategory,
            edit_description,
        )
    }

    fn create_properties_tab() -> (ScrolledWindow, SpinButton, SpinButton, Entry, Entry) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.add_css_class("gk-page-padding-sm");
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let mut row = 0;

        // Density
        let density_label = Label::new(Some("Density (kg/m³):"));
        density_label.set_halign(Align::Start);
        let edit_density = SpinButton::with_range(0.0, 20000.0, 1.0);
        edit_density.set_value(Self::DEFAULT_DENSITY);
        edit_density.set_hexpand(true);
        grid.attach(&density_label, 0, row, 1, 1);
        grid.attach(&edit_density, 1, row, 1, 1);
        row += 1;

        // Machinability
        let mach_label = Label::new(Some("Machinability (1-10):"));
        mach_label.set_halign(Align::Start);
        let edit_machinability = SpinButton::with_range(1.0, 10.0, 1.0);
        edit_machinability.set_value(Self::DEFAULT_MACHINABILITY);
        grid.attach(&mach_label, 0, row, 1, 1);
        grid.attach(&edit_machinability, 1, row, 1, 1);
        row += 1;

        // Tensile strength
        let tens_label = Label::new(Some("Tensile Strength (MPa):"));
        tens_label.set_halign(Align::Start);
        let edit_tensile_strength = Entry::new();
        edit_tensile_strength.set_placeholder_text(Some("Optional"));
        grid.attach(&tens_label, 0, row, 1, 1);
        grid.attach(&edit_tensile_strength, 1, row, 1, 1);
        row += 1;

        // Melting point
        let melt_label = Label::new(Some("Melting Point (°C):"));
        melt_label.set_halign(Align::Start);
        let edit_melting_point = Entry::new();
        edit_melting_point.set_placeholder_text(Some("Optional"));
        grid.attach(&melt_label, 0, row, 1, 1);
        grid.attach(&edit_melting_point, 1, row, 1, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_density,
            edit_machinability,
            edit_tensile_strength,
            edit_melting_point,
        )
    }

    fn create_machining_tab() -> (
        ScrolledWindow,
        ComboBoxText,
        ComboBoxText,
        ComboBoxText,
        ComboBoxText,
    ) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.add_css_class("gk-page-padding-sm");
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let mut row = 0;

        // Chip type
        let chip_label = Label::new(Some("Chip Type:"));
        chip_label.set_halign(Align::Start);
        let edit_chip_type = ComboBoxText::new();
        edit_chip_type.append(Some("continuous"), "Continuous");
        edit_chip_type.append(Some("segmented"), "Segmented");
        edit_chip_type.append(Some("granular"), "Granular");
        edit_chip_type.append(Some("small"), "Small");
        edit_chip_type.set_active_id(Some("continuous"));
        grid.attach(&chip_label, 0, row, 1, 1);
        grid.attach(&edit_chip_type, 1, row, 1, 1);
        row += 1;

        // Heat sensitivity
        let heat_label = Label::new(Some("Heat Sensitivity:"));
        heat_label.set_halign(Align::Start);
        let edit_heat_sensitivity = ComboBoxText::new();
        edit_heat_sensitivity.append(Some("low"), "Low");
        edit_heat_sensitivity.append(Some("moderate"), "Moderate");
        edit_heat_sensitivity.append(Some("high"), "High");
        edit_heat_sensitivity.set_active_id(Some("low"));
        grid.attach(&heat_label, 0, row, 1, 1);
        grid.attach(&edit_heat_sensitivity, 1, row, 1, 1);
        row += 1;

        // Abrasiveness
        let abr_label = Label::new(Some("Abrasiveness:"));
        abr_label.set_halign(Align::Start);
        let edit_abrasiveness = ComboBoxText::new();
        edit_abrasiveness.append(Some("low"), "Low");
        edit_abrasiveness.append(Some("moderate"), "Moderate");
        edit_abrasiveness.append(Some("high"), "High");
        edit_abrasiveness.set_active_id(Some("low"));
        grid.attach(&abr_label, 0, row, 1, 1);
        grid.attach(&edit_abrasiveness, 1, row, 1, 1);
        row += 1;

        // Surface finish
        let surf_label = Label::new(Some("Surface Finish:"));
        surf_label.set_halign(Align::Start);
        let edit_surface_finish = ComboBoxText::new();
        edit_surface_finish.append(Some("excellent"), "Excellent");
        edit_surface_finish.append(Some("good"), "Good");
        edit_surface_finish.append(Some("fair"), "Fair");
        edit_surface_finish.append(Some("rough"), "Rough");
        edit_surface_finish.set_active_id(Some("good"));
        grid.attach(&surf_label, 0, row, 1, 1);
        grid.attach(&edit_surface_finish, 1, row, 1, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_chip_type,
            edit_heat_sensitivity,
            edit_abrasiveness,
            edit_surface_finish,
        )
    }

    fn create_safety_tab() -> (ScrolledWindow, ComboBoxText, ComboBoxText, CheckButton) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.add_css_class("gk-page-padding-sm");
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let mut row = 0;

        // Dust hazard
        let dust_label = Label::new(Some("Dust Hazard:"));
        dust_label.set_halign(Align::Start);
        let edit_dust_hazard = ComboBoxText::new();
        edit_dust_hazard.append(Some("none"), "None");
        edit_dust_hazard.append(Some("minimal"), "Minimal");
        edit_dust_hazard.append(Some("moderate"), "Moderate");
        edit_dust_hazard.append(Some("high"), "High");
        edit_dust_hazard.set_active_id(Some("minimal"));
        grid.attach(&dust_label, 0, row, 1, 1);
        grid.attach(&edit_dust_hazard, 1, row, 1, 1);
        row += 1;

        // Fume hazard
        let fume_label = Label::new(Some("Fume Hazard:"));
        fume_label.set_halign(Align::Start);
        let edit_fume_hazard = ComboBoxText::new();
        edit_fume_hazard.append(Some("none"), "None");
        edit_fume_hazard.append(Some("minimal"), "Minimal");
        edit_fume_hazard.append(Some("moderate"), "Moderate");
        edit_fume_hazard.append(Some("high"), "High");
        edit_fume_hazard.set_active_id(Some("none"));
        grid.attach(&fume_label, 0, row, 1, 1);
        grid.attach(&edit_fume_hazard, 1, row, 1, 1);
        row += 1;

        // Coolant required
        let edit_coolant_required = CheckButton::with_label("Coolant Required");
        grid.attach(&edit_coolant_required, 0, row, 2, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_dust_hazard,
            edit_fume_hazard,
            edit_coolant_required,
        )
    }

    fn create_notes_tab() -> (ScrolledWindow, TextView) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);

        let vbox = Box::new(Orientation::Vertical, 10);
        vbox.add_css_class("gk-page-padding-sm");

        let label = Label::new(Some("Additional Notes and Tips:"));
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
        // New material button
        let view = self.clone();
        self.new_btn.connect_clicked(move |_| {
            view.start_create_new();
        });

        // Save button
        let view = self.clone();
        self.save_btn.connect_clicked(move |_| {
            view.save_material();
        });

        // Cancel button
        let view = self.clone();
        self.cancel_btn.connect_clicked(move |_| {
            view.cancel_edit();
        });

        // Delete button
        let view = self.clone();
        self.delete_btn.connect_clicked(move |_| {
            view.delete_material();
        });

        // Search
        let view = self.clone();
        self.search_entry.connect_search_changed(move |_| {
            view.load_materials();
        });

        // Category filter
        let view = self.clone();
        self.category_filter.connect_changed(move |_| {
            view.load_materials();
        });

        // List selection
        let view = self.clone();
        self.materials_list.connect_row_activated(move |_, row| {
            let id = unsafe {
                row.data::<String>("material-id")
                    .map(|p| p.as_ref().clone())
            };
            if let Some(id) = id {
                view.load_material_for_edit(&id);
            }
        });

        // Form validation
        let view = self.clone();
        self.edit_id
            .connect_changed(move |_| view.update_save_sensitivity());

        let view = self.clone();
        self.edit_name
            .connect_changed(move |_| view.update_save_sensitivity());

        let view = self.clone();
        self.edit_subcategory
            .connect_changed(move |_| view.update_save_sensitivity());

        let view = self.clone();
        self.edit_tensile_strength
            .connect_changed(move |_| view.update_save_sensitivity());

        let view = self.clone();
        self.edit_melting_point
            .connect_changed(move |_| view.update_save_sensitivity());

        let view = self.clone();
        self.edit_description
            .buffer()
            .connect_changed(move |_| view.update_save_sensitivity());

        let view = self.clone();
        self.edit_notes
            .buffer()
            .connect_changed(move |_| view.update_save_sensitivity());
    }

    fn category_sort_key(category: MaterialCategory) -> u8 {
        match category {
            MaterialCategory::Wood => 0,
            MaterialCategory::EngineeredWood => 1,
            MaterialCategory::Plastic => 2,
            MaterialCategory::NonFerrousMetal => 3,
            MaterialCategory::FerrousMetal => 4,
            MaterialCategory::Composite => 5,
            MaterialCategory::StoneAndCeramic => 6,
        }
    }

    fn load_materials(&self) {
        // Clear list
        while let Some(child) = self.materials_list.first_child() {
            self.materials_list.remove(&child);
        }

        let selected_id = self
            .selected_material
            .borrow()
            .as_ref()
            .map(|m| m.id.0.clone());

        let backend = self.backend.borrow();
        let search_query = self.search_entry.text().to_string();
        let mut materials = backend.search_materials(&search_query);

        // Category filter
        if let Some(cat_id) = self.category_filter.active_id() {
            let cat_id = cat_id.to_string();
            if cat_id != "all" {
                if let Some(category) = materials_manager_backend::category_id_to_category(&cat_id)
                {
                    materials.retain(|m| m.category == category);
                }
            }
        }

        // Sort
        materials.sort_by(|a, b| {
            Self::category_sort_key(a.category)
                .cmp(&Self::category_sort_key(b.category))
                .then(a.name.cmp(&b.name))
        });

        let mut row_to_select: Option<ListBoxRow> = None;

        for material in materials {
            let row_box = Box::new(Orientation::Vertical, 5);
            row_box.set_margin_top(5);
            row_box.set_margin_bottom(5);
            row_box.set_margin_start(10);
            row_box.set_margin_end(10);

            let name_label = Label::new(Some(&material.name));
            name_label.add_css_class("title-4");
            name_label.set_halign(Align::Start);
            name_label.set_xalign(0.0);
            name_label.set_wrap(true);
            name_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            name_label.set_max_width_chars(30);
            row_box.append(&name_label);

            let info = format!("{} - {}", material.category, material.subcategory);
            let info_label = Label::new(Some(&info));
            info_label.add_css_class("dim-label");
            info_label.set_halign(Align::Start);
            info_label.set_xalign(0.0);
            info_label.set_wrap(true);
            info_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            info_label.set_max_width_chars(30);
            row_box.append(&info_label);

            let mach_info = format!("Machinability: {}/10", material.machinability_rating);
            let mach_label = Label::new(Some(&mach_info));
            mach_label.set_halign(Align::Start);
            mach_label.set_xalign(0.0);
            row_box.append(&mach_label);

            let row = ListBoxRow::new();
            row.set_child(Some(&row_box));
            unsafe {
                row.set_data("material-id", material.id.0.clone());
            }

            if selected_id.as_deref() == Some(material.id.0.as_str()) {
                row_to_select = Some(row.clone());
            }

            self.materials_list.append(&row);
        }

        if let Some(row) = row_to_select {
            self.materials_list.select_row(Some(&row));
        }
    }

    fn start_create_new(&self) {
        *self.is_creating.borrow_mut() = true;
        *self.selected_material.borrow_mut() = None;

        self.clear_form();
        self.edit_id_row.set_visible(true);
        self.edit_id.set_sensitive(true);

        self.right_panel_stack.set_visible_child_name("edit");
        self.cancel_btn.set_sensitive(true);
        self.delete_btn.set_sensitive(false);

        self.update_save_sensitivity();
    }

    fn load_material_for_edit(&self, material_id: &str) {
        let backend = self.backend.borrow();
        let mat_id = MaterialId(material_id.to_string());

        if let Some(material) = backend.get_material(&mat_id) {
            *self.is_creating.borrow_mut() = false;
            *self.selected_material.borrow_mut() = Some(material.clone());

            self.write_form(material);

            self.edit_id_row.set_visible(false);
            self.edit_id.set_sensitive(false);

            self.right_panel_stack.set_visible_child_name("edit");
            self.cancel_btn.set_sensitive(true);
            self.delete_btn.set_sensitive(material.custom);

            self.update_save_sensitivity();
        }
    }

    fn update_save_sensitivity(&self) {
        let enabled = self.is_form_valid();
        self.save_btn.set_sensitive(enabled);
    }

    fn is_form_valid(&self) -> bool {
        if self.right_panel_stack.visible_child_name().as_deref() != Some("edit") {
            return false;
        }

        let name_ok = !self.edit_name.text().trim().is_empty();
        let subcat_ok = !self.edit_subcategory.text().trim().is_empty();

        if !name_ok || !subcat_ok {
            return false;
        }

        // Numeric optional fields must be valid if present
        let ts = self.edit_tensile_strength.text().trim().to_string();
        if !ts.is_empty() && ts.parse::<f32>().is_err() {
            return false;
        }
        let mp = self.edit_melting_point.text().trim().to_string();
        if !mp.is_empty() && mp.parse::<f32>().is_err() {
            return false;
        }

        if *self.is_creating.borrow() {
            let id = self.edit_id.text().to_string();
            let id = id.trim();
            if id.is_empty() {
                return false;
            }
            if id.contains(char::is_whitespace) {
                return false;
            }
        }

        true
    }

    fn read_textview(tv: &TextView) -> String {
        let buffer = tv.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        buffer.text(&start, &end, true).to_string()
    }

    fn write_form(&self, material: &Material) {
        self.edit_id.set_text(&material.id.0);
        self.edit_name.set_text(&material.name);
        self.edit_category
            .set_active_id(Some(materials_manager_backend::category_to_id(
                material.category,
            )));
        self.edit_subcategory.set_text(&material.subcategory);
        self.edit_description
            .buffer()
            .set_text(&material.description);
        self.edit_density.set_value(material.density as f64);
        self.edit_machinability
            .set_value(material.machinability_rating as f64);

        self.edit_tensile_strength.set_text(
            &material
                .tensile_strength
                .map(|v| v.to_string())
                .unwrap_or_default(),
        );
        self.edit_melting_point.set_text(
            &material
                .melting_point
                .map(|v| v.to_string())
                .unwrap_or_default(),
        );

        self.edit_chip_type
            .set_active_id(Some(materials_manager_backend::chip_type_to_id(
                material.chip_type,
            )));
        self.edit_heat_sensitivity.set_active_id(Some(
            materials_manager_backend::heat_sensitivity_to_id(material.heat_sensitivity),
        ));
        self.edit_abrasiveness
            .set_active_id(Some(materials_manager_backend::abrasiveness_to_id(
                material.abrasiveness,
            )));
        self.edit_surface_finish.set_active_id(Some(
            materials_manager_backend::surface_finish_to_id(material.surface_finish),
        ));
        self.edit_dust_hazard
            .set_active_id(Some(materials_manager_backend::hazard_level_to_id(
                material.dust_hazard,
            )));
        self.edit_fume_hazard
            .set_active_id(Some(materials_manager_backend::hazard_level_to_id(
                material.fume_hazard,
            )));

        self.edit_coolant_required
            .set_active(material.coolant_required);
        self.edit_notes.buffer().set_text(&material.notes);
    }

    fn read_form(&self) -> Result<Material, String> {
        let is_creating = *self.is_creating.borrow();

        let name = self.edit_name.text().trim().to_string();
        if name.is_empty() {
            return Err("Name is required".to_string());
        }

        let subcategory = self.edit_subcategory.text().trim().to_string();
        if subcategory.is_empty() {
            return Err("Subcategory is required".to_string());
        }

        let category_id = self
            .edit_category
            .active_id()
            .map(|s| s.to_string())
            .ok_or_else(|| "Category is required".to_string())?;
        let category = materials_manager_backend::category_id_to_category(&category_id)
            .ok_or_else(|| "Invalid category".to_string())?;

        let tensile_strength = {
            let text = self.edit_tensile_strength.text().trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(
                    text.parse::<f32>()
                        .map_err(|_| "Tensile strength must be a number".to_string())?,
                )
            }
        };

        let melting_point = {
            let text = self.edit_melting_point.text().trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(
                    text.parse::<f32>()
                        .map_err(|_| "Melting point must be a number".to_string())?,
                )
            }
        };

        let chip_type = materials_manager_backend::chip_type_id_to_chip_type(
            &self
                .edit_chip_type
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "continuous".to_string()),
        );
        let heat_sensitivity = materials_manager_backend::heat_sensitivity_id_to_heat_sensitivity(
            &self
                .edit_heat_sensitivity
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "low".to_string()),
        );
        let abrasiveness = materials_manager_backend::abrasiveness_id_to_abrasiveness(
            &self
                .edit_abrasiveness
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "low".to_string()),
        );
        let surface_finish = materials_manager_backend::surface_finish_id_to_surface_finish(
            &self
                .edit_surface_finish
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "good".to_string()),
        );
        let dust_hazard = materials_manager_backend::hazard_level_id_to_hazard_level(
            &self
                .edit_dust_hazard
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "minimal".to_string()),
        );
        let fume_hazard = materials_manager_backend::hazard_level_id_to_hazard_level(
            &self
                .edit_fume_hazard
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "none".to_string()),
        );

        let description = Self::read_textview(&self.edit_description);
        let notes = Self::read_textview(&self.edit_notes);

        let density: f32 = self.edit_density.value() as f32;
        let machinability_rating: u8 = self.edit_machinability.value() as u8;
        let coolant_required = self.edit_coolant_required.is_active();

        let mut material = if is_creating {
            let id = self.edit_id.text().trim().to_string();
            if id.is_empty() {
                return Err("ID is required".to_string());
            }
            if id.contains(char::is_whitespace) {
                return Err("ID must not contain spaces".to_string());
            }
            Material::new(MaterialId(id), name.clone(), category, subcategory.clone())
        } else {
            self.selected_material
                .borrow()
                .as_ref()
                .cloned()
                .ok_or_else(|| "No material selected".to_string())?
        };

        // Update editable fields
        material.name = name;
        material.category = category;
        material.subcategory = subcategory;
        material.description = description;
        material.density = density;
        material.machinability_rating = machinability_rating;
        material.tensile_strength = tensile_strength;
        material.melting_point = melting_point;
        material.chip_type = chip_type;
        material.heat_sensitivity = heat_sensitivity;
        material.abrasiveness = abrasiveness;
        material.surface_finish = surface_finish;
        material.dust_hazard = dust_hazard;
        material.fume_hazard = fume_hazard;
        material.coolant_required = coolant_required;
        material.notes = notes;

        // Any saved material becomes a custom override so it persists.
        material.custom = true;

        Ok(material)
    }

    fn save_material(&self) {
        let is_creating = *self.is_creating.borrow();

        let material = match self.read_form() {
            Ok(m) => m,
            Err(e) => {
                Self::show_error_dialog("Cannot Save Material", &e);
                return;
            }
        };

        // Creating: enforce unique ID
        if is_creating {
            if self.backend.borrow().get_material(&material.id).is_some() {
                Self::show_error_dialog(
                    "Cannot Save Material",
                    "A material with this ID already exists.",
                );
                return;
            }
        }

        let id = material.id.0.clone();
        {
            let mut backend = self.backend.borrow_mut();
            backend.add_material(material);
        }

        *self.is_creating.borrow_mut() = false;
        self.load_materials();
        self.load_material_for_edit(&id);
    }

    fn delete_material(&self) {
        if let Some(ref material) = *self.selected_material.borrow() {
            if !material.custom {
                Self::show_error_dialog(
                    "Cannot Delete Material",
                    "Only custom materials can be deleted.",
                );
                return;
            }

            if let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() {
                let mat_id = material.id.clone();
                let mat_name = material.name.clone();
                let backend = self.backend.clone();
                let view = Rc::new(self.clone());

                let dialog = MessageDialog::builder()
                    .transient_for(&window)
                    .modal(true)
                    .message_type(MessageType::Warning)
                    .buttons(ButtonsType::YesNo)
                    .text("Delete Material?")
                    .secondary_text(&format!(
                        "Are you sure you want to delete '{}' ({} )?\n\nThis action cannot be undone.",
                        mat_name, mat_id.0
                    ))
                    .build();

                dialog.connect_response(move |dialog, response| {
                    if response == ResponseType::Yes {
                        let mut backend_mut = backend.borrow_mut();
                        backend_mut.remove_material(&mat_id);
                        drop(backend_mut);

                        view.load_materials();
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
        *self.selected_material.borrow_mut() = None;
        self.clear_form();
        self.edit_id_row.set_visible(false);
        self.right_panel_stack.set_visible_child_name("empty");
        self.save_btn.set_sensitive(false);
        self.cancel_btn.set_sensitive(false);
        self.delete_btn.set_sensitive(false);
    }

    fn clear_form(&self) {
        self.edit_id.set_text("");
        self.edit_name.set_text("");
        self.edit_category.set_active_id(Some("wood"));
        self.edit_subcategory.set_text("");
        self.edit_description.buffer().set_text("");
        self.edit_density.set_value(Self::DEFAULT_DENSITY);
        self.edit_machinability
            .set_value(Self::DEFAULT_MACHINABILITY);
        self.edit_tensile_strength.set_text("");
        self.edit_melting_point.set_text("");
        self.edit_chip_type.set_active_id(Some("continuous"));
        self.edit_heat_sensitivity.set_active_id(Some("low"));
        self.edit_abrasiveness.set_active_id(Some("low"));
        self.edit_surface_finish.set_active_id(Some("good"));
        self.edit_dust_hazard.set_active_id(Some("minimal"));
        self.edit_fume_hazard.set_active_id(Some("none"));
        self.edit_coolant_required.set_active(false);
        self.edit_notes.buffer().set_text("");
    }
}
