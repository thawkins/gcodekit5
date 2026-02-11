//! Tab creation methods for the device manager window.

use super::*;

impl DeviceManagerWindow {
    pub(crate) fn create_general_tab() -> (ScrolledWindow, Entry, Entry, ComboBoxText, ComboBoxText)
    {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let mut row = 0;

        // Name
        let name_label = Label::new(Some("Name:"));
        name_label.set_halign(Align::Start);
        let edit_name = Entry::new();
        edit_name.set_hexpand(true);
        grid.attach(&name_label, 0, row, 1, 1);
        grid.attach(&edit_name, 1, row, 1, 1);
        row += 1;

        // Description
        let desc_label = Label::new(Some("Description:"));
        desc_label.set_halign(Align::Start);
        let edit_description = Entry::new();
        edit_description.set_hexpand(true);
        grid.attach(&desc_label, 0, row, 1, 1);
        grid.attach(&edit_description, 1, row, 1, 1);
        row += 1;

        // Device Type
        let type_label = Label::new(Some("Device Type:"));
        type_label.set_halign(Align::Start);
        let edit_device_type = ComboBoxText::new();
        edit_device_type.append(Some("CNC Mill"), "CNC Mill");
        edit_device_type.append(Some("CNC Lathe"), "CNC Lathe");
        edit_device_type.append(Some("Laser Cutter"), "Laser Cutter");
        edit_device_type.append(Some("3D Printer"), "3D Printer");
        edit_device_type.append(Some("Plotter"), "Plotter");
        edit_device_type.set_active_id(Some("CNC Mill"));
        grid.attach(&type_label, 0, row, 1, 1);
        grid.attach(&edit_device_type, 1, row, 1, 1);
        row += 1;

        // Controller Type
        let ctrl_label = Label::new(Some("Controller:"));
        ctrl_label.set_halign(Align::Start);
        let edit_controller_type = ComboBoxText::new();
        edit_controller_type.append(Some("GRBL"), "GRBL");
        edit_controller_type.append(Some("TinyG"), "TinyG");
        edit_controller_type.append(Some("g2core"), "g2core");
        edit_controller_type.append(Some("Smoothieware"), "Smoothieware");
        edit_controller_type.append(Some("FluidNC"), "FluidNC");
        edit_controller_type.append(Some("Marlin"), "Marlin");
        edit_controller_type.set_active_id(Some("GRBL"));
        grid.attach(&ctrl_label, 0, row, 1, 1);
        grid.attach(&edit_controller_type, 1, row, 1, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_name,
            edit_description,
            edit_device_type,
            edit_controller_type,
        )
    }

    pub(crate) fn create_connection_tab() -> (
        ScrolledWindow,
        ComboBoxText,
        Entry,
        ComboBoxText,
        Entry,
        Entry,
        CheckButton,
    ) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let mut row = 0;

        // Connection Type
        let conn_label = Label::new(Some("Connection Type:"));
        conn_label.set_halign(Align::Start);
        let edit_connection_type = ComboBoxText::new();
        edit_connection_type.append(Some("Serial"), "Serial");
        edit_connection_type.append(Some("TCP/IP"), "TCP/IP");
        edit_connection_type.append(Some("WebSocket"), "WebSocket");
        edit_connection_type.set_active_id(Some("Serial"));
        grid.attach(&conn_label, 0, row, 1, 1);
        grid.attach(&edit_connection_type, 1, row, 1, 1);
        row += 1;

        // Port
        let port_label = Label::new(Some("Port:"));
        port_label.set_halign(Align::Start);
        let edit_port = Entry::new();
        edit_port.set_placeholder_text(Some("/dev/ttyUSB0"));
        grid.attach(&port_label, 0, row, 1, 1);
        grid.attach(&edit_port, 1, row, 1, 1);
        row += 1;

