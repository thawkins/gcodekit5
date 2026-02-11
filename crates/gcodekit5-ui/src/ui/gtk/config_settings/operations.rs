use gcodekit5_communication::firmware::grbl::settings::SettingsManager;
use gcodekit5_communication::{Communicator, SerialCommunicator};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Dialog, DialogFlags, Entry, Grid, Label, ListBoxRow, Orientation, ResponseType,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use super::ConfigSettingRow;
use super::ConfigSettingsView;
use crate::device_status;

impl ConfigSettingsView {
    pub(crate) fn has_settings(&self) -> bool {
        !self.settings_manager.borrow().get_all_settings().is_empty()
    }

    pub(crate) fn sync_settings_from_connected_device(&self) {
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

    pub(crate) fn retrieve_settings(&self) {
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

                                let row = ConfigSettingsView::create_setting_row_static(
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

    pub(crate) fn refresh_display(&self) {
        self.apply_filter();
    }

    pub(crate) fn apply_filter(&self) {
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

    pub(crate) fn create_setting_row_static(
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

    pub(crate) fn show_edit_dialog(
        parent: &Box,
        setting: &ConfigSettingRow,
        communicator: Rc<RefCell<Option<Arc<Mutex<SerialCommunicator>>>>>,
        settings_manager: Rc<RefCell<SettingsManager>>,
        refresh_callback: impl Fn() + 'static,
    ) {
        let Some(window) = parent.root().and_downcast::<gtk4::Window>() else {
            tracing::warn!("Failed to get parent window for settings dialog");
            return;
        };

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

    pub(crate) fn save_to_file(&self) {
        let Some(window) = self.container.root().and_downcast::<gtk4::Window>() else {
            tracing::warn!("Failed to get parent window for export dialog");
            return;
        };

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

    pub(crate) fn load_from_file(&self) {
        let Some(window) = self.container.root().and_downcast::<gtk4::Window>() else {
            tracing::warn!("Failed to get parent window for import dialog");
            return;
        };

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

    pub(crate) fn restore_to_device(&self) {
        self.status_label
            .set_text("Restoring settings to device...");
        // This would send settings to device via communicator
    }
}
