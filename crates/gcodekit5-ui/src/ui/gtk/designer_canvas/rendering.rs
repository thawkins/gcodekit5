//! Rendering and drawing methods for the designer canvas

use super::*;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::{DesignerShape, Point, Shape};
use gcodekit5_designer::toolpath::{Toolpath, ToolpathSegmentType};

impl DesignerCanvas {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw(
        cr: &gtk4::cairo::Context,
        state: &DesignerState,
        width: f64,
        height: f64,
        mouse_pos: (f64, f64),
        preview_start: Option<(f64, f64)>,
        preview_current: Option<(f64, f64)>,
        polyline_points: &[Point],
        preview_shapes: &[Shape],
        toolpaths: &[Toolpath],
        device_bounds: (f64, f64, f64, f64),
        style_context: &gtk4::StyleContext,
        grid_major_line_width: f64,
        grid_minor_line_width: f64,
    ) {
        // Background handled by CSS

        let fg_color = style_context.color();
        let accent_color = style_context
            .lookup_color("accent_color")
            .unwrap_or(gtk4::gdk::RGBA::new(0.0, 0.5, 1.0, 1.0));
        let success_color = style_context
            .lookup_color("success_color")
            .unwrap_or(gtk4::gdk::RGBA::new(0.0, 0.8, 0.0, 1.0));
        let warning_color = style_context
            .lookup_color("warning_color")
            .unwrap_or(gtk4::gdk::RGBA::new(1.0, 1.0, 0.0, 1.0));
        let error_color = style_context
            .lookup_color("error_color")
            .unwrap_or(gtk4::gdk::RGBA::new(1.0, 0.0, 0.0, 1.0));

        // Setup coordinate system
        // Designer uses Y-up (Cartesian), Cairo uses Y-down

        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();

        // Transform to bottom-left, flip Y, then apply pan and zoom
        // Origin is bottom-left of the widget
        cr.translate(0.0, height);
        cr.scale(1.0, -1.0);

        // Apply Pan (in screen pixels, but Y is flipped so +Y pan moves up)
        cr.translate(pan_x, pan_y);

        // Apply Zoom
        cr.scale(zoom, zoom);

        // Draw Grid
        if state.show_grid {
            Self::draw_grid(
                cr,
                width,
                height,
                state.grid_spacing_mm.max(0.1),
                &fg_color,
                zoom,
                grid_major_line_width,
                grid_minor_line_width,
            );
        }

        // Draw Device Bounds
        let (min_x, min_y, max_x, max_y) = device_bounds;
        let width = max_x - min_x;
        let height = max_y - min_y;

        let _ = cr.save();
        cr.set_source_rgb(0.0, 0.0, 1.0); // Blue
        cr.set_line_width(2.0 / zoom); // 2px wide on screen
        cr.rectangle(min_x, min_y, width, height);
        let _ = cr.stroke();
        let _ = cr.restore();

        // Draw Origin Crosshair
        Self::draw_origin_crosshair(cr, zoom);

        // Draw Toolpaths (if enabled)
        if state.show_toolpaths {
            let _ = cr.save();
            cr.set_line_width(2.0 / zoom); // Constant screen width

            for toolpath in toolpaths {
                for segment in &toolpath.segments {
                    match segment.segment_type {
                        ToolpathSegmentType::RapidMove => {
                            cr.set_source_rgba(
                                warning_color.red() as f64,
                                warning_color.green() as f64,
                                warning_color.blue() as f64,
                                0.5,
                            );
                            cr.set_dash(&[2.0 / zoom, 2.0 / zoom], 0.0);
                            cr.move_to(segment.start.x, segment.start.y);
                            cr.line_to(segment.end.x, segment.end.y);
                            let _ = cr.stroke();
                        }
                        ToolpathSegmentType::LinearMove => {
                            cr.set_source_rgba(
                                success_color.red() as f64,
                                success_color.green() as f64,
                                success_color.blue() as f64,
                                0.7,
                            );
                            cr.set_dash(&[], 0.0);
                            cr.move_to(segment.start.x, segment.start.y);
                            cr.line_to(segment.end.x, segment.end.y);
                            let _ = cr.stroke();
                        }
                        ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                            cr.set_source_rgba(
                                success_color.red() as f64,
                                success_color.green() as f64,
                                success_color.blue() as f64,
                                0.7,
                            );
                            cr.set_dash(&[], 0.0);

                            if let Some(center) = segment.center {
                                let radius = center.distance_to(&segment.start);
                                let angle1 =
                                    (segment.start.y - center.y).atan2(segment.start.x - center.x);
                                let angle2 =
                                    (segment.end.y - center.y).atan2(segment.end.x - center.x);

                                cr.move_to(segment.start.x, segment.start.y); // Ensure we start at correct point
                                                                              // Note: Cairo adds a line from current point to start of arc if they differ.
                                                                              // But we just moved there.

                                if segment.segment_type == ToolpathSegmentType::ArcCW {
                                    cr.arc_negative(center.x, center.y, radius, angle1, angle2);
                                } else {
                                    cr.arc(center.x, center.y, radius, angle1, angle2);
                                }
                            } else {
                                cr.move_to(segment.start.x, segment.start.y);
                                cr.line_to(segment.end.x, segment.end.y);
                            }
                            let _ = cr.stroke();
                        }
                    }
                }
            }

            let _ = cr.restore();
        }

