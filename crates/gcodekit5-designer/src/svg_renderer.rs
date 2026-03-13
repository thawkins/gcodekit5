//! SVG-based canvas renderer for designer shapes
//! Renders shapes as SVG path commands for display
//! Features:
//! - Bright yellow crosshair at world origin (0,0)
//! - Shape rendering with selection indicators
//! - Viewport-based coordinate transformation

use crate::model::DesignerShape;
use crate::{font_manager, Canvas};
use rusttype::{point as rt_point, OutlineBuilder, Scale};

/// Render crosshair at origin (0,0) as SVG path
pub fn render_crosshair(canvas: &Canvas, width: u32, height: u32) -> String {
    let viewport = canvas.viewport();

    // Convert world origin to screen coordinates
    let (origin_x, origin_y) = viewport.world_to_pixel(0.0, 0.0);

    let mut path = String::new();

    // Horizontal line (X axis) - only check if within reasonable bounds
    // Allow a small buffer outside canvas for visibility
    if origin_y >= -10.0 && origin_y <= (height as f64 + 10.0) {
        path.push_str(&format!("M 0 {} L {} {} ", origin_y, width, origin_y));
    }

    // Vertical line (Y axis) - only check if within reasonable bounds
    if origin_x >= -10.0 && origin_x <= (width as f64 + 10.0) {
        path.push_str(&format!("M {} 0 L {} {} ", origin_x, origin_x, height));
    }

    path
}

/// Render grid as SVG path commands
pub fn render_grid(canvas: &Canvas, width: u32, height: u32) -> (String, f64) {
    let viewport = canvas.viewport();
    let mut path = String::new();
    const MAX_ITERATIONS: usize = 100000;
    const GRID_MAJOR_STEP_MM: f64 = 10.0;

    // Calculate the world coordinate range needed to fill entire viewport
    // Add extra margin to ensure full coverage
    let margin_pixels = 500.0;
    let top_left = viewport.pixel_to_world(-margin_pixels, -margin_pixels);
    let bottom_right =
    viewport.pixel_to_world(width as f64 + margin_pixels, height as f64 + margin_pixels);

    let world_left = top_left.x.min(bottom_right.x);
    let world_right = top_left.x.max(bottom_right.x);
    let world_bottom = top_left.y.min(bottom_right.y);
    let world_top = top_left.y.max(bottom_right.y);

    let world_width = world_right - world_left;
    let world_height = world_top - world_bottom;

    // Adaptive grid spacing
    // Start with 10mm, increase by 10x if too dense
    let mut step = GRID_MAJOR_STEP_MM;
    while (world_width / step) > 100.0 || (world_height / step) > 100.0 {
        step *= 10.0;
    }

    // Round to nearest grid line, ensuring we cover the full range
    let start_x = (world_left / step).floor() * step;
    let end_x = (world_right / step).ceil() * step;

    let start_y = (world_bottom / step).floor() * step;
    let end_y = (world_top / step).ceil() * step;

    // Draw vertical grid lines
    let mut x = start_x;
    let mut iterations = 0;
    while x <= end_x && iterations < MAX_ITERATIONS {
        let (screen_x, _) = viewport.world_to_pixel(x, 0.0);
        // Draw line across full height, no need to clip
        path.push_str(&format!("M {} 0 L {} {} ", screen_x, screen_x, height));
        x += step;
        iterations += 1;
    }

    // Draw horizontal grid lines
    let mut y = start_y;
    iterations = 0;
    while y <= end_y && iterations < MAX_ITERATIONS {
        let (_, screen_y) = viewport.world_to_pixel(0.0, y);
        // Draw line across full width, no need to clip
        path.push_str(&format!("M 0 {} L {} {} ", screen_y, width, screen_y));
        y += step;
        iterations += 1;
    }

    (path, step)
}

/// Render origin marker at (0,0) as yellow cross
pub fn render_origin(canvas: &Canvas, width: u32, height: u32) -> String {
    let viewport = canvas.viewport();
    let (origin_x, origin_y) = viewport.world_to_pixel(0.0, 0.0);

    let mut path = String::new();

    // Vertical line (full height)
    path.push_str(&format!("M {} 0 L {} {} ", origin_x, origin_x, height));

    // Horizontal line (full width)
    path.push_str(&format!("M 0 {} L {} {} ", origin_y, width, origin_y));

    path
}

