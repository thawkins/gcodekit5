//! # Configuration Settings Panel
//!
//! Provides the settings UI for configuring machine parameters,
//! GRBL settings, and operational preferences. Organized into
//! sub-modules for different setting categories.

mod grbl_settings;
mod operations;

use crate::ui::gtk::common::text_dropdown::{new_text_dropdown, TextDropDownExt};
use gcodekit5_communication::firmware::grbl::settings::{Setting, SettingsManager};
use gcodekit5_communication::SerialCommunicator;
use gcodekit5_settings::controller::SettingsController;
use gtk4::prelude::*;
use gtk4::{
    Box, Button, DropDown, Label, ListBox, Orientation, PolicyType, ScrolledWindow, SearchEntry,
    Separator,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
pub struct ConfigSettingRow {
    pub number: u16,
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
            40..=65 => "System",
            70..=75 => "Network",
            100..=105 => "Steps Per Unit",
            110..=115 => "Max Rate",
            120..=125 => "Acceleration",
            130..=135 => "Max Travel",
            200..=210 => "Drivers",
            300..=321 => "Network",
            340..=344 => "Tool",
            370..=372 => "I/O",
            384..=385 => "Motion",
            395..=397 => "Startup",
            481 => "System",
            550..=560 => "Plasma",
            600..=603 => "Encoder",
            650..=651 => "Modbus",
            680 => "RS485",
            _ => "Other",
        };

        Self {
            number: s.number,
            name: s.name.clone(),
            value: s.value.clone(),
            unit: s.unit.clone().unwrap_or_default(),
            description: s.description.clone(),
            category: category.to_string(),
            read_only: s.read_only,
        }
    }
}

use crate::t;
use crate::ui::gtk::common::spacing;
use crate::ui::gtk::device_console::DeviceConsoleView;
use crate::ui::gtk::device_info::DeviceInfoView;
use crate::ui::gtk::help_browser;
use gcodekit5_core::{shared, shared_none, Shared, SharedOption, ThreadSafe};

/// Setting categories offered by the category filter (also the filter ids).
pub(crate) const SETTING_CATEGORIES: &[&str] = &[
    "All",
    "System",
    "Motion",
    "Steps Per Unit",
    "Max Rate",
    "Acceleration",
    "Max Travel",
    "Homing",
    "Spindle",
    "Network",
    "Drivers",
    "Tool",
    "I/O",
    "Startup",
    "Plasma",
    "Encoder",
    "Modbus",
    "RS485",
    "Other",
];

pub struct ConfigSettingsView {
    pub container: Box,
    pub device_info_view: Rc<DeviceInfoView>,
    // Fields used internally for settings sync and persistence tracking.
    #[allow(dead_code)]
    pub(crate) settings_controller: Rc<SettingsController>,
    pub(crate) settings_manager: Shared<SettingsManager>,
    pub(crate) last_synced_settings_count: Cell<usize>,
    pub(crate) last_persisted_settings_count: Cell<usize>,
    pub(crate) device_manager: RefCell<Option<std::sync::Arc<gcodekit5_devicedb::DeviceManager>>>,

    pub(crate) settings_list: ListBox,
    pub(crate) search_entry: SearchEntry,
    pub(crate) category_filter: DropDown,
    pub(crate) status_label: Label,
    pub(crate) reload_btn: Button,
    pub(crate) save_btn: Button,
    pub(crate) restore_btn: Button,
    pub(crate) communicator: SharedOption<ThreadSafe<SerialCommunicator>>,
    pub(crate) device_console: SharedOption<Rc<DeviceConsoleView>>,
}

