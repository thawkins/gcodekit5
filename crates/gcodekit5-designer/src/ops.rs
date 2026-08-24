//! # Boolean Operations
//!
//! Provides CSG boolean operations (union, difference, intersection)
//! on designer shapes using the `csgrs` and `cavalier_contours` libraries.
//! Includes polyline cleaning utilities for robust boolean results.

use crate::model::{DesignPath, DesignerShape, Point, Shape};
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use csgrs::traits::CSG;
use lyon::path::iterator::PathIterator;
use std::f64::consts::PI;
use std::panic;

pub enum BooleanOp {
    Union,
    Difference,
    Intersection,
}

pub fn clean_polyline(mut pline: Polyline) -> Polyline {
    pline.remove_repeat_pos(1e-5);
    if pline.is_closed() && pline.vertex_count() > 1 {
        if let (Some(first), Some(last)) = (pline.get(0), pline.get(pline.vertex_count() - 1)) {
            if (first.x - last.x).abs() < 1e-5 && (first.y - last.y).abs() < 1e-5 {
                pline.remove(pline.vertex_count() - 1);
            }
        }
    }
    pline
}

fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ax * by - ay * bx
}

fn extract_open_polyline_points(path: &lyon::path::Path) -> Option<Vec<Point>> {
    let mut points: Vec<Point> = Vec::new();
    let mut begin_count = 0usize;

    for event in path.iter().flattened(0.1) {
        match event {
            lyon::path::Event::Begin { at } => {
                begin_count += 1;
                if begin_count > 1 {
                    return None;
                }
                points.push(Point::new(at.x as f64, at.y as f64));
            }
            lyon::path::Event::Line { to, .. } => {
                points.push(Point::new(to.x as f64, to.y as f64));
            }
            lyon::path::Event::End { close, .. } => {
                if close {
                    return None;
                }
            }
            _ => {}
        }
    }

    if points.len() < 2 {
        return None;
    }

    let mut deduped = Vec::with_capacity(points.len());
    for p in points {
        if deduped
            .last()
            .map(|q: &Point| (q.x - p.x).hypot(q.y - p.y) > 1e-6)
            .unwrap_or(true)
        {
            deduped.push(p);
        }
    }

    if deduped.len() < 2 {
        None
    } else {
        Some(deduped)
    }
}

fn extract_closed_polyline_points(path: &lyon::path::Path) -> Option<Vec<Point>> {
    let mut points: Vec<Point> = Vec::new();
    let mut begin_count = 0usize;
    let mut saw_closed_end = false;

    for event in path.iter().flattened(0.1) {
        match event {
            lyon::path::Event::Begin { at } => {
                begin_count += 1;
                if begin_count > 1 {
                    return None;
                }
                points.push(Point::new(at.x as f64, at.y as f64));
            }
            lyon::path::Event::Line { to, .. } => {
                points.push(Point::new(to.x as f64, to.y as f64));
            }
            lyon::path::Event::End { close, .. } => {
                if !close {
                    return None;
                }
                saw_closed_end = true;
            }
            _ => {}
        }
    }

    if !saw_closed_end {
        return None;
    }

    let mut deduped = Vec::with_capacity(points.len());
    for p in points {
        if deduped
            .last()
            .map(|q: &Point| (q.x - p.x).hypot(q.y - p.y) > 1e-6)
            .unwrap_or(true)
        {
            deduped.push(p);
        }
    }

    if deduped.len() >= 2 {
        let first = deduped[0];
        let last = *deduped.last()?;
        if (first.x - last.x).hypot(first.y - last.y) <= 1e-6 {
            deduped.pop();
        }
    }

    if deduped.len() < 3 {
        None
    } else {
        Some(deduped)
    }
}

