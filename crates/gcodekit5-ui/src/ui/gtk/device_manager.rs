use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, Label, ListView, Orientation,
    ScrolledWindow, SelectionMode, SignalListItemFactory, SingleSelection, StringList, Window,
    Stack, StackSidebar,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup, PreferencesPage, PreferencesWindow};
use std::cell::RefCell;
use std::rc::Rc;

use gcodekit5_devicedb::ui_integration::{DeviceProfileUiModel, DeviceUiController};

pub struct DeviceManagerWindow {
    pub content: Box,
    controller: Rc<DeviceUiController>,
    stack: Stack,
    sidebar: StackSidebar,
}

impl DeviceManagerWindow {
    pub fn new(controller: Rc<DeviceUiController>) -> Self {
        let content = Box::new(Orientation::Horizontal, 0);
        
        let sidebar = StackSidebar::new();
        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::SlideUpDown);
        
        sidebar.set_stack(&stack);
        sidebar.set_width_request(250);
        
        content.append(&sidebar);
        content.append(&stack);
        
        let manager = Self {
            content,
            controller,
            stack,
            sidebar,
        };
        
        manager.refresh_devices();
        manager
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    pub fn refresh_devices(&self) {
        // Clear existing pages (not directly supported by Stack, usually we'd rebuild or manage children)
        // For simplicity in this phase, we'll just append new ones. In a real app, we'd diff or clear.
        
        let profiles = self.controller.get_ui_profiles();
        
        for profile in profiles {
            let page = self.create_device_page(&profile);
            self.stack.add_titled(&page, Some(&profile.id), &profile.name);
        }
        
        // Add "New Device" button/page
        let new_btn = Button::with_label("Create New Device");
        let controller = self.controller.clone();
        // In a real implementation, this would trigger a refresh
        new_btn.connect_clicked(move |_| {
            let _ = controller.create_new_profile();
        });
        self.stack.add_titled(&new_btn, Some("new_device"), "+ New Device");
    }

    fn create_device_page(&self, profile: &DeviceProfileUiModel) -> Box {
        let page = Box::new(Orientation::Vertical, 12);
        page.set_margin_top(24);
        page.set_margin_bottom(24);
        page.set_margin_start(24);
        page.set_margin_end(24);

        // Header
        let title = Label::builder()
            .label(&profile.name)
            .css_classes(vec!["title-1"])
            .halign(Align::Start)
            .build();
        page.append(&title);

        // Tabs for General, Connection, Dimensions, Capabilities
        let notebook = gtk4::Notebook::new();
        
        // General Tab
        let general_box = Box::new(Orientation::Vertical, 12);
        general_box.set_margin_top(12);
        
        self.add_entry_row(&general_box, "Name", &profile.name);
        self.add_entry_row(&general_box, "Description", &profile.description);
        self.add_combo_row(&general_box, "Device Type", &profile.device_type, &["CNC Mill", "CNC Lathe", "Laser Cutter", "3D Printer", "Plotter"]);
        self.add_combo_row(&general_box, "Controller", &profile.controller_type, &["GRBL", "TinyG", "g2core", "Smoothieware", "FluidNC", "Marlin"]);
        
        notebook.append_page(&general_box, Some(&Label::new(Some("General"))));

        // Connection Tab
        let conn_box = Box::new(Orientation::Vertical, 12);
        conn_box.set_margin_top(12);
        
        self.add_combo_row(&conn_box, "Connection Type", &profile.connection_type, &["Serial", "Network", "Mock"]);
        
        if profile.connection_type == "Serial" {
            self.add_entry_row(&conn_box, "Port", &profile.port);
            self.add_combo_row(&conn_box, "Baud Rate", &profile.baud_rate, &["9600", "19200", "38400", "57600", "115200", "250000"]);
        } else {
            self.add_entry_row(&conn_box, "TCP Port", &profile.tcp_port);
        }
        
        self.add_entry_row(&conn_box, "Timeout (ms)", &profile.timeout_ms);
        self.add_check_row(&conn_box, "Auto Reconnect", profile.auto_reconnect);
        
        notebook.append_page(&conn_box, Some(&Label::new(Some("Connection"))));

        // Dimensions Tab
        let dim_box = Box::new(Orientation::Vertical, 12);
        dim_box.set_margin_top(12);
        
        self.add_axis_row(&dim_box, "X Axis", &profile.x_min, &profile.x_max);
        self.add_axis_row(&dim_box, "Y Axis", &profile.y_min, &profile.y_max);
        self.add_axis_row(&dim_box, "Z Axis", &profile.z_min, &profile.z_max);
        
        notebook.append_page(&dim_box, Some(&Label::new(Some("Dimensions"))));

        // Capabilities Tab
        let cap_box = Box::new(Orientation::Vertical, 12);
        cap_box.set_margin_top(12);
        
        self.add_check_row(&cap_box, "Has Spindle", profile.has_spindle);
        self.add_check_row(&cap_box, "Has Laser", profile.has_laser);
        self.add_check_row(&cap_box, "Has Coolant", profile.has_coolant);
        
        if profile.has_spindle {
            self.add_entry_row(&cap_box, "Spindle Power (W)", &profile.cnc_spindle_watts);
        }
        if profile.has_laser {
            self.add_entry_row(&cap_box, "Laser Power (W)", &profile.laser_watts);
        }
        
        notebook.append_page(&cap_box, Some(&Label::new(Some("Capabilities"))));

        page.append(&notebook);
        
        // Actions
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_halign(Align::End);
        
        let save_btn = Button::with_label("Save");
        save_btn.add_css_class("suggested-action");
        
        let delete_btn = Button::with_label("Delete");
        delete_btn.add_css_class("destructive-action");
        
        action_box.append(&delete_btn);
        action_box.append(&save_btn);
        
        page.append(&action_box);

        page
    }

    fn add_entry_row(&self, container: &Box, label: &str, value: &str) {
        let row = Box::new(Orientation::Horizontal, 12);
        let lbl = Label::builder().label(label).halign(Align::Start).width_request(150).build();
        let entry = Entry::builder().text(value).hexpand(true).build();
        row.append(&lbl);
        row.append(&entry);
        container.append(&row);
    }

    fn add_combo_row(&self, container: &Box, label: &str, value: &str, options: &[&str]) {
        let row = Box::new(Orientation::Horizontal, 12);
        let lbl = Label::builder().label(label).halign(Align::Start).width_request(150).build();
        let combo = ComboBoxText::new();
        for opt in options {
            combo.append(Some(opt), opt);
            if *opt == value {
                combo.set_active_id(Some(opt));
            }
        }
        row.append(&lbl);
        row.append(&combo);
        container.append(&row);
    }

    fn add_check_row(&self, container: &Box, label: &str, value: bool) {
        let row = Box::new(Orientation::Horizontal, 12);
        let check = CheckButton::with_label(label);
        check.set_active(value);
        row.append(&check);
        container.append(&row);
    }

    fn add_axis_row(&self, container: &Box, label: &str, min: &str, max: &str) {
        let row = Box::new(Orientation::Horizontal, 12);
        let lbl = Label::builder().label(label).halign(Align::Start).width_request(100).build();
        
        let min_lbl = Label::new(Some("Min:"));
        let min_entry = Entry::builder().text(min).width_chars(8).build();
        
        let max_lbl = Label::new(Some("Max:"));
        let max_entry = Entry::builder().text(max).width_chars(8).build();
        
        row.append(&lbl);
        row.append(&min_lbl);
        row.append(&min_entry);
        row.append(&max_lbl);
        row.append(&max_entry);
        container.append(&row);
    }
}
