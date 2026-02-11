use super::*;

impl ToolsManagerView {
    pub(crate) fn setup_splitter_persistence(self: &Rc<Self>) {
        // Restore position once the Paned has a real size.
        let view = self.clone();
        let did_init = Rc::new(RefCell::new(false));
        let did_init2 = did_init.clone();
        self.widget.connect_map(move |_| {
            if *did_init2.borrow() {
                return;
            }
            *did_init2.borrow_mut() = true;
            view.apply_initial_splitter_position();
        });

        // Persist user choice (ignore bogus early values)
        let view = self.clone();
        self.widget
            .connect_notify_local(Some("position"), move |paned, _| {
                let width = paned.width();
                if width <= 0 {
                    return;
                }

                let min_pos = 260;
                let max_35 = ((width as f64) * 0.35) as i32;
                let max_canvas = (width - 700).max(min_pos);
                let max_pos = max_35.min(max_canvas).max(min_pos);

                let mut pos = paned.position();
                if pos < min_pos {
                    return;
                }
                if pos > max_pos {
                    pos = max_pos;
                }

                view.persist_tools_sidebar_position(pos);
            });
    }

    pub(crate) fn apply_initial_splitter_position(&self) {
        let width = self.widget.width();
        if width <= 0 {
            return;
        }

        let min_pos = 260;
        let max_35 = ((width as f64) * 0.35) as i32;
        let max_canvas = (width - 700).max(min_pos);
        let max_pos = max_35.min(max_canvas).max(min_pos);

        let mut pos = {
            let p = self.settings_controller.persistence.borrow();
            p.config().ui.tools_sidebar_position.unwrap_or(0)
        };

        if pos <= 0 {
            pos = ((width as f64) * 0.25) as i32;
        }
        if pos < min_pos {
            pos = min_pos;
        }
        if pos > max_pos {
            pos = max_pos;
        }

        self.widget.set_position(pos);
    }

    pub(crate) fn setup_event_handlers(self: &Rc<Self>) {
        // Sidebar actions
        {
            let view = self.clone();
            self.new_btn.connect_clicked(move |_| {
                view.start_create_new();
            });
        }

        {
            let view = self.clone();
            self.import_zip_btn.connect_clicked(move |_| {
                view.import_gtc_zip();
            });
        }

        {
            let view = self.clone();
            self.import_json_btn.connect_clicked(move |_| {
                view.import_gtc_json();
            });
        }

        {
            let view = self.clone();
            self.export_btn.connect_clicked(move |_| {
                view.export_custom_tools();
            });
        }

        {
            let view = self.clone();
            self.reset_btn.connect_clicked(move |_| {
                view.reset_custom_tools();
            });
        }

        // Save/Cancel/Delete
        {
            let view = self.clone();
            self.save_btn.connect_clicked(move |_| {
                view.save_tool();
            });
        }

        {
            let view = self.clone();
            self.cancel_btn.connect_clicked(move |_| {
                view.cancel_edit_with_confirm();
            });
        }

        {
            let view = self.clone();
            self.delete_btn.connect_clicked(move |_| {
                view.delete_tool();
            });
        }

        // Search + filters
        {
            let view = self.clone();
            self.search_entry.connect_search_changed(move |_| {
                view.persist_ui_state();
                view.load_tools();
            });
        }

        {
            let view = self.clone();
            self.type_filter.connect_changed(move |_| {
                view.persist_ui_state();
                view.load_tools();
            });
        }

        {
            let view = self.clone();
            self.material_filter.connect_changed(move |_| {
                view.persist_ui_state();
                view.load_tools();
            });
        }

        {
            let view = self.clone();
            self.dia_min.connect_changed(move |_| {
                view.persist_ui_state();
                view.load_tools();
            });
        }

        {
            let view = self.clone();
            self.dia_max.connect_changed(move |_| {
                view.persist_ui_state();
                view.load_tools();
            });
        }

        // Selection
        {
            let view = self.clone();
            self.tools_list.connect_row_selected(move |list, row_opt| {
                let Some(row) = row_opt else {
                    return;
                };
                let tool_id = unsafe {
                    row.data::<String>(ROW_TOOL_ID_KEY)
                        .map(|p| p.as_ref().clone())
                };
                let Some(tool_id) = tool_id else {
                    return;
                };

                // Switching selection: confirm discard if dirty.
                if view.is_dirty() {
                    // If we are re-selecting the currently loaded tool (e.g. programmatically reverting),
                    // don't prompt.
                    if let Some(current) = view.selected_tool.borrow().as_ref() {
                        if current.id.0 == tool_id {
                            return;
                        }
                    }

                    let prev = view.last_selected_tool_id.borrow().clone();
                    let window_opt = view.widget.root().and_downcast::<gtk4::Window>();
                    if let Some(window) = window_opt {
                        let dialog = gtk4::MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(gtk4::MessageType::Question)
                            .buttons(gtk4::ButtonsType::YesNo)
                            .text("Discard changes?")
                            .secondary_text(
                                "You have unsaved changes. Discard them and switch tools?",
                            )
                            .build();

                        let view2 = view.clone();
                        let list = list.clone();
                        dialog.connect_response(move |d, resp| {
                            if resp == gtk4::ResponseType::Yes {
                                view2.load_tool_for_edit(&tool_id);
                                view2.persist_ui_state();
                            } else if let Some(prev_id) = prev.as_deref() {
                                view2.select_row_by_tool_id(&list, prev_id);
                            } else {
                                list.unselect_all();
                            }
                            d.close();
                        });

                        dialog.show();
                    }

                    return;
                }

                view.load_tool_for_edit(&tool_id);
                view.persist_ui_state();
            });
        }

        self.setup_form_change_tracking();

        // Tool type changes affects which geometry fields are relevant.
        {
            let view = self.clone();
            self.edit_tool_type.connect_changed(move |_| {
                view.update_type_dependent_fields();
                view.update_save_state();
            });
        }
    }