fn chamfer_closed_points(points: &[Point], distance: f64) -> Vec<Point> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }

    let d_req = distance.max(0.0);
    if d_req <= 1e-9 {
        return points.to_vec();
    }

    let mut out: Vec<Point> = Vec::with_capacity(n * 2);
    for i in 0..n {
        let prev = points[(i + n - 1) % n];
        let cur = points[i];
        let next = points[(i + 1) % n];

        let vin_x = prev.x - cur.x;
        let vin_y = prev.y - cur.y;
        let vout_x = next.x - cur.x;
        let vout_y = next.y - cur.y;

        let len_in = vin_x.hypot(vin_y);
        let len_out = vout_x.hypot(vout_y);

        if len_in <= 1e-9 || len_out <= 1e-9 {
            out.push(cur);
            continue;
        }

        let d = d_req.min(len_in * 0.49).min(len_out * 0.49);
        if d <= 1e-9 {
            out.push(cur);
            continue;
        }

        let pin = Point::new(cur.x + (vin_x / len_in) * d, cur.y + (vin_y / len_in) * d);
        let pout = Point::new(
            cur.x + (vout_x / len_out) * d,
            cur.y + (vout_y / len_out) * d,
        );

        out.push(pin);
        out.push(pout);
    }

    out
}

fn try_open_path_chamfer(path: &DesignPath, distance: f64) -> Option<DesignPath> {
    let original = path.original_path.as_ref()?;
    let points = extract_open_polyline_points(original)?;

    if points.len() < 3 {
        return Some(DesignPath::from_points(&points, false));
    }

    let d_req = distance.max(0.0);
    if d_req <= 1e-9 {
        return Some(DesignPath::from_points(&points, false));
    }

    let mut out: Vec<Point> = Vec::with_capacity(points.len() * 2);
    out.push(points[0]);

    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let cur = points[i];
        let next = points[i + 1];

        let vin_x = prev.x - cur.x;
        let vin_y = prev.y - cur.y;
        let vout_x = next.x - cur.x;
        let vout_y = next.y - cur.y;

        let len_in = vin_x.hypot(vin_y);
        let len_out = vout_x.hypot(vout_y);

        if len_in <= 1e-9 || len_out <= 1e-9 {
            out.push(cur);
            continue;
        }

        let d = d_req.min(len_in * 0.49).min(len_out * 0.49);
        if d <= 1e-9 {
            out.push(cur);
            continue;
        }

        let pin = Point::new(cur.x + (vin_x / len_in) * d, cur.y + (vin_y / len_in) * d);
        let pout = Point::new(
            cur.x + (vout_x / len_out) * d,
            cur.y + (vout_y / len_out) * d,
        );

        out.push(pin);
        out.push(pout);
    }

    out.push(*points.last()?);
    Some(DesignPath::from_points(&out, false))
}

fn try_closed_path_chamfer(path: &DesignPath, distance: f64) -> Option<DesignPath> {
    let original = path.original_path.as_ref()?;
    let points = extract_closed_polyline_points(original)?;
    let chamfered = chamfer_closed_points(&points, distance);
    Some(DesignPath::from_points(&chamfered, true))
}

