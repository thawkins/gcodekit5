use gtk4::prelude::*;
use gtk4::{
    accessible::Property as AccessibleProperty, Align, Box, Button, Expander, Image, Label,
    ListBox, ListBoxRow, Orientation, PolicyType, ScrolledWindow,
};

use crate::device_status;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct CapabilityItem {
    pub name: String,
    pub enabled: bool,
    pub notes: String,
}

pub struct DeviceInfoView {
    pub container: Box,
    status_icon: Image,
    device_name_label: Label,
    firmware_label: Label,
    version_label: Label,
    refresh_btn: Button,
    copy_btn: Button,
    capabilities_list: ListBox,
    capabilities_expander: Expander,
    connected: Rc<RefCell<bool>>,
}

impl DeviceInfoView {
    pub fn new() -> Rc<Self> {
        let container = Box::new(Orientation::Vertical, 8);
        container.set_hexpand(true);
        container.set_vexpand(true);
        container.set_margin_top(10);
        container.set_margin_bottom(10);
        container.set_margin_start(10);
        container.set_margin_end(10);
        container.add_css_class("sidebar");

        // Header: status icon + compact action buttons
        let header = Box::new(Orientation::Horizontal, 6);
        header.set_hexpand(true);

        let status_icon = Image::from_icon_name("network-offline-symbolic");
        status_icon.set_pixel_size(18);
        header.append(&status_icon);

        let title = Label::new(Some("Device"));
        title.add_css_class("mc-group-title");
        title.set_halign(Align::Start);
        title.set_hexpand(true);
        header.append(&title);

        let refresh_btn = Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_tooltip_text(Some("Refresh device info"));
        refresh_btn.update_property(&[AccessibleProperty::Label("Refresh device info")]);
        refresh_btn.set_sensitive(false);

        let copy_btn = Button::from_icon_name("edit-copy-symbolic");
        copy_btn.set_tooltip_text(Some("Copy device info"));
        copy_btn.update_property(&[AccessibleProperty::Label("Copy device info")]);
        copy_btn.set_sensitive(false);

        header.append(&refresh_btn);
        header.append(&copy_btn);

        container.append(&header);

        // Compact info rows
        let device_name_label = Self::add_compact_row(&container, "Name", "No device");
        let firmware_label = Self::add_compact_row(&container, "Firmware", "-");
        let version_label = Self::add_compact_row(&container, "Version", "-");

        // Capabilities (collapsed by default to keep the sidebar dense)
        let capabilities_list = ListBox::new();
        capabilities_list.add_css_class("boxed-list");

        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&capabilities_list));

        let capabilities_expander = Expander::new(Some("Capabilities"));
        capabilities_expander.set_expanded(false);
        capabilities_expander.set_child(Some(&scroll));
        capabilities_expander.set_sensitive(false);
        container.append(&capabilities_expander);

        let view = Rc::new(Self {
            container,
            status_icon,
            device_name_label,
            firmware_label,
            version_label,
            refresh_btn: refresh_btn.clone(),
            copy_btn: copy_btn.clone(),
            capabilities_list,
            capabilities_expander,
            connected: Rc::new(RefCell::new(false)),
        });

        // Connect signals
        {
            let view_clone = view.clone();
            refresh_btn.connect_clicked(move |_| {
                view_clone.refresh_info();
            });
        }

        {
            let view_clone = view.clone();
            copy_btn.connect_clicked(move |_| {
                view_clone.copy_config();
            });
        }

        view
    }

    fn add_compact_row(container: &Box, label: &str, value: &str) -> Label {
        let row = Box::new(Orientation::Horizontal, 6);
        row.set_hexpand(true);

        let lbl = Label::new(Some(label));
        lbl.add_css_class("caption");
        lbl.add_css_class("dim-label");
        lbl.set_xalign(0.0);
        lbl.set_width_chars(10);
        row.append(&lbl);

        let val = Label::new(Some(value));
        val.set_hexpand(true);
        val.set_xalign(0.0);
        val.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        row.append(&val);

        container.append(&row);
        val
    }

    pub fn set_connected(&self, connected: bool, device_name: &str, firmware: &str, version: &str) {
        *self.connected.borrow_mut() = connected;

        if connected {
            self.status_icon
                .set_icon_name(Some("network-transmit-receive-symbolic"));
            self.device_name_label.set_text(device_name);
            self.firmware_label.set_text(firmware);
            self.firmware_label.add_css_class("success");
            self.version_label.set_text(version);
            self.version_label.add_css_class("accent");
            self.refresh_btn.set_sensitive(true);
            self.copy_btn.set_sensitive(true);
            self.capabilities_expander.set_sensitive(true);
        } else {
            self.status_icon
                .set_icon_name(Some("network-offline-symbolic"));
            self.device_name_label.set_text("No device");
            self.firmware_label.set_text("-");
            self.firmware_label.remove_css_class("success");
            self.version_label.set_text("-");
            self.version_label.remove_css_class("accent");
            self.refresh_btn.set_sensitive(false);
            self.copy_btn.set_sensitive(false);
            self.capabilities_expander.set_sensitive(false);
        }
    }

    pub fn set_capabilities(&self, capabilities: Vec<CapabilityItem>) {
        while let Some(child) = self.capabilities_list.first_child() {
            self.capabilities_list.remove(&child);
        }

        for cap in capabilities {
            self.capabilities_list
                .append(&Self::create_capability_row(&cap));
        }
    }

    fn create_capability_row(cap: &CapabilityItem) -> ListBoxRow {
        let row = ListBoxRow::new();
        row.set_activatable(false);

        let hbox = Box::new(Orientation::Horizontal, 8);
        hbox.set_margin_start(10);
        hbox.set_margin_end(10);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);

        let icon = Image::from_icon_name(if cap.enabled {
            "emblem-ok-symbolic"
        } else {
            "dialog-error-symbolic"
        });
        icon.set_pixel_size(14);
        hbox.append(&icon);

        let name_label = Label::new(Some(&cap.name));
        name_label.set_xalign(0.0);
        name_label.set_hexpand(true);
        hbox.append(&name_label);

        let notes_label = Label::new(Some(&cap.notes));
        notes_label.set_xalign(0.0);
        notes_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        notes_label.add_css_class("dim-label");
        notes_label.set_hexpand(true);
        hbox.append(&notes_label);

        row.set_child(Some(&hbox));
        row
    }

    fn refresh_info(&self) {
        self.load_grbl_capabilities_from_status();
    }

    fn copy_config(&self) {
        let config = format!(
            "Device: {}\nFirmware: {}\nVersion: {}",
            self.device_name_label.text(),
            self.firmware_label.text(),
            self.version_label.text()
        );

        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&config);
        }
    }

    pub fn load_grbl_capabilities_from_status(&self) {
        let homing = device_status::get_grbl_setting_numeric(22).map(|v| v >= 1.0);
        let laser = device_status::get_grbl_setting_numeric(32).map(|v| v >= 1.0);

        let (homing_enabled, homing_notes) = match homing {
            Some(true) => (true, "Enabled ($22=1)".to_string()),
            Some(false) => (false, "Disabled ($22=0)".to_string()),
            None => (false, "Unknown (settings not loaded)".to_string()),
        };

        let (laser_enabled, laser_notes) = match laser {
            Some(true) => (true, "Enabled ($32=1)".to_string()),
            Some(false) => (false, "Disabled ($32=0)".to_string()),
            None => (false, "Unknown (settings not loaded)".to_string()),
        };

        let capabilities = vec![
            CapabilityItem {
                name: "Variable Spindle".to_string(),
                enabled: true,
                notes: "PWM spindle control".to_string(),
            },
            CapabilityItem {
                name: "Homing Cycle".to_string(),
                enabled: homing_enabled,
                notes: homing_notes,
            },
            CapabilityItem {
                name: "Laser Mode".to_string(),
                enabled: laser_enabled,
                notes: laser_notes,
            },
        ];

        self.set_capabilities(capabilities);
    }
}
