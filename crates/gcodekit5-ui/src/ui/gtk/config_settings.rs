use gcodekit5_communication::firmware::grbl::settings::{Setting, SettingsManager};
use gcodekit5_communication::{Communicator, SerialCommunicator};
use gcodekit5_settings::controller::SettingsController;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, ComboBoxText, Dialog, DialogFlags, Entry, Grid, Label, ListBox, ListBoxRow,
    Orientation, PolicyType, ResponseType, ScrolledWindow, SearchEntry, Separator,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ConfigSettingRow {
    pub number: u8,
    pub name: String,
    pub value: String,
    pub unit: String,
    pub description: String,
    pub category: String,
    pub read_only: bool,
}

impl From<&Setting> for ConfigSettingRow {
    fn from(s: &Setting) -> Self {
        let category = match s.number {
            0..=20 => "System",
            21..=32 => "Homing",
            33..=39 => "Spindle",
            100..=102 => "Steps Per Unit",
            110..=112 => "Max Rate",
            120..=122 => "Acceleration",
            130..=132 => "Max Travel",
            _ => "Other",
        };

        Self {
            number: s.number,
            name: s.name.clone(),
            value: s.value.clone(),
            unit: String::new(),
            description: s.description.clone(),
            category: category.to_string(),
            read_only: s.read_only,
        }
    }
}

use crate::device_status;
use crate::ui::gtk::device_console::DeviceConsoleView;
use crate::ui::gtk::device_info::DeviceInfoView;
use crate::ui::gtk::help_browser;

pub struct ConfigSettingsView {
    pub container: Box,
    pub device_info_view: Rc<DeviceInfoView>,
    #[allow(dead_code)]
    settings_controller: Rc<SettingsController>,
    settings_manager: Rc<RefCell<SettingsManager>>,
    last_synced_settings_count: Cell<usize>,
    last_persisted_settings_count: Cell<usize>,
    device_manager: RefCell<Option<std::sync::Arc<gcodekit5_devicedb::DeviceManager>>>,

    settings_list: ListBox,
    search_entry: SearchEntry,
    category_filter: ComboBoxText,
    status_label: Label,
    reload_btn: Button,
    save_btn: Button,
    restore_btn: Button,
    communicator: Rc<RefCell<Option<Arc<Mutex<SerialCommunicator>>>>>,
    device_console: Rc<RefCell<Option<Rc<DeviceConsoleView>>>>,
}