fn try_open_path_fillet(path: &DesignPath, radius: f64) -> Option<DesignPath> {
    let original = path.original_path.as_ref()?;
    let points = extract_open_polyline_points(original)?;

    if points.len() < 3 {
        return Some(DesignPath::from_points(&points, false));
    }

    let r_req = radius.max(0.0);
    if r_req <= 1e-9 {
        return Some(DesignPath::from_points(&points, false));
    }

    let mut out: Vec<Point> = Vec::new();
    out.push(points[0]);

    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let cur = points[i];
        let next = points[i + 1];

        let a_x = cur.x - prev.x;
        let a_y = cur.y - prev.y;
        let b_x = next.x - cur.x;
        let b_y = next.y - cur.y;

        let len_a = a_x.hypot(a_y);
        let len_b = b_x.hypot(b_y);

        if len_a <= 1e-9 || len_b <= 1e-9 {
            out.push(cur);
            continue;
        }

        let ua_x = a_x / len_a;
        let ua_y = a_y / len_a;
        let ub_x = b_x / len_b;
        let ub_y = b_y / len_b;

        let dot = (-(ua_x * ub_x + ua_y * ub_y)).clamp(-1.0, 1.0);
        let angle = dot.acos();
        let tan_half = (angle * 0.5).tan();

        if angle <= 1e-6 || (PI - angle).abs() <= 1e-6 || tan_half.abs() <= 1e-9 {
            out.push(cur);
            continue;
        }

        let mut t = r_req / tan_half;
        t = t.min(len_a * 0.49).min(len_b * 0.49);
        if t <= 1e-9 {
            out.push(cur);
            continue;
        }

        let tp1 = Point::new(cur.x - ua_x * t, cur.y - ua_y * t);
        let tp2 = Point::new(cur.x + ub_x * t, cur.y + ub_y * t);

        let turn = cross(ua_x, ua_y, ub_x, ub_y);
        if turn.abs() <= 1e-9 {
            out.push(cur);
            continue;
        }

        let (n1_x, n1_y, n2_x, n2_y) = if turn > 0.0 {
            (-ua_y, ua_x, -ub_y, ub_x)
        } else {
            (ua_y, -ua_x, ub_y, -ub_x)
        };

        let den = cross(n1_x, n1_y, n2_x, n2_y);
        if den.abs() <= 1e-9 {
            out.push(tp1);
            out.push(tp2);
            continue;
        }

        let dx = tp2.x - tp1.x;
        let dy = tp2.y - tp1.y;
        let s = cross(dx, dy, n2_x, n2_y) / den;
        let cx = tp1.x + n1_x * s;
        let cy = tp1.y + n1_y * s;
        let arc_r = (tp1.x - cx).hypot(tp1.y - cy);

        let a1 = (tp1.y - cy).atan2(tp1.x - cx);
        let a2 = (tp2.y - cy).atan2(tp2.x - cx);
        let mut delta = a2 - a1;
        if turn > 0.0 {
            if delta < 0.0 {
                delta += 2.0 * PI;
            }
        } else if delta > 0.0 {
            delta -= 2.0 * PI;
        }

        let segs = ((delta.abs() / (PI / 18.0)).ceil() as usize).max(2);
        out.push(tp1);
        for k in 1..segs {
            let t_arc = k as f64 / segs as f64;
            let a = a1 + delta * t_arc;
            out.push(Point::new(cx + arc_r * a.cos(), cy + arc_r * a.sin()));
        }
        out.push(tp2);
    }

    out.push(*points.last()?);
    Some(DesignPath::from_points(&out, false))
}

