//! Speeds and Feeds Calculator Tool - Milestone 4 Implementation
//!
//! Provides a comprehensive UI for the feeds and speeds calculator with
//! integration to Tools, Materials, and Devices databases.

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Dialog, Label, ListBox, ListBoxRow, Orientation,
    Paned, ResponseType, ScrolledWindow, SpinButton, Stack, Window,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::common::set_paned_initial_fraction;
use crate::t;
use crate::ui::gtk::help_browser;
use crate::ui::materials_manager_backend::MaterialsManagerBackend;
use crate::ui::tools_manager_backend::ToolsManagerBackend;
use gcodekit5_camtools::speeds_feeds::{CalculationInput, OperationType, SpeedsFeedsCalculator};
use gcodekit5_core::data::materials::Material;
use gcodekit5_core::data::tools::Tool;
use gcodekit5_devicedb::model::DeviceProfile;
use gcodekit5_devicedb::DeviceManager;
use gcodekit5_settings::SettingsController;
use std::sync::Arc;

/// State for the currently selected items
#[derive(Debug, Clone)]
struct CalculatorState {
    selected_tool: Option<Tool>,
    selected_material: Option<Material>,
    operation: OperationType,
    depth_of_cut: f32,
    width_of_cut: Option<f32>,
    tool_stick_out: f32,
    coolant_enabled: bool,
    conservative: bool,
    device_manager: Option<Arc<DeviceManager>>,
}

impl Default for CalculatorState {
    fn default() -> Self {
        // Try to load the device manager from config
        let device_manager = Self::load_device_manager();

        Self {
            selected_tool: None,
            selected_material: None,
            operation: OperationType::Slotting,
            depth_of_cut: 3.0,
            width_of_cut: None,
            tool_stick_out: 25.0,
            coolant_enabled: true,
            conservative: false,
            device_manager,
        }
    }
}

impl CalculatorState {
    /// Load the device manager from the config file
    fn load_device_manager() -> Option<Arc<DeviceManager>> {
        let config_dir = dirs::config_dir()?;
        let device_config_path = config_dir.join("gcodekit5").join("devices.json");

        eprintln!(
            "[SpeedsFeeds] Loading device manager from: {:?}",
            device_config_path
        );

        let manager = Arc::new(DeviceManager::new(device_config_path.clone()));
        match manager.load() {
            Ok(_) => {
                eprintln!("[SpeedsFeeds] Device manager loaded successfully");
                Some(manager)
            }
            Err(e) => {
                eprintln!("[SpeedsFeeds] Failed to load device manager: {:?}", e);
                None
            }
        }
    }

    /// Get the active device profile if available
    fn get_active_device(&self) -> Option<DeviceProfile> {
        let manager = self.device_manager.as_ref()?;
        eprintln!("[SpeedsFeeds] Device manager exists, getting active profile...");
        let profile = manager.get_active_profile();
        eprintln!(
            "[SpeedsFeeds] Active profile result: {:?}",
            profile.as_ref().map(|p| &p.name)
        );
        if profile.is_none() {
            eprintln!("[SpeedsFeeds] No active device profile found");
        }
        profile
    }
}

/// Labels that can be updated with calculation results
struct ResultLabels {
    spindle_speed: Label,
    feed_rate: Label,
    surface_speed: Label,
    chip_load: Label,
    mrr: Label,
    power: Label,
    deflection: Label,
    device_name: Label,
    device_rpm: Label,
    device_power: Label,
    device_max_feed: Label,
}

pub struct SpeedsFeedsTool {
    content: Box,
    #[allow(dead_code)]
    state: Rc<RefCell<CalculatorState>>,
    #[allow(dead_code)]
    result_labels: Rc<RefCell<Option<ResultLabels>>>,
}