impl ConfigSettingsView {
    pub fn new(settings_controller: Rc<SettingsController>) -> Rc<Self> {
        // Outer container splits into left (Device Info) and right (Config Settings)
        let outer = Box::new(Orientation::Horizontal, 10);
        outer.set_hexpand(true);
        outer.set_vexpand(true);
        outer.set_margin_top(10);
        outer.set_margin_bottom(10);
        outer.set_margin_start(10);
        outer.set_margin_end(10);

        // Left panel - Device Info (embedded)
        let left_panel = Box::new(Orientation::Vertical, 0);
        left_panel.set_width_request(320);
        left_panel.set_margin_top(10);
        left_panel.set_margin_bottom(10);
        left_panel.set_margin_start(10);
        left_panel.set_margin_end(10);

        // Create DeviceInfoView and add to left panel
        let device_info_view = DeviceInfoView::new();
        left_panel.append(&device_info_view.container);

        // Separator
        let sep = Separator::new(Orientation::Vertical);

        // Right panel - Config Settings content
        let container = Box::new(Orientation::Vertical, 10);
        container.set_hexpand(true);
        container.set_vexpand(true);
        container.set_margin_top(10);
        container.set_margin_bottom(10);
        container.set_margin_start(10);
        container.set_margin_end(10);

        // Toolbar
        let toolbar = Box::new(Orientation::Horizontal, 10);

        let reload_btn = Button::with_label("Retrieve");
        reload_btn.set_tooltip_text(Some("Retrieve Settings from Device ($$)"));
        toolbar.append(&reload_btn);

        let save_btn = Button::with_label("Save");
        save_btn.set_tooltip_text(Some("Save Settings to File"));
        save_btn.set_sensitive(false);
        toolbar.append(&save_btn);

        let load_btn = Button::with_label("Load");
        load_btn.set_tooltip_text(Some("Load Settings from File"));
        toolbar.append(&load_btn);

        let restore_btn = Button::with_label("Restore");
        restore_btn.set_tooltip_text(Some("Restore Settings to Device"));
        restore_btn.set_sensitive(false);
        toolbar.append(&restore_btn);

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        toolbar.append(&spacer);
        toolbar.append(&help_browser::make_help_button("device_config"));

        container.append(&toolbar);

        // Filter bar (right panel)
        let filter_bar = Box::new(Orientation::Horizontal, 10);

        let filter_label = Label::new(Some("Filter:"));
        filter_bar.append(&filter_label);

        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search settings..."));
        search_entry.set_hexpand(true);
        filter_bar.append(&search_entry);

        let category_label = Label::new(Some("Category:"));
        filter_bar.append(&category_label);

        let category_filter = ComboBoxText::new();
        category_filter.append_text("All");
        category_filter.append_text("Steps Per Unit");
        category_filter.append_text("Max Rate");
        category_filter.append_text("Acceleration");
        category_filter.append_text("Max Travel");
        category_filter.append_text("Homing");
        category_filter.append_text("System");
        category_filter.append_text("Spindle");
        category_filter.append_text("Other");
        category_filter.set_active(Some(0));
        filter_bar.append(&category_filter);

        container.append(&filter_bar);

        // Settings List Header
        let header = Box::new(Orientation::Horizontal, 5);
        header.add_css_class("list-header");
        header.set_margin_start(5);
        header.set_margin_end(5);

        let id_lbl = Label::new(Some("ID"));
        id_lbl.set_width_request(50);
        id_lbl.set_xalign(0.0);
        id_lbl.add_css_class("heading");
        header.append(&id_lbl);

        let name_lbl = Label::new(Some("Name"));
        name_lbl.set_width_request(200);
        name_lbl.set_xalign(0.0);
        name_lbl.add_css_class("heading");
        header.append(&name_lbl);

        let value_lbl = Label::new(Some("Value"));
        value_lbl.set_width_request(100);
        value_lbl.set_xalign(0.0);
        value_lbl.add_css_class("heading");
        header.append(&value_lbl);

        let unit_lbl = Label::new(Some("Unit"));
        unit_lbl.set_width_request(80);
        unit_lbl.set_xalign(0.0);
        unit_lbl.add_css_class("heading");
        header.append(&unit_lbl);

        let cat_lbl = Label::new(Some("Category"));
        cat_lbl.set_width_request(150);
        cat_lbl.set_xalign(0.0);
        cat_lbl.add_css_class("heading");
        header.append(&cat_lbl);

        let desc_lbl = Label::new(Some("Description"));
        desc_lbl.set_hexpand(true);
        desc_lbl.set_xalign(0.0);
        desc_lbl.add_css_class("heading");
        header.append(&desc_lbl);

        container.append(&header);

        // Settings List
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);

        let settings_list = ListBox::new();
        settings_list.add_css_class("boxed-list");
        settings_list.set_activate_on_single_click(true);
        scroll.set_child(Some(&settings_list));
        container.append(&scroll);

        // Status Bar (right panel)
        let status_bar = Box::new(Orientation::Horizontal, 10);
        status_bar.add_css_class("status-bar");
        status_bar.set_margin_start(5);
        status_bar.set_margin_end(5);

        let status_label = Label::new(Some("Not connected"));
        status_label.set_xalign(0.0);
        status_bar.append(&status_label);

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        status_bar.append(&spacer);

        let count_label = Label::new(Some("0 / 0 settings"));
        count_label.add_css_class("dim-label");
        status_bar.append(&count_label);

        container.append(&status_bar);

        // Construct view with device_info embedded
        let settings_manager = Rc::new(RefCell::new(SettingsManager::new()));

        outer.append(&left_panel);
        outer.append(&sep);
        outer.append(&container);

