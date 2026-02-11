//! Canvas renderer for designer shapes
//! Renders shapes to an image buffer for display in the UI using tiny-skia for high-quality 2D rendering.
//!
//! Features:
//! - Anti-aliased rendering
//! - High performance
//! - Viewport-based coordinate transformation
//! - Shape rendering with selection indicators

use crate::model::DesignerShape;
use crate::{font_manager, Canvas};
use image::{Rgb, RgbImage};
use rusttype::{point as rt_point, Scale};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

const HANDLE_SIZE: f32 = 18.0; // Increased from 12.0 for easier cursor positioning

fn bg_color() -> Color {
    Color::from_rgba8(52, 73, 94, 255)
}
fn shape_color() -> Color {
    Color::from_rgba8(52, 152, 219, 255)
}
fn selection_color() -> Color {
    Color::from_rgba8(255, 235, 59, 255)
}
fn crosshair_color() -> Color {
    Color::from_rgba8(255, 255, 0, 255)
}

/// Render canvas shapes to an image buffer
pub fn render_canvas(
    canvas: &Canvas,
    width: u32,
    height: u32,
    _zoom: f32,
    _pan_x: f32,
    _pan_y: f32,
) -> RgbImage {
    let Some(mut pixmap) = Pixmap::new(width, height) else {
        return RgbImage::new(width, height);
    };
    pixmap.fill(bg_color());

    let viewport = canvas.viewport();
    let zoom = viewport.zoom() as f32;
    let pan_x = viewport.pan_x() as f32;
    let pan_y = viewport.pan_y() as f32;
    let canvas_height = height as f32;

    // Transform: World -> Screen
    // pixel_x = world_x * zoom + pan_x
    // pixel_y = canvas_height - (world_y * zoom + pan_y) = -world_y * zoom + (canvas_height - pan_y)
    let transform = Transform::from_scale(zoom, -zoom).post_translate(pan_x, canvas_height - pan_y);

    // Draw Crosshair (Axes)
    let mut paint = Paint::default();
    paint.set_color(crosshair_color());
    paint.anti_alias = false; // Sharp lines for axes
    let stroke = Stroke {
        width: 2.0,
        ..Default::default()
    };

    let mut pb = PathBuilder::new();
    // X-axis (y=0)
    // We want to draw from x=-infinity to +infinity in world space, clipped to screen.
    // Easier: draw from screen left to screen right, transform back to world to find y?
    // Or just draw a very long line in world space.
    // World bounds visible:
    // min_x = (0 - pan_x) / zoom
    // max_x = (width - pan_x) / zoom
    // min_y = (height - (canvas_height - pan_y)) / -zoom ... math is messy.
    // Let's just draw the axes if they are visible.

    // Origin in screen space
    let (origin_x, origin_y) = viewport.world_to_pixel(0.0, 0.0);
    let origin_x = origin_x as f32;
    let origin_y = origin_y as f32;

    if origin_y >= 0.0 && origin_y < canvas_height {
        pb.move_to(0.0, origin_y);
        pb.line_to(width as f32, origin_y);
    }
    if origin_x >= 0.0 && origin_x < width as f32 {
        pb.move_to(origin_x, 0.0);
        pb.line_to(origin_x, canvas_height);
    }
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    // Render Shapes
    for shape_obj in canvas.shapes() {
        let mut paint = Paint::default();
        paint.set_color(shape_color());
        paint.anti_alias = true;

        // We can fill or stroke. Let's assume stroke for lines/paths and fill for closed shapes?
        // The original renderer drew filled rectangles/circles/ellipses and stroked lines/paths.

        let effective_shape = shape_obj.get_effective_shape();
        match &effective_shape {
            crate::model::Shape::Rectangle(rect) => {
                let rect_path = Rect::from_xywh(
                    (rect.center.x - rect.width / 2.0) as f32,
                    (rect.center.y - rect.height / 2.0) as f32,
                    rect.width as f32,
                    rect.height as f32,
                );
                if let Some(r) = rect_path {
                    let path = PathBuilder::from_rect(r);
                    pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
                }
            }
            crate::model::Shape::Circle(circle) => {
                let path = PathBuilder::from_circle(
                    circle.center.x as f32,
                    circle.center.y as f32,
                    circle.radius as f32,
                );
                if let Some(p) = path {
                    pixmap.fill_path(&p, &paint, FillRule::Winding, transform, None);
                }
            }
            crate::model::Shape::Line(line) => {
                let mut pb = PathBuilder::new();
                pb.move_to(line.start.x as f32, line.start.y as f32);
                pb.line_to(line.end.x as f32, line.end.y as f32);
                if let Some(path) = pb.finish() {
                    let stroke = Stroke {
                        width: 1.0 / zoom,
                        ..Default::default()
                    };
                    pixmap.stroke_path(&path, &paint, &stroke, transform, None);
                }
            }
            crate::model::Shape::Ellipse(ellipse) => {
                // tiny-skia doesn't have direct ellipse primitive, use scale on circle
                let path = PathBuilder::from_circle(0.0, 0.0, 1.0); // Unit circle
                if let Some(p) = path {
                    // Transform for ellipse: translate to center, scale by rx/ry
                    let ellipse_transform = transform
                        .pre_translate(ellipse.center.x as f32, ellipse.center.y as f32)
                        .pre_scale(ellipse.rx as f32, ellipse.ry as f32);

                    pixmap.fill_path(&p, &paint, FillRule::Winding, ellipse_transform, None);
                }
            }
            crate::model::Shape::Path(path_shape) => {
                // Convert lyon path to tiny-skia path
                let mut pb = PathBuilder::new();
                for event in path_shape.render().iter() {
                    match event {
                        lyon::path::Event::Begin { at } => {
                            pb.move_to(at.x, at.y);
                        }
                        lyon::path::Event::Line { from: _, to } => {
                            pb.line_to(to.x, to.y);
                        }
                        lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                            pb.quad_to(ctrl.x, ctrl.y, to.x, to.y);
                        }
                        lyon::path::Event::Cubic {
                            from: _,
                            ctrl1,
                            ctrl2,
                            to,
                        } => {
                            pb.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y);
                        }
                        lyon::path::Event::End {
                            last: _,
                            first: _,
                            close,
                        } => {
                            if close {
                                pb.close();
                            }
                        }
                    }
                }
                if let Some(path) = pb.finish() {
                    let stroke = Stroke {
                        width: 1.0 / zoom,
                        ..Default::default()
                    };
                    pixmap.stroke_path(&path, &paint, &stroke, transform, None);
                }
            }
            crate::model::Shape::Triangle(triangle) => {
                let path = triangle.render();
                let mut pb = PathBuilder::new();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => pb.move_to(at.x, at.y),
                        lyon::path::Event::Line { to, .. } => pb.line_to(to.x, to.y),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                pb.close();
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(p) = pb.finish() {
                    pixmap.fill_path(&p, &paint, FillRule::Winding, transform, None);
                }
            }
            crate::model::Shape::Polygon(polygon) => {
                let path = polygon.render();
                let mut pb = PathBuilder::new();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => pb.move_to(at.x, at.y),
                        lyon::path::Event::Line { to, .. } => pb.line_to(to.x, to.y),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                pb.close();
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(p) = pb.finish() {
                    pixmap.fill_path(&p, &paint, FillRule::Winding, transform, None);
                }
            }
            crate::model::Shape::Gear(gear) => {
                let path = gear.render();
                let mut pb = PathBuilder::new();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => pb.move_to(at.x, at.y),
                        lyon::path::Event::Line { to, .. } => pb.line_to(to.x, to.y),
                        lyon::path::Event::Quadratic { ctrl, to, .. } => {
                            pb.quad_to(ctrl.x, ctrl.y, to.x, to.y)
                        }
                        lyon::path::Event::Cubic {
                            ctrl1, ctrl2, to, ..
                        } => pb.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                pb.close();
                            }
                        }
                    }
                }
                if let Some(p) = pb.finish() {
                    pixmap.fill_path(&p, &paint, FillRule::Winding, transform, None);
                }
            }
            crate::model::Shape::Sprocket(sprocket) => {
                let path = sprocket.render();
                let mut pb = PathBuilder::new();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => pb.move_to(at.x, at.y),
                        lyon::path::Event::Line { to, .. } => pb.line_to(to.x, to.y),
                        lyon::path::Event::Quadratic { ctrl, to, .. } => {
                            pb.quad_to(ctrl.x, ctrl.y, to.x, to.y)
                        }
                        lyon::path::Event::Cubic {
                            ctrl1, ctrl2, to, ..
                        } => pb.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                pb.close();
                            }
                        }
                    }
                }
                if let Some(p) = pb.finish() {
                    pixmap.fill_path(&p, &paint, FillRule::Winding, transform, None);
                }
            }
            crate::model::Shape::Text(text_shape) => {
                // Text rendering using rusttype, drawing di(rect.center.y - rect.height/2.0) to pixmap pixels or using paths
                // For simplicity and quality, let's convert glyphs to paths if possible, or just draw pixels.
                // tiny-skia doesn't support text di(rect.center.y - rect.height/2.0).
                // We'll use the existing pixel-based approach but adapted for tiny-skia's buffer.

                let font = font_manager::get_font_for(
                    &text_shape.font_family,
                    text_shape.bold,
                    text_shape.italic,
                );
                let font_size_screen = text_shape.font_size as f32 * zoom;
                let scale = Scale::uniform(font_size_screen);

                let (screen_x, screen_y) = viewport.world_to_pixel(text_shape.x, text_shape.y);
                let v_metrics = font.v_metrics(scale);
                let start = rt_point(screen_x as f32, screen_y as f32 + v_metrics.ascent);

                for glyph in font.layout(&text_shape.text, scale, start) {
                    if let Some(bounding_box) = glyph.pixel_bounding_box() {
                        glyph.draw(|gx, gy, v| {
                            let px = gx as i32 + bounding_box.min.x;
                            let py = gy as i32 + bounding_box.min.y;

                            if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                                let alpha = (v * 255.0) as u8;
                                if alpha > 0 {
                                    // Blend with existing color? Or just set?
                                    // tiny-skia is premultiplied alpha.
                                    // SHAPE_COLOR is opaque.
                                    // We can just set the pixel if we want simple text.
                                    // Or use tiny-skia's pixel manipulation if exposed.
                                    // Pixmap data is a slice of u8.
                                    let idx = ((py as u32 * width + px as u32) * 4) as usize;
                                    let pixel = &mut pixmap.data_mut()[idx..idx + 4];

                                    // Simple alpha blending over background
                                    // Src: SHAPE_COLOR with alpha
                                    // Dst: pixel
                                    // This is getting complicated to do manually.
                                    // Let's try to use PathBuilder with rusttype's outline_glyph if possible.
                                    // But rusttype outline is experimental/complex?
                                    // Actually, let's just draw pixels manually for now, it's easier.

                                    let r = shape_color().red();
                                    let g = shape_color().green();
                                    let b = shape_color().blue();

                                    // Premultiplied alpha
                                    let a = alpha;
                                    let r = (r as u16 * a as u16 / 255) as u8;
                                    let g = (g as u16 * a as u16 / 255) as u8;
                                    let b = (b as u16 * a as u16 / 255) as u8;

                                    // Just overwrite for now (assuming opaque text on opaque bg)
                                    pixel[0] = r;
                                    pixel[1] = g;
                                    pixel[2] = b;
                                    pixel[3] = a;
                                }
                            }
                        });
                    }
                }
            }
        }

        // Draw Selection Indicators
        if shape_obj.selected {
            let (x1, y1, x2, y2) = shape_obj.shape.bounds();

            // Draw bounding box
            let rect = Rect::from_ltrb(
                x1 as f32,
                y1.min(y2) as f32, // Ensure min/max correct
                x2 as f32,
                y1.max(y2) as f32,
            );

            if let Some(r) = rect {
                let path = PathBuilder::from_rect(r);
                let stroke = Stroke {
                    width: 1.0 / zoom,
                    ..Default::default()
                };
                let mut paint = Paint::default();
                paint.set_color(selection_color());

                pixmap.stroke_path(&path, &paint, &stroke, transform, None);

                // Draw handles
                let handles = [
                    (x1, y1),
                    (x2, y1),
                    (x1, y2),
                    (x2, y2),
                    ((x1 + x2) / 2.0, (y1 + y2) / 2.0),
                ];

                let handle_size_world = HANDLE_SIZE / zoom;

                for (i, (hx, hy)) in handles.iter().enumerate() {
                    // Make center handle (index 4) 25% bigger for easier grabbing
                    let size = if i == 4 {
                        handle_size_world * 1.25
                    } else {
                        handle_size_world
                    };

                    let h_rect = Rect::from_xywh(
                        (hx - size as f64 / 2.0) as f32,
                        (hy - size as f64 / 2.0) as f32,
                        size,
                        size,
                    );
                    if let Some(hr) = h_rect {
                        let h_path = PathBuilder::from_rect(hr);
                        pixmap.fill_path(&h_path, &paint, FillRule::Winding, transform, None);
                    }
                }
            }
        }
    }

    // Convert Pixmap to RgbImage
    let data = pixmap.data();
    RgbImage::from_fn(width, height, |x, y| {
        let idx = ((y * width + x) * 4) as usize;
        let r = data[idx];
        let g = data[idx + 1];
        let b = data[idx + 2];
        // Ignore alpha, assume opaque
        Rgb([r, g, b])
    })
}
