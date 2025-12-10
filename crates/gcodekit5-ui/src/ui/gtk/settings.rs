use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, Entry, FileChooserAction, FileChooserNative, Orientation, ResponseType,
    StringList, Switch,
};
use libadwaita::prelude::*;
use libadwaita::{
    ActionRow, ComboRow, PreferencesGroup, PreferencesPage, PreferencesRow, PreferencesWindow,
};
use std::rc::Rc;
use tracing::error;

use gcodekit5_settings::controller::{SettingUiModel, SettingsController};
use gcodekit5_settings::view_model::SettingsCategory;

pub struct SettingsWindow {
    window: PreferencesWindow,
    controller: Rc<SettingsController>,
}

impl SettingsWindow {
    pub fn new(controller: Rc<SettingsController>) -> Self {
        let window = PreferencesWindow::builder()
            .title("Preferences")
            .modal(true)
            .default_width(800)
            .default_height(600)
            .build();

        let settings_window = Self { window, controller };
        settings_window.setup_pages();
        settings_window
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn setup_pages(&self) {
        // General Page
        self.add_page(SettingsCategory::General, "General", "system-run-symbolic");

        // Controller Page
        self.add_page(
            SettingsCategory::Controller,
            "Controller",
            "input-gaming-symbolic",
        );

        // UI Page
        self.add_page(
            SettingsCategory::UserInterface,
            "User Interface",
            "preferences-desktop-display-symbolic",
        );

        // File Processing Page
        self.add_page(
            SettingsCategory::FileProcessing,
            "File Processing",
            "document-open-symbolic",
        );

        // Shortcuts Page
        self.add_page(
            SettingsCategory::KeyboardShortcuts,
            "Shortcuts",
            "input-keyboard-symbolic",
        );

        // Advanced Page
        self.add_page(
            SettingsCategory::Advanced,
            "Advanced",
            "preferences-system-symbolic",
        );
    }

    fn add_page(&self, category: SettingsCategory, title: &str, icon_name: &str) {
        let page = PreferencesPage::builder()
            .title(title)
            .icon_name(icon_name)
            .build();

        let group = PreferencesGroup::builder().title(title).build();

        let settings = self.controller.get_settings_for_ui(Some(category));

        for setting in settings {
            let row = self.create_setting_row(&setting);
            group.add(&row);
        }

        page.add(&group);
        self.window.add(&page);
    }

    fn create_setting_row(&self, setting: &SettingUiModel) -> PreferencesRow {
        let controller = self.controller.clone();
        let id = setting.id.clone();

        match setting.value_type.as_str() {
            "Boolean" => {
                let row = ActionRow::builder()
                    .title(&setting.name)
                    .subtitle(&setting.description)
                    .build();

                let switch = Switch::builder()
                    .active(setting.value == "true")
                    .valign(Align::Center)
                    .build();

                let id_clone = id.clone();
                let controller_clone = controller.clone();

                switch.connect_state_set(move |_, state| {
                    controller_clone.update_setting(&id_clone, &state.to_string());
                    if let Err(e) = controller_clone.save() {
                        error!("Failed to save settings: {}", e);
                    }
                    glib::Propagation::Proceed
                });

                row.add_suffix(&switch);
                row.set_activatable_widget(Some(&switch));
                row.upcast()
            }
            "Enum" => {
                let model = StringList::new(&[]);
                for option in &setting.options {
                    model.append(option);
                }

                let row = ComboRow::builder()
                    .title(&setting.name)
                    .subtitle(&setting.description)
                    .model(&model)
                    .selected(setting.current_index as u32)
                    .build();

                let id_clone = id.clone();
                let controller_clone = controller.clone();
                let options = setting.options.clone();

                row.connect_selected_notify(move |r| {
                    let idx = r.selected() as usize;
                    if let Some(val) = options.get(idx) {
                        controller_clone.update_setting(&id_clone, val);
                        let _ = controller_clone.save();
                    }
                });

                row.upcast()
            }
            "Path" => {
                let row = ActionRow::builder()
                    .title(&setting.name)
                    .subtitle(&setting.description)
                    .build();

                let entry = Entry::builder()
                    .text(&setting.value)
                    .valign(Align::Center)
                    .width_chars(20)
                    .build();

                let browse_btn = Button::builder()
                    .icon_name("folder-open-symbolic")
                    .valign(Align::Center)
                    .build();

                let parent_window = self.window.clone();
                let entry_clone = entry.clone();

                browse_btn.connect_clicked(move |_| {
                    let file_chooser = FileChooserNative::builder()
                        .title("Select Directory")
                        .transient_for(&parent_window)
                        .action(FileChooserAction::SelectFolder)
                        .accept_label("Select")
                        .cancel_label("Cancel")
                        .modal(true)
                        .build();

                    let entry = entry_clone.clone();
                    file_chooser.connect_response(move |dialog, response| {
                        if response == ResponseType::Accept {
                            if let Some(file) = dialog.file() {
                                if let Some(path) = file.path() {
                                    entry.set_text(&path.to_string_lossy());
                                }
                            }
                        }
                        dialog.destroy();
                    });

                    file_chooser.show();
                });

                let id_clone = id.clone();
                let controller_clone = controller.clone();
                entry.connect_changed(move |e| {
                    controller_clone.update_setting(&id_clone, &e.text());
                    let _ = controller_clone.save();
                });

                let box_container = Box::new(Orientation::Horizontal, 6);
                box_container.append(&entry);
                box_container.append(&browse_btn);

                row.add_suffix(&box_container);
                row.upcast()
            }
            _ => {
                // String, Integer, Float
                let row = ActionRow::builder()
                    .title(&setting.name)
                    .subtitle(&setting.description)
                    .build();

                let entry = Entry::builder()
                    .text(&setting.value)
                    .valign(Align::Center)
                    .width_chars(20)
                    .build();

                let id_clone = id.clone();
                let controller_clone = controller.clone();
                entry.connect_changed(move |e| {
                    controller_clone.update_setting(&id_clone, &e.text());
                    let _ = controller_clone.save();
                });

                row.add_suffix(&entry);
                row.upcast()
            }
        }
    }
}
