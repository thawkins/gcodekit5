use gcodekit5_communication::{
    Communicator, ConnectionDriver, ConnectionParams, SerialCommunicator,
};
use gcodekit5_communication::firmware::grbl::status_parser::StatusParser;
use gcodekit5_core::units::{format_length, MeasurementSystem};
use gcodekit5_settings::controller::SettingsController;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, ComboBoxText, Frame, Grid, Label, Orientation, Paned, Picture,
    ToggleButton,
};
use gtk4::glib;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

use crate::ui::gtk::status_bar::StatusBar;
use crate::ui::gtk::device_console::DeviceConsoleView;
use crate::ui::gtk::editor::GcodeEditor;
use crate::ui::gtk::visualizer::GcodeVisualizer;
use crate::device_status;
use std::rc::Rc;

#[derive(Clone)]
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
    pub communicator: Arc<Mutex<SerialCommunicator>>,
    pub status_bar: Option<StatusBar>,
    pub device_console: Option<Rc<DeviceConsoleView>>,
    pub editor: Option<Rc<GcodeEditor>>,
    pub visualizer: Option<Rc<GcodeVisualizer>>,
    pub send_queue: Arc<Mutex<VecDeque<String>>>,
    pub total_lines: Arc<Mutex<usize>>,
    pub is_streaming: Arc<Mutex<bool>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub waiting_for_ack: Arc<Mutex<bool>>,
    pub current_units: Arc<Mutex<MeasurementSystem>>,
}

impl MachineControlView {
    pub fn new(
        status_bar: Option<StatusBar>,
        device_console: Option<Rc<DeviceConsoleView>>,
        editor: Option<Rc<GcodeEditor>>,
        visualizer: Option<Rc<GcodeVisualizer>>,
        settings_controller: Option<Rc<SettingsController>>,
    ) -> Self {
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);
        
        // Helper function to disable connection-dependent buttons
        fn set_controls_enabled(
            send_btn: &Button,
            stop_btn: &Button,
            pause_btn: &Button,
            resume_btn: &Button,
            home_btn: &Button,
            unlock_btn: &Button,
            reset_g53_btn: &Button,
            wcs_btns: &[Button],
            x_zero_btn: &Button,
            y_zero_btn: &Button,
            z_zero_btn: &Button,
            zero_all_btn: &Button,
            step_0_1_btn: &ToggleButton,
            step_1_0_btn: &ToggleButton,
            step_10_btn: &ToggleButton,
            step_50_btn: &ToggleButton,
            jog_x_pos: &Button,
            jog_x_neg: &Button,
            jog_y_pos: &Button,
            jog_y_neg: &Button,
            jog_z_pos: &Button,
            jog_z_neg: &Button,
            estop_btn: &Button,
            enabled: bool,
        ) {
            send_btn.set_sensitive(enabled);
            stop_btn.set_sensitive(enabled);
            pause_btn.set_sensitive(enabled);
            resume_btn.set_sensitive(enabled);
            home_btn.set_sensitive(enabled);
            unlock_btn.set_sensitive(enabled);
            reset_g53_btn.set_sensitive(enabled);
            for btn in wcs_btns {
                btn.set_sensitive(enabled);
            }
            x_zero_btn.set_sensitive(enabled);
            y_zero_btn.set_sensitive(enabled);
            z_zero_btn.set_sensitive(enabled);
            zero_all_btn.set_sensitive(enabled);
            step_0_1_btn.set_sensitive(enabled);
            step_1_0_btn.set_sensitive(enabled);
            step_10_btn.set_sensitive(enabled);
            step_50_btn.set_sensitive(enabled);
            jog_x_pos.set_sensitive(enabled);
            jog_x_neg.set_sensitive(enabled);
            jog_y_pos.set_sensitive(enabled);
            jog_y_neg.set_sensitive(enabled);
            jog_z_pos.set_sensitive(enabled);
            jog_z_neg.set_sensitive(enabled);
            estop_btn.set_sensitive(enabled);
        }

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
        let estop_btn = Button::new();
        let estop_picture = Picture::for_filename("assets/Pictures/eStop2.png");
        estop_picture.set_can_shrink(true);
        estop_btn.set_child(Some(&estop_picture));
        
