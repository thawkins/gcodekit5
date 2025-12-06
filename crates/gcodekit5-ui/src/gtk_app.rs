use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Orientation, Stack, StackSwitcher, HeaderBar,
    MenuButton, PopoverMenu, Label, Button, CssProvider, StyleContext, PopoverMenuBar,
};
use libadwaita::prelude::*;
use libadwaita::Application as AdwApplication;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::path::PathBuf;

use crate::ui::gtk::editor::GcodeEditor;
use crate::ui::gtk::visualizer::GcodeVisualizer;
use crate::ui::gtk::designer::DesignerCanvas;
use crate::ui::gtk::settings::SettingsWindow;
use crate::ui::gtk::device_manager::DeviceManagerWindow;
use crate::ui::gtk::cam_tools::CamToolsView;

use gcodekit5_settings::{SettingsController, SettingsDialog, SettingsPersistence, SettingsManager};
use gcodekit5_devicedb::{DeviceManager, DeviceUiController};
use gcodekit5_designer::designer_state::DesignerState;

pub fn main() {
    let app = AdwApplication::builder()
        .application_id("com.github.thawkins.gcodekit5")
        .build();

    app.connect_startup(|_| {
        load_css();
    });

    app.connect_activate(|app| {
        // Initialize Backend Systems
        let config_path = SettingsManager::config_file_path().unwrap_or_else(|_| PathBuf::from("config.json"));
        
        let persistence = if config_path.exists() {
            SettingsPersistence::load_from_file(&config_path).unwrap_or_else(|e| {
                eprintln!("Failed to load settings: {}", e);
                SettingsPersistence::new()
            })
        } else {
            SettingsPersistence::new()
        };
        
        let settings_persistence = Rc::new(RefCell::new(persistence));
        let settings_dialog = Rc::new(RefCell::new(SettingsDialog::new()));
        
        // Populate dialog with settings
        settings_persistence.borrow().populate_dialog(&mut settings_dialog.borrow_mut());
        
        let settings_controller = Rc::new(SettingsController::new(settings_dialog.clone(), settings_persistence.clone()));
        
        // Ensure config dir exists
        let _ = SettingsManager::ensure_config_dir();

        let config_dir = dirs::config_dir()
            .map(|p| p.join("gcodekit5"))
            .unwrap_or_else(|| PathBuf::from("config"));
            
        let device_manager = Arc::new(DeviceManager::new(config_dir));
        let device_controller = Rc::new(DeviceUiController::new(device_manager.clone()));

        let designer_state = Rc::new(RefCell::new(DesignerState::new()));

        // Build UI
        let window = ApplicationWindow::builder()
            .application(app)
            .title("GCodeKit5")
            .default_width(1200)
            .default_height(800)
            .build();

        let main_box = Box::new(Orientation::Vertical, 0);
        
        // Header Bar
        let header = HeaderBar::new();
        window.set_titlebar(Some(&header));

        // Main Menu Bar
        let menu_bar_model = gio::Menu::new();

        // File Menu
        let file_menu = gio::Menu::new();
        file_menu.append(Some("New"), Some("app.file_new"));
        file_menu.append(Some("Open..."), Some("app.file_open"));
        file_menu.append(Some("Save"), Some("app.file_save"));
        file_menu.append(Some("Save As..."), Some("app.file_save_as"));
        file_menu.append(Some("Export..."), Some("app.file_export"));
        file_menu.append(Some("Exit"), Some("app.quit"));
        menu_bar_model.append_submenu(Some("File"), &file_menu);

        // Edit Menu
        let edit_menu = gio::Menu::new();
        edit_menu.append(Some("Undo"), Some("app.edit_undo"));
        edit_menu.append(Some("Redo"), Some("app.edit_redo"));
        edit_menu.append(Some("Cut"), Some("app.edit_cut"));
        edit_menu.append(Some("Copy"), Some("app.edit_copy"));
        edit_menu.append(Some("Paste"), Some("app.edit_paste"));
        
        let preferences_section = gio::Menu::new();
        preferences_section.append(Some("Preferences"), Some("app.preferences"));
        edit_menu.append_section(None, &preferences_section);
        
        menu_bar_model.append_submenu(Some("Edit"), &edit_menu);

        // View Menu
        let view_menu = gio::Menu::new();
        view_menu.append(Some("Toolbars"), Some("app.view_toolbars"));
        view_menu.append(Some("Status Bar"), Some("app.view_status_bar"));
        view_menu.append(Some("Console"), Some("app.view_console"));
        view_menu.append(Some("Visualizer"), Some("app.view_visualizer"));
        menu_bar_model.append_submenu(Some("View"), &view_menu);

        // Tools Menu
        let tools_menu = gio::Menu::new();
        tools_menu.append(Some("Device Manager"), Some("app.devices"));
        tools_menu.append(Some("CAM Tools"), Some("app.cam_tools"));
        menu_bar_model.append_submenu(Some("Tools"), &tools_menu);

        // Machine Menu
        let machine_menu = gio::Menu::new();
        machine_menu.append(Some("Connect"), Some("app.machine_connect"));
        machine_menu.append(Some("Disconnect"), Some("app.machine_disconnect"));
        machine_menu.append(Some("Home"), Some("app.machine_home"));
        machine_menu.append(Some("Reset"), Some("app.machine_reset"));
        menu_bar_model.append_submenu(Some("Machine"), &machine_menu);

        // Help Menu
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
        
        // 1. Editor
        let editor = Rc::new(GcodeEditor::new());
        stack.add_titled(&editor.widget, Some("editor"), "G-Code Editor");
        
        // 2. Visualizer
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
        
        // 3. Designer
        let designer = DesignerCanvas::new(designer_state.clone());
        stack.add_titled(&designer.widget, Some("designer"), "Designer");
        
        // 4. Device Manager
        let device_manager_view = DeviceManagerWindow::new(device_controller.clone());
        stack.add_titled(device_manager_view.widget(), Some("devices"), "Device Manager");

        // 5. CAM Tools
        let editor_clone = editor.clone();
        let stack_clone_for_cam = stack.clone();
        let cam_tools_view = CamToolsView::new(move |gcode| {
            editor_clone.set_text(&gcode);
            stack_clone_for_cam.set_visible_child_name("editor");
        });
        stack.add_titled(cam_tools_view.widget(), Some("cam_tools"), "CAM Tools");

        // 6. Machine Control (Placeholder)
        let machine_box = Box::new(Orientation::Vertical, 0);
        machine_box.append(&Label::new(Some("Machine Control Panel (Coming Soon)")));
        stack.add_titled(&machine_box, Some("machine"), "Machine Control");

        main_box.append(&content_box);
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

        // Placeholder Actions for Menu Items
        let action_names = vec![
            "file_new", "file_open", "file_save", "file_save_as", "file_export", "quit",
            "edit_undo", "edit_redo", "edit_cut", "edit_copy", "edit_paste",
            "view_toolbars", "view_status_bar", "view_console", "view_visualizer",
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