impl SpeedsFeedsTool {
    pub fn new(stack: &Stack, _settings: Rc<SettingsController>) -> Self {
        let state = Rc::new(RefCell::new(CalculatorState::default()));
        let result_labels = Rc::new(RefCell::new(None));
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });
        header.append(&back_btn);

        let title = Label::builder()
            .label(t!("Speeds and Feeds Calculator"))
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);
        header.append(&help_browser::make_help_button("speeds_feeds_calculator"));
        content_box.append(&header);

        // Paned Layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar (35%) - Results Panel
        let (sidebar, labels) = Self::create_results_panel();

        // Populate device info immediately with active device data
        if let Some(active_device) = state.borrow().get_active_device() {
            labels.device_name.set_label(&active_device.name);
            labels
                .device_rpm
                .set_label(&active_device.max_spindle_speed_rpm.to_string());
            let power_text = if active_device.has_spindle && active_device.cnc_spindle_watts > 0.0 {
                format!("{:.0} W", active_device.cnc_spindle_watts)
            } else {
                "N/A".to_string()
            };
            labels.device_power.set_label(&power_text);
            labels
                .device_max_feed
                .set_label(&format!("{:.0}", active_device.max_feed_rate));
        } else {
            labels.device_name.set_label(&t!("No device selected"));
            labels.device_rpm.set_label("--");
            labels.device_power.set_label("--");
            labels.device_max_feed.set_label("--");
        }

        *result_labels.borrow_mut() = Some(labels);
        paned.set_start_child(Some(&sidebar));
        // Content Area (65%) - Input Panel
        let right_panel = Self::create_input_panel(&state, &result_labels);
        paned.set_end_child(Some(&right_panel));

        // Set initial paned position (35% for sidebar)
        set_paned_initial_fraction(&paned, 0.35);

        content_box.append(&paned);

        Self {
            content: content_box,
            state,
            result_labels,
        }
    }

    /// Create the results panel (left sidebar) - returns the labels for updating
    fn create_results_panel() -> (Box, ResultLabels) {
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.set_margin_top(12);
        sidebar.set_margin_bottom(12);
        sidebar.set_margin_start(12);
        sidebar.set_margin_end(12);
        sidebar.set_width_request(250);

        // Results Group
        let results_group = PreferencesGroup::builder()
            .title(t!("Calculated Results"))
            .build();

        // Spindle Speed
        let (spindle_row, spindle_label) =
            Self::create_result_row_with_label(&t!("Spindle Speed (RPM):"), "--", true);
        results_group.add(&spindle_row);

        // Feed Rate
        let (feed_row, feed_label) =
            Self::create_result_row_with_label(&t!("Feed Rate (mm/min):"), "--", true);
        results_group.add(&feed_row);

        // Surface Speed
        let (sfm_row, sfm_label) =
            Self::create_result_row_with_label(&t!("Surface Speed (SFM):"), "--", false);
        results_group.add(&sfm_row);

        // Chip Load
        let (chip_row, chip_label) =
            Self::create_result_row_with_label(&t!("Chip Load (mm/tooth):"), "--", false);
        results_group.add(&chip_row);

        // MRR
        let (mrr_row, mrr_label) =
            Self::create_result_row_with_label(&t!("MRR (mm³/min):"), "--", false);
        results_group.add(&mrr_row);

        // Power
        let (power_row, power_label) =
            Self::create_result_row_with_label(&t!("Power (W):"), "--", false);
        results_group.add(&power_row);

        // Deflection
        let (deflection_row, deflection_label) =
            Self::create_result_row_with_label(&t!("Est. Deflection (mm):"), "--", false);
        results_group.add(&deflection_row);

        sidebar.append(&results_group);

        // Warnings Section
        let warnings_group = PreferencesGroup::builder().title(t!("Warnings")).build();

        let no_warnings_label = Label::builder()
            .label(t!("No warnings"))
            .css_classes(vec!["caption", "dim-label"])
            .halign(Align::Start)
            .build();
        warnings_group.add(&no_warnings_label);

        sidebar.append(&warnings_group);

        // Device Info Section - uses same format as calculator results
        let device_info_group = PreferencesGroup::builder()
            .title(t!("Active Device"))
            .build();

        // Device Name row
        let (device_name_row, device_name_label) =
            Self::create_result_row_with_label(&t!("Device:"), &t!("Loading..."), true);
        device_info_group.add(&device_name_row);

        // Max RPM row
        let (device_rpm_row, device_rpm_label) =
            Self::create_result_row_with_label(&t!("Max RPM:"), "--", false);
        device_info_group.add(&device_rpm_row);

        // Spindle Power row
        let (device_power_row, device_power_label) =
            Self::create_result_row_with_label(&t!("Spindle Power:"), "--", false);
        device_info_group.add(&device_power_row);

        // Max Feed Rate row
        let (device_feed_row, device_feed_label) =
            Self::create_result_row_with_label(&t!("Max Feed Rate:"), "--", false);
        device_info_group.add(&device_feed_row);

        sidebar.append(&device_info_group);
        let labels = ResultLabels {
            spindle_speed: spindle_label,
            feed_rate: feed_label,
            surface_speed: sfm_label,
            chip_load: chip_label,
            mrr: mrr_label,
            power: power_label,
            deflection: deflection_label,
            device_name: device_name_label,
            device_rpm: device_rpm_label,
            device_power: device_power_label,
            device_max_feed: device_feed_label,
        };
        (sidebar, labels)
    }

    /// Create a single result row with a label that can be updated
    fn create_result_row_with_label(
        label_text: &str,
        value_text: &str,
        highlight: bool,
    ) -> (Box, Label) {
        let row = Box::new(Orientation::Horizontal, 8);
        row.set_margin_top(6);
        row.set_margin_bottom(6);

        let label = Label::builder()
            .label(label_text)
            .css_classes(vec!["caption", "dim-label"])
            .halign(Align::Start)
            .hexpand(true)
            .build();
        row.append(&label);

        let value_classes = if highlight {
            vec!["title-4"]
        } else {
            vec!["caption"]
        };
        let value = Label::builder()
            .label(value_text)
            .css_classes(value_classes)
            .halign(Align::End)
            .build();
        row.append(&value);

        (row, value)
    }

    /// Create the input panel (right side)
    fn create_input_panel(
        state: &Rc<RefCell<CalculatorState>>,
        result_labels: &Rc<RefCell<Option<ResultLabels>>>,
    ) -> Box {
        let right_panel = Box::new(Orientation::Vertical, 0);

        let scroll_content = Box::new(Orientation::Vertical, 12);
        scroll_content.set_margin_top(12);
        scroll_content.set_margin_bottom(12);
        scroll_content.set_margin_start(12);
        scroll_content.set_margin_end(12);

        // Get active device info for display
        let active_device = state.borrow().get_active_device();
        let (device_name, max_rpm, has_spindle, spindle_power_kw, has_coolant, device_max_feed) =
            active_device
                .map(|d| {
                    let has_spindle = d.has_spindle;
                    let rpm = if has_spindle {
                        d.max_spindle_speed_rpm
                    } else {
                        0
                    };
                    let power = if has_spindle {
                        d.cnc_spindle_watts as f32 / 1000.0
                    } else {
                        0.0
                    };
                    (
                        d.name,
                        rpm,
                        has_spindle,
                        power,
                        d.has_coolant,
                        d.max_feed_rate,
                    )
                })
                .unwrap_or_else(|| {
                    (
                        "No device selected".to_string(),
                        0,
                        false,
                        0.0,
                        false,
                        5000.0,
                    )
                }); // Device Info Section - Uses same format as calculator results
        let device_info_group = PreferencesGroup::builder()
            .title(t!("Active Device"))
            .build();

        // Device Name
        let (device_name_row, _device_name_label) =
            Self::create_result_row_with_label(&t!("Device:"), &device_name, true);
        device_info_group.add(&device_name_row);

        // Max RPM
        let rpm_value = if has_spindle {
            if max_rpm == 0 {
                "⚠ Not set".to_string()
            } else {
                max_rpm.to_string()
            }
        } else {
            "N/A (No spindle)".to_string()
        };
        let (device_rpm_row, _device_rpm_label) =
            Self::create_result_row_with_label(&t!("Max RPM:"), &rpm_value, false);
        device_info_group.add(&device_rpm_row);

        // Spindle Power (in Watts, not kW)
        let power_value = if has_spindle {
            if spindle_power_kw == 0.0 {
                "⚠ Not set".to_string()
            } else {
                format!("{:.0} W", spindle_power_kw * 1000.0)
            }
        } else {
            "N/A (No spindle)".to_string()
        };
        let (device_power_row, _device_power_label) =
            Self::create_result_row_with_label(&t!("Spindle Power:"), &power_value, false);
        device_info_group.add(&device_power_row);

        // Max Feed Rate
        let (device_feed_row, _device_feed_label) = Self::create_result_row_with_label(
            &t!("Max Feed Rate:"),
            &format!("{:.0} mm/min", device_max_feed),
            false,
        );
        device_info_group.add(&device_feed_row);

        scroll_content.append(&device_info_group);

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Selection Group
        let selection_group = PreferencesGroup::builder().title(t!("Selections")).build();

        // Tool Selection Row
        let tool_btn = Button::with_label(&t!("Select Tool..."));
        tool_btn.add_css_class("flat");
        let tool_label = Label::builder()
            .label(t!("No tool selected"))
            .css_classes(vec!["caption", "dim-label"])
            .build();
        let tool_row = Box::new(Orientation::Horizontal, 12);
        tool_row.set_margin_top(6);
        tool_row.set_margin_bottom(6);
        let tool_title = Label::builder()
            .label(t!("Tool:"))
            .hexpand(true)
            .halign(Align::Start)
            .build();
        tool_row.append(&tool_title);
        tool_row.append(&tool_btn);
        tool_row.append(&tool_label);
        selection_group.add(&tool_row);

        // Material Selection Row
        let material_btn = Button::with_label(&t!("Select Material..."));
        material_btn.add_css_class("flat");
        let material_label = Label::builder()
            .label(t!("No material selected"))
            .css_classes(vec!["caption", "dim-label"])
            .build();
        let material_row = Box::new(Orientation::Horizontal, 12);
        material_row.set_margin_top(6);
        material_row.set_margin_bottom(6);
        let material_title = Label::builder()
            .label(t!("Material:"))
            .hexpand(true)
            .halign(Align::Start)
            .build();
        material_row.append(&material_title);
        material_row.append(&material_btn);
        material_row.append(&material_label);
        selection_group.add(&material_row);

        // Update state with device-dependent defaults
        state.borrow_mut().coolant_enabled = has_coolant; // Set coolant based on device capability
        state.borrow_mut().conservative = true; // Default conservative to ON
        state.borrow_mut().width_of_cut = Some(1.0); // Default stepover to 1.0 mm

        // Clone labels for updating when selections are made
        let tool_label_clone = tool_label.clone();
        let material_label_clone = material_label.clone();
        scroll_content.append(&selection_group);
        // Operation Group
        let operation_group = PreferencesGroup::builder().title(t!("Operation")).build();

        let operation_combo = ComboBoxText::new();
        operation_combo.append(Some("slotting"), &t!("Slotting"));
        operation_combo.append(Some("pocketing"), &t!("Pocketing"));
        operation_combo.append(Some("profiling"), &t!("Profiling"));
        operation_combo.append(Some("adaptive"), &t!("Adaptive / HSM"));
        operation_combo.append(Some("drilling"), &t!("Drilling"));
        operation_combo.append(Some("plunging"), &t!("Plunging"));
        operation_combo.set_active_id(Some("slotting"));
        let operation_row = ActionRow::builder().title(t!("Operation Type:")).build();
        operation_row.add_suffix(&operation_combo);
        operation_group.add(&operation_row);

        scroll_content.append(&operation_group);

        // Parameters Group
        let params_group = PreferencesGroup::builder()
            .title(t!("Cutting Parameters"))
            .build();

        // Depth of Cut
        let depth_spin = SpinButton::with_range(0.1, 50.0, 0.1);
        depth_spin.set_value(3.0);
        depth_spin.set_digits(1);
        let depth_row = ActionRow::builder().title(t!("Depth of Cut (mm):")).build();
        depth_row.add_suffix(&depth_spin);
        params_group.add(&depth_row);

        // Width of Cut / Stepover - default to 1.0 mm
        let width_spin = SpinButton::with_range(0.0, 100.0, 0.1);
        width_spin.set_value(1.0); // Default to 1.0 mm
        width_spin.set_digits(1);
        width_spin.set_tooltip_text(Some(&t!("0 = Use operation default")));
        let width_row = ActionRow::builder().title(t!("Stepover (mm):")).build();
        width_row.add_suffix(&width_spin);
        params_group.add(&width_row);
        // Tool Stick-out
        let stickout_spin = SpinButton::with_range(5.0, 100.0, 1.0);
        stickout_spin.set_value(25.0);
        stickout_spin.set_digits(0);
        let stickout_row = ActionRow::builder()
            .title(t!("Tool Stick-out (mm):"))
            .build();
        stickout_row.add_suffix(&stickout_spin);
        params_group.add(&stickout_row);

        scroll_content.append(&params_group);

        // Options Group
        let options_group = PreferencesGroup::builder().title(t!("Options")).build();

        // Coolant - disabled if device doesn't support coolant
        let coolant_check = CheckButton::with_label(&t!("Coolant Enabled"));
        coolant_check.set_active(has_coolant); // Only enable if device supports it
        coolant_check.set_sensitive(has_coolant); // Disable interaction if no coolant support
        if !has_coolant {
            coolant_check.set_tooltip_text(Some(&t!("Device does not support coolant")));
        }
        let coolant_row = ActionRow::builder().title(t!("Coolant:")).build();
        coolant_row.add_suffix(&coolant_check);
        options_group.add(&coolant_row);

        // Conservative calculation - default to ON (true)
        let conservative_check = CheckButton::with_label(&t!("Conservative Calculation"));
        conservative_check.set_active(true); // Default to ON
        let conservative_row = ActionRow::builder()
            .title(t!("Safety Factor:"))
            .subtitle(t!("Reduce calculated values by 20%"))
            .build();
        conservative_row.add_suffix(&conservative_check);
        options_group.add(&conservative_row);
        scroll_content.append(&options_group);

        right_panel.append(&scrolled);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let calculate_btn = Button::with_label(&t!("Calculate"));
        calculate_btn.add_css_class("suggested-action");
        action_box.append(&calculate_btn);

        let reset_btn = Button::with_label(&t!("Reset"));
        action_box.append(&reset_btn);

        right_panel.append(&action_box);

        // Connect signals
        let state_clone = state.clone();
        let tool_label_for_btn = tool_label_clone.clone();
        tool_btn.connect_clicked(move |btn| {
            let window = btn.root().and_downcast::<gtk4::Window>();
            if let Some(parent) = window {
                let state_clone2 = state_clone.clone();
                let tool_label2 = tool_label_for_btn.clone();
                ToolSelectionDialog::show(&parent, move |tool| {
                    let name = format!("{} - ø{:.1}mm", tool.name, tool.diameter);
                    tool_label2.set_label(&name);
                    state_clone2.borrow_mut().selected_tool = Some(tool.clone());
                });
            }
        });

        let state_clone = state.clone();
        let material_label_for_btn = material_label_clone.clone();
        material_btn.connect_clicked(move |btn| {
            let window = btn.root().and_downcast::<gtk4::Window>();
            if let Some(parent) = window {
                let state_clone2 = state_clone.clone();
                let material_label2 = material_label_for_btn.clone();
                MaterialSelectionDialog::show(&parent, move |material| {
                    material_label2.set_label(&material.name);
                    state_clone2.borrow_mut().selected_material = Some(material.clone());
                });
            }
        });

        let state_clone = state.clone();
        operation_combo.connect_changed(move |combo| {
            let mut s = state_clone.borrow_mut();
            s.operation = match combo.active_id().as_deref() {
                Some("pocketing") => OperationType::Pocketing,
                Some("profiling") => OperationType::Profiling,
                Some("adaptive") => OperationType::Adaptive,
                Some("drilling") => OperationType::Drilling,
                Some("plunging") => OperationType::Plunging,
                _ => OperationType::Slotting,
            };
        });

        let state_clone = state.clone();
        depth_spin.connect_value_changed(move |spin| {
            state_clone.borrow_mut().depth_of_cut = spin.value() as f32;
        });

        let state_clone = state.clone();
        width_spin.connect_value_changed(move |spin| {
            let val = spin.value() as f32;
            state_clone.borrow_mut().width_of_cut = if val > 0.0 { Some(val) } else { None };
        });

        let state_clone = state.clone();
        stickout_spin.connect_value_changed(move |spin| {
            state_clone.borrow_mut().tool_stick_out = spin.value() as f32;
        });

        let state_clone = state.clone();
        coolant_check.connect_toggled(move |check| {
            state_clone.borrow_mut().coolant_enabled = check.is_active();
        });

        let state_clone = state.clone();
        conservative_check.connect_toggled(move |check| {
            state_clone.borrow_mut().conservative = check.is_active();
        });

        // Calculate button - performs the actual calculation
        let state_clone = state.clone();
        let result_labels_clone = result_labels.clone();
        calculate_btn.connect_clicked(move |_| {
            let state = state_clone.borrow();

                          if let (Some(tool), Some(material)) = (&state.selected_tool, &state.selected_material) {
                              // Get active device or use defaults
                              let active_device = state.get_active_device();

                              // Store device info for later display (clone before we consume active_device)
                              let device_info = active_device.clone();

                              let (max_rpm, has_spindle, spindle_power_kw) = active_device
                                  .map(|d| (d.max_spindle_speed_rpm, d.has_spindle, d.cnc_spindle_watts as f32 / 1000.0))
                                  .unwrap_or((0, false, 0.0));                // Check if spindle is configured
                if !has_spindle {
                    let dialog = gtk4::MessageDialog::builder()
                        .title(t!("No Spindle Configured"))
                        .text(t!("The active device does not have a spindle configured. Please configure a spindle in the Device Manager."))
                        .message_type(gtk4::MessageType::Warning)
                        .buttons(gtk4::ButtonsType::Ok)
                        .modal(true)
                        .build();
                    dialog.connect_response(|d, _| d.close());
                    dialog.show();
                    return;
                }

                // Check required spindle parameters
                let mut missing_params = Vec::new();
                if max_rpm == 0 {
                    missing_params.push("Maximum Spindle RPM");
                }
                if spindle_power_kw == 0.0 {
                    missing_params.push("Spindle Power (watts)");
                }

                if !missing_params.is_empty() {
                    let param_list = missing_params.join(", ");
                    let dialog = gtk4::MessageDialog::builder()
                        .title(t!("Spindle Configuration Required"))
                        .text(format!("The following spindle parameters are not configured:\n\n{}\n\nPlease configure these values in the Device Manager.", param_list))
                        .message_type(gtk4::MessageType::Warning)
                        .buttons(gtk4::ButtonsType::Ok)
                        .modal(true)
                        .build();
                    dialog.connect_response(|d, _| d.close());
                    dialog.show();
                    return;
                }

                                                                                                          // Create device profile for calculation using active device settings
                                                                                                          let device = DeviceProfile {
                                                                                                              max_spindle_speed_rpm: max_rpm,
                                                                                                              max_feed_rate: 5000.0,
                                                                                                              ..Default::default()
                                                                                                          };

                                                                                                          let input = CalculationInput {                                      material: material.clone(),
                                      tool: tool.clone(),
                                      device: device.clone(),
                                      operation: state.operation,
                                      depth_of_cut: state.depth_of_cut,
                                      width_of_cut: state.width_of_cut,
                                      tool_stick_out: state.tool_stick_out,
                                      coolant_enabled: state.coolant_enabled,
                                      conservative: state.conservative,
                                  };
                                  let result = SpeedsFeedsCalculator::calculate(&input);
                // Clamp RPM to max device RPM if needed
                let clamped_rpm = if result.rpm > max_rpm {
                    max_rpm
                } else {
                    result.rpm
                };

                // Calculate adjusted feed rate based on clamped RPM
                let rpm_ratio = if result.rpm > 0 {
                    clamped_rpm as f32 / result.rpm as f32
                } else {
                    1.0
                };
                let adjusted_feed_rate = result.feed_rate * rpm_ratio;

                                  // Update the result labels
                                  if let Some(labels) = result_labels_clone.borrow().as_ref() {
                                      // Show clamped RPM with original in parentheses if clamped
                                      if clamped_rpm != result.rpm {
                                          labels.spindle_speed.set_label(&format!("{} (clamped from {})", clamped_rpm, result.rpm));
                                          labels.feed_rate.set_label(&format!("{:.1} (adjusted)", adjusted_feed_rate));
                                      } else {
                                          labels.spindle_speed.set_label(&clamped_rpm.to_string());
                                          labels.feed_rate.set_label(&format!("{:.1}", result.feed_rate));
                                      }
                                      labels.surface_speed.set_label(&format!("{:.1}", result.surface_speed_m_min));
                                      labels.chip_load.set_label(&format!("{:.4}", result.chip_load_mm));
                                      labels.mrr.set_label(&format!("{:.1}", result.material_removal_rate));
                                      labels.power.set_label(&format!("{:.0}", result.power_required_kw * 1000.0));
                                      labels.deflection.set_label(&format!("{:.4}", result.estimated_deflection_mm));

                                                                              // Update device info section
                                                                              if let Some(ref device) = device_info {
                                                                                  labels.device_name.set_label(&device.name.to_string());
                                                                                  labels.device_rpm.set_label(&format!("Max RPM: {}", device.max_spindle_speed_rpm));
                                                                                                                                                                      let power_text = if device.has_spindle && device.cnc_spindle_watts > 0.0 {                                                                                      format!("Spindle: {:.1} kW", device.cnc_spindle_watts as f32 / 1000.0)
                                                                                  } else {
                                                                                      "Spindle: N/A".to_string()
                                                                                  };
                                                                                  labels.device_power.set_label(&power_text);
                                                                                  labels.device_max_feed.set_label(&format!("Max Feed: {:.0} mm/min", device.max_feed_rate));
                                                                              } else {
                                                                                  labels.device_name.set_label(&t!("No device selected"));
                                                                                  labels.device_rpm.set_label("");
                                                                                  labels.device_power.set_label("");
                                                                                  labels.device_max_feed.set_label("");
                                                                              }                                  }            }
        });

        let state_clone = state.clone();
        reset_btn.connect_clicked(move |_| {
            *state_clone.borrow_mut() = CalculatorState::default();
        });

        right_panel
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }
}

