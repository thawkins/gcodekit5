use std::rc::Rc;
use slint::ComponentHandle;
use crate::slint_generatedMainWindow::{self, MainWindow, SettingItem};
use gcodekit5::{SettingsController, SettingsCategory};
use slint::VecModel;
use tracing::warn;

pub fn register_callbacks(
    main_window: &MainWindow,
    settings_controller: Rc<SettingsController>,
) {
    // Bind Settings Controller callbacks
    {
        let controller = settings_controller.clone();
        let window_weak = main_window.as_weak();
        main_window.on_config_retrieve_settings(move || {
            if let Some(window) = window_weak.upgrade() {
                let category_str = window.get_settings_category();
                let category = if category_str == "All" {
                    None
                } else {
                    Some(SettingsController::get_category_from_string(&category_str))
                };

                let settings = controller.get_settings_for_ui(category);

                let slint_settings: Vec<SettingItem> = settings
                    .iter()
                    .map(|s| SettingItem {
                        id: s.id.clone().into(),
                        name: s.name.clone().into(),
                        value: s.value.clone().into(),
                        value_type: s.value_type.clone().into(),
                        category: s.category.clone().into(),
                        description: s.description.clone().into(),
                        options: slint::ModelRc::new(VecModel::from(
                            s.options
                                .iter()
                                .map(|o| o.clone().into())
                                .collect::<Vec<slint::SharedString>>(),
                        )),
                        current_index: s.current_index,
                    })
                    .collect();

                window.set_current_settings(slint::ModelRc::new(VecModel::from(slint_settings)));
            }
        });
    }

    {
        let controller = settings_controller.clone();
        let window_weak = main_window.as_weak();
        // let persistence_clone = settings_persistence.clone(); // Not available here, handled inside controller
        main_window.on_menu_settings_save(move || {
            match controller.save() {
                Ok(_) => {
                    warn!("Settings saved to file");
                    // Apply UI settings
                    if let Some(_window) = window_weak.upgrade() {
                        // let _persistence = persistence_clone.borrow();
                        // window.set_show_menu_shortcuts(persistence.config().ui.show_menu_shortcuts);
                    }
                }
                Err(e) => warn!("Failed to save settings: {}", e),
            }
        });
    }

    {
        let controller = settings_controller.clone();
        let window_weak = main_window.as_weak();
        main_window.on_menu_settings_restore_defaults(move || {
            controller.restore_defaults();
            if let Some(window) = window_weak.upgrade() {
                window.invoke_config_retrieve_settings();
            }
        });
    }

    {
        let controller = settings_controller.clone();
        let window_weak = main_window.as_weak();
        main_window.on_update_setting(move |id, value| {
            controller.update_setting(&id, &value);
            // Refresh UI
            if let Some(window) = window_weak.upgrade() {
                window.invoke_config_retrieve_settings();
            }
        });
    }

    {
        let window_weak = main_window.as_weak();
        main_window.on_settings_category_selected(move |_category| {
            if let Some(window) = window_weak.upgrade() {
                window.invoke_config_retrieve_settings();
            }
        });
    }

    // Set up menu-edit-preferences callback
    let window_weak = main_window.as_weak();
    let controller_clone = settings_controller.clone();
    main_window.on_menu_edit_preferences(move || {
        let ui_settings = controller_clone.get_settings_for_ui(Some(SettingsCategory::Controller));

        let mut settings_items = Vec::new();
        for item in ui_settings {
            let options: Vec<slint::SharedString> = item.options.iter().map(|s| s.into()).collect();

            settings_items.push(slint_generatedMainWindow::SettingItem {
                id: item.id.into(),
                name: item.name.into(),
                value: item.value.into(),
                value_type: item.value_type.into(),
                category: item.category.into(),
                description: item.description.into(),
                options: slint::ModelRc::from(Rc::new(slint::VecModel::from(options))),
                current_index: item.current_index,
            });
        }

        if let Some(window) = window_weak.upgrade() {
            let model = std::rc::Rc::new(slint::VecModel::from(settings_items));
            window.set_current_settings(slint::ModelRc::new(model));
            window.set_settings_category("controller".into());
            window.set_connection_status(slint::SharedString::from("Preferences dialog opened"));
        }
    });

    // Set up settings-category-selected callback (Duplicate? No, this one updates the model directly)
    // The previous one invoked invoke_config_retrieve_settings.
    // Let's keep both logic but maybe merge them if possible.
    // The previous one (lines 686-692 in main.rs) calls invoke_config_retrieve_settings.
    // The one at 2166 calls get_settings_for_ui and updates the model.
    // invoke_config_retrieve_settings triggers on_config_retrieve_settings (lines 608-640), which does the same thing.
    // So the second one is redundant or an optimization.
    // I'll include the second one as it was in main.rs, but I need to be careful about overwriting the callback.
    // Slint callbacks can only have one handler. The last one registered wins.
    // In main.rs, line 687 registers on_settings_category_selected.
    // Then line 2169 registers it AGAIN.
    // So the first one was effectively overwritten. I will only include the second one (the one that does the work directly).
    
    let window_weak = main_window.as_weak();
    let controller_clone = settings_controller.clone();
    main_window.on_settings_category_selected(move |category_str| {
        let category = SettingsController::get_category_from_string(category_str.as_str());
        let ui_settings = controller_clone.get_settings_for_ui(Some(category));

        let mut settings_items = Vec::new();
        for item in ui_settings {
            let options: Vec<slint::SharedString> = item.options.iter().map(|s| s.into()).collect();

            settings_items.push(slint_generatedMainWindow::SettingItem {
                id: item.id.into(),
                name: item.name.into(),
                value: item.value.into(),
                value_type: item.value_type.into(),
                category: item.category.into(),
                description: item.description.into(),
                options: slint::ModelRc::from(Rc::new(slint::VecModel::from(options))),
                current_index: item.current_index,
            });
        }

        if let Some(window) = window_weak.upgrade() {
            let model = std::rc::Rc::new(slint::VecModel::from(settings_items));
            window.set_current_settings(slint::ModelRc::new(model));
        }
    });

    // Set up menu-settings-save callback (Again, duplicate?)
    // Line 647 registers on_menu_settings_save.
    // Line 2198 registers on_menu_settings_save.
    // The second one updates connection status. The first one logs warnings.
    // I should combine them or use the last one.
    // I'll use the second one but add the logging from the first one.
    
    let window_weak = main_window.as_weak();
    let controller_clone = settings_controller.clone();
    main_window.on_menu_settings_save(move || match controller_clone.save() {
        Ok(_) => {
            warn!("Settings saved to file");
            if let Some(window) = window_weak.upgrade() {
                window.set_connection_status(slint::SharedString::from("Settings saved"));
            }
        }
        Err(e) => {
            warn!("Failed to save settings: {}", e);
            if let Some(window) = window_weak.upgrade() {
                window.set_connection_status(slint::SharedString::from(format!(
                    "Error saving settings: {}",
                    e
                )));
            }
        }
    });

    // Set up menu-settings-cancel callback
    let window_weak = main_window.as_weak();
    main_window.on_menu_settings_cancel(move || {
        if let Some(window) = window_weak.upgrade() {
            window.set_connection_status(slint::SharedString::from("Settings dialog closed"));
        }
    });

    // Set up menu-settings-restore-defaults callback (Duplicate?)
    // Line 665 registers on_menu_settings_restore_defaults.
    // Line 2225 registers on_menu_settings_restore_defaults.
    // The second one updates connection status. The first one invokes invoke_config_retrieve_settings.
    // I'll combine them.
    
    let window_weak = main_window.as_weak();
    let controller_clone = settings_controller.clone();
    main_window.on_menu_settings_restore_defaults(move || {
        controller_clone.restore_defaults();

        if let Some(window) = window_weak.upgrade() {
            window.invoke_config_retrieve_settings();
            window
                .set_connection_status(slint::SharedString::from("Settings restored to defaults"));
        }
    });

    // Set up browse-path callback
    let window_weak = main_window.as_weak();
    let controller_clone = settings_controller.clone();
    main_window.on_browse_path(move |id| {
        use rfd::FileDialog;
        if let Some(window) = window_weak.upgrade() {
            if let Some(path) = crate::platform::pick_folder_with_parent(FileDialog::new(), window.window()) {
                let path_str = path.to_string_lossy().to_string();
                controller_clone.update_setting(&id, &path_str);
                
                // Refresh UI
                window.invoke_config_retrieve_settings();
            }
        }
    });
}
