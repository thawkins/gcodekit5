//! G-code generation for designer state

use super::DesignerState;
use crate::canvas::DrawingObject;
use crate::designer_state::MachineMode;
use crate::model::DesignerShape;
use crate::model::LaserParams;
use crate::shapes::OperationType;
use crate::ToolpathToGcode;
use csgrs::traits::CSG;
use gcodekit5_core::Units;
use crate::gcode_gen::MachineLimits;

impl DesignerState {
    fn shape_embedded_laser_use_global(shape: &DrawingObject) -> bool {
        match &shape.shape {
            crate::model::Shape::Rectangle(s) => s.laser_params.use_global,
            crate::model::Shape::Circle(s) => s.laser_params.use_global,
            crate::model::Shape::Line(s) => s.laser_params.use_global,
            crate::model::Shape::Ellipse(s) => s.laser_params.use_global,
            crate::model::Shape::Path(s) => s.laser_params.use_global,
            crate::model::Shape::Text(s) => s.laser_params.use_global,
            crate::model::Shape::Triangle(s) => s.laser_params.use_global,
            crate::model::Shape::Polygon(s) => s.laser_params.use_global,
            crate::model::Shape::Gear(s) => s.laser_params.use_global,
            crate::model::Shape::Sprocket(s) => s.laser_params.use_global,
            crate::model::Shape::RasterImage(_) => shape.laser_params.use_global,
        }
    }

    fn apply_laser_params_to_shape(shape: &mut crate::model::Shape, params: LaserParams) {
        match shape {
            crate::model::Shape::Rectangle(s) => s.laser_params = params,
            crate::model::Shape::Circle(s) => s.laser_params = params,
            crate::model::Shape::Line(s) => s.laser_params = params,
            crate::model::Shape::Ellipse(s) => s.laser_params = params,
            crate::model::Shape::Path(s) => s.laser_params = params,
            crate::model::Shape::Text(s) => s.laser_params = params,
            crate::model::Shape::Triangle(s) => s.laser_params = params,
            crate::model::Shape::Polygon(s) => s.laser_params = params,
            crate::model::Shape::Gear(s) => s.laser_params = params,
            crate::model::Shape::Sprocket(s) => s.laser_params = params,
            crate::model::Shape::RasterImage(_) => {}
        }
    }

    /// Computes effective start depth for CNC from UI Z.
    ///
    /// In CNC mode, UI Z is treated as depth from stock top.
    /// Machine Z is absolute with table at 0 and stock top at stock_thickness,
    /// therefore: machine_z = stock_thickness - ui_depth.
    fn effective_cnc_start_depth_from_shape(&self, shape: &DrawingObject) -> f64 {
        let stock_thickness = self
            .stock_material
            .as_ref()
            .map(|s| s.thickness as f64)
            .unwrap_or(10.0);
        stock_thickness - shape.start_depth
    }

    /// Returns a safety violation summary when any object starts at/above safe Z in CNC mode.
    ///
    /// Tuple format: (safe_z, violating_count, max_object_start_z).
    pub fn safe_z_clearance_violation_summary(&self) -> Option<(f64, usize, f64)> {
        if self.machine_mode() != MachineMode::Cnc3D {
            return None;
        }

        let safe_z = self
            .stock_material
            .as_ref()
            .map(|s| s.safe_z as f64)
            .unwrap_or(10.0);

        let mut violating_count = 0usize;
        let mut max_start_z = f64::NEG_INFINITY;

        for shape in self.canvas.shape_store.iter() {
            let start_z = self.effective_cnc_start_depth_from_shape(shape);
            if start_z >= safe_z {
                violating_count += 1;
                if start_z > max_start_z {
                    max_start_z = start_z;
                }
            }
        }

        if violating_count > 0 {
            Some((safe_z, violating_count, max_start_z))
        } else {
            None
        }
    }

