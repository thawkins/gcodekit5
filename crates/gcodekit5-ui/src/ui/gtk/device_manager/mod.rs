mod operations;
mod tabs;

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, Grid, Label, ListBox, ListBoxRow,
    MessageDialog, MessageType, Orientation, Paned, PolicyType, ResponseType, ScrolledWindow,
    SearchEntry, Stack, StackSwitcher,
};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::error;

use crate::ui::gtk::help_browser;

use crate::device_status;
use gcodekit5_devicedb::ui_integration::{DeviceProfileUiModel, DeviceUiController};

use gcodekit5_core::units::{
    format_feed_rate, format_length, get_unit_label, parse_feed_rate, parse_length, FeedRateUnits,
    MeasurementSystem,
};
use gcodekit5_settings::controller::SettingsController;
use gcodekit5_settings::manager::SettingsManager;

#[derive(Clone)]
pub struct DeviceManagerWindow {
    pub widget: Paned,
    pub(crate) controller: Rc<DeviceUiController>,
    pub(crate) current_units: Rc<RefCell<MeasurementSystem>>,
    pub(crate) current_feed_units: Rc<RefCell<FeedRateUnits>>,
    pub(crate) devices_list: ListBox,
    pub(crate) search_entry: SearchEntry,

    // Edit form widgets
    pub(crate) edit_name: Entry,
    pub(crate) edit_description: Entry,
    pub(crate) edit_device_type: ComboBoxText,
    pub(crate) edit_controller_type: ComboBoxText,
    pub(crate) edit_connection_type: ComboBoxText,
    pub(crate) edit_port: Entry,
    pub(crate) edit_baud_rate: ComboBoxText,
    pub(crate) edit_tcp_port: Entry,
    pub(crate) edit_timeout: Entry,
    pub(crate) edit_auto_reconnect: CheckButton,
    pub(crate) edit_x_min: Entry,
    pub(crate) edit_x_max: Entry,
    pub(crate) edit_y_min: Entry,
    pub(crate) edit_y_max: Entry,
    pub(crate) edit_z_min: Entry,
    pub(crate) edit_z_max: Entry,
    pub(crate) edit_x_min_unit: Label,
    pub(crate) edit_x_max_unit: Label,
    pub(crate) edit_y_min_unit: Label,
    pub(crate) edit_y_max_unit: Label,
    pub(crate) edit_z_min_unit: Label,
    pub(crate) edit_z_max_unit: Label,
    pub(crate) edit_has_spindle: CheckButton,
    pub(crate) edit_has_laser: CheckButton,
    pub(crate) edit_has_coolant: CheckButton,
    pub(crate) edit_num_axes: Entry,
    pub(crate) edit_max_feed_rate: Entry,
    pub(crate) edit_max_feed_rate_unit: Label,
    pub(crate) edit_max_s_value: Entry,
    pub(crate) edit_spindle_watts: Entry,
    pub(crate) edit_max_spindle_speed_rpm: Entry,
    pub(crate) edit_laser_watts: Entry,

    // State
    pub(crate) selected_device: Rc<RefCell<Option<DeviceProfileUiModel>>>,

    // Action buttons
    pub(crate) save_btn: Button,
    pub(crate) cancel_btn: Button,
    pub(crate) delete_btn: Button,
    pub(crate) sync_btn: Button,
    pub(crate) set_active_btn: Button,
    pub(crate) new_btn: Button,

    // UI Stacks
    pub(crate) edit_stack: Stack,
}

impl DeviceManagerWindow {
    pub fn new(
        controller: Rc<DeviceUiController>,
        settings_controller: Rc<SettingsController>,
    ) -> Rc<Self> {
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        // Units are stored internally as mm (and mm/min); convert for display/input.
        let (initial_units, initial_feed_units) =
            if let Ok(path) = SettingsManager::config_file_path() {
                if let Ok(mgr) = SettingsManager::load_from_file(&path) {
                    (
                        mgr.config().ui.measurement_system,
                        mgr.config().ui.feed_rate_units,
                    )
                } else {
                    (MeasurementSystem::Metric, FeedRateUnits::default())
                }
            } else {
                (MeasurementSystem::Metric, FeedRateUnits::default())
            };

        let current_units = Rc::new(RefCell::new(initial_units));
        let current_feed_units = Rc::new(RefCell::new(initial_feed_units));

        // LEFT SIDEBAR - Devices List
        let sidebar = Box::new(Orientation::Vertical, 10);
        sidebar.add_css_class("sidebar");
        sidebar.set_width_request(250);
        sidebar.set_margin_top(10);
        sidebar.set_margin_bottom(10);
        sidebar.set_margin_start(10);
        sidebar.set_margin_end(10);

        // Header
        let header_box = Box::new(Orientation::Horizontal, 10);
        header_box.set_margin_start(5);
        let title = Label::new(Some("Devices"));
        title.add_css_class("title-4");
        title.set_halign(Align::Start);
        header_box.append(&title);

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header_box.append(&spacer);
        header_box.append(&help_browser::make_help_button("device_manager"));

        sidebar.append(&header_box);

        // Search
        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search devices…"));
        sidebar.append(&search_entry);

        // Devices list
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);

