use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, Grid, Label, ListBox, ListBoxRow,
    MessageDialog, MessageType, Orientation, Paned, PolicyType, ResponseType, ScrolledWindow,
    SearchEntry, Stack, StackSwitcher,
};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::error;

use crate::device_status;
use gcodekit5_devicedb::ui_integration::{DeviceProfileUiModel, DeviceUiController};

use gcodekit5_core::units::{
    format_feed_rate, format_length, get_unit_label, parse_feed_rate, parse_length, FeedRateUnits,
    MeasurementSystem,
};
use gcodekit5_settings::controller::SettingsController;
use gcodekit5_settings::manager::SettingsManager;

#[derive(Clone)]
pub struct DeviceManagerWindow {
    pub widget: Paned,
    controller: Rc<DeviceUiController>,
    current_units: Rc<RefCell<MeasurementSystem>>,
    current_feed_units: Rc<RefCell<FeedRateUnits>>,
    devices_list: ListBox,
    search_entry: SearchEntry,

    // Edit form widgets
    edit_name: Entry,
    edit_description: Entry,
    edit_device_type: ComboBoxText,
    edit_controller_type: ComboBoxText,
    edit_connection_type: ComboBoxText,
    edit_port: Entry,
    edit_baud_rate: ComboBoxText,
    edit_tcp_port: Entry,
    edit_timeout: Entry,
    edit_auto_reconnect: CheckButton,
    edit_x_min: Entry,
    edit_x_max: Entry,
    edit_y_min: Entry,
    edit_y_max: Entry,
    edit_z_min: Entry,
    edit_z_max: Entry,
    edit_x_min_unit: Label,
    edit_x_max_unit: Label,
    edit_y_min_unit: Label,
    edit_y_max_unit: Label,
    edit_z_min_unit: Label,
    edit_z_max_unit: Label,
    edit_has_spindle: CheckButton,
    edit_has_laser: CheckButton,
    edit_has_coolant: CheckButton,
    edit_max_feed_rate: Entry,
    edit_max_feed_rate_unit: Label,
    edit_max_s_value: Entry,
    edit_spindle_watts: Entry,
    edit_max_spindle_speed_rpm: Entry,
    edit_laser_watts: Entry,

    // State
    selected_device: Rc<RefCell<Option<DeviceProfileUiModel>>>,

    // Action buttons
    save_btn: Button,
    cancel_btn: Button,
    delete_btn: Button,
    set_active_btn: Button,
    new_btn: Button,

    // UI Stacks
    edit_stack: Stack,
}

impl DeviceManagerWindow {
    pub fn new(
        controller: Rc<DeviceUiController>,
        settings_controller: Rc<SettingsController>,
    ) -> Rc<Self> {
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        // Units are stored internally as mm (and mm/min); convert for display/input.
        let (initial_units, initial_feed_units) =
            if let Ok(path) = SettingsManager::config_file_path() {
                if let Ok(mgr) = SettingsManager::load_from_file(&path) {
                    (mgr.config().ui.measurement_system, mgr.config().ui.feed_rate_units)
                } else {
                    (MeasurementSystem::Metric, FeedRateUnits::default())
                }
            } else {
                (MeasurementSystem::Metric, FeedRateUnits::default())
            };

        let current_units = Rc::new(RefCell::new(initial_units));
        let current_feed_units = Rc::new(RefCell::new(initial_feed_units));

        // LEFT SIDEBAR - Devices List
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
        let title = Label::new(Some("Devices"));
        title.add_css_class("title-4");
        title.set_halign(Align::Start);
        header_box.append(&title);
        sidebar.append(&header_box);

        // Search
        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search devices…"));
        sidebar.append(&search_entry);

        // Devices list
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);

        let devices_list = ListBox::new();
        devices_list.add_css_class("boxed-list");
        scroll.set_child(Some(&devices_list));
        sidebar.append(&scroll);

        // New device button
        let new_btn = Button::with_label("➕ Add Device");
        new_btn.add_css_class("suggested-action");
        sidebar.append(&new_btn);

        widget.set_start_child(Some(&sidebar));

        // RIGHT PANEL - Device Details/Edit Form
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
        let set_active_btn = Button::with_label("✓ Set Active");
        set_active_btn.set_sensitive(false);

        action_bar.append(&save_btn);
        action_bar.append(&cancel_btn);
        action_bar.append(&delete_btn);

        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        action_bar.append(&spacer);

        action_bar.append(&set_active_btn);

        main_content.append(&action_bar);

        // Stack with tabs
        let stack = Stack::new();
        stack.set_vexpand(true);

