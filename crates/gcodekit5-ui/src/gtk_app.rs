use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Orientation, Stack, StackSidebar, HeaderBar,
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
use crate::ui::gtk::cam_tools::TabbedBoxDialog;

use gcodekit5_settings::{SettingsController, SettingsDialog, SettingsPersistence};
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
        let settings_dialog = Rc::new(RefCell::new(SettingsDialog::new()));
        let settings_persistence = Rc::new(RefCell::new(SettingsPersistence::new()));
        let settings_controller = Rc::new(SettingsController::new(settings_dialog.clone(), settings_persistence.clone()));
        
        // Load settings
        if let Err(e) = settings_controller.save() { 
            eprintln!("Settings init warning: {}", e);
        }

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
        main_box.append(&header);

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
        let content_box = Box::new(Orientation::Horizontal, 0);
        let sidebar = StackSidebar::new();
        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);
        
        sidebar.set_stack(&stack);
        sidebar.set_width_request(200);
        
        content_box.append(&sidebar);
        content_box.append(&stack);
        
        // 1. Editor
        let editor = GcodeEditor::new();
        stack.add_titled(&editor.widget, Some("editor"), "G-Code Editor");
        
        // 2. Visualizer
        let visualizer = GcodeVisualizer::new();
        stack.add_titled(&visualizer.widget, Some("visualizer"), "Visualizer");
        
        // 3. Designer
        let designer = DesignerCanvas::new(designer_state.clone());
        stack.add_titled(&designer.widget, Some("designer"), "Designer");
        
        // 4. Machine Control (Placeholder)
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

        let devices_action = gio::SimpleAction::new("devices", None);
        let device_controller_clone = device_controller.clone();
        devices_action.connect_activate(move |_, _| {
            let win = DeviceManagerWindow::new(device_controller_clone.clone());
            win.present();
        });
        app.add_action(&devices_action);

        let cam_action = gio::SimpleAction::new("cam_tools", None);
        cam_action.connect_activate(move |_, _| {
            let win = TabbedBoxDialog::new();
            win.present();
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