fn try_closed_path_fillet(path: &DesignPath, radius: f64) -> Option<DesignPath> {
    let original = path.original_path.as_ref()?;

    let mut points: Vec<Point> = Vec::new();
    let mut begin_count = 0usize;
    let mut saw_closed_end = false;

    for event in original.iter().flattened(0.1) {
        match event {
            lyon::path::Event::Begin { at } => {
                begin_count += 1;
                if begin_count > 1 {
                    return None;
                }
                points.push(Point::new(at.x as f64, at.y as f64));
            }
            lyon::path::Event::Line { to, .. } => {
                points.push(Point::new(to.x as f64, to.y as f64));
            }
            lyon::path::Event::End { close, .. } => {
                if !close {
                    return None;
                }
                saw_closed_end = true;
            }
            _ => {}
        }
    }

    if !saw_closed_end {
        return None;
    }

    let mut deduped = Vec::with_capacity(points.len());
    for p in points {
        if deduped
            .last()
            .map(|q: &Point| (q.x - p.x).hypot(q.y - p.y) > 1e-6)
            .unwrap_or(true)
        {
            deduped.push(p);
        }
    }

    if deduped.len() >= 2 {
        let first = deduped[0];
        let last = *deduped.last()?;
        if (first.x - last.x).hypot(first.y - last.y) <= 1e-6 {
            deduped.pop();
        }
    }

    if deduped.len() < 3 {
        return None;
    }

    let r_req = radius.max(0.0);
    if r_req <= 1e-9 {
        return Some(DesignPath::from_points(&deduped, true));
    }

    let n = deduped.len();
    let mut out: Vec<Point> = Vec::with_capacity(n * 8);

    for i in 0..n {
        let prev = deduped[(i + n - 1) % n];
        let cur = deduped[i];
        let next = deduped[(i + 1) % n];

        let a_x = cur.x - prev.x;
        let a_y = cur.y - prev.y;
        let b_x = next.x - cur.x;
        let b_y = next.y - cur.y;

        let len_a = a_x.hypot(a_y);
        let len_b = b_x.hypot(b_y);
        if len_a <= 1e-9 || len_b <= 1e-9 {
            out.push(cur);
            continue;
        }

        let ua_x = a_x / len_a;
        let ua_y = a_y / len_a;
        let ub_x = b_x / len_b;
        let ub_y = b_y / len_b;

        let dot = (-(ua_x * ub_x + ua_y * ub_y)).clamp(-1.0, 1.0);
        let angle = dot.acos();
        let tan_half = (angle * 0.5).tan();
        if angle <= 1e-6 || (PI - angle).abs() <= 1e-6 || tan_half.abs() <= 1e-9 {
            out.push(cur);
            continue;
        }

        let mut t = r_req / tan_half;
        t = t.min(len_a * 0.49).min(len_b * 0.49);
        if t <= 1e-9 {
            out.push(cur);
            continue;
        }

        let tp1 = Point::new(cur.x - ua_x * t, cur.y - ua_y * t);
        let tp2 = Point::new(cur.x + ub_x * t, cur.y + ub_y * t);

        let turn = cross(ua_x, ua_y, ub_x, ub_y);
        if turn.abs() <= 1e-9 {
            out.push(cur);
            continue;
        }

        let (n1_x, n1_y, n2_x, n2_y) = if turn > 0.0 {
            (-ua_y, ua_x, -ub_y, ub_x)
        } else {
            (ua_y, -ua_x, ub_y, -ub_x)
        };

        let den = cross(n1_x, n1_y, n2_x, n2_y);
        if den.abs() <= 1e-9 {
            out.push(tp1);
            out.push(tp2);
            continue;
        }

        let dx = tp2.x - tp1.x;
        let dy = tp2.y - tp1.y;
        let s = cross(dx, dy, n2_x, n2_y) / den;
        let cx = tp1.x + n1_x * s;
        let cy = tp1.y + n1_y * s;
        let arc_r = (tp1.x - cx).hypot(tp1.y - cy);

        let a1 = (tp1.y - cy).atan2(tp1.x - cx);
        let a2 = (tp2.y - cy).atan2(tp2.x - cx);
        let mut delta = a2 - a1;
        if turn > 0.0 {
            if delta < 0.0 {
                delta += 2.0 * PI;
            }
        } else if delta > 0.0 {
            delta -= 2.0 * PI;
        }

        let segs = ((delta.abs() / (PI / 18.0)).ceil() as usize).max(2);
        out.push(tp1);
        for k in 1..segs {
            let t_arc = k as f64 / segs as f64;
            let a = a1 + delta * t_arc;
            out.push(Point::new(cx + arc_r * a.cos(), cy + arc_r * a.sin()));
        }
        out.push(tp2);
    }

    Some(DesignPath::from_points(&out, true))
}

pub fn perform_boolean(a: &Shape, b: &Shape, op: BooleanOp) -> Shape {
    let csg_a = a.as_csg();
    let csg_b = b.as_csg();

    let result_csg = match op {
        BooleanOp::Union => csg_a.union(&csg_b),
        BooleanOp::Difference => csg_a.difference(&csg_b),
        BooleanOp::Intersection => csg_a.intersection(&csg_b),
    };

    Shape::Path(DesignPath::from_csg(result_csg))
}