        // Create tab pages
        let (general_page, edit_name, edit_description, edit_device_type, edit_controller_type) =
            Self::create_general_tab();
        let (
            connection_page,
            edit_connection_type,
            edit_port,
            edit_baud_rate,
            edit_tcp_port,
            edit_timeout,
            edit_auto_reconnect,
        ) = Self::create_connection_tab();
        let (
            dimensions_page,
            edit_x_min,
            edit_x_max,
            edit_y_min,
            edit_y_max,
            edit_z_min,
            edit_z_max,
            edit_x_min_unit,
            edit_x_max_unit,
            edit_y_min_unit,
            edit_y_max_unit,
            edit_z_min_unit,
            edit_z_max_unit,
        ) = Self::create_dimensions_tab(*current_units.borrow());
        let (
            capabilities_page,
            edit_has_spindle,
            edit_has_laser,
            edit_has_coolant,
            edit_max_feed_rate,
            edit_max_feed_rate_unit,
            edit_max_s_value,
            edit_spindle_watts,
            edit_max_spindle_speed_rpm,
            edit_laser_watts,
        ) = Self::create_capabilities_tab(*current_feed_units.borrow());

        stack.add_titled(&general_page, Some("general"), "General");
        stack.add_titled(&connection_page, Some("connection"), "Connection");
        stack.add_titled(&dimensions_page, Some("dimensions"), "Dimensions");
        stack.add_titled(&capabilities_page, Some("capabilities"), "Capabilities");

        let switcher = StackSwitcher::new();
        switcher.set_stack(Some(&stack));
        switcher.set_halign(Align::Center);

        main_content.append(&switcher);
        main_content.append(&stack);

        // Right Panel Stack (Switcher between Edit Form and Placeholder)
        let edit_stack = Stack::new();
        edit_stack.set_transition_type(gtk4::StackTransitionType::Crossfade);

        // -- Placeholder View
        let placeholder_box = Box::new(Orientation::Vertical, 0);
        placeholder_box.set_valign(Align::Center);
        placeholder_box.set_halign(Align::Center);

        let placeholder_label = Label::new(Some("Please select or create a device entry to edit"));
        placeholder_label.add_css_class("dim-label");
        placeholder_label.add_css_class("title-3");

        placeholder_box.append(&placeholder_label);

        // -- Main Edit Form (Wraps existing main_content)
        edit_stack.add_named(&placeholder_box, Some("placeholder"));
        edit_stack.add_named(&main_content, Some("edit_form"));

        // Default to placeholder
        edit_stack.set_visible_child_name("placeholder");

        widget.set_end_child(Some(&edit_stack));

        // Set initial position (once)
        widget.connect_map(|paned| {
            let paned = paned.clone();
            gtk4::glib::idle_add_local_once(move || {
                let width = paned.width();
                if width > 0 {
                    paned.set_position((width as f64 * 0.2) as i32);
                }
            });
        });

        let view = Rc::new(Self {
            widget,
            controller: controller.clone(),
            current_units,
            current_feed_units,
            devices_list,
            search_entry,
            edit_name,
            edit_description,
            edit_device_type,
            edit_controller_type,
            edit_connection_type,
            edit_port,
            edit_baud_rate,
            edit_tcp_port,
            edit_timeout,
            edit_auto_reconnect,
            edit_x_min,
            edit_x_max,
            edit_y_min,
            edit_y_max,
            edit_z_min,
            edit_z_max,
            edit_x_min_unit,
            edit_x_max_unit,
            edit_y_min_unit,
            edit_y_max_unit,
            edit_z_min_unit,
            edit_z_max_unit,
            edit_has_spindle,
            edit_has_laser,
            edit_has_coolant,
            edit_max_feed_rate,
            edit_max_feed_rate_unit,
            edit_max_s_value,
            edit_spindle_watts,
            edit_max_spindle_speed_rpm,
            edit_laser_watts,
            selected_device: Rc::new(RefCell::new(None)),
            save_btn,
            cancel_btn,
            delete_btn,
            set_active_btn,
            new_btn,
            edit_stack,
        });

        {
            let view = view.clone();
            settings_controller.on_setting_changed(move |key, value| {
                if key == "measurement_system" || key == "units.measurement_system" {
                    let new_units = match value {
                        "Imperial" => MeasurementSystem::Imperial,
                        _ => MeasurementSystem::Metric,
                    };
                    {
                        *view.current_units.borrow_mut() = new_units;
                    }
                    view.refresh_units_display();
                }

                if key == "feed_rate_units" {
                    let new_units = match value {
                        "mm/sec" => FeedRateUnits::MmPerSec,
                        "in/min" => FeedRateUnits::InPerMin,
                        "in/sec" => FeedRateUnits::InPerSec,
                        _ => FeedRateUnits::MmPerMin,
                    };
                    {
                        *view.current_feed_units.borrow_mut() = new_units;
                    }
                    view.refresh_units_display();
                }
            });
        }

        view.setup_event_handlers();
        view.update_connection_field_sensitivity();
        view.load_devices();