/// Render all shapes as SVG path
pub fn render_shapes(canvas: &Canvas, _width: u32, _height: u32) -> String {
    let viewport = canvas.viewport();
    let mut path = String::new();

    for shape_obj in canvas.shapes() {
        // Skip selected shapes - they'll be rendered separately
        // Also skip grouped shapes - they'll be rendered in green
        if shape_obj.selected || shape_obj.group_id.is_some() {
            continue;
        }

        let effective_shape = shape_obj.get_effective_shape();
        let shape_path = render_shape_trait(&effective_shape, viewport);
        path.push_str(&shape_path);
    }

    path
}

/// Render selected shapes with highlight
pub fn render_selected_shapes(canvas: &Canvas, _width: u32, _height: u32) -> String {
    let viewport = canvas.viewport();
    let mut path = String::new();

    for shape_obj in canvas.shapes() {
        // Only render selected shapes that are NOT in a group
        // Grouped shapes are rendered in green regardless of selection state
        if !shape_obj.selected || shape_obj.group_id.is_some() {
            continue;
        }

        let effective_shape = shape_obj.get_effective_shape();
        let shape_path = render_shape_trait(&effective_shape, viewport);
        path.push_str(&shape_path);
    }

    path
}

/// Render grouped shapes (green)
pub fn render_grouped_shapes(canvas: &Canvas, _width: u32, _height: u32) -> String {
    let viewport = canvas.viewport();
    let mut path = String::new();

    for shape_obj in canvas.shapes() {
        if shape_obj.group_id.is_some() {
            let effective_shape = shape_obj.get_effective_shape();
            let shape_path = render_shape_trait(&effective_shape, viewport);
            path.push_str(&shape_path);
        }
    }

    path
}

/// Render selection handles for selected shapes (unified bounding box)
pub fn render_selection_handles(canvas: &Canvas, _width: u32, _height: u32) -> String {
    let viewport = canvas.viewport();
    let mut path = String::new();
    const HANDLE_SIZE: f64 = 8.0;

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut has_selected = false;

    for shape_obj in canvas.shapes() {
        if shape_obj.selected {
            let (x1, y1, x2, y2) = shape_obj.shape.bounds();
            // Normalize coordinates for min/max calculation
            let (tx1, tx2) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
            let (ty1, ty2) = if y1 < y2 { (y1, y2) } else { (y2, y1) };

            min_x = min_x.min(tx1);
            min_y = min_y.min(ty1);
            max_x = max_x.max(tx2);
            max_y = max_y.max(ty2);
            has_selected = true;
        }
    }

    if !has_selected {
        return path;
    }

    // Convert to screen coordinates
    let (sx1, sy1) = viewport.world_to_pixel(min_x, min_y);
    let (sx2, sy2) = viewport.world_to_pixel(max_x, max_y);

    // X coordinates maintain order (left < right)
    // Y coordinates are flipped, so we need min/max
    let screen_left = sx1;
    let screen_right = sx2;
    let screen_top = sy1.min(sy2);
    let screen_bottom = sy1.max(sy2);

    // Calculate handle positions (corners and center) in screen space
    let handles = [
        (screen_left, screen_top),     // Top-left (screen)
        (screen_right, screen_top),    // Top-right (screen)
        (screen_left, screen_bottom),  // Bottom-left (screen)
        (screen_right, screen_bottom), // Bottom-right (screen)
        (
            (screen_left + screen_right) / 2.0,
         (screen_top + screen_bottom) / 2.0,
        ), // Center
    ];

    // Draw handles as small rectangles
    for (hx, hy) in &handles {
        let x = hx - HANDLE_SIZE / 2.0;
        let y = hy - HANDLE_SIZE / 2.0;
        path.push_str(&format!(
            "M {} {} L {} {} L {} {} L {} {} Z ",
            x,
            y,
            x + HANDLE_SIZE,
            y,
            x + HANDLE_SIZE,
            y + HANDLE_SIZE,
            x,
            y + HANDLE_SIZE
        ));
    }

    path
}