        // Draw polyline in progress
        if !polyline_points.is_empty() {
            let _ = cr.save();
            cr.set_source_rgba(
                accent_color.red() as f64,
                accent_color.green() as f64,
                accent_color.blue() as f64,
                1.0,
            );
            cr.set_line_width(2.0 / zoom);

            // Draw existing segments
            if let Some(first) = polyline_points.first() {
                cr.move_to(first.x, first.y);
                for p in polyline_points.iter().skip(1) {
                    cr.line_to(p.x, p.y);
                }

                // Draw rubber band to mouse
                cr.line_to(mouse_pos.0, mouse_pos.1);
            }

            let _ = cr.stroke();

            // Draw points
            for p in polyline_points {
                cr.arc(p.x, p.y, 3.0 / zoom, 0.0, 2.0 * std::f64::consts::PI);
                let _ = cr.fill();
            }

            let _ = cr.restore();
        }

        let selected_count = state
            .canvas
            .shape_store
            .iter()
            .filter(|o| o.selected)
            .count();

        // Draw Shapes
        for obj in state.canvas.shape_store.iter() {
            // 1. Draw Base Shape
            let _ = cr.save();

            if obj.selected {
                cr.set_source_rgba(
                    error_color.red() as f64,
                    error_color.green() as f64,
                    error_color.blue() as f64,
                    1.0,
                );
                cr.set_line_width(3.0 / zoom);
            } else if obj.group_id.is_some() {
                cr.set_source_rgba(
                    success_color.red() as f64,
                    success_color.green() as f64,
                    success_color.blue() as f64,
                    1.0,
                );
                cr.set_line_width(2.0 / zoom);
            } else {
                cr.set_source_rgba(
                    fg_color.red() as f64,
                    fg_color.green() as f64,
                    fg_color.blue() as f64,
                    fg_color.alpha() as f64,
                );
                cr.set_line_width(2.0 / zoom);
            }

            Self::draw_shape_geometry(cr, &obj.shape);

            // Draw resize handles on BASE shape
            if selected_count <= 1 && obj.selected {
                let bounds = Self::selection_bounds(&obj.shape);
                Self::draw_resize_handles(cr, &bounds, zoom, &accent_color);
            }

            let _ = cr.restore();

            // 2. Draw Effective Shape (Yellow Overlay) if modified
            if obj.offset.abs() > 1e-6 || obj.fillet.abs() > 1e-6 || obj.chamfer.abs() > 1e-6 {
                let _ = cr.save();
                cr.set_source_rgba(
                    warning_color.red() as f64,
                    warning_color.green() as f64,
                    warning_color.blue() as f64,
                    1.0,
                );
                cr.set_line_width(2.0 / zoom);
                Self::draw_shape_geometry(cr, &obj.get_effective_shape());
                let _ = cr.restore();
            }
        }

        // Draw Preview Shapes (e.g. for offset/fillet) in yellow
        for shape in preview_shapes {
            let _ = cr.save();
            cr.set_source_rgba(
                warning_color.red() as f64,
                warning_color.green() as f64,
                warning_color.blue() as f64,
                1.0,
            );
            cr.set_line_width(2.0 / zoom);
            Self::draw_shape_geometry(cr, shape);
            let _ = cr.restore();
        }

