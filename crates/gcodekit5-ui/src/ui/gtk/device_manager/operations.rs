//! Event handlers and operations for the device manager window.

use super::*;

impl DeviceManagerWindow {
    pub(crate) fn setup_event_handlers(self: &Rc<Self>) {
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

        // Sync from Device button
        let view = self.clone();
        self.sync_btn.connect_clicked(move |_| {
            view.sync_from_device();
        });

        // Set Active button
        let view = self.clone();
        self.set_active_btn.connect_clicked(move |_| {
            view.set_as_active();
        });

        // Search
        let view = self.clone();
        self.search_entry
            .connect_search_changed(move |_| view.load_devices());

        // Connection field toggles
        let view = self.clone();
        self.edit_connection_type
            .connect_changed(move |_| view.update_connection_field_sensitivity());

        // Capability field toggles
        let view = self.clone();
        self.edit_has_spindle
            .connect_toggled(move |_| view.update_capabilities_field_sensitivity());
        let view = self.clone();
        self.edit_has_laser
            .connect_toggled(move |_| view.update_capabilities_field_sensitivity());

        // List selection
        let view = self.clone();
        self.devices_list.connect_row_activated(move |_, row| {
            let device_id = row.widget_name();
            if !device_id.is_empty() {
                view.load_device_for_edit(device_id.as_str());
            }
        });
    }

    pub(crate) fn load_devices(&self) {
        // Clear list
        while let Some(child) = self.devices_list.first_child() {
            self.devices_list.remove(&child);
        }

        let mut profiles = self.controller.get_ui_profiles();
        let q = self.search_entry.text().trim().to_lowercase();
        if !q.is_empty() {
            profiles.retain(|p| {
                let hay = format!(
                    "{}\n{}\n{}\n{}\n{}",
                    p.name, p.description, p.device_type, p.controller_type, p.connection_type
                )
                .to_lowercase();
                hay.contains(&q)
            });
        }

        let status = device_status::get_status();
        let connected_port = status.port_name.clone().unwrap_or_default();
        let is_connected = status.is_connected;

        for profile in profiles {
            let row_box = Box::new(Orientation::Vertical, 5);

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

            // Connected badge (serial-only for now)
            if is_connected
                && profile.connection_type == "Serial"
                && !connected_port.is_empty()
                && profile.port.trim() == connected_port.trim()
            {
                let badge = Label::new(Some("Connected"));
                badge.add_css_class("active-badge");
                badge.set_halign(Align::End);
                badge.set_valign(Align::Center);
                details_box.append(&badge);
            }

            // Active Badge
            if profile.is_active {
                let badge = Label::new(Some("Active"));
                badge.add_css_class("active-badge");
                badge.set_halign(Align::End);
                badge.set_valign(Align::Center);
                details_box.append(&badge);
            }

            row_box.append(&details_box);

            let row = ListBoxRow::new();
            row.set_margin_top(5);
            row.set_margin_bottom(5);
            row.set_margin_start(10);
            row.set_margin_end(10);
            row.set_child(Some(&row_box));
            row.set_widget_name(&profile.id);

            self.devices_list.append(&row);
        }
    }

    pub(crate) fn start_create_new(&self) {
        if let Ok(id) = self.controller.create_new_profile() {
            self.load_devices();
            self.load_device_for_edit(&id);
        } else {
            self.load_devices();
        }
    }