/// Render dotted bounding box for selected group/multiple shapes
pub fn render_group_bounding_box(canvas: &Canvas, _width: u32, _height: u32) -> String {
    let viewport = canvas.viewport();
    let mut path = String::new();

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut selected_count = 0;
    let mut has_group = false;

    for shape_obj in canvas.shapes() {
        if shape_obj.selected {
            let (x1, y1, x2, y2) = shape_obj.shape.bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
            selected_count += 1;
            if shape_obj.group_id.is_some() {
                has_group = true;
            }
        }
    }

    // Only draw if we have a group or multiple items selected
    if selected_count < 2 && !has_group {
        return path;
    }

    let (sx1, sy1) = viewport.world_to_pixel(min_x, min_y);
    let (sx2, sy2) = viewport.world_to_pixel(max_x, max_y);

    let left = sx1;
    let right = sx2;
    let top = sy1.min(sy2);
    let bottom = sy1.max(sy2);

    // Simulate dotted line
    let dash_len = 4.0;
    let gap_len = 4.0;

    // Top edge
    let mut x = left;
    while x < right {
        let next_x = (x + dash_len).min(right);
        path.push_str(&format!("M {} {} L {} {} ", x, top, next_x, top));
        x += dash_len + gap_len;
    }

    // Right edge
    let mut y = top;
    while y < bottom {
        let next_y = (y + dash_len).min(bottom);
        path.push_str(&format!("M {} {} L {} {} ", right, y, right, next_y));
        y += dash_len + gap_len;
    }

    // Bottom edge
    let mut x = right;
    while x > left {
        let next_x = (x - dash_len).max(left);
        path.push_str(&format!("M {} {} L {} {} ", x, bottom, next_x, bottom));
        x -= dash_len + gap_len;
    }

    // Left edge
    let mut y = bottom;
    while y > top {
        let next_y = (y - dash_len).max(top);
        path.push_str(&format!("M {} {} L {} {} ", left, y, left, next_y));
        y -= dash_len + gap_len;
    }

    path
}

fn rotate_point(x: f64, y: f64, cx: f64, cy: f64, angle_deg: f64) -> (f64, f64) {
    if angle_deg.abs() < 1e-6 {
        return (x, y);
    }
    let angle_rad = angle_deg.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let dx = x - cx;
    let dy = y - cy;
    (cx + dx * cos_a - dy * sin_a, cy + dx * sin_a + dy * cos_a)
}