// ---
// FUNCIÓN AUXILIAR: Ajustada al sistema de Cavalier
fn interpolate_polyline_arcs(pline: &Polyline, tolerance: f64) -> Vec<[f64; 2]> {
    if pline.vertex_count() == 0 {
        return Vec::new();
    }

    let mut points = Vec::new();
    let count = pline.vertex_count();

    // Iteramos explícitamente por los índices para controlar el inicio y el fin del bucle cerrado
    for i in 0..count {
        let v_start = pline.get(i).unwrap();
        // Si está cerrado, el siguiente del último es el primero (0)
        let v_end = pline.get((i + 1) % count).unwrap();

        // Añadimos siempre el inicio del segmento actual
        points.push([v_start.x, v_start.y]);

        // Si el segmento actual es un arco (bulge)
        if v_start.bulge.abs() > 1e-6 {
            let (radius, center) = cavalier_contours::polyline::seg_arc_radius_and_center(v_start, v_end);

            let start_angle = (v_start.y - center.y).atan2(v_start.x - center.x);
            let mut end_angle = (v_end.y - center.y).atan2(v_end.x - center.x);

            let is_clockwise = v_start.bulge < 0.0;

            if is_clockwise && end_angle > start_angle {
                end_angle -= 2.0 * std::f64::consts::PI;
            } else if !is_clockwise && end_angle < start_angle {
                end_angle += 2.0 * std::f64::consts::PI;
            }

            let angle_diff = (end_angle - start_angle).abs();

            let steps = if radius > tolerance {
                let arc_cos = 1.0 - (tolerance / radius);
                let step_angle = (2.0 * arc_cos.acos()).max(0.01);
                (angle_diff / step_angle).ceil() as usize
            } else {
                5
            };

            for step in 1..steps {
                let t = step as f64 / steps as f64;
                let angle = start_angle + (end_angle - start_angle) * t;
                let x = center.x + radius * angle.cos();
                let y = center.y + radius * angle.sin();
                points.push([x, y]);
            }
        }
    }

    // CORRECCIÓN DEDUP
    points.dedup_by(|a, b| {
        (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5
    });

    points
}

pub fn perform_offset(shape: &Shape, distance: f64) -> Shape {
    let (sketch, rotation) = if let Some(path) = shape.as_any().downcast_ref::<DesignPath>() {
        if path.rotation.abs() > 1e-6 {
            use nalgebra::{Matrix4, Vector3};

            let bb = csgrs::traits::CSG::bounding_box(&path.sketch);
            let center_x = (bb.mins.x + bb.maxs.x) / 2.0;
            let center_y = (bb.mins.y + bb.maxs.y) / 2.0;

            let angle_rad = path.rotation.to_radians();
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();

            let to_origin = Matrix4::new_translation(&Vector3::new(-center_x, -center_y, 0.0));
            let rotation_mat = Matrix4::new(
                cos_a, -sin_a, 0.0, 0.0, sin_a, cos_a, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            );
            let from_origin = Matrix4::new_translation(&Vector3::new(center_x, center_y, 0.0));
            let transform = from_origin * rotation_mat * to_origin;

            (path.sketch.transform(&transform), 0.0)
        } else {
            (path.sketch.clone(), 0.0)
        }
    } else {
        (shape.as_csg(), 0.0)
    };

    let mp = sketch.to_multipolygon();
    let mut result_sketch = csgrs::sketch::Sketch::new();

    for poly in mp.0 {
        // --- CONTORNO EXTERIOR (Perfil del piñón / Triángulo) ---
        let mut ext_pline = Polyline::new();
        for coord in poly.exterior().0.iter() {
            ext_pline.add_vertex(PlineVertex::new(coord.x, coord.y, 0.0));
        }
        ext_pline.set_is_closed(true);
        let ext_pline = clean_polyline(ext_pline);

        let offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            ext_pline.parallel_offset(distance)
        }))
        .unwrap_or_default();

        for offset in offsets {
            // Muestreamos con una tolerancia estricta de 0.01mm para máxima fidelidad en el piñón
            let pts = interpolate_polyline_arcs(&offset, 0.01);
            if pts.len() >= 3 {
                result_sketch = result_sketch.union(&csgrs::sketch::Sketch::polygon(&pts, None));
            }
        }

        // --- CONTORNOS INTERIORES ---
        for interior in poly.interiors() {
            let mut int_pline = Polyline::new();
            for coord in interior.0.iter() {
                int_pline.add_vertex(PlineVertex::new(coord.x, coord.y, 0.0));
            }
            int_pline.set_is_closed(true);
            let int_pline = clean_polyline(int_pline);

            let offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                int_pline.parallel_offset(distance)
            }))
            .unwrap_or_default();

            for offset in offsets {
                let pts = interpolate_polyline_arcs(&offset, 0.01);
                if pts.len() >= 3 {
                    result_sketch = result_sketch.union(&csgrs::sketch::Sketch::polygon(&pts, None));
                }
            }
        }
    }

    let mut result_path = DesignPath::from_csg(result_sketch);
    result_path.rotation = rotation;
    Shape::Path(result_path)
}
// ---

