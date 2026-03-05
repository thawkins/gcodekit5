//! # Boolean Operations
//!
//! Provides CSG boolean operations (union, difference, intersection)
//! on designer shapes using the `csgrs` and `cavalier_contours` libraries.
//! Includes polyline cleaning utilities for robust boolean results.

use crate::model::{DesignPath, DesignerShape, Shape};
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use csgrs::traits::CSG;
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

pub fn perform_fillet(shape: &Shape, radius: f64) -> Shape {
    // Fillet all corners using the offset trick:
    // 1. Offset inward by radius (rounds convex corners)
    // 2. Offset outward by radius (restores size, keeping rounded corners)
    let inward = perform_offset(shape, -radius);
    perform_offset(&inward, radius)
}

pub fn perform_chamfer(shape: &Shape, distance: f64) -> Shape {
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