        let view = Rc::new(Self {
            container: outer,
            device_info_view: device_info_view.clone(),
            settings_controller,
            settings_manager: settings_manager.clone(),
            last_synced_settings_count: Cell::new(0),
            last_persisted_settings_count: Cell::new(0),
            device_manager: RefCell::new(None),
            settings_list: settings_list.clone(),
            search_entry: search_entry.clone(),
            category_filter: category_filter.clone(),
            status_label: status_label.clone(),
            reload_btn: reload_btn.clone(),
            save_btn: save_btn.clone(),
            restore_btn: restore_btn.clone(),
            communicator: Rc::new(RefCell::new(None)),
            device_console: Rc::new(RefCell::new(None)),
        });

        // Set up callback from device_info_view to refresh settings display
        {
            let view_clone = view.clone();
            device_info_view.set_on_setting_changed(move || {
                view_clone.refresh_display();
            });
        }
        
        // Pass settings_manager to device_info_view so it can update settings
        device_info_view.set_settings_manager(settings_manager.clone());

        // Connect signals
        let view_clone = view.clone();
        search_entry.connect_search_changed(move |_| {
            view_clone.apply_filter();
        });

        let view_clone = view.clone();
        category_filter.connect_changed(move |_| {
            view_clone.apply_filter();
        });

        let view_clone = view.clone();
        reload_btn.connect_clicked(move |_| {
            view_clone.retrieve_settings();
        });

        let view_clone = view.clone();
        save_btn.connect_clicked(move |_| {
            view_clone.save_to_file();
        });

        let view_clone = view.clone();
        load_btn.connect_clicked(move |_| {
            view_clone.load_from_file();
        });

        let view_clone = view.clone();
        restore_btn.connect_clicked(move |_| {
            view_clone.restore_to_device();
        });