/// Render a single shape as SVG path (trait object version)
fn render_shape_trait(shape: &crate::model::Shape, viewport: &crate::viewport::Viewport) -> String {
    // Get shape type and bounding box
    // Use local_bounding_box to find the pivot point (center of unrotated shape)
    let (x1, y1, x2, y2) = shape.bounds();
    let center_x = (x1 + x2) / 2.0;
    let center_y = (y1 + y2) / 2.0;
    let rotation = shape.rotation();

    match shape {
        crate::model::Shape::Rectangle(rect) => {
            // Use unrotated dimensions di(rect.center.y - rect.height/2.0) from the rect struct
            let min_x = rect.center.x - rect.width / 2.0;
            let min_y = rect.center.y - rect.height / 2.0;
            let max_x = (rect.center.x - rect.width / 2.0) + rect.width;
            let max_y = (rect.center.y - rect.height / 2.0) + rect.height;

            let (sx1_raw, sy1_raw) = viewport.world_to_pixel(min_x, min_y);
            let (sx2_raw, sy2_raw) = viewport.world_to_pixel(max_x, max_y);

            let r = rect.corner_radius * viewport.zoom();

            // Clamp radius to half of min dimension in screen pixels to prevent artifacts
            let width = (sx2_raw - sx1_raw).abs();
            let height = (sy1_raw - sy2_raw).abs();
            let max_r = width.min(height) / 2.0;
            let r_pixel = r.min(max_r);

            // We need to work in world coordinates for rotation, then convert to pixel
            // rect.corner_radius is in world units.
            let r_world = rect
            .corner_radius
            .min(rect.width / 2.0)
            .min(rect.height / 2.0);

            if r_world < 0.001 {
                // Sharp rectangle
                let p1 = rotate_point(min_x, min_y, center_x, center_y, rotation);
                let p2 = rotate_point(max_x, min_y, center_x, center_y, rotation);
                let p3 = rotate_point(max_x, max_y, center_x, center_y, rotation);
                let p4 = rotate_point(min_x, max_y, center_x, center_y, rotation);

                let (sx1, sy1) = viewport.world_to_pixel(p1.0, p1.1);
                let (sx2, sy2) = viewport.world_to_pixel(p2.0, p2.1);
                let (sx3, sy3) = viewport.world_to_pixel(p3.0, p3.1);
                let (sx4, sy4) = viewport.world_to_pixel(p4.0, p4.1);

                format!(
                    "M {} {} L {} {} L {} {} L {} {} Z ",
                    sx1, sy1, sx2, sy2, sx3, sy3, sx4, sy4
                )
            } else {
                // Rounded rectangle
                // Points:
                // P1: (min_x + r, min_y)
                // P2: (max_x - r, min_y)
                // P3: (max_x, min_y + r)
                // P4: (max_x, max_y - r)
                // P5: (max_x - r, max_y)
                // P6: (min_x + r, max_y)
                // P7: (min_x, max_y - r)
                // P8: (min_x, min_y + r)

                let pts = [
                    (min_x + r_world, min_y), // Start bottom edge (if y up) or top edge
                    (max_x - r_world, min_y), // End bottom/top edge
                    (max_x, min_y + r_world), // Start right edge
                    (max_x, max_y - r_world), // End right edge
                    (max_x - r_world, max_y), // Start top/bottom edge
                    (min_x + r_world, max_y), // End top/bottom edge
                    (min_x, max_y - r_world), // Start left edge
                    (min_x, min_y + r_world), // End left edge
                ];

                let mut s_pts = Vec::new();
                for (x, y) in pts.iter() {
                    let (rx, ry) = rotate_point(*x, *y, center_x, center_y, rotation);
                    s_pts.push(viewport.world_to_pixel(rx, ry));
                }

                // Radius in pixels
                let r = r_pixel;

                format!(
                    "M {} {} L {} {} A {} {} {} 0 0 {} {} L {} {} A {} {} {} 0 0 {} {} L {} {} A {} {} {} 0 0 {} {} L {} {} A {} {} {} 0 0 {} {} Z ",
                    s_pts[0].0, s_pts[0].1,
                    s_pts[1].0, s_pts[1].1,
                    r, r, -rotation, s_pts[2].0, s_pts[2].1,
                    s_pts[3].0, s_pts[3].1,
                    r, r, -rotation, s_pts[4].0, s_pts[4].1,
                    s_pts[5].0, s_pts[5].1,
                    r, r, -rotation, s_pts[6].0, s_pts[6].1,
                    s_pts[7].0, s_pts[7].1,
                    r, r, -rotation, s_pts[0].0, s_pts[0].1
                )
            }
        }
        crate::model::Shape::Circle(_) => {
            let radius = ((x2 - x1) / 2.0).abs();
            let (cx, cy) = viewport.world_to_pixel(center_x, center_y);
            let screen_radius = radius * viewport.zoom();

            // Circle is invariant under rotation (unless we draw orientation mark, which we don't)
            format!(
                "M {} {} A {} {} 0 0 1 {} {} A {} {} 0 0 1 {} {} A {} {} 0 0 1 {} {} A {} {} 0 0 1 {} {} Z ",
                cx + screen_radius, cy,
                screen_radius, screen_radius, cx, cy + screen_radius,
                screen_radius, screen_radius, cx - screen_radius, cy,
                screen_radius, screen_radius, cx, cy - screen_radius,
                screen_radius, screen_radius, cx + screen_radius, cy
            )
        }
        crate::model::Shape::Line(line) => {
            let p1 = rotate_point(line.start.x, line.start.y, center_x, center_y, rotation);
            let p2 = rotate_point(line.end.x, line.end.y, center_x, center_y, rotation);

            let (sx1, sy1) = viewport.world_to_pixel(p1.0, p1.1);
            let (sx2, sy2) = viewport.world_to_pixel(p2.0, p2.1);

            format!("M {} {} L {} {} ", sx1, sy1, sx2, sy2)
        }
        crate::model::Shape::Ellipse(_) => {
            let rx = ((x2 - x1) / 2.0).abs();
            let ry = ((y2 - y1) / 2.0).abs();

            let screen_rx = rx * viewport.zoom();
            let screen_ry = ry * viewport.zoom();

            // For rotated ellipse, we can use SVG transform or calculate points.
            // Using SVG A command with rotation is easiest.
            // A rx ry x-axis-rotation large-arc-flag sweep-flag x y

            // We need start point.
            // Start at (center_x + rx, center_y) rotated.
            let start = rotate_point(center_x + rx, center_y, center_x, center_y, rotation);
            let (sx, sy) = viewport.world_to_pixel(start.0, start.1);

            // End point (same as start for full ellipse, but we need 2 arcs)
            let mid = rotate_point(center_x - rx, center_y, center_x, center_y, rotation);
            let (mx, my) = viewport.world_to_pixel(mid.0, mid.1);

            format!(
                "M {} {} A {} {} {} 0 1 {} {} A {} {} {} 0 1 {} {} Z ",
                sx,
                sy,
                screen_rx,
                screen_ry,
                -rotation,
                mx,
                my,
                screen_rx,
                screen_ry,
                -rotation,
                sx,
                sy
            )
        }
        crate::model::Shape::Text(text_shape) => {
            let font = font_manager::get_font_for(
                &text_shape.font_family,
                text_shape.bold,
                text_shape.italic,
            );
            let scale = Scale::uniform(text_shape.font_size as f32);
            let v_metrics = font.v_metrics(scale);

            let start = rt_point(text_shape.x as f32, text_shape.y as f32 + v_metrics.ascent);

            let mut builder = SvgPathBuilder {
                path: String::new(),
                viewport: viewport.clone(),
                rotation,
                center: (center_x, center_y),
            };

            for glyph in font.layout(&text_shape.text, scale, start) {
                glyph.build_outline(&mut builder);
            }

            builder.path
        }
        crate::model::Shape::Triangle(triangle) => {
            let path = triangle.render();
            let mut path_str = String::new();
            for event in path.iter() {
                match event {
                    lyon::path::Event::Begin { at } => {
                        let (rx, ry) =
                        rotate_point(at.x as f64, at.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("M {} {} ", sx, sy));
                    }
                    lyon::path::Event::Line { to, .. } => {
                        let (rx, ry) =
                        rotate_point(to.x as f64, to.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("L {} {} ", sx, sy));
                    }
                    lyon::path::Event::End { close, .. } => {
                        if close {
                            path_str.push_str("Z ");
                        }
                    }
                    _ => {}
                }
            }
            path_str
        }
        crate::model::Shape::Polygon(polygon) => {
            let path = polygon.render();
            let mut path_str = String::new();
            for event in path.iter() {
                match event {
                    lyon::path::Event::Begin { at } => {
                        let (rx, ry) =
                        rotate_point(at.x as f64, at.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("M {} {} ", sx, sy));
                    }
                    lyon::path::Event::Line { to, .. } => {
                        let (rx, ry) =
                        rotate_point(to.x as f64, to.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("L {} {} ", sx, sy));
                    }
                    lyon::path::Event::End { close, .. } => {
                        if close {
                            path_str.push_str("Z ");
                        }
                    }
                    _ => {}
                }
            }
            path_str
        }
        crate::model::Shape::Path(path_shape) => {
            let mut path_str = String::new();
            for event in path_shape.render().iter() {
                match event {
                    lyon::path::Event::Begin { at } => {
                        let (rx, ry) =
                        rotate_point(at.x as f64, at.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("M {} {} ", sx, sy));
                    }
                    lyon::path::Event::Line { from: _, to } => {
                        let (rx, ry) =
                        rotate_point(to.x as f64, to.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("L {} {} ", sx, sy));
                    }
                    lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                        let (rcx, rcy) = rotate_point(
                            ctrl.x as f64,
                            ctrl.y as f64,
                            center_x,
                            center_y,
                            rotation,
                        );
                        let (rtx, rty) =
                        rotate_point(to.x as f64, to.y as f64, center_x, center_y, rotation);
                        let (cx, cy) = viewport.world_to_pixel(rcx, rcy);
                        let (sx, sy) = viewport.world_to_pixel(rtx, rty);
                        path_str.push_str(&format!("Q {} {} {} {} ", cx, cy, sx, sy));
                    }
                    lyon::path::Event::Cubic {
                        from: _,
                        ctrl1,
                        ctrl2,
                        to,
                    } => {
                        let (rc1x, rc1y) = rotate_point(
                            ctrl1.x as f64,
                            ctrl1.y as f64,
                            center_x,
                            center_y,
                            rotation,
                        );
                        let (rc2x, rc2y) = rotate_point(
                            ctrl2.x as f64,
                            ctrl2.y as f64,
                            center_x,
                            center_y,
                            rotation,
                        );
                        let (rtx, rty) =
                        rotate_point(to.x as f64, to.y as f64, center_x, center_y, rotation);
                        let (c1x, c1y) = viewport.world_to_pixel(rc1x, rc1y);
                        let (c2x, c2y) = viewport.world_to_pixel(rc2x, rc2y);
                        let (sx, sy) = viewport.world_to_pixel(rtx, rty);
                        path_str
                        .push_str(&format!("C {} {} {} {} {} {} ", c1x, c1y, c2x, c2y, sx, sy));
                    }
                    lyon::path::Event::End {
                        last: _,
                        first: _,
                        close,
                    } => {
                        if close {
                            path_str.push_str("Z ");
                        }
                    }
                }
            }
            path_str
        }
        crate::model::Shape::Gear(gear) => {
            let path = gear.render();
            let mut path_str = String::new();
            for event in path.iter() {
                match event {
                    lyon::path::Event::Begin { at } => {
                        let (rx, ry) =
                        rotate_point(at.x as f64, at.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("M {} {} ", sx, sy));
                    }
                    lyon::path::Event::Line { to, .. } => {
                        let (rx, ry) =
                        rotate_point(to.x as f64, to.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("L {} {} ", sx, sy));
                    }
                    lyon::path::Event::End { close, .. } => {
                        if close {
                            path_str.push_str("Z ");
                        }
                    }
                    _ => {}
                }
            }
            path_str
        }
        crate::model::Shape::Sprocket(sprocket) => {
            let path = sprocket.render();
            let mut path_str = String::new();
            for event in path.iter() {
                match event {
                    lyon::path::Event::Begin { at } => {
                        let (rx, ry) =
                        rotate_point(at.x as f64, at.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("M {} {} ", sx, sy));
                    }
                    lyon::path::Event::Line { to, .. } => {
                        let (rx, ry) =
                        rotate_point(to.x as f64, to.y as f64, center_x, center_y, rotation);
                        let (sx, sy) = viewport.world_to_pixel(rx, ry);
                        path_str.push_str(&format!("L {} {} ", sx, sy));
                    }
                    lyon::path::Event::End { close, .. } => {
                        if close {
                            path_str.push_str("Z ");
                        }
                    }
                    _ => {}
                }
            }
            path_str
        }
    }
}

