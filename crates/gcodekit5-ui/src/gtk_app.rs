//! gcodekit5-ui/src/gtk_app.rs
//! # GTK Application Entry Point
//!
//! Initializes and runs the main GTK4 application window.
//! Sets up the application lifecycle, CSS theming, and
//! top-level window construction.

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
use crate::ui::gtk::help_browser;
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
    glib, Application, ApplicationWindow, Box as GtkBox, CssProvider, Orientation, PopoverMenuBar,
    Stack, StackSwitcher,
};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{debug, info};

pub fn main() {
    crate::glib_log_filter::install();

    let app = Application::builder()
        .application_id("com.gcodekit5.app")
        .build();

    app.connect_startup(|_| {
        // Load settings early to get language preference
        let language = gcodekit5_settings::SettingsManager::config_file_path()
            .ok()
            .and_then(|path| gcodekit5_settings::SettingsPersistence::load_from_file(&path).ok())
            .map(|p| p.config().ui.language.clone())
            .unwrap_or_else(|| "system".to_string());

        i18n::init(Some(language));
        if let Err(e) = libadwaita::init() {
            tracing::error!("Failed to initialize LibAdwaita: {}", e);
            std::process::exit(1);
        }

        // GTK 4.20+ on Wayland no longer handles dead keys or compose sequences
        // without an input method (IBus, Fcitx5), so typing '´' + 'a' gives only
        // 'a'. Select GTK's "simple" input method, which restores the classic
        // behaviour, unless the user configured one with GTK_IM_MODULE (e.g. ibus
        // for Japanese/Chinese), which GTK applies instead of this setting.
        if std::env::var_os("GTK_IM_MODULE").is_none() {
            if let Some(settings) = gtk4::Settings::default() {
                settings.set_gtk_im_module(Some("simple"));
            }
        }
        load_resources();
        load_css();
    });

    app.connect_activate(|app| {
        // Initialize Controllers
        let settings_dialog = Rc::new(RefCell::new(gcodekit5_settings::SettingsDialog::new()));

        // Load settings from file if it exists, otherwise use defaults
        // A config file that fails to load is kept: the defaults are used and
        // the file is backed up before the first save replaces it.
        let mut settings_load_error: Option<String> = None;
        let settings_persistence = match gcodekit5_settings::SettingsManager::config_file_path() {
            Ok(config_path) => {
                let (persistence, error) =
                    gcodekit5_settings::SettingsPersistence::load_or_default(&config_path);
                match &error {
                    Some(e) => tracing::error!(
                        path = %config_path.display(),
                        error = %e,
                        "Settings file could not be loaded; using defaults"
                    ),
                    None => info!("Loaded settings from {:?}", config_path),
                }
                settings_load_error = error.map(|e| format!("{}\n\n{e}", config_path.display()));
                Rc::new(RefCell::new(persistence))
            }
            Err(e) => {
                tracing::error!(error = %e, "No settings directory; using defaults");
                Rc::new(RefCell::new(gcodekit5_settings::SettingsPersistence::new()))
            }
        };

        let settings_controller = Rc::new(gcodekit5_settings::SettingsController::new(
            settings_dialog.clone(),
            settings_persistence.clone(),
        ));

        // Populate settings from persistence so the dialog isn't empty
        settings_persistence
            .borrow()
            .populate_dialog(&mut settings_dialog.borrow_mut());

        let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let device_config_path = config_dir.join("gcodekit5").join("devices.json");
        if let Some(parent) = device_config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let device_manager =
            std::sync::Arc::new(gcodekit5_devicedb::DeviceManager::new(device_config_path));
        device_manager.load().ok();
        // Sync active device num_axes to global state
        if let Some(profile) = device_manager.get_active_profile() {
            crate::device_status::set_active_num_axes(profile.num_axes);
        }

        // Designer state is now managed internally by DesignerView
        // Window size from Preferences (saved again when the window closes).
        let (window_width, window_height) = {
            let p = settings_persistence.borrow();
            let ui = &p.config().ui;
            (ui.window_width.max(640) as i32, ui.window_height.max(480) as i32)
        };
        let window = ApplicationWindow::builder()
            .application(app)
            .title(t!("GCodeKit5"))
            .default_width(window_width)
            .default_height(window_height)
            .build();

        // Use HeaderBar as titlebar
        let header_bar = gtk4::HeaderBar::new();
        window.set_titlebar(Some(&header_bar));

        let main_box = GtkBox::new(Orientation::Vertical, 0);

        // Menu Bar
        let menu_bar_model = gio::Menu::new();

        let file_menu = gio::Menu::new();
        file_menu.append(Some(&t!("New 2D")), Some("app.file_new_2d"));
        file_menu.append(Some(&t!("New 3D")), Some("app.file_new_3d"));
        file_menu.append(Some(&t!("Open")), Some("app.file_open"));
        file_menu.append(Some(&t!("Save")), Some("app.file_save"));
        file_menu.append(Some(&t!("Save As...")), Some("app.file_save_as"));
        file_menu.append(Some(&t!("Import")), Some("app.file_import"));
        file_menu.append(Some(&t!("Import Image...")), Some("app.file_import_image"));
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
        let content_box = GtkBox::new(Orientation::Vertical, 0);
        content_box.set_vexpand(true);
        let stack_switcher = StackSwitcher::new();
        stack_switcher.add_css_class("custom-stack-switcher"); // New style
        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);

        stack_switcher.set_stack(Some(&stack));

        content_box.append(&stack_switcher);
        content_box.append(&stack);

        // Device Console
        let device_console = DeviceConsoleView::new();
        let status_bar = StatusBar::new();
        device_manager.load().ok();

        // Refresh status bar active device
        let status_bar_clone = status_bar.clone();
        let device_manager_clone = device_manager.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(1000), move || {
            status_bar_clone.refresh_device_info(&device_manager_clone);

            if let Some(profile) = device_manager_clone.get_active_profile() {
                crate::device_status::set_active_controller_type(Some(
                    profile.controller_type.as_str().to_string(),
                ));
                crate::device_status::set_active_pen_mode(profile.pen_mode);
            } else {
                crate::device_status::set_active_controller_type(None);
                crate::device_status::set_active_pen_mode(false);
            }
            glib::ControlFlow::Continue
        });

        // Sync active device num_axes to global state
        if let Some(profile) = device_manager.get_active_profile() {
            crate::device_status::set_active_num_axes(profile.num_axes);

            // Usamos as_str() (== Display) para coherencia con el combo y el registry.
            let controller_type = profile.controller_type.as_str().to_string();
            let device_name = &profile.name;
            let device_type = format!("{:?}", profile.device_type);

            // Publicar el ControllerType de la activa en el estado global
            crate::device_status::set_active_controller_type(Some(controller_type.clone()));
            crate::device_status::set_active_pen_mode(profile.pen_mode);

            status_bar.set_device_info(device_name, &controller_type, &device_type);
        }

        let device_controller = Rc::new(gcodekit5_devicedb::DeviceUiController::new(
            device_manager.clone(),
        ));

        // G-Code Editor (Moved up to be available for MachineControl)
        let editor = Rc::new(GcodeEditor::new(
            Some(status_bar.clone()),
            Some(settings_controller.clone()),
        ));

        // Apply initial theme
        let current_theme = settings_persistence.borrow().config().ui.theme;
        apply_theme(current_theme);
        let editor_for_theme = editor.clone();
        // Listen for theme changes
        settings_controller.on_setting_changed(move |key, value| {
            if key == "theme" {
                let theme = match value {
                    "Light" => Theme::Light,
                    "Dark" => Theme::Dark,
                    _ => Theme::System,
                };
                apply_theme(theme);
                // Actualizar el editor inmediatamente
                editor_for_theme.update_theme_for_editor();
            }
        });

        // ==========================================
        // DESIGNER
        // ==========================================
        let designer = DesignerView::new(
            Some(device_manager.clone()),
            settings_controller.clone(),
            Some(status_bar.clone()),
        );

        // Visualizer (Created early for MachineControl dependency)
        let visualizer = Rc::new(GcodeVisualizer::new(
            Some(device_manager.clone()),
            settings_controller.clone(),
            Some(status_bar.clone()),
            Some(designer.get_state()),
        ));

        // Keep window title synced with active document context.
        {
            let window_for_title = window.clone();
            let stack_for_title = stack.clone();
            let designer_for_title = designer.clone();
            let mut last_title = String::new();

            glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
                let title = if let Some(name) = stack_for_title.visible_child_name() {
                    if name.as_str() == "designer" {
                        format!(
                            "{} - {}",
                            t!("GCodeKit5"),
                            designer_for_title.window_title_suffix()
                        )
                    } else {
                        t!("GCodeKit5")
                    }
                } else {
                    t!("GCodeKit5")
                };

                if title != last_title {
                    window_for_title.set_title(Some(&title));
                    last_title = title;
                }

                glib::ControlFlow::Continue
            });
        }

        // Forzar ajuste al área de trabajo del dispositivo después
        // de que la ventana esté cargada
        let designer_fit = designer.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(1500), move || {
            designer_fit.canvas.fit_to_device_area();
            designer_fit.canvas.widget.queue_draw();
            //    println!("fit_to_device_area aplicado al designer");
            glib::ControlFlow::Break
        });

        // ==========================================
        // MACHINE CONTROL
        // ==========================================
        let machine_control = MachineControlView::new(
            Some(status_bar.clone()),
            Some(device_console.clone()),
            Some(editor.clone()),
            Some(visualizer.clone()),
            Some(settings_controller.clone()),
        );
        // The active device profile decides whether Connect opens a serial port or a
        // TCP/WebSocket host; keep the port area in sync when the profile changes.
        machine_control.set_device_manager(device_manager.clone());
        {
            let mc = machine_control.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(1000), move || {
                mc.refresh_connection_target();
                glib::ControlFlow::Continue
            });
        }

        // ==========================================
        // CAM TOOLS
        // ==========================================
        let stack_for_cam = stack.clone();
        let editor_for_cam = editor.clone();
        let cam_tools_view = CamToolsView::new_with_designer(
            settings_controller.clone(),
            Some(machine_control.clone()),
            move |gcode| {
                editor_for_cam.set_text(&gcode);
                stack_for_cam.set_visible_child_name("visualizer");
                editor_for_cam.grab_focus();
            },
            Some(designer.clone()),
            Some(device_manager.clone()),
        );

        // ==========================================
        // DEVICE CONFIG
        // ==========================================
        let config_settings = ConfigSettingsView::new(settings_controller.clone());
        config_settings.set_communicator(machine_control.communicator.clone());
        config_settings.set_device_console(device_console.clone());
        config_settings.set_device_manager(device_manager.clone());

        // ==========================================
        // DEVICE MANAGER
        // ==========================================
        let device_manager_view =
            DeviceManagerWindow::new(device_controller.clone(), settings_controller.clone());

        // ==========================================
        // CNC TOOLS
        // ==========================================
        let tools_manager = ToolsManagerView::new(settings_controller.clone());

        // ==========================================
        // MATERIALS
        // ==========================================
        let materials_manager = MaterialsManagerView::new();

        // ==========================================
        // AÑADIR PESTAÑAS AL STACK (ORDEN)
        // ==========================================
        // 1. Diseñador
        stack.add_titled(&designer.widget, Some("designer"), &t!("Designer"));

        // 2. Visualizador
        stack.add_titled(&visualizer.widget, Some("visualizer"), &t!("Visualizer"));

        // 3. Control de máquina
        stack.add_titled(
            &machine_control.widget,
            Some("machine"),
            &t!("Machine Control"),
        );

        // 4. Herramientas CAM
        stack.add_titled(cam_tools_view.widget(), Some("cam_tools"), &t!("CAM Tools"));

        // 5. Administrador de dispositivos
        stack.add_titled(
            &device_manager_view.widget,
            Some("devices"),
            &t!("Device Manager"),
        );

        // 6. Configuración de dispositivo
        stack.add_titled(
            &config_settings.container,
            Some("config"),
            &t!("Device Config"),
        );

        // 7. Herramientas CNC
        stack.add_titled(&tools_manager.widget, Some("tools"), &t!("CNC Tools"));

        // 8. Materiales
        stack.add_titled(
            &materials_manager.widget,
            Some("materials"),
            &t!("Materials"),
        );

        // ==========================================
        // CONEXIONES ENTRE COMPONENTES
        // ==========================================

        // Wire up ConsoleListener for command parsing and logging
        let console_manager = crate::ui::device_console_manager::get_console_manager();
        let console_listener =
            crate::ui::device_console_manager::ConsoleListener::new(console_manager);

        {
            let mut comm = machine_control.communicator.lock();
            comm.add_listener(console_listener);
        }

        // Console send (button + Enter) is wired by MachineControlView, which shares this
        // DeviceConsoleView; wiring it here as well registered a second, order-dependent
        // handler for the same signals.

        // Connect Editor to Visualizer
        let vis_clone = visualizer.clone();
        editor.connect_changed(move |buffer| {
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let text = buffer.text(&start, &end, true);
            vis_clone.set_gcode(&text);
        });

        let editor_clone_gen = editor.clone();
        let stack_clone_gen = stack.clone();
        designer.set_on_gcode_generated(move |gcode| {
            editor_clone_gen.set_text(&gcode);
            //            stack_clone_gen.set_visible_child_name("machine");
            stack_clone_gen.set_visible_child_name("visualizer");
            editor_clone_gen.grab_focus();
        });

        // Connect device info and config to machine control connection state
        let config_settings_clone = config_settings.clone();
        let communicator_for_device = machine_control.communicator.clone();

        // Update Device Config when the connection or the reported device info changes.
        // Only push changes: re-applying them every tick overwrote Device Config status
        // messages and rebuilt the capabilities list twice a second.
        let last_device_info: Rc<RefCell<Option<DeviceInfoSnapshot>>> = Rc::new(RefCell::new(None));
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            // Don't block the GTK main thread while a streaming thread holds the port.
            let Some(comm) = communicator_for_device.try_lock() else {
                return glib::ControlFlow::Continue;
            };
            let connected = comm.is_connected();
            drop(comm);

            let snapshot = if connected {
                let status = device_status::get_status();
                let device_name = status
                    .device_name
                    .clone()
                    .or_else(|| status.port_name.clone())
                    .unwrap_or_else(|| t!("CNC Device").to_string());
                DeviceInfoSnapshot {
                    connected: true,
                    device_name,
                    firmware_type: status
                        .firmware_type
                        .clone()
                        .unwrap_or_else(|| "GRBL".to_string()),
                    firmware_version: status
                        .firmware_version
                        .clone()
                        .unwrap_or_else(|| t!("Unknown").to_string()),
                }
            } else {
                DeviceInfoSnapshot::default()
            };

            let mut last = last_device_info.borrow_mut();
            let connection_changed = last.as_ref().map(|l| l.connected) != Some(connected);
            if connection_changed {
                config_settings_clone.set_connected(connected);
            }
            if last.as_ref() != Some(&snapshot) {
                config_settings_clone.set_device_info(
                    snapshot.connected,
                    &snapshot.device_name,
                    &snapshot.firmware_type,
                    &snapshot.firmware_version,
                );
                *last = Some(snapshot);
            }
            drop(last);

            // Settings from `$$` arrive after connecting; pick up new ones (cheap no-op
            // when nothing changed).
            if connected {
                config_settings_clone.sync_device_settings();
            }

            glib::ControlFlow::Continue
        });

        main_box.append(&content_box);

        // Append the StatusBar; Preferences > Show Status Bar controls it.
        main_box.append(&status_bar.widget);
        let show_status_bar = settings_persistence
            .borrow()
            .config()
            .ui
            .panel_visibility
            .get("status_bar")
            .copied()
            .unwrap_or(true);
        status_bar.widget.set_visible(show_status_bar);
        {
            let status_bar_widget = status_bar.widget.clone();
            settings_controller.on_setting_changed(move |key, value| {
                if key == "show_status_bar" {
                    status_bar_widget.set_visible(value == "true");
                }
            });
        }

        // Connect eStop (Ctrl-X / 0x18): delegate to MachineControlView so the DirectSender
        // worker thread is stopped too (resetting only the flags let it keep streaming).
        {
            let machine_control_estop = machine_control.clone();
            status_bar.estop_btn.connect_clicked(move |_| {
                machine_control_estop.emergency_stop();
            });
        }

        window.set_child(Some(&main_box));

        // Actions
        let settings_action = gio::SimpleAction::new("preferences", None);
        let settings_controller_clone = settings_controller.clone();
        let visualizer_settings = visualizer.clone();
        let designer_settings = designer.clone();
        settings_action.connect_activate(move |_, _| {
            let visualizer_redraw = visualizer_settings.clone();
            let designer_redraw = designer_settings.clone();
            let on_save = Box::new(move || {
                // Queue redraws for visualizer and designer when settings are saved
                visualizer_redraw.queue_draw();
                designer_redraw.queue_draw();
            });
            let win =
                SettingsWindow::new_with_callback(settings_controller_clone.clone(), Some(on_save));
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
        let editor_run = editor.clone();
        let visualizer_run = visualizer.clone();
        let stack_clone = stack.clone();
        let window_run = window.clone();
        run_action.connect_activate(move |_, _| {
            let gcode = editor_run.get_text();
            stack_clone.set_visible_child_name("visualizer");

            match visualizer_run.run_preview_from_gcode(&gcode) {
                crate::ui::gtk::visualizer::RunPreviewResult::Started => {}
                crate::ui::gtk::visualizer::RunPreviewResult::EmptyInput => {
                    crate::ui::gtk::common::dialog::show_warning(
                        &t!("No G-code to run"),
                        &t!("Generate or open G-code before using Run."),
                        Some(window_run.upcast_ref::<gtk4::Window>()),
                    );
                }
                crate::ui::gtk::visualizer::RunPreviewResult::NoMotion => {
                    crate::ui::gtk::common::dialog::show_warning(
                        &t!("No motion commands found"),
                        &t!("The loaded G-code has no G0/G1/G2/G3 movement to preview."),
                        Some(window_run.upcast_ref::<gtk4::Window>()),
                    );
                }
                crate::ui::gtk::visualizer::RunPreviewResult::NoTrajectory => {
                    crate::ui::gtk::common::dialog::show_warning(
                        &t!("No preview trajectory generated"),
                        &t!("Run could not build a visible preview path from the current G-code."),
                        Some(window_run.upcast_ref::<gtk4::Window>()),
                    );
                }
            }
        });
        app.add_action(&run_action);

        // Generate Frame
        let editor_frame = editor.clone();
        let designer_frame = designer.clone();
        let stack_frame = stack.clone();

        // We clone so that the "closure" owns the references
        designer.toolbox.connect_frame_clicked(move || {
            // 1. Obtain drawing limits adapted for raster image overscan
            if let Some((x1, y1, x2, y2)) = designer_frame.get_frame_bounds() {
                // 2. Generate the G-Code string
                let gcode = crate::ui::gtk::designer_toolbox::generate_frame_gcode(x1, y1, x2, y2);
                // 3. Insert into the editor
                editor_frame.set_text(&gcode);
                // 4. Jump to the editor tab to view the code
                //                stack_frame.set_visible_child_name("machine");
                stack_frame.set_visible_child_name("visualizer");
            } else {
                tracing::debug!("Frame requested but canvas is empty - nothing to frame");
            }
        });

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
                    "machine" => editor_clone.new_file(),
                    _ => {}
                }
            }
        });
        app.add_action(&new_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let new_2d_action = gio::SimpleAction::new("file_new_2d", None);
        new_2d_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.new_file_2d(),
                    "editor" => editor_clone.new_file(),
                    "machine" => editor_clone.new_file(),
                    _ => {}
                }
            }
        });
        app.add_action(&new_2d_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let new_3d_action = gio::SimpleAction::new("file_new_3d", None);
        new_3d_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.new_file_3d(),
                    "editor" => editor_clone.new_file(),
                    "machine" => editor_clone.new_file(),
                    _ => {}
                }
            }
        });
        app.add_action(&new_3d_action);

        let stack_clone = stack.clone();
        let designer_clone = designer.clone();
        let editor_clone = editor.clone();
        let open_action = gio::SimpleAction::new("file_open", None);
        open_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone.visible_child_name() {
                match name.as_str() {
                    "designer" => designer_clone.open_file(),
                    "editor" => editor_clone.open_file(),
                    "machine" => editor_clone.open_file(),
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
                    "machine" => editor_clone.save_file(),
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
                    "machine" => editor_clone.save_as_file(),
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
                if name.as_str() == "designer" {
                    designer_clone.import_file()
                }
            }
        });
        app.add_action(&import_action);

        let import_image_action = gio::SimpleAction::new("file_import_image", None);

        let designer_clone_image = designer.clone();
        import_image_action.connect_activate(move |_, _| {
            designer_clone_image.canvas.import_raster_image();
        });
        app.add_action(&import_image_action);
        app.set_accels_for_action("app.file_import_image", &["<Control><Shift>i"]);

        let export_gcode_action = gio::SimpleAction::new("file_export_gcode", None);
        let designer_clone_gcode = designer.clone();
        let stack_clone_gcode = stack.clone();
        export_gcode_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone_gcode.visible_child_name() {
                if name.as_str() == "designer" {
                    designer_clone_gcode.export_gcode()
                }
            }
        });
        app.add_action(&export_gcode_action);

        let export_svg_action = gio::SimpleAction::new("file_export_svg", None);
        let designer_clone_svg = designer.clone();
        let stack_clone_svg = stack.clone();
        export_svg_action.connect_activate(move |_, _| {
            if let Some(name) = stack_clone_svg.visible_child_name() {
                if name.as_str() == "designer" {
                    designer_clone_svg.export_svg()
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
                .authors(vec![t!(
                    "Tim Hawkins and GCodeKit Contributors: \n (feveal)"
                )])
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
                    if label.text() == "GCodeKit5 Design" {
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
                    "machine" => editor_clone.undo(),
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
                    "machine" => editor_clone.redo(),
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
                    "machine" => editor_clone.cut(),
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
                    "machine" => editor_clone.copy(),
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
                    "machine" => editor_clone.paste(),
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
                // Close the main window instead of quitting outright so Quit goes
                // through the same unsaved-changes prompt as the window close button
                // (see `connect_close_request` below). The window's destroy handler
                // ends the application once the close goes ahead.
                let window_weak = window.downgrade();
                action.connect_activate(move |_, _| {
                    if let Some(window) = window_weak.upgrade() {
                        window.close();
                    }
                });
            } else if name == "machine_connect" || name == "machine_disconnect" {
                // Toggle via the Machine Control connect button, but only in the
                // direction the menu item names.
                let want_connected = name == "machine_connect";
                let mc = machine_control.clone();
                action.connect_activate(move |_, _| {
                    // The button is insensitive while a connect is in progress; its worker
                    // thread then holds the port lock, so never block on it here.
                    if !mc.connect_btn.is_sensitive() {
                        return;
                    }
                    let Some(connected) = mc.communicator.try_lock().map(|c| c.is_connected())
                    else {
                        return;
                    };
                    if connected != want_connected {
                        mc.connect_btn.emit_clicked();
                    }
                });
            } else if name == "machine_home" {
                let mc = machine_control.clone();
                action.connect_activate(move |_, _| {
                    // Respect the button state (disabled while disconnected).
                    if mc.home_btn.is_sensitive() {
                        mc.home_btn.emit_clicked();
                    }
                });
            } else if name == "machine_reset" {
                let mc = machine_control.clone();
                action.connect_activate(move |_, _| {
                    if mc.communicator.lock().is_connected() {
                        mc.emergency_stop();
                    }
                });
            } else if name == "help_docs" {
                // Acción para abrir la ayuda principal
                action.connect_activate(move |_, _| {
                    help_browser::present("index");
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
        let designer_for_visualizer_sync = designer.clone();
        let visualizer_for_tool_sync = visualizer.clone();
        stack.connect_visible_child_name_notify(move |stack| {
            if let Some(name) = stack.visible_child_name() {
                let name_str = name.as_str();
                let is_designer = name_str == "designer";
                let is_editor = name_str == "editor";
                let is_machine = name_str == "machine";
                let is_visualizer = name_str == "visualizer";

                if is_visualizer {
                    let tool_diameter_mm = designer_for_visualizer_sync.current_tool_diameter_mm();
                    visualizer_for_tool_sync.set_stock_tool_diameter_mm(tool_diameter_mm);
                }

                // Las acciones del editor también deben habilitarse en "machine"
                let enable_editor_actions = is_designer || is_editor || is_machine;

                let set_enabled = |action_name: &str, enabled: bool| {
                    if let Some(action) = app_clone.lookup_action(action_name) {
                        if let Some(simple_action) = action.downcast_ref::<gio::SimpleAction>() {
                            simple_action.set_enabled(enabled);
                        }
                    }
                };

                // Edit actions - ahora también en machine
                set_enabled("edit_undo", enable_editor_actions);
                set_enabled("edit_redo", enable_editor_actions);
                set_enabled("edit_cut", enable_editor_actions);
                set_enabled("edit_copy", enable_editor_actions);
                set_enabled("edit_paste", enable_editor_actions);

                // File actions - ahora también en machine
                set_enabled("file_new", enable_editor_actions);
                set_enabled("file_new_2d", enable_editor_actions);
                set_enabled("file_new_3d", enable_editor_actions);
                set_enabled("file_open", enable_editor_actions);
                set_enabled("file_save", enable_editor_actions);
                set_enabled("file_save_as", enable_editor_actions);
                set_enabled("file_run", is_editor || is_machine || is_visualizer);

                // Acciones exclusivas del diseñador
                set_enabled("file_import", is_designer);
                set_enabled("file_import_image", is_designer);
                set_enabled("file_export_gcode", is_designer);
                set_enabled("file_export_svg", is_designer);
            }
        });

        // Set Keyboard Shortcuts (Accelerators)
        // Using centralized constants from common::accelerators module
        use crate::ui::gtk::common::accelerators::StandardShortcuts;

        app.set_accels_for_action("app.file_new_2d", &[StandardShortcuts::FILE_NEW]);
        app.set_accels_for_action("app.file_open", &[StandardShortcuts::FILE_OPEN]);
        app.set_accels_for_action("app.file_save", &[StandardShortcuts::FILE_SAVE]);
        app.set_accels_for_action("app.file_save_as", &[StandardShortcuts::FILE_SAVE_AS]);
        app.set_accels_for_action("app.file_run", &[StandardShortcuts::FILE_RUN, "F5"]);
        app.set_accels_for_action("app.quit", &[StandardShortcuts::FILE_QUIT]);

        app.set_accels_for_action("app.edit_undo", &[StandardShortcuts::EDIT_UNDO]);
        app.set_accels_for_action(
            "app.edit_redo",
            &[StandardShortcuts::EDIT_REDO, "<Control><Shift>z"],
        );
        app.set_accels_for_action("app.edit_cut", &[StandardShortcuts::EDIT_CUT]);
        app.set_accels_for_action("app.edit_copy", &[StandardShortcuts::EDIT_COPY]);
        app.set_accels_for_action("app.edit_paste", &[StandardShortcuts::EDIT_PASTE]);

        app.set_accels_for_action("app.help_docs", &[StandardShortcuts::HELP_DOCS]);
        app.set_accels_for_action("app.machine_home", &[StandardShortcuts::MACHINE_HOME]);

        // Set initial tab based on user configuration
        let startup_tab = settings_persistence.borrow().config().ui.startup_tab;
        let initial_tab = match startup_tab {
            StartupTab::Designer => "designer",
            StartupTab::Visualizer => "visualizer",
            StartupTab::Machine => "machine", // Editor está dentro de machine
            StartupTab::Editor => "machine",
            StartupTab::Console => "machine", // Console está dentro de machine
            StartupTab::CamTools => "cam_tools",
            StartupTab::DeviceInfo => "config", // Device Info está en config
            StartupTab::Config => "config",
            StartupTab::Devices => "devices",
            StartupTab::Tools => "tools",
            StartupTab::Materials => "materials",
        };
        stack.set_visible_child_name(initial_tab);

        // Remember the window size for the next start. Connected before the
        // unsaved-changes prompt so it runs on every close request.
        {
            let persistence = settings_persistence.clone();
            window.connect_close_request(move |win| {
                if !win.is_maximized() {
                    let (width, height) = (win.default_width(), win.default_height());
                    if width > 0 && height > 0 {
                        if let Ok(mut p) = persistence.try_borrow_mut() {
                            p.config_mut().ui.window_width = width as u32;
                            p.config_mut().ui.window_height = height as u32;
                            p.persist_or_log();
                        }
                    }
                }
                glib::Propagation::Proceed
            });
        }

        // Ask before closing the window while the design has unsaved changes.
        {
            let designer_close = designer.clone();
            window.connect_close_request(move |win| {
                if !designer_close.has_unsaved_changes() {
                    return glib::Propagation::Proceed;
                }
                let win = win.clone();
                // `destroy` does not emit close-request again, so this cannot loop.
                designer_close.confirm_discard_changes(move || win.destroy());
                glib::Propagation::Stop
            });

            // Once the main window is gone the application is finished, even if
            // other application windows (e.g. the help browser) are still open.
            let app_for_quit = app.clone();
            window.connect_destroy(move |_| {
                tracing::info!("Main window destroyed; quitting application");
                app_for_quit.quit();
            });
        }

        //        window.maximize();
        window.present();

        if let Some(details) = settings_load_error {
            crate::ui::gtk::common::dialog::show_warning(
                &t!("Settings could not be loaded"),
                &format!(
                    "{}\n\n{details}",
                    t!("Default settings are in use. The settings file will be backed up before it is replaced.")
                ),
                Some(window.upcast_ref()),
            );
        }

        // Forzar actualización del editor después de que la ventana esté visible
        let editor_for_startup = editor.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            editor_for_startup.update_theme_for_editor();
            glib::ControlFlow::Break
        });

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
                    .authors(vec![t!(
                        "Tim Hawkins and GCodeKit Contributors: \n (feveal)"
                    )])
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
                        if label.text() == "GCodeKit5 Design" {
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
    match gio::Resource::from_data(&resource_data) {
        Ok(resource) => gio::resources_register(&resource),
        Err(e) => {
            tracing::error!("Failed to load resources: {}", e);
            std::process::exit(1);
        }
    }
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("ui/gtk/style.css"));

    let Some(display) = gtk4::gdk::Display::default() else {
        tracing::error!("Could not connect to a display");
        return;
    };
    gtk4::style_context_add_provider_for_display(
        &display,
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

/// Connection and firmware details last pushed to the Device Config view, so the
/// periodic refresh only updates it when something actually changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DeviceInfoSnapshot {
    connected: bool,
    device_name: String,
    firmware_type: String,
    firmware_version: String,
}