impl ConfigSettingsView {
    /// Builds the Device Config page (device info panel plus GRBL settings list).
    ///
    /// # Arguments
    /// * `settings_controller` - Application settings.
    pub fn new(settings_controller: Rc<SettingsController>) -> Rc<Self> {
        // Outer container splits into left (Device Info) and right (Config Settings)
        let outer = Box::new(Orientation::Horizontal, spacing::MEDIUM);
        outer.set_hexpand(true);
        outer.set_vexpand(true);
        outer.set_margin_top(spacing::MEDIUM);
        outer.set_margin_bottom(spacing::MEDIUM);
        outer.set_margin_start(spacing::MEDIUM);
        outer.set_margin_end(spacing::MEDIUM);

        // Left panel - Device Info (embedded)
        let left_panel = Box::new(Orientation::Vertical, 0);
        left_panel.set_width_request(320);
        left_panel.set_margin_top(spacing::MEDIUM);
        left_panel.set_margin_bottom(spacing::MEDIUM);
        left_panel.set_margin_start(spacing::MEDIUM);
        left_panel.set_margin_end(spacing::MEDIUM);

        // Create DeviceInfoView and add to left panel
        let device_info_view = DeviceInfoView::new();
        left_panel.append(&device_info_view.container);

        // Separator
        let sep = Separator::new(Orientation::Vertical);

        // Right panel - Config Settings content
        let container = Box::new(Orientation::Vertical, spacing::MEDIUM);
        container.set_hexpand(true);
        container.set_vexpand(true);
        container.set_margin_top(spacing::MEDIUM);
        container.set_margin_bottom(spacing::MEDIUM);
        container.set_margin_start(spacing::MEDIUM);
        container.set_margin_end(spacing::MEDIUM);

        // Toolbar
        let toolbar = Box::new(Orientation::Horizontal, spacing::MEDIUM);

        let reload_btn = Button::with_label(&t!("Retrieve"));
        reload_btn.set_tooltip_text(Some(&t!("Retrieve Settings from Device ($$)")));
        toolbar.append(&reload_btn);

        let save_btn = Button::with_label(&t!("Save"));
        save_btn.set_tooltip_text(Some(&t!("Save Settings to File")));
        save_btn.set_sensitive(false);
        toolbar.append(&save_btn);

        let load_btn = Button::with_label(&t!("Load"));
        load_btn.set_tooltip_text(Some(&t!("Load Settings from File")));
        toolbar.append(&load_btn);

        let restore_btn = Button::with_label(&t!("Restore"));
        restore_btn.set_tooltip_text(Some(&t!("Restore Settings to Device")));
        restore_btn.set_sensitive(false);
        // Not shown: restoring needs each `$n=v` paced by the controller's `ok`,
        // which this page cannot see. (It used to claim a restore it never did.)
        restore_btn.set_visible(false);
        toolbar.append(&restore_btn);

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        toolbar.append(&spacer);
        toolbar.append(&help_browser::make_help_button("device_config"));

        container.append(&toolbar);

        // Filter bar (right panel)
        let filter_bar = Box::new(Orientation::Horizontal, 10);

        let filter_label = Label::new(Some(&t!("Filter:")));
        filter_bar.append(&filter_label);

        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some(&t!("Search settings...")));
        search_entry.set_hexpand(true);
        filter_bar.append(&search_entry);

        let category_label = Label::new(Some(&t!("Category:")));
        filter_bar.append(&category_label);

        let category_filter = new_text_dropdown();
        // Ids stay in English (they match `ConfigSettingRow::category`); labels are translated.
        for category in SETTING_CATEGORIES {
            category_filter.append(Some(category), &t!(*category));
        }
        category_filter.set_active_id(Some("All"));
        filter_bar.append(&category_filter);

        container.append(&filter_bar);

        // Settings List Header
        let header = Box::new(Orientation::Horizontal, spacing::SMALL);
        header.add_css_class("list-header");
        header.set_margin_start(spacing::SMALL);
        header.set_margin_end(spacing::SMALL);

        let id_lbl = Label::new(Some(&t!("ID")));
        id_lbl.set_width_request(50);
        id_lbl.set_xalign(0.0);
        id_lbl.add_css_class("heading");
        header.append(&id_lbl);

        let name_lbl = Label::new(Some(&t!("Name")));
        name_lbl.set_width_request(200);
        name_lbl.set_xalign(0.0);
        name_lbl.add_css_class("heading");
        header.append(&name_lbl);

