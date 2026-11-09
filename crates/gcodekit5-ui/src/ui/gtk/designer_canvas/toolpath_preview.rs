//! Toolpath preview generation for the designer canvas.
//!
//! The preview must match the generated G-code, so the per-shape parameters are
//! resolved exactly as `DesignerState::generate_gcode_with_warning_info` does:
//! in CNC mode shapes without custom values use the global/default CAM values and
//! depths are measured from the stock top; in laser mode the shape's own values apply.

use super::*;
use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::model::{DesignPath, DesignerShape, Shape};
use gcodekit5_designer::shapes::OperationType;
use gcodekit5_designer::toolpath::{Toolpath, ToolpathGenerator};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Settings snapshot needed to resolve the effective per-shape CAM parameters.
struct PreviewSettings {
    is_cnc_mode: bool,
    /// Global step down from the tool settings (mm).
    global_step_down: f64,
    /// Stock top Z (mm); G-code assumes 10 mm when no stock is configured.
    stock_top_z: f64,
    /// Default CAM properties applied to shapes that do not use custom values.
    defaults: DrawingObject,
}

impl PreviewSettings {
    /// Returns the shape with the CAM parameters G-code generation would use.
    fn effective_object(&self, shape: &DrawingObject) -> DrawingObject {
        if !self.is_cnc_mode {
            return shape.clone();
        }
        let mut effective = shape.clone();
        if !shape.use_custom_values {
            effective.operation_type = self.defaults.operation_type;
            effective.step_in = self.defaults.step_in;
            effective.ramp_angle = self.defaults.ramp_angle;
            effective.pocket_strategy = self.defaults.pocket_strategy;
            effective.raster_fill_ratio = self.defaults.raster_fill_ratio;
            effective.step_down = if self.global_step_down > 0.0 {
                self.global_step_down as f32
            } else {
                self.defaults.step_down
            };
        }
        if effective.step_down <= 0.0 {
            effective.step_down = 0.1;
        }
        if effective.pocket_depth < 0.0 {
            effective.pocket_depth = 0.0;
        }
        effective
    }

    /// Returns `(start_depth, cut_depth)` for the toolpath generator.
    fn depths(&self, effective: &DrawingObject) -> (f64, f64) {
        if self.is_cnc_mode {
            // Machine Z is absolute (table at 0, stock top at stock thickness); passes
            // start at the stock top and descend to the object's Z.
            let start_z = self.stock_top_z - effective.start_depth;
            (self.stock_top_z, (self.stock_top_z - start_z).max(0.0))
        } else {
            (effective.start_depth, 0.0)
        }
    }
}

/// Generates the toolpaths for one shape using the same per-shape parameters and
/// generator calls as G-code generation.
fn shape_preview_toolpaths(
    gen: &mut ToolpathGenerator,
    settings: &PreviewSettings,
    shape: &DrawingObject,
) -> Vec<Toolpath> {
    let obj = settings.effective_object(shape);
    let (start_depth, cut_depth) = settings.depths(&obj);

    gen.set_pocket_strategy(obj.pocket_strategy);
    gen.set_start_depth(start_depth);
    gen.set_cut_depth(cut_depth);
    gen.set_step_in(obj.step_in as f64);
    gen.set_ramp_angle(obj.ramp_angle as f64);
    gen.set_raster_fill_ratio(obj.raster_fill_ratio);

    let is_pocket = obj.operation_type == OperationType::Pocket;
    let step_down = obj.step_down as f64;
    let step_in = obj.step_in as f64;
    let pocket_depth = obj.effective_pocket_depth();

    let effective_shape = obj.get_effective_shape();
    match &effective_shape {
        Shape::Rectangle(rect) if is_pocket => {
            gen.generate_rectangle_pocket(rect, pocket_depth, step_down, step_in)
        }
        Shape::Rectangle(rect) => gen.generate_rectangle_contour(rect, step_down),
        Shape::Circle(circle) if is_pocket => {
            gen.generate_circle_pocket(circle, pocket_depth, step_down, step_in)
        }
        Shape::Circle(circle) => gen.generate_circle_contour(circle, step_down),
        Shape::Line(line) => gen.generate_line_contour(line, step_down),
        Shape::Ellipse(ellipse) if is_pocket => {
            gen.generate_ellipse_pocket(ellipse, pocket_depth, step_down, step_in)
        }
        Shape::Ellipse(ellipse) => gen.generate_ellipse_contour(ellipse, step_down),
        Shape::Path(path) if is_pocket => {
            gen.generate_path_pocket(path, pocket_depth, step_down, step_in)
        }
        Shape::Path(path) => gen.generate_path_contour(path, step_down),
        Shape::Text(text) if is_pocket => {
            let pocket = gen.generate_text_pocket_toolpath(text, step_down);
            let length: f64 = pocket.iter().map(|tp| tp.total_length()).sum();
            if length <= 1e-9 {
                // Same fallback as G-code generation: profile the glyphs.
                gen.generate_text_toolpath(text, step_down)
            } else {
                pocket
            }
        }
        Shape::Text(text) => gen.generate_text_toolpath(text, step_down),
        Shape::Triangle(triangle) if is_pocket => {
            gen.generate_triangle_pocket(triangle, pocket_depth, step_down, step_in)
        }
        Shape::Triangle(triangle) => gen.generate_triangle_contour(triangle, step_down),
        Shape::Polygon(polygon) if is_pocket => {
            gen.generate_polygon_pocket(polygon, pocket_depth, step_down, step_in)
        }
        Shape::Polygon(polygon) => gen.generate_polygon_contour(polygon, step_down),
        // Raster images are engraved line by line, not as a vector toolpath.
        Shape::RasterImage(_) => Vec::new(),
        // Gears and sprockets go through their rendered outline, as in G-code.
        Shape::Gear(_) | Shape::Sprocket(_) => {
            let path = DesignPath::from_lyon_path(&effective_shape.render());
            if is_pocket {
                gen.generate_path_pocket(&path, pocket_depth, step_down, step_in)
            } else {
                gen.generate_path_contour(&path, step_down)
            }
        }
    }
}

