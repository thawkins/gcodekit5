use gcodekit5_communication::{
    Communicator, ConnectionDriver, ConnectionParams, SerialCommunicator, SerialParity,
};
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, ComboBoxText, Frame, Grid, Label, Orientation, Paned, PolicyType,
    ScrolledWindow, TextView, ToggleButton,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::gtk::status_bar::StatusBar;

pub struct MachineControlView {
    pub widget: Paned,
    pub port_combo: ComboBoxText,
    pub connect_btn: Button,
    pub refresh_btn: Button,
    pub send_btn: Button,
    pub stop_btn: Button,
    pub pause_btn: Button,
    pub resume_btn: Button,
    pub state_label: Label,
    pub home_btn: Button,
    pub unlock_btn: Button,
    pub reset_g53_btn: Button,
    pub wcs_btns: Vec<Button>,
    pub status_text: TextView,
    pub x_dro: Label,
    pub y_dro: Label,
    pub z_dro: Label,
    pub x_zero_btn: Button,
    pub y_zero_btn: Button,
    pub z_zero_btn: Button,
    pub zero_all_btn: Button,
    pub world_x: Label,
    pub world_y: Label,
    pub world_z: Label,
    pub step_0_1_btn: ToggleButton,
    pub step_1_0_btn: ToggleButton,
    pub step_10_btn: ToggleButton,
    pub step_50_btn: ToggleButton,
    pub jog_x_pos: Button,
    pub jog_x_neg: Button,
    pub jog_y_pos: Button,
    pub jog_y_neg: Button,
    pub jog_z_pos: Button,
    pub jog_z_neg: Button,
    pub estop_btn: Button,
    pub communicator: Rc<RefCell<SerialCommunicator>>,
    pub status_bar: Option<Rc<StatusBar>>,
}

