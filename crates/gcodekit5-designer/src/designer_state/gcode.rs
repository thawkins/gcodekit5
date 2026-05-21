//! G-code generation for designer state


use super::DesignerState;
use crate::canvas::DrawingObject;
use crate::model::DesignerShape;
use crate::model::LaserParams;
use crate::shapes::OperationType;
use crate::designer_state::MachineMode;
use crate::ToolpathToGcode;
use csgrs::traits::CSG;
use gcodekit5_core::Units;

use crate::Shape;


impl DesignerState {
    /// Obtiene los parámetros láser efectivos para un objeto
    fn get_effective_laser_params(&self, shape: &DrawingObject) -> Option<LaserParams> {

        if shape.use_global_laser {
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
            // Usar valores específicos del objeto
            match &shape.shape {
                Shape::Rectangle(rect) => {
                    let mut params = rect.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::Circle(circle) => {
                    let mut params = circle.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::Ellipse(ellipse) => {
                    let mut params = ellipse.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::Line(line) => {
                    let mut params = line.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::Path(path) => {
                    let mut params = path.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::Polygon(poly) => {
                    let mut params = poly.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::Triangle(tri) => {
                    let mut params = tri.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::Text(text) => {
                    let mut params = text.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::Gear(gear) => {
                    let mut params = gear.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::Sprocket(sprocket) => {
                    let mut params = sprocket.laser_params;
                    params.use_global = false;
                    Some(params)
                },
                Shape::RasterImage(_) => None,
            }
        }
    }

    /// Generates G-code from the current design.
    #[allow(clippy::collapsible_else_if)]
    pub fn generate_gcode(&mut self) -> String {

        let mut gcode = String::new();
        // Get safe_z from stock_material, default to 10.0 if not set
        let safe_z = self
            .stock_material
            .as_ref()
            .map(|s| s.safe_z as f64)
            .unwrap_or(10.0);
        let mut gcode_gen = ToolpathToGcode::new(Units::MM, safe_z);
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
        let selected_id = self.canvas.selection_manager.selected_id();

        // Collect shape IDs in reverse draw order (front to back) for G-code generation
        let shape_ids: Vec<u64> = self.canvas.shape_store.draw_order_iter().collect();



        for shape_id in shape_ids {
            let Some(shape_obj) = self.canvas.shape_store.get(shape_id) else {
                continue;
            };

            if let Some(sel_id) = selected_id {
                if shape_obj.id != sel_id {
                    continue;
                }
            }

            // ============================================================
            // 1. Obtener parámetros láser EFECTIVOS (respetando use_global_laser)
            // ============================================================
            let effective_params = self.get_effective_laser_params(shape_obj);

            // Aplicar override ANTES de generar toolpaths
            if gcode_gen.is_laser_2d {
                if let Some(params) = &effective_params {
                    self.toolpath_generator.set_feed_rate(params.feed_rate);
                    self.toolpath_generator.set_spindle_speed((params.power_percent * 10.0) as u32);
                } else {
                    // Restaurar valores globales
                    self.toolpath_generator.set_feed_rate(self.tool_settings.feed_rate);
                    self.toolpath_generator.set_spindle_speed(self.tool_settings.spindle_speed);
                }
            }

            // ============================================================
            // 2. Configurar otros parámetros
            // ============================================================
            self.toolpath_generator.set_pocket_strategy(shape_obj.pocket_strategy);
            self.toolpath_generator.set_start_depth(shape_obj.start_depth);
            self.toolpath_generator.set_cut_depth(shape_obj.pocket_depth);
            self.toolpath_generator.set_step_in(shape_obj.step_in as f64);
            self.toolpath_generator.set_ramp_angle(shape_obj.ramp_angle as f64);
            self.toolpath_generator.set_raster_fill_ratio(shape_obj.raster_fill_ratio);

            let effective_shape = shape_obj.get_effective_shape();

            // ============================================================
            // 3. Generar toolpaths
            // ============================================================
                    let (toolpaths, pocket_fallback_to_profile) = match &effective_shape {
                        crate::model::Shape::Rectangle(rect) => {
                            if shape_obj.operation_type == OperationType::Pocket {
                                (
                                    self.toolpath_generator.generate_rectangle_pocket(
                                        rect,
                                        shape_obj.pocket_depth,
                                        shape_obj.step_down as f64,
                                        shape_obj.step_in as f64,
                                    ),
                                    false,
                                )
                            } else {
                                (
                                    self.toolpath_generator
                                        .generate_rectangle_contour(rect, shape_obj.step_down as f64),
                                    false,
                                )
                            }
                        }
                        crate::model::Shape::Circle(circle) => {
                            if shape_obj.operation_type == OperationType::Pocket {
                                (
                                    self.toolpath_generator.generate_circle_pocket(
                                        circle,
                                        shape_obj.pocket_depth,
                                        shape_obj.step_down as f64,
                                        shape_obj.step_in as f64,
                                    ),
                                    false,
                                )
                            } else {
                                (
                                    self.toolpath_generator
                                        .generate_circle_contour(circle, shape_obj.step_down as f64),
                                    false,
                                )
                            }
                        }

                        crate::model::Shape::Line(line) => (
                            self.toolpath_generator
                                .generate_line_contour(line, shape_obj.step_down as f64),
                            false,
                        ),

                        crate::model::Shape::Ellipse(ellipse) => {
                            if shape_obj.operation_type == OperationType::Pocket {
                                (
                                    self.toolpath_generator.generate_ellipse_pocket(
                                        ellipse,
                                        shape_obj.pocket_depth,
                                        shape_obj.step_down as f64,
                                        shape_obj.step_in as f64,
                                    ),
                                    false,
                                )
                            } else {
                                (
                                    self.toolpath_generator
                                        .generate_ellipse_contour(ellipse, shape_obj.step_down as f64),
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
                            if shape_obj.operation_type == OperationType::Pocket {
                                (
                                    self.toolpath_generator.generate_path_pocket(
                                        &rotated_path,
                                        shape_obj.pocket_depth,
                                        shape_obj.step_down as f64,
                                        shape_obj.step_in as f64,
                                    ),
                                    false,
                                )
                            } else {
                                (
                                    self.toolpath_generator
                                        .generate_path_contour(&rotated_path, shape_obj.step_down as f64),
                                    false,
                                )
                            }
                        }

                        crate::model::Shape::Text(text) => {
                            if shape_obj.operation_type == OperationType::Pocket {
                                let pocket = self
                                    .toolpath_generator
                                    .generate_text_pocket_toolpath(text, shape_obj.step_down as f64);
                                let pocket_len: f64 = pocket.iter().map(|tp| tp.total_length()).sum();

                                if pocket_len <= 1e-9 {
                                    (
                                        self.toolpath_generator
                                            .generate_text_toolpath(text, shape_obj.step_down as f64),
                                        true,
                                    )
                                } else {
                                    (pocket, false)
                                }
                            } else {
                                (
                                    self.toolpath_generator
                                        .generate_text_toolpath(text, shape_obj.step_down as f64),
                                    false,
                                )
                            }
                        }
                        crate::model::Shape::Triangle(triangle) => {
                            if shape_obj.operation_type == OperationType::Pocket {
                                (
                                    self.toolpath_generator.generate_triangle_pocket(
                                        triangle,
                                        shape_obj.pocket_depth,
                                        shape_obj.step_down as f64,
                                        shape_obj.step_in as f64,
                                    ),
                                    false,
                                )
                            } else {
                                (
                                    self.toolpath_generator
                                        .generate_triangle_contour(triangle, shape_obj.step_down as f64),
                                    false,
                                )
                            }
                        }
                        crate::model::Shape::Polygon(polygon) => {
                            if shape_obj.operation_type == OperationType::Pocket {
                                (
                                    self.toolpath_generator.generate_polygon_pocket(
                                        polygon,
                                        shape_obj.pocket_depth,
                                        shape_obj.step_down as f64,
                                        shape_obj.step_in as f64,
                                    ),
                                    false,
                                )
                            } else {
                                (
                                    self.toolpath_generator
                                        .generate_polygon_contour(polygon, shape_obj.step_down as f64),
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
                            let toolpaths = if shape_obj.operation_type == OperationType::Pocket {
                                self.toolpath_generator.generate_path_pocket(
                                    &design_path,
                                    shape_obj.pocket_depth,
                                    shape_obj.step_down as f64,
                                    shape_obj.step_in as f64,
                                )
                            } else {
                                self.toolpath_generator
                                    .generate_path_contour(&design_path, shape_obj.step_down as f64)
                            };
                            (toolpaths, false)
                        }
                    };

            // ============================================================
            // 4. Guardar el resultado
            // ============================================================
            shape_toolpaths.push((shape_obj.clone(), toolpaths, pocket_fallback_to_profile));
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
                    self.toolpath_generator.set_spindle_speed((params.power_percent * 10.0) as u32);
                } else {

                    self.toolpath_generator.set_feed_rate(self.tool_settings.feed_rate);
                    self.toolpath_generator.set_spindle_speed(self.tool_settings.spindle_speed);
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
                gcode.push_str(&format!("; Size: {:.2} x {:.2} mm\n", raster.width_mm, raster.height_mm));

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
                    Ok(engraver) => {
                        match engraver.generate_gcode() {
                            Ok(image_gcode) => {
                                gcode.push_str(&image_gcode);
                                gcode.push('\n');
                            }
                            Err(e) => {
                                eprintln!("Error generating G-code for image ID={}: {}", shape.id, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error creating ImageEngraver for image ID={}: {}", shape.id, e);
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
                    "; Pocket depth: {:.3}mm, Step down: {:.3}mm, Step in: {:.3}mm\n",
                    shape.pocket_depth, shape.step_down, shape.step_in
                ));
                gcode.push_str(&format!("; Strategy: {:?}\n", shape.pocket_strategy));
            } else {
                if gcode_gen.is_laser_2d {
                } else {
                    gcode.push_str(&format!(
                        "; Cut depth: {:.3}mm, Step down: {:.3}mm\n",
                        shape.pocket_depth, shape.step_down
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
                // CNC MODE: use step_down
                let total_depth = (shape.start_depth - shape.pocket_depth).abs();
                if total_depth <= 0.001 {
                    1
                } else {
                    let step = if shape.step_down <= 0.001 {
                        total_depth
                    } else {
                        shape.step_down as f64
                    };
                    (total_depth / step).ceil() as usize
                }
            };

            for pass in 0..num_passes {
                if pass > 0 {
                    // Reposition to the start for subsequent passes
                    if let Some(first_tp) = toolpaths.first() {
                        if let Some(first_seg) = first_tp.segments.first() {
                            gcode.push_str(&format!(
                                "G00 X{:.3} Y{:.3}   ; Reposition for pass {}\n",
                                first_seg.start.x,
                                first_seg.start.y,
                                pass + 1
                            ));
                        }
                    }
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
        gcode
    } // pub fn generate_gcode

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