struct SvgPathBuilder {
    path: String,
    viewport: crate::viewport::Viewport,
    rotation: f64,
    center: (f64, f64),
}

impl OutlineBuilder for SvgPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let (rx, ry) = rotate_point(
            x as f64,
            y as f64,
            self.center.0,
            self.center.1,
            self.rotation,
        );
        let (sx, sy) = self.viewport.world_to_pixel(rx, ry);
        self.path.push_str(&format!("M {} {} ", sx, sy));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (rx, ry) = rotate_point(
            x as f64,
            y as f64,
            self.center.0,
            self.center.1,
            self.rotation,
        );
        let (sx, sy) = self.viewport.world_to_pixel(rx, ry);
        self.path.push_str(&format!("L {} {} ", sx, sy));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (rx1, ry1) = rotate_point(
            x1 as f64,
            y1 as f64,
            self.center.0,
            self.center.1,
            self.rotation,
        );
        let (rx, ry) = rotate_point(
            x as f64,
            y as f64,
            self.center.0,
            self.center.1,
            self.rotation,
        );
        let (sx1, sy1) = self.viewport.world_to_pixel(rx1, ry1);
        let (sx, sy) = self.viewport.world_to_pixel(rx, ry);
        self.path
        .push_str(&format!("Q {} {} {} {} ", sx1, sy1, sx, sy));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (rx1, ry1) = rotate_point(
            x1 as f64,
            y1 as f64,
            self.center.0,
            self.center.1,
            self.rotation,
        );
        let (rx2, ry2) = rotate_point(
            x2 as f64,
            y2 as f64,
            self.center.0,
            self.center.1,
            self.rotation,
        );
        let (rx, ry) = rotate_point(
            x as f64,
            y as f64,
            self.center.0,
            self.center.1,
            self.rotation,
        );
        let (sx1, sy1) = self.viewport.world_to_pixel(rx1, ry1);
        let (sx2, sy2) = self.viewport.world_to_pixel(rx2, ry2);
        let (sx, sy) = self.viewport.world_to_pixel(rx, ry);
        self.path
        .push_str(&format!("C {} {} {} {} {} {} ", sx1, sy1, sx2, sy2, sx, sy));
    }

    fn close(&mut self) {
        self.path.push_str("Z ");
    }
}