        // Baud Rate
        let baud_label = Label::new(Some("Baud Rate:"));
        baud_label.set_halign(Align::Start);
        let edit_baud_rate = ComboBoxText::new();
        edit_baud_rate.append(Some("9600"), "9600");
        edit_baud_rate.append(Some("19200"), "19200");
        edit_baud_rate.append(Some("38400"), "38400");
        edit_baud_rate.append(Some("57600"), "57600");
        edit_baud_rate.append(Some("115200"), "115200");
        edit_baud_rate.append(Some("250000"), "250000");
        edit_baud_rate.set_active_id(Some("115200"));
        grid.attach(&baud_label, 0, row, 1, 1);
        grid.attach(&edit_baud_rate, 1, row, 1, 1);
        row += 1;

        // TCP Port
        let tcp_label = Label::new(Some("TCP Port:"));
        tcp_label.set_halign(Align::Start);
        let edit_tcp_port = Entry::new();
        edit_tcp_port.set_input_purpose(gtk4::InputPurpose::Number);
        edit_tcp_port.set_text("23");
        grid.attach(&tcp_label, 0, row, 1, 1);
        grid.attach(&edit_tcp_port, 1, row, 1, 1);
        row += 1;

        // Timeout
        let timeout_label = Label::new(Some("Timeout (ms):"));
        timeout_label.set_halign(Align::Start);
        let edit_timeout = Entry::new();
        edit_timeout.set_input_purpose(gtk4::InputPurpose::Number);
        edit_timeout.set_text("5000");
        grid.attach(&timeout_label, 0, row, 1, 1);
        grid.attach(&edit_timeout, 1, row, 1, 1);
        row += 1;

        // Auto Reconnect
        let edit_auto_reconnect = CheckButton::with_label("Auto Reconnect");
        grid.attach(&edit_auto_reconnect, 0, row, 2, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
            edit_connection_type,
            edit_port,
            edit_baud_rate,
            edit_tcp_port,
            edit_timeout,
            edit_auto_reconnect,
        )
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn create_dimensions_tab(
        units: MeasurementSystem,
    ) -> (
        ScrolledWindow,
        Entry,
        Entry,
        Entry,
        Entry,
        Entry,
        Entry,
        Label,
        Label,
        Label,
        Label,
        Label,
        Label,
    ) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let grid = Grid::new();
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let unit_label = get_unit_label(units);

        let mut row = 0;

        // X Axis
        let x_label = Label::new(Some("X Axis:"));
        x_label.set_halign(Align::Start);

        let x_min_label = Label::new(Some("Min:"));
        let edit_x_min = Entry::new();
        edit_x_min.set_input_purpose(gtk4::InputPurpose::Number);
        edit_x_min.set_width_chars(8);

        let edit_x_min_unit = Label::new(Some(unit_label));
        edit_x_min_unit.set_width_chars(4);
        edit_x_min_unit.set_halign(Align::End);
        edit_x_min_unit.set_xalign(1.0);

        let x_max_label = Label::new(Some("Max:"));
        let edit_x_max = Entry::new();
        edit_x_max.set_input_purpose(gtk4::InputPurpose::Number);
        edit_x_max.set_width_chars(8);

        let edit_x_max_unit = Label::new(Some(unit_label));
        edit_x_max_unit.set_width_chars(4);
        edit_x_max_unit.set_halign(Align::End);
        edit_x_max_unit.set_xalign(1.0);

        grid.attach(&x_label, 0, row, 1, 1);
        grid.attach(&x_min_label, 1, row, 1, 1);
        grid.attach(&edit_x_min, 2, row, 1, 1);
        grid.attach(&edit_x_min_unit, 3, row, 1, 1);
        grid.attach(&x_max_label, 4, row, 1, 1);
        grid.attach(&edit_x_max, 5, row, 1, 1);
        grid.attach(&edit_x_max_unit, 6, row, 1, 1);
        row += 1;

        // Y Axis
        let y_label = Label::new(Some("Y Axis:"));
        y_label.set_halign(Align::Start);

        let y_min_label = Label::new(Some("Min:"));
        let edit_y_min = Entry::new();
        edit_y_min.set_input_purpose(gtk4::InputPurpose::Number);
        edit_y_min.set_width_chars(8);