        let devices_list = ListBox::new();
        devices_list.add_css_class("boxed-list");
        scroll.set_child(Some(&devices_list));
        sidebar.append(&scroll);

        // New device button
        let new_btn = Button::with_label("➕ Add Device");
        new_btn.add_css_class("suggested-action");
        sidebar.append(&new_btn);

        widget.set_start_child(Some(&sidebar));

        // RIGHT PANEL - Device Details/Edit Form
        let main_content = Box::new(Orientation::Vertical, 10);
        main_content.set_margin_top(20);
        main_content.set_margin_bottom(20);
        main_content.set_margin_start(20);
        main_content.set_margin_end(20);

        // Action buttons bar
        let action_bar = Box::new(Orientation::Horizontal, 10);
        let save_btn = Button::with_label("💾 Save");
        save_btn.add_css_class("suggested-action");
        save_btn.set_sensitive(false);
        let cancel_btn = Button::with_label("❌ Cancel");
        cancel_btn.set_sensitive(false);
        let delete_btn = Button::with_label("🗑️ Delete");
        delete_btn.add_css_class("destructive-action");
        delete_btn.set_sensitive(false);
        let sync_btn = Button::with_label("🔄 Sync from Device");
        sync_btn.set_tooltip_text(Some("Update device information from connected device"));
        sync_btn.set_sensitive(false);
        let set_active_btn = Button::with_label("✓ Set Active");
        set_active_btn.set_sensitive(false);

        action_bar.append(&save_btn);
        action_bar.append(&cancel_btn);
        action_bar.append(&delete_btn);
        action_bar.append(&sync_btn);

        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        action_bar.append(&spacer);

        action_bar.append(&set_active_btn);

        main_content.append(&action_bar);

        // Stack with tabs
        let stack = Stack::new();
        stack.set_vexpand(true);

        // Create tab pages
        let (general_page, edit_name, edit_description, edit_device_type, edit_controller_type) =
            Self::create_general_tab();
        let (
            connection_page,
            edit_connection_type,
            edit_port,
            edit_baud_rate,
            edit_tcp_port,
            edit_timeout,
            edit_auto_reconnect,
        ) = Self::create_connection_tab();
        let (
            dimensions_page,
            edit_x_min,
            edit_x_max,
            edit_y_min,
            edit_y_max,
            edit_z_min,
            edit_z_max,
            edit_x_min_unit,
            edit_x_max_unit,
            edit_y_min_unit,
            edit_y_max_unit,
            edit_z_min_unit,
            edit_z_max_unit,
        ) = Self::create_dimensions_tab(*current_units.borrow());
        let (
            capabilities_page,
            edit_has_spindle,
            edit_has_laser,
            edit_has_coolant,
            edit_num_axes,
            edit_max_feed_rate,
            edit_max_feed_rate_unit,
            edit_max_s_value,
            edit_spindle_watts,
            edit_max_spindle_speed_rpm,
            edit_laser_watts,
        ) = Self::create_capabilities_tab(*current_feed_units.borrow());

        stack.add_titled(&general_page, Some("general"), "General");
        stack.add_titled(&connection_page, Some("connection"), "Connection");
        stack.add_titled(&dimensions_page, Some("dimensions"), "Dimensions");
        stack.add_titled(&capabilities_page, Some("capabilities"), "Capabilities");

        let switcher = StackSwitcher::new();
        switcher.set_stack(Some(&stack));
        switcher.set_halign(Align::Center);

        main_content.append(&switcher);
        main_content.append(&stack);

        // Right Panel Stack (Switcher between Edit Form and Placeholder)
        let edit_stack = Stack::new();
        edit_stack.set_transition_type(gtk4::StackTransitionType::Crossfade);

        // -- Placeholder View
        let placeholder_box = Box::new(Orientation::Vertical, 0);
        placeholder_box.set_valign(Align::Center);
        placeholder_box.set_halign(Align::Center);

        let placeholder_label = Label::new(Some("Please select or create a device entry to edit"));
        placeholder_label.add_css_class("dim-label");
        placeholder_label.add_css_class("title-3");

        placeholder_box.append(&placeholder_label);

