use crate::ui::gtk::cam_tools::CamToolsView;
use crate::ui::gtk::config_settings::ConfigSettingsView;
use crate::ui::gtk::designer::DesignerView;
use crate::ui::gtk::device_console::DeviceConsoleView;
// DeviceInfoView is now embedded in the Device Config panel; standalone import removed.
use crate::device_status;
use crate::i18n;
use crate::t;
use crate::ui::gtk::device_manager::DeviceManagerWindow;
use crate::ui::gtk::editor::GcodeEditor;
use crate::ui::gtk::machine_control::MachineControlView;
use crate::ui::gtk::materials_manager::MaterialsManagerView;
use crate::ui::gtk::settings::SettingsWindow;
use crate::ui::gtk::status_bar::StatusBar;
use crate::ui::gtk::tools_manager::ToolsManagerView;
use crate::ui::gtk::visualizer::GcodeVisualizer;
use gcodekit5_communication::Communicator;
use gcodekit5_settings::config::{StartupTab, Theme};
use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{
    glib, Application, ApplicationWindow, Box, CssProvider, Orientation, PopoverMenuBar, Stack,
    StackSwitcher,
};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{debug, info};

pub fn main() {
    let app = Application::builder()
        .application_id("com.gcodekit5.app")
        .build();

    app.connect_startup(|_| {
        // Load settings early to get language preference
        let config_path = gcodekit5_settings::SettingsManager::config_file_path()
            .unwrap_or_else(|_| std::path::PathBuf::from("config.json"));

        let language = if config_path.exists() {
            gcodekit5_settings::SettingsPersistence::load_from_file(&config_path)
                .map(|p| p.config().ui.language.clone())
                .unwrap_or_else(|_| "system".to_string())
        } else {
            "system".to_string()
        };

        i18n::init(Some(language));
        libadwaita::init().expect("Failed to initialize LibAdwaita");
        load_resources();
        load_css();
    });

    app.connect_activate(|app| {
        // Initialize Controllers
        let settings_dialog = Rc::new(RefCell::new(gcodekit5_settings::SettingsDialog::new()));

        // Load settings from file if it exists, otherwise use defaults
        let config_path = gcodekit5_settings::SettingsManager::config_file_path()
            .unwrap_or_else(|_| std::path::PathBuf::from("config.json"));
        let settings_persistence = if config_path.exists() {
            match gcodekit5_settings::SettingsPersistence::load_from_file(&config_path) {
                Ok(persistence) => {
                    info!("Loaded settings from {:?}", config_path);
                    Rc::new(RefCell::new(persistence))
                }
                Err(e) => {
                    info!("Failed to load settings: {}, using defaults", e);
                    Rc::new(RefCell::new(gcodekit5_settings::SettingsPersistence::new()))
                }
            }
        } else {
            info!("Config file not found at {:?}, using defaults", config_path);
            Rc::new(RefCell::new(gcodekit5_settings::SettingsPersistence::new()))
        };

        let settings_controller = Rc::new(gcodekit5_settings::SettingsController::new(
            settings_dialog.clone(),
            settings_persistence.clone(),
        ));

        // Populate settings from persistence so the dialog isn't empty
        settings_persistence
            .borrow()
            .populate_dialog(&mut settings_dialog.borrow_mut());

        // Apply initial theme
        let current_theme = settings_persistence.borrow().config().ui.theme;
        apply_theme(current_theme);

        // Listen for theme changes
        settings_controller.on_setting_changed(move |key, value| {
            if key == "theme" {
                let theme = match value {
                    "Light" => Theme::Light,
                    "Dark" => Theme::Dark,
                    _ => Theme::System,
                };
                apply_theme(theme);
            }
        });

        let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let device_config_path = config_dir.join("gcodekit5").join("devices.json");
        if let Some(parent) = device_config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let device_manager =
            std::sync::Arc::new(gcodekit5_devicedb::DeviceManager::new(device_config_path));
        device_manager.load().ok();
        let device_controller = Rc::new(gcodekit5_devicedb::DeviceUiController::new(
            device_manager.clone(),
        ));
        // Designer state is now managed internally by DesignerView

        let window = ApplicationWindow::builder()
            .application(app)
            .title(t!("GCodeKit5"))
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
        file_menu.append(Some(&t!("New")), Some("app.file_new"));
        file_menu.append(Some(&t!("Open")), Some("app.file_open"));
        file_menu.append(Some(&t!("Save")), Some("app.file_save"));
        file_menu.append(Some(&t!("Save As...")), Some("app.file_save_as"));
        file_menu.append(Some(&t!("Import")), Some("app.file_import"));
        file_menu.append(Some(&t!("Export G-Code...")), Some("app.file_export_gcode"));
        file_menu.append(Some(&t!("Export SVG...")), Some("app.file_export_svg"));
        file_menu.append(Some(&t!("Run")), Some("app.file_run"));
        file_menu.append(Some(&t!("Quit")), Some("app.quit"));
        menu_bar_model.append_submenu(Some(&t!("File")), &file_menu);

        let edit_menu = gio::Menu::new();
        edit_menu.append(Some(&t!("Undo")), Some("app.edit_undo"));
        edit_menu.append(Some(&t!("Redo")), Some("app.edit_redo"));
        edit_menu.append(Some(&t!("Cut")), Some("app.edit_cut"));
        edit_menu.append(Some(&t!("Copy")), Some("app.edit_copy"));
        edit_menu.append(Some(&t!("Paste")), Some("app.edit_paste"));
        edit_menu.append(Some(&t!("Preferences")), Some("app.preferences"));
        menu_bar_model.append_submenu(Some(&t!("Edit")), &edit_menu);

        let machine_menu = gio::Menu::new();
        machine_menu.append(Some(&t!("Connect")), Some("app.machine_connect"));
        machine_menu.append(Some(&t!("Disconnect")), Some("app.machine_disconnect"));
        machine_menu.append(Some(&t!("Home")), Some("app.machine_home"));
        machine_menu.append(Some(&t!("Reset")), Some("app.machine_reset"));
        menu_bar_model.append_submenu(Some(&t!("Machine")), &machine_menu);

        let help_menu = gio::Menu::new();
        help_menu.append(Some(&t!("Documentation")), Some("app.help_docs"));
        help_menu.append(Some(&t!("About")), Some("app.about"));
        menu_bar_model.append_submenu(Some(&t!("Help")), &help_menu);

        let menu_bar = PopoverMenuBar::from_model(Some(&menu_bar_model));
        main_box.append(&menu_bar);

        // Content Area
        let content_box = Box::new(Orientation::Vertical, 0);
        content_box.set_vexpand(true);
        let stack_switcher = StackSwitcher::new();
        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);

        stack_switcher.set_stack(Some(&stack));

        content_box.append(&stack_switcher);
        content_box.append(&stack);

        // 1. Device Console
        let device_console = DeviceConsoleView::new();

        let status_bar = StatusBar::new();

        // 3. G-Code Editor (Moved up to be available for MachineControl)
        let editor = Rc::new(GcodeEditor::new(Some(status_bar.clone())));

        // 4. Visualizer (Created early for MachineControl dependency)
        let visualizer = Rc::new(GcodeVisualizer::new(
            Some(device_manager.clone()),
            settings_controller.clone(),
            Some(status_bar.clone()),
        ));

        // 2. Machine Control
        let machine_control = MachineControlView::new(
            Some(status_bar.clone()),
            Some(device_console.clone()),
            Some(editor.clone()),
            Some(visualizer.clone()),
            Some(settings_controller.clone()),
        );
        stack.add_titled(
            &machine_control.widget,
            Some("machine"),
            &t!("Machine Control"),
        );

        // Machine Control event handlers are wired up internally in MachineControlView::new()

        // Device Console is now embedded in the Machine Control right-hand panel

        // Wire up console send
        let communicator = machine_control.communicator.clone();
        let console_clone = device_console.clone();

        let send_cmd = move || {
            let text = console_clone.command_entry.text();
            if !text.is_empty() {
                let mut comm = communicator.lock().unwrap();
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

        // Polling is now handled centrally by MachineControlView to avoid race conditions on serial read

        // Add Editor to Stack
        stack.add_titled(&editor.widget, Some("editor"), &t!("G-Code Editor"));

        // 4. Visualizer (Already created)
        stack.add_titled(&visualizer.widget, Some("visualizer"), &t!("Visualizer"));

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
        let settings_controller_cam = settings_controller.clone();
        let cam_tools_view = CamToolsView::new(settings_controller_cam, move |gcode| {
            editor_clone.set_text(&gcode);
            stack_clone_for_cam.set_visible_child_name("editor");
            editor_clone.grab_focus();
        });
        stack.add_titled(cam_tools_view.widget(), Some("cam_tools"), &t!("CAM Tools"));

        // 6. Designer
        let designer = DesignerView::new(
            Some(device_manager.clone()),
            settings_controller.clone(),
            Some(status_bar.clone()),
        );
        stack.add_titled(&designer.widget, Some("designer"), &t!("Designer"));

        // Connect Designer G-Code Generation to Editor
        let editor_clone_gen = editor.clone();
        let stack_clone_gen = stack.clone();
        designer.set_on_gcode_generated(move |gcode| {
            editor_clone_gen.set_text(&gcode);
            stack_clone_gen.set_visible_child_name("editor");
            editor_clone_gen.grab_focus();
        });

        // 7. Device Config (single panel now includes device info on the left)
        let config_settings = ConfigSettingsView::new(settings_controller.clone());
        config_settings.set_communicator(machine_control.communicator.clone());
        config_settings.set_device_console(device_console.clone());
        config_settings.set_device_manager(device_manager.clone());
        stack.add_titled(
            &config_settings.container,
            Some("config"),
            &t!("Device Config"),
        );

        // Connect device info and config to machine control connection state
        let config_settings_clone = config_settings.clone();
        let communicator_for_device = machine_control.communicator.clone();

        // Update device info when connection changes
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            let comm = communicator_for_device.lock().unwrap();
            let connected = comm.is_connected();

            if connected {
                // Get firmware info from device status
                let status = device_status::get_status();
                let firmware_type = status.firmware_type.as_deref().unwrap_or("GRBL");
                let firmware_version = status.firmware_version.as_deref().unwrap_or("Unknown");
                let device_name = status
                    .device_name
                    .as_deref()
                    .unwrap_or_else(|| status.port_name.as_deref().unwrap_or("CNC Device"));

                config_settings_clone.set_connected(true);
                config_settings_clone.set_device_info(
                    true,
                    device_name,
                    firmware_type,
                    firmware_version,
                );
            } else {
                config_settings_clone.set_connected(false);
                config_settings_clone.set_device_info(false, "", "", "");
            }

            glib::ControlFlow::Continue
        });

        // 9. Device Manager
        let device_manager_view =
            DeviceManagerWindow::new(device_controller.clone(), settings_controller.clone());
        stack.add_titled(
            &device_manager_view.widget,
            Some("devices"),
            &t!("Device Manager"),
        );

        // 10. CNC Tools
        let tools_manager = ToolsManagerView::new(settings_controller.clone());
        stack.add_titled(&tools_manager.widget, Some("tools"), &t!("CNC Tools"));

        // 11. Materials
        let materials_manager = MaterialsManagerView::new();
        stack.add_titled(
            &materials_manager.widget,
            Some("materials"),
            &t!("Materials"),
        );

        main_box.append(&content_box);

        // Append the StatusBar (created earlier before MachineControlView)
        main_box.append(&status_bar.widget);

        // Connect eStop (Ctrl-X / 0x18), same behavior as MachineControlView's E-Stop.
        {
            let communicator = machine_control.communicator.clone();
            let is_streaming = machine_control.is_streaming.clone();
            let is_paused = machine_control.is_paused.clone();
            let waiting_for_ack = machine_control.waiting_for_ack.clone();
            let send_queue = machine_control.send_queue.clone();
            let sb = status_bar.clone();
            let estop_btn = status_bar.estop_btn.clone();
            let device_console = device_console.clone();

            estop_btn.connect_clicked(move |_| {
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x18]);
                }

                *is_streaming.lock().unwrap() = false;
                *is_paused.lock().unwrap() = false;
                *waiting_for_ack.lock().unwrap() = false;
                send_queue.lock().unwrap().clear();

                sb.set_progress(0.0, "", "");

                device_console.append_log(&format!("{}\n", t!("Emergency stop (Ctrl-X)")));
            });
        }

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
        let machine_control_clone_for_console = machine_control.clone();
        console_action.connect_activate(move |_, _| {
            stack_clone.set_visible_child_name("machine");
            // Focus the console input on request
            if let Some(console_view) = machine_control_clone_for_console.device_console.as_ref() {
                console_view.command_entry.grab_focus();
            }
        });
        app.add_action(&console_action);

        // File Run Action
        let run_action = gio::SimpleAction::new("file_run", None);
        let machine_control_clone = machine_control.clone();
        let stack_clone = stack.clone();
        run_action.connect_activate(move |_, _| {
            // Switch to machine control view
            stack_clone.set_visible_child_name("machine");
            // Trigger send button
            machine_control_clone.send_btn.emit_clicked();
        });
        app.add_action(&run_action);

        // File Actions
        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let new_action = gio::SimpleAction::new("file_new", None);
        new_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.new_file(),
                    "editor" => editor_clone.new_file(),
                    _ => {}
                }
            }
        });
        app.add_action(&new_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let open_action = gio::SimpleAction::new("file_open", None);
        open_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.open_file(),
                    "editor" => editor_clone.open_file(),
                    _ => {}
                }
            }
        });
        app.add_action(&open_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let save_action = gio::SimpleAction::new("file_save", None);
        save_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.save_file(),
                    "editor" => editor_clone.save_file(),
                    _ => {}
                }
            }
        });
        app.add_action(&save_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let save_as_action = gio::SimpleAction::new("file_save_as", None);
        save_as_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.save_as_file(),
                    "editor" => editor_clone.save_as_file(),
                    _ => {}
                }
            }
        });
        app.add_action(&save_as_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let import_action = gio::SimpleAction::new("file_import", None);
        import_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.import_file(),
                    _ => {}
                }
            }
        });
        app.add_action(&import_action);

        let export_gcode_action = gio::SimpleAction::new("file_export_gcode", None);
        let designer_clone_gcode = designer.clone();
        let stack_clone_gcode = stack.clone();
        export_gcode_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone_gcode.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone_gcode.export_gcode(),
                    _ => {}
                }
            }
        });
        app.add_action(&export_gcode_action);

        let export_svg_action = gio::SimpleAction::new("file_export_svg", None);
        let designer_clone_svg = designer.clone();
        let stack_clone_svg = stack.clone();
        export_svg_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone_svg.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone_svg.export_svg(),
                    _ => {}
                }
            }
        });
        app.add_action(&export_svg_action);

        // About Dialog Action
        let app_clone = app.clone();
        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate(move |_, _| {
            let about_dialog = gtk4::AboutDialog::builder()
                .program_name(t!("GCodeKit5"))
                .version(env!("CARGO_PKG_VERSION"))
                .comments(t!("GCode Toolkit for CNC/Laser Machines"))
                .website("https://github.com/thawkins/gcodekit5")
                .license_type(gtk4::License::MitX11)
                .authors(vec![t!("Tim Hawkins and GCodeKit Contributors")])
                .build();

            about_dialog.set_logo_icon_name(None);

            // Size: +30% and match background image aspect ratio (gcodekit5.png: 550x362).
            about_dialog.set_default_size(780, 514);
            about_dialog.set_resizable(false);

            fn right_align_labels(root: &gtk4::Widget) {
                if let Ok(label) = root.clone().downcast::<gtk4::Label>() {
                    label.set_xalign(1.0);
                    label.set_justify(gtk4::Justification::Right);
                }

                let mut child = root.first_child();
                while let Some(w) = child {
                    right_align_labels(&w);
                    child = w.next_sibling();
                }
            }

            right_align_labels(about_dialog.upcast_ref::<gtk4::Widget>());

            fn mark_about_title(root: &gtk4::Widget) {
                if let Ok(label) = root.clone().downcast::<gtk4::Label>() {
                    if label.text() == "GCodeKit5" {
                        label.add_css_class("gk-about-title");
                    }
                }

                let mut child = root.first_child();
                while let Some(w) = child {
                    mark_about_title(&w);
                    child = w.next_sibling();
                }
            }

            mark_about_title(about_dialog.upcast_ref::<gtk4::Widget>());

            about_dialog.add_css_class("gk-about-dialog");
            about_dialog.set_transient_for(app_clone.active_window().as_ref());
            about_dialog.present();
        });
        app.add_action(&about_action);

        // Edit Actions
        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let undo_action = gio::SimpleAction::new("edit_undo", None);
        undo_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.undo(),
                    "editor" => editor_clone.undo(),
                    _ => {}
                }
            }
        });
        app.add_action(&undo_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let redo_action = gio::SimpleAction::new("edit_redo", None);
        redo_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.redo(),
                    "editor" => editor_clone.redo(),
                    _ => {}
                }
            }
        });
        app.add_action(&redo_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let cut_action = gio::SimpleAction::new("edit_cut", None);
        cut_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.cut(),
                    "editor" => editor_clone.cut(),
                    _ => {}
                }
            }
        });
        app.add_action(&cut_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let copy_action = gio::SimpleAction::new("edit_copy", None);
        copy_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.copy(),
                    "editor" => editor_clone.copy(),
                    _ => {}
                }
            }
        });
        app.add_action(&copy_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let paste_action = gio::SimpleAction::new("edit_paste", None);
        paste_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.paste(),
                    "editor" => editor_clone.paste(),
                    _ => {}
                }
            }
        });
        app.add_action(&paste_action);

        // Placeholder Actions for remaining items
        let action_names = vec![
            "quit",
            "view_toolbars",
            "view_status_bar",
            "view_visualizer",
            "machine_connect",
            "machine_disconnect",
            "machine_home",
            "machine_reset",
            "help_docs",
        ];

        for name in action_names {
            let action = gio::SimpleAction::new(name, None);
            if name == "quit" {
                let app_for_quit = app.clone();
                action.connect_activate(move |_, _| {
                    // Gracefully quit the application
                    app_for_quit.quit();
                });
            } else {
                let name = name.to_string();
                action.connect_activate(move |_, _| {
                    debug!("Action triggered: {}", name);
                });
            }
            app.add_action(&action);
        }

        // Enable/Disable actions based on active tab
        let app_clone = app.clone();
        stack.connect_visible_child_name_notify(move |stack| {
            if let Some(name) = stack.visible_child_name() {
                let name_str = name.as_str();
                let is_designer = name_str == "designer";
                let is_editor = name_str == "editor";
                // let is_machine = name_str == "machine";

                let set_enabled = |action_name: &str, enabled: bool| {
                    if let Some(action) = app_clone.lookup_action(action_name) {
                        if let Some(simple_action) = action.downcast_ref::<gio::SimpleAction>() {
                            simple_action.set_enabled(enabled);
                        }
                    }
                };

                // Edit actions
                set_enabled("edit_undo", is_designer || is_editor);
                set_enabled("edit_redo", is_designer || is_editor);
                set_enabled("edit_cut", is_designer || is_editor);
                set_enabled("edit_copy", is_designer || is_editor);
                set_enabled("edit_paste", is_designer || is_editor);

                // File actions
                set_enabled("file_new", is_designer || is_editor);
                set_enabled("file_open", is_designer || is_editor);
                set_enabled("file_save", is_designer || is_editor);
                set_enabled("file_save_as", is_designer || is_editor);
                set_enabled("file_import", is_designer);
                set_enabled("file_export_gcode", is_designer);
                set_enabled("file_export_svg", is_designer);
            }
        });

        // Trigger initial update
        if let Some(_name) = stack.visible_child_name() {
            // We can't easily trigger the signal manually with the same closure logic without extracting it.
            // But the default state of actions is enabled.
            // We should probably set them initially.
            // For now, let's just let the first switch handle it, or duplicate the logic briefly if needed.
            // Actually, SimpleAction defaults to enabled=true.
            // If we start in "machine" tab (which is likely), we might want to disable them.
            // But "machine" is the first tab added?
            // "machine" is added first.
        }

        // Set Keyboard Shortcuts (Accelerators)
        app.set_accels_for_action("app.file_new", &["<Control>n"]);
        app.set_accels_for_action("app.file_open", &["<Control>o"]);
        app.set_accels_for_action("app.file_save", &["<Control>s"]);
        app.set_accels_for_action("app.file_save_as", &["<Control><Shift>s"]);
        app.set_accels_for_action("app.file_run", &["<Control>r", "F5"]); // F5 also used for reset often, but Control-R is standard
        app.set_accels_for_action("app.quit", &["<Control>q"]);

        app.set_accels_for_action("app.edit_undo", &["<Control>z"]);
        app.set_accels_for_action("app.edit_redo", &["<Control>y", "<Control><Shift>z"]);
        app.set_accels_for_action("app.edit_cut", &["<Control>x"]);
        app.set_accels_for_action("app.edit_copy", &["<Control>c"]);
        app.set_accels_for_action("app.edit_paste", &["<Control>v"]);

        app.set_accels_for_action("app.help_docs", &["F1"]);
        app.set_accels_for_action("app.machine_home", &["<Control>h"]);
        // app.set_accels_for_action("app.machine_reset", &["F5"]); // Disabled as F5 is mapped to Run now if desired, or keep F5 for refresh/reset? keeping both might be conflict using alternate for Run

        // Set initial tab based on settings
        let startup_tab = settings_persistence.borrow().config().ui.startup_tab;
        let tab_name = match startup_tab {
            StartupTab::Machine => "machine",
            StartupTab::Console => "machine",
            StartupTab::Editor => "editor",
            StartupTab::Visualizer => "visualizer",
            StartupTab::CamTools => "cam_tools",
            StartupTab::Designer => "designer",
            StartupTab::DeviceInfo => "config",
            StartupTab::Config => "config",
            StartupTab::Devices => "devices",
            StartupTab::Tools => "tools",
            StartupTab::Materials => "materials",
        };
        stack.set_visible_child_name(tab_name);

        window.maximize();
        window.present();

        if settings_persistence
            .borrow()
            .config()
            .ui
            .show_about_on_startup
        {
            let window_weak = window.downgrade();
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                let Some(window) = window_weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };

                let about_dialog = gtk4::AboutDialog::builder()
                    .program_name(t!("GCodeKit5"))
                    .version(env!("CARGO_PKG_VERSION"))
                    .comments(t!("GCode Toolkit for CNC/Laser Machines"))
                    .website("https://github.com/thawkins/gcodekit5")
                    .license_type(gtk4::License::MitX11)
                    .authors(vec![t!("Tim Hawkins and GCodeKit Contributors")])
                    .build();

                about_dialog.set_logo_icon_name(None);

                // Size: +30% and match background image aspect ratio (gcodekit5.png: 550x362).
                about_dialog.set_default_size(780, 514);
                about_dialog.set_resizable(false);

                fn right_align_labels(root: &gtk4::Widget) {
                    if let Ok(label) = root.clone().downcast::<gtk4::Label>() {
                        label.set_xalign(1.0);
                        label.set_justify(gtk4::Justification::Right);
                    }

                    let mut child = root.first_child();
                    while let Some(w) = child {
                        right_align_labels(&w);
                        child = w.next_sibling();
                    }
                }

                right_align_labels(about_dialog.upcast_ref::<gtk4::Widget>());

                fn mark_about_title(root: &gtk4::Widget) {
                    if let Ok(label) = root.clone().downcast::<gtk4::Label>() {
                        if label.text() == "GCodeKit5" {
                            label.add_css_class("gk-about-title");
                        }
                    }

                    let mut child = root.first_child();
                    while let Some(w) = child {
                        mark_about_title(&w);
                        child = w.next_sibling();
                    }
                }

                mark_about_title(about_dialog.upcast_ref::<gtk4::Widget>());

                about_dialog.add_css_class("gk-about-dialog");
                about_dialog.set_transient_for(Some(&window));
                about_dialog.set_modal(true);
                about_dialog.present();

                let about_dialog_weak = about_dialog.downgrade();
                glib::timeout_add_seconds_local(15, move || {
                    if let Some(dlg) = about_dialog_weak.upgrade() {
                        dlg.close();
                    }
                    glib::ControlFlow::Break
                });

                glib::ControlFlow::Break
            });
        }
    });

    app.run();
}

fn load_resources() {
    let resources = include_bytes!(concat!(env!("OUT_DIR"), "/gcodekit5.gresource"));
    let resource_data = glib::Bytes::from_static(resources);
    let resource = gio::Resource::from_data(&resource_data).expect("Failed to load resources");
    gio::resources_register(&resource);
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("ui/gtk/style.css"));

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn apply_theme(theme: Theme) {
    let manager = libadwaita::StyleManager::default();
    match theme {
        Theme::System => manager.set_color_scheme(libadwaita::ColorScheme::Default),
        Theme::Light => manager.set_color_scheme(libadwaita::ColorScheme::ForceLight),
        Theme::Dark => manager.set_color_scheme(libadwaita::ColorScheme::ForceDark),
    }
}
