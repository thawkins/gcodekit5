//! Selection update and focus tracking for the properties panel.

use super::*;
use gcodekit5_designer::designer_state::MachineMode;
use gcodekit5_designer::model::ParametricPathSource;
/// Resumen de los parámetros láser de N shapes seleccionados.
/// Cada campo es `Some(v)` si todos coinciden en `v`, `None` si difieren.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LaserSummary {
    /// true si TODOS los seleccionados tienen `use_global = true`.
    pub all_use_global: bool,
    /// true si ALGUNO tiene `use_global = false` (override).
    pub any_override: bool,
    /// Valores resueltos (aplicando el global a los que lo usan).
    pub feed_rate: Option<f64>,
    pub power_percent: Option<f64>,
    pub passes: Option<u32>,
}

pub(crate) enum LaserSelection {
    Single(gcodekit5_designer::model::LaserParams),
    Summary(LaserSummary),
}

/// Laser values of one shape after resolving global settings:
/// `(use_global, feed_rate, power_percent, passes)`.
type ResolvedLaser = (bool, f64, f64, u32);

impl LaserSummary {
    pub fn from_slice(
        params: &[gcodekit5_designer::model::LaserParams],
        global: &gcodekit5_designer::designer_state::ToolSettings,
    ) -> Self {
        let resolved: Vec<ResolvedLaser> = params
            .iter()
            .map(|p| {
                let (feed, power, passes) = if p.use_global {
                    (
                        global.feed_rate,
                        (global.spindle_speed as f64) / 10.0,
                        (global.step_down as u32).max(1),
                    )
                } else {
                    (p.feed_rate, p.power_percent, p.passes)
                };
                (p.use_global, feed, power, passes)
            })
            .collect();

        let all_use_global = resolved.iter().all(|(u, _, _, _)| *u);
        let any_override = resolved.iter().any(|(u, _, _, _)| !*u);

        let common_f64 = |f: &dyn Fn(&ResolvedLaser) -> f64| -> Option<f64> {
            let mut it = resolved.iter();
            let first = f(it.next()?);
            if it.all(|x| (f(x) - first).abs() < 1e-9) {
                Some(first)
            } else {
                None
            }
        };
        let common_u32 = |f: &dyn Fn(&ResolvedLaser) -> u32| -> Option<u32> {
            let mut it = resolved.iter();
            let first = f(it.next()?);
            if it.all(|x| f(x) == first) {
                Some(first)
            } else {
                None
            }
        };

        Self {
            all_use_global,
            any_override,
            feed_rate: common_f64(&|x| x.1),
            power_percent: common_f64(&|x| x.2),
            passes: common_u32(&|x| x.3),
        }
    }
}

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
    ///
    /// Does nothing while the user is editing one of the panel's entries.
    pub fn update_from_selection(&self) {
        self.refresh_from_selection(false);
    }

    /// Switch the measurement system used to display and parse lengths, and
    /// refresh the panel so every length field shows the new units.
    ///
    /// The refresh runs even while an entry has focus: otherwise its text would
    /// still be in the old units but be parsed in the new ones when committed.
    ///
    /// # Arguments
    /// * `system` - The newly selected measurement system.
    pub fn set_measurement_system(&self, system: units::MeasurementSystem) {
        if self.measurement_system.get() == system {
            return;
        }
        tracing::debug!(
            ?system,
            "Designer properties panel switching measurement system"
        );
        self.measurement_system.set(system);
        self.refresh_from_selection(true);
    }

    /// Refresh the panel from the current selection.
    ///
    /// # Arguments
    /// * `force` - Refresh even while the user is editing an entry.
    fn refresh_from_selection(&self, force: bool) {
        // Do not rewrite entries while the user is typing; this preserves caret
        // position and avoids forced formatting mid-edit.
        if !force && *self.has_focus.borrow() {
            return;
        }

        // The panel's measurement system is the single source for both display
        // and parsing (see `set_measurement_system`).
        let system = self.measurement_system.get();
        let unit_label = units::get_unit_label(system);

        // Update unit labels
        self.x_unit_label.set_text(unit_label);
        self.y_unit_label.set_text(unit_label);
        self.z_unit_label.set_text(unit_label);
        self.width_unit_label.set_text(unit_label);
        self.height_unit_label.set_text(unit_label);
        self.radius_unit_label.set_text(unit_label);
        self.font_size_unit_label.set_text("pt");
        self.step_down_unit_label.set_text(unit_label);
        self.step_in_unit_label.set_text(unit_label);
        self.offset_unit_label.set_text(unit_label);
        self.chamfer_unit_label.set_text(unit_label);

        // Extract data first to avoid holding the borrow while updating widgets
        let selection_data = {
            let designer_state = self.state.borrow();

            let selected: Vec<_> = designer_state
                .canvas
                .shapes()
                .filter(|s| s.selected)
                .collect();

            let data = if selected.is_empty() {
                None
            } else if selected.len() == 1 {
                let obj = &selected[0];
                let any_not_text = !matches!(obj.shape, Shape::Text(_));

                let laser_params_opt = match &obj.shape {
                    Shape::Rectangle(r) => Some(r.laser_params),
                    Shape::Circle(c) => Some(c.laser_params),
                    Shape::Ellipse(e) => Some(e.laser_params),
                    Shape::Line(l) => Some(l.laser_params),
                    Shape::Path(p) => Some(p.laser_params),
                    Shape::Polygon(p) => Some(p.laser_params),
                    Shape::Triangle(t) => Some(t.laser_params),
                    Shape::Text(t) => Some(t.laser_params),
                    Shape::Gear(g) => Some(g.laser_params),
                    Shape::Sprocket(s) => Some(s.laser_params),
                    Shape::RasterImage(_) => None,
                };

                Some((
                    vec![obj.id],
                    Some(obj.shape.clone()),
                    obj.use_custom_values,
                    obj.operation_type,
                    obj.start_depth,
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
                    laser_params_opt.map(LaserSelection::Single),
                ))
            } else {
                let obj = &selected[0];
                let any_not_text = selected.iter().any(|s| !matches!(s.shape, Shape::Text(_)));

                // --- NUEVO: calcular resumen láser ---
                let any_raster = selected
                    .iter()
                    .any(|s| matches!(s.shape, Shape::RasterImage(_)));

                let all_laser: Vec<gcodekit5_designer::model::LaserParams> = selected
                    .iter()
                    .filter_map(|s| match &s.shape {
                        Shape::Rectangle(r) => Some(r.laser_params),
                        Shape::Circle(c) => Some(c.laser_params),
                        Shape::Ellipse(e) => Some(e.laser_params),
                        Shape::Line(l) => Some(l.laser_params),
                        Shape::Path(p) => Some(p.laser_params),
                        Shape::Polygon(p) => Some(p.laser_params),
                        Shape::Triangle(t) => Some(t.laser_params),
                        Shape::Text(t) => Some(t.laser_params),
                        Shape::Gear(g) => Some(g.laser_params),
                        Shape::Sprocket(s) => Some(s.laser_params),
                        Shape::RasterImage(_) => None,
                    })
                    .collect();

                let laser_summary = if all_laser.is_empty() || any_raster {
                    None
                } else {
                    let global = self.state.borrow().tool_settings.clone();
                    Some(LaserSummary::from_slice(&all_laser, &global))
                };

                Some((
                    selected.iter().map(|s| s.id).collect(),
                    None,
                    obj.use_custom_values,
                    obj.operation_type,
                    obj.start_depth,
                    obj.step_down,
                    obj.step_in,
                    obj.ramp_angle,
                    obj.pocket_strategy,
                    obj.raster_fill_ratio,
                    obj.offset,
                    obj.fillet,
                    obj.chamfer,
                    any_not_text,
                    false,
                    laser_summary.map(LaserSelection::Summary),
                ))
            };
            data
        };

        if let Some((
            ids,
            shape_opt,
            use_custom_values,
            op_type,
            start_depth,
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
            laser_params_opt,
        )) = selection_data
        {
            let (w, h, rot) = if let Some(shape) = &shape_opt {
                match shape {
                    Shape::Rectangle(r) => (r.width, r.height, r.rotation),
                    Shape::Triangle(t) => (t.width, t.height, t.rotation),
                    Shape::Ellipse(e) => (e.rx * 2.0, e.ry * 2.0, e.rotation),
                    Shape::Circle(c) => (c.radius * 2.0, c.radius * 2.0, 0.0),
                    Shape::Polygon(p) => (p.radius * 2.0, p.radius * 2.0, p.rotation),
                    Shape::Path(p) => {
                        let (x1, y1, x2, y2) = p.bounds();
                        (x2 - x1, y2 - y1, p.rotation)
                    }
                    Shape::RasterImage(r) => (r.width_mm, r.height_mm, r.rotation),
                    _ => (0.0, 0.0, 0.0),
                }
            } else {
                (0.0, 0.0, 0.0)
            };
            // Set flag to prevent feedback loop during updates. This must be set before
            // update_dimensions_ui, which toggles the lock-aspect checkbox.
            *self.updating.borrow_mut() = true;

            self.update_dimensions_ui(&self.width_entry, &self.height_entry, w, h, rot, system);

            match laser_params_opt {
                Some(LaserSelection::Single(params)) => {
                    // Leer en tiempo real el estado global de la máquina
                    let global_settings = self.state.borrow().tool_settings.clone();

                    // Calcular los valores a mostrar
                    let display_feed = if params.use_global {
                        global_settings.feed_rate
                    } else {
                        params.feed_rate
                    };

                    let display_power = if params.use_global {
                        (global_settings.spindle_speed as f64) / 10.0
                    } else {
                        params.power_percent
                    };

                    let display_passes = if params.use_global {
                        (global_settings.step_down as u32).max(1)
                    } else {
                        params.passes
                    };

                    // Solo actualizar si el valor ha cambiado y NO hay foco
                    if !*self.has_focus.borrow() {
                        self.laser_feed_rate_entry
                            .set_text(&display_feed.to_string());
                        self.laser_power_entry.set_text(&display_power.to_string());
                        self.laser_passes_entry
                            .set_text(&display_passes.to_string());
                        self.laser_use_global_check.set_active(params.use_global);

                        self.laser_feed_rate_entry.set_sensitive(!params.use_global);
                        self.laser_power_entry.set_sensitive(!params.use_global);
                        self.laser_passes_entry.set_sensitive(!params.use_global);
                    }
                    self.laser_use_global_check.set_active(params.use_global);

                    // Deshabilitar campos si es global
                    self.laser_feed_rate_entry.set_sensitive(!params.use_global);
                    self.laser_power_entry.set_sensitive(!params.use_global);
                    self.laser_passes_entry.set_sensitive(!params.use_global);

                    let display_passes = if params.use_global {
                        (global_settings.step_down as u32).max(1)
                    } else {
                        params.passes
                    };

                    // Asignar feed_rate a la caja de texto
                    if let Ok(current) = self.laser_feed_rate_entry.text().parse::<f64>() {
                        if (current - display_feed).abs() > 1e-6 {
                            self.laser_feed_rate_entry
                                .set_text(&display_feed.to_string());
                        }
                    } else {
                        self.laser_feed_rate_entry
                            .set_text(&display_feed.to_string());
                    }

                    // Asignar power a la caja de texto
                    if let Ok(current) = self.laser_power_entry.text().parse::<f64>() {
                        if (current - display_power).abs() > 1e-6 {
                            self.laser_power_entry.set_text(&display_power.to_string());
                        }
                    } else {
                        self.laser_power_entry.set_text(&display_power.to_string());
                    }

                    // Asignar passes a la caja de texto
                    if let Ok(current) = self.laser_passes_entry.text().parse::<u32>() {
                        if current != display_passes {
                            self.laser_passes_entry
                                .set_text(&display_passes.to_string());
                        }
                    } else {
                        self.laser_passes_entry
                            .set_text(&display_passes.to_string());
                    }

                    // Forzar al Checkbox visual a ponerse en su sitio
                    if self.laser_use_global_check.is_active() != params.use_global {
                        self.laser_use_global_check.set_active(params.use_global);
                    }

                    // Deshabilitar los campos si es global
                    self.laser_feed_rate_entry.set_sensitive(!params.use_global);
                    self.laser_power_entry.set_sensitive(!params.use_global);
                    self.laser_passes_entry.set_sensitive(!params.use_global);
                }
                Some(LaserSelection::Summary(sum)) => {
                    let enabled = sum.any_override;
                    self.laser_feed_rate_entry.set_sensitive(enabled);
                    self.laser_power_entry.set_sensitive(enabled);
                    self.laser_passes_entry.set_sensitive(enabled);

                    self.laser_use_global_check.set_active(sum.all_use_global);

                    match sum.feed_rate {
                        Some(v) => self.laser_feed_rate_entry.set_text(&v.to_string()),
                        None => self.laser_feed_rate_entry.set_text("---"),
                    }
                    match sum.power_percent {
                        Some(v) => self.laser_power_entry.set_text(&v.to_string()),
                        None => self.laser_power_entry.set_text("---"),
                    }
                    match sum.passes {
                        Some(v) => self.laser_passes_entry.set_text(&v.to_string()),
                        None => self.laser_passes_entry.set_text("---"),
                    }
                }
                None => {}
            }

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

            // Obtener el modo de máquina
            let machine_mode = self.state.borrow().machine_mode();
            let is_laser = machine_mode == MachineMode::Laser2D;

            let is_raster = matches!(shape_opt, Some(Shape::RasterImage(_)));
            // CAM frame: only relevant for CNC 3D and non-raster shapes.
            self.cam_frame.set_visible(!is_laser && !is_raster);
            self.ops_frame.set_visible(!is_raster && any_not_text);

            // Z position is only editable in CNC/3D mode.
            self.pos_z_entry.set_sensitive(!is_laser);
            self.z_label.set_sensitive(!is_laser);

            // El panel de propiedades láser solo se muestra en modo láser
            self.laser_override_frame
                .set_visible(is_laser && !is_raster);

            if let Some(shape) = shape_opt {
                // Single selection - show shape-specific properties
                self.rotation_entry.set_tooltip_text(Some(&t!(
                    "Rotation angle (degrees). Applied on Enter or when leaving the field."
                )));
                self.pos_frame.set_visible(true);
                self.size_frame.set_visible(true);
                self.rot_frame.set_visible(true);
                // Reset frames that only some shape kinds show; the match below
                // re-enables the relevant ones.
                self.image_engraving_frame.set_visible(false);
                self.path_frame.set_visible(false);
                // --- Object Center
                let (x1, y1, x2, y2) = shape.bounds();
                let center_x = (x1 + x2) / 2.0;
                let center_y = (y1 + y2) / 2.0;

                self.set_entry_text_if_changed(&self.pos_x_entry, center_x as f32, system);
                self.set_entry_text_if_changed(&self.pos_y_entry, center_y as f32, system);
                if is_laser {
                    self.pos_z_entry.set_text("0");
                } else {
                    self.set_entry_text_if_changed(&self.pos_z_entry, start_depth as f32, system);
                }

                // Don't overwrite the forced lock state applied by update_dimensions_ui
                // for rotated objects, otherwise the checkbox reverts to unchecked while
                // the entries stay disabled.
                if rot.abs() <= 0.01 {
                    self.lock_aspect_ratio.set_active(lock_aspect);
                }

                // Shape-specific properties
                match &shape {
                    Shape::RasterImage(r) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.timing_pulley_frame.set_visible(false);
                        self.ops_frame.set_visible(false);
                        self.cam_frame.set_visible(false);
                        self.image_engraving_frame.set_visible(true);

                        self.image_feed_rate_entry
                            .set_text(&r.feed_rate.to_string());
                        self.image_travel_rate_entry
                            .set_text(&r.travel_rate.to_string());
                        self.image_min_power_entry
                            .set_text(&r.min_power.to_string());
                        self.image_max_power_entry
                            .set_text(&r.max_power.to_string());
                        self.image_ppi_entry.set_text(&r.ppi.to_string());
                        self.image_scan_direction_combo
                            .set_active_id(Some(&r.scan_direction));
                        self.image_bidirectional_check.set_active(r.bidirectional);
                        self.image_invert_check.set_active(r.invert);
                        //                        self.image_dithering_combo.set_active_id(Some(&r.dithering));
                        //Force lock aspect ratio and disable the button so it cannot be changed
                        //                        if !*self.has_focus.borrow() {
                        //                            self.image_halftone_threshold_entry
                        //                                .set_text(&r.halftone_threshold.to_string());
                        //                        }
                        self.lock_aspect_ratio.set_active(true);
                        self.lock_aspect_ratio.set_sensitive(false);
                        // No rotation
                        self.rotation_entry.set_text("0.0");
                        self.rot_frame.set_visible(false);
                        self.laser_override_frame.set_visible(false);
                    }

                    Shape::Rectangle(r) => {
                        // Disable input X and Y
                        self.pos_x_entry.set_sensitive(true);
                        self.pos_y_entry.set_sensitive(true);
                        self.corner_frame.set_visible(true);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.timing_pulley_frame.set_visible(false);
                        self.path_frame.set_visible(false);
                        self.set_entry_text_if_changed(
                            &self.corner_radius_entry,
                            r.corner_radius as f32,
                            system,
                        );
                        self.is_slot_check.set_sensitive(true);
                        self.is_slot_check.set_active(r.is_slot);
                        self.rotation_entry.set_text(&format!("{:.1}", r.rotation));
                        self.rotation_entry.set_sensitive(true);
                        self.set_entry_text_if_changed(&self.width_entry, r.width as f32, system);
                        self.set_entry_text_if_changed(&self.height_entry, r.height as f32, system);
                    }

                    Shape::Circle(_) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.timing_pulley_frame.set_visible(false);
                        self.path_frame.set_visible(false);
                        self.rotation_entry.set_text("0.0");
                        self.rot_frame.set_visible(false);
                    }

                    Shape::Ellipse(e) => {
                        let has_rotation = e.rotation.abs() > f64::EPSILON;
                        self.rotation_entry.set_text(&format!("{:.1}", e.rotation));
                        if has_rotation {
                            self.width_entry.set_sensitive(false);
                            self.height_entry.set_sensitive(false);
                            self.width_entry
                                .set_text(&units::format_length((e.rx * 2.0) as f32, system));
                            self.height_entry
                                .set_text(&units::format_length((e.ry * 2.0) as f32, system));
                        } else {
                            self.width_entry.set_sensitive(true);
                            self.height_entry.set_sensitive(true);
                            self.set_entry_text_if_changed(
                                &self.width_entry,
                                (e.rx * 2.0) as f32,
                                system,
                            );
                            self.set_entry_text_if_changed(
                                &self.height_entry,
                                (e.ry * 2.0) as f32,
                                system,
                            );
                        }
                        self.rotation_entry.set_sensitive(true);
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.timing_pulley_frame.set_visible(false);
                    }

                    Shape::Text(text_shape) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(true);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.timing_pulley_frame.set_visible(false);
                        self.path_frame.set_visible(false);
                        self.text_entry.set_text(&text_shape.text);
                        self.font_size_entry
                            .set_text(&format_font_points(text_shape.font_size));
                        self.font_bold_check.set_active(text_shape.bold);
                        self.font_italic_check.set_active(text_shape.italic);

                        // Set font family in dropdown
                        // Note: no early return here, `updating` must be reset below.
                        if let Some(model) =
                            self.font_family_combo.model().and_downcast::<StringList>()
                        {
                            for i in 0..model.n_items() {
                                if let Some(item) = model.string(i) {
                                    if item == text_shape.font_family {
                                        self.font_family_combo.set_selected(i);
                                        break;
                                    }
                                }
                            }
                        }
                        self.rotation_entry.set_sensitive(true);
                    }
                    Shape::Polygon(p) => {
                        let has_rotation = p.rotation.abs() > f64::EPSILON;
                        self.rotation_entry.set_text(&format!("{:.1}", p.rotation));

                        if has_rotation {
                            self.sides_entry.set_sensitive(false);
                        } else {
                            self.sides_entry.set_sensitive(true);
                        }

                        self.size_frame.set_visible(true);
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(true);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.timing_pulley_frame.set_visible(false);
                        self.path_frame.set_visible(false);
                        self.sides_entry.set_text(&p.sides.to_string());
                        self.rotation_entry.set_text(&format!("{:.1}", p.rotation));
                        self.rotation_entry.set_sensitive(true);
                        self.height_entry.set_sensitive(false);
                    }
                    Shape::Gear(g) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(true);
                        self.sprocket_frame.set_visible(false);
                        self.timing_pulley_frame.set_visible(false);
                        self.path_frame.set_visible(false);
                        self.gear_module_entry.set_text(&format!("{:.2}", g.module));
                        self.gear_teeth_entry.set_text(&g.teeth.to_string());
                        // Ahora pressure_angle_deg ya está en grados
                        self.gear_pressure_angle_entry
                            .set_text(&format!("{:.1}", g.pressure_angle_deg));
                        self.rotation_entry.set_text("0.0");
                        self.rot_frame.set_visible(false);
                    }
                    Shape::Sprocket(s) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(true);
                        self.timing_pulley_frame.set_visible(false);
                        self.path_frame.set_visible(false);
                        self.sprocket_pitch_entry
                            .set_text(&format!("{:.2}", s.pitch));
                        self.sprocket_teeth_entry.set_text(&s.teeth.to_string());
                        self.sprocket_roller_diameter_entry
                            .set_text(&format!("{:.2}", s.roller_diameter));
                        self.rotation_entry.set_text("0.0");
                        self.rot_frame.set_visible(false);
                    }

                    Shape::Path(p) => {
                        let has_rotation = p.rotation.abs() > f64::EPSILON;
                        self.rotation_entry.set_text(&format!("{:.1}", p.rotation));

                        let (x1, y1, x2, y2) = shape.bounds();
                        self.set_entry_text_if_changed(&self.width_entry, (x2 - x1) as f32, system);
                        self.set_entry_text_if_changed(
                            &self.height_entry,
                            (y2 - y1) as f32,
                            system,
                        );
                        self.corner_frame.set_visible(true);
                        self.set_entry_text_if_changed(
                            &self.corner_radius_entry,
                            fillet as f32,
                            system,
                        );
                        self.is_slot_check.set_active(false);
                        self.is_slot_check.set_sensitive(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        let timing_params = match &p.parametric_source {
                            Some(ParametricPathSource::TimingPulley {
                                pitch,
                                teeth,
                                belt_width,
                                hole_radius,
                            }) => Some((*pitch, *teeth, *belt_width, *hole_radius)),
                            _ => None,
                        };
                        self.timing_pulley_frame
                            .set_visible(timing_params.is_some());
                        self.path_frame.set_visible(true);
                        self.path_closed_check.set_active(p.closed);

                        if let Some((pitch, teeth, belt_width, hole_radius)) = timing_params {
                            self.timing_pulley_pitch_entry
                                .set_text(&format!("{:.2}", pitch));
                            self.timing_pulley_teeth_entry.set_text(&teeth.to_string());
                            self.timing_pulley_belt_width_entry
                                .set_text(&format!("{:.2}", belt_width));
                            self.timing_pulley_hole_radius_entry
                                .set_text(&format!("{:.2}", hole_radius));
                        }

                        // No edit X Y
                        self.pos_x_entry.set_sensitive(true);
                        self.pos_y_entry.set_sensitive(true);
                        self.width_entry.set_sensitive(!has_rotation);
                        self.height_entry.set_sensitive(!has_rotation);
                    }

                    Shape::Triangle(tri) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.timing_pulley_frame.set_visible(false);
                        self.path_frame.set_visible(false);
                        self.set_entry_text_if_changed(&self.width_entry, tri.width as f32, system);
                        self.set_entry_text_if_changed(
                            &self.height_entry,
                            tri.height as f32,
                            system,
                        );
                        self.rotation_entry
                            .set_text(&format!("{:.1}", tri.rotation));

                        // --- Laser Parameters
                        self.set_entry_text_if_changed(
                            &self.laser_feed_rate_entry,
                            tri.laser_params.feed_rate as f32,
                            system,
                        );
                        self.set_entry_text_if_changed(
                            &self.laser_power_entry,
                            tri.laser_params.power_percent as f32,
                            system,
                        );
                        self.set_entry_text_if_changed(
                            &self.laser_passes_entry,
                            tri.laser_params.passes as f32,
                            system,
                        );
                    }
                    Shape::Line(line) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);
                        self.gear_frame.set_visible(false);
                        self.sprocket_frame.set_visible(false);
                        self.timing_pulley_frame.set_visible(false);
                        self.path_frame.set_visible(false);

                        // Posición y rotación
                        let (x1, y1, x2, y2) = shape.bounds();
                        let center_x = (x1 + x2) / 2.0;
                        let center_y = (y1 + y2) / 2.0;
                        self.set_entry_text_if_changed(&self.pos_x_entry, center_x as f32, system);
                        self.set_entry_text_if_changed(&self.pos_y_entry, center_y as f32, system);
                        self.set_entry_text_if_changed(&self.width_entry, (x2 - x1) as f32, system);
                        self.set_entry_text_if_changed(
                            &self.height_entry,
                            (y2 - y1) as f32,
                            system,
                        );
                        self.rotation_entry
                            .set_text(&format!("{:.1}", line.rotation));
                        self.rotation_entry.set_sensitive(true);
                    }
                }
            } else {
                // Multi-selection - hide shape-specific properties, show common props
                self.pos_frame.set_visible(true);
                self.size_frame.set_visible(true);
                self.rot_frame.set_visible(true);
                self.corner_frame.set_visible(false);
                self.text_frame.set_visible(false);
                self.polygon_frame.set_visible(false);
                self.gear_frame.set_visible(false);
                self.sprocket_frame.set_visible(false);
                self.timing_pulley_frame.set_visible(false);
                self.path_frame.set_visible(false);
                self.image_engraving_frame.set_visible(false);

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

                drop(designer_state);

                // Position entries are interpreted as the selection centre by the
                // position handlers, so display the centre here too.
                let center_x = (min_x + max_x) / 2.0;
                let center_y = (min_y + max_y) / 2.0;
                self.set_entry_text_if_changed(&self.pos_x_entry, center_x as f32, system);
                self.set_entry_text_if_changed(&self.pos_y_entry, center_y as f32, system);
                if is_laser {
                    self.pos_z_entry.set_text("0");
                } else {
                    self.set_entry_text_if_changed(&self.pos_z_entry, start_depth as f32, system);
                }
                self.set_entry_text_if_changed(&self.width_entry, (max_x - min_x) as f32, system);
                self.set_entry_text_if_changed(&self.height_entry, (max_y - min_y) as f32, system);

                // For a multi-selection the rotation entry is a relative angle: the
                // committed value rotates the whole selection once, then resets to 0.
                self.rotation_entry.set_text("0.0");
                self.rotation_entry.set_sensitive(true);
                self.rotation_entry.set_tooltip_text(Some(&t!(
                    "Rotate the selection by this angle (degrees). Applied on Enter or when leaving the field."
                )));
            }

            // Update CAM properties (common to all shapes)
            let cam_enabled = !is_laser && !is_raster;
            let use_global_cam = !use_custom_values;
            self.cam_use_global_check.set_sensitive(cam_enabled);
            self.cam_use_global_check.set_active(use_global_cam);

            let cam_entries_enabled = cam_enabled && use_custom_values;
            self.op_type_combo.set_sensitive(cam_entries_enabled);
            self.step_down_entry.set_sensitive(cam_entries_enabled);
            self.ramp_angle_entry.set_sensitive(cam_entries_enabled);

            self.op_type_combo
                .set_selected(if op_type == OperationType::Pocket {
                    1
                } else {
                    0
                });

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
            self.offset_entry
                .set_text(&units::format_length(offset as f32, system));
            self.chamfer_entry
                .set_text(&units::format_length(chamfer as f32, system));

            let is_pocket = op_type == OperationType::Pocket;
            self.strategy_combo
                .set_sensitive(cam_entries_enabled && is_pocket);
            self.step_in_entry
                .set_sensitive(cam_entries_enabled && is_pocket);
            self.step_in_label
                .set_sensitive(cam_entries_enabled && is_pocket);
            self.step_in_unit_label
                .set_sensitive(cam_entries_enabled && is_pocket);
            self.raster_fill_entry
                .set_sensitive(cam_entries_enabled && is_pocket);

            // Refresh the locked aspect ratio from the newly displayed size; the
            // checkbox handler only captures it when toggled, so it would otherwise
            // keep the ratio of a previously selected shape.
            if let (Ok(cur_w), Ok(cur_h)) = (
                units::parse_length(&self.width_entry.text(), system),
                units::parse_length(&self.height_entry.text(), system),
            ) {
                if cur_h > 0.0 {
                    *self.aspect_ratio.borrow_mut() = cur_w as f64 / cur_h as f64;
                }
            }

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
            self.timing_pulley_frame.set_visible(false);
            self.path_frame.set_visible(false);
            self.cam_frame.set_visible(false);
            self.ops_frame.set_visible(false);
            self.image_engraving_frame.set_visible(false);
            self.laser_override_frame.set_visible(false);
            self.header.set_text(&t!("Properties"));

            // Clear entries
            *self.updating.borrow_mut() = true;
            self.pos_x_entry.set_text("");
            self.pos_y_entry.set_text("");
            self.pos_z_entry.set_text("");
            self.width_entry.set_text("");
            self.height_entry.set_text("");
            self.rotation_entry.set_text("");
            self.corner_radius_entry.set_text("");
            self.step_down_entry.set_text("");
            self.step_in_entry.set_text("");
            self.ramp_angle_entry.set_text("");

            // Disable widgets when nothing selected
            self.op_type_combo.set_sensitive(false);
            self.cam_use_global_check.set_sensitive(false);
            self.cam_use_global_check.set_active(false);
            self.step_down_entry.set_sensitive(false);
            self.step_in_entry.set_sensitive(false);
            self.ramp_angle_entry.set_sensitive(false);
            self.strategy_combo.set_sensitive(false);
            self.raster_fill_entry.set_sensitive(false);

            self.raster_fill_entry.set_text("");
            self.lock_aspect_ratio.set_active(false);
            *self.updating.borrow_mut() = false;
        }
    }

    pub(crate) fn setup_focus_tracking(&self) {
        // Track focus for all entries to prevent updates while user is editing
        let entries = vec![
            &self.pos_x_entry,
            &self.pos_y_entry,
            &self.pos_z_entry,
            &self.width_entry,
            &self.height_entry,
            &self.rotation_entry,
            &self.corner_radius_entry,
            &self.font_size_entry,
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
            &self.timing_pulley_pitch_entry,
            &self.timing_pulley_teeth_entry,
            &self.timing_pulley_belt_width_entry,
            &self.timing_pulley_hole_radius_entry,
            &self.offset_entry,
            &self.chamfer_entry,
            &self.image_feed_rate_entry,
            &self.image_travel_rate_entry,
            &self.image_min_power_entry,
            &self.image_max_power_entry,
            &self.image_ppi_entry,
            // ---
            &self.laser_feed_rate_entry,
            &self.laser_power_entry,
            &self.laser_passes_entry,
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

    fn update_dimensions_ui(
        &self,
        entry_w: &gtk4::Entry,
        entry_h: &gtk4::Entry,
        width: f64,
        height: f64,
        rotation: f64,
        system: gcodekit5_core::units::MeasurementSystem,
    ) {
        let has_rotation = rotation.abs() > 0.01;

        // --- NOTICE CONTROL ---
        self.rotation_warning_label.set_visible(has_rotation);

        if has_rotation {
            self.rotation_warning_label
                .set_text("⚠️ Rotated object: Dimensions locked");
            self.rotation_warning_label.set_visible(true);
            entry_w.set_sensitive(false);
            entry_h.set_sensitive(false);
            entry_w.set_text(&units::format_length(width as f32, system));
            entry_h.set_text(&units::format_length(height as f32, system));
            self.lock_aspect_ratio.set_active(true);
            self.lock_aspect_ratio.set_sensitive(false);
        } else {
            entry_w.set_sensitive(true);
            entry_h.set_sensitive(true);
            self.set_entry_text_if_changed(entry_w, width as f32, system);
            self.set_entry_text_if_changed(entry_h, height as f32, system);
            self.rotation_warning_label.set_text("");
            self.rotation_warning_label.set_visible(false);
            self.lock_aspect_ratio.set_sensitive(true);
        }
    }
}