/// Tool Selection Dialog
pub struct ToolSelectionDialog;

impl ToolSelectionDialog {
    pub fn show(parent: &Window, on_select: impl Fn(&Tool) + 'static) {
        let dialog = Dialog::builder()
            .title(t!("Select Tool"))
            .modal(true)
            .transient_for(parent)
            .build();

        dialog.add_button(&t!("Cancel"), ResponseType::Cancel);
        dialog.add_button(&t!("Select"), ResponseType::Accept);

        let content = dialog.content_area();
        content.set_spacing(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        // Search entry
        let search_entry = gtk4::SearchEntry::new();
        search_entry.set_placeholder_text(Some(&t!("Search tools...")));
        content.append(&search_entry);

        // Tool list
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .min_content_height(300)
            .build();

        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        scrolled.set_child(Some(&list_box));
        content.append(&scrolled);

        // Load tools from backend
        let backend = ToolsManagerBackend::new();
        let mut tools: Vec<Tool> = backend.get_all_tools().into_iter().cloned().collect();

        // Sort tools alphabetically by name, with numbers having priority
        tools.sort_by(|a, b| {
            // Extract leading numbers for numeric comparison
            let a_num: Option<u32> = a
                .name
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok();
            let b_num: Option<u32> = b
                .name
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok();

            match (a_num, b_num) {
                (Some(a_n), Some(b_n)) => a_n.cmp(&b_n).then_with(|| a.name.cmp(&b.name)),
                (Some(_), None) => std::cmp::Ordering::Less, // Numbers come first
                (None, Some(_)) => std::cmp::Ordering::Greater, // Numbers come first
                (None, None) => a.name.cmp(&b.name),         // Alphabetical sort
            }
        });
        let tools_map: Rc<RefCell<HashMap<String, Tool>>> = Rc::new(RefCell::new(HashMap::new()));

        if tools.is_empty() {
            // Show placeholder when no tools
            let row = Box::new(Orientation::Vertical, 8);
            row.set_margin_top(20);
            row.set_margin_bottom(20);

            let label = Label::builder()
                .label(t!("No tools available"))
                .css_classes(vec!["title-3"])
                .build();
            row.append(&label);

            let hint = Label::builder()
                .label(t!("Add tools in the CNC Tools manager"))
                .css_classes(vec!["caption", "dim-label"])
                .build();
            row.append(&hint);

            list_box.append(&row);
        } else {
            for tool in tools {
                let row = ListBoxRow::new();
                let content = Box::new(Orientation::Horizontal, 12);
                content.set_margin_top(6);
                content.set_margin_bottom(6);

                let name = Label::builder()
                    .label(format!(
                        "{} - ø{:.1}mm {}",
                        tool.name, tool.diameter, tool.tool_type
                    ))
                    .halign(Align::Start)
                    .hexpand(true)
                    .build();
                content.append(&name);

                let info = Label::builder()
                    .label(format!("{} flutes", tool.flutes))
                    .css_classes(vec!["caption", "dim-label"])
                    .build();
                content.append(&info);

                // Store tool_id on the ListBoxRow
                let tool_id = tool.id.0.clone();
                unsafe {
                    row.set_data("tool_id", tool_id.clone());
                }
                tools_map.borrow_mut().insert(tool_id, tool.clone());

                row.set_child(Some(&content));
                list_box.append(&row);
            }
        }

        // Track selected row
        let selected_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let _selected_id_for_dblclick = selected_id.clone();
        let _tools_map_for_dblclick = tools_map.clone();
        let on_select_clone = Rc::new(RefCell::new(Some(on_select)));
        let _on_select_for_dblclick = on_select_clone.clone();
        let dialog_clone = dialog.clone();
        list_box.connect_row_selected({
            let selected_id = selected_id.clone();
            move |_, row| {
                if let Some(r) = row {
                    // Retrieve tool_id directly from the ListBoxRow
                    let tool_id: Option<String> =
                        unsafe { r.data::<String>("tool_id").map(|d| d.as_ref().clone()) };
                    *selected_id.borrow_mut() = tool_id;
                } else {
                    *selected_id.borrow_mut() = None;
                }
            }
        });

        // Support double-click to select
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(gtk4::gdk::BUTTON_PRIMARY);
        gesture.connect_pressed({
            let _dialog = dialog_clone.clone();
            move |gesture, n_press, _x, _y| {
                if n_press == 2 {
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                    dialog_clone.response(ResponseType::Accept);
                }
            }
        });
        list_box.add_controller(gesture);
        dialog.connect_response({
            let selected_id = selected_id.clone();
            let tools_map = tools_map.clone();
            let on_select_clone = on_select_clone.clone();
            move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(id) = selected_id.borrow().as_ref() {
                        if let Some(tool) = tools_map.borrow().get(id) {
                            if let Some(ref on_select) = *on_select_clone.borrow() {
                                on_select(tool);
                            }
                        }
                    }
                }
                dialog.close();
            }
        });
        dialog.show();
    }
}