    pub(crate) fn load_device_for_edit(&self, device_id: &str) {
        let profiles = self.controller.get_ui_profiles();

        if let Some(profile) = profiles.iter().find(|p| p.id == device_id) {
            *self.selected_device.borrow_mut() = Some(profile.clone());

            // Switch to Edit View
            self.edit_stack.set_visible_child_name("edit_form");

            // Load into form
            self.edit_name.set_text(&profile.name);
            self.edit_description.set_text(&profile.description);

            self.edit_device_type
                .set_active_id(Some(profile.device_type.as_str()));
            self.edit_controller_type
                .set_active_id(Some(profile.controller_type.as_str()));

            self.edit_connection_type
                .set_active_id(Some(profile.connection_type.as_str()));
            self.edit_port.set_text(&profile.port);
            self.edit_baud_rate
                .set_active_id(Some(profile.baud_rate.as_str()));
            self.edit_tcp_port.set_text(profile.tcp_port.trim());
            self.edit_timeout.set_text(profile.timeout_ms.trim());
            self.edit_auto_reconnect.set_active(profile.auto_reconnect);

            self.refresh_units_display();

            self.edit_has_spindle.set_active(profile.has_spindle);
            self.edit_has_laser.set_active(profile.has_laser);
            self.edit_has_coolant.set_active(profile.has_coolant);
            self.edit_num_axes.set_text(profile.num_axes.trim());
            self.edit_max_s_value.set_text(profile.max_s_value.trim());
            self.edit_spindle_watts
                .set_text(profile.cnc_spindle_watts.trim());
            self.edit_max_spindle_speed_rpm
                .set_text(profile.max_spindle_speed_rpm.trim());
            self.edit_laser_watts.set_text(profile.laser_watts.trim());

            self.update_connection_field_sensitivity();
            self.update_capabilities_field_sensitivity();

            // Enable buttons
            self.save_btn.set_sensitive(true);
            self.cancel_btn.set_sensitive(true);
            self.delete_btn.set_sensitive(true);
            self.set_active_btn.set_sensitive(!profile.is_active);

            // Enable sync button only if a device is connected
            let status = device_status::get_status();
            self.sync_btn.set_sensitive(status.is_connected);
        }
    }

