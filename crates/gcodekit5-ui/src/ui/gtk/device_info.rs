//! # Device Information Panel
//!
//! Displays real-time information about the connected CNC device
//! including firmware version, machine status, position coordinates,
//! and connection state.

use gcodekit5_communication::Communicator;
use gtk4::prelude::*;
use gtk4::{
    accessible::Property as AccessibleProperty, Align, Box, Button, CheckButton, Expander, Image,
    Label, ListBox, ListBoxRow, Orientation, PolicyType, ScrolledWindow, Separator,
};

use crate::device_status;
use crate::t;
use gcodekit5_communication::firmware::grbl::settings::{Setting, SettingsManager};
use gcodekit5_communication::SerialCommunicator;
use gcodekit5_core::{shared, shared_none, Shared, SharedOption, ThreadSafe};
use std::rc::Rc;

#[derive(Clone)]
pub struct CapabilityItem {
    pub name: String,
    pub enabled: bool,
    pub notes: String,
}

// Complex type due to GTK widget and real-time data fields.
#[allow(clippy::type_complexity)]
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
    connected: Shared<bool>,

    // Device type radio buttons
    pub device_type_cnc: CheckButton,
    pub device_type_laser: CheckButton,
    pub device_type_other: CheckButton,
    /// Feedback for device type changes (sent / failed).
    type_status_label: Label,
    /// The device type currently selected ("CNC", "Laser" or "Other").
    current_type: Shared<&'static str>,
    /// True while the selection is changed programmatically (nothing is sent).
    suppress_type_change: Shared<bool>,
    /// True once the selection was initialised from `$32` for this connection.
    type_initialised: Shared<bool>,

    // Communicator for sending commands
    communicator: SharedOption<ThreadSafe<SerialCommunicator>>,

    // Settings manager to update when settings change
    settings_manager: SharedOption<Shared<SettingsManager>>,

    // Callback to notify when settings change (for refreshing settings display)
    on_setting_changed: SharedOption<std::boxed::Box<dyn Fn()>>,
}

impl DeviceInfoView {
    /// Builds the device information panel.
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
        let type_title = Label::new(Some(&t!("Device Type")));
        type_title.add_css_class("mc-group-title");
        type_title.set_halign(Align::Start);
        type_title.set_margin_bottom(4);
        container.append(&type_title);

        let device_type_cnc = CheckButton::with_label(&t!("CNC"));
        device_type_cnc
            .set_tooltip_text(Some(&t!("Spindle-based machining (laser mode off, $32=0)")));
        let device_type_laser = CheckButton::with_label(&t!("Laser"));
        device_type_laser
            .set_tooltip_text(Some(&t!("Laser cutter/engraver (laser mode on, $32=1)")));
        device_type_laser.set_group(Some(&device_type_cnc));
        let device_type_other = CheckButton::with_label(&t!("Other"));
        device_type_other.set_tooltip_text(Some(&t!("Other device type ($32 unchanged)")));
        device_type_other.set_group(Some(&device_type_cnc));
        device_type_other.set_active(true); // Default to Other until $32 is known

        // The type maps to the controller's $32, so it can only change while connected.
        for btn in [&device_type_cnc, &device_type_laser, &device_type_other] {
            btn.set_sensitive(false);
        }

        container.append(&device_type_cnc);
        container.append(&device_type_laser);
        container.append(&device_type_other);

        let type_status_label = Label::new(None);
        type_status_label.add_css_class("caption");
        type_status_label.set_halign(Align::Start);
        type_status_label.set_wrap(true);
        type_status_label.set_visible(false);
        container.append(&type_status_label);

        // Separator after device type
        container.append(&Separator::new(Orientation::Horizontal));

        // Header: status icon + compact action buttons
        let header = Box::new(Orientation::Horizontal, 6);
        header.set_hexpand(true);

        let status_icon = Image::from_icon_name("network-offline-symbolic");
        status_icon.set_pixel_size(18);
        header.append(&status_icon);

        let title = Label::new(Some(&t!("Device")));
        title.add_css_class("mc-group-title");
        title.set_halign(Align::Start);
        title.set_hexpand(true);
        header.append(&title);

        let refresh_btn = Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_tooltip_text(Some(&t!("Refresh device info")));
        refresh_btn.update_property(&[AccessibleProperty::Label(&t!("Refresh device info"))]);
        refresh_btn.set_sensitive(false);

        let copy_btn = Button::from_icon_name("edit-copy-symbolic");
        copy_btn.set_tooltip_text(Some(&t!("Copy device info")));
        copy_btn.update_property(&[AccessibleProperty::Label(&t!("Copy device info"))]);
        copy_btn.set_sensitive(false);

        header.append(&refresh_btn);
        header.append(&copy_btn);

        container.append(&header);