        let edit_y_min_unit = Label::new(Some(unit_label));
        edit_y_min_unit.set_width_chars(4);
        edit_y_min_unit.set_halign(Align::End);
        edit_y_min_unit.set_xalign(1.0);

        let y_max_label = Label::new(Some("Max:"));
        let edit_y_max = Entry::new();
        edit_y_max.set_input_purpose(gtk4::InputPurpose::Number);
        edit_y_max.set_width_chars(8);

        let edit_y_max_unit = Label::new(Some(unit_label));
        edit_y_max_unit.set_width_chars(4);
        edit_y_max_unit.set_halign(Align::End);
        edit_y_max_unit.set_xalign(1.0);

        grid.attach(&y_label, 0, row, 1, 1);
        grid.attach(&y_min_label, 1, row, 1, 1);
        grid.attach(&edit_y_min, 2, row, 1, 1);
        grid.attach(&edit_y_min_unit, 3, row, 1, 1);
        grid.attach(&y_max_label, 4, row, 1, 1);
        grid.attach(&edit_y_max, 5, row, 1, 1);
        grid.attach(&edit_y_max_unit, 6, row, 1, 1);
        row += 1;

        // Z Axis
        let z_label = Label::new(Some("Z Axis:"));
        z_label.set_halign(Align::Start);

        let z_min_label = Label::new(Some("Min:"));
        let edit_z_min = Entry::new();
        edit_z_min.set_input_purpose(gtk4::InputPurpose::Number);
        edit_z_min.set_width_chars(8);

        let edit_z_min_unit = Label::new(Some(unit_label));
        edit_z_min_unit.set_width_chars(4);
        edit_z_min_unit.set_halign(Align::End);
        edit_z_min_unit.set_xalign(1.0);

        let z_max_label = Label::new(Some("Max:"));
        let edit_z_max = Entry::new();
        edit_z_max.set_input_purpose(gtk4::InputPurpose::Number);
        edit_z_max.set_width_chars(8);

        let edit_z_max_unit = Label::new(Some(unit_label));
        edit_z_max_unit.set_width_chars(4);
        edit_z_max_unit.set_halign(Align::End);
        edit_z_max_unit.set_xalign(1.0);

        grid.attach(&z_label, 0, row, 1, 1);
        grid.attach(&z_min_label, 1, row, 1, 1);
        grid.attach(&edit_z_min, 2, row, 1, 1);
        grid.attach(&edit_z_min_unit, 3, row, 1, 1);
        grid.attach(&z_max_label, 4, row, 1, 1);
        grid.attach(&edit_z_max, 5, row, 1, 1);
        grid.attach(&edit_z_max_unit, 6, row, 1, 1);