    /// Obtiene los parámetros láser efectivos para un objeto
    fn get_effective_laser_params(&self, shape: &DrawingObject) -> Option<LaserParams> {
        let use_global = shape.use_global_laser
            || shape.laser_params.use_global
            || Self::shape_embedded_laser_use_global(shape);

        if use_global {
            let passes = if self.machine_mode() == MachineMode::Laser2D {
                // En modo láser: step_down ES el número de pasadas (1-10)
                self.tool_settings.step_down.round().clamp(1.0, 10.0) as u32
            } else {
                // En modo CNC: convertir profundidad a número de pasadas
                (self.tool_settings.step_down / 0.1).max(1.0).round() as u32
            };

            Some(LaserParams {
                feed_rate: self.tool_settings.feed_rate,
                power_percent: (self.tool_settings.spindle_speed as f64) / 10.0,
                passes,
                use_global: true,
            })
        } else {
            let mut params = shape.laser_params;
            params.use_global = false;
            Some(params)
        }
    }

    /// Normaliza parámetros efectivos para generación CNC 3D.
    ///
    /// En CNC, cuando el objeto no usa valores custom, se deben usar los
    /// parámetros globales de herramienta para evitar inconsistencias entre UI y G-code.
    fn build_effective_cnc_shape(&self, shape: &DrawingObject) -> DrawingObject {
        let mut effective = shape.clone();
        let cam_defaults = &self.default_properties_shape;

        if !shape.use_custom_values {
            effective.operation_type = cam_defaults.operation_type;
            effective.step_in = cam_defaults.step_in;
            effective.ramp_angle = cam_defaults.ramp_angle;
            effective.pocket_strategy = cam_defaults.pocket_strategy;
            effective.raster_fill_ratio = cam_defaults.raster_fill_ratio;

            effective.step_down = if self.tool_settings.step_down > 0.0 {
                self.tool_settings.step_down as f32
            } else {
                cam_defaults.step_down
            };

            effective.laser_params.use_global = true;
            effective.laser_params.feed_rate = self.tool_settings.feed_rate;
            effective.laser_params.power_percent = (self.tool_settings.spindle_speed as f64) / 10.0;
        } else {
            if effective.laser_params.use_global {
                effective.laser_params.feed_rate = self.tool_settings.feed_rate;
                effective.laser_params.power_percent =
                    (self.tool_settings.spindle_speed as f64) / 10.0;
            }
        }

        // CNC multipass must be driven by object CAM properties. Keep only a
        // technical fallback for legacy objects with invalid/empty step-down.
        if effective.step_down <= 0.0 {
            effective.step_down = 0.1;
        }
        if effective.pocket_depth < 0.0 {
            effective.pocket_depth = 0.0;
        }

        effective
    }