        estop_btn.add_css_class("estop-big");
        estop_btn.set_width_request(150);
        estop_btn.set_height_request(150);
        z_estop_box.append(&estop_btn);

        pads_box.append(&z_estop_box);
        jog_area.append(&pads_box);

        main_area.append(&jog_area);
        
        // Setup Paned
        // Use an inner paned so we have: [sidebar] | [main area] | [device console]
        let inner_paned = Paned::new(Orientation::Horizontal);
        inner_paned.set_start_child(Some(&main_area));

        let console_container = Box::new(Orientation::Vertical, 10);
        console_container.set_hexpand(true);
        console_container.set_vexpand(true);
        console_container.set_margin_top(10);
        console_container.set_margin_bottom(10);
        console_container.set_margin_start(10);
        console_container.set_margin_end(10);

        // Embed Device Console if present
        if let Some(ref console_view) = device_console {
            console_container.append(&console_view.widget);
        } else {
            let placeholder = Label::new(Some("Device Console not available"));
            placeholder.set_halign(Align::Center);
            console_container.append(&placeholder);
        }

        inner_paned.set_end_child(Some(&console_container));

        // Dynamic resizing for main area vs console (70% main / 30% console)
        inner_paned.add_tick_callback(|paned, _clock| {
            let width = paned.width();
            let target = (width as f64 * 0.7) as i32;
            if (paned.position() - target).abs() > 2 {
                paned.set_position(target);
            }
            gtk4::glib::ControlFlow::Continue
        });

        widget.set_start_child(Some(&sidebar));
        widget.set_end_child(Some(&inner_paned));
        
        // Dynamic resizing for 20% sidebar width
        widget.add_tick_callback(|paned, _clock| {
            let width = paned.width();
            let target = (width as f64 * 0.2) as i32;
            if (paned.position() - target).abs() > 2 {
                paned.set_position(target);
            }
            gtk4::glib::ControlFlow::Continue
        });

        let communicator = Arc::new(Mutex::new(SerialCommunicator::new()));

        // Initialize units from settings if available
        let initial_units = if let Some(controller) = &settings_controller {
            controller.persistence.borrow().config().ui.measurement_system
        } else {
            MeasurementSystem::Metric
        };
        let current_units = Arc::new(Mutex::new(initial_units));

        // Listen for unit changes
        if let Some(controller) = &settings_controller {
            let units_clone = current_units.clone();
            controller.on_setting_changed(move |key, value| {
                if key == "measurement_system" {
                    let new_units = match value {
                        "Imperial" => MeasurementSystem::Imperial,
                        _ => MeasurementSystem::Metric,
                    };
                    if let Ok(mut u) = units_clone.lock() {
                        *u = new_units;
                    }
                }
            });
        }

        let view = Self {
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
            device_console: device_console.clone(),
            editor,
            visualizer,
            send_queue: Arc::new(Mutex::new(VecDeque::new())),
            total_lines: Arc::new(Mutex::new(0)),
            is_streaming: Arc::new(Mutex::new(false)),
            is_paused: Arc::new(Mutex::new(false)),
            waiting_for_ack: Arc::new(Mutex::new(false)),
            current_units,
        };

        view.refresh_ports();
        
        // Disable all controls initially (until connected)
        set_controls_enabled(
            &view.send_btn,
            &view.stop_btn,
            &view.pause_btn,
            &view.resume_btn,
            &view.home_btn,
            &view.unlock_btn,
            &view.reset_g53_btn,
            &view.wcs_btns,
            &view.x_zero_btn,
            &view.y_zero_btn,
            &view.z_zero_btn,
            &view.zero_all_btn,
            &view.step_0_1_btn,
            &view.step_1_0_btn,
            &view.step_10_btn,
            &view.step_50_btn,
            &view.jog_x_pos,
            &view.jog_x_neg,
            &view.jog_y_pos,
            &view.jog_y_neg,
            &view.jog_z_pos,
            &view.jog_z_neg,
            &view.estop_btn,
            false,
        );