        let value_lbl = Label::new(Some(&t!("Value")));
        value_lbl.set_width_request(100);
        value_lbl.set_xalign(0.0);
        value_lbl.add_css_class("heading");
        header.append(&value_lbl);

        let unit_lbl = Label::new(Some(&t!("Unit")));
        unit_lbl.set_width_request(80);
        unit_lbl.set_xalign(0.0);
        unit_lbl.add_css_class("heading");
        header.append(&unit_lbl);

        let cat_lbl = Label::new(Some(&t!("Category")));
        cat_lbl.set_width_request(150);
        cat_lbl.set_xalign(0.0);
        cat_lbl.add_css_class("heading");
        header.append(&cat_lbl);

        let desc_lbl = Label::new(Some(&t!("Description")));
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
        let status_bar = Box::new(Orientation::Horizontal, spacing::MEDIUM);
        status_bar.add_css_class("status-bar");
        status_bar.set_margin_start(spacing::SMALL);
        status_bar.set_margin_end(spacing::SMALL);

        let status_label = Label::new(Some(&t!("Not connected")));
        status_label.set_xalign(0.0);
        status_bar.append(&status_label);

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        status_bar.append(&spacer);

        let count_label = Label::new(Some(&format!("0 / 0 {}", t!("settings"))));
        count_label.add_css_class("dim-label");
        status_bar.append(&count_label);

        container.append(&status_bar);

        // Construct view with device_info embedded
        let settings_manager = shared(SettingsManager::new());

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
            communicator: shared_none(),
            device_console: shared_none(),
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
                let category = view_clone.selected_category();

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
                        !(category != "All" && setting.category != category)
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

    /// Sets the serial connection used to read and write settings.
    pub fn set_communicator(&self, communicator: ThreadSafe<SerialCommunicator>) {
        *self.communicator.borrow_mut() = Some(communicator.clone());

        // Also pass the communicator to the device info view so it can send $32 commands
        self.device_info_view.set_communicator(communicator);
    }

    /// Sets the device console whose log receives `$$` output.
    pub fn set_device_console(&self, console: Rc<DeviceConsoleView>) {
        *self.device_console.borrow_mut() = Some(console);
    }

    /// Sets the device profile manager used to persist reported settings.
    pub fn set_device_manager(&self, manager: std::sync::Arc<gcodekit5_devicedb::DeviceManager>) {
        *self.device_manager.borrow_mut() = Some(manager);
    }

    /// Updates the view for a new connection state.
    ///
    /// Call this only when the connection state changes; it resets status messages.
    /// New settings reported by the controller are picked up by `sync_device_settings`.
    ///
    /// # Arguments
    /// * `connected` - True when a controller is connected.
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
                self.status_label
                    .set_text(&t!("Connected - settings loaded"));
            } else {
                self.status_label
                    .set_text(&t!("Connected - loading settings..."));
            }
        } else {
            self.status_label.set_text(&t!("Not connected"));
            self.last_synced_settings_count.set(0);
            self.last_persisted_settings_count.set(0);
        }
    }

    /// Applies settings the controller reported since the last call (from `$$`).
    ///
    /// Cheap when nothing changed, so it can be called periodically. Only the first
    /// successful load changes the status message.
    pub fn sync_device_settings(&self) {
        let before = self.last_synced_settings_count.get();
        self.sync_settings_from_connected_device();
        if before == 0 && self.last_synced_settings_count.get() > 0 {
            self.status_label
                .set_text(&t!("Connected - settings loaded"));
        }
    }

    /// Returns the category filter's selected id ("All" when nothing is selected).
    pub(crate) fn selected_category(&self) -> String {
        self.category_filter
            .active_id()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "All".to_string())
    }

    /// Updates the embedded Device Info panel.
    ///
    /// # Arguments
    /// * `connected` - True when a controller is connected.
    /// * `device_name` - Name shown for the device.
    /// * `firmware` - Firmware type.
    /// * `version` - Firmware version.
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
}
