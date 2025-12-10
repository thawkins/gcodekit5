use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, Grid, Label, ListBox, Orientation,
    Paned, PolicyType, ScrolledWindow, Stack, StackSwitcher,
};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::error;

use gcodekit5_devicedb::ui_integration::{DeviceProfileUiModel, DeviceUiController};

#[derive(Clone)]
pub struct DeviceManagerWindow {
    pub widget: Paned,
    controller: Rc<DeviceUiController>,
    devices_list: ListBox,

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
    edit_has_spindle: CheckButton,
    edit_has_laser: CheckButton,
    edit_has_coolant: CheckButton,
    edit_max_feed_rate: Entry,
    edit_max_s_value: Entry,
    edit_spindle_watts: Entry,
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
    pub fn new(controller: Rc<DeviceUiController>) -> Rc<Self> {
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

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
        ) = Self::create_dimensions_tab();
        let (
            capabilities_page,
            edit_has_spindle,
            edit_has_laser,
            edit_has_coolant,
            edit_max_feed_rate,
            edit_max_s_value,
            edit_spindle_watts,
            edit_laser_watts,
        ) = Self::create_capabilities_tab();

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
            controller: controller.clone(),
            devices_list,
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
            edit_has_spindle,
            edit_has_laser,
            edit_has_coolant,
            edit_max_feed_rate,
            edit_max_s_value,
            edit_spindle_watts,
            edit_laser_watts,
            selected_device: Rc::new(RefCell::new(None)),
            save_btn,
            cancel_btn,
            delete_btn,
            set_active_btn,
            new_btn,
            edit_stack,
        });

        view.setup_event_handlers();
        view.load_devices();

        view
    }

    pub fn widget(&self) -> &Paned {
        &self.widget
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
        edit_device_type.append_text("CNC Mill");
        edit_device_type.append_text("CNC Lathe");
        edit_device_type.append_text("Laser Cutter");
        edit_device_type.append_text("3D Printer");
        edit_device_type.append_text("Plotter");
        edit_device_type.set_active(Some(0));
        grid.attach(&type_label, 0, row, 1, 1);
        grid.attach(&edit_device_type, 1, row, 1, 1);
        row += 1;

        // Controller Type
        let ctrl_label = Label::new(Some("Controller:"));
        ctrl_label.set_halign(Align::Start);
        let edit_controller_type = ComboBoxText::new();
        edit_controller_type.append_text("GRBL");
        edit_controller_type.append_text("TinyG");
        edit_controller_type.append_text("g2core");
        edit_controller_type.append_text("Smoothieware");
        edit_controller_type.append_text("FluidNC");
        edit_controller_type.append_text("Marlin");
        edit_controller_type.set_active(Some(0));
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
        edit_connection_type.append_text("Serial");
        edit_connection_type.append_text("TCP/IP");
        edit_connection_type.append_text("WebSocket");
        edit_connection_type.set_active(Some(0));
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
        edit_baud_rate.append_text("9600");
        edit_baud_rate.append_text("19200");
        edit_baud_rate.append_text("38400");
        edit_baud_rate.append_text("57600");
        edit_baud_rate.append_text("115200");
        edit_baud_rate.append_text("250000");
        edit_baud_rate.set_active(Some(4));
        grid.attach(&baud_label, 0, row, 1, 1);
        grid.attach(&edit_baud_rate, 1, row, 1, 1);
        row += 1;

        // TCP Port
        let tcp_label = Label::new(Some("TCP Port:"));
        tcp_label.set_halign(Align::Start);
        let edit_tcp_port = Entry::new();
        edit_tcp_port.set_placeholder_text(Some("23"));
        grid.attach(&tcp_label, 0, row, 1, 1);
        grid.attach(&edit_tcp_port, 1, row, 1, 1);
        row += 1;

        // Timeout
        let timeout_label = Label::new(Some("Timeout (ms):"));
        timeout_label.set_halign(Align::Start);
        let edit_timeout = Entry::new();
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

    fn create_dimensions_tab() -> (ScrolledWindow, Entry, Entry, Entry, Entry, Entry, Entry) {
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

        // X Axis
        let x_label = Label::new(Some("X Axis:"));
        x_label.set_halign(Align::Start);
        let x_min_label = Label::new(Some("Min:"));
        let edit_x_min = Entry::new();
        edit_x_min.set_text("0.0");
        edit_x_min.set_width_chars(8);
        let x_max_label = Label::new(Some("Max:"));
        let edit_x_max = Entry::new();
        edit_x_max.set_text("300.0");
        edit_x_max.set_width_chars(8);
        grid.attach(&x_label, 0, row, 1, 1);
        grid.attach(&x_min_label, 1, row, 1, 1);
        grid.attach(&edit_x_min, 2, row, 1, 1);
        grid.attach(&x_max_label, 3, row, 1, 1);
        grid.attach(&edit_x_max, 4, row, 1, 1);
        row += 1;

        // Y Axis
        let y_label = Label::new(Some("Y Axis:"));
        y_label.set_halign(Align::Start);
        let y_min_label = Label::new(Some("Min:"));
        let edit_y_min = Entry::new();
        edit_y_min.set_text("0.0");
        edit_y_min.set_width_chars(8);
        let y_max_label = Label::new(Some("Max:"));
        let edit_y_max = Entry::new();
        edit_y_max.set_text("300.0");
        edit_y_max.set_width_chars(8);
        grid.attach(&y_label, 0, row, 1, 1);
        grid.attach(&y_min_label, 1, row, 1, 1);
        grid.attach(&edit_y_min, 2, row, 1, 1);
        grid.attach(&y_max_label, 3, row, 1, 1);
        grid.attach(&edit_y_max, 4, row, 1, 1);
        row += 1;

        // Z Axis
        let z_label = Label::new(Some("Z Axis:"));
        z_label.set_halign(Align::Start);
        let z_min_label = Label::new(Some("Min:"));
        let edit_z_min = Entry::new();
        edit_z_min.set_text("0.0");
        edit_z_min.set_width_chars(8);
        let z_max_label = Label::new(Some("Max:"));
        let edit_z_max = Entry::new();
        edit_z_max.set_text("100.0");
        edit_z_max.set_width_chars(8);
        grid.attach(&z_label, 0, row, 1, 1);
        grid.attach(&z_min_label, 1, row, 1, 1);
        grid.attach(&edit_z_min, 2, row, 1, 1);
        grid.attach(&z_max_label, 3, row, 1, 1);
        grid.attach(&edit_z_max, 4, row, 1, 1);

        scroll.set_child(Some(&grid));
        (
            scroll, edit_x_min, edit_x_max, edit_y_min, edit_y_max, edit_z_min, edit_z_max,
        )
    }

    fn create_capabilities_tab() -> (
        ScrolledWindow,
        CheckButton,
        CheckButton,
        CheckButton,
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
        edit_max_feed_rate.set_text("1000");

        let s_label = Label::new(Some("Max S-Value:"));
        s_label.set_halign(Align::Start);
        let edit_max_s_value = Entry::new();
        edit_max_s_value.set_text("1000");

        limits_grid.attach(&feed_label, 0, 0, 1, 1);
        limits_grid.attach(&edit_max_feed_rate, 1, 0, 1, 1);
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
        edit_spindle_watts.set_text("0");
        spindle_box.append(&spindle_label);
        spindle_box.append(&edit_spindle_watts);
        vbox.append(&spindle_box);

        // Has Laser
        let edit_has_laser = CheckButton::with_label("Has Laser");
        vbox.append(&edit_has_laser);

        // Laser Watts
        let laser_box = Box::new(Orientation::Horizontal, 10);
        let laser_label = Label::new(Some("Laser Power (W):"));
        laser_label.set_halign(Align::Start);
        laser_label.set_width_request(150);
        let edit_laser_watts = Entry::new();
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
            edit_max_s_value,
            edit_spindle_watts,
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

        // List selection
        let view = self.clone();
        self.devices_list.connect_row_activated(move |_, row| {
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
                    let device_id = label.label().to_string();
                    view.load_device_for_edit(&device_id);
                }
            }
        });
    }

    fn load_devices(&self) {
        // Clear list
        while let Some(child) = self.devices_list.first_child() {
            self.devices_list.remove(&child);
        }

        let profiles = self.controller.get_ui_profiles();

        for profile in profiles {
            let row_box = Box::new(Orientation::Vertical, 5);
            row_box.set_margin_top(5);
            row_box.set_margin_bottom(5);
            row_box.set_margin_start(10);
            row_box.set_margin_end(10);

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

            // Active Badge
            if profile.is_active {
                let badge = Label::new(Some("Active"));
                badge.add_css_class("active-badge");
                badge.set_halign(Align::End);
                badge.set_valign(Align::Center);
                details_box.append(&badge);
            }

            row_box.append(&details_box);

            // Store device ID as hidden label
            let id_label = Label::new(Some(&profile.id));
            id_label.set_visible(false);
            row_box.append(&id_label);

            self.devices_list.append(&row_box);
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
            // Set combos by matching text (simplified)
            self.edit_port.set_text(&profile.port);
            self.edit_tcp_port.set_text(&profile.tcp_port);
            self.edit_timeout.set_text(&profile.timeout_ms);
            self.edit_auto_reconnect.set_active(profile.auto_reconnect);
            self.edit_x_min.set_text(&profile.x_min);
            self.edit_x_max.set_text(&profile.x_max);
            self.edit_y_min.set_text(&profile.y_min);
            self.edit_y_max.set_text(&profile.y_max);
            self.edit_z_min.set_text(&profile.z_min);
            self.edit_z_max.set_text(&profile.z_max);
            self.edit_has_spindle.set_active(profile.has_spindle);
            self.edit_has_laser.set_active(profile.has_laser);
            self.edit_has_coolant.set_active(profile.has_coolant);
            self.edit_max_feed_rate.set_text(&profile.max_feed_rate);
            self.edit_max_s_value.set_text(&profile.max_s_value);
            self.edit_spindle_watts.set_text(&profile.cnc_spindle_watts);
            self.edit_laser_watts.set_text(&profile.laser_watts);

            // Enable buttons
            self.save_btn.set_sensitive(true);
            self.cancel_btn.set_sensitive(true);
            self.delete_btn.set_sensitive(true);
            self.set_active_btn.set_sensitive(!profile.is_active);
        }
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
            model.tcp_port = self.edit_tcp_port.text().to_string();
            model.timeout_ms = self.edit_timeout.text().to_string();
            model.auto_reconnect = self.edit_auto_reconnect.is_active();

            // Dimensions
            model.x_min = self.edit_x_min.text().to_string();
            model.x_max = self.edit_x_max.text().to_string();
            model.y_min = self.edit_y_min.text().to_string();
            model.y_max = self.edit_y_max.text().to_string();
            model.z_min = self.edit_z_min.text().to_string();
            model.z_max = self.edit_z_max.text().to_string();

            // Capabilities
            model.has_spindle = self.edit_has_spindle.is_active();
            model.has_laser = self.edit_has_laser.is_active();
            model.has_coolant = self.edit_has_coolant.is_active();
            model.max_feed_rate = self.edit_max_feed_rate.text().to_string();
            model.max_s_value = self.edit_max_s_value.text().to_string();
            model.cnc_spindle_watts = self.edit_spindle_watts.text().to_string();
            model.laser_watts = self.edit_laser_watts.text().to_string();

            // Save
            if let Err(e) = self.controller.update_profile_from_ui(model) {
                error!("Failed to save device profile: {}", e);
            }

            self.load_devices();
            self.cancel_edit();
        }
    }

    fn delete_device(&self) {
        let id_opt = self.selected_device.borrow().as_ref().map(|d| d.id.clone());
        if let Some(id) = id_opt {
            let _ = self.controller.delete_profile(&id);
            self.load_devices();
            self.cancel_edit();
        }
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
