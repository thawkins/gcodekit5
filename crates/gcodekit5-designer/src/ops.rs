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
        let first = pline.get(0).unwrap();
        let last = pline.get(pline.vertex_count() - 1).unwrap();
        if (first.x - last.x).abs() < 1e-5 && (first.y - last.y).abs() < 1e-5 {
            pline.remove(pline.vertex_count() - 1);
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
    let sketch = shape.as_csg();
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

    Shape::Path(DesignPath::from_csg(result_sketch))
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
                let v = inward.get(i).unwrap();
                inward.set(i, v.x, v.y, 0.0);
            }

            let inward = clean_polyline(inward);

            // Offset back outward
            let outward_offsets =
                panic::catch_unwind(panic::AssertUnwindSafe(|| inward.parallel_offset(distance)))
                    .unwrap_or_default();

            for mut offset in outward_offsets {
                // Remove bulges again
                for i in 0..offset.vertex_count() {
                    let v = offset.get(i).unwrap();
                    offset.set(i, v.x, v.y, 0.0);
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
                    let v = inward.get(i).unwrap();
                    inward.set(i, v.x, v.y, 0.0);
                }

                let inward = clean_polyline(inward);

                let outward_offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    inward.parallel_offset(distance)
                }))
                .unwrap_or_default();

                for mut offset in outward_offsets {
                    for i in 0..offset.vertex_count() {
                        let v = offset.get(i).unwrap();
                        offset.set(i, v.x, v.y, 0.0);
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