impl DesignerCanvas {
    /// Regenerates the toolpath preview in a background thread.
    ///
    /// If a generation is already running it is cancelled and restarted once it stops.
    pub fn generate_preview_toolpaths(&self) {
        if self.preview_generating.get() {
            self.preview_pending.set(true);
            self.preview_cancel.store(true, Ordering::SeqCst);
            return;
        }

        self.preview_generating.set(true);
        self.preview_cancel.store(false, Ordering::SeqCst);

        let started_at = std::time::Instant::now();

        let (shapes, feed_rate, spindle_speed, tool_diameter, settings) = {
            let state = self.state.borrow();
            let settings = PreviewSettings {
                is_cnc_mode: state.machine_mode()
                    == gcodekit5_designer::designer_state::MachineMode::Cnc3D,
                global_step_down: state.tool_settings.step_down,
                stock_top_z: state
                    .stock_material
                    .as_ref()
                    .map(|s| s.thickness as f64)
                    .unwrap_or(10.0),
                defaults: state.default_properties_shape.clone(),
            };
            (
                state.canvas.shapes().cloned().collect::<Vec<_>>(),
                state.tool_settings.feed_rate,
                state.tool_settings.spindle_speed,
                state.tool_settings.tool_diameter,
                settings,
            )
        };

        let total_shapes = shapes.len().max(1);
        let done_shapes: Arc<std::sync::atomic::AtomicUsize> =
            Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // Global status bar progress + cancel (non-blocking)
        if let Some(sb) = self.status_bar.as_ref() {
            let cancel_flag = self.preview_cancel.clone();
            let generating = self.preview_generating.clone();
            sb.set_progress(0.1, "0s", "");
            sb.set_cancel_action(Some(std::boxed::Box::new(move || {
                cancel_flag.store(true, Ordering::SeqCst);
                generating.set(false);
            })));
        }

        let cancel = self.preview_cancel.clone();
        let done_shapes_thread = done_shapes.clone();
        let result_arc: gcodekit5_core::ThreadSafeOption<Vec<Toolpath>> =
            gcodekit5_core::thread_safe_none();
        let result_arc_thread = result_arc.clone();

        std::thread::spawn(move || {
            let mut gen = ToolpathGenerator::new();
            gen.set_feed_rate(feed_rate);
            gen.set_spindle_speed(spindle_speed);
            gen.set_tool_diameter(tool_diameter);
            gen.set_step_in(tool_diameter * 0.4);

            let mut toolpaths = Vec::new();
            for shape in &shapes {
                if cancel.load(Ordering::SeqCst) {
                    return;
                }
                toolpaths.extend(shape_preview_toolpaths(&mut gen, &settings, shape));
                done_shapes_thread.fetch_add(1, Ordering::Relaxed);
            }

            *result_arc_thread.lock() = Some(toolpaths);
        });

        // Poll for completion (non-blocking)
        let poll_count = Rc::new(RefCell::new(0u32));
        let poll_count_clone = poll_count.clone();
        let result_arc_poll = result_arc.clone();
        let canvas = self.widget.clone();
        let out = self.preview_toolpaths.clone();
        let generating = self.preview_generating.clone();
        let pending = self.preview_pending.clone();
        let cancel_poll = self.preview_cancel.clone();
        let done_shapes_poll = done_shapes.clone();
        let sb_poll = self.status_bar.clone();
        let self_ref = self.clone();

        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            *poll_count_clone.borrow_mut() += 1;

            if let Some(sb) = sb_poll.as_ref() {
                let done = done_shapes_poll.load(Ordering::Relaxed).min(total_shapes);
                let pct = (done as f64 / total_shapes as f64) * 100.0;
                let elapsed = started_at.elapsed().as_secs_f64();
                sb.set_progress(pct.max(0.1), &format!("{:.0}s", elapsed), "");
            }

            if cancel_poll.load(Ordering::SeqCst) {
                generating.set(false);
                if let Some(sb) = sb_poll.as_ref() {
                    sb.set_progress(0.0, "", "");
                    sb.set_cancel_action(None);
                }
                if pending.replace(false) {
                    self_ref.generate_preview_toolpaths();
                }
                return gtk4::glib::ControlFlow::Break;
            }

            if *poll_count_clone.borrow() > 400 {
                generating.set(false);
                if let Some(sb) = sb_poll.as_ref() {
                    sb.set_progress(0.0, "", "");
                    sb.set_cancel_action(None);
                }
                return gtk4::glib::ControlFlow::Break;
            }

            if let Some(mut guard) = result_arc_poll.try_lock() {
                if let Some(tp) = guard.take() {
                    if !cancel_poll.load(Ordering::SeqCst) {
                        *out.borrow_mut() = tp;
                        canvas.queue_draw();
                    }

                    generating.set(false);
                    if let Some(sb) = sb_poll.as_ref() {
                        sb.set_progress(0.0, "", "");
                        sb.set_cancel_action(None);
                    }
                    if pending.replace(false) {
                        self_ref.generate_preview_toolpaths();
                    }
                    return gtk4::glib::ControlFlow::Break;
                }
            }

            gtk4::glib::ControlFlow::Continue
        });
    }
}