/*
pub fn perform_offset(shape: &Shape, distance: f64) -> Shape {
    // For DesignPath, we need to apply rotation to the sketch before offsetting
    let (sketch, rotation) = if let Some(path) = shape.as_any().downcast_ref::<DesignPath>() {
        // If the path has rotation, apply it to the sketch first
        if path.rotation.abs() > 1e-6 {
            use nalgebra::{Matrix4, Vector3};

            // Calculate center of rotation
            let bb = csgrs::traits::CSG::bounding_box(&path.sketch);
            let center_x = (bb.mins.x + bb.maxs.x) / 2.0;
            let center_y = (bb.mins.y + bb.maxs.y) / 2.0;

            // Create rotation matrix around center (rotation is in degrees)
            let angle_rad = path.rotation.to_radians();
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();

            // Translate to origin, rotate, translate back
            let to_origin = Matrix4::new_translation(&Vector3::new(-center_x, -center_y, 0.0));
            let rotation_mat = Matrix4::new(
                cos_a, -sin_a, 0.0, 0.0, sin_a, cos_a, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                1.0,
            );
            let from_origin = Matrix4::new_translation(&Vector3::new(center_x, center_y, 0.0));
            let transform = from_origin * rotation_mat * to_origin;

            // Apply rotation to sketch and set rotation to 0 (rotation is now baked into geometry)
            (path.sketch.transform(&transform), 0.0)
        } else {
            (path.sketch.clone(), 0.0)
        }
    } else {
        (shape.as_csg(), 0.0)
    };

    let mp = sketch.to_multipolygon();

    let mut result_sketch = csgrs::sketch::Sketch::new();

    for poly in mp.0 {
        // Convert exterior to Polyline
        let mut ext_pline = Polyline::new();
        for coord in poly.exterior().0.iter() {
            ext_pline.add_vertex(PlineVertex::new(coord.x, coord.y, 0.0));
        }
        ext_pline.set_is_closed(true);
        let ext_pline = clean_polyline(ext_pline);

        // Offset
        let offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            ext_pline.parallel_offset(distance)
        }))
        .unwrap_or_default();

        for offset in offsets {
            let pts: Vec<[f64; 2]> = offset.vertex_data.iter().map(|v| [v.x, v.y]).collect();
            if pts.len() >= 3 {
                result_sketch = result_sketch.union(&csgrs::sketch::Sketch::polygon(&pts, None));
            }
        }

        // Handle interiors (holes)
        for interior in poly.interiors() {
            let mut int_pline = Polyline::new();
            for coord in interior.0.iter() {
                int_pline.add_vertex(PlineVertex::new(coord.x, coord.y, 0.0));
            }
            int_pline.set_is_closed(true);
            let int_pline = clean_polyline(int_pline);

            // Offset holes
            let offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                int_pline.parallel_offset(distance)
            }))
            .unwrap_or_default();

            for offset in offsets {
                let pts: Vec<[f64; 2]> = offset.vertex_data.iter().map(|v| [v.x, v.y]).collect();
                if pts.len() >= 3 {
                    result_sketch =
                        result_sketch.difference(&csgrs::sketch::Sketch::polygon(&pts, None));
                }
            }
        }
    }

    let mut result_path = DesignPath::from_csg(result_sketch);
    result_path.rotation = rotation;
    Shape::Path(result_path)
}
*/

pub fn perform_fillet(shape: &Shape, radius: f64) -> Shape {
    if let Shape::Path(path) = shape {
        if path.closed {
            if let Some(result) = try_closed_path_fillet(path, radius) {
                return Shape::Path(result);
            }
        } else {
            if let Some(result) = try_open_path_fillet(path, radius) {
                return Shape::Path(result);
            }
        }
    }

    // Fillet all corners using the offset trick:
    // 1. Offset inward by radius (rounds convex corners)
    // 2. Offset outward by radius (restores size, keeping rounded corners)
    let inward = perform_offset(shape, -radius);
    perform_offset(&inward, radius)
}