    /// Generates G-code from the current design and reports whether any toolpath
    /// falls outside the machine limits.
    #[allow(clippy::collapsible_else_if)]
    pub fn generate_gcode_with_warning_info(
        &mut self,
        machine_limits: Option<MachineLimits>,
    ) -> (String, bool) {
        let mut gcode = String::new();
        // Get safe_z from stock_material, default to 10.0 if not set
        let safe_z = self
            .stock_material
            .as_ref()
            .map(|s| s.safe_z as f64)
            .unwrap_or(10.0);
        let mut gcode_gen = ToolpathToGcode::new(Units::MM, safe_z)
            .with_continuous_z_between_passes(self.tool_settings.continuous_z_between_passes);


    if let Some(limits) = machine_limits {
        gcode_gen = gcode_gen.with_machine_limits(limits);
    }

        // Use state mode:
        match self.machine_mode() {
            MachineMode::Laser2D => {
                gcode_gen = gcode_gen.with_laser_2d();
            }
            MachineMode::Cnc3D => {}
        }

        gcode_gen.num_axes = self.num_axes;
        if self.num_axes < 3 {
            // Use state mode:
            match self.machine_mode() {
                MachineMode::Laser2D => {
                    gcode_gen = gcode_gen
                        .with_laser_2d()
                        .with_curve_tolerance(0.05) // Adjust as needed (0.03 ~ 0.15)
                        .with_min_point_distance(0.15);
                }
                MachineMode::Cnc3D => {}
            }
        }
        // Store shape-to-toolpath mapping
        let mut shape_toolpaths: Vec<(DrawingObject, Vec<crate::Toolpath>, bool)> = Vec::new();

        // Obtener el ID del objeto seleccionado (si hay)
        let selected_ids = self
            .canvas
            .selection_manager
            .selected_ids(&self.canvas.shape_store);

        // Collect shape IDs in reverse draw order (front to back) for G-code generation
        let shape_ids: Vec<u64> = self.canvas.shape_store.draw_order_iter().collect();

        for shape_id in shape_ids {
            let Some(shape_obj) = self.canvas.shape_store.get(shape_id) else {
                continue;
            };

            if !selected_ids.is_empty() {
                if !selected_ids.contains(&shape_obj.id) {
                    continue; // Saltar si no está en la selección
                }
            }

            let effective_shape_obj = if self.machine_mode() == MachineMode::Cnc3D {
                self.build_effective_cnc_shape(shape_obj)
            } else {
                shape_obj.clone()
            };
            // ============================================================
            // 1. Obtener parámetros láser EFECTIVOS (respetando use_global_laser)
            // ============================================================
            let effective_params = self.get_effective_laser_params(&effective_shape_obj);

            // Aplicar override ANTES de generar toolpaths
            if gcode_gen.is_laser_2d {
                if let Some(params) = &effective_params {
                    self.toolpath_generator.set_feed_rate(params.feed_rate);
                    self.toolpath_generator
                        .set_spindle_speed((params.power_percent * 10.0) as u32);
                } else {
                    // Restaurar valores globales
                    self.toolpath_generator
                        .set_feed_rate(self.tool_settings.feed_rate);
                    self.toolpath_generator
                        .set_spindle_speed(self.tool_settings.spindle_speed);
                }
            }

            // ============================================================
            // 2. Configurar otros parámetros
            // ============================================================
            let stock_top_z = self
                .stock_material
                .as_ref()
                .map(|s| s.thickness as f64)
                .unwrap_or(10.0);

            let effective_start_depth = if self.machine_mode() == MachineMode::Cnc3D {
                self.effective_cnc_start_depth_from_shape(&effective_shape_obj)
            } else {
                effective_shape_obj.start_depth
            };

            let (toolpath_start_depth, toolpath_cut_depth) =
                if self.machine_mode() == MachineMode::Cnc3D {
                    // CNC workflow: always start passes from stock top and descend to
                    // final object Z.
                    let cut_depth_from_stock = (stock_top_z - effective_start_depth).max(0.0);
                    (stock_top_z, cut_depth_from_stock)
                } else {
                    (effective_start_depth, 0.0)
                };

            self.toolpath_generator
                .set_pocket_strategy(effective_shape_obj.pocket_strategy);
            self.toolpath_generator
                .set_start_depth(toolpath_start_depth);
            self.toolpath_generator
                .set_cut_depth(toolpath_cut_depth);
            self.toolpath_generator
                .set_step_in(effective_shape_obj.step_in as f64);
            self.toolpath_generator
                .set_ramp_angle(effective_shape_obj.ramp_angle as f64);
            self.toolpath_generator
                .set_raster_fill_ratio(effective_shape_obj.raster_fill_ratio);

            let mut effective_shape = effective_shape_obj.get_effective_shape();

            // Keep shape-embedded laser params in sync with the effective
            // per-object/global choice so 2D toolpath generation cannot drift.
            if gcode_gen.is_laser_2d {
                if let Some(params) = effective_params {
                    Self::apply_laser_params_to_shape(&mut effective_shape, params);
                }
            }

            // ============================================================
            // 3. Generar toolpaths
            // ============================================================
            let (toolpaths, pocket_fallback_to_profile) = match &effective_shape {
                crate::model::Shape::Rectangle(rect) => {
                    if effective_shape_obj.operation_type == OperationType::Pocket {
                        (
                            self.toolpath_generator.generate_rectangle_pocket(
                                rect,
                                effective_shape_obj.start_depth,
                                effective_shape_obj.step_down as f64,
                                effective_shape_obj.step_in as f64,
                            ),
                            false,
                        )
                    } else {
                        (
                            self.toolpath_generator
                                .generate_rectangle_contour(rect, effective_shape_obj.step_down as f64),
                            false,
                        )
                    }
                }
                crate::model::Shape::Circle(circle) => {
                    if effective_shape_obj.operation_type == OperationType::Pocket {
                        (
                            self.toolpath_generator.generate_circle_pocket(
                                circle,
                                effective_shape_obj.pocket_depth,
                                effective_shape_obj.step_down as f64,
                                effective_shape_obj.step_in as f64,
                            ),
                            false,
                        )
                    } else {
                        (
                            self.toolpath_generator
                                .generate_circle_contour(circle, effective_shape_obj.step_down as f64),
                            false,
                        )
                    }
                }

                crate::model::Shape::Line(line) => (
                    self.toolpath_generator
                        .generate_line_contour(line, effective_shape_obj.step_down as f64),
                    false,
                ),

                crate::model::Shape::Ellipse(ellipse) => {
                    if effective_shape_obj.operation_type == OperationType::Pocket {
                        (
                            self.toolpath_generator.generate_ellipse_pocket(
                                ellipse,
                                effective_shape_obj.pocket_depth,
                                effective_shape_obj.step_down as f64,
                                effective_shape_obj.step_in as f64,
                            ),
                            false,
                        )
                    } else {
                        (
                            self.toolpath_generator
                                .generate_ellipse_contour(ellipse, effective_shape_obj.step_down as f64),
                            false,
                        )
                    }
                }

                crate::model::Shape::Path(path_shape) => {
                    // 1. We clone so as not to alter the original object on the canvas
                    let mut rotated_path = path_shape.clone();

                    // 2. We apply the rotation if it exists
                    if rotated_path.rotation.abs() > f64::EPSILON {
                        let (x1, y1, x2, y2) = rotated_path.bounds();
                        let cx = (x1 + x2) / 2.0;
                        let cy = (y1 + y2) / 2.0;
                        let rad = rotated_path.rotation.to_radians();
                        let translation_to_origin = nalgebra::Matrix4::new_translation(
                            &nalgebra::Vector3::new(-cx, -cy, 0.0),
                        );
                        let rotation_matrix =
                            nalgebra::Matrix4::new_rotation(nalgebra::Vector3::z() * rad);
                        let translation_back = nalgebra::Matrix4::new_translation(
                            &nalgebra::Vector3::new(cx, cy, 0.0),
                        );

                        let full_transform =
                            translation_back * rotation_matrix * translation_to_origin;

                        // Passing Matrix4
                        rotated_path.sketch = rotated_path.sketch.transform(&full_transform);
                    }

                    // 3. We generate the G-code with the object already rotated
                    if effective_shape_obj.operation_type == OperationType::Pocket {
                        (
                            self.toolpath_generator.generate_path_pocket(
                                &rotated_path,
                                effective_shape_obj.pocket_depth,
                                effective_shape_obj.step_down as f64,
                                effective_shape_obj.step_in as f64,
                            ),
                            false,
                        )
                    } else {
                        (
                            self.toolpath_generator
                                .generate_path_contour(&rotated_path, effective_shape_obj.step_down as f64),
                            false,
                        )
                    }
                }

                crate::model::Shape::Text(text) => {
                    if effective_shape_obj.operation_type == OperationType::Pocket {
                        let pocket = self
                            .toolpath_generator
                            .generate_text_pocket_toolpath(text, effective_shape_obj.step_down as f64);
                        let pocket_len: f64 = pocket.iter().map(|tp| tp.total_length()).sum();

                        if pocket_len <= 1e-9 {
                            (
                                self.toolpath_generator
                                    .generate_text_toolpath(text, effective_shape_obj.step_down as f64),
                                true,
                            )
                        } else {
                            (pocket, false)
                        }
                    } else {
                        (
                            self.toolpath_generator
                                .generate_text_toolpath(text, effective_shape_obj.step_down as f64),
                            false,
                        )
                    }
                }
                crate::model::Shape::Triangle(triangle) => {
                    if effective_shape_obj.operation_type == OperationType::Pocket {
                        (
                            self.toolpath_generator.generate_triangle_pocket(
                                triangle,
                                effective_shape_obj.pocket_depth,
                                effective_shape_obj.step_down as f64,
                                effective_shape_obj.step_in as f64,
                            ),
                            false,
                        )
                    } else {
                        (
                            self.toolpath_generator
                                .generate_triangle_contour(triangle, effective_shape_obj.step_down as f64),
                            false,
                        )
                    }
                }
                crate::model::Shape::Polygon(polygon) => {
                    if effective_shape_obj.operation_type == OperationType::Pocket {
                        (
                            self.toolpath_generator.generate_polygon_pocket(
                                polygon,
                                effective_shape_obj.pocket_depth,
                                effective_shape_obj.step_down as f64,
                                effective_shape_obj.step_in as f64,
                            ),
                            false,
                        )
                    } else {
                        (
                            self.toolpath_generator
                                .generate_polygon_contour(polygon, effective_shape_obj.step_down as f64),
                            false,
                        )
                    }
                }

                crate::model::Shape::RasterImage(_) => {
                    // Raster images do not directly generate a toolpath.
                    (Vec::new(), false)
                }
                _ => {
                    let path = effective_shape.render();
                    let design_path = crate::model::DesignPath::from_lyon_path(&path);
                    let toolpaths = if effective_shape_obj.operation_type == OperationType::Pocket {
                        self.toolpath_generator.generate_path_pocket(
                            &design_path,
                            effective_shape_obj.pocket_depth,
                            effective_shape_obj.step_down as f64,
                            effective_shape_obj.step_in as f64,
                        )
                    } else {
                        self.toolpath_generator
                            .generate_path_contour(&design_path, effective_shape_obj.step_down as f64)
                    };
                    (toolpaths, false)
                }
            };

            // ============================================================
            // 4. Guardar el resultado
            // ============================================================
            shape_toolpaths.push((effective_shape_obj, toolpaths, pocket_fallback_to_profile));
        }

        // Calculate total length from all toolpaths
        let total_length: f64 = shape_toolpaths
            .iter()
            .flat_map(|(_, tps, _)| tps.iter())
            .map(|tp| tp.total_length())
            .sum();

        // Use settings from first toolpath if available, or defaults
        let (header_speed, header_feed, header_diam, header_depth) =
            if let Some((_, tps, _)) = shape_toolpaths.first() {
                if let Some(first) = tps.first() {
                    let s = first
                        .segments
                        .first()
                        .map(|seg| seg.spindle_speed)
                        .unwrap_or(3000);
                    let f = first
                        .segments
                        .first()
                        .map(|seg| seg.feed_rate)
                        .unwrap_or(100.0);
                    (s, f, first.tool_diameter, first.depth)
                } else {
                    (3000, 100.0, 3.175, -5.0)
                }
            } else {
                (3000, 100.0, 3.175, -5.0)
            };

        gcode.push_str(&gcode_gen.generate_header(
            header_speed,
            header_feed,
            header_diam,
            header_depth,
            total_length,
        ));

        // ========== ADVERTENCIA DE LÍMITES  ==========
        let mut has_violation = false;
        for (_, toolpaths, _) in &shape_toolpaths {
            for toolpath in toolpaths {
                if gcode_gen.has_boundary_violation(toolpath) {
                    has_violation = true;
                    break;
                }
            }
            if has_violation {
                break;
            }
        }

        if has_violation {
            gcode.push_str(";\n");
            gcode.push_str("; *********************************************\n");
            gcode.push_str("; ********** WARNING: OUT OF LIMITS ***********\n");
            gcode.push_str("; ** This toolpath contains coordinates      **\n");
            gcode.push_str("; ** outside the machine working area.       **\n");
            gcode.push_str("; ** Risk of collision if no limit switches! **\n");
            gcode.push_str("; *********************************************\n");
            gcode.push_str(";\n");
        }
        // =============== FIN ADVERTENCIA ===============

        let mut line_number = 10;

        // ------ Loop Shape --------
        for (shape, toolpaths, pocket_fallback_to_profile) in shape_toolpaths.iter() {
            // ============================================================
            // Obtener parámetros efectivos para G-code
            // ============================================================
            let effective_params = self.get_effective_laser_params(shape);

            // Aplicar override si estamos en modo láser
            if gcode_gen.is_laser_2d {
                if let Some(params) = &effective_params {
                    self.toolpath_generator.set_feed_rate(params.feed_rate);
                    self.toolpath_generator
                        .set_spindle_speed((params.power_percent * 10.0) as u32);
                } else {
                    self.toolpath_generator
                        .set_feed_rate(self.tool_settings.feed_rate);
                    self.toolpath_generator
                        .set_spindle_speed(self.tool_settings.spindle_speed);
                }
            }

            if let crate::model::Shape::RasterImage(raster) = &shape.shape {
                // Generar G-code para raster
                use crate::engraving::ImageEngraver;

                if raster.image_data.is_empty() && raster.original_path.is_none() {
                    eprintln!("Warning: RasterImage ID={} has no image data", shape.id);
                    continue;
                }

                // Calcular esquina inferior izquierda
                let start_x = raster.center.x - raster.width_mm / 2.0;
                let start_y = raster.center.y - raster.height_mm / 2.0;

                // Añadir comentarios
                gcode.push_str(&format!("\n; Raster Image ID={}\n", shape.id));
                gcode.push_str(&format!("; Name: {}\n", shape.name));
                gcode.push_str(&format!("; Position: ({:.3}, {:.3})\n", start_x, start_y));
                gcode.push_str(&format!(
                    "; Size: {:.2} x {:.2} mm\n",
                    raster.width_mm, raster.height_mm
                ));

                // Crear parámetros y generar G-code
                let params = crate::engraving::EngravingParams {
                    width_mm: raster.width_mm as f32,
                    height_mm: Some(raster.height_mm as f32),
                    feed_rate: raster.feed_rate as f32,
                    travel_rate: raster.travel_rate as f32,
                    min_power: raster.min_power as f32,
                    max_power: raster.max_power as f32,
                    ppi: raster.ppi as f32,
                    scan_direction: if raster.scan_direction == "vertical" {
                        crate::engraving::ScanDirection::Vertical
                    } else {
                        crate::engraving::ScanDirection::Horizontal
                    },
                    bidirectional: raster.bidirectional,
                    invert: raster.invert,
                    mirror_x: false,
                    mirror_y: false,
                    rotation: crate::engraving::RotationAngle::Degrees0,
                    halftone: crate::engraving::HalftoneMethod::None,
                    halftone_threshold: 128,
                    offset_x: start_x as f32,
                    offset_y: start_y as f32,
                    power_scale: 1000.0,
                    line_spacing: 1.0,
                };

                let engraver_result = if let Some(path) = &raster.original_path {
                    ImageEngraver::from_file(path, params)
                } else if !raster.image_data.is_empty() {
                    match image::load_from_memory(&raster.image_data) {
                        Ok(img) => ImageEngraver::from_image(img, params),
                        Err(e) => {
                            eprintln!("Error loading image from memory: {}", e);
                            continue;
                        }
                    }
                } else {
                    continue;
                };

                match engraver_result {
                    Ok(engraver) => match engraver.generate_gcode() {
                        Ok(image_gcode) => {
                            gcode.push_str(&image_gcode);
                            gcode.push('\n');
                        }
                        Err(e) => {
                            eprintln!("Error generating G-code for image ID={}: {}", shape.id, e);
                        }
                    },
                    Err(e) => {
                        eprintln!(
                            "Error creating ImageEngraver for image ID={}: {}",
                            shape.id, e
                        );
                    }
                }

                continue; // Saltar el procesamiento normal para este shape
            }

            // Add shape metadata as comments
            gcode.push_str(&format!(
                "\n; Shape ID={}, Type={:?}\n",
                shape.id,
                shape.shape.shape_type()
            ));
            gcode.push_str(&format!("; Name: {}\n", shape.name));
            gcode.push_str(&format!("; Operation: {:?}\n", shape.operation_type));
            if *pocket_fallback_to_profile {
                gcode.push_str("; NOTE: Text pocketing produced no valid pocket area for the current tool/text size; fell back to profile toolpath.\n");
            }

            // Add shape-specific data
            Self::append_shape_metadata(&mut gcode, shape);

            if shape.operation_type == OperationType::Pocket {
                gcode.push_str(&format!(
                    "; Z depth: {:.3}mm, Step down: {:.3}mm, Step in: {:.3}mm\n",
                    shape.start_depth, shape.step_down, shape.step_in
                ));
                gcode.push_str(&format!("; Strategy: {:?}\n", shape.pocket_strategy));
            } else {
                if gcode_gen.is_laser_2d {
                } else {
                    let cnc_effective_depth = if self.machine_mode() == MachineMode::Cnc3D {
                        let stock_top_z = self
                            .stock_material
                            .as_ref()
                            .map(|s| s.thickness as f64)
                            .unwrap_or(10.0);
                        let object_final_z = self.effective_cnc_start_depth_from_shape(shape);
                        (stock_top_z - object_final_z).max(0.0)
                    } else {
                        shape.pocket_depth
                    };
                    gcode.push_str(&format!(
                        "; Cut depth: {:.3}mm, Step down: {:.3}mm\n",
                        cnc_effective_depth, shape.step_down
                    ));
                }
            }

            // Generate G-code for all toolpaths associated with this shape
            let mut current_z = gcode_gen.safe_z;
            // Init
            let global_step_down = self.tool_settings.step_down;

            let num_passes = if gcode_gen.is_laser_2d {
                if let Some(params) = &effective_params {
                    params.passes.max(1) as usize
                } else {
                    global_step_down.max(1.0) as usize
                }
            } else {
                // CNC MODE: toolpaths already include multipass depth logic.
                1
            };

            for pass in 0..num_passes {
                if pass > 0 {
                    // Reposition to the start for subsequent passes
                    gcode.push_str(&format!("; Reposition for pass {}\n", pass + 1));
                }

                if gcode_gen.is_laser_2d {
                    for toolpath in toolpaths {
                        // Optimizes curves and joins collinear segments
                        let optimized = gcode_gen.optimize_toolpath_for_laser(toolpath);

                        let (body_gcode, final_z) =
                            gcode_gen.generate_body_continuing(&optimized, line_number, current_z);

                        gcode.push_str(&body_gcode);
                        line_number += (optimized.segments.len() as u32) * 10;
                        current_z = final_z;
                    }
                } else {
                    // CNC mode without optimization
                    for toolpath in toolpaths {
                        let (body_gcode, final_z) =
                            gcode_gen.generate_body_continuing(toolpath, line_number, current_z);
                        gcode.push_str(&body_gcode);
                        line_number += (toolpath.segments.len() as u32) * 10;
                        current_z = final_z;
                    }
                }
            }

            if let Some(last_tp) = toolpaths.last() {
                for seg in last_tp.segments.iter().rev() {
                    if seg.end.x != 0.0 || seg.end.y != 0.0 {
                        break;
                    }
                }
            }
        } // for (shape, toolpaths, pocket_fallback_to_profile)

        gcode.push_str(&gcode_gen.generate_footer());

        self.generated_gcode = gcode.clone();
        self.gcode_generated = self.canvas.shape_count() > 0;
        (gcode, has_violation)
    }

