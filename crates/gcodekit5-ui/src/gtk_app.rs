use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Orientation, Stack, StackSidebar, HeaderBar,
    MenuButton, PopoverMenu, Label, Button, CssProvider, StyleContext,
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
        
        // Menu
        let menu_model = gio::Menu::new();
        menu_model.append(Some("Preferences"), Some("app.preferences"));
        menu_model.append(Some("Device Manager"), Some("app.devices"));
        menu_model.append(Some("CAM Tools"), Some("app.cam_tools"));
        menu_model.append(Some("About"), Some("app.about"));
        
        let menu_btn = MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu_model)
            .tooltip_text("Main Menu")
            .build();
        header.pack_end(&menu_btn);
        
        main_box.append(&header);

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