        // Setup jog button handlers
        {
            let communicator = view.communicator.clone();
            let step_0_1 = view.step_0_1_btn.clone();
            let step_1_0 = view.step_1_0_btn.clone();
            let step_10 = view.step_10_btn.clone();
            view.jog_x_pos.connect_clicked(move |_| {
                let step_size = if step_0_1.is_active() {
                    0.1
                } else if step_1_0.is_active() {
                    1.0
                } else if step_10.is_active() {
                    10.0
                } else {
                    50.0
                };
                let jog_cmd = format!("$J=G91 X{} F2000\n", step_size);
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(jog_cmd.as_bytes());
                }
            });
        }

        {
            let communicator = view.communicator.clone();
            let step_0_1 = view.step_0_1_btn.clone();
            let step_1_0 = view.step_1_0_btn.clone();
            let step_10 = view.step_10_btn.clone();
            view.jog_x_neg.connect_clicked(move |_| {
                let step_size = if step_0_1.is_active() {
                    0.1
                } else if step_1_0.is_active() {
                    1.0
                } else if step_10.is_active() {
                    10.0
                } else {
                    50.0
                };
                let jog_cmd = format!("$J=G91 X-{} F2000\n", step_size);
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(jog_cmd.as_bytes());
                }
            });
        }

        {
            let communicator = view.communicator.clone();
            let step_0_1 = view.step_0_1_btn.clone();
            let step_1_0 = view.step_1_0_btn.clone();
            let step_10 = view.step_10_btn.clone();
            view.jog_y_pos.connect_clicked(move |_| {
                let step_size = if step_0_1.is_active() {
                    0.1
                } else if step_1_0.is_active() {
                    1.0
                } else if step_10.is_active() {
                    10.0
                } else {
                    50.0
                };
                let jog_cmd = format!("$J=G91 Y{} F2000\n", step_size);
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(jog_cmd.as_bytes());
                }
            });
        }

        {
            let communicator = view.communicator.clone();
            let step_0_1 = view.step_0_1_btn.clone();
            let step_1_0 = view.step_1_0_btn.clone();
            let step_10 = view.step_10_btn.clone();
            view.jog_y_neg.connect_clicked(move |_| {
                let step_size = if step_0_1.is_active() {
                    0.1
                } else if step_1_0.is_active() {
                    1.0
                } else if step_10.is_active() {
                    10.0
                } else {
                    50.0
                };
                let jog_cmd = format!("$J=G91 Y-{} F2000\n", step_size);
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(jog_cmd.as_bytes());
                }
            });
        }

        {
            let communicator = view.communicator.clone();
            let step_0_1 = view.step_0_1_btn.clone();
            let step_1_0 = view.step_1_0_btn.clone();
            let step_10 = view.step_10_btn.clone();
            view.jog_z_pos.connect_clicked(move |_| {
                let step_size = if step_0_1.is_active() {
                    0.1
                } else if step_1_0.is_active() {
                    1.0
                } else if step_10.is_active() {
                    10.0
                } else {
                    50.0
                };
                let jog_cmd = format!("$J=G91 Z{} F2000\n", step_size);
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(jog_cmd.as_bytes());
                }
            });
        }

        {
            let communicator = view.communicator.clone();
            let step_0_1 = view.step_0_1_btn.clone();
            let step_1_0 = view.step_1_0_btn.clone();
            let step_10 = view.step_10_btn.clone();
            view.jog_z_neg.connect_clicked(move |_| {
                let step_size = if step_0_1.is_active() {
                    0.1
                } else if step_1_0.is_active() {
                    1.0
                } else if step_10.is_active() {
                    10.0
                } else {
                    50.0
                };
                let jog_cmd = format!("$J=G91 Z-{} F2000\n", step_size);
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(jog_cmd.as_bytes());
                }
            });
        }

        let view_clone = view.clone();
        view.refresh_btn.connect_clicked(move |_| {
            view_clone.refresh_ports();
        });

        // Transmission Controls
        {
            let communicator = view.communicator.clone();
            let is_paused = view.is_paused.clone();
            view.pause_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(b"!");
                }
                *is_paused.lock().unwrap() = true;
            });
        }
        {
            let communicator = view.communicator.clone();
            let is_paused = view.is_paused.clone();
            let is_streaming = view.is_streaming.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let send_queue = view.send_queue.clone();
            
            view.resume_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(b"~");
                }
                *is_paused.lock().unwrap() = false;
                
                // Kickstart if stalled (streaming, not waiting for ack, and has commands)
                if *is_streaming.lock().unwrap() && !*waiting_for_ack.lock().unwrap() {
                     let mut queue = send_queue.lock().unwrap();
                     if let Some(cmd) = queue.pop_front() {
                          if let Ok(mut comm) = communicator.lock() {
                               let _ = comm.send_command(&cmd);
                               *waiting_for_ack.lock().unwrap() = true;
                          }
                     }
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let is_streaming = view.is_streaming.clone();
            let is_paused = view.is_paused.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let send_queue = view.send_queue.clone();
            let status_bar = view.status_bar.clone();
            view.stop_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    // 0x18 = Ctrl-x = Soft Reset
                    let _ = comm.send(&[0x18]);
                }
                *is_streaming.lock().unwrap() = false;
                *is_paused.lock().unwrap() = false;
                *waiting_for_ack.lock().unwrap() = false;
                send_queue.lock().unwrap().clear();

                // Reset progress
                if let Some(sb) = status_bar.as_ref() {
                    sb.set_progress(0.0, "", "");
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let editor = view.editor.clone();
            let send_queue = view.send_queue.clone();
            let is_streaming = view.is_streaming.clone();
            let is_paused = view.is_paused.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let total_lines = view.total_lines.clone();
            let console = view.device_console.clone();
            
            view.send_btn.connect_clicked(move |_| {
                if *is_streaming.lock().unwrap() {
                    return;
                }
                
                let mut content = String::new();
                if let Some(ed) = editor.as_ref() {
                    content = ed.get_text();

                } else {

                }

                if content.trim().is_empty() {
                    let dialog = gtk4::MessageDialog::builder()
                        .message_type(gtk4::MessageType::Error)
                        .buttons(gtk4::ButtonsType::Ok)
                        .text("No G-Code to Send")
                        .secondary_text("Please load or type G-Code into the editor first.")
                        .build();
                    dialog.connect_response(|d, _| d.close());
                    dialog.show();
                    return;
                }
                
                let lines: Vec<String> = content.lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty() && !s.starts_with(';') && !s.starts_with('('))
                    .collect();
                    
                if lines.is_empty() {
                    if let Some(c) = console.as_ref() {
                         c.append_log("No valid G-Code lines found.\n");
                    }
                    return;
                }

                {
                    let mut queue = send_queue.lock().unwrap();
                    queue.clear();
                    for line in lines.iter() {
                        queue.push_back(line.clone());
                    }
                    *total_lines.lock().unwrap() = queue.len();

                }
                
                *is_streaming.lock().unwrap() = true;
                *is_paused.lock().unwrap() = false;
                *waiting_for_ack.lock().unwrap() = false;
                
                // Kickstart
                if let Ok(mut comm) = communicator.lock() {
                    let mut queue = send_queue.lock().unwrap();
                    if let Some(cmd) = queue.pop_front() {

                        let _ = comm.send_command(&cmd);
                        *waiting_for_ack.lock().unwrap() = true;
                    } else {

                    }
                } else {

                }
            });
        }

        // Machine State Controls
        {
            let communicator = view.communicator.clone();
            view.home_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("$H");
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            view.unlock_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("$X");
                }
            });
        }

        // WCS Controls
        {
            let communicator = view.communicator.clone();
            view.reset_g53_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("G53");
                }
            });
        }
        
        for (i, btn) in view.wcs_btns.iter().enumerate() {
            let communicator = view.communicator.clone();
            let cmd = format!("G{}", 54 + i);
            btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command(&cmd);
                }
            });
        }

        // Zero Controls
        {
            let communicator = view.communicator.clone();
            view.x_zero_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("G92 X0");
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            view.y_zero_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("G92 Y0");
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            view.z_zero_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("G92 Z0");
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            view.zero_all_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send_command("G92 X0 Y0 Z0");
                }
            });
        }

        // E-Stop
        {
            let communicator = view.communicator.clone();
            let is_streaming = view.is_streaming.clone();
            let is_paused = view.is_paused.clone();
            let waiting_for_ack = view.waiting_for_ack.clone();
            let send_queue = view.send_queue.clone();
            let status_bar = view.status_bar.clone();

            view.estop_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x18]);
                }
                
                *is_streaming.lock().unwrap() = false;
                *is_paused.lock().unwrap() = false;
                *waiting_for_ack.lock().unwrap() = false;
                send_queue.lock().unwrap().clear();

                // Reset progress
                if let Some(sb) = status_bar.as_ref() {
                    sb.set_progress(0.0, "", "");
                }
            });
        }


        let view_clone = view.clone();
        view.connect_btn.connect_clicked(move |_| {
            let is_connected = view_clone.communicator.lock().unwrap().is_connected();
            
            if is_connected {
                // Disconnect
                let mut comm = view_clone.communicator.lock().unwrap();
                match comm.disconnect() {
                    Ok(_) => {
                        view_clone.connect_btn.set_label("Connect");
                        view_clone.connect_btn.remove_css_class("destructive-action");
                        view_clone.connect_btn.add_css_class("suggested-action");
                        view_clone.port_combo.set_sensitive(true);
                        view_clone.refresh_btn.set_sensitive(true);
                        view_clone.state_label.set_text("DISCONNECTED");
                        
                        // Update global device status
                        device_status::update_connection_status(false, None);
                        
                        // Disable all controls on disconnect
                        set_controls_enabled(
                            &view_clone.send_btn,
                            &view_clone.stop_btn,
                            &view_clone.pause_btn,
                            &view_clone.resume_btn,
                            &view_clone.home_btn,
                            &view_clone.unlock_btn,
                            &view_clone.reset_g53_btn,
                            &view_clone.wcs_btns,
                            &view_clone.x_zero_btn,
                            &view_clone.y_zero_btn,
                            &view_clone.z_zero_btn,
                            &view_clone.zero_all_btn,
                            &view_clone.step_0_1_btn,
                            &view_clone.step_1_0_btn,
                            &view_clone.step_10_btn,
                            &view_clone.step_50_btn,
                            &view_clone.jog_x_pos,
                            &view_clone.jog_x_neg,
                            &view_clone.jog_y_pos,
                            &view_clone.jog_y_neg,
                            &view_clone.jog_z_pos,
                            &view_clone.jog_z_neg,
                            &view_clone.estop_btn,
                            false,
                        );
                        
                        // Update StatusBar
                        if let Some(ref status_bar) = view_clone.status_bar {
                            status_bar.set_connected(false, "");
                        }
                        
                        // Log to device console
                        if let Some(ref console) = view_clone.device_console {
                            console.append_log("Disconnected\n");
                        }
                    }
                    Err(e) => {
                        // Log error to device console
                        if let Some(ref console) = view_clone.device_console {
                            console.append_log(&format!("Error disconnecting: {}\n", e));
                        }
                    }
                }
            } else {
                // Connect - use blocking call in idle callback to avoid freezing UI
                if let Some(port_name) = view_clone.port_combo.active_text() {
                    if port_name == "No ports available" {
                        return;
                    }
                    
                    // Disable connect button while connecting
                    view_clone.connect_btn.set_sensitive(false);
                    view_clone.state_label.set_text("CONNECTING...");
                    
                    let params = ConnectionParams {
                        driver: ConnectionDriver::Serial,
                        port: port_name.to_string(),
                        baud_rate: 115200,
                        ..Default::default()
                    };
                    
                    // Perform synchronous connection (it's fast)
                    let result = view_clone.communicator.lock().unwrap().connect(&params);
                    
                    match result {
                        Ok(_) => {
                            view_clone.connect_btn.set_label("Disconnect");
                            view_clone.connect_btn.remove_css_class("suggested-action");
                            view_clone.connect_btn.add_css_class("destructive-action");
                            view_clone.connect_btn.set_sensitive(true);
                            view_clone.port_combo.set_sensitive(false);
                            view_clone.refresh_btn.set_sensitive(false);
                            view_clone.state_label.set_text("CONNECTED");
                            
                            // Update global device status
                            device_status::update_connection_status(true, Some(port_name.to_string()));
                            
                            // Update StatusBar
                            if let Some(ref sb) = view_clone.status_bar {
                                sb.set_connected(true, &port_name.to_string());
                            }
                            
                            // Log to device console
                            if let Some(ref console) = view_clone.device_console {
                                console.append_log(&format!("Connected to {}\n", port_name));
                            }
                            
                            // Enable all controls on successful connection
                            set_controls_enabled(
                                &view_clone.send_btn,
                                &view_clone.stop_btn,
                                &view_clone.pause_btn,
                                &view_clone.resume_btn,
                                &view_clone.home_btn,
                                &view_clone.unlock_btn,
                                &view_clone.reset_g53_btn,
                                &view_clone.wcs_btns,
                                &view_clone.x_zero_btn,
                                &view_clone.y_zero_btn,
                                &view_clone.z_zero_btn,
                                &view_clone.zero_all_btn,
                                &view_clone.step_0_1_btn,
                                &view_clone.step_1_0_btn,
                                &view_clone.step_10_btn,
                                &view_clone.step_50_btn,
                                &view_clone.jog_x_pos,
                                &view_clone.jog_x_neg,
                                &view_clone.jog_y_pos,
                                &view_clone.jog_y_neg,
                                &view_clone.jog_z_pos,
                                &view_clone.jog_z_neg,
                                &view_clone.estop_btn,
                                true,
                            );

                            // Unlock button should initially be disabled until ALARM state is detected
                            view_clone.unlock_btn.set_sensitive(false);
                            
                            // Query firmware version on connect
                            if let Ok(mut comm) = view_clone.communicator.lock() {
                                let _ = comm.send_command("$I");
                            }
                            
                            // Simple polling using glib::timeout_add_local - runs on main thread, no blocking
                            let state_label_poll = view_clone.state_label.clone();
                            let x_dro_poll = view_clone.x_dro.clone();
                            let y_dro_poll = view_clone.y_dro.clone();
                            let z_dro_poll = view_clone.z_dro.clone();
                            let unlock_btn_poll = view_clone.unlock_btn.clone();
                            let communicator_poll = view_clone.communicator.clone();
                            let status_bar_poll = view_clone.status_bar.clone();
                            let is_streaming_poll = view_clone.is_streaming.clone();
                            let is_paused_poll = view_clone.is_paused.clone();
                            let waiting_for_ack_poll = view_clone.waiting_for_ack.clone();
                            let send_queue_poll = view_clone.send_queue.clone();
                            let device_console_poll = view_clone.device_console.clone();
                            let visualizer_poll = view_clone.visualizer.clone();
                            let total_lines_poll = view_clone.total_lines.clone();
                            let current_units_poll = view_clone.current_units.clone();
                            
                            let mut query_counter = 0u32;
                            let mut response_buffer = String::new();
                            let mut firmware_detected = false;
                            
                            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                                query_counter += 1;
                                
                                // Check if still connected
                                let is_connected = {
                                    if let Ok(comm) = communicator_poll.try_lock() {
                                        comm.is_connected()
                                    } else {
                                        return glib::ControlFlow::Continue;
                                    }
                                };
                                
                                if !is_connected {
                                    return glib::ControlFlow::Break;
                                }
                                
                                // Try to read data (non-blocking, quick)
                                if let Ok(mut comm) = communicator_poll.try_lock() {
                                    if let Ok(response_bytes) = comm.receive() {
                                        if !response_bytes.is_empty() {
                                            let s = String::from_utf8_lossy(&response_bytes);

                                            response_buffer.push_str(&s);
                                            
                                            // Process complete lines
                                            while let Some(idx) = response_buffer.find('\n') {
                                                let line = response_buffer[..idx].trim().to_string();
                                                response_buffer.drain(..idx + 1);
                                                
                                                if line.is_empty() { continue; }

                                                // Detect firmware version info
                                                if !firmware_detected && (line.starts_with("[VER:") || line.contains("Grbl")) {
                                                    use gcodekit5_communication::firmware::firmware_detector::FirmwareDetector;
                                                    if let Ok(detection) = FirmwareDetector::parse_response(&line) {
                                                        let fw_type = format!("{:?}", detection.firmware_type);
                                                        let fw_version = detection.version_string.clone();
                                                        device_status::update_firmware_info(fw_type, fw_version, None);
                                                        firmware_detected = true;
                                                    }
                                                }
                                                
                                                // Log to console, filtering out status reports and 'ok' acks to avoid spam
                                                if !line.starts_with('<') && line != "ok" {
                                                    if let Some(c) = device_console_poll.as_ref() {
                                                        c.append_log(&format!("{}\n", line));
                                                    }
                                                }

                                                // Handle 'ok' or 'error' for streaming
                                                let is_ack = line == "ok";
                                                let is_error = line.starts_with("error:");
                                                
                                                if is_ack || is_error {
                                                     *waiting_for_ack_poll.lock().unwrap() = false;
                                                     
                                                     // If error, we might want to stop, but for now we continue
                                                     // if is_error { ... logic to stop ... }

                                                     if *is_streaming_poll.lock().unwrap() {
                                                         if !*is_paused_poll.lock().unwrap() {
                                                              let mut queue = send_queue_poll.lock().unwrap();
                                                              let total_lines_val = *total_lines_poll.lock().unwrap();
                                                              let remaining = queue.len();
                                                              let sent = total_lines_val - remaining;
                                                              
                                                              // Update progress bar
                                                              if let Some(sb) = status_bar_poll.as_ref() {
                                                                  let progress = if total_lines_val > 0 {
                                                                      (sent as f64 / total_lines_val as f64) * 100.0
                                                                  } else {
                                                                      0.0
                                                                  };
                                                                  
                                                                  // Simple time estimation (very rough)
                                                                  // Assuming average 0.1s per command for now
                                                                  let elapsed_secs = sent as f64 * 0.1;
                                                                  let remaining_secs = remaining as f64 * 0.1;
                                                                  
                                                                  let format_time = |secs: f64| {
                                                                      let h = (secs / 3600.0).floor();
                                                                      let m = ((secs % 3600.0) / 60.0).floor();
                                                                      let s = (secs % 60.0).floor();
                                                                      format!("{:02}:{:02}:{:02}", h, m, s)
                                                                  };
                                                                  
                                                                  sb.set_progress(
                                                                      progress,
                                                                      &format_time(elapsed_secs),
                                                                      &format_time(remaining_secs)
                                                                  );
                                                              }

                                                              if let Some(next_cmd) = queue.pop_front() {

                                                                   let _ = comm.send_command(&next_cmd);
                                                                    *waiting_for_ack_poll.lock().unwrap() = true;
                                                              } else {
                                                                   // Done
                                                                   *is_streaming_poll.lock().unwrap() = false;
                                                                   *is_paused_poll.lock().unwrap() = false;
                                                                   if let Some(c) = device_console_poll.as_ref() {
                                                                       c.append_log("Job Completed.\n");
                                                                   }
                                                                   // Reset progress
                                                                   if let Some(sb) = status_bar_poll.as_ref() {
                                                                       sb.set_progress(0.0, "", "");
                                                                   }
                                                              }
                                                         }
                                                     }
                                                }
                                                
                                                // Parse GRBL status: <Idle|MPos:0.000,0.000,0.000|...>
                                                if line.starts_with('<') && line.ends_with('>') {
                                                    // Update machine state
                                                    if let Some(state) = StatusParser::parse_machine_state(&line) {
                                                        state_label_poll.set_text(&state);
                                                        device_status::update_state(state.clone());
                                                        
                                                        // Update StatusBar with state
                                                        if let Some(sb) = status_bar_poll.as_ref() {
                                                            sb.set_state(&state);
                                                        }

                                                        // Unlock button only enabled in ALARM state
                                                        let is_alarm = state.to_lowercase().starts_with("alarm");
                                                        unlock_btn_poll.set_sensitive(is_alarm);
                                                    }
                                                    
                                                    // Parse and update machine position
                                                    if let Some(mpos) = StatusParser::parse_mpos(&line) {
                                                        let units = *current_units_poll.lock().unwrap();
                                                        let unit_label = gcodekit5_core::units::get_unit_label(units);
                                                        
                                                        x_dro_poll.set_text(&format!("{} {}", format_length(mpos.x as f32, units), unit_label));
                                                        y_dro_poll.set_text(&format!("{} {}", format_length(mpos.y as f32, units), unit_label));
                                                        z_dro_poll.set_text(&format!("{} {}", format_length(mpos.z as f32, units), unit_label));
                                                        device_status::update_machine_position(mpos);
                                                        
                                                        // Update StatusBar with position
                                                        if let Some(sb) = status_bar_poll.as_ref() {
                                                            sb.set_position(
                                                                mpos.x as f32, 
                                                                mpos.y as f32, 
                                                                mpos.z as f32,
                                                                mpos.a.unwrap_or(0.0) as f32,
                                                                mpos.b.unwrap_or(0.0) as f32,
                                                                mpos.c.unwrap_or(0.0) as f32,
                                                                units
                                                            );
                                                        }

                                                        // Update Visualizer with position
                                                        if let Some(vis) = visualizer_poll.as_ref() {
                                                            vis.set_current_position(mpos.x as f32, mpos.y as f32, mpos.z as f32);
                                                        }
                                                    }
                                                    
                                                    // Parse and update work position
                                                    if let Some(wpos) = StatusParser::parse_wpos(&line) {
                                                        device_status::update_work_position(wpos);
                                                    }
                                                    
                                                    // Parse and update work coordinate offset
                                                    if let Some(wco) = StatusParser::parse_wco(&line) {
                                                        device_status::update_work_coordinate_offset(wco);
                                                    }
                                                    
                                                    // Parse and update buffer state
                                                    if let Some(buffer) = StatusParser::parse_buffer(&line) {
                                                        device_status::update_buffer_state(buffer);
                                                    }
                                                    
                                                    // Parse and update feed/spindle state
                                                    if let Some(feed_spindle) = StatusParser::parse_feed_spindle(&line) {
                                                        device_status::update_feed_spindle_state(feed_spindle);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Send status query every ~250ms (every 5 cycles of 50ms)
                                    if query_counter % 5 == 0 {
                                        let _ = comm.send(b"?");
                                    }
                                }
                                
                                glib::ControlFlow::Continue
                            });
                        }
                        Err(e) => {
                            view_clone.connect_btn.set_sensitive(true);
                            view_clone.state_label.set_text("DISCONNECTED");
                            // Log error to device console
                            if let Some(ref console) = view_clone.device_console {
                                console.append_log(&format!("Error connecting: {}\n", e));
                            }
                        }
                    }
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