pub fn perform_chamfer(shape: &Shape, distance: f64) -> Shape {
    if let Shape::Path(path) = shape {
        if path.closed {
            if let Some(result) = try_closed_path_chamfer(path, distance) {
                return Shape::Path(result);
            }
        } else {
            if let Some(result) = try_open_path_chamfer(path, distance) {
                return Shape::Path(result);
            }
        }
    }

    // For simple closed polygonal shapes, compute chamfer directly on rendered
    // polyline vertices so the edge distance matches the user-entered value.
    if matches!(shape, Shape::Rectangle(_) | Shape::Triangle(_) | Shape::Polygon(_)) {
        let rendered = shape.render();
        if let Some(points) = extract_closed_polyline_points(&rendered) {
            let chamfered = chamfer_closed_points(&points, distance);
            return Shape::Path(DesignPath::from_points(&chamfered, true));
        }
    }

    // Chamfer using the offset trick + removing bulges (arcs)
    let sketch = shape.as_csg();
    let mp = sketch.to_multipolygon();

    let mut result_sketch = csgrs::sketch::Sketch::new();

    for poly in mp.0 {
        let mut ext_pline = Polyline::new();
        for coord in poly.exterior().0.iter() {
            ext_pline.add_vertex(PlineVertex::new(coord.x, coord.y, 0.0));
        }
        ext_pline.set_is_closed(true);
        let ext_pline = clean_polyline(ext_pline);

        // Offset inward
        let inward_offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            ext_pline.parallel_offset(-distance)
        }))
        .unwrap_or_default();

        for mut inward in inward_offsets {
            // Remove bulges to make it "chamfered"
            for i in 0..inward.vertex_count() {
                if let Some(v) = inward.get(i) {
                    inward.set(i, v.x, v.y, 0.0);
                }
            }

            let inward = clean_polyline(inward);

            // Offset back outward
            let outward_offsets =
                panic::catch_unwind(panic::AssertUnwindSafe(|| inward.parallel_offset(distance)))
                    .unwrap_or_default();

            for mut offset in outward_offsets {
                // Remove bulges again
                for i in 0..offset.vertex_count() {
                    if let Some(v) = offset.get(i) {
                        offset.set(i, v.x, v.y, 0.0);
                    }
                }

                let pts: Vec<[f64; 2]> = offset.vertex_data.iter().map(|v| [v.x, v.y]).collect();
                if pts.len() >= 3 {
                    result_sketch =
                        result_sketch.union(&csgrs::sketch::Sketch::polygon(&pts, None));
                }
            }
        }

        // Handle interiors (holes)
        for interior in poly.interiors() {
            let mut int_pline = Polyline::new();
            for coord in interior.0.iter() {
                int_pline.add_vertex(PlineVertex::new(coord.x, coord.y, 0.0));
            }
            int_pline.set_is_closed(true);
            let int_pline = clean_polyline(int_pline);

            let inward_offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                int_pline.parallel_offset(-distance)
            }))
            .unwrap_or_default();

            for mut inward in inward_offsets {
                for i in 0..inward.vertex_count() {
                    if let Some(v) = inward.get(i) {
                        inward.set(i, v.x, v.y, 0.0);
                    }
                }

                let inward = clean_polyline(inward);

                let outward_offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    inward.parallel_offset(distance)
                }))
                .unwrap_or_default();

                for mut offset in outward_offsets {
                    for i in 0..offset.vertex_count() {
                        if let Some(v) = offset.get(i) {
                            offset.set(i, v.x, v.y, 0.0);
                        }
                    }

                    let pts: Vec<[f64; 2]> =
                        offset.vertex_data.iter().map(|v| [v.x, v.y]).collect();
                    if pts.len() >= 3 {
                        result_sketch =
                            result_sketch.difference(&csgrs::sketch::Sketch::polygon(&pts, None));
                    }
                }
            }
        }
    }

    Shape::Path(DesignPath::from_csg(result_sketch))
}
