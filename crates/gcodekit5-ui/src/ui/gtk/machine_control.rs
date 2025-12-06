use gtk4::prelude::*;
use gtk4::{
    Box, Orientation, Label, Button, ComboBoxText, Frame, Grid, Align, Image, ToggleButton,
    TextView, ScrolledWindow, PolicyType,
};
use std::rc::Rc;
use std::cell::RefCell;

pub struct MachineControlView {
    pub widget: Box,
    // Connection
    pub port_combo: ComboBoxText,
    pub connect_btn: Button,
    pub refresh_btn: Button,
    // Transmission
    pub send_btn: Button,
    pub stop_btn: Button,
    pub pause_btn: Button,
    pub resume_btn: Button,
    // State
    pub state_label: Label,
    pub home_btn: Button,
    pub unlock_btn: Button,
    // Work Coordinates
    pub reset_g53_btn: Button,
    pub wcs_btns: Vec<Button>,
    // Status
    pub status_text: TextView,
    // DRO
    pub x_dro: Label,
    pub y_dro: Label,
    pub z_dro: Label,
    pub x_zero_btn: Button,
    pub y_zero_btn: Button,
    pub z_zero_btn: Button,
    pub zero_all_btn: Button,
    // World Coords
    pub world_x: Label,
    pub world_y: Label,
    pub world_z: Label,
    // Jog
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
}

impl MachineControlView {
    pub fn new() -> Rc<Self> {
        let widget = Box::new(Orientation::Horizontal, 0);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        // ═════════════════════════════════════════════
        // LEFT SIDEBAR
        // ═════════════════════════════════════════════
        let sidebar = Box::new(Orientation::Vertical, 10);
        sidebar.set_width_request(250);
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

        widget.append(&sidebar);

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
        
        let jog_y_pos = Button::with_label("▲");
        let jog_x_neg = Button::with_label("◀");
        let jog_x_pos = Button::with_label("▶");
        let jog_y_neg = Button::with_label("▼");
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
        let jog_z_pos = Button::with_label("▲");
        let z_label = Label::new(Some("Z"));
        let jog_z_neg = Button::with_label("▼");
        
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
        widget.append(&main_area);

        Rc::new(Self {
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
        })
    }

    pub fn get_step_size(&self) -> f64 {
        if self.step_0_1_btn.is_active() { 0.1 }
        else if self.step_1_0_btn.is_active() { 1.0 }
        else if self.step_10_btn.is_active() { 10.0 }
        else if self.step_50_btn.is_active() { 50.0 }
        else { 1.0 }
    }
}