/// Material Selection Dialog
pub struct MaterialSelectionDialog;

impl MaterialSelectionDialog {
    pub fn show(parent: &Window, on_select: impl Fn(&Material) + 'static) {
        let dialog = Dialog::builder()
            .title(t!("Select Material"))
            .modal(true)
            .transient_for(parent)
            .build();

        dialog.add_button(&t!("Cancel"), ResponseType::Cancel);
        dialog.add_button(&t!("Select"), ResponseType::Accept);

        let content = dialog.content_area();
        content.set_spacing(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        // Search entry
        let search_entry = gtk4::SearchEntry::new();
        search_entry.set_placeholder_text(Some(&t!("Search materials...")));
        content.append(&search_entry);

        // Category filter
        let category_combo = ComboBoxText::new();
        category_combo.append(Some("all"), &t!("All Categories"));
        category_combo.append(Some("metal"), &t!("Metals"));
        category_combo.append(Some("wood"), &t!("Wood"));
        category_combo.append(Some("plastic"), &t!("Plastics"));
        category_combo.append(Some("composite"), &t!("Composites"));
        category_combo.set_active_id(Some("all"));
        content.append(&category_combo);

        // Material list
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .min_content_height(300)
            .build();

        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        scrolled.set_child(Some(&list_box));
        content.append(&scrolled);

        // Load materials from backend
        let backend = MaterialsManagerBackend::new();
        let mut materials: Vec<Material> =
            backend.get_all_materials().into_iter().cloned().collect();

        // Sort materials alphabetically by name, with numbers having priority
        materials.sort_by(|a, b| {
            // Extract leading numbers for numeric comparison
            let a_num: Option<u32> = a
                .name
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok();
            let b_num: Option<u32> = b
                .name
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok();

            match (a_num, b_num) {
                (Some(a_n), Some(b_n)) => a_n.cmp(&b_n).then_with(|| a.name.cmp(&b.name)),
                (Some(_), None) => std::cmp::Ordering::Less, // Numbers come first
                (None, Some(_)) => std::cmp::Ordering::Greater, // Numbers come first
                (None, None) => a.name.cmp(&b.name),         // Alphabetical sort
            }
        });
        let materials_map: Rc<RefCell<HashMap<String, Material>>> =
            Rc::new(RefCell::new(HashMap::new()));

        if materials.is_empty() {
            // Show placeholder when no materials
            let row = Box::new(Orientation::Vertical, 8);
            row.set_margin_top(20);
            row.set_margin_bottom(20);

            let label = Label::builder()
                .label(t!("No materials available"))
                .css_classes(vec!["title-3"])
                .build();
            row.append(&label);

            let hint = Label::builder()
                .label(t!("Add materials in the Materials manager"))
                .css_classes(vec!["caption", "dim-label"])
                .build();
            row.append(&hint);

            list_box.append(&row);
        } else {
            for material in materials {
                let row = ListBoxRow::new();
                let content = Box::new(Orientation::Vertical, 4);
                content.set_margin_top(8);
                content.set_margin_bottom(8);

                let name_row = Box::new(Orientation::Horizontal, 8);
                let name_label = Label::builder()
                    .label(&material.name)
                    .halign(Align::Start)
                    .hexpand(true)
                    .build();
                name_row.append(&name_label);

                let cat_label = Label::builder()
                    .label(format!("{:?}", material.category))
                    .css_classes(vec!["caption", "dim-label"])
                    .build();
                name_row.append(&cat_label);

                content.append(&name_row);

                // Use machinability_rating directly
                if material.machinability_rating > 0 {
                    let info_row = Box::new(Orientation::Horizontal, 8);
                    let info_label = Label::builder()
                        .label(format!(
                            "Machinability: {}/10",
                            material.machinability_rating
                        ))
                        .css_classes(vec!["caption"])
                        .halign(Align::Start)
                        .build();
                    info_row.append(&info_label);
                    content.append(&info_row);
                }

                // Store material_id on the ListBoxRow
                let material_id = material.id.0.clone();
                unsafe {
                    row.set_data("material_id", material_id.clone());
                }
                materials_map
                    .borrow_mut()
                    .insert(material_id, material.clone());

                row.set_child(Some(&content));
                list_box.append(&row);
            }
        }

        // Track selected row
        let selected_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let _selected_id_for_click = selected_id.clone();
        let dialog_for_click = dialog.clone();
        list_box.connect_row_selected({
            let selected_id = selected_id.clone();
            move |_, row| {
                if let Some(r) = row {
                    let material_id: Option<String> =
                        unsafe { r.data::<String>("material_id").map(|d| d.as_ref().clone()) };
                    *selected_id.borrow_mut() = material_id;
                } else {
                    *selected_id.borrow_mut() = None;
                }
            }
        });

        // Support double-click to select
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(gtk4::gdk::BUTTON_PRIMARY);
        gesture.connect_pressed({
            move |gesture, n_press, _x, _y| {
                if n_press == 2 {
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                    dialog_for_click.response(ResponseType::Accept);
                }
            }
        });
        list_box.add_controller(gesture);
        dialog.connect_response({
            let selected_id = selected_id.clone();
            let materials_map = materials_map.clone();
            move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(id) = selected_id.borrow().as_ref() {
                        if let Some(material) = materials_map.borrow().get(id) {
                            on_select(material);
                        }
                    }
                }
                dialog.close();
            }
        });

        dialog.show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculator_state_default() {
        let state = CalculatorState::default();
        assert_eq!(state.operation, OperationType::Slotting);
        assert_eq!(state.depth_of_cut, 3.0);
        assert_eq!(state.tool_stick_out, 25.0);
        assert!(state.coolant_enabled);
        assert!(!state.conservative);
    }
}
