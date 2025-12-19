use gtk4::prelude::*;
use gtk4::{
    accessible::Property as AccessibleProperty, Align, Box, Button, CheckButton, Expander, Image,
    Label, ListBox, ListBoxRow, Orientation, PolicyType, ScrolledWindow, Separator,
};

use crate::device_status;
use gcodekit5_communication::firmware::grbl::settings::{Setting, SettingsManager};
use gcodekit5_communication::SerialCommunicator;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
    
    // Device type radio buttons
    pub device_type_cnc: CheckButton,
    pub device_type_laser: CheckButton,
    pub device_type_other: CheckButton,
    
    // Communicator for sending commands
    communicator: Rc<RefCell<Option<Arc<Mutex<SerialCommunicator>>>>>,
    
    // Settings manager to update when settings change
    settings_manager: Rc<RefCell<Option<Rc<RefCell<SettingsManager>>>>>,
    
    // Callback to notify when settings change (for refreshing settings display)
    on_setting_changed: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
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

        // Device Type Section (at the very top)
        let type_title = Label::new(Some("Device Type"));
        type_title.add_css_class("mc-group-title");
        type_title.set_halign(Align::Start);
        type_title.set_margin_bottom(4);
        container.append(&type_title);
        
        let device_type_cnc = CheckButton::with_label("CNC");
        device_type_cnc.set_tooltip_text(Some("Spindle-based machining"));
        let device_type_laser = CheckButton::with_label("Laser");
        device_type_laser.set_tooltip_text(Some("Laser cutter/engraver"));
        device_type_laser.set_group(Some(&device_type_cnc));
        let device_type_other = CheckButton::with_label("Other");
        device_type_other.set_tooltip_text(Some("Other device type"));
        device_type_other.set_group(Some(&device_type_cnc));
        device_type_other.set_active(true); // Default to Other
        
        container.append(&device_type_cnc);
        container.append(&device_type_laser);
        container.append(&device_type_other);
        
        // Separator after device type
        container.append(&Separator::new(Orientation::Horizontal));

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
            device_type_cnc: device_type_cnc.clone(),
            device_type_laser: device_type_laser.clone(),
            device_type_other: device_type_other.clone(),
            communicator: Rc::new(RefCell::new(None)),
            settings_manager: Rc::new(RefCell::new(None)),
            on_setting_changed: Rc::new(RefCell::new(None)),
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
        
        // Connect device type radio buttons
        {
            let view_clone = view.clone();
            device_type_cnc.connect_toggled(move |btn| {
                if btn.is_active() {
                    view_clone.on_device_type_changed("CNC");
                }
            });
        }
        
        {
            let view_clone = view.clone();
            device_type_laser.connect_toggled(move |btn| {
                if btn.is_active() {
                    view_clone.on_device_type_changed("Laser");
                }
            });
        }
        
        {
            let view_clone = view.clone();
            device_type_other.connect_toggled(move |btn| {
                if btn.is_active() {
                    view_clone.on_device_type_changed("Other");
                }
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
    
    /// Set the communicator for sending commands to the device
    pub fn set_communicator(&self, communicator: Arc<Mutex<SerialCommunicator>>) {
        *self.communicator.borrow_mut() = Some(communicator);
    }
    
    /// Set the settings manager for updating settings
    pub fn set_settings_manager(&self, manager: Rc<RefCell<SettingsManager>>) {
        *self.settings_manager.borrow_mut() = Some(manager);
    }
    
    /// Set a callback to be called when a setting is changed
    pub fn set_on_setting_changed<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.on_setting_changed.borrow_mut() = Some(std::boxed::Box::new(callback));
    }
    
    /// Called when device type radio button changes
    fn on_device_type_changed(&self, device_type: &str) {
        if !*self.connected.borrow() {
            return;
        }
        
        // Determine $32 value based on device type
        let laser_mode_value = match device_type {
            "CNC" => "0",      // Disable laser mode for CNC
            "Laser" => "1",    // Enable laser mode for Laser
            "Other" => return, // Don't change $32 for Other
            _ => return,
        };
        
        // Send $32 command if we have a communicator
        if let Some(ref comm) = *self.communicator.borrow() {
            let command = format!("$32={}", laser_mode_value);
            
            if let Ok(mut comm_lock) = comm.try_lock() {
                // Send command using Communicator trait method
                use gcodekit5_communication::Communicator;
                if Communicator::send_command(&mut *comm_lock, &command).is_ok() {
                    // Update the cached setting value immediately in device_status
                    use crate::device_status;
                    device_status::update_grbl_setting(32, laser_mode_value.to_string());
                    
                    // Also update the settings_manager so the settings list displays correctly
                    if let Some(ref manager_rc) = *self.settings_manager.borrow() {
                        let mut manager = manager_rc.borrow_mut();
                        let setting = Setting {
                            number: 32,
                            value: laser_mode_value.to_string(),
                            name: "Laser Mode".to_string(),
                            description: "Enable laser mode".to_string(),
                            numeric_value: Some(laser_mode_value.parse::<f64>().unwrap_or(0.0)),
                            range: Some((0.0, 1.0)),
                            read_only: false,
                        };
                        manager.set_setting(setting);
                    }
                    
                    // Notify that a setting has changed (triggers settings list refresh)
                    if let Some(ref callback) = *self.on_setting_changed.borrow() {
                        callback();
                    }
                    
                    // Refresh the capabilities display to show updated $32
                    let view_clone = Rc::new(self.clone());
                    glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                        view_clone.load_grbl_capabilities_from_status();
                    });
                }
            }
        }
    }
}

// Implement Clone for DeviceInfoView so we can use it in callbacks
impl Clone for DeviceInfoView {
    fn clone(&self) -> Self {
        Self {
            container: self.container.clone(),
            status_icon: self.status_icon.clone(),
            device_name_label: self.device_name_label.clone(),
            firmware_label: self.firmware_label.clone(),
            version_label: self.version_label.clone(),
            refresh_btn: self.refresh_btn.clone(),
            copy_btn: self.copy_btn.clone(),
            capabilities_list: self.capabilities_list.clone(),
            capabilities_expander: self.capabilities_expander.clone(),
            connected: self.connected.clone(),
            device_type_cnc: self.device_type_cnc.clone(),
            device_type_laser: self.device_type_laser.clone(),
            device_type_other: self.device_type_other.clone(),
            communicator: self.communicator.clone(),
            settings_manager: self.settings_manager.clone(),
            on_setting_changed: self.on_setting_changed.clone(),
        }
    }
}