    /// Generates G-code from the current design.
    pub fn generate_gcode(&mut self, machine_limits: Option<MachineLimits>) -> String {
        self.generate_gcode_with_warning_info(machine_limits).0
    }

    /// Appends shape-specific metadata to G-code comments.
    fn append_shape_metadata(gcode: &mut String, shape: &DrawingObject) {
        match &shape.shape {
            crate::model::Shape::Rectangle(rect) => {
                let (x1, y1, x2, y2) = rect.bounds();
                gcode.push_str(&format!(
                    "; Position: ({:.3}, {:.3}) to ({:.3}, {:.3})\n",
                    x1, y1, x2, y2
                ));
                gcode.push_str(&format!("; Corner radius: {:.3}mm\n", rect.corner_radius));
            }
            crate::model::Shape::Circle(circle) => {
                gcode.push_str(&format!(
                    "; Center: ({:.3}, {:.3}), Radius: {:.3}mm\n",
                    circle.center.x, circle.center.y, circle.radius
                ));
            }
            crate::model::Shape::Line(line) => {
                gcode.push_str(&format!(
                    "; Start: ({:.3}, {:.3}), End: ({:.3}, {:.3})\n",
                    line.start.x, line.start.y, line.end.x, line.end.y
                ));
            }
            crate::model::Shape::Ellipse(ellipse) => {
                let (x1, y1, x2, y2) = ellipse.bounds();
                gcode.push_str(&format!(
                    "; Position: ({:.3}, {:.3}) to ({:.3}, {:.3})\n",
                    x1, y1, x2, y2
                ));
            }
            crate::model::Shape::Path(path) => {
                let (x1, y1, x2, y2) = path.bounds();
                gcode.push_str(&format!(
                    "; Path bounds: ({:.3}, {:.3}) to ({:.3}, {:.3})\n",
                    x1, y1, x2, y2
                ));
            }
            crate::model::Shape::Text(text) => {
                gcode.push_str(&format!(
                    "; Text: \"{}\", Font size: {:.3}mm\n",
                    text.text, text.font_size
                ));
                gcode.push_str(&format!("; Position: ({:.3}, {:.3})\n", text.x, text.y));
            }
            crate::model::Shape::Triangle(triangle) => {
                gcode.push_str(&format!(
                    "; Triangle: Center ({:.3}, {:.3}), Width: {:.3}mm, Height: {:.3}mm\n",
                    triangle.center.x, triangle.center.y, triangle.width, triangle.height
                ));
            }
            crate::model::Shape::Polygon(polygon) => {
                gcode.push_str(&format!(
                    "; Polygon: Center ({:.3}, {:.3}), Radius: {:.3}mm, Sides: {}\n",
                    polygon.center.x, polygon.center.y, polygon.radius, polygon.sides
                ));
            }
            crate::model::Shape::Gear(gear) => {
                gcode.push_str(&format!(
                    "; Gear: Center ({:.3}, {:.3}), Module: {:.3}, Teeth: {}\n",
                    gear.center.x, gear.center.y, gear.module, gear.teeth
                ));
            }
            crate::model::Shape::Sprocket(sprocket) => {
                gcode.push_str(&format!(
                    "; Sprocket: Center ({:.3}, {:.3}), Pitch: {:.3}, Teeth: {}\n",
                    sprocket.center.x, sprocket.center.y, sprocket.pitch, sprocket.teeth
                ));
            }

            crate::model::Shape::RasterImage(raster) => {
                let (x1, y1, x2, y2) = raster.bounds();
                gcode.push_str(&format!(
                    "; Raster Image: ({:.3}, {:.3}) to ({:.3}, {:.3})\n",
                    x1, y1, x2, y2
                ));
                if let Some(path) = &raster.original_path {
                    gcode.push_str(&format!("; Original file: {}\n", path.display()));
                }
            }
        }
    }
}