        // Connect ListBox row-activated signal to handle editing
        let view_clone = view.clone();
        settings_list.connect_row_activated(move |_listbox, row| {
            // Get the row index
            let index = row.index();
            if index >= 0 {
                // Get the setting at this index
                let manager = view_clone.settings_manager.borrow();
                let mut all_settings: Vec<ConfigSettingRow> = manager
                    .get_all_settings()
                    .iter()
                    .map(|s| ConfigSettingRow::from(*s))
                    .collect();
                all_settings.sort_by_key(|s| s.number);

                // Apply same filtering as display
                let search_text = view_clone.search_entry.text().to_string().to_lowercase();
                let category = view_clone
                    .category_filter
                    .active_text()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "All".to_string());

                let filtered_settings: Vec<ConfigSettingRow> = all_settings
                    .into_iter()
                    .filter(|setting| {
                        // Apply search filter
                        if !search_text.is_empty() {
                            let matches = setting.name.to_lowercase().contains(&search_text)
                                || setting.description.to_lowercase().contains(&search_text)
                                || format!("${}", setting.number).contains(&search_text);
                            if !matches {
                                return false;
                            }
                        }

                        // Apply category filter
                        if category != "All" && setting.category != category {
                            return false;
                        }

                        true
                    })
                    .collect();

                if let Some(setting) = filtered_settings.get(index as usize) {
                    if !setting.read_only {
                        let view_for_refresh = view_clone.clone();
                        Self::show_edit_dialog(
                            &view_clone.container,
                            setting,
                            view_clone.communicator.clone(),
                            view_clone.settings_manager.clone(),
                            move || {
                                view_for_refresh.refresh_display();
                            },
                        );
                    }
                }
            }
        });

        view
    }

    pub fn set_communicator(&self, communicator: Arc<Mutex<SerialCommunicator>>) {
        *self.communicator.borrow_mut() = Some(communicator.clone());
        
        // Also pass the communicator to the device info view so it can send $32 commands
        self.device_info_view.set_communicator(communicator);
    }

    pub fn set_device_console(&self, console: Rc<DeviceConsoleView>) {
        *self.device_console.borrow_mut() = Some(console);
    }

    pub fn set_device_manager(&self, manager: std::sync::Arc<gcodekit5_devicedb::DeviceManager>) {
        *self.device_manager.borrow_mut() = Some(manager);
    }

    pub fn set_connected(&self, connected: bool) {
        self.reload_btn.set_sensitive(connected);
        self.restore_btn
            .set_sensitive(connected && self.has_settings());
        if connected {
            // Prime defaults once, then overlay connected-device settings as they arrive.
            if self.last_synced_settings_count.get() == 0
                && self.settings_manager.borrow().get_all_settings().is_empty()
            {
                self.load_default_grbl_settings();
            }

            self.sync_settings_from_connected_device();

            if self.last_synced_settings_count.get() > 0 {
                self.status_label.set_text("Connected - settings loaded");
            } else {
                self.status_label
                    .set_text("Connected - loading settings...");
            }
        } else {
            self.status_label.set_text("Not connected");
            self.last_synced_settings_count.set(0);
            self.last_persisted_settings_count.set(0);
        }
    }

    pub fn set_device_info(
        &self,
        connected: bool,
        device_name: &str,
        firmware: &str,
        version: &str,
    ) {
        // Forward to embedded DeviceInfoView
        self.device_info_view
            .set_connected(connected, device_name, firmware, version);
        if connected {
            self.device_info_view.load_grbl_capabilities_from_status();
        }
    }

    fn has_settings(&self) -> bool {
        !self.settings_manager.borrow().get_all_settings().is_empty()
    }

    fn sync_settings_from_connected_device(&self) {
        let status = device_status::get_status();
        let count = status.grbl_settings.len();
        if count == 0 || count == self.last_synced_settings_count.get() {
            return;
        }

        {
            let mut manager = self.settings_manager.borrow_mut();
            for (n, v) in status.grbl_settings.iter() {
                if let Some(setting) = manager.get_setting(*n) {
                    let mut updated = setting.clone();
                    updated.value = v.clone();
                    updated.numeric_value = v.parse::<f64>().ok();
                    manager.set_setting(updated);
                }
            }
        }

        self.last_synced_settings_count.set(count);
        self.refresh_display();
        self.save_btn.set_sensitive(true);
        self.restore_btn.set_sensitive(true);
        self.device_info_view.load_grbl_capabilities_from_status();

        // Persist to matching device profile (best-effort) so other tooling can use settings.
        // Avoid rewriting the profiles file for every single settings line: persist once when we
        // have a reasonable number of settings loaded.
        if self.last_persisted_settings_count.get() == 0 && count >= 10 {
            if let Some(dm) = self.device_manager.borrow().as_ref() {
                if let Some(port) = status.port_name.as_deref() {
                    let mut candidate = dm
                        .get_active_profile()
                        .filter(|p| p.port == port || p.port == "Auto")
                        .or_else(|| dm.get_all_profiles().into_iter().find(|p| p.port == port));

                    if let Some(mut profile) = candidate.take() {
                        profile.grbl_settings = status.grbl_settings.clone();

                        if let Some(max_s) = device_status::get_grbl_setting_numeric(30) {
                            profile.max_s_value = max_s;
                        }

                        if let Some(laser) = device_status::get_grbl_setting_numeric(32) {
                            profile.has_laser = laser >= 1.0;
                        }

                        if dm.save_profile(profile).is_ok() {
                            self.last_persisted_settings_count.set(count);
                        }
                    }
                }
            }
        }
    }

    fn retrieve_settings(&self) {
        // Load settings definitions first
        self.load_default_grbl_settings();

        if let Some(ref comm) = *self.communicator.borrow() {
            if let Ok(mut comm_lock) = comm.lock() {
                if comm_lock.is_connected() {
                    // Send $$ command to get all settings
                    self.status_label
                        .set_text("Retrieving settings from device...");

                    if let Err(e) = comm_lock.send_command("$$") {
                        self.status_label
                            .set_text(&format!("Error sending $$: {}", e));
                        return;
                    }
                    drop(comm_lock); // Release lock

                    // Wait for console to receive responses (machine control polling handles this)
                    let manager_clone = self.settings_manager.clone();
                    let status_label_clone = self.status_label.clone();
                    let save_btn_clone = self.save_btn.clone();
                    let restore_btn_clone = self.restore_btn.clone();
                    let settings_list_clone = self.settings_list.clone();
                    let search_entry_clone = self.search_entry.clone();
                    let category_filter_clone = self.category_filter.clone();
                    let container_clone = self.container.clone();
                    let device_console_clone = self.device_console.clone();
                    let communicator_clone = self.communicator.clone();

                    let start_log_length = if let Some(ref console) = *self.device_console.borrow()
                    {
                        console.get_log_text().len()
                    } else {
                        0
                    };

                    let attempt_count = Rc::new(RefCell::new(0));

                    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                        *attempt_count.borrow_mut() += 1;

                        // Get console log
                        let console_text = if let Some(ref console) = *device_console_clone.borrow()
                        {
                            console.get_log_text()
                        } else {
                            String::new()
                        };

                        // Check if we got response (console log grew)
                        let has_settings_response = console_text.len() > start_log_length
                            && (console_text.contains("$0=") || console_text.contains("$100="));

                        if has_settings_response || *attempt_count.borrow() > 40 {
                            // 2 seconds timeout
                            // Parse settings from console log
                            let mut count = 0;
                            for line in console_text.lines() {
                                let line = line.trim();
                                if line.starts_with('$') && line.contains('=') {
                                    if let Some((number, value)) =
                                        SettingsManager::parse_setting_line(line)
                                    {
                                        let mut manager = manager_clone.borrow_mut();
                                        if let Some(setting) = manager.get_setting(number) {
                                            let mut updated = setting.clone();
                                            updated.value = value.clone();
                                            updated.numeric_value =
                                                crate::device_status::get_grbl_setting_numeric(
                                                    number,
                                                )
                                                .or_else(|| value.parse::<f64>().ok());
                                            manager.set_setting(updated);
                                            count += 1;
                                        }
                                    }
                                }
                            }

                            // Refresh display by calling apply_filter logic
                            while let Some(child) = settings_list_clone.first_child() {
                                settings_list_clone.remove(&child);
                            }

                            let search_text = search_entry_clone.text().to_string().to_lowercase();
                            let category = category_filter_clone
                                .active_text()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "All".to_string());

                            let manager = manager_clone.borrow();
                            let mut settings: Vec<ConfigSettingRow> = manager
                                .get_all_settings()
                                .iter()
                                .map(|s| ConfigSettingRow::from(*s))
                                .collect();

                            settings.sort_by_key(|s| s.number);

                            let mut displayed_count = 0;
                            for setting in settings {
                                if !search_text.is_empty() {
                                    let matches =
                                        setting.name.to_lowercase().contains(&search_text)
                                            || setting
                                                .description
                                                .to_lowercase()
                                                .contains(&search_text)
                                            || format!("${}", setting.number)
                                                .contains(&search_text);
                                    if !matches {
                                        continue;
                                    }
                                }

                                if category != "All" && setting.category != category {
                                    continue;
                                }

                                let row = Self::create_setting_row_static(
                                    &setting,
                                    &container_clone,
                                    communicator_clone.clone(),
                                );
                                settings_list_clone.append(&row);
                                displayed_count += 1;
                            }

                            let total = manager.get_all_settings().len();
                            if let Some(count_label) = container_clone
                                .last_child()
                                .and_then(|w| w.last_child().and_downcast::<Label>())
                            {
                                count_label
                                    .set_text(&format!("{} / {} settings", displayed_count, total));
                            }

                            status_label_clone
                                .set_text(&format!("Retrieved {} settings from device", count));
                            save_btn_clone.set_sensitive(true);
                            restore_btn_clone.set_sensitive(true);

                            return glib::ControlFlow::Break;
                        }

                        glib::ControlFlow::Continue
                    });
                    return;
                }
            }
        }

        // Fallback if not connected
        self.refresh_display();
        self.status_label
            .set_text("Not connected - showing defaults");
        self.save_btn.set_sensitive(true);
        self.restore_btn.set_sensitive(false);
    }

    fn load_default_grbl_settings(&self) {
        let mut manager = self.settings_manager.borrow_mut();
        manager.clear();

        // Load standard GRBL settings definitions
        for (number, name, description, read_only) in Self::grbl_settings_definitions() {
            manager.set_setting(Setting {
                number,
                name: name.to_string(),
                value: "0".to_string(),
                numeric_value: Some(0.0),
                description: description.to_string(),
                range: None,
                read_only,
            });
        }
    }

    fn grbl_settings_definitions() -> Vec<(u8, &'static str, &'static str, bool)> {
        vec![
            (
                0,
                "Step Pulse",
                "Step pulse duration in microseconds",
                false,
            ),
            (
                1,
                "Step Idle Delay",
                "Step idle delay in milliseconds",
                false,
            ),
            (2, "Step Port Invert", "Step port invert mask", false),
            (
                3,
                "Direction Port Invert",
                "Direction port invert mask",
                false,
            ),
            (4, "Step Enable Invert", "Invert step enable pin", false),
            (5, "Limit Pins Invert", "Invert limit pins", false),
            (6, "Probe Pin Invert", "Invert probe pin", false),
            (10, "Status Report", "Status report options mask", false),
            (11, "Junction Deviation", "Junction deviation in mm", false),
            (12, "Arc Tolerance", "Arc tolerance in mm", false),
            (13, "Report Inches", "Report in inches instead of mm", false),
            (20, "Soft Limits", "Enable soft limits", false),
            (21, "Hard Limits", "Enable hard limits", false),
            (22, "Homing Cycle", "Enable homing cycle", false),
            (
                23,
                "Homing Dir Invert",
                "Homing direction invert mask",
                false,
            ),
            (24, "Homing Feed", "Homing feed rate in mm/min", false),
            (25, "Homing Seek", "Homing seek rate in mm/min", false),
            (26, "Homing Debounce", "Homing debounce in ms", false),
            (
                27,
                "Homing Pull-Off",
                "Homing pull-off distance in mm",
                false,
            ),
            (
                30,
                "Max Spindle Speed",
                "Maximum spindle speed in RPM",
                false,
            ),
            (
                31,
                "Min Spindle Speed",
                "Minimum spindle speed in RPM",
                false,
            ),
            (32, "Laser Mode", "Enable laser mode", false),
            (100, "X Steps/mm", "X-axis steps per mm", false),
            (101, "Y Steps/mm", "Y-axis steps per mm", false),
            (102, "Z Steps/mm", "Z-axis steps per mm", false),
            (110, "X Max Rate", "X-axis maximum rate in mm/min", false),
            (111, "Y Max Rate", "Y-axis maximum rate in mm/min", false),
            (112, "Z Max Rate", "Z-axis maximum rate in mm/min", false),
            (
                120,
                "X Acceleration",
                "X-axis acceleration in mm/sec²",
                false,
            ),
            (
                121,
                "Y Acceleration",
                "Y-axis acceleration in mm/sec²",
                false,
            ),
            (
                122,
                "Z Acceleration",
                "Z-axis acceleration in mm/sec²",
                false,
            ),
            (130, "X Max Travel", "X-axis maximum travel in mm", false),
            (131, "Y Max Travel", "Y-axis maximum travel in mm", false),
            (132, "Z Max Travel", "Z-axis maximum travel in mm", false),
        ]
    }

    fn refresh_display(&self) {
        self.apply_filter();
    }

    fn apply_filter(&self) {
        // Clear existing rows
        while let Some(child) = self.settings_list.first_child() {
            self.settings_list.remove(&child);
        }

        let search_text = self.search_entry.text().to_string().to_lowercase();
        let category = self
            .category_filter
            .active_text()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "All".to_string());

        let manager = self.settings_manager.borrow();
        let mut settings: Vec<ConfigSettingRow> = manager
            .get_all_settings()
            .iter()
            .map(|s| ConfigSettingRow::from(*s))
            .collect();

        settings.sort_by_key(|s| s.number);

        let mut count = 0;
        for setting in settings {
            // Apply filters
            if !search_text.is_empty() {
                let matches = setting.name.to_lowercase().contains(&search_text)
                    || setting.description.to_lowercase().contains(&search_text)
                    || format!("${}", setting.number).contains(&search_text);
                if !matches {
                    continue;
                }
            }

            if category != "All" && setting.category != category {
                continue;
            }

            let row = self.create_setting_row(&setting);
            self.settings_list.append(&row);
            count += 1;
        }

        let total = manager.get_all_settings().len();
        let count_label = self
            .container
            .last_child()
            .and_then(|w| w.last_child().and_downcast::<Label>());
        if let Some(label) = count_label {
            label.set_text(&format!("{} / {} settings", count, total));
        }
    }

    fn create_setting_row_static(
        setting: &ConfigSettingRow,
        _parent: &Box,
        _communicator: Rc<RefCell<Option<Arc<Mutex<SerialCommunicator>>>>>,
    ) -> ListBoxRow {
        let row = ListBoxRow::new();
        let hbox = Box::new(Orientation::Horizontal, 5);
        hbox.set_margin_start(5);
        hbox.set_margin_end(5);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);

        let id_lbl = Label::new(Some(&format!("${}", setting.number)));
        id_lbl.set_width_request(50);
        id_lbl.set_xalign(0.0);
        if setting.read_only {
            id_lbl.add_css_class("dim-label");
        }
        hbox.append(&id_lbl);

        let name_lbl = Label::new(Some(&setting.name));
        name_lbl.set_width_request(200);
        name_lbl.set_xalign(0.0);
        name_lbl.add_css_class("accent");
        if setting.read_only {
            name_lbl.add_css_class("dim-label");
        }
        hbox.append(&name_lbl);

        let value_lbl = Label::new(Some(&setting.value));
        value_lbl.set_width_request(100);
        value_lbl.set_xalign(0.0);
        value_lbl.add_css_class("success");
        hbox.append(&value_lbl);

        let unit_lbl = Label::new(Some(&setting.unit));
        unit_lbl.set_width_request(80);
        unit_lbl.set_xalign(0.0);
        unit_lbl.add_css_class("dim-label");
        hbox.append(&unit_lbl);

        let cat_lbl = Label::new(Some(&setting.category));
        cat_lbl.set_width_request(150);
        cat_lbl.set_xalign(0.0);
        cat_lbl.add_css_class("dim-label");
        hbox.append(&cat_lbl);

        let desc_lbl = Label::new(Some(&setting.description));
        desc_lbl.set_hexpand(true);
        desc_lbl.set_xalign(0.0);
        desc_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        desc_lbl.add_css_class("dim-label");
        hbox.append(&desc_lbl);

        row.set_child(Some(&hbox));

        if !setting.read_only {
            row.set_activatable(true);
            // Note: Individual row activation is handled by ListBox signal instead
            row.set_activatable(true);
        } else {
            row.set_activatable(false);
        }

        row
    }

    fn create_setting_row(&self, setting: &ConfigSettingRow) -> ListBoxRow {
        Self::create_setting_row_static(setting, &self.container, self.communicator.clone())
    }

    fn show_edit_dialog(
        parent: &Box,
        setting: &ConfigSettingRow,
        communicator: Rc<RefCell<Option<Arc<Mutex<SerialCommunicator>>>>>,
        settings_manager: Rc<RefCell<SettingsManager>>,
        refresh_callback: impl Fn() + 'static,
    ) {
        let window = parent.root().and_downcast::<gtk4::Window>().unwrap();

        let dialog = Dialog::with_buttons(
            Some(&format!("Edit Setting ${}", setting.number)),
            Some(&window),
            DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );

        let content = dialog.content_area();
        content.set_spacing(10);
        content.set_margin_top(10);
        content.set_margin_bottom(10);
        content.set_margin_start(10);
        content.set_margin_end(10);

        let grid = Grid::new();
        grid.set_row_spacing(10);
        grid.set_column_spacing(10);

        let name_label = Label::new(Some("Name:"));
        name_label.set_xalign(0.0);
        name_label.add_css_class("heading");
        grid.attach(&name_label, 0, 0, 1, 1);

        let name_value = Label::new(Some(&setting.name));
        name_value.set_xalign(0.0);
        grid.attach(&name_value, 1, 0, 2, 1);

        let desc_label = Label::new(Some("Description:"));
        desc_label.set_xalign(0.0);
        desc_label.set_valign(Align::Start);
        desc_label.add_css_class("heading");
        grid.attach(&desc_label, 0, 1, 1, 1);

        let desc_value = Label::new(Some(&setting.description));
        desc_value.set_xalign(0.0);
        desc_value.set_wrap(true);
        desc_value.set_max_width_chars(50);
        desc_value.add_css_class("dim-label");
        grid.attach(&desc_value, 1, 1, 2, 1);

        let value_label = Label::new(Some("Value:"));
        value_label.set_xalign(0.0);
        value_label.add_css_class("heading");
        grid.attach(&value_label, 0, 2, 1, 1);

        let value_entry = Entry::new();
        value_entry.set_text(&setting.value);
        value_entry.set_hexpand(true);
        grid.attach(&value_entry, 1, 2, 1, 1);

        let unit_label = Label::new(Some(&setting.unit));
        unit_label.add_css_class("dim-label");
        grid.attach(&unit_label, 2, 2, 1, 1);

        content.append(&grid);

        // Connect the response signal to handle Save/Cancel
        let setting_number = setting.number;
        let comm_clone = communicator.clone();
        let manager_clone = settings_manager.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                let new_value = value_entry.text().to_string();

                // Update local settings manager first
                let mut manager = manager_clone.borrow_mut();
                if let Some(setting) = manager.get_setting(setting_number) {
                    let mut updated = setting.clone();
                    updated.value = new_value.clone();
                    updated.numeric_value = new_value.parse::<f64>().ok();
                    manager.set_setting(updated);
                }
                drop(manager); // Release the borrow

                // Refresh the display
                refresh_callback();

                // Send to device
                if let Some(ref comm) = *comm_clone.borrow() {
                    if let Ok(mut comm_lock) = comm.lock() {
                        if comm_lock.is_connected() {
                            let command = format!("${}={}", setting_number, new_value);
                            let _ = comm_lock.send_command(&command);
                        }
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    }

    fn save_to_file(&self) {
        let window = self
            .container
            .root()
            .and_downcast::<gtk4::Window>()
            .unwrap();

        let dialog = gtk4::FileChooserDialog::new(
            Some("Export Settings"),
            Some(&window),
            gtk4::FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );

        dialog.set_current_name("grbl_settings.json");

        let status_label = self.status_label.clone();
        let manager = self.settings_manager.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(path) = dialog.file().and_then(|f| f.path()) {
                    let res = manager.borrow().export_to_file(&path);
                    match res {
                        Ok(_) => status_label.set_text(&format!("Exported settings to {:?}", path)),
                        Err(e) => status_label.set_text(&format!("Export failed: {}", e)),
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    }

    fn load_from_file(&self) {
        let window = self
            .container
            .root()
            .and_downcast::<gtk4::Window>()
            .unwrap();

        let dialog = gtk4::FileChooserDialog::new(
            Some("Import Settings"),
            Some(&window),
            gtk4::FileChooserAction::Open,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );

        let status_label = self.status_label.clone();
        let manager = self.settings_manager.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(path) = dialog.file().and_then(|f| f.path()) {
                    let res = manager.borrow_mut().import_from_file(&path);
                    match res {
                        Ok(_) => {
                            status_label.set_text(&format!("Imported settings from {:?}", path));
                        }
                        Err(e) => status_label.set_text(&format!("Import failed: {}", e)),
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    }

    fn restore_to_device(&self) {
        self.status_label
            .set_text("Restoring settings to device...");
        // This would send settings to device via communicator
    }
}