        if selected_count > 1 {
            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;

            for obj in state.canvas.shape_store.iter().filter(|o| o.selected) {
                let (x1, y1, x2, y2) = Self::selection_bounds(&obj.shape);
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
            }

            if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
                let bounds = (min_x, min_y, max_x, max_y);
                Self::draw_resize_handles(cr, &bounds, zoom, &accent_color);
            }
        }

        // Draw preview marquee if creating a shape (only when no shapes are selected)
        if selected_count == 0 {
            if let (Some(start), Some(current)) = (preview_start, preview_current) {
                let _ = cr.save();

                // Draw dashed preview outline
                cr.set_source_rgba(
                    accent_color.red() as f64,
                    accent_color.green() as f64,
                    accent_color.blue() as f64,
                    0.7,
                );
                cr.set_line_width(2.0 / zoom);
                cr.set_dash(&[5.0 / zoom, 5.0 / zoom], 0.0); // Dashed line

                // Draw bounding box for the preview
                let x1 = start.0.min(current.0);
                let y1 = start.1.min(current.1);
                let x2 = start.0.max(current.0);
                let y2 = start.1.max(current.1);

                cr.rectangle(x1, y1, x2 - x1, y2 - y1);
                let _ = cr.stroke();

                let _ = cr.restore();
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_grid(
        cr: &gtk4::cairo::Context,
        width: f64,
        height: f64,
        grid_spacing: f64,
        fg_color: &gtk4::gdk::RGBA,
        zoom: f64,
        major_line_width: f64,
        minor_line_width: f64,
    ) {
        let _ = cr.save();

        let minor_spacing = grid_spacing / 5.0;

        // Get current transform to find canvas bounds
        let matrix = cr.matrix();
        let x0 = -matrix.x0() / matrix.xx();
        let x1 = (width - matrix.x0()) / matrix.xx();
        let y0 = -matrix.y0() / matrix.yy();
        let y1 = (height - matrix.y0()) / matrix.yy();

        // Minor grid lines (lighter) - configurable constant width
        cr.set_source_rgba(
            fg_color.red() as f64,
            fg_color.green() as f64,
            fg_color.blue() as f64,
            0.2,
        );
        cr.set_line_width(minor_line_width / zoom);

        // Vertical minor grid lines
        let start_x = (x0 / minor_spacing).floor() * minor_spacing;
        let mut x = start_x;
        while x <= x1 {
            if ((x / grid_spacing).round() - x / grid_spacing).abs() > 0.01 {
                cr.move_to(x, y1);
                cr.line_to(x, y0);
                let _ = cr.stroke();
            }
            x += minor_spacing;
        }

        // Horizontal minor grid lines
        let start_y = (y1 / minor_spacing).floor() * minor_spacing;
        let mut y = start_y;
        while y <= y0 {
            if ((y / grid_spacing).round() - y / grid_spacing).abs() > 0.01 {
                cr.move_to(x0, y);
                cr.line_to(x1, y);
                let _ = cr.stroke();
            }
            y += minor_spacing;
        }

        // Major grid lines (darker) - configurable constant width
        cr.set_source_rgba(
            fg_color.red() as f64,
            fg_color.green() as f64,
            fg_color.blue() as f64,
            0.4,
        );
        cr.set_line_width(major_line_width / zoom);

        // Vertical major grid lines
        x = (x0 / grid_spacing).floor() * grid_spacing;
        while x <= x1 {
            cr.move_to(x, y1);
            cr.line_to(x, y0);
            let _ = cr.stroke();
            x += grid_spacing;
        }

        // Horizontal major grid lines
        y = (y1 / grid_spacing).floor() * grid_spacing;
        while y <= y0 {
            cr.move_to(x0, y);
            cr.line_to(x1, y);
            let _ = cr.stroke();
            y += grid_spacing;
        }

        // Draw axes (thicker, darker) - only if they're visible - uses major line width
        cr.set_source_rgba(
            fg_color.red() as f64,
            fg_color.green() as f64,
            fg_color.blue() as f64,
            0.8,
        );
        cr.set_line_width(major_line_width / zoom);

        // X-axis (y=0)
        if y1 <= 0.0 && y0 >= 0.0 {
            cr.move_to(x0, 0.0);
            cr.line_to(x1, 0.0);
        }

        // Y-axis (x=0)
        if x0 <= 0.0 && x1 >= 0.0 {
            cr.move_to(0.0, y1);
            cr.line_to(0.0, y0);
        }
        let _ = cr.stroke();

        let _ = cr.restore();
    }

    pub(super) fn selection_bounds(shape: &Shape) -> (f64, f64, f64, f64) {
        fn rotate_xy(x: f64, y: f64, cx: f64, cy: f64, angle: f64) -> (f64, f64) {
            let s = angle.sin();
            let c = angle.cos();
            let dx = x - cx;
            let dy = y - cy;
            (cx + dx * c - dy * s, cy + dx * s + dy * c)
        }

        match shape {
            Shape::Rectangle(rect) => {
                if rect.rotation.abs() <= 1e-9 {
                    return rect.bounds();
                }

                let cx = rect.center.x;
                let cy = rect.center.y;
                let hw = rect.width / 2.0;
                let hh = rect.height / 2.0;
                let corners = [
                    (cx - hw, cy - hh),
                    (cx + hw, cy - hh),
                    (cx + hw, cy + hh),
                    (cx - hw, cy + hh),
                ];

                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;

                for (x, y) in corners {
                    let (rx, ry) = rotate_xy(x, y, cx, cy, rect.rotation.to_radians());
                    min_x = min_x.min(rx);
                    min_y = min_y.min(ry);
                    max_x = max_x.max(rx);
                    max_y = max_y.max(ry);
                }

                (min_x, min_y, max_x, max_y)
            }
            Shape::Circle(circle) => circle.bounds(),
            Shape::Line(line) => line.bounds(),
            Shape::Ellipse(ellipse) => {
                if ellipse.rotation.abs() <= 1e-9 {
                    return ellipse.bounds();
                }

                // Axis-aligned bounding box of a rotated ellipse.
                let theta = ellipse.rotation.to_radians();
                let cos_t = theta.cos();
                let sin_t = theta.sin();
                let half_w = ((ellipse.rx * cos_t).powi(2) + (ellipse.ry * sin_t).powi(2)).sqrt();
                let half_h = ((ellipse.rx * sin_t).powi(2) + (ellipse.ry * cos_t).powi(2)).sqrt();

                (
                    ellipse.center.x - half_w,
                    ellipse.center.y - half_h,
                    ellipse.center.x + half_w,
                    ellipse.center.y + half_h,
                )
            }
            Shape::Path(path_shape) => {
                if path_shape.rotation.abs() <= 1e-9 {
                    return path_shape.bounds();
                }

                // Match the draw behavior: rotate about the path's AABB center.
                let (x1, y1, x2, y2) = path_shape.bounds();
                let cx = (x1 + x2) / 2.0;
                let cy = (y1 + y2) / 2.0;
                let corners = [(x1, y1), (x2, y1), (x2, y2), (x1, y2)];

                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;

                for (x, y) in corners {
                    let (rx, ry) = rotate_xy(x, y, cx, cy, path_shape.rotation.to_radians());
                    min_x = min_x.min(rx);
                    min_y = min_y.min(ry);
                    max_x = max_x.max(rx);
                    max_y = max_y.max(ry);
                }

                (min_x, min_y, max_x, max_y)
            }
            Shape::Text(text) => text.bounds(),
            Shape::Triangle(triangle) => triangle.bounds(),
            Shape::Polygon(polygon) => polygon.bounds(),
            Shape::Gear(gear) => gear.bounds(),
            Shape::Sprocket(sprocket) => sprocket.bounds(),
        }
    }

    fn draw_shape_geometry(cr: &gtk4::cairo::Context, shape: &Shape) {
        match shape {
            Shape::Rectangle(rect) => {
                let _ = cr.save();
                cr.translate(rect.center.x, rect.center.y);
                if rect.rotation.abs() > 1e-9 {
                    cr.rotate(rect.rotation.to_radians());
                }

                let effective_radius = rect.effective_corner_radius();
                if effective_radius > 0.001 {
                    let x = -rect.width / 2.0;
                    let y = -rect.height / 2.0;
                    let w = rect.width;
                    let h = rect.height;
                    let r = effective_radius.min(w / 2.0).min(h / 2.0);
                    let pi = std::f64::consts::PI;

                    cr.new_sub_path();
                    // Start at right edge, bottom of TR corner
                    cr.arc(x + w - r, y + h - r, r, 0.0, 0.5 * pi); // TR
                    cr.arc(x + r, y + h - r, r, 0.5 * pi, pi); // TL
                    cr.arc(x + r, y + r, r, pi, 1.5 * pi); // BL
                    cr.arc(x + w - r, y + r, r, 1.5 * pi, 2.0 * pi); // BR
                    cr.close_path();
                    let _ = cr.stroke();
                } else {
                    let x = -rect.width / 2.0;
                    let y = -rect.height / 2.0;
                    cr.rectangle(x, y, rect.width, rect.height);
                    let _ = cr.stroke();
                }

                let _ = cr.restore();
            }
            Shape::Circle(circle) => {
                cr.arc(
                    circle.center.x,
                    circle.center.y,
                    circle.radius,
                    0.0,
                    2.0 * std::f64::consts::PI,
                );
                let _ = cr.stroke();
            }
            Shape::Line(line) => {
                cr.move_to(line.start.x, line.start.y);
                cr.line_to(line.end.x, line.end.y);
                let _ = cr.stroke();
            }
            Shape::Ellipse(ellipse) => {
                let _ = cr.save();
                cr.translate(ellipse.center.x, ellipse.center.y);
                if ellipse.rotation.abs() > 1e-9 {
                    cr.rotate(ellipse.rotation.to_radians());
                }
                let base_width = cr.line_width();
                let scale_factor = ellipse.rx.abs().max(ellipse.ry.abs()).max(1e-6);
                cr.set_line_width(base_width / scale_factor);
                cr.scale(ellipse.rx, ellipse.ry);
                cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * std::f64::consts::PI);
                let _ = cr.stroke();
                let _ = cr.restore();
            }
            Shape::Path(path_shape) => {
                let (x1, y1, x2, y2) = path_shape.bounds();
                let cx = (x1 + x2) / 2.0;
                let cy = (y1 + y2) / 2.0;

                let _ = cr.save();
                if path_shape.rotation.abs() > 1e-9 {
                    cr.translate(cx, cy);
                    cr.rotate(path_shape.rotation.to_radians());
                    cr.translate(-cx, -cy);
                }

                cr.new_path();
                // Iterate lyon path
                for event in path_shape.render().iter() {
                    match event {
                        lyon::path::Event::Begin { at } => {
                            cr.move_to(at.x as f64, at.y as f64);
                        }
                        lyon::path::Event::Line { from: _, to } => {
                            cr.line_to(to.x as f64, to.y as f64);
                        }
                        lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                            // Cairo doesn't have quadratic, convert to cubic.
                            // We use current point as 'from'.
                            let (x0, y0) = cr.current_point().unwrap_or((0.0, 0.0));
                            let x1 = x0 + (2.0 / 3.0) * (ctrl.x as f64 - x0);
                            let y1 = y0 + (2.0 / 3.0) * (ctrl.y as f64 - y0);
                            let x2 = to.x as f64 + (2.0 / 3.0) * (ctrl.x as f64 - to.x as f64);
                            let y2 = to.y as f64 + (2.0 / 3.0) * (ctrl.y as f64 - to.y as f64);
                            cr.curve_to(x1, y1, x2, y2, to.x as f64, to.y as f64);
                        }
                        lyon::path::Event::Cubic {
                            from: _,
                            ctrl1,
                            ctrl2,
                            to,
                        } => {
                            cr.curve_to(
                                ctrl1.x as f64,
                                ctrl1.y as f64,
                                ctrl2.x as f64,
                                ctrl2.y as f64,
                                to.x as f64,
                                to.y as f64,
                            );
                        }
                        lyon::path::Event::End {
                            last: _,
                            first: _,
                            close,
                        } => {
                            if close {
                                cr.close_path();
                            }
                        }
                    }
                }
                let _ = cr.stroke();
                let _ = cr.restore();
            }
            Shape::Text(text) => {
                // Basic text placeholder
                let _ = cr.save();
                // Rotate around text bounds center, then flip Y for text rendering.
                let (x1, y1, x2, y2) = text.bounds();
                let cx = (x1 + x2) / 2.0;
                let cy = (y1 + y2) / 2.0;
                // Use negative angle because we flip Y after rotation.
                // Note: text.rotation is in degrees, convert to radians for Cairo
                let angle = -text.rotation.to_radians();

                cr.translate(cx, cy);
                cr.rotate(angle);
                cr.translate(-cx, -cy);

                // Flip Y back for text so it's not upside down
                cr.translate(text.x, text.y);
                cr.scale(1.0, -1.0);
                let slant = if text.italic {
                    gtk4::cairo::FontSlant::Italic
                } else {
                    gtk4::cairo::FontSlant::Normal
                };
                let weight = if text.bold {
                    gtk4::cairo::FontWeight::Bold
                } else {
                    gtk4::cairo::FontWeight::Normal
                };
                cr.select_font_face(&text.font_family, slant, weight);
                cr.set_font_size(text.font_size);
                let _ = cr.show_text(&text.text);
                let _ = cr.restore();
            }
            Shape::Triangle(triangle) => {
                let path = triangle.render();
                cr.new_path();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => cr.move_to(at.x as f64, at.y as f64),
                        lyon::path::Event::Line { to, .. } => cr.line_to(to.x as f64, to.y as f64),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                cr.close_path();
                            }
                        }
                        _ => {}
                    }
                }
                let _ = cr.stroke();
            }
            Shape::Polygon(polygon) => {
                let path = polygon.render();
                cr.new_path();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => cr.move_to(at.x as f64, at.y as f64),
                        lyon::path::Event::Line { to, .. } => cr.line_to(to.x as f64, to.y as f64),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                cr.close_path();
                            }
                        }
                        _ => {}
                    }
                }
                let _ = cr.stroke();
            }
            Shape::Gear(gear) => {
                let path = gear.render();
                cr.new_path();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => cr.move_to(at.x as f64, at.y as f64),
                        lyon::path::Event::Line { to, .. } => cr.line_to(to.x as f64, to.y as f64),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                cr.close_path();
                            }
                        }
                        _ => {}
                    }
                }
                let _ = cr.stroke();
            }
            Shape::Sprocket(sprocket) => {
                let path = sprocket.render();
                cr.new_path();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => cr.move_to(at.x as f64, at.y as f64),
                        lyon::path::Event::Line { to, .. } => cr.line_to(to.x as f64, to.y as f64),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                cr.close_path();
                            }
                        }
                        _ => {}
                    }
                }
                let _ = cr.stroke();
            }
        }
    }

    fn draw_origin_crosshair(cr: &gtk4::cairo::Context, zoom: f64) {
        let _ = cr.save();

        // Draw Origin Axes (Full World Extent)
        let extent = core_constants::WORLD_EXTENT_MM;
        cr.set_line_width(1.0 / zoom); // Thinner line for full axes

        // X Axis Red
        cr.set_source_rgb(1.0, 0.0, 0.0);
        cr.move_to(-extent, 0.0);
        cr.line_to(extent, 0.0);
        let _ = cr.stroke();

        // Y Axis Green
        cr.set_source_rgb(0.0, 1.0, 0.0);
        cr.move_to(0.0, -extent);
        cr.line_to(0.0, extent);
        let _ = cr.stroke();

        let _ = cr.restore();
    }

    pub(super) fn get_resize_handle_at(
        &self,
        x: f64,
        y: f64,
        bounds: &(f64, f64, f64, f64),
        zoom: f64,
    ) -> Option<ResizeHandle> {
        // Handles are drawn as ~8 screen pixels; in canvas units that's 8/zoom.
        let zoom = zoom.max(1e-6);
        let handle_size = 8.0 / zoom;
        let handle_tolerance = handle_size / 2.0;

        let (min_x, min_y, max_x, max_y) = *bounds;

        let corners = [
            (min_x, max_y, ResizeHandle::TopLeft), // Top-left (Y-up coords)
            (max_x, max_y, ResizeHandle::TopRight), // Top-right
            (min_x, min_y, ResizeHandle::BottomLeft), // Bottom-left
            (max_x, min_y, ResizeHandle::BottomRight), // Bottom-right
        ];

        for (cx, cy, handle) in corners {
            let dx = x - cx;
            let dy = y - cy;
            if dx.abs() <= handle_tolerance && dy.abs() <= handle_tolerance {
                return Some(handle);
            }
        }

        None
    }

    pub(super) fn apply_resize(
        &self,
        handle: ResizeHandle,
        _shape_id: u64,
        current_x: f64,
        current_y: f64,
        shift_pressed: bool,
    ) {
        let orig_bounds = match *self.resize_original_bounds.borrow() {
            Some(b) => b,
            None => return,
        };

        let start = match *self.creation_start.borrow() {
            Some(s) => s,
            None => return,
        };

        let (orig_x, orig_y, orig_width, orig_height) = orig_bounds;

        // Calculate deltas
        let mut dx = current_x - start.0;
        let mut dy = current_y - start.1;

        if shift_pressed {
            // Maintain aspect ratio
            let ratio = if orig_height.abs() > 0.001 {
                orig_width / orig_height
            } else {
                1.0
            };

            // Calculate "natural" new dimensions based on mouse position
            let natural_w = match handle {
                ResizeHandle::TopLeft | ResizeHandle::BottomLeft => orig_width - dx,
                ResizeHandle::TopRight | ResizeHandle::BottomRight => orig_width + dx,
            };

            let natural_h = match handle {
                ResizeHandle::TopLeft | ResizeHandle::TopRight => orig_height + dy,
                ResizeHandle::BottomLeft | ResizeHandle::BottomRight => orig_height - dy,
            };

            // Determine which dimension to follow (use the one with larger relative change)
            let w_scale = (natural_w / orig_width).abs();
            let h_scale = (natural_h / orig_height).abs();

            let (new_w, new_h) = if w_scale > h_scale {
                // Width is dominant, adjust height
                (natural_w, natural_w / ratio)
            } else {
                // Height is dominant, adjust width
                (natural_h * ratio, natural_h)
            };

            // Back-calculate dx and dy to achieve new_w and new_h
            match handle {
                ResizeHandle::TopLeft => {
                    dx = orig_width - new_w;
                    dy = new_h - orig_height;
                }
                ResizeHandle::TopRight => {
                    dx = new_w - orig_width;
                    dy = new_h - orig_height;
                }
                ResizeHandle::BottomLeft => {
                    dx = orig_width - new_w;
                    dy = orig_height - new_h;
                }
                ResizeHandle::BottomRight => {
                    dx = new_w - orig_width;
                    dy = orig_height - new_h;
                }
            }
        }

        // Calculate new bounds based on which handle is being dragged
        let (_new_x, _new_y, new_width, new_height) = match handle {
            ResizeHandle::TopLeft => {
                // Moving top-left corner (min_x, max_y in Y-up)
                let new_min_x = orig_x + dx;
                let new_max_y = orig_y + orig_height + dy;
                let new_width = (orig_x + orig_width) - new_min_x;
                let new_height = new_max_y - orig_y;
                (new_min_x, orig_y, new_width, new_height)
            }
            ResizeHandle::TopRight => {
                // Moving top-right corner (max_x, max_y in Y-up)
                let new_max_x = orig_x + orig_width + dx;
                let new_max_y = orig_y + orig_height + dy;
                let new_width = new_max_x - orig_x;
                let new_height = new_max_y - orig_y;
                (orig_x, orig_y, new_width, new_height)
            }
            ResizeHandle::BottomLeft => {
                // Moving bottom-left corner (min_x, min_y in Y-up)
                let new_min_x = orig_x + dx;
                let new_min_y = orig_y + dy;
                let new_width = (orig_x + orig_width) - new_min_x;
                let new_height = (orig_y + orig_height) - new_min_y;
                (new_min_x, new_min_y, new_width, new_height)
            }
            ResizeHandle::BottomRight => {
                // Moving bottom-right corner (max_x, min_y in Y-up)
                let new_max_x = orig_x + orig_width + dx;
                let new_min_y = orig_y + dy;
                let new_width = new_max_x - orig_x;
                let new_height = (orig_y + orig_height) - new_min_y;
                (orig_x, new_min_y, new_width, new_height)
            }
        };

        // Apply minimum size constraints
        if new_width.abs() < 5.0 || new_height.abs() < 5.0 {
            return;
        }

        // Prevent flips (negative dimensions) which would invert shapes.
        if new_width <= 0.0 || new_height <= 0.0 {
            return;
        }

        let sx = if orig_width.abs() > 1e-6 {
            new_width / orig_width
        } else {
            1.0
        };
        let sy = if orig_height.abs() > 1e-6 {
            new_height / orig_height
        } else {
            1.0
        };

        let (anchor_x, anchor_y) = match handle {
            ResizeHandle::TopLeft => (orig_x + orig_width, orig_y),
            ResizeHandle::TopRight => (orig_x, orig_y),
            ResizeHandle::BottomLeft => (orig_x + orig_width, orig_y + orig_height),
            ResizeHandle::BottomRight => (orig_x, orig_y + orig_height),
        };

        // Update the shape
        let mut state = self.state.borrow_mut();

        // Restore original shapes first so drag updates don't compound transforms.
        // (Without this, we repeatedly multiply already-scaled dimensions and the selection
        // shrinks/grows exponentially.)
        if let Some(originals) = self.resize_original_shapes.borrow().as_ref() {
            for (id, original_shape) in originals {
                if let Some(obj) = state.canvas.shape_store.get_mut(*id) {
                    if obj.selected {
                        obj.shape = original_shape.clone();
                    }
                }
            }
        }

        // Apply scaling to all selected shapes (single or multiple)
        // This ensures consistent behavior for rotated shapes where AABB resizing
        // should be treated as a scaling operation relative to the anchor point.
        let anchor = Point::new(anchor_x, anchor_y);
        for obj in state.canvas.shape_store.iter_mut() {
            if !obj.selected {
                continue;
            }
            match &mut obj.shape {
                Shape::Rectangle(rect) => {
                    rect.center.x = anchor.x + (rect.center.x - anchor.x) * sx;
                    rect.center.y = anchor.y + (rect.center.y - anchor.y) * sy;
                    rect.width *= sx.abs();
                    rect.height *= sy.abs();

                    // Only scale corner_radius if not in slot mode
                    // (slot mode calculates radius dynamically)
                    if !rect.is_slot {
                        rect.corner_radius *= sx.abs().min(sy.abs());
                    }
                }
                Shape::Circle(circle) => {
                    circle.center.x = anchor.x + (circle.center.x - anchor.x) * sx;
                    circle.center.y = anchor.y + (circle.center.y - anchor.y) * sy;
                    let s = sx.abs().min(sy.abs());
                    circle.radius *= s;
                }
                Shape::Ellipse(ellipse) => {
                    ellipse.center.x = anchor.x + (ellipse.center.x - anchor.x) * sx;
                    ellipse.center.y = anchor.y + (ellipse.center.y - anchor.y) * sy;
                    ellipse.rx *= sx.abs();
                    ellipse.ry *= sy.abs();
                }
                Shape::Line(line) => {
                    line.start.x = anchor.x + (line.start.x - anchor.x) * sx;
                    line.start.y = anchor.y + (line.start.y - anchor.y) * sy;
                    line.end.x = anchor.x + (line.end.x - anchor.x) * sx;
                    line.end.y = anchor.y + (line.end.y - anchor.y) * sy;
                }
                Shape::Path(path_shape) => {
                    path_shape.scale(sx, sy, anchor);
                }
                Shape::Text(text) => {
                    text.scale(sx, sy, anchor);
                }
                Shape::Triangle(triangle) => {
                    triangle.center.x = anchor.x + (triangle.center.x - anchor.x) * sx;
                    triangle.center.y = anchor.y + (triangle.center.y - anchor.y) * sy;
                    triangle.width *= sx.abs();
                    triangle.height *= sy.abs();
                }
                Shape::Polygon(polygon) => {
                    polygon.center.x = anchor.x + (polygon.center.x - anchor.x) * sx;
                    polygon.center.y = anchor.y + (polygon.center.y - anchor.y) * sy;
                    let s = sx.abs().min(sy.abs());
                    polygon.radius *= s;
                }
                Shape::Gear(gear) => {
                    gear.center.x = anchor.x + (gear.center.x - anchor.x) * sx;
                    gear.center.y = anchor.y + (gear.center.y - anchor.y) * sy;
                    let s = sx.abs().min(sy.abs());
                    gear.module *= s;
                }
                Shape::Sprocket(sprocket) => {
                    sprocket.center.x = anchor.x + (sprocket.center.x - anchor.x) * sx;
                    sprocket.center.y = anchor.y + (sprocket.center.y - anchor.y) * sy;
                    let s = sx.abs().min(sy.abs());
                    sprocket.pitch *= s;
                    sprocket.roller_diameter *= s;
                }
            }
        }
    }

    fn draw_resize_handles(
        cr: &gtk4::cairo::Context,
        bounds: &(f64, f64, f64, f64),
        zoom: f64,
        accent_color: &gtk4::gdk::RGBA,
    ) {
        let handle_size = 8.0 / zoom;
        let half_size = handle_size / 2.0;

        let (min_x, min_y, max_x, max_y) = *bounds;

        let _ = cr.save();

        // Draw handles at corners
        let corners = [
            (min_x, max_y), // Top-left (Y-up)
            (max_x, max_y), // Top-right
            (min_x, min_y), // Bottom-left
            (max_x, min_y), // Bottom-right
        ];

        for (cx, cy) in corners {
            // Draw white fill
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.rectangle(cx - half_size, cy - half_size, handle_size, handle_size);
            let _ = cr.fill();

            // Draw accent border
            cr.set_source_rgba(
                accent_color.red() as f64,
                accent_color.green() as f64,
                accent_color.blue() as f64,
                accent_color.alpha() as f64,
            );
            cr.set_line_width(2.0 / zoom);
            cr.rectangle(cx - half_size, cy - half_size, handle_size, handle_size);
            let _ = cr.stroke();
        }

        let _ = cr.restore();
    }
}
