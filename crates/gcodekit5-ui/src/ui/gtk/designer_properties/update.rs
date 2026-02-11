//! Selection update and focus tracking for the properties panel.

use super::*;

impl PropertiesPanel {
    pub(crate) fn set_entry_text_if_changed(
        &self,
        entry: &Entry,
        new_value: f32,
        system: gcodekit5_core::units::MeasurementSystem,
    ) {
        if let Ok(current_parsed) = units::parse_length(&entry.text(), system) {
            if (current_parsed as f64 - new_value as f64).abs() > 1e-6 {
                entry.set_text(&units::format_length(new_value, system));
            }
        } else {
            entry.set_text(&units::format_length(new_value, system));
        }
    }

    /// Update the panel from the current selection.
    pub fn update_from_selection(&self) {
        // Don't update if any widget has focus (user is editing)
        if *self.has_focus.borrow() {
            return;
        }

        // Get current measurement system
        let system = self.settings.borrow().config().ui.measurement_system;
        let unit_label = units::get_unit_label(system);

        // Update unit labels
        self.x_unit_label.set_text(unit_label);
        self.y_unit_label.set_text(unit_label);
        self.width_unit_label.set_text(unit_label);
        self.height_unit_label.set_text(unit_label);
        self.radius_unit_label.set_text(unit_label);
        self.font_size_unit_label.set_text("pt");
        self.depth_unit_label.set_text(unit_label);
        self.step_down_unit_label.set_text(unit_label);
        self.step_in_unit_label.set_text(unit_label);
        self.offset_unit_label.set_text(unit_label);
        self.fillet_unit_label.set_text(unit_label);
        self.chamfer_unit_label.set_text(unit_label);

        // Extract data first to avoid holding the borrow while updating widgets
        let selection_data = {
            let designer_state = self.state.borrow();
            let selected: Vec<_> = designer_state
                .canvas
                .shapes()
                .filter(|s| s.selected)
                .collect();

            if selected.is_empty() {
                None
            } else if selected.len() == 1 {
                // Single selection - show all properties
                let obj = &selected[0];
                let any_not_text = !matches!(obj.shape, Shape::Text(_));
                Some((
                    vec![obj.id],
                    Some(obj.shape.clone()),
                    obj.operation_type,
                    obj.pocket_depth,
                    obj.step_down,
                    obj.step_in,
                    obj.ramp_angle,
                    obj.pocket_strategy,
                    obj.raster_fill_ratio,
                    obj.offset,
                    obj.fillet,
                    obj.chamfer,
                    any_not_text,
                    obj.lock_aspect_ratio,
                ))
            } else {
                // Multiple selection - only show CAM properties (use first shape's values)
                let obj = &selected[0];
                let any_not_text = selected.iter().any(|s| !matches!(s.shape, Shape::Text(_)));
                Some((
                    selected.iter().map(|s| s.id).collect(),
                    None, // No shape data for multi-selection
                    obj.operation_type,
                    obj.pocket_depth,
                    obj.step_down,
                    obj.step_in,
                    obj.ramp_angle,
                    obj.pocket_strategy,
                    obj.raster_fill_ratio,
                    obj.offset,
                    obj.fillet,
                    obj.chamfer,
                    any_not_text,
                    false, // Multi-selection: don't lock aspect ratio
                ))
            }
        };

        if let Some((
            ids,
            shape_opt,
            op_type,
            depth,
            step_down,
            step_in,
            ramp_angle,
            strategy,
            raster_fill,
            offset,
            fillet,
            chamfer,
            any_not_text,
            lock_aspect,
        )) = selection_data
        {
            // Set flag to prevent feedback loop during updates
            *self.updating.borrow_mut() = true;

            // Update header with shape ID(s)
            if ids.len() == 1 {
                self.header
                    .set_text(&format!("{} [{}]", t!("Properties"), ids[0]));
            } else {
                self.header.set_text(&format!(
                    "{} [{} {}]",
                    t!("Properties"),
                    ids.len(),
                    t!("shapes")
                ));
            }

            // Show/hide appropriate sections
            self.empty_label.set_visible(false);
            self.cam_frame.set_visible(true);
            self.ops_frame.set_visible(any_not_text);

            if let Some(shape) = shape_opt {
                // Single selection - show shape-specific properties
                self.pos_frame.set_visible(true);
                self.size_frame.set_visible(true);
                self.rot_frame.set_visible(true);

                // Update position and size using bounding box
                let (min_x, min_y, max_x, max_y) = shape.bounds();
                self.set_entry_text_if_changed(&self.pos_x_entry, min_x as f32, system);
                self.set_entry_text_if_changed(&self.pos_y_entry, min_y as f32, system);
                self.set_entry_text_if_changed(&self.width_entry, (max_x - min_x) as f32, system);
                self.set_entry_text_if_changed(&self.height_entry, (max_y - min_y) as f32, system);
                self.lock_aspect_ratio.set_active(lock_aspect);

                // Shape-specific properties
                match &shape {
                    Shape::Rectangle(r) => {
                        self.corner_frame.set_visible(true);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.set_entry_text_if_changed(
                            &self.corner_radius_entry,
                            r.corner_radius as f32,
                            system,
                        );
                        self.is_slot_check.set_active(r.is_slot);
                        self.rotation_entry.set_text(&format!("{:.1}", r.rotation));
                    }
                    Shape::Circle(_) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.rotation_entry.set_text("0.0");
                    }
                    Shape::Ellipse(e) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.rotation_entry
                            .set_text(&format!("{:.1}", e.rotation.to_degrees()));
                    }
                    Shape::Text(t) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(true);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.text_entry.set_text(&t.text);
                        self.font_size_entry
                            .set_text(&format_font_points(t.font_size));
                        self.font_bold_check.set_active(t.bold);
                        self.font_italic_check.set_active(t.italic);

                        // Set font family in dropdown
                        let Some(model) =
                            self.font_family_combo.model().and_downcast::<StringList>()
                        else {
                            return;
                        };
                        for i in 0..model.n_items() {
                            if let Some(item) = model.string(i) {
                                if item == t.font_family {
                                    self.font_family_combo.set_selected(i);
                                    break;
                                }
                            }
                        }
                        self.rotation_entry.set_text("0.0");
                    }
                    Shape::Polygon(p) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(true);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.sides_entry.set_text(&p.sides.to_string());
                        self.rotation_entry
                            .set_text(&format!("{:.1}", p.rotation.to_degrees()));
                    }
                    Shape::Gear(g) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(true);
                        self.sprocket_frame.set_visible(false);
                        self.gear_module_entry.set_text(&format!("{:.2}", g.module));
                        self.gear_teeth_entry.set_text(&g.teeth.to_string());
                        self.gear_pressure_angle_entry
                            .set_text(&format!("{:.1}", g.pressure_angle.to_degrees()));
                        self.rotation_entry.set_text("0.0");
                    }
                    Shape::Sprocket(s) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(true);
                        self.sprocket_pitch_entry
                            .set_text(&format!("{:.2}", s.pitch));
                        self.sprocket_teeth_entry.set_text(&s.teeth.to_string());
                        self.sprocket_roller_diameter_entry
                            .set_text(&format!("{:.2}", s.roller_diameter));
                        self.rotation_entry.set_text("0.0");
                    }
                    _ => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.rotation_entry.set_text("0.0");
                    }
                }
            } else {
                // Multi-selection - hide shape-specific properties, show common props
                self.pos_frame.set_visible(true);
                self.size_frame.set_visible(true);
                self.rot_frame.set_visible(false);
                self.corner_frame.set_visible(false);
                self.text_frame.set_visible(false);
                self.polygon_frame.set_visible(false);
                self.gear_frame.set_visible(false);
                self.sprocket_frame.set_visible(false);

                // Calculate bounding box of all selected shapes
                let designer_state = self.state.borrow();
                let mut min_x = f64::MAX;
                let mut min_y = f64::MAX;
                let mut max_x = f64::MIN;
                let mut max_y = f64::MIN;

                for shape in designer_state.canvas.shapes().filter(|s| s.selected) {
                    let (x1, y1, x2, y2) = shape.shape.bounds();
                    min_x = min_x.min(x1);
                    min_y = min_y.min(y1);
                    max_x = max_x.max(x2);
                    max_y = max_y.max(y2);
                }

                self.set_entry_text_if_changed(&self.pos_x_entry, min_x as f32, system);
                self.set_entry_text_if_changed(&self.pos_y_entry, min_y as f32, system);
                self.set_entry_text_if_changed(&self.width_entry, (max_x - min_x) as f32, system);
                self.set_entry_text_if_changed(&self.height_entry, (max_y - min_y) as f32, system);
            }

            // Update CAM properties (common to all shapes)
            self.op_type_combo
                .set_selected(if op_type == OperationType::Pocket {
                    1
                } else {
                    0
                });
            self.set_entry_text_if_changed(&self.depth_entry, depth as f32, system);
            self.set_entry_text_if_changed(&self.step_down_entry, step_down, system);
            self.set_entry_text_if_changed(&self.step_in_entry, step_in, system);
            self.ramp_angle_entry
                .set_text(&format!("{:.1}", ramp_angle));

            let strategy_index = match strategy {
                PocketStrategy::Raster { .. } => 0,
                PocketStrategy::ContourParallel => 1,
                PocketStrategy::Adaptive => 2,
            };
            self.strategy_combo.set_selected(strategy_index);
            self.raster_fill_entry
                .set_text(&format!("{:.0}", raster_fill * 100.0));

            // Update geometry ops values
            self.offset_entry.set_text(&format!("{:.2}", offset));
            self.fillet_entry.set_text(&format!("{:.2}", fillet));
            self.chamfer_entry.set_text(&format!("{:.2}", chamfer));

            // Enable/disable pocket-specific controls
            let is_pocket = op_type == OperationType::Pocket;
            self.strategy_combo.set_sensitive(is_pocket);
            self.step_in_entry.set_sensitive(is_pocket);
            self.raster_fill_entry.set_sensitive(is_pocket);

            *self.updating.borrow_mut() = false;
        } else {
            // Nothing selected - show empty state
            self.empty_label.set_visible(true);
            self.pos_frame.set_visible(false);
            self.size_frame.set_visible(false);
            self.rot_frame.set_visible(false);
            self.corner_frame.set_visible(false);
            self.text_frame.set_visible(false);
            self.polygon_frame.set_visible(false);
            self.gear_frame.set_visible(false);
            self.sprocket_frame.set_visible(false);
            self.cam_frame.set_visible(false);
            self.ops_frame.set_visible(false);
            self.header.set_text(&t!("Properties"));

            // Clear entries
            *self.updating.borrow_mut() = true;
            self.pos_x_entry.set_text("");
            self.pos_y_entry.set_text("");
            self.width_entry.set_text("");
            self.height_entry.set_text("");
            self.rotation_entry.set_text("");
            self.corner_radius_entry.set_text("");
            self.depth_entry.set_text("");
            self.step_down_entry.set_text("");
            self.step_in_entry.set_text("");
            self.ramp_angle_entry.set_text("");

            // Disable widgets when nothing selected
            self.op_type_combo.set_sensitive(false);
            self.depth_entry.set_sensitive(false);
            self.step_down_entry.set_sensitive(false);
            self.step_in_entry.set_sensitive(false);
            self.ramp_angle_entry.set_sensitive(false);
            self.strategy_combo.set_sensitive(false);
            self.raster_fill_entry.set_sensitive(false);

            self.raster_fill_entry.set_text("");
            *self.updating.borrow_mut() = false;
        }
    }

    pub(crate) fn setup_focus_tracking(&self) {
        // Track focus for all entries to prevent updates while user is editing
        let entries = vec![
            &self.pos_x_entry,
            &self.pos_y_entry,
            &self.width_entry,
            &self.height_entry,
            &self.rotation_entry,
            &self.corner_radius_entry,
            &self.font_size_entry,
            &self.depth_entry,
            &self.step_down_entry,
            &self.step_in_entry,
            &self.ramp_angle_entry,
            &self.raster_fill_entry,
            &self.sides_entry,
            &self.gear_module_entry,
            &self.gear_teeth_entry,
            &self.gear_pressure_angle_entry,
            &self.sprocket_pitch_entry,
            &self.sprocket_teeth_entry,
            &self.sprocket_roller_diameter_entry,
            &self.offset_entry,
            &self.fillet_entry,
            &self.chamfer_entry,
        ];

        for entry in entries {
            let focus_controller = EventControllerFocus::new();
            let has_focus_enter = self.has_focus.clone();
            focus_controller.connect_enter(move |_| {
                *has_focus_enter.borrow_mut() = true;
            });

            let has_focus_leave = self.has_focus.clone();
            focus_controller.connect_leave(move |_| {
                *has_focus_leave.borrow_mut() = false;
            });

            entry.add_controller(focus_controller);
        }

        // Track focus for text entry (content)
        let focus_controller = EventControllerFocus::new();
        let has_focus_enter = self.has_focus.clone();
        focus_controller.connect_enter(move |_| {
            *has_focus_enter.borrow_mut() = true;
        });

        let has_focus_leave = self.has_focus.clone();
        focus_controller.connect_leave(move |_| {
            *has_focus_leave.borrow_mut() = false;
        });
        self.text_entry.add_controller(focus_controller);
    }

    /// Clear the focus flag - call this when user interacts with the canvas
    pub fn clear_focus(&self) {
        *self.has_focus.borrow_mut() = false;
    }
}