impl MachineControlView {
    pub fn new(status_bar: Option<Rc<StatusBar>>) -> Rc<Self> {
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        // ═════════════════════════════════════════════
        // LEFT SIDEBAR
        // ═════════════════════════════════════════════
        let sidebar = Box::new(Orientation::Vertical, 10);
        sidebar.set_width_request(200); // Minimum width
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_start(10);
        sidebar.set_margin_end(10);
        sidebar.set_margin_top(10);
        sidebar.set_margin_bottom(10);

        // Connection Section
        let conn_frame = Frame::new(Some("Connection"));
        let conn_box = Box::new(Orientation::Vertical, 5);
        conn_box.set_margin_top(5);
        conn_box.set_margin_bottom(5);
        conn_box.set_margin_start(5);
        conn_box.set_margin_end(5);
        
        let port_combo = ComboBoxText::new();
        port_combo.append(Some("none"), "No ports available");
        port_combo.set_active_id(Some("none"));
        conn_box.append(&port_combo);

        let conn_btn_box = Box::new(Orientation::Horizontal, 5);
        let connect_btn = Button::with_label("Connect");
        connect_btn.add_css_class("suggested-action");
        connect_btn.set_hexpand(true);
        let refresh_btn = Button::from_icon_name("view-refresh-symbolic");
        conn_btn_box.append(&connect_btn);
        conn_btn_box.append(&refresh_btn);
        conn_box.append(&conn_btn_box);
        conn_frame.set_child(Some(&conn_box));
        sidebar.append(&conn_frame);

        // Transmission Section
        let trans_frame = Frame::new(Some("Transmission"));
        let trans_box = Box::new(Orientation::Vertical, 5);
        trans_box.set_margin_top(5);
        trans_box.set_margin_bottom(5);
        trans_box.set_margin_start(5);
        trans_box.set_margin_end(5);

        let trans_row1 = Box::new(Orientation::Horizontal, 5);
        let send_btn = Button::with_label("Send");
        send_btn.add_css_class("suggested-action");
        send_btn.set_hexpand(true);
        let stop_btn = Button::with_label("Stop");
        stop_btn.add_css_class("destructive-action");
        stop_btn.set_hexpand(true);
        trans_row1.append(&send_btn);
        trans_row1.append(&stop_btn);
        trans_box.append(&trans_row1);

        let trans_row2 = Box::new(Orientation::Horizontal, 5);
        let pause_btn = Button::with_label("Pause");
        pause_btn.set_hexpand(true);
        let resume_btn = Button::with_label("Resume");
        resume_btn.set_hexpand(true);
        trans_row2.append(&pause_btn);
        trans_row2.append(&resume_btn);
        trans_box.append(&trans_row2);
        trans_frame.set_child(Some(&trans_box));
        sidebar.append(&trans_frame);

        // Machine State Section
        let state_frame = Frame::new(Some("Machine State"));
        let state_box = Box::new(Orientation::Vertical, 5);
        state_box.set_margin_top(5);
        state_box.set_margin_bottom(5);
        state_box.set_margin_start(5);
        state_box.set_margin_end(5);

        let state_label = Label::new(Some("DISCONNECTED"));
        state_label.add_css_class("title-2");
        state_label.set_height_request(40);
        // state_label background handled by CSS classes in update
        state_box.append(&state_label);

        let state_btn_box = Box::new(Orientation::Horizontal, 5);
        let home_btn = Button::with_label("Home");
        home_btn.set_hexpand(true);
        let unlock_btn = Button::with_label("Unlock");
        unlock_btn.set_hexpand(true);
        state_btn_box.append(&home_btn);
        state_btn_box.append(&unlock_btn);
        state_box.append(&state_btn_box);
        state_frame.set_child(Some(&state_box));
        sidebar.append(&state_frame);

        // Work Coordinates Section
        let wcs_frame = Frame::new(Some("Work Coordinates"));
        let wcs_box = Box::new(Orientation::Vertical, 5);
        wcs_box.set_margin_top(5);
        wcs_box.set_margin_bottom(5);
        wcs_box.set_margin_start(5);
        wcs_box.set_margin_end(5);

        let reset_g53_btn = Button::with_label("Reset (G53)");
        wcs_box.append(&reset_g53_btn);

        let wcs_grid = Grid::new();
        wcs_grid.set_column_spacing(5);
        wcs_grid.set_row_spacing(5);
        wcs_grid.set_halign(Align::Center);

        let mut wcs_btns = Vec::new();
        for i in 0..6 {
            let label = format!("G{}", 54 + i);
            let btn = Button::with_label(&label);
            btn.set_hexpand(true);
            wcs_btns.push(btn.clone());
            wcs_grid.attach(&btn, (i % 3) as i32, (i / 3) as i32, 1, 1);
        }
        wcs_box.append(&wcs_grid);
        wcs_frame.set_child(Some(&wcs_box));
        sidebar.append(&wcs_frame);

        // Status Message Section
        let status_frame = Frame::new(Some("Status Message:"));
        let status_scroll = ScrolledWindow::new();
        status_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
        status_scroll.set_height_request(100);
        
        let status_text = TextView::new();
        status_text.set_editable(false);
        status_text.set_wrap_mode(gtk4::WrapMode::Word);
        status_text.add_css_class("monospace");
        status_scroll.set_child(Some(&status_text));
        status_frame.set_child(Some(&status_scroll));
        sidebar.append(&status_frame);

        // widget.append(&sidebar); // Moved to Paned setup

        // ═════════════════════════════════════════════
        // MAIN AREA
        // ═════════════════════════════════════════════
        let main_area = Box::new(Orientation::Vertical, 20);
        main_area.set_hexpand(true);
        main_area.set_vexpand(true);
        main_area.set_margin_top(20);
        main_area.set_margin_bottom(20);
        main_area.set_margin_start(20);
        main_area.set_margin_end(20);
        main_area.set_valign(Align::Center);

        // DRO Section
        let dro_box = Box::new(Orientation::Vertical, 10);
        dro_box.set_halign(Align::Center);
        dro_box.set_width_request(600);

        let create_dro = |label: &str| -> (Box, Label, Button) {
            let b = Box::new(Orientation::Horizontal, 10);
            b.add_css_class("dro-axis");
            b.set_height_request(60);
            
            let l = Label::new(Some(label));
            l.add_css_class("dro-label");
            l.set_width_request(50);
            
            let v = Label::new(Some("0.000"));
            v.add_css_class("dro-value");
            v.set_hexpand(true);
            v.set_halign(Align::End);
            
            let z = Button::with_label("⊙");
            z.add_css_class("circular");
            z.set_valign(Align::Center);
            
            b.append(&l);
            b.append(&v);
            b.append(&z);
            (b, v, z)
        };

        let (x_box, x_dro, x_zero_btn) = create_dro("X");
        let (y_box, y_dro, y_zero_btn) = create_dro("Y");
        let (z_box, z_dro, z_zero_btn) = create_dro("Z");

        dro_box.append(&x_box);
        dro_box.append(&y_box);
        dro_box.append(&z_box);

        let zero_all_btn = Button::with_label("Zero All Axes");
        zero_all_btn.set_margin_top(10);
        dro_box.append(&zero_all_btn);

        // World Coordinates
        let world_box = Box::new(Orientation::Vertical, 5);
        world_box.set_margin_top(20);
        let world_title = Label::new(Some("World Coordinates (G53)"));
        world_title.add_css_class("dim-label");
        world_box.append(&world_title);
        
        let world_vals = Box::new(Orientation::Horizontal, 20);
        world_vals.set_halign(Align::Center);
        let world_x = Label::new(Some("X: 0.000"));
        let world_y = Label::new(Some("Y: 0.000"));
        let world_z = Label::new(Some("Z: 0.000"));
        world_vals.append(&world_x);
        world_vals.append(&world_y);
        world_vals.append(&world_z);
        world_box.append(&world_vals);
        dro_box.append(&world_box);

        main_area.append(&dro_box);

        // Jog Controls
        let jog_area = Box::new(Orientation::Vertical, 20);
        jog_area.set_halign(Align::Center);
        jog_area.set_margin_top(40);

        // Step Size
        let step_box = Box::new(Orientation::Horizontal, 5);
        step_box.set_halign(Align::Center);
        step_box.append(&Label::new(Some("Step (mm): ")));
        
        let step_0_1_btn = ToggleButton::with_label("0.1");
        let step_1_0_btn = ToggleButton::with_label("1.0");
        let step_10_btn = ToggleButton::with_label("10");
        let step_50_btn = ToggleButton::with_label("50");
        
        // Group them
        step_1_0_btn.set_group(Some(&step_0_1_btn));
        step_10_btn.set_group(Some(&step_0_1_btn));
        step_50_btn.set_group(Some(&step_0_1_btn));
        step_1_0_btn.set_active(true); // Default 1.0

        step_box.append(&step_0_1_btn);
        step_box.append(&step_1_0_btn);
        step_box.append(&step_10_btn);
        step_box.append(&step_50_btn);
        jog_area.append(&step_box);

        // Directional Pads
        let pads_box = Box::new(Orientation::Horizontal, 60);
        pads_box.set_halign(Align::Center);

        // XY Pad
        let xy_grid = Grid::new();
        xy_grid.set_column_spacing(5);
        xy_grid.set_row_spacing(5);
        
        let jog_y_pos = Button::from_icon_name("go-up-symbolic");
        let jog_x_neg = Button::from_icon_name("go-previous-symbolic");
        let jog_x_pos = Button::from_icon_name("go-next-symbolic");
        let jog_y_neg = Button::from_icon_name("go-down-symbolic");
        let home_center = Label::new(Some("XY"));
        
        // Style buttons
        for btn in [&jog_y_pos, &jog_x_neg, &jog_x_pos, &jog_y_neg] {
            btn.set_width_request(60);
            btn.set_height_request(60);
        }

        xy_grid.attach(&jog_y_pos, 1, 0, 1, 1);
        xy_grid.attach(&jog_x_neg, 0, 1, 1, 1);
        xy_grid.attach(&home_center, 1, 1, 1, 1);
        xy_grid.attach(&jog_x_pos, 2, 1, 1, 1);
        xy_grid.attach(&jog_y_neg, 1, 2, 1, 1);
        pads_box.append(&xy_grid);

        // Z Pad & eStop
        let z_estop_box = Box::new(Orientation::Horizontal, 20);
        
        let z_box = Box::new(Orientation::Vertical, 5);
        let jog_z_pos = Button::from_icon_name("go-up-symbolic");
        let z_label = Label::new(Some("Z"));
        let jog_z_neg = Button::from_icon_name("go-down-symbolic");
        
        for btn in [&jog_z_pos, &jog_z_neg] {
            btn.set_width_request(60);
            btn.set_height_request(60);
        }
        
        z_box.append(&jog_z_pos);
        z_box.append(&z_label);
        z_box.append(&jog_z_neg);
        z_estop_box.append(&z_box);

        // eStop
        let estop_btn = Button::with_label("EMERGENCY\nSTOP");
        estop_btn.add_css_class("estop-big");
        estop_btn.set_width_request(150);
        estop_btn.set_height_request(150);
        z_estop_box.append(&estop_btn);

        pads_box.append(&z_estop_box);
        jog_area.append(&pads_box);

        main_area.append(&jog_area);
        
        // Setup Paned
        widget.set_start_child(Some(&sidebar));
        widget.set_end_child(Some(&main_area));
        
        // Dynamic resizing for 20% sidebar width
        widget.add_tick_callback(|paned, _clock| {
            let width = paned.width();
            let target = (width as f64 * 0.2) as i32;
            if (paned.position() - target).abs() > 2 {
                paned.set_position(target);
            }
            gtk4::glib::ControlFlow::Continue
        });

        let communicator = Rc::new(RefCell::new(SerialCommunicator::new()));

        let view = Rc::new(Self {
            widget,
            port_combo,
            connect_btn,
            refresh_btn,
            send_btn,
            stop_btn,
            pause_btn,
            resume_btn,
            state_label,
            home_btn,
            unlock_btn,
            reset_g53_btn,
            wcs_btns,
            status_text,
            x_dro,
            y_dro,
            z_dro,
            x_zero_btn,
            y_zero_btn,
            z_zero_btn,
            zero_all_btn,
            world_x,
            world_y,
            world_z,
            step_0_1_btn,
            step_1_0_btn,
            step_10_btn,
            step_50_btn,
            jog_x_pos,
            jog_x_neg,
            jog_y_pos,
            jog_y_neg,
            jog_z_pos,
            jog_z_neg,
            estop_btn,
            communicator,
            status_bar: status_bar.clone(),
        });

        view.refresh_ports();

        let view_clone = view.clone();
        view.refresh_btn.connect_clicked(move |_| {
            view_clone.refresh_ports();
        });

        let view_clone = view.clone();
        view.connect_btn.connect_clicked(move |_| {
            let is_connected = view_clone.communicator.borrow().is_connected();
            
            if is_connected {
                // Disconnect
                let mut comm = view_clone.communicator.borrow_mut();
                match comm.disconnect() {
                    Ok(_) => {
                        view_clone.connect_btn.set_label("Connect");
                        view_clone.connect_btn.remove_css_class("destructive-action");
                        view_clone.connect_btn.add_css_class("suggested-action");
                        view_clone.port_combo.set_sensitive(true);
                        view_clone.refresh_btn.set_sensitive(true);
                        view_clone.state_label.set_text("DISCONNECTED");
                        
                        // Update StatusBar
                        if let Some(ref status_bar) = view_clone.status_bar {
                            status_bar.set_connected(false, "");
                        }
                        
                        // Log to status
                        let buffer = view_clone.status_text.buffer();
                        let mut iter = buffer.end_iter();
                        buffer.insert(&mut iter, "Disconnected\n");
                    }
                    Err(e) => {
                        let buffer = view_clone.status_text.buffer();
                        let mut iter = buffer.end_iter();
                        buffer.insert(&mut iter, &format!("Error disconnecting: {}\n", e));
                    }
                }
            } else {
                // Connect - spawn in separate thread
                if let Some(port_name) = view_clone.port_combo.active_text() {
                    if port_name == "No ports available" {
                        return;
                    }
                    
                    let params = ConnectionParams {
                        driver: ConnectionDriver::Serial,
                        port: port_name.to_string(),
                        baud_rate: 115200,
                        ..Default::default()
                    };
                    
                    let communicator = view_clone.communicator.clone();
                    let connect_btn = view_clone.connect_btn.clone();
                    let port_combo = view_clone.port_combo.clone();
                    let refresh_btn = view_clone.refresh_btn.clone();
                    let state_label = view_clone.state_label.clone();
                    let status_text = view_clone.status_text.clone();
                    let status_bar = view_clone.status_bar.clone();
                    let port_name_copy = port_name.to_string();

                    // Spawn connection in a separate thread
                    std::thread::spawn(move || {
                        let result = communicator.borrow_mut().connect(&params);
                        
                        // Update UI on main thread
                        glib::idle_add_once(move || {
                            match result {
                                Ok(_) => {
                                    connect_btn.set_label("Disconnect");
                                    connect_btn.remove_css_class("suggested-action");
                                    connect_btn.add_css_class("destructive-action");
                                    port_combo.set_sensitive(false);
                                    refresh_btn.set_sensitive(false);
                                    state_label.set_text("CONNECTED");
                                    
                                    // Update StatusBar
                                    if let Some(ref sb) = status_bar {
                                        sb.set_connected(true, &port_name_copy);
                                    }
                                    
                                    // Log to status
                                    let buffer = status_text.buffer();
                                    let mut iter = buffer.end_iter();
                                    buffer.insert(&mut iter, &format!("Connected to {}\n", port_name_copy));
                                }
                                Err(e) => {
                                    let buffer = status_text.buffer();
                                    let mut iter = buffer.end_iter();
                                    buffer.insert(&mut iter, &format!("Error connecting: {}\n", e));
                                }
                            }
                        });
                    });
                }
            }
        });

        view
    }

    pub fn refresh_ports(&self) {
        self.port_combo.remove_all();
        
        match gcodekit5_communication::list_ports() {
            Ok(ports) if !ports.is_empty() => {
                for port in ports {
                    self.port_combo.append(Some(&port.port_name), &port.port_name);
                }
                // Select the first port
                self.port_combo.set_active(Some(0));
            }
            _ => {
                self.port_combo.append(Some("none"), "No ports available");
                self.port_combo.set_active_id(Some("none"));
            }
        }
    }

    pub fn get_step_size(&self) -> f64 {
        if self.step_0_1_btn.is_active() { 0.1 }
        else if self.step_1_0_btn.is_active() { 1.0 }
        else if self.step_10_btn.is_active() { 10.0 }
        else if self.step_50_btn.is_active() { 50.0 }
        else { 1.0 }
    }
}