        // -- Main Edit Form (Wraps existing main_content)
        edit_stack.add_named(&placeholder_box, Some("placeholder"));
        edit_stack.add_named(&main_content, Some("edit_form"));

        // Default to placeholder
        edit_stack.set_visible_child_name("placeholder");

        widget.set_end_child(Some(&edit_stack));

        // Set initial position (once)
        widget.connect_map(|paned| {
            let paned = paned.clone();
            gtk4::glib::idle_add_local_once(move || {
                let width = paned.width();
                if width > 0 {
                    paned.set_position((width as f64 * 0.2) as i32);
                }
            });
        });

        let view = Rc::new(Self {
            widget,
            controller: controller.clone(),
            current_units,
            current_feed_units,
            devices_list,
            search_entry,
            edit_name,
            edit_description,
            edit_device_type,
            edit_controller_type,
            edit_connection_type,
            edit_port,
            edit_baud_rate,
            edit_tcp_port,
            edit_timeout,
            edit_auto_reconnect,
            edit_x_min,
            edit_x_max,
            edit_y_min,
            edit_y_max,
            edit_z_min,
            edit_z_max,
            edit_x_min_unit,
            edit_x_max_unit,
            edit_y_min_unit,
            edit_y_max_unit,
            edit_z_min_unit,
            edit_z_max_unit,
            edit_has_spindle,
            edit_has_laser,
            edit_has_coolant,
            edit_num_axes,
            edit_max_feed_rate,
            edit_max_feed_rate_unit,
            edit_max_s_value,
            edit_spindle_watts,
            edit_max_spindle_speed_rpm,
            edit_laser_watts,
            selected_device: Rc::new(RefCell::new(None)),
            save_btn,
            cancel_btn,
            delete_btn,
            sync_btn,
            set_active_btn,
            new_btn,
            edit_stack,
        });

        {
            let view = view.clone();
            settings_controller.on_setting_changed(move |key, value| {
                if key == "measurement_system" || key == "units.measurement_system" {
                    let new_units = match value {
                        "Imperial" => MeasurementSystem::Imperial,
                        _ => MeasurementSystem::Metric,
                    };
                    {
                        *view.current_units.borrow_mut() = new_units;
                    }
                    view.refresh_units_display();
                }

                if key == "feed_rate_units" {
                    let new_units = match value {
                        "mm/sec" => FeedRateUnits::MmPerSec,
                        "in/min" => FeedRateUnits::InPerMin,
                        "in/sec" => FeedRateUnits::InPerSec,
                        _ => FeedRateUnits::MmPerMin,
                    };
                    {
                        *view.current_feed_units.borrow_mut() = new_units;
                    }
                    view.refresh_units_display();
                }
            });
        }

        view.setup_event_handlers();
        view.update_connection_field_sensitivity();
        view.load_devices();

        view
    }

    pub fn widget(&self) -> &Paned {
        &self.widget
    }

    fn refresh_units_display(&self) {
        let units = *self.current_units.borrow();
        let feed_units = *self.current_feed_units.borrow();

        let unit_label = get_unit_label(units);
        self.edit_x_min_unit.set_text(unit_label);
        self.edit_x_max_unit.set_text(unit_label);
        self.edit_y_min_unit.set_text(unit_label);
        self.edit_y_max_unit.set_text(unit_label);
        self.edit_z_min_unit.set_text(unit_label);
        self.edit_z_max_unit.set_text(unit_label);

        self.edit_max_feed_rate_unit
            .set_text(&feed_units.to_string());

        let model_opt = self.selected_device.borrow().clone();
        if let Some(profile) = model_opt {
            self.edit_x_min.set_text(&format_length(
                profile.x_min.parse::<f32>().unwrap_or(0.0),
                units,
            ));
            self.edit_x_max.set_text(&format_length(
                profile.x_max.parse::<f32>().unwrap_or(200.0),
                units,
            ));
            self.edit_y_min.set_text(&format_length(
                profile.y_min.parse::<f32>().unwrap_or(0.0),
                units,
            ));
            self.edit_y_max.set_text(&format_length(
                profile.y_max.parse::<f32>().unwrap_or(200.0),
                units,
            ));
            self.edit_z_min.set_text(&format_length(
                profile.z_min.parse::<f32>().unwrap_or(0.0),
                units,
            ));
            self.edit_z_max.set_text(&format_length(
                profile.z_max.parse::<f32>().unwrap_or(100.0),
                units,
            ));

            self.edit_max_feed_rate.set_text(&format_feed_rate(
                profile.max_feed_rate.parse::<f32>().unwrap_or(1000.0),
                feed_units,
            ));
        }
    }
}