        view
    }

    pub fn widget(&self) -> &Paned {
        &self.widget
    }

    fn refresh_units_display(&self) {
        let units = *self.current_units.borrow();
        let feed_units = *self.current_feed_units.borrow();

        let unit_label = get_unit_label(units);
        self.edit_x_min_unit.set_text(unit_label);
        self.edit_x_max_unit.set_text(unit_label);
        self.edit_y_min_unit.set_text(unit_label);
        self.edit_y_max_unit.set_text(unit_label);
        self.edit_z_min_unit.set_text(unit_label);
        self.edit_z_max_unit.set_text(unit_label);

        self.edit_max_feed_rate_unit.set_text(&feed_units.to_string());

        let model_opt = self.selected_device.borrow().clone();
        if let Some(profile) = model_opt {
            self.edit_x_min
                .set_text(&format_length(profile.x_min.parse::<f32>().unwrap_or(0.0), units));
            self.edit_x_max
                .set_text(&format_length(profile.x_max.parse::<f32>().unwrap_or(200.0), units));
            self.edit_y_min
                .set_text(&format_length(profile.y_min.parse::<f32>().unwrap_or(0.0), units));
            self.edit_y_max
                .set_text(&format_length(profile.y_max.parse::<f32>().unwrap_or(200.0), units));
            self.edit_z_min
                .set_text(&format_length(profile.z_min.parse::<f32>().unwrap_or(0.0), units));
            self.edit_z_max
                .set_text(&format_length(profile.z_max.parse::<f32>().unwrap_or(100.0), units));

            self.edit_max_feed_rate.set_text(&format_feed_rate(
                profile.max_feed_rate.parse::<f32>().unwrap_or(1000.0),
                feed_units,
            ));
        }
    }

    fn create_general_tab() -> (ScrolledWindow, Entry, Entry, ComboBoxText, ComboBoxText) {
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

        // Name
        let name_label = Label::new(Some("Name:"));
        name_label.set_halign(Align::Start);
        let edit_name = Entry::new();
        edit_name.set_hexpand(true);
        grid.attach(&name_label, 0, row, 1, 1);
        grid.attach(&edit_name, 1, row, 1, 1);
        row += 1;

        // Description
        let desc_label = Label::new(Some("Description:"));
        desc_label.set_halign(Align::Start);
        let edit_description = Entry::new();
        edit_description.set_hexpand(true);
        grid.attach(&desc_label, 0, row, 1, 1);
        grid.attach(&edit_description, 1, row, 1, 1);
        row += 1;

        // Device Type
        let type_label = Label::new(Some("Device Type:"));
        type_label.set_halign(Align::Start);
        let edit_device_type = ComboBoxText::new();
        edit_device_type.append(Some("CNC Mill"), "CNC Mill");
        edit_device_type.append(Some("CNC Lathe"), "CNC Lathe");
        edit_device_type.append(Some("Laser Cutter"), "Laser Cutter");
        edit_device_type.append(Some("3D Printer"), "3D Printer");
        edit_device_type.append(Some("Plotter"), "Plotter");
        edit_device_type.set_active_id(Some("CNC Mill"));
        grid.attach(&type_label, 0, row, 1, 1);
        grid.attach(&edit_device_type, 1, row, 1, 1);
        row += 1;

        // Controller Type
        let ctrl_label = Label::new(Some("Controller:"));
        ctrl_label.set_halign(Align::Start);
        let edit_controller_type = ComboBoxText::new();
        edit_controller_type.append(Some("GRBL"), "GRBL");
        edit_controller_type.append(Some("TinyG"), "TinyG");
        edit_controller_type.append(Some("g2core"), "g2core");
        edit_controller_type.append(Some("Smoothieware"), "Smoothieware");
        edit_controller_type.append(Some("FluidNC"), "FluidNC");
        edit_controller_type.append(Some("Marlin"), "Marlin");
        edit_controller_type.set_active_id(Some("GRBL"));
        grid.attach(&ctrl_label, 0, row, 1, 1);
        grid.attach(&edit_controller_type, 1, row, 1, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_name,
            edit_description,
            edit_device_type,
            edit_controller_type,
        )
    }

    fn create_connection_tab() -> (
        ScrolledWindow,
        ComboBoxText,
        Entry,
        ComboBoxText,
        Entry,
        Entry,
        CheckButton,
    ) {
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

        // Connection Type
        let conn_label = Label::new(Some("Connection Type:"));
        conn_label.set_halign(Align::Start);
        let edit_connection_type = ComboBoxText::new();
        edit_connection_type.append(Some("Serial"), "Serial");
        edit_connection_type.append(Some("TCP/IP"), "TCP/IP");
        edit_connection_type.append(Some("WebSocket"), "WebSocket");
        edit_connection_type.set_active_id(Some("Serial"));
        grid.attach(&conn_label, 0, row, 1, 1);
        grid.attach(&edit_connection_type, 1, row, 1, 1);
        row += 1;

        // Port
        let port_label = Label::new(Some("Port:"));
        port_label.set_halign(Align::Start);
        let edit_port = Entry::new();
        edit_port.set_placeholder_text(Some("/dev/ttyUSB0"));
        grid.attach(&port_label, 0, row, 1, 1);
        grid.attach(&edit_port, 1, row, 1, 1);
        row += 1;

        // Baud Rate
        let baud_label = Label::new(Some("Baud Rate:"));
        baud_label.set_halign(Align::Start);
        let edit_baud_rate = ComboBoxText::new();
        edit_baud_rate.append(Some("9600"), "9600");
        edit_baud_rate.append(Some("19200"), "19200");
        edit_baud_rate.append(Some("38400"), "38400");
        edit_baud_rate.append(Some("57600"), "57600");
        edit_baud_rate.append(Some("115200"), "115200");
        edit_baud_rate.append(Some("250000"), "250000");
        edit_baud_rate.set_active_id(Some("115200"));
        grid.attach(&baud_label, 0, row, 1, 1);
        grid.attach(&edit_baud_rate, 1, row, 1, 1);
        row += 1;

        // TCP Port
        let tcp_label = Label::new(Some("TCP Port:"));
        tcp_label.set_halign(Align::Start);
        let edit_tcp_port = Entry::new();
        edit_tcp_port.set_input_purpose(gtk4::InputPurpose::Number);
        edit_tcp_port.set_text("23");
        grid.attach(&tcp_label, 0, row, 1, 1);
        grid.attach(&edit_tcp_port, 1, row, 1, 1);
        row += 1;

        // Timeout
        let timeout_label = Label::new(Some("Timeout (ms):"));
        timeout_label.set_halign(Align::Start);
        let edit_timeout = Entry::new();
        edit_timeout.set_input_purpose(gtk4::InputPurpose::Number);
        edit_timeout.set_text("5000");
        grid.attach(&timeout_label, 0, row, 1, 1);
        grid.attach(&edit_timeout, 1, row, 1, 1);
        row += 1;

        // Auto Reconnect
        let edit_auto_reconnect = CheckButton::with_label("Auto Reconnect");
        grid.attach(&edit_auto_reconnect, 0, row, 2, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_connection_type,
            edit_port,
            edit_baud_rate,
            edit_tcp_port,
            edit_timeout,
            edit_auto_reconnect,
        )
    }

    fn create_dimensions_tab(
        units: MeasurementSystem,
    ) -> (
        ScrolledWindow,
        Entry,
        Entry,
        Entry,
        Entry,
        Entry,
        Entry,
        Label,
        Label,
        Label,
        Label,
        Label,
        Label,
    ) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let unit_label = get_unit_label(units);

        let mut row = 0;

        // X Axis
        let x_label = Label::new(Some("X Axis:"));
        x_label.set_halign(Align::Start);

        let x_min_label = Label::new(Some("Min:"));
        let edit_x_min = Entry::new();
        edit_x_min.set_input_purpose(gtk4::InputPurpose::Number);
        edit_x_min.set_width_chars(8);

        let edit_x_min_unit = Label::new(Some(unit_label));
        edit_x_min_unit.set_width_chars(4);
        edit_x_min_unit.set_halign(Align::End);
        edit_x_min_unit.set_xalign(1.0);

        let x_max_label = Label::new(Some("Max:"));
        let edit_x_max = Entry::new();
        edit_x_max.set_input_purpose(gtk4::InputPurpose::Number);
        edit_x_max.set_width_chars(8);

        let edit_x_max_unit = Label::new(Some(unit_label));
        edit_x_max_unit.set_width_chars(4);
        edit_x_max_unit.set_halign(Align::End);
        edit_x_max_unit.set_xalign(1.0);

        grid.attach(&x_label, 0, row, 1, 1);
        grid.attach(&x_min_label, 1, row, 1, 1);
        grid.attach(&edit_x_min, 2, row, 1, 1);
        grid.attach(&edit_x_min_unit, 3, row, 1, 1);
        grid.attach(&x_max_label, 4, row, 1, 1);
        grid.attach(&edit_x_max, 5, row, 1, 1);
        grid.attach(&edit_x_max_unit, 6, row, 1, 1);
        row += 1;

        // Y Axis
        let y_label = Label::new(Some("Y Axis:"));
        y_label.set_halign(Align::Start);

        let y_min_label = Label::new(Some("Min:"));
        let edit_y_min = Entry::new();
        edit_y_min.set_input_purpose(gtk4::InputPurpose::Number);
        edit_y_min.set_width_chars(8);

        let edit_y_min_unit = Label::new(Some(unit_label));
        edit_y_min_unit.set_width_chars(4);
        edit_y_min_unit.set_halign(Align::End);
        edit_y_min_unit.set_xalign(1.0);

        let y_max_label = Label::new(Some("Max:"));
        let edit_y_max = Entry::new();
        edit_y_max.set_input_purpose(gtk4::InputPurpose::Number);
        edit_y_max.set_width_chars(8);

        let edit_y_max_unit = Label::new(Some(unit_label));
        edit_y_max_unit.set_width_chars(4);
        edit_y_max_unit.set_halign(Align::End);
        edit_y_max_unit.set_xalign(1.0);

        grid.attach(&y_label, 0, row, 1, 1);
        grid.attach(&y_min_label, 1, row, 1, 1);
        grid.attach(&edit_y_min, 2, row, 1, 1);
        grid.attach(&edit_y_min_unit, 3, row, 1, 1);
        grid.attach(&y_max_label, 4, row, 1, 1);
        grid.attach(&edit_y_max, 5, row, 1, 1);
        grid.attach(&edit_y_max_unit, 6, row, 1, 1);
        row += 1;

        // Z Axis
        let z_label = Label::new(Some("Z Axis:"));
        z_label.set_halign(Align::Start);

        let z_min_label = Label::new(Some("Min:"));
        let edit_z_min = Entry::new();
        edit_z_min.set_input_purpose(gtk4::InputPurpose::Number);
        edit_z_min.set_width_chars(8);

        let edit_z_min_unit = Label::new(Some(unit_label));
        edit_z_min_unit.set_width_chars(4);
        edit_z_min_unit.set_halign(Align::End);
        edit_z_min_unit.set_xalign(1.0);

        let z_max_label = Label::new(Some("Max:"));
        let edit_z_max = Entry::new();
        edit_z_max.set_input_purpose(gtk4::InputPurpose::Number);
        edit_z_max.set_width_chars(8);

        let edit_z_max_unit = Label::new(Some(unit_label));
        edit_z_max_unit.set_width_chars(4);
        edit_z_max_unit.set_halign(Align::End);
        edit_z_max_unit.set_xalign(1.0);

        grid.attach(&z_label, 0, row, 1, 1);
        grid.attach(&z_min_label, 1, row, 1, 1);
        grid.attach(&edit_z_min, 2, row, 1, 1);
        grid.attach(&edit_z_min_unit, 3, row, 1, 1);
        grid.attach(&z_max_label, 4, row, 1, 1);
        grid.attach(&edit_z_max, 5, row, 1, 1);
        grid.attach(&edit_z_max_unit, 6, row, 1, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_x_min,
            edit_x_max,
            edit_y_min,
            edit_y_max,
            edit_z_min,
            edit_z_max,
            edit_x_min_unit,
            edit_x_max_unit,
            edit_y_min_unit,
            edit_y_max_unit,
            edit_z_min_unit,
            edit_z_max_unit,
        )
    }

    fn create_capabilities_tab(
        feed_units: FeedRateUnits,
    ) -> (
        ScrolledWindow,
        CheckButton,
        CheckButton,
        CheckButton,
        Entry,
        Label,
        Entry,
        Entry,
        Entry,
        Entry,
    ) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let vbox = Box::new(Orientation::Vertical, 15);
        vbox.set_margin_top(10);
        vbox.set_margin_bottom(10);
        vbox.set_margin_start(10);
        vbox.set_margin_end(10);

        vbox.set_margin_end(10);

        // Limits
        let limits_label = Label::new(Some("Limits"));
        limits_label.set_css_classes(&["title-4"]);
        limits_label.set_halign(Align::Start);
        vbox.append(&limits_label);

        let limits_grid = Grid::new();
        limits_grid.set_column_spacing(10);
        limits_grid.set_row_spacing(10);

        let feed_label = Label::new(Some("Max Feed Rate:"));
        feed_label.set_halign(Align::Start);
        let edit_max_feed_rate = Entry::new();
        edit_max_feed_rate.set_input_purpose(gtk4::InputPurpose::Number);
        edit_max_feed_rate.set_text("1000");

        let edit_max_feed_rate_unit = Label::new(Some(&feed_units.to_string()));
        edit_max_feed_rate_unit.set_width_chars(6);
        edit_max_feed_rate_unit.set_halign(Align::End);
        edit_max_feed_rate_unit.set_xalign(1.0);

        let s_label = Label::new(Some("Max S-Value:"));
        s_label.set_halign(Align::Start);
        let edit_max_s_value = Entry::new();
        edit_max_s_value.set_input_purpose(gtk4::InputPurpose::Number);
        edit_max_s_value.set_text("1000");

        limits_grid.attach(&feed_label, 0, 0, 1, 1);
        limits_grid.attach(&edit_max_feed_rate, 1, 0, 1, 1);
        limits_grid.attach(&edit_max_feed_rate_unit, 2, 0, 1, 1);
        limits_grid.attach(&s_label, 0, 1, 1, 1);
        limits_grid.attach(&edit_max_s_value, 1, 1, 1, 1);

        vbox.append(&limits_grid);
        vbox.append(&gtk4::Separator::new(Orientation::Horizontal));

        let caps_label = Label::new(Some("Capabilities"));
        caps_label.set_css_classes(&["title-4"]);
        caps_label.set_halign(Align::Start);
        vbox.append(&caps_label);

        // Has Spindle
        let edit_has_spindle = CheckButton::with_label("Has Spindle");
        vbox.append(&edit_has_spindle);

        // Spindle Watts
        let spindle_box = Box::new(Orientation::Horizontal, 10);
        let spindle_label = Label::new(Some("Spindle Power (W):"));
        spindle_label.set_halign(Align::Start);
        spindle_label.set_width_request(150);
        let edit_spindle_watts = Entry::new();
        edit_spindle_watts.set_input_purpose(gtk4::InputPurpose::Number);
        edit_spindle_watts.set_text("0");
        spindle_box.append(&spindle_label);
        spindle_box.append(&edit_spindle_watts);
        vbox.append(&spindle_box);

        // Max Spindle Speed
        let spindle_speed_box = Box::new(Orientation::Horizontal, 10);
        let spindle_speed_label = Label::new(Some("Max Spindle Speed (RPM):"));
        spindle_speed_label.set_halign(Align::Start);
        spindle_speed_label.set_width_request(150);
        let edit_max_spindle_speed_rpm = Entry::new();
        edit_max_spindle_speed_rpm.set_input_purpose(gtk4::InputPurpose::Number);
        edit_max_spindle_speed_rpm.set_text("12000");
        spindle_speed_box.append(&spindle_speed_label);
        spindle_speed_box.append(&edit_max_spindle_speed_rpm);
        vbox.append(&spindle_speed_box);

        // Has Laser
        let edit_has_laser = CheckButton::with_label("Has Laser");
        vbox.append(&edit_has_laser);

        // Laser Watts
        let laser_box = Box::new(Orientation::Horizontal, 10);
        let laser_label = Label::new(Some("Laser Power (W):"));
        laser_label.set_halign(Align::Start);
        laser_label.set_width_request(150);
        let edit_laser_watts = Entry::new();
        edit_laser_watts.set_input_purpose(gtk4::InputPurpose::Number);
        edit_laser_watts.set_text("0");
        laser_box.append(&laser_label);
        laser_box.append(&edit_laser_watts);
        vbox.append(&laser_box);

        // Has Coolant
        let edit_has_coolant = CheckButton::with_label("Has Coolant");
        vbox.append(&edit_has_coolant);

        scroll.set_child(Some(&vbox));
        (
            scroll,
            edit_has_spindle,
            edit_has_laser,
            edit_has_coolant,
            edit_max_feed_rate,
            edit_max_feed_rate_unit,
            edit_max_s_value,
            edit_spindle_watts,
            edit_max_spindle_speed_rpm,
            edit_laser_watts,
        )
    }

    fn setup_event_handlers(self: &Rc<Self>) {
        // New device button
        let view = self.clone();
        self.new_btn.connect_clicked(move |_| {
            view.start_create_new();
        });

        // Save button
        let view = self.clone();
        self.save_btn.connect_clicked(move |_| {
            view.save_device();
        });

        // Cancel button
        let view = self.clone();
        self.cancel_btn.connect_clicked(move |_| {
            view.cancel_edit();
        });

        // Delete button
        let view = self.clone();
        self.delete_btn.connect_clicked(move |_| {
            view.delete_device();
        });

        // Set Active button
        let view = self.clone();
        self.set_active_btn.connect_clicked(move |_| {
            view.set_as_active();
        });

        // Search
        let view = self.clone();
        self.search_entry
            .connect_search_changed(move |_| view.load_devices());

        // Connection field toggles
        let view = self.clone();
        self.edit_connection_type
            .connect_changed(move |_| view.update_connection_field_sensitivity());

        // Capability field toggles
        let view = self.clone();
        self.edit_has_spindle
            .connect_toggled(move |_| view.update_capabilities_field_sensitivity());
        let view = self.clone();
        self.edit_has_laser
            .connect_toggled(move |_| view.update_capabilities_field_sensitivity());

        // List selection
        let view = self.clone();
        self.devices_list.connect_row_activated(move |_, row| {
            let device_id = row.widget_name();
            if !device_id.is_empty() {
                view.load_device_for_edit(device_id.as_str());
            }
        });
    }

    fn load_devices(&self) {
        // Clear list
        while let Some(child) = self.devices_list.first_child() {
            self.devices_list.remove(&child);
        }

        let mut profiles = self.controller.get_ui_profiles();
        let q = self.search_entry.text().trim().to_lowercase();
        if !q.is_empty() {
            profiles.retain(|p| {
                let hay = format!(
                    "{}\n{}\n{}\n{}\n{}",
                    p.name, p.description, p.device_type, p.controller_type, p.connection_type
                )
                .to_lowercase();
                hay.contains(&q)
            });
        }

        let status = device_status::get_status();
        let connected_port = status.port_name.clone().unwrap_or_default();
        let is_connected = status.is_connected;

        for profile in profiles {
            let row_box = Box::new(Orientation::Vertical, 5);

            // Name Label (Wrapped)
            let name_label = Label::new(Some(&profile.name));
            name_label.add_css_class("title-4");
            name_label.set_halign(Align::Start);
            name_label.set_wrap(true);
            name_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            name_label.set_xalign(0.0);
            row_box.append(&name_label);

            // Details and Badge container
            let details_box = Box::new(Orientation::Horizontal, 5);

            // Device Info
            let info = format!("{} - {}", profile.device_type, profile.controller_type);
            let info_label = Label::new(Some(&info));
            info_label.add_css_class("dim-label");
            info_label.set_halign(Align::Start);
            info_label.set_hexpand(true); // Pushes badge to the right
            details_box.append(&info_label);

            // Connected badge (serial-only for now)
            if is_connected
                && profile.connection_type == "Serial"
                && !connected_port.is_empty()
                && profile.port.trim() == connected_port.trim()
            {
                let badge = Label::new(Some("Connected"));
                badge.add_css_class("active-badge");
                badge.set_halign(Align::End);
                badge.set_valign(Align::Center);
                details_box.append(&badge);
            }

            // Active Badge
            if profile.is_active {
                let badge = Label::new(Some("Active"));
                badge.add_css_class("active-badge");
                badge.set_halign(Align::End);
                badge.set_valign(Align::Center);
                details_box.append(&badge);
            }

            row_box.append(&details_box);

            let row = ListBoxRow::new();
            row.set_margin_top(5);
            row.set_margin_bottom(5);
            row.set_margin_start(10);
            row.set_margin_end(10);
            row.set_child(Some(&row_box));
            row.set_widget_name(&profile.id);

            self.devices_list.append(&row);
        }
    }

    fn start_create_new(&self) {
        if let Ok(id) = self.controller.create_new_profile() {
            self.load_devices();
            self.load_device_for_edit(&id);
        } else {
            self.load_devices();
        }
    }

    fn load_device_for_edit(&self, device_id: &str) {
        let profiles = self.controller.get_ui_profiles();

        if let Some(profile) = profiles.iter().find(|p| p.id == device_id) {
            *self.selected_device.borrow_mut() = Some(profile.clone());

            // Switch to Edit View
            self.edit_stack.set_visible_child_name("edit_form");

            // Load into form
            self.edit_name.set_text(&profile.name);
            self.edit_description.set_text(&profile.description);

            self.edit_device_type
                .set_active_id(Some(profile.device_type.as_str()));
            self.edit_controller_type
                .set_active_id(Some(profile.controller_type.as_str()));

            self.edit_connection_type
                .set_active_id(Some(profile.connection_type.as_str()));
            self.edit_port.set_text(&profile.port);
            self.edit_baud_rate.set_active_id(Some(profile.baud_rate.as_str()));
            self.edit_tcp_port.set_text(profile.tcp_port.trim());
            self.edit_timeout.set_text(profile.timeout_ms.trim());
            self.edit_auto_reconnect.set_active(profile.auto_reconnect);

            self.refresh_units_display();

            self.edit_has_spindle.set_active(profile.has_spindle);
            self.edit_has_laser.set_active(profile.has_laser);
            self.edit_has_coolant.set_active(profile.has_coolant);
            self.edit_max_s_value.set_text(profile.max_s_value.trim());
            self.edit_spindle_watts.set_text(profile.cnc_spindle_watts.trim());
            self.edit_max_spindle_speed_rpm
                .set_text(profile.max_spindle_speed_rpm.trim());
            self.edit_laser_watts.set_text(profile.laser_watts.trim());

            self.update_connection_field_sensitivity();
            self.update_capabilities_field_sensitivity();

            // Enable buttons
            self.save_btn.set_sensitive(true);
            self.cancel_btn.set_sensitive(true);
            self.delete_btn.set_sensitive(true);
            self.set_active_btn.set_sensitive(!profile.is_active);
        }
    }

    fn update_connection_field_sensitivity(&self) {
        let conn = self
            .edit_connection_type
            .active_id()
            .map(|s| s.to_string())
            .or_else(|| self.edit_connection_type.active_text().map(|s| s.to_string()))
            .unwrap_or_else(|| "Serial".to_string());

        match conn.as_str() {
            "Serial" => {
                self.edit_port.set_sensitive(true);
                self.edit_baud_rate.set_sensitive(true);
                self.edit_tcp_port.set_sensitive(false);
            }
            "TCP/IP" | "WebSocket" => {
                self.edit_port.set_sensitive(false);
                self.edit_baud_rate.set_sensitive(false);
                self.edit_tcp_port.set_sensitive(true);
            }
            _ => {}
        }
    }

    fn update_capabilities_field_sensitivity(&self) {
        let has_spindle = self.edit_has_spindle.is_active();
        self.edit_spindle_watts.set_sensitive(has_spindle);
        self.edit_max_spindle_speed_rpm.set_sensitive(has_spindle);

        let has_laser = self.edit_has_laser.is_active();
        self.edit_laser_watts.set_sensitive(has_laser);
    }

    fn show_error_dialog(&self, title: &str, details: &str) {
        let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() else {
            return;
        };

        let dialog = MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(MessageType::Error)
            .buttons(gtk4::ButtonsType::Ok)
            .text(title)
            .secondary_text(details)
            .build();

        dialog.connect_response(|d, _| d.close());
        dialog.show();
    }

    fn save_device(&self) {
        // Collect model first, then drop the borrow
        let model_opt = self.selected_device.borrow().clone();

        if let Some(mut model) = model_opt {
            // General
            model.name = self.edit_name.text().to_string();
            model.description = self.edit_description.text().to_string();
            if let Some(txt) = self.edit_device_type.active_text() {
                model.device_type = txt.to_string();
            }
            if let Some(txt) = self.edit_controller_type.active_text() {
                model.controller_type = txt.to_string();
            }

            // Connection
            if let Some(txt) = self.edit_connection_type.active_text() {
                model.connection_type = txt.to_string();
            }
            model.port = self.edit_port.text().to_string();
            if let Some(txt) = self.edit_baud_rate.active_text() {
                model.baud_rate = txt.to_string();
            }
            let tcp_port: u16 = match self.edit_tcp_port.text().trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    self.show_error_dialog("Invalid TCP Port", "TCP Port must be a number");
                    return;
                }
            };
            let timeout_ms: u32 = match self.edit_timeout.text().trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    self.show_error_dialog("Invalid Timeout", "Timeout must be a number (ms)");
                    return;
                }
            };
            model.tcp_port = tcp_port.to_string();
            model.timeout_ms = timeout_ms.to_string();
            model.auto_reconnect = self.edit_auto_reconnect.is_active();

            // Dimensions
            let units = *self.current_units.borrow();
            let feed_units = *self.current_feed_units.borrow();

            let x_min_mm = match parse_length(&self.edit_x_min.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid X Min", &e);
                    return;
                }
            };
            let x_max_mm = match parse_length(&self.edit_x_max.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid X Max", &e);
                    return;
                }
            };
            let y_min_mm = match parse_length(&self.edit_y_min.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid Y Min", &e);
                    return;
                }
            };
            let y_max_mm = match parse_length(&self.edit_y_max.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid Y Max", &e);
                    return;
                }
            };
            let z_min_mm = match parse_length(&self.edit_z_min.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid Z Min", &e);
                    return;
                }
            };
            let z_max_mm = match parse_length(&self.edit_z_max.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid Z Max", &e);
                    return;
                }
            };
            model.x_min = format!("{:.2}", x_min_mm);
            model.x_max = format!("{:.2}", x_max_mm);
            model.y_min = format!("{:.2}", y_min_mm);
            model.y_max = format!("{:.2}", y_max_mm);
            model.z_min = format!("{:.2}", z_min_mm);
            model.z_max = format!("{:.2}", z_max_mm);

            // Capabilities
            model.has_spindle = self.edit_has_spindle.is_active();
            model.has_laser = self.edit_has_laser.is_active();
            model.has_coolant = self.edit_has_coolant.is_active();
            let max_feed_mm_per_min = match parse_feed_rate(&self.edit_max_feed_rate.text(), feed_units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid Max Feed Rate", &e);
                    return;
                }
            };
            let max_s_value: f32 = match self.edit_max_s_value.text().trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    self.show_error_dialog("Invalid Max S-Value", "Max S-Value must be a number");
                    return;
                }
            };
            let (spindle_watts, max_spindle_speed_rpm) = if model.has_spindle {
                let spindle_watts: f32 = match self.edit_spindle_watts.text().trim().parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.show_error_dialog(
                            "Invalid Spindle Power",
                            "Spindle power must be a number (W)",
                        );
                        return;
                    }
                };

                let max_spindle_speed_rpm: u32 = match self.edit_max_spindle_speed_rpm.text().trim().parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.show_error_dialog(
                            "Invalid Max Spindle Speed",
                            "Max spindle speed must be an integer (RPM)",
                        );
                        return;
                    }
                };

                (spindle_watts, max_spindle_speed_rpm)
            } else {
                (0.0, 0)
            };

            let laser_watts: f32 = if model.has_laser {
                match self.edit_laser_watts.text().trim().parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.show_error_dialog(
                            "Invalid Laser Power",
                            "Laser power must be a number (W)",
                        );
                        return;
                    }
                }
            } else {
                0.0
            };
            model.max_feed_rate = format!("{:.0}", max_feed_mm_per_min);
            model.max_s_value = format!("{:.0}", max_s_value);
            model.max_spindle_speed_rpm = max_spindle_speed_rpm.to_string();
            model.cnc_spindle_watts = format!("{:.0}", spindle_watts);
            model.laser_watts = format!("{:.0}", laser_watts);

            // Save
            if let Err(e) = self.controller.update_profile_from_ui(model) {
                error!("Failed to save device profile: {}", e);
                self.show_error_dialog("Failed to save device", &e.to_string());
                return;
            }

            self.load_devices();
            self.cancel_edit();
        }
    }

    fn delete_device(self: &Rc<Self>) {
        let Some(model) = self.selected_device.borrow().clone() else {
            return;
        };

        let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() else {
            return;
        };

        let dialog = MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(MessageType::Question)
            .buttons(gtk4::ButtonsType::YesNo)
            .text("Delete device?")
            .secondary_text(format!(
                "Delete “{}”? This cannot be undone.",
                model.name
            ))
            .build();

        let view = self.clone();
        dialog.connect_response(move |d, resp| {
            if resp == ResponseType::Yes {
                let _ = view.controller.delete_profile(&model.id);
                view.load_devices();
                view.cancel_edit();
            }
            d.close();
        });

        dialog.show();
    }

    fn set_as_active(&self) {
        let id_opt = self.selected_device.borrow().as_ref().map(|d| d.id.clone());
        if let Some(id) = id_opt {
            let _ = self.controller.set_active_profile(&id);
            self.load_devices();
            self.cancel_edit();
        }
    }

    fn cancel_edit(&self) {
        *self.selected_device.borrow_mut() = None;
        self.edit_stack.set_visible_child_name("placeholder");
        self.save_btn.set_sensitive(false);
        self.cancel_btn.set_sensitive(false);
        self.delete_btn.set_sensitive(false);
        self.set_active_btn.set_sensitive(false);
    }
}