        scroll.set_child(Some(&grid));
        (
            scroll,
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
        )
    }

    pub(crate) fn create_capabilities_tab(
        feed_units: FeedRateUnits,
    ) -> (
        ScrolledWindow,
        CheckButton,
        CheckButton,
        CheckButton,
        Entry,
        Entry,
        Label,
        Entry,
        Entry,
        Entry,
        Entry,
    ) {
        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);

        let vbox = Box::new(Orientation::Vertical, 15);
        vbox.set_margin_top(10);
        vbox.set_margin_bottom(10);
        vbox.set_margin_start(10);
        vbox.set_margin_end(10);

        vbox.set_margin_end(10);

        // Limits
        let limits_label = Label::new(Some("Limits"));
        limits_label.set_css_classes(&["title-4"]);
        limits_label.set_halign(Align::Start);
        vbox.append(&limits_label);

        let limits_grid = Grid::new();
        limits_grid.set_column_spacing(10);
        limits_grid.set_row_spacing(10);

        let feed_label = Label::new(Some("Max Feed Rate:"));
        feed_label.set_halign(Align::Start);
        let edit_max_feed_rate = Entry::new();
        edit_max_feed_rate.set_input_purpose(gtk4::InputPurpose::Number);
        edit_max_feed_rate.set_text("1000");

        let edit_max_feed_rate_unit = Label::new(Some(&feed_units.to_string()));
        edit_max_feed_rate_unit.set_width_chars(6);
        edit_max_feed_rate_unit.set_halign(Align::End);
        edit_max_feed_rate_unit.set_xalign(1.0);

        let s_label = Label::new(Some("Max S-Value:"));
        s_label.set_halign(Align::Start);
        let edit_max_s_value = Entry::new();
        edit_max_s_value.set_input_purpose(gtk4::InputPurpose::Number);
        edit_max_s_value.set_text("1000");

        limits_grid.attach(&feed_label, 0, 0, 1, 1);
        limits_grid.attach(&edit_max_feed_rate, 1, 0, 1, 1);
        limits_grid.attach(&edit_max_feed_rate_unit, 2, 0, 1, 1);
        limits_grid.attach(&s_label, 0, 1, 1, 1);
        limits_grid.attach(&edit_max_s_value, 1, 1, 1, 1);

        vbox.append(&limits_grid);
        vbox.append(&gtk4::Separator::new(Orientation::Horizontal));

        let caps_label = Label::new(Some("Capabilities"));
        caps_label.set_css_classes(&["title-4"]);
        caps_label.set_halign(Align::Start);
        vbox.append(&caps_label);

        // Number of Axes
        let axes_box = Box::new(Orientation::Horizontal, 10);
        let axes_label = Label::new(Some("No of Axes:"));
        axes_label.set_halign(Align::Start);
        axes_label.set_width_request(150);
        let edit_num_axes = Entry::new();
        edit_num_axes.set_input_purpose(gtk4::InputPurpose::Number);
        edit_num_axes.set_text("3");
        edit_num_axes.set_max_width_chars(4);
        axes_box.append(&axes_label);
        axes_box.append(&edit_num_axes);
        vbox.append(&axes_box);

        // Has Spindle
        let edit_has_spindle = CheckButton::with_label("Has Spindle");
        vbox.append(&edit_has_spindle);

        // Spindle Watts
        let spindle_box = Box::new(Orientation::Horizontal, 10);
        let spindle_label = Label::new(Some("Spindle Power (W):"));
        spindle_label.set_halign(Align::Start);
        spindle_label.set_width_request(150);
        let edit_spindle_watts = Entry::new();
        edit_spindle_watts.set_input_purpose(gtk4::InputPurpose::Number);
        edit_spindle_watts.set_text("0");
        spindle_box.append(&spindle_label);
        spindle_box.append(&edit_spindle_watts);
        vbox.append(&spindle_box);

        // Max Spindle Speed
        let spindle_speed_box = Box::new(Orientation::Horizontal, 10);
        let spindle_speed_label = Label::new(Some("Max Spindle Speed (RPM):"));
        spindle_speed_label.set_halign(Align::Start);
        spindle_speed_label.set_width_request(150);
        let edit_max_spindle_speed_rpm = Entry::new();
        edit_max_spindle_speed_rpm.set_input_purpose(gtk4::InputPurpose::Number);
        edit_max_spindle_speed_rpm.set_text("12000");
        spindle_speed_box.append(&spindle_speed_label);
        spindle_speed_box.append(&edit_max_spindle_speed_rpm);
        vbox.append(&spindle_speed_box);

        // Has Laser
        let edit_has_laser = CheckButton::with_label("Has Laser");
        vbox.append(&edit_has_laser);

        // Laser Watts
        let laser_box = Box::new(Orientation::Horizontal, 10);
        let laser_label = Label::new(Some("Laser Power (W):"));
        laser_label.set_halign(Align::Start);
        laser_label.set_width_request(150);
        let edit_laser_watts = Entry::new();
        edit_laser_watts.set_input_purpose(gtk4::InputPurpose::Number);
        edit_laser_watts.set_text("0");
        laser_box.append(&laser_label);
        laser_box.append(&edit_laser_watts);
        vbox.append(&laser_box);

        // Has Coolant
        let edit_has_coolant = CheckButton::with_label("Has Coolant");
        vbox.append(&edit_has_coolant);

        scroll.set_child(Some(&vbox));
        (
            scroll,
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
        )
    }
}
