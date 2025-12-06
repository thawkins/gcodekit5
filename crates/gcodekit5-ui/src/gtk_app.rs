use crate::ui::gtk::machine_control::MachineControlView;
use crate::ui::gtk::device_console::DeviceConsoleView;
use crate::ui::gtk::editor::GcodeEditor;
use crate::ui::gtk::visualizer::GcodeVisualizer;
use crate::ui::gtk::cam_tools::CamToolsView;
use crate::ui::gtk::designer::DesignerCanvas;
use crate::ui::gtk::device_manager::DeviceManagerWindow;
use crate::ui::gtk::status_bar::StatusBar;
use crate::ui::gtk::settings::SettingsWindow;
use crate::ui::gtk::device_info::DeviceInfoView;
use crate::ui::gtk::config_settings::ConfigSettingsView;
use crate::ui::gtk::tools_manager::ToolsManagerView;
use crate::ui::gtk::materials_manager::MaterialsManagerView;
use gcodekit5_communication::Communicator;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, CssProvider, Orientation, PopoverMenuBar, Stack, StackSwitcher, StyleContext};
use gtk4::gio;
use std::rc::Rc;
use std::cell::RefCell;

pub fn main() {
    let app = Application::builder()
        .application_id("com.gcodekit5.app")
        .build();

    app.connect_startup(|_| load_css());

    app.connect_activate(|app| {
        // Initialize Controllers
        let settings_dialog = Rc::new(RefCell::new(gcodekit5_settings::SettingsDialog::new()));
        let settings_persistence = Rc::new(RefCell::new(gcodekit5_settings::SettingsPersistence::new()));
        let settings_controller = Rc::new(gcodekit5_settings::SettingsController::new(settings_dialog, settings_persistence));
        let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let device_config_path = config_dir.join("gcodekit5").join("devices.json");
        if let Some(parent) = device_config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let device_manager = std::sync::Arc::new(gcodekit5_devicedb::DeviceManager::new(device_config_path));
        let device_controller = Rc::new(gcodekit5_devicedb::DeviceUiController::new(device_manager));
        let designer_state = Rc::new(RefCell::new(gcodekit5_designer::DesignerState::new()));

        let window = ApplicationWindow::builder()
            .application(app)
            .title("GCodeKit5")
            .default_width(1200)
            .default_height(800)
            .build();

        // Use HeaderBar as titlebar
        let header_bar = gtk4::HeaderBar::new();
        window.set_titlebar(Some(&header_bar));

        let main_box = Box::new(Orientation::Vertical, 0);

        // Menu Bar
        let menu_bar_model = gio::Menu::new();
        
        let file_menu = gio::Menu::new();
        file_menu.append(Some("New"), Some("app.file_new"));
        file_menu.append(Some("Open"), Some("app.file_open"));
        file_menu.append(Some("Save"), Some("app.file_save"));
        file_menu.append(Some("Save As..."), Some("app.file_save_as"));
        file_menu.append(Some("Export"), Some("app.file_export"));
        file_menu.append(Some("Quit"), Some("app.quit"));
        menu_bar_model.append_submenu(Some("File"), &file_menu);

        let edit_menu = gio::Menu::new();
        edit_menu.append(Some("Undo"), Some("app.edit_undo"));
        edit_menu.append(Some("Redo"), Some("app.edit_redo"));
        edit_menu.append(Some("Cut"), Some("app.edit_cut"));
        edit_menu.append(Some("Copy"), Some("app.edit_copy"));
        edit_menu.append(Some("Paste"), Some("app.edit_paste"));
        edit_menu.append(Some("Preferences"), Some("app.preferences"));
        menu_bar_model.append_submenu(Some("Edit"), &edit_menu);

        let view_menu = gio::Menu::new();
        view_menu.append(Some("Toolbars"), Some("app.view_toolbars"));
        view_menu.append(Some("Status Bar"), Some("app.view_status_bar"));
        view_menu.append(Some("Visualizer"), Some("app.view_visualizer"));
        view_menu.append(Some("Console"), Some("app.view_console"));
        menu_bar_model.append_submenu(Some("View"), &view_menu);

        let tools_menu = gio::Menu::new();
        tools_menu.append(Some("Device Manager"), Some("app.devices"));
        tools_menu.append(Some("CAM Tools"), Some("app.cam_tools"));
        menu_bar_model.append_submenu(Some("Tools"), &tools_menu);

        let machine_menu = gio::Menu::new();
        machine_menu.append(Some("Connect"), Some("app.machine_connect"));
        machine_menu.append(Some("Disconnect"), Some("app.machine_disconnect"));
        machine_menu.append(Some("Home"), Some("app.machine_home"));
        machine_menu.append(Some("Reset"), Some("app.machine_reset"));
        menu_bar_model.append_submenu(Some("Machine"), &machine_menu);

        let help_menu = gio::Menu::new();
        help_menu.append(Some("Documentation"), Some("app.help_docs"));
        help_menu.append(Some("About"), Some("app.about"));
        menu_bar_model.append_submenu(Some("Help"), &help_menu);

        let menu_bar = PopoverMenuBar::from_model(Some(&menu_bar_model));
        main_box.append(&menu_bar);

        // Content Area
        let content_box = Box::new(Orientation::Vertical, 0);
        let stack_switcher = StackSwitcher::new();
        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);
        
        stack_switcher.set_stack(Some(&stack));
        
        content_box.append(&stack_switcher);
        content_box.append(&stack);
        
        // 1. Machine Control
        let machine_control = MachineControlView::new();
        stack.add_titled(&machine_control.widget, Some("machine"), "Machine Control");

        // Wire up Machine Control
        let mc = machine_control.clone();
        mc.connect_btn.connect_clicked(|_| println!("Connect clicked"));
        mc.refresh_btn.connect_clicked(|_| println!("Refresh ports clicked"));
        mc.send_btn.connect_clicked(|_| println!("Send clicked"));
        mc.stop_btn.connect_clicked(|_| println!("Stop clicked"));
        mc.pause_btn.connect_clicked(|_| println!("Pause clicked"));
        mc.resume_btn.connect_clicked(|_| println!("Resume clicked"));
        mc.home_btn.connect_clicked(|_| println!("Home clicked"));
        mc.unlock_btn.connect_clicked(|_| println!("Unlock clicked"));
        mc.reset_g53_btn.connect_clicked(|_| println!("Reset G53 clicked"));
        
        for (i, btn) in mc.wcs_btns.iter().enumerate() {
            let idx = i;
            btn.connect_clicked(move |_| println!("G{} clicked", 54 + idx));
        }

        mc.x_zero_btn.connect_clicked(|_| println!("Zero X clicked"));
        mc.y_zero_btn.connect_clicked(|_| println!("Zero Y clicked"));
        mc.z_zero_btn.connect_clicked(|_| println!("Zero Z clicked"));
        mc.zero_all_btn.connect_clicked(|_| println!("Zero All clicked"));

        let step_size_closure = move || {
            // This would need to be a method on MachineControlView or we access the buttons
            // For now just print
            println!("Jog clicked");
        };

        mc.jog_x_pos.connect_clicked(move |_| println!("Jog X+"));
        mc.jog_x_neg.connect_clicked(move |_| println!("Jog X-"));
        mc.jog_y_pos.connect_clicked(move |_| println!("Jog Y+"));
        mc.jog_y_neg.connect_clicked(move |_| println!("Jog Y-"));
        mc.jog_z_pos.connect_clicked(move |_| println!("Jog Z+"));
        mc.jog_z_neg.connect_clicked(move |_| println!("Jog Z-"));
        mc.estop_btn.connect_clicked(|_| println!("E-STOP clicked"));

        // 2. Device Console
        let device_console = DeviceConsoleView::new();
        stack.add_titled(&device_console.widget, Some("console"), "Device Console");

        // Wire up console send
        let communicator = machine_control.communicator.clone();
        let console_clone = device_console.clone();

        let send_cmd = move || {
            let text = console_clone.command_entry.text();
            if !text.is_empty() {
                let mut comm = communicator.borrow_mut();
                if comm.is_connected() {
                     if let Err(e) = comm.send_command(&text) {
                         console_clone.append_log(&format!("Error sending: {}\n", e));
                     } else {
                         console_clone.append_log(&format!("> {}\n", text));
                         console_clone.command_entry.set_text("");
                     }
                } else {
                     console_clone.append_log("Not connected\n");
                }
            }
        };

        let send_cmd_clone = send_cmd.clone();
        device_console.send_btn.connect_clicked(move |_| {
            send_cmd_clone();
        });

        let send_cmd_clone = send_cmd.clone();
        device_console.command_entry.connect_activate(move |_| {
            send_cmd_clone();
        });

        // Poll for incoming data
        let communicator = machine_control.communicator.clone();
        let console_clone = device_console.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let mut comm = communicator.borrow_mut();
            if comm.is_connected() {
                match comm.receive() {
                    Ok(data) if !data.is_empty() => {
                        let text = String::from_utf8_lossy(&data);
                        console_clone.append_log(&text);
                    }
                    Ok(_) => {} // No data
                    Err(_) => {} // Ignore errors for now
                }
            }
            gtk4::glib::ControlFlow::Continue
        });

        // 3. G-Code Editor
        let editor = Rc::new(GcodeEditor::new());
        stack.add_titled(&editor.widget, Some("editor"), "G-Code Editor");
        
        // 4. Visualizer
        let visualizer = Rc::new(GcodeVisualizer::new());
        stack.add_titled(&visualizer.widget, Some("visualizer"), "Visualizer");

        // Connect Editor to Visualizer
        let vis_clone = visualizer.clone();
        editor.connect_changed(move |buffer| {
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let text = buffer.text(&start, &end, true);
            vis_clone.set_gcode(&text);
        });
        
        // 5. CAM Tools
        let editor_clone = editor.clone();
        let stack_clone_for_cam = stack.clone();
        let cam_tools_view = CamToolsView::new(move |gcode| {
            editor_clone.set_text(&gcode);
            stack_clone_for_cam.set_visible_child_name("editor");
        });
        stack.add_titled(cam_tools_view.widget(), Some("cam_tools"), "CAM Tools");

        // 6. Designer
        let designer = DesignerCanvas::new(designer_state.clone());
        stack.add_titled(&designer.widget, Some("designer"), "Designer");
        
        // 7. Device Info
        let device_info = DeviceInfoView::new();
        stack.add_titled(&device_info.container, Some("device_info"), "Device Info");

        // 8. Device Config
        let config_settings = ConfigSettingsView::new();
        stack.add_titled(&config_settings.container, Some("config"), "Device Config");

        // 9. Device Manager
        let device_manager_view = DeviceManagerWindow::new(device_controller.clone());
        stack.add_titled(device_manager_view.widget(), Some("devices"), "Device Manager");

        // 10. CNC Tools
        let tools_manager = ToolsManagerView::new();
        stack.add_titled(&tools_manager.container, Some("tools"), "CNC Tools");

        // 11. Materials
        let materials_manager = MaterialsManagerView::new();
        stack.add_titled(&materials_manager.container, Some("materials"), "Materials");

        main_box.append(&content_box);

        // Status Bar
        let status_bar = Rc::new(StatusBar::new());
        main_box.append(&status_bar.widget);
        
        // Connect eStop
        status_bar.estop_btn.connect_clicked(|_| {
            println!("Emergency Stop Triggered!");
            // TODO: Implement actual eStop logic
        });

        window.set_child(Some(&main_box));

        // Actions
        let settings_action = gio::SimpleAction::new("preferences", None);
        let settings_controller_clone = settings_controller.clone();
        settings_action.connect_activate(move |_, _| {
            let win = SettingsWindow::new(settings_controller_clone.clone());
            win.present();
        });
        app.add_action(&settings_action);

        // Menu actions now just switch tabs
        let stack_clone = stack.clone();
        let devices_action = gio::SimpleAction::new("devices", None);
        devices_action.connect_activate(move |_, _| {
            stack_clone.set_visible_child_name("devices");
        });
        app.add_action(&devices_action);

        let stack_clone = stack.clone();
        let cam_action = gio::SimpleAction::new("cam_tools", None);
        cam_action.connect_activate(move |_, _| {
            stack_clone.set_visible_child_name("cam_tools");
        });
        app.add_action(&cam_action);

        let stack_clone = stack.clone();
        let console_action = gio::SimpleAction::new("view_console", None);
        console_action.connect_activate(move |_, _| {
            stack_clone.set_visible_child_name("console");
        });
        app.add_action(&console_action);

        // Placeholder Actions for Menu Items
        let action_names = vec![
            "file_new", "file_open", "file_save", "file_save_as", "file_export", "quit",
            "edit_undo", "edit_redo", "edit_cut", "edit_copy", "edit_paste",
            "view_toolbars", "view_status_bar", "view_visualizer",
            "machine_connect", "machine_disconnect", "machine_home", "machine_reset",
            "help_docs", "about"
        ];

        for name in action_names {
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(move |_, _| {
                println!("Action triggered: {}", name);
            });
            app.add_action(&action);
        }

        // Set Keyboard Shortcuts (Accelerators)
        app.set_accels_for_action("app.file_new", &["<Control>n"]);
        app.set_accels_for_action("app.file_open", &["<Control>o"]);
        app.set_accels_for_action("app.file_save", &["<Control>s"]);
        app.set_accels_for_action("app.file_save_as", &["<Control><Shift>s"]);
        app.set_accels_for_action("app.quit", &["<Control>q"]);
        
        app.set_accels_for_action("app.edit_undo", &["<Control>z"]);
        app.set_accels_for_action("app.edit_redo", &["<Control>y", "<Control><Shift>z"]);
        app.set_accels_for_action("app.edit_cut", &["<Control>x"]);
        app.set_accels_for_action("app.edit_copy", &["<Control>c"]);
        app.set_accels_for_action("app.edit_paste", &["<Control>v"]);
        
        app.set_accels_for_action("app.help_docs", &["F1"]);
        app.set_accels_for_action("app.machine_home", &["<Control>h"]);
        app.set_accels_for_action("app.machine_reset", &["F5"]);

        window.present();
    });

    app.run();
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("ui/gtk/style.css"));

    StyleContext::add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
