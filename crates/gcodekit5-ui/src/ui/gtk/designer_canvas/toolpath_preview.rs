//! Toolpath preview generation for the designer canvas

use super::*;
use gcodekit5_designer::model::{DesignCircle as Circle, DesignerShape, Point, Shape};
use gcodekit5_designer::shapes::OperationType;
use gcodekit5_designer::toolpath::Toolpath;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;

impl DesignerCanvas {
    pub fn generate_preview_toolpaths(&self) {
        if self.preview_generating.get() {
            self.preview_pending.set(true);
            self.preview_cancel.store(true, Ordering::SeqCst);
            return;
        }

        self.preview_generating.set(true);
        self.preview_cancel.store(false, Ordering::SeqCst);

        let started_at = std::time::Instant::now();

        let (shapes, feed_rate, spindle_speed, tool_diameter, cut_depth) = {
            let state = self.state.borrow();
            (
                state.canvas.shapes().cloned().collect::<Vec<_>>(),
                state.tool_settings.feed_rate,
                state.tool_settings.spindle_speed,
                state.tool_settings.tool_diameter,
                state.tool_settings.cut_depth,
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
        let result_arc: Arc<std::sync::Mutex<Option<Vec<Toolpath>>>> =
            Arc::new(std::sync::Mutex::new(None));
        let result_arc_thread = result_arc.clone();

        std::thread::spawn(move || {
            use gcodekit5_designer::toolpath::ToolpathGenerator;
            let mut gen = ToolpathGenerator::new();
            gen.set_feed_rate(feed_rate);
            gen.set_spindle_speed(spindle_speed);
            gen.set_tool_diameter(tool_diameter);
            gen.set_cut_depth(cut_depth);
            gen.set_step_in(tool_diameter * 0.4);

            let mut toolpaths = Vec::new();
            for shape in shapes {
                if cancel.load(Ordering::SeqCst) {
                    return;
                }

                gen.set_pocket_strategy(shape.pocket_strategy);
                gen.set_start_depth(shape.start_depth);
                gen.set_cut_depth(shape.pocket_depth);
                gen.set_step_in(shape.step_in as f64);
                gen.set_raster_fill_ratio(shape.raster_fill_ratio);

                let effective_shape = shape.get_effective_shape();
                let shape_toolpaths = match &effective_shape {
                    Shape::Rectangle(rect) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_rectangle_pocket(
                                rect,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_rectangle_contour(rect, shape.step_down as f64)
                        }
                    }
                    Shape::Circle(circle) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_circle_pocket(
                                circle,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_circle_contour(circle, shape.step_down as f64)
                        }
                    }
                    Shape::Line(line) => gen.generate_line_contour(line, shape.step_down as f64),
                    Shape::Ellipse(ellipse) => {
                        let (x1, y1, x2, y2) = ellipse.bounds();
                        let cx = (x1 + x2) / 2.0;
                        let cy = (y1 + y2) / 2.0;
                        let radius = ((x2 - x1).abs().max((y2 - y1).abs())) / 2.0;
                        let circle = Circle::new(Point::new(cx, cy), radius);
                        gen.generate_circle_contour(&circle, shape.step_down as f64)
                    }
                    Shape::Path(path_shape) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_path_pocket(
                                path_shape,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_path_contour(path_shape, shape.step_down as f64)
                        }
                    }
                    Shape::Text(text) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_text_pocket_toolpath(text, shape.step_down as f64)
                        } else {
                            gen.generate_text_toolpath(text, shape.step_down as f64)
                        }
                    }
                    Shape::Triangle(triangle) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_triangle_pocket(
                                triangle,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_triangle_contour(triangle, shape.step_down as f64)
                        }
                    }
                    Shape::Polygon(polygon) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_polygon_pocket(
                                polygon,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_polygon_contour(polygon, shape.step_down as f64)
                        }
                    }
                    Shape::Gear(gear) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_gear_pocket(
                                gear,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_gear_contour(gear, shape.step_down as f64)
                        }
                    }
                    Shape::Sprocket(sprocket) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_sprocket_pocket(
                                sprocket,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_sprocket_contour(sprocket, shape.step_down as f64)
                        }
                    }
                };
                toolpaths.extend(shape_toolpaths);
                done_shapes_thread.fetch_add(1, Ordering::Relaxed);
            }

            *result_arc_thread.lock().unwrap_or_else(|p| p.into_inner()) = Some(toolpaths);
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

            if let Ok(mut guard) = result_arc_poll.try_lock() {
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
