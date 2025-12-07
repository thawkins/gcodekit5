use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Label, ListBox, ListBoxRow, Orientation, PolicyType, ScrolledWindow, Separator};
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
    status_icon: Label,
    device_name_label: Label,
    firmware_label: Label,
    version_label: Label,
    refresh_btn: Button,
    copy_btn: Button,
    capabilities_list: ListBox,
    placeholder: Box,
    capabilities_container: ScrolledWindow,
    connected: Rc<RefCell<bool>>,
}

impl DeviceInfoView {
    pub fn new() -> Rc<Self> {
        let container = Box::new(Orientation::Horizontal, 0);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Sidebar
        let sidebar = Box::new(Orientation::Vertical, 10);
        sidebar.set_width_request(250);
        sidebar.set_margin_top(15);
        sidebar.set_margin_bottom(15);
        sidebar.set_margin_start(15);
        sidebar.set_margin_end(15);
        sidebar.add_css_class("sidebar");

        // Status Icon
        let status_icon = Label::new(Some("🔌"));
        status_icon.set_css_classes(&["status-icon-large"]);
        status_icon.set_height_request(100);
        sidebar.append(&status_icon);

        // Device Info
        let device_name_label = Self::add_info_row(&sidebar, "DEVICE NAME", "No Device");
        let firmware_label = Self::add_info_row(&sidebar, "FIRMWARE", "-");
        let version_label = Self::add_info_row(&sidebar, "VERSION", "-");

        sidebar.append(&Separator::new(Orientation::Horizontal));

        // Actions
        let refresh_btn = Button::with_label("🔄 Refresh Info");
        refresh_btn.set_tooltip_text(Some("Refresh Device Information"));
        refresh_btn.set_sensitive(false);
        sidebar.append(&refresh_btn);

        let copy_btn = Button::with_label("📋 Copy Config");
        copy_btn.set_tooltip_text(Some("Copy Configuration to Clipboard"));
        copy_btn.set_sensitive(false);
        sidebar.append(&copy_btn);

        // Spacer
        let spacer = Box::new(Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        sidebar.append(&spacer);

        container.append(&sidebar);
        container.append(&Separator::new(Orientation::Vertical));

        // Main Content
        let main_content = Box::new(Orientation::Vertical, 15);
        main_content.set_hexpand(true);
        main_content.set_vexpand(true);
        main_content.set_margin_top(20);
        main_content.set_margin_bottom(20);
        main_content.set_margin_start(20);
        main_content.set_margin_end(20);

        let title = Label::new(Some("Firmware Capabilities"));
        title.set_css_classes(&["title-2"]);
        title.set_halign(Align::Start);
        main_content.append(&title);

        // Placeholder for not connected
        let placeholder = Box::new(Orientation::Vertical, 0);
        placeholder.set_vexpand(true);
        placeholder.set_valign(Align::Center);
        let placeholder_label = Label::new(Some("Connect a device to view capabilities"));
        placeholder_label.add_css_class("dim-label");
        placeholder_label.set_css_classes(&["title-3", "dim-label"]);
        placeholder.append(&placeholder_label);

        // Capabilities list
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_visible(false);

        let capabilities_list = ListBox::new();
        capabilities_list.add_css_class("boxed-list");
        scroll.set_child(Some(&capabilities_list));

        main_content.append(&placeholder);
        main_content.append(&scroll);

        container.append(&main_content);

        let view = Rc::new(Self {
            container,
            status_icon,
            device_name_label,
            firmware_label,
            version_label,
            refresh_btn: refresh_btn.clone(),
            copy_btn: copy_btn.clone(),
            capabilities_list,
            placeholder,
            capabilities_container: scroll,
            connected: Rc::new(RefCell::new(false)),
        });

        // Connect signals
        let view_clone = view.clone();
        refresh_btn.connect_clicked(move |_| {
            view_clone.refresh_info();
        });

        let view_clone = view.clone();
        copy_btn.connect_clicked(move |_| {
            view_clone.copy_config();
        });

        view
    }

    fn add_info_row(container: &Box, label: &str, value: &str) -> Label {
        let vbox = Box::new(Orientation::Vertical, 5);
        let lbl = Label::new(Some(label));
        lbl.add_css_class("caption");
        lbl.add_css_class("dim-label");
        lbl.set_xalign(0.0);
        vbox.append(&lbl);

        let val = Label::new(Some(value));
        val.add_css_class("heading");
        val.set_xalign(0.0);
        val.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        vbox.append(&val);

        container.append(&vbox);
        val
    }

    pub fn set_connected(&self, connected: bool, device_name: &str, firmware: &str, version: &str) {
        *self.connected.borrow_mut() = connected;
        
        if connected {
            self.status_icon.set_text("📠");
            self.device_name_label.set_text(device_name);
            self.firmware_label.set_text(firmware);
            self.firmware_label.add_css_class("success");
            self.version_label.set_text(version);
            self.version_label.add_css_class("accent");
            self.refresh_btn.set_sensitive(true);
            self.copy_btn.set_sensitive(true);
            self.placeholder.set_visible(false);
            self.capabilities_container.set_visible(true);
        } else {
            self.status_icon.set_text("🔌");
            self.device_name_label.set_text("No Device");
            self.firmware_label.set_text("-");
            self.firmware_label.remove_css_class("success");
            self.version_label.set_text("-");
            self.version_label.remove_css_class("accent");
            self.refresh_btn.set_sensitive(false);
            self.copy_btn.set_sensitive(false);
            self.placeholder.set_visible(true);
            self.capabilities_container.set_visible(false);
        }
    }

    pub fn set_capabilities(&self, capabilities: Vec<CapabilityItem>) {
        // Clear existing
        while let Some(child) = self.capabilities_list.first_child() {
            self.capabilities_list.remove(&child);
        }

        for (index, cap) in capabilities.iter().enumerate() {
            let row = self.create_capability_row(cap, index);
            self.capabilities_list.append(&row);
        }
    }

    fn create_capability_row(&self, cap: &CapabilityItem, index: usize) -> ListBoxRow {
        let row = ListBoxRow::new();
        row.set_activatable(false);
        if index % 2 == 0 {
            row.add_css_class("alternating-row");
        }

        let hbox = Box::new(Orientation::Horizontal, 20);
        hbox.set_margin_start(20);
        hbox.set_margin_end(10);
        hbox.set_margin_top(10);
        hbox.set_margin_bottom(10);

        // Status pill
        let status_box = Box::new(Orientation::Horizontal, 0);
        status_box.set_width_request(80);
        status_box.set_halign(Align::Start);

        let status_label = Label::new(Some(if cap.enabled { "ENABLED" } else { "DISABLED" }));
        status_label.add_css_class("caption");
        status_label.add_css_class(if cap.enabled { "success" } else { "error" });
        status_label.set_width_chars(8);
        status_label.set_xalign(0.5);
        
        let status_frame = Box::new(Orientation::Horizontal, 0);
        status_frame.append(&status_label);
        status_frame.add_css_class("pill");
        if cap.enabled {
            status_frame.add_css_class("success-bg");
        } else {
            status_frame.add_css_class("error-bg");
        }
        status_box.append(&status_frame);
        hbox.append(&status_box);

        // Capability name
        let name_label = Label::new(Some(&cap.name));
        name_label.set_width_request(250);
        name_label.set_xalign(0.0);
        name_label.add_css_class("heading");
        hbox.append(&name_label);

        // Notes
        let notes_label = Label::new(Some(&cap.notes));
        notes_label.set_hexpand(true);
        notes_label.set_xalign(0.0);
        notes_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        notes_label.add_css_class("dim-label");
        hbox.append(&notes_label);

        row.set_child(Some(&hbox));
        row
    }

    fn refresh_info(&self) {
        // This would query device for updated info
        // For now, reload sample capabilities
        self.load_sample_capabilities();
    }

    fn copy_config(&self) {
        // Copy firmware configuration to clipboard
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

    pub fn load_sample_capabilities(&self) {
        let capabilities = vec![
            CapabilityItem {
                name: "Variable Spindle".to_string(),
                enabled: true,
                notes: "PWM spindle control supported".to_string(),
            },
            CapabilityItem {
                name: "Homing Cycle".to_string(),
                enabled: true,
                notes: "Automatic homing enabled".to_string(),
            },
            CapabilityItem {
                name: "Limit Switches".to_string(),
                enabled: true,
                notes: "Hard limits configured".to_string(),
            },
            CapabilityItem {
                name: "Probe Support".to_string(),
                enabled: true,
                notes: "Tool length probe available".to_string(),
            },
            CapabilityItem {
                name: "Laser Mode".to_string(),
                enabled: false,
                notes: "Not configured".to_string(),
            },
            CapabilityItem {
                name: "Parking".to_string(),
                enabled: false,
                notes: "Not enabled".to_string(),
            },
            CapabilityItem {
                name: "Sleep Mode".to_string(),
                enabled: true,
                notes: "Idle power saving enabled".to_string(),
            },
            CapabilityItem {
                name: "Soft Limits".to_string(),
                enabled: true,
                notes: "Software travel limits active".to_string(),
            },
        ];

        self.set_capabilities(capabilities);
    }
}