    pub(crate) fn update_connection_field_sensitivity(&self) {
        let conn = self
            .edit_connection_type
            .active_id()
            .map(|s| s.to_string())
            .or_else(|| {
                self.edit_connection_type
                    .active_text()
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Serial".to_string());

        match conn.as_str() {
            "Serial" => {
                self.edit_port.set_sensitive(true);
                self.edit_baud_rate.set_sensitive(true);
                self.edit_tcp_port.set_sensitive(false);
            }
            "TCP/IP" | "WebSocket" => {
                self.edit_port.set_sensitive(false);
                self.edit_baud_rate.set_sensitive(false);
                self.edit_tcp_port.set_sensitive(true);
            }
            _ => {}
        }
    }

    pub(crate) fn update_capabilities_field_sensitivity(&self) {
        let has_spindle = self.edit_has_spindle.is_active();
        self.edit_spindle_watts.set_sensitive(has_spindle);
        self.edit_max_spindle_speed_rpm.set_sensitive(has_spindle);

        let has_laser = self.edit_has_laser.is_active();
        self.edit_laser_watts.set_sensitive(has_laser);
    }

    pub(crate) fn show_error_dialog(&self, title: &str, details: &str) {
        let parent = crate::ui::gtk::file_dialog::parent_window(&self.widget);
        crate::ui::gtk::file_dialog::show_error_dialog(title, details, parent.as_ref());
    }

    pub(crate) fn save_device(&self) {
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
            let tcp_port: u16 = match self.edit_tcp_port.text().trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    self.show_error_dialog("Invalid TCP Port", "TCP Port must be a number");
                    return;
                }
            };
            let timeout_ms: u32 = match self.edit_timeout.text().trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    self.show_error_dialog("Invalid Timeout", "Timeout must be a number (ms)");
                    return;
                }
            };
            model.tcp_port = tcp_port.to_string();
            model.timeout_ms = timeout_ms.to_string();
            model.auto_reconnect = self.edit_auto_reconnect.is_active();

            // Dimensions
            let units = *self.current_units.borrow();
            let feed_units = *self.current_feed_units.borrow();

            let x_min_mm = match parse_length(&self.edit_x_min.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid X Min", &e);
                    return;
                }
            };
            let x_max_mm = match parse_length(&self.edit_x_max.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid X Max", &e);
                    return;
                }
            };
            let y_min_mm = match parse_length(&self.edit_y_min.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid Y Min", &e);
                    return;
                }
            };
            let y_max_mm = match parse_length(&self.edit_y_max.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid Y Max", &e);
                    return;
                }
            };
            let z_min_mm = match parse_length(&self.edit_z_min.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid Z Min", &e);
                    return;
                }
            };
            let z_max_mm = match parse_length(&self.edit_z_max.text(), units) {
                Ok(v) => v,
                Err(e) => {
                    self.show_error_dialog("Invalid Z Max", &e);
                    return;
                }
            };
            model.x_min = format!("{:.2}", x_min_mm);
            model.x_max = format!("{:.2}", x_max_mm);
            model.y_min = format!("{:.2}", y_min_mm);
            model.y_max = format!("{:.2}", y_max_mm);
            model.z_min = format!("{:.2}", z_min_mm);
            model.z_max = format!("{:.2}", z_max_mm);

            // Capabilities
            model.has_spindle = self.edit_has_spindle.is_active();
            model.has_laser = self.edit_has_laser.is_active();
            model.has_coolant = self.edit_has_coolant.is_active();

            let num_axes: u8 = match self.edit_num_axes.text().trim().parse() {
                Ok(v) if (1..=6).contains(&v) => v,
                _ => {
                    self.show_error_dialog(
                        "Invalid No of Axes",
                        "Number of axes must be an integer between 1 and 6",
                    );
                    return;
                }
            };
            model.num_axes = num_axes.to_string();

            let max_feed_mm_per_min =
                match parse_feed_rate(&self.edit_max_feed_rate.text(), feed_units) {
                    Ok(v) => v,
                    Err(e) => {
                        self.show_error_dialog("Invalid Max Feed Rate", &e);
                        return;
                    }
                };
            let max_s_value: f32 = match self.edit_max_s_value.text().trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    self.show_error_dialog("Invalid Max S-Value", "Max S-Value must be a number");
                    return;
                }
            };
            let (spindle_watts, max_spindle_speed_rpm) = if model.has_spindle {
                let spindle_watts: f32 = match self.edit_spindle_watts.text().trim().parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.show_error_dialog(
                            "Invalid Spindle Power",
                            "Spindle power must be a number (W)",
                        );
                        return;
                    }
                };

                let max_spindle_speed_rpm: u32 =
                    match self.edit_max_spindle_speed_rpm.text().trim().parse() {
                        Ok(v) => v,
                        Err(_) => {
                            self.show_error_dialog(
                                "Invalid Max Spindle Speed",
                                "Max spindle speed must be an integer (RPM)",
                            );
                            return;
                        }
                    };

                (spindle_watts, max_spindle_speed_rpm)
            } else {
                (0.0, 0)
            };

            let laser_watts: f32 = if model.has_laser {
                match self.edit_laser_watts.text().trim().parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.show_error_dialog(
                            "Invalid Laser Power",
                            "Laser power must be a number (W)",
                        );
                        return;
                    }
                }
            } else {
                0.0
            };
            model.max_feed_rate = format!("{:.0}", max_feed_mm_per_min);
            model.max_s_value = format!("{:.0}", max_s_value);
            model.max_spindle_speed_rpm = max_spindle_speed_rpm.to_string();
            model.cnc_spindle_watts = format!("{:.0}", spindle_watts);
            model.laser_watts = format!("{:.0}", laser_watts);

            // Save
            if let Err(e) = self.controller.update_profile_from_ui(model) {
                error!("Failed to save device profile: {}", e);
                self.show_error_dialog("Failed to save device", &e.to_string());
                return;
            }

            self.load_devices();
            self.cancel_edit();
        }
    }

    pub(crate) fn delete_device(self: &Rc<Self>) {
        let Some(model) = self.selected_device.borrow().clone() else {
            return;
        };

        let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() else {
            return;
        };

        let dialog = MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(MessageType::Question)
            .buttons(gtk4::ButtonsType::YesNo)
            .text("Delete device?")
            .secondary_text(format!(
                "Delete \u{201c}{}\u{201d}? This cannot be undone.",
                model.name
            ))
            .build();

        let view = self.clone();
        dialog.connect_response(move |d, resp| {
            if resp == ResponseType::Yes {
                let _ = view.controller.delete_profile(&model.id);
                view.load_devices();
                view.cancel_edit();
            }
            d.close();
        });

        dialog.show();
    }

    pub(crate) fn set_as_active(&self) {
        let id_opt = self.selected_device.borrow().as_ref().map(|d| d.id.clone());
        if let Some(id) = id_opt {
            let _ = self.controller.set_active_profile(&id);
            if let Some(profile) = self.selected_device.borrow().as_ref() {
                let num: u8 = profile.num_axes.trim().parse().unwrap_or(3);
                crate::device_status::set_active_num_axes(num);
            }
            self.load_devices();
            self.cancel_edit();
        }
    }

    pub(crate) fn cancel_edit(&self) {
        *self.selected_device.borrow_mut() = None;
        self.edit_stack.set_visible_child_name("placeholder");
        self.save_btn.set_sensitive(false);
        self.cancel_btn.set_sensitive(false);
        self.delete_btn.set_sensitive(false);
        self.sync_btn.set_sensitive(false);
        self.set_active_btn.set_sensitive(false);
    }

    pub(crate) fn sync_from_device(&self) {
        let status = device_status::get_status();

        if !status.is_connected {
            self.show_error_dialog("Not Connected", "Please connect to a device first.");
            return;
        }

        let model_opt = self.selected_device.borrow().clone();
        let Some(mut model) = model_opt else {
            return;
        };

        // Update dimensions from $130, $131, $132 (X/Y/Z max travel)
        if let Some(x_max) = status.grbl_settings.get(&130) {
            if let Ok(val) = x_max.parse::<f64>() {
                model.x_max = format!("{:.2}", val);
            }
        }
        if let Some(y_max) = status.grbl_settings.get(&131) {
            if let Ok(val) = y_max.parse::<f64>() {
                model.y_max = format!("{:.2}", val);
            }
        }
        if let Some(z_max) = status.grbl_settings.get(&132) {
            if let Ok(val) = z_max.parse::<f64>() {
                model.z_max = format!("{:.2}", val);
            }
        }

        // Update max spindle speed from $30 (Max spindle speed, RPM)
        if let Some(max_rpm) = status.grbl_settings.get(&30) {
            if let Ok(val) = max_rpm.parse::<u32>() {
                model.max_spindle_speed_rpm = val.to_string();
                model.max_s_value = format!("{:.0}", val as f32);
                if val > 0 {
                    model.has_spindle = true;
                }
            }
        }

        // Check for laser mode from $32 (Laser mode enable)
        if let Some(laser_mode) = status.grbl_settings.get(&32) {
            if let Ok(val) = laser_mode.parse::<u32>() {
                model.has_laser = val == 1;
            }
        }

        // Update firmware version if available
        if let Some(fw_version) = &status.firmware_version {
            model.description = format!(
                "{} (Firmware: {})",
                model
                    .description
                    .split('(')
                    .next()
                    .unwrap_or(&model.description)
                    .trim(),
                fw_version
            );
        }

        // Reload the updated model into the form
        *self.selected_device.borrow_mut() = Some(model.clone());
        self.edit_name.set_text(&model.name);
        self.edit_description.set_text(&model.description);

        self.refresh_units_display();

        self.edit_has_spindle.set_active(model.has_spindle);
        self.edit_has_laser.set_active(model.has_laser);
        self.edit_max_spindle_speed_rpm
            .set_text(&model.max_spindle_speed_rpm);
        self.edit_max_s_value.set_text(&model.max_s_value);

        self.update_capabilities_field_sensitivity();

        // Show success message
        let Some(window) = self.widget.root().and_downcast::<gtk4::Window>() else {
            return;
        };

        let dialog = MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(MessageType::Info)
            .buttons(gtk4::ButtonsType::Ok)
            .text("Device Information Updated")
            .secondary_text("Device dimensions, spindle speed, and capabilities have been updated from the connected device. Click Save to apply changes.")
            .build();

        dialog.connect_response(|d, _| d.close());
        dialog.show();
    }
}
