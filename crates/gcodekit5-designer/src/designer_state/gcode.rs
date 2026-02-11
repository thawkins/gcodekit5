//! G-code generation for designer state.

use super::DesignerState;
use crate::canvas::DrawingObject;
use crate::model::DesignerShape;
use crate::shapes::OperationType;
use crate::{Circle, Point, ToolpathToGcode};
use gcodekit5_core::Units;

impl DesignerState {
    /// Generates G-code from the current design.
    pub fn generate_gcode(&mut self) -> String {
        let mut gcode = String::new();
        // Get safe_z from stock_material, default to 10.0 if not set
        let safe_z = self
            .stock_material
            .as_ref()
            .map(|s| s.safe_z as f64)
            .unwrap_or(10.0);
        let mut gcode_gen = ToolpathToGcode::new(Units::MM, safe_z);
        gcode_gen.num_axes = self.num_axes;

        // Store shape-to-toolpath mapping (plus whether we had to fall back from pocket->profile)
        let mut shape_toolpaths: Vec<(DrawingObject, Vec<crate::Toolpath>, bool)> = Vec::new();

        // Collect shape IDs in reverse draw order (front to back) for G-code generation
        let shape_ids: Vec<u64> = self.canvas.shape_store.draw_order_iter().rev().collect();

        for shape_id in shape_ids {
            let Some(shape_obj) = self.canvas.shape_store.get(shape_id) else {
                continue;
            };
            self.toolpath_generator
                .set_pocket_strategy(shape_obj.pocket_strategy);
            self.toolpath_generator
                .set_start_depth(shape_obj.start_depth);
            self.toolpath_generator
                .set_cut_depth(shape_obj.pocket_depth);
            self.toolpath_generator
                .set_step_in(shape_obj.step_in as f64);
            self.toolpath_generator
                .set_ramp_angle(shape_obj.ramp_angle as f64);
            self.toolpath_generator
                .set_raster_fill_ratio(shape_obj.raster_fill_ratio);

            let effective_shape = shape_obj.get_effective_shape();

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
                    let (x1, y1, x2, y2) = ellipse.bounds();
                    let cx = (x1 + x2) / 2.0;
                    let cy = (y1 + y2) / 2.0;
                    let radius = ((x2 - x1).abs().max((y2 - y1).abs())) / 2.0;
                    let circle = Circle::new(Point::new(cx, cy), radius);
                    (
                        self.toolpath_generator
                            .generate_circle_contour(&circle, shape_obj.step_down as f64),
                        false,
                    )
                }
                crate::model::Shape::Path(path_shape) => {
                    if shape_obj.operation_type == OperationType::Pocket {
                        (
                            self.toolpath_generator.generate_path_pocket(
                                path_shape,
                                shape_obj.pocket_depth,
                                shape_obj.step_down as f64,
                                shape_obj.step_in as f64,
                            ),
                            false,
                        )
                    } else {
                        (
                            self.toolpath_generator
                                .generate_path_contour(path_shape, shape_obj.step_down as f64),
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
        let mut is_first_shape = true;

        for (shape, toolpaths, pocket_fallback_to_profile) in shape_toolpaths.iter() {
            if !is_first_shape && self.num_axes >= 3 {
                gcode.push_str(&format!(
                    "G00 Z{:.3}   ; Retract to safe Z before next shape\n",
                    safe_z
                ));
                line_number += 10;
            }
            is_first_shape = false;

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
                gcode.push_str(&format!(
                    "; Cut depth: {:.3}mm, Step down: {:.3}mm\n",
                    shape.pocket_depth, shape.step_down
                ));
            }

            // Generate G-code for all toolpaths associated with this shape
            let mut current_z = gcode_gen.safe_z;
            for toolpath in toolpaths {
                let (body_gcode, final_z) =
                    gcode_gen.generate_body_continuing(toolpath, line_number, current_z);
                gcode.push_str(&body_gcode);
                line_number += (toolpath.segments.len() as u32) * 10;
                current_z = final_z;
            }
        }

        gcode.push_str(&gcode_gen.generate_footer());

        self.generated_gcode = gcode.clone();
        self.gcode_generated = self.canvas.shape_count() > 0;
        gcode
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
        }
    }
}