        // Compact info rows
        let device_name_label = Self::add_compact_row(&container, &t!("Name"), &t!("No device"));
        let firmware_label = Self::add_compact_row(&container, &t!("Firmware"), "-");
        let version_label = Self::add_compact_row(&container, &t!("Version"), "-");

        // Capabilities (collapsed by default to keep the sidebar dense)
        let capabilities_list = ListBox::new();
        capabilities_list.add_css_class("boxed-list");

        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&capabilities_list));

        let capabilities_expander = Expander::new(Some(&t!("Capabilities")));
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
            connected: shared(false),
            device_type_cnc: device_type_cnc.clone(),
            device_type_laser: device_type_laser.clone(),
            device_type_other: device_type_other.clone(),
            type_status_label,
            current_type: shared("Other"),
            suppress_type_change: shared(false),
            type_initialised: shared(false),
            communicator: shared_none(),
            settings_manager: shared_none(),
            on_setting_changed: shared_none(),
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

    /// Shows the connection state and firmware details.
    ///
    /// The device type selector is only enabled while connected; on disconnect it is
    /// re-initialised from `$32` at the next connection.
    ///
    /// # Arguments
    /// * `connected` - True when a controller is connected.
    /// * `device_name` - Name shown for the device.
    /// * `firmware` - Firmware type.
    /// * `version` - Firmware version.
    pub fn set_connected(&self, connected: bool, device_name: &str, firmware: &str, version: &str) {
        *self.connected.borrow_mut() = connected;
        for btn in [
            &self.device_type_cnc,
            &self.device_type_laser,
            &self.device_type_other,
        ] {
            btn.set_sensitive(connected);
        }
        if !connected {
            *self.type_initialised.borrow_mut() = false;
            self.type_status_label.set_visible(false);
        }

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
            self.device_name_label.set_text(&t!("No device"));
            self.firmware_label.set_text("-");
            self.firmware_label.remove_css_class("success");
            self.version_label.set_text("-");
            self.version_label.remove_css_class("accent");
            self.refresh_btn.set_sensitive(false);
            self.copy_btn.set_sensitive(false);
            self.capabilities_expander.set_sensitive(false);
        }
    }

    /// Replaces the capabilities list.
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
            "{} {}\n{} {}\n{} {}",
            t!("Device:"),
            self.device_name_label.text(),
            t!("Firmware:"),
            self.firmware_label.text(),
            t!("Version:"),
            self.version_label.text()
        );

        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&config);
        }
    }

    /// Rebuilds the capabilities list from the reported GRBL settings and syncs the
    /// device type selector with `$32`.
    pub fn load_grbl_capabilities_from_status(&self) {
        self.sync_device_type_from_settings();

        let homing = device_status::get_grbl_setting_numeric(22).map(|v| v >= 1.0);
        let laser = device_status::get_grbl_setting_numeric(32).map(|v| v >= 1.0);

        let (homing_enabled, homing_notes) = match homing {
            Some(true) => (true, t!("Enabled ($22=1)").to_string()),
            Some(false) => (false, t!("Disabled ($22=0)").to_string()),
            None => (false, t!("Unknown (settings not loaded)").to_string()),
        };

        let (laser_enabled, laser_notes) = match laser {
            Some(true) => (true, t!("Enabled ($32=1)").to_string()),
            Some(false) => (false, t!("Disabled ($32=0)").to_string()),
            None => (false, t!("Unknown (settings not loaded)").to_string()),
        };

        let capabilities = vec![
            CapabilityItem {
                name: t!("Variable Spindle").to_string(),
                enabled: true,
                notes: t!("PWM spindle control").to_string(),
            },
            CapabilityItem {
                name: t!("Homing Cycle").to_string(),
                enabled: homing_enabled,
                notes: homing_notes,
            },
            CapabilityItem {
                name: t!("Laser Mode").to_string(),
                enabled: laser_enabled,
                notes: laser_notes,
            },
        ];

        self.set_capabilities(capabilities);
    }

    /// Set the communicator for sending commands to the device
    pub fn set_communicator(&self, communicator: ThreadSafe<SerialCommunicator>) {
        *self.communicator.borrow_mut() = Some(communicator);
    }

    /// Set the settings manager for updating settings
    pub fn set_settings_manager(&self, manager: Shared<SettingsManager>) {
        *self.settings_manager.borrow_mut() = Some(manager);
    }

    /// Set a callback to be called when a setting is changed
    pub fn set_on_setting_changed<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.on_setting_changed.borrow_mut() = Some(std::boxed::Box::new(callback));
    }

    /// Selects a device type without sending anything to the controller.
    fn select_type_silently(&self, device_type: &'static str) {
        *self.suppress_type_change.borrow_mut() = true;
        match device_type {
            "CNC" => self.device_type_cnc.set_active(true),
            "Laser" => self.device_type_laser.set_active(true),
            _ => self.device_type_other.set_active(true),
        }
        *self.suppress_type_change.borrow_mut() = false;
        *self.current_type.borrow_mut() = device_type;
    }

    /// Initialises (or corrects) the device type from the controller's `$32`.
    ///
    /// On the first report after connecting, `$32=1` selects Laser and `$32=0` selects
    /// CNC. Later reports only correct a selection that contradicts `$32`, so "Other"
    /// (which also means laser mode off) is kept.
    fn sync_device_type_from_settings(&self) {
        if !*self.connected.borrow() {
            return;
        }
        let Some(laser) = device_status::get_grbl_setting_numeric(32).map(|v| v >= 1.0) else {
            return;
        };
        let current = *self.current_type.borrow();
        let initialised = *self.type_initialised.borrow();
        let wanted = match (laser, initialised, current) {
            (true, _, "Laser") => None,
            (true, _, _) => Some("Laser"),
            (false, false, _) => Some("CNC"),
            (false, true, "Laser") => Some("CNC"),
            (false, true, _) => None,
        };
        *self.type_initialised.borrow_mut() = true;
        if let Some(device_type) = wanted {
            self.select_type_silently(device_type);
        }
    }

    /// Shows feedback below the device type selector.
    fn show_type_status(&self, message: &str, is_error: bool) {
        self.type_status_label.set_text(message);
        self.type_status_label.set_visible(true);
        if is_error {
            self.type_status_label.add_css_class("error");
        } else {
            self.type_status_label.remove_css_class("error");
        }
    }

    /// Sends `$32=<value>` to the controller.
    ///
    /// # Returns
    /// An error message if the command could not be sent.
    fn send_laser_mode(&self, value: &str) -> Result<(), String> {
        // GRBL rejects `$` settings unless idle (or in alarm), and a line sent while a job
        // streams would corrupt the job's buffer accounting.
        if let Some(reason) = device_status::settings_command_blocked() {
            return Err(format!("{} {reason}", t!("cannot change $32:")));
        }
        let comm = self
            .communicator
            .borrow()
            .clone()
            .ok_or_else(|| t!("no connection").to_string())?;
        // Blocking lock: the status poller holds it only briefly, and a dropped change
        // would leave the UI and the controller out of sync.
        let mut comm = comm.lock();
        if !comm.is_connected() {
            return Err(t!("not connected").to_string());
        }
        comm.send_command(&format!("$32={value}"))
            .map_err(|e| e.to_string())
    }

    /// Called when a device type radio button becomes active.
    fn on_device_type_changed(&self, device_type: &'static str) {
        if *self.suppress_type_change.borrow() {
            return;
        }
        let previous = *self.current_type.borrow();
        if !*self.connected.borrow() {
            // The selector is insensitive while disconnected; never let it drift.
            self.select_type_silently(previous);
            return;
        }

        let laser_mode_value = match device_type {
            "CNC" => "0",
            "Laser" => "1",
            // "Other" leaves $32 unchanged.
            _ => {
                *self.current_type.borrow_mut() = device_type;
                return;
            }
        };

        if let Err(e) = self.send_laser_mode(laser_mode_value) {
            tracing::warn!(value = laser_mode_value, error = %e, "Failed to set $32");
            self.show_type_status(&format!("{} {e}", t!("Could not set laser mode:")), true);
            self.select_type_silently(previous);
            return;
        }

        *self.current_type.borrow_mut() = device_type;
        self.show_type_status(&format!("{} $32={laser_mode_value}", t!("Sent")), false);

        // Update the cached setting value immediately in device_status.
        device_status::update_grbl_setting(32, laser_mode_value.to_string());

        // Also update the settings_manager so the settings list displays correctly.
        if let Some(ref manager_rc) = *self.settings_manager.borrow() {
            let mut manager = manager_rc.borrow_mut();
            let setting = Setting {
                number: 32,
                value: laser_mode_value.to_string(),
                name: "Laser Mode".to_string(),
                description: "Enable laser mode".to_string(),
                numeric_value: laser_mode_value.parse::<f64>().ok(),
                range: Some((0.0, 1.0)),
                read_only: false,
                unit: Some("bool".to_string()),
            };
            manager.set_setting(setting);
        }

        // Notify that a setting has changed (triggers settings list refresh).
        if let Some(ref callback) = *self.on_setting_changed.borrow() {
            callback();
        }
        self.load_grbl_capabilities_from_status();
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
            type_status_label: self.type_status_label.clone(),
            current_type: self.current_type.clone(),
            suppress_type_change: self.suppress_type_change.clone(),
            type_initialised: self.type_initialised.clone(),
            communicator: self.communicator.clone(),
            settings_manager: self.settings_manager.clone(),
            on_setting_changed: self.on_setting_changed.clone(),
        }
    }
}