    pub(crate) fn setup_form_change_tracking(self: &Rc<Self>) {
        let track_entry = |e: &Entry, view: Rc<Self>| {
            e.connect_changed(move |_| {
                view.update_save_state();
            });
        };

        track_entry(&self.edit_id, self.clone());
        track_entry(&self.edit_name, self.clone());
        track_entry(&self.edit_diameter, self.clone());
        track_entry(&self.edit_length, self.clone());
        track_entry(&self.edit_flute_length, self.clone());
        track_entry(&self.edit_shaft_diameter, self.clone());
        track_entry(&self.edit_corner_radius, self.clone());
        track_entry(&self.edit_tip_angle, self.clone());
        track_entry(&self.edit_manufacturer, self.clone());
        track_entry(&self.edit_part_number, self.clone());

        {
            let view = self.clone();
            self.edit_flutes.connect_value_changed(move |_| {
                view.update_save_state();
            });
        }

        {
            let view = self.clone();
            self.edit_material.connect_changed(move |_| {
                view.update_save_state();
            });
        }

        {
            let view = self.clone();
            self.edit_coating.connect_changed(move |_| {
                view.update_save_state();
            });
        }

        {
            let view = self.clone();
            self.edit_shank.connect_changed(move |_| {
                view.update_save_state();
            });
        }

        {
            let view = self.clone();
            self.edit_description.buffer().connect_changed(move |_| {
                view.update_save_state();
            });
        }

        {
            let view = self.clone();
            self.edit_notes.buffer().connect_changed(move |_| {
                view.update_save_state();
            });
        }
    }

    pub(crate) fn restore_ui_state(&self) {
        let ui = {
            self.settings_controller
                .persistence
                .borrow()
                .config()
                .ui
                .clone()
        };

        if !ui.tools_manager_search.is_empty() {
            self.search_entry.set_text(&ui.tools_manager_search);
        }

        if !ui.tools_manager_type_filter.is_empty() {
            self.type_filter
                .set_active_id(Some(ui.tools_manager_type_filter.as_str()));
        }

        if !ui.tools_manager_material_filter.is_empty() {
            self.material_filter
                .set_active_id(Some(ui.tools_manager_material_filter.as_str()));
        }

        if let Some(min) = ui.tools_manager_dia_min {
            self.dia_min.set_text(&format!("{min:.3}"));
        }
        if let Some(max) = ui.tools_manager_dia_max {
            self.dia_max.set_text(&format!("{max:.3}"));
        }
        if let Some(selected) = ui.tools_manager_selected_tool {
            *self.last_selected_tool_id.borrow_mut() = Some(selected);
        }
    }
}
