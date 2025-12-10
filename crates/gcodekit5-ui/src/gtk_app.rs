use crate::ui::gtk::cam_tools::CamToolsView;
use crate::ui::gtk::config_settings::ConfigSettingsView;
use crate::ui::gtk::designer::DesignerView;
use crate::ui::gtk::device_console::DeviceConsoleView;
use crate::ui::gtk::device_info::DeviceInfoView;
use crate::ui::gtk::device_manager::DeviceManagerWindow;
use crate::ui::gtk::editor::GcodeEditor;
use crate::ui::gtk::machine_control::MachineControlView;
use crate::ui::gtk::materials_manager::MaterialsManagerView;
use crate::ui::gtk::settings::SettingsWindow;
use crate::ui::gtk::status_bar::StatusBar;
use crate::ui::gtk::tools_manager::ToolsManagerView;
use crate::ui::gtk::visualizer::GcodeVisualizer;
use gcodekit5_communication::Communicator;
use crate::device_status;
use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, CssProvider, Orientation, PopoverMenuBar, Stack,
    StackSwitcher,
};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{info, debug};

pub fn main() {
    let app = Application::builder()
        .application_id("com.gcodekit5.app")
        .build();

    app.connect_startup(|_| {
        load_resources();
        load_css();
    });

    app.connect_activate(|app| {
        // Initialize Controllers
        let settings_dialog = Rc::new(RefCell::new(gcodekit5_settings::SettingsDialog::new()));
        let settings_persistence =
            Rc::new(RefCell::new(gcodekit5_settings::SettingsPersistence::new()));
        let settings_controller = Rc::new(gcodekit5_settings::SettingsController::new(
            settings_dialog.clone(),
            settings_persistence.clone(),
        ));

        // Populate settings from persistence so the dialog isn't empty
        settings_persistence
            .borrow_mut()
            .populate_dialog(&mut settings_dialog.borrow_mut());

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
        file_menu.append(Some("Import"), Some("app.file_import"));
        file_menu.append(Some("Export G-Code..."), Some("app.file_export_gcode"));
        file_menu.append(Some("Export SVG..."), Some("app.file_export_svg"));
        file_menu.append(Some("Run"), Some("app.file_run"));
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
        let visualizer = Rc::new(GcodeVisualizer::new(Some(device_manager.clone())));

        // 2. Machine Control
        let machine_control = MachineControlView::new(
            Some(status_bar.clone()),
            Some(device_console.clone()),
            Some(editor.clone()),
            Some(visualizer.clone()),
        );
        stack.add_titled(&machine_control.widget, Some("machine"), "Machine Control");

        // Machine Control event handlers are wired up internally in MachineControlView::new()

        // Add Device Console to stack
        stack.add_titled(&device_console.widget, Some("console"), "Device Console");

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
        stack.add_titled(&editor.widget, Some("editor"), "G-Code Editor");

        // 4. Visualizer (Already created)
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
            editor_clone.grab_focus();
        });
        stack.add_titled(cam_tools_view.widget(), Some("cam_tools"), "CAM Tools");

        // 6. Designer
        let designer = DesignerView::new(Some(device_manager.clone()));
        stack.add_titled(&designer.widget, Some("designer"), "Designer");

        // Connect Designer G-Code Generation to Editor
        let editor_clone_gen = editor.clone();
        let stack_clone_gen = stack.clone();
        designer.set_on_gcode_generated(move |gcode| {
            editor_clone_gen.set_text(&gcode);
            stack_clone_gen.set_visible_child_name("editor");
            editor_clone_gen.grab_focus();
        });

        // 7. Device Info
        let device_info = DeviceInfoView::new();
        stack.add_titled(&device_info.container, Some("device_info"), "Device Info");

        // 8. Device Config
        let config_settings = ConfigSettingsView::new();
        config_settings.set_communicator(machine_control.communicator.clone());
        config_settings.set_device_console(device_console.clone());
        stack.add_titled(&config_settings.container, Some("config"), "Device Config");

        // Connect device info and config to machine control connection state
        let device_info_clone = device_info.clone();
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
                let device_name = status.device_name.as_deref().unwrap_or_else(|| {
                    status.port_name.as_deref().unwrap_or("CNC Device")
                });
                
                device_info_clone.set_connected(
                    true,
                    device_name,
                    firmware_type,
                    firmware_version
                );
                device_info_clone.load_sample_capabilities();
                config_settings_clone.set_connected(true);
            } else {
                device_info_clone.set_connected(false, "", "", "");
                config_settings_clone.set_connected(false);
            }
            
            glib::ControlFlow::Continue
        });

        // 9. Device Manager
        let device_manager_view = DeviceManagerWindow::new(device_controller.clone());
        stack.add_titled(
            &device_manager_view.widget,
            Some("devices"),
            "Device Manager",
        );

        // 10. CNC Tools
        let tools_manager = ToolsManagerView::new();
        stack.add_titled(&tools_manager.widget, Some("tools"), "CNC Tools");

        // 11. Materials
        let materials_manager = MaterialsManagerView::new();
        stack.add_titled(&materials_manager.widget, Some("materials"), "Materials");

        main_box.append(&content_box);

        // Append the StatusBar (created earlier before MachineControlView)
        main_box.append(&status_bar.widget);

        // Connect eStop
        status_bar.estop_btn.connect_clicked(|_| {
            info!("Emergency Stop Triggered!");
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
                .program_name("GCodeKit5")
                .version(env!("CARGO_PKG_VERSION"))
                .comments("GCode Toolkit for CNC/Laser Machines")
                .website("https://github.com/thawkins/gcodekit5")
                .license_type(gtk4::License::MitX11)
                .authors(vec!["GCodeKit Contributors".to_string()])
                .logo_icon_name("application-x-executable")
                .build();

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
            action.connect_activate(move |_, _| {
                debug!("Action triggered: {}", name);
            });
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

        window.present();
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
