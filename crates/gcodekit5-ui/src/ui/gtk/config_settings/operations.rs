//! # Settings Operations
//!
//! Backend operations for reading, writing, and validating
//! configuration settings. Handles communication with the
//! connected CNC device.

use gcodekit5_communication::firmware::grbl::settings::{Setting, SettingsManager};
use gcodekit5_communication::{Communicator, SerialCommunicator};
use gcodekit5_core::{Shared, SharedOption, ThreadSafe, shared};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Entry, Grid, Label, ListBoxRow, Orientation};

use super::ConfigSettingRow;
use super::ConfigSettingsView;
use crate::device_status;
use crate::ui::gtk::common::text_dropdown::TextDropDownExt;
use crate::ui::gtk::file_dialog::FileDialogHelpers;

use crate::t;

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

        if self.last_persisted_settings_count.get() == 0
            && count >= 10
            && let Some(dm) = self.device_manager.borrow().as_ref()
            && let Some(port) = status.port_name.as_deref()
        {
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

    pub(crate) fn parse_repetier_epr_line(line: &str) -> Option<Setting> {
        let (_, rest) = line.split_once("EPR:")?;
        let mut parts = rest.trim().splitn(4, ' ');

        let _data_type = parts.next()?;
        let pos: u16 = parts.next()?.parse().ok()?;
        let value = parts.next()?.to_string();
        let raw_desc = parts.next().unwrap_or("Repetier Parameter").trim();

        let (name, unit) = if let Some((n, u)) = raw_desc.rsplit_once('[') {
            (
                n.trim().to_string(),
                Some(u.trim_end_matches(']').to_string()),
            )
        } else {
            (raw_desc.to_string(), None)
        };

        Some(Setting {
            number: 1000 + pos,
            name,
            value: value.clone(),
            numeric_value: value.parse::<f64>().ok(),
            description: format!("EEPROM Pos {}: {}", pos, raw_desc),
            range: None,
            read_only: false,
            unit,
        })
    }

    pub(crate) fn retrieve_settings(&self) {
        let mut is_connected = false;
        if let Some(ref comm) = *self.communicator.borrow()
            && comm.lock().is_connected()
        {
            is_connected = true;
        }

        if !is_connected {
            let parent = crate::ui::gtk::file_dialog::parent_window(&self.container);
            crate::ui::gtk::common::dialog::show_error(
                &t!("Connection Notice"),
                &t!("Please connect the machine before retrieving parameters."),
                parent.as_ref(),
            );
            self.status_label
                .set_text(&t!("Operation cancelled: Device not connected"));
            return;
        }

        // Check first: loading the definitions resets every value, and a
        // refused Retrieve must keep the values already on the page.
        if let Some(reason) = crate::device_status::settings_command_blocked() {
            self.status_label
                .set_text(&format!("{} {}", t!("Cannot retrieve settings:"), reason));
            return;
        }

        // Detect the controller type from the active profile or the device panel.
        let mut is_repetier = false;
        if let Some(dm) = self.device_manager.borrow().as_ref()
            && let Some(profile) = dm.get_active_profile()
        {
            let fw = format!("{:?}", profile.controller_type).to_lowercase();
            if fw.contains("repetier") {
                is_repetier = true;
            }
        }
        if self
            .device_info_view
            .get_firmware()
            .to_lowercase()
            .contains("repetier")
        {
            is_repetier = true;
        }

        // Load the matching default settings table.
        if is_repetier {
            self.load_default_repetier_settings();
        } else {
            self.load_default_grbl_settings();
        }

        if let Some(ref comm) = *self.communicator.borrow() {
            let mut comm_lock = comm.lock();
            self.status_label
                .set_text(&t!("Retrieving settings and device info..."));

            // The identity query is sent after the settings dump. The controller
            // answers in order, so its reply marks the end of the dump.
            let identity_sent = if is_repetier {
                if let Err(e) = comm_lock.send_command("M205") {
                    self.status_label
                        .set_text(&format!("{} {}", t!("Error sending M205:"), e));
                    return;
                }
                match comm_lock.send_command("M115") {
                    Ok(()) => true,
                    Err(e) => {
                        tracing::warn!("Error sending M115: {}", e);
                        false
                    }
                }
            } else {
                if let Err(e) = comm_lock.send_command("$$") {
                    self.status_label
                        .set_text(&format!("{} {}", t!("Error sending $$:"), e));
                    return;
                }
                match comm_lock.send_command("$I") {
                    Ok(()) => true,
                    Err(e) => {
                        tracing::warn!("Error sending $I: {}", e);
                        false
                    }
                }
            };

            drop(comm_lock);

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
            let device_info_view_clone = self.device_info_view.clone();

            // Position in the console log (stable while old lines are trimmed).
            let start_log_pos = match *self.device_console.borrow() {
                Some(ref console) => console.log_position(),
                _ => 0,
            };

            let attempt_count = shared(0);
            let last_log_pos = shared(start_log_pos);

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                *attempt_count.borrow_mut() += 1;

                // Only look at output that arrived after the request was sent; older
                // `$n=` / `EPR:` lines (previous reads, another device) would otherwise
                // be parsed as current values.
                let (new_text, log_pos) = match *device_console_clone.borrow() {
                    Some(ref console) => (
                        console.log_text_since(start_log_pos),
                        console.log_position(),
                    ),
                    _ => (String::new(), 0),
                };
                let new_console_text = new_text.as_str();

                // The reply streams in over several polls. The dump is complete once the
                // identity reply that follows it has arrived (`FIRMWARE_NAME:` for M115,
                // `[VER:` for $I) and the log has stopped growing; a pause between dump
                // lines alone does not end the wait.
                let settled = log_pos == *last_log_pos.borrow();
                *last_log_pos.borrow_mut() = log_pos;
                let (has_settings, has_identity) = if is_repetier {
                    (
                        new_console_text.contains("EPR:"),
                        new_console_text.contains("FIRMWARE_NAME:"),
                    )
                } else {
                    (
                        new_console_text.contains("$0=") || new_console_text.contains("$100="),
                        new_console_text.contains("[VER:"),
                    )
                };
                let complete = settled && has_settings && (has_identity || !identity_sent);
                // 5 s at 50 ms per poll.
                let timed_out = *attempt_count.borrow() > 100;

                if complete || timed_out {
                    let mut count = 0;

                    let mut detected_machine = device_info_view_clone.get_device_name();
                    let mut detected_firmware = device_info_view_clone.get_firmware();
                    let mut detected_version = device_info_view_clone.get_version();
                    let mut info_updated = false;

                    for line in new_console_text.lines() {
                        let line = line.trim();

                        if is_repetier {
                            // Repetier EEPROM lines (EPR: anywhere in the line)
                            if line.contains("EPR:")
                                && let Some(setting) = Self::parse_repetier_epr_line(line)
                            {
                                let mut manager = manager_clone.borrow_mut();
                                manager.set_setting(setting);
                                count += 1;
                            }

                            // Repetier M115 reply (FIRMWARE_NAME:Repetier_1.0.6 ...)
                            if line.contains("FIRMWARE_NAME:") {
                                if let Some(fw_part) = line.split("FIRMWARE_NAME:").nth(1) {
                                    let fw_token =
                                        fw_part.split_whitespace().next().unwrap_or("Repetier");
                                    if let Some((name, ver)) = fw_token.split_once('_') {
                                        detected_firmware = name.to_string();
                                        detected_version = ver.to_string();
                                    } else {
                                        detected_firmware = fw_token.to_string();
                                    }
                                    info_updated = true;
                                }
                                if let Some(machine_part) = line.split("MACHINE_TYPE:").nth(1) {
                                    let m_name = machine_part
                                        .split("EXTRUDER_COUNT:")
                                        .next()
                                        .unwrap_or("")
                                        .trim();
                                    if !m_name.is_empty() {
                                        detected_machine = m_name.to_string();
                                        info_updated = true;
                                    }
                                }
                            }
                        } else {
                            // GRBL settings lines ($0=...)
                            if line.starts_with('$')
                                && line.contains('=')
                                && let Some((number, value)) =
                                    SettingsManager::parse_setting_line(line)
                            {
                                let mut manager = manager_clone.borrow_mut();
                                if let Some(setting) = manager.get_setting(number) {
                                    let mut updated = setting.clone();
                                    updated.value = value.clone();
                                    updated.numeric_value =
                                        crate::device_status::get_grbl_setting_numeric(number)
                                            .or_else(|| value.parse::<f64>().ok());
                                    manager.set_setting(updated);
                                    count += 1;
                                }
                            }
                            // GRBL $I blocks ([Machine:...])
                            if line.starts_with('[') && line.ends_with(']') {
                                let content = &line[1..line.len() - 1];
                                if let Some((key, val)) = content.split_once(':') {
                                    match key {
                                        "Machine" => {
                                            detected_machine = val.to_string();
                                            info_updated = true;
                                        }
                                        "VER" => {
                                            detected_version = val
                                                .split_once('.')
                                                .map(|(v, _)| v)
                                                .unwrap_or(val)
                                                .to_string();
                                            if detected_firmware == "-"
                                                || detected_firmware.is_empty()
                                            {
                                                detected_firmware = "Grbl".to_string();
                                            }
                                            info_updated = true;
                                        }
                                        "Software" => {
                                            detected_firmware = format!("Grbl Ext ({})", val);
                                            info_updated = true;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            if line.starts_with("Grbl ")
                                && let Some(ver) = line.split_whitespace().nth(1)
                            {
                                detected_version = ver.to_string();
                                if detected_firmware == "-" || detected_firmware.is_empty() {
                                    detected_firmware = "Grbl".to_string();
                                }
                                info_updated = true;
                            }
                        }
                    }

                    if count == 0 {
                        manager_clone.borrow_mut().clear();
                        while let Some(child) = settings_list_clone.first_child() {
                            settings_list_clone.remove(&child);
                        }

                        if let Some(count_label) = container_clone
                            .last_child()
                            .and_then(|w| w.last_child().and_downcast::<Label>())
                        {
                            count_label.set_text(&format!("0 / 0 {}", t!("settings")));
                        }

                        status_label_clone.set_text(&t!(
                            "Protocol mismatch: No valid parameters received from device"
                        ));

                        return glib::ControlFlow::Break;
                    }

                    if info_updated {
                        device_info_view_clone.set_connected(
                            true,
                            &detected_machine,
                            &detected_firmware,
                            &detected_version,
                        );
                    }
                    if !is_repetier {
                        device_info_view_clone.load_grbl_capabilities_from_status();
                    }
                    while let Some(child) = settings_list_clone.first_child() {
                        settings_list_clone.remove(&child);
                    }
                    let search_text = search_entry_clone.text().to_string().to_lowercase();
                    let category = category_filter_clone
                        .active_id()
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
                        if !setting.matches_filter(&search_text, &category) {
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
                        count_label.set_text(&format!(
                            "{} / {} {}",
                            displayed_count,
                            total,
                            t!("settings")
                        ));
                    }
                    let status = if complete {
                        t!("Settings retrieved from device:")
                    } else {
                        t!("Settings retrieval incomplete (timed out), settings received:")
                    };
                    status_label_clone.set_text(&format!("{} {}", status, count));
                    save_btn_clone.set_sensitive(true);
                    restore_btn_clone.set_sensitive(true);
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            });
        }
    }
    pub(crate) fn refresh_display(&self) {
        self.apply_filter();
    }
    pub(crate) fn apply_filter(&self) {
        while let Some(child) = self.settings_list.first_child() {
            self.settings_list.remove(&child);
        }
        let search_text = self.search_entry.text().to_string().to_lowercase();
        let category = self.selected_category();
        let manager = self.settings_manager.borrow();
        let mut settings: Vec<ConfigSettingRow> = manager
            .get_all_settings()
            .iter()
            .map(|s| ConfigSettingRow::from(*s))
            .collect();
        settings.sort_by_key(|s| s.number);
        let mut count = 0;
        for setting in settings {
            if !setting.matches_filter(&search_text, &category) {
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
            label.set_text(&format!("{} / {} {}", count, total, t!("settings")));
        }
    }
    pub(crate) fn create_setting_row_static(
        setting: &ConfigSettingRow,
        _parent: &Box,
        _communicator: SharedOption<ThreadSafe<SerialCommunicator>>,
    ) -> ListBoxRow {
        let row = ListBoxRow::new();
        let hbox = Box::new(Orientation::Horizontal, 5);
        hbox.set_margin_start(5);
        hbox.set_margin_end(5);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);
        let id_text = if setting.number >= 1000 {
            format!("EPR:{}", setting.number - 1000)
        } else {
            format!("${}", setting.number)
        };
        let id_lbl = Label::new(Some(&id_text));
        id_lbl.set_width_request(65);
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
        let cat_lbl = Label::new(Some(&t!(setting.category.as_str())));
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
        communicator: SharedOption<ThreadSafe<SerialCommunicator>>,
        settings_manager: Shared<SettingsManager>,
        refresh_callback: impl Fn() + 'static,
    ) {
        let Some(parent_window) = parent.root().and_downcast::<gtk4::Window>() else {
            tracing::warn!("Failed to get parent window for settings dialog");
            return;
        };

        let title = if setting.number >= 1000 {
            format!("{} {}", t!("Edit EEPROM Pos"), setting.number - 1000)
        } else {
            format!("{} ${}", t!("Edit Setting"), setting.number)
        };

        let dialog = gtk4::Window::builder()
            .title(&title)
            .transient_for(&parent_window)
            .modal(true)
            .resizable(false)
            .build();

        let vbox = Box::new(Orientation::Vertical, 12);
        vbox.set_margin_top(15);
        vbox.set_margin_bottom(15);
        vbox.set_margin_start(15);
        vbox.set_margin_end(15);

        let grid = Grid::new();
        grid.set_row_spacing(10);
        grid.set_column_spacing(10);

        let name_label = Label::new(Some(&t!("Name:")));
        name_label.set_xalign(0.0);
        name_label.add_css_class("heading");
        grid.attach(&name_label, 0, 0, 1, 1);

        let name_value = Label::new(Some(&setting.name));
        name_value.set_xalign(0.0);
        grid.attach(&name_value, 1, 0, 2, 1);

        let desc_label = Label::new(Some(&t!("Description:")));
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

        let value_label = Label::new(Some(&t!("Value:")));
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

        vbox.append(&grid);

        // Cancel and Save buttons
        let btn_box = Box::new(Orientation::Horizontal, 10);
        btn_box.set_halign(Align::End);

        let cancel_btn = Button::with_label(&t!("Cancel"));
        let dialog_cancel = dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            dialog_cancel.close();
        });
        btn_box.append(&cancel_btn);

        let save_btn = Button::with_label(&t!("Save"));
        save_btn.add_css_class("suggested-action");

        let setting_number = setting.number;
        let comm_clone = communicator.clone();
        let manager_clone = settings_manager.clone();
        let dialog_save = dialog.clone();
        let refresh_callback = std::rc::Rc::new(refresh_callback);

        save_btn.connect_clicked(move |_| {
            let new_value = value_entry.text().to_string();
            let command = if setting_number >= 1000 {
                let epr_pos = setting_number - 1000;
                if new_value.contains('.') {
                    format!("M206 T3 P{} X{}", epr_pos, new_value)
                } else {
                    format!("M206 T2 P{} S{}", epr_pos, new_value)
                }
            } else {
                format!("${}={}", setting_number, new_value)
            };

            // Send to the device first: the page must only show a value the
            // controller actually accepted.
            // A GRBL `$n=v` is tagged so its own reply decides whether the cache
            // and this page take the new value (Repetier `M206` is not tracked).
            let setting_id = if setting_number >= 1000 {
                None
            } else {
                crate::device_status::begin_setting_change(&command)
            };
            let sent = match crate::device_status::settings_command_blocked() {
                Some(reason) => Err(reason),
                None => match *comm_clone.borrow() {
                    Some(ref comm) => {
                        let mut comm_lock = comm.lock();
                        if comm_lock.is_connected() {
                            match setting_id {
                                Some(id) => comm_lock.send_tagged_command(&command, id),
                                None => comm_lock.send_command(&command),
                            }
                            .map_err(|e| e.to_string())
                        } else {
                            Err(t!("not connected").to_string())
                        }
                    }
                    None => Err(t!("not connected").to_string()),
                },
            };
            if sent.is_err()
                && let Some(id) = setting_id
            {
                crate::device_status::cancel_setting_change(id);
            }
            if let Err(reason) = sent {
                crate::ui::gtk::common::dialog::show_warning(
                    &t!("Setting not changed"),
                    &format!("{command}: {reason}"),
                    None,
                );
                dialog_save.close();
                return;
            }

            dialog_save.close();

            // Repetier EEPROM writes are acknowledged as `ok <n>`, which is not
            // tracked, so they are shown as sent.
            let Some(setting_id) = setting_id else {
                apply_setting_value(&manager_clone, setting_number, &new_value);
                refresh_callback();
                return;
            };

            // A GRBL `$n=v` is shown only once the controller accepts it.
            let manager = manager_clone.clone();
            let refresh = refresh_callback.clone();
            super::await_setting_reply(setting_id, move |outcome| {
                let failure = match outcome {
                    super::SettingOutcome::Accepted => {
                        // Show the cache's value: it follows the controller's replies in
                        // order, so a later edit's value is never replaced by an older one.
                        let value = crate::device_status::get_grbl_setting(setting_number)
                            .unwrap_or(new_value);
                        apply_setting_value(&manager, setting_number, &value);
                        refresh();
                        return;
                    }
                    super::SettingOutcome::Rejected(code) => {
                        format!("{} error:{code}", t!("the controller replied"))
                    }
                    super::SettingOutcome::NoReply => {
                        t!("no reply from the controller").to_string()
                    }
                };
                crate::ui::gtk::common::dialog::show_warning(
                    &t!("Setting not changed"),
                    &format!("${setting_number}={new_value}: {failure}"),
                    None,
                );
            });
        });
        btn_box.append(&save_btn);

        vbox.append(&btn_box);
        dialog.set_child(Some(&vbox));
        dialog.present();
    }

    pub(crate) fn save_to_file(&self) {
        let Some(window) = self.container.root().and_downcast::<gtk4::Window>() else {
            tracing::warn!("Failed to get parent window for export dialog");
            return;
        };

        let dialog = crate::ui::gtk::file_dialog::new_file_dialog(&t!("Export Settings"));
        dialog.set_initial_name(Some("grbl_settings.json"));

        let status_label = self.status_label.clone();
        let manager = self.settings_manager.clone();
        dialog.pick_save(Some(&window), move |file| {
            if let Some(path) = file.path() {
                let res = manager.borrow().export_to_file(&path);
                match res {
                    Ok(_) => status_label.set_text(&format!(
                        "{} {}",
                        t!("Exported settings to"),
                        path.display()
                    )),
                    Err(e) => status_label.set_text(&format!("{} {}", t!("Export failed:"), e)),
                }
            }
        });
    }

    pub(crate) fn load_from_file(&self) {
        let Some(window) = self.container.root().and_downcast::<gtk4::Window>() else {
            tracing::warn!("Failed to get parent window for import dialog");
            return;
        };

        let dialog = crate::ui::gtk::file_dialog::new_file_dialog(&t!("Import Settings"));

        let status_label = self.status_label.clone();
        let manager = self.settings_manager.clone();

        dialog.pick_open(Some(&window), move |file| {
            if let Some(path) = file.path() {
                let res = manager.borrow_mut().import_from_file(&path);
                match res {
                    Ok(_) => {
                        status_label.set_text(&format!(
                            "{} {}",
                            t!("Imported settings from"),
                            path.display()
                        ));
                    }
                    Err(e) => status_label.set_text(&format!("{} {}", t!("Import failed:"), e)),
                }
            }
        });
    }
}

/// Stores a new value for a setting in the Device Config list.
///
/// # Arguments
/// * `manager` - The settings shown on the page.
/// * `number` - Setting number (Repetier EEPROM positions are offset by 1000).
/// * `value` - The value the controller now holds.
fn apply_setting_value(manager: &Shared<SettingsManager>, number: u16, value: &str) {
    let mut manager = manager.borrow_mut();
    if let Some(setting) = manager.get_setting(number) {
        let mut updated = setting.clone();
        updated.value = value.to_string();
        updated.numeric_value = value.parse::<f64>().ok();
        manager.set_setting(updated);
    }
}
