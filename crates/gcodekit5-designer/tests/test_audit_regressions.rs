//! Regression tests for defects found during the designer crate audit.
//!
//! Covers rotation/scale pivots, hit-testing of rotated shapes, line rendering,
//! G-code modal/line-number output, Bezier-to-arc fitting, shape ID allocation,
//! save/load round-trips, mirror undo, and SVG/DXF import parsing.

use gcodekit5_core::Units;
use gcodekit5_designer::designer_state::MachineMode;
use gcodekit5_designer::model::{
    DesignCircle, DesignGear, DesignLine, DesignPath, DesignPolygon, DesignRectangle,
    DesignTriangle, DesignerShape, Point, Shape,
};
use gcodekit5_designer::serialization::DesignFile;
use gcodekit5_designer::{
    DesignerState, DxfImporter, SvgImporter, Toolpath, ToolpathGenerator, ToolpathSegment,
    ToolpathSegmentType, ToolpathToGcode,
};

fn bounds_center(b: (f64, f64, f64, f64)) -> (f64, f64) {
    ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0)
}

fn path_bounds(path: &lyon::path::Path) -> (f64, f64, f64, f64) {
    let bb = lyon::algorithms::aabb::bounding_box(path.iter());
    (bb.min.x as f64, bb.min.y as f64, bb.max.x as f64, bb.max.y as f64)
}

#[test]
fn test_circle_rotated_render_stays_at_center() {
    let mut c = DesignCircle::new(Point::new(50.0, 50.0), 10.0);
    c.rotation = 45.0;
    let (cx, cy) = bounds_center(path_bounds(&c.render()));
    assert!((cx - 50.0).abs() < 0.01 && (cy - 50.0).abs() < 0.01, "({cx}, {cy})");
}

#[test]
fn test_gear_rotated_render_stays_at_center() {
    let mut g = DesignGear::new(Point::new(100.0, 40.0), 2.0, 20);
    let unrotated = bounds_center(path_bounds(&g.render()));
    g.rotation = 30.0;
    let rotated = bounds_center(path_bounds(&g.render()));
    assert!((unrotated.0 - rotated.0).abs() < 1.0, "{unrotated:?} vs {rotated:?}");
    assert!((unrotated.1 - rotated.1).abs() < 1.0, "{unrotated:?} vs {rotated:?}");
}

#[test]
fn test_shape_rotate_uses_pivot() {
    // Rotating a circle 90 degrees about (0, 0) moves (10, 0) to (0, 10).
    let mut s = Shape::Circle(DesignCircle::new(Point::new(10.0, 0.0), 1.0));
    s.rotate(std::f64::consts::FRAC_PI_2, 0.0, 0.0);
    let (cx, cy) = bounds_center(s.bounds());
    assert!(cx.abs() < 0.01 && (cy - 10.0).abs() < 0.01, "({cx}, {cy})");
}

#[test]
fn test_path_scale_keeps_pivot() {
    let mut p = DesignPath::from_points(
        &[
            Point::new(10.0, 10.0),
            Point::new(20.0, 10.0),
            Point::new(20.0, 20.0),
            Point::new(10.0, 20.0),
        ],
        true,
    );
    p.scale(2.0, 2.0, Point::new(15.0, 15.0));
    let (x1, y1, x2, y2) = p.bounds();
    assert!((x1 - 5.0).abs() < 0.01 && (y1 - 5.0).abs() < 0.01);
    assert!((x2 - 25.0).abs() < 0.01 && (y2 - 25.0).abs() < 0.01);
}

#[test]
fn test_path_transform_does_not_double_rotate() {
    let mut p = DesignPath::from_points(
        &[Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(10.0, 5.0)],
        true,
    );
    // Rotation is baked into the geometry, so the render-time rotation stays 0.
    p.transform(&lyon::math::Transform::rotation(lyon::math::Angle::degrees(30.0)));
    assert!(p.rotation.abs() < 1e-9);
    // Mirroring must not turn into a 180 degree render rotation either.
    p.scale(-1.0, 1.0, Point::new(5.0, 0.0));
    assert!(p.rotation.abs() < 1e-9);
}

#[test]
fn test_rotated_rectangle_hit_test_uses_degrees() {
    let mut r = DesignRectangle::new(-50.0, -5.0, 100.0, 10.0);
    r.rotation = 90.0;
    assert!(r.contains_point(Point::new(0.0, 40.0), 0.0));
    assert!(!r.contains_point(Point::new(40.0, 0.0), 0.0));
}

#[test]
fn test_rotated_polygon_and_triangle_hit_test_use_degrees() {
    let mut p = DesignPolygon::new(Point::new(0.0, 0.0), 10.0, 4);
    // Unrotated square has vertices on the axes; at 45 degrees (9, 0) is outside.
    p.rotation = 45.0;
    assert!(!p.contains_point(Point::new(9.0, 0.0), 0.0));
    assert!(p.contains_point(Point::new(6.0, 0.0), 0.0));

    let mut t = DesignTriangle::new(Point::new(0.0, 0.0), 20.0, 20.0);
    t.rotation = 180.0;
    // Right angle corner (bottom-left) is now at top-right.
    assert!(t.contains_point(Point::new(9.0, 9.0), 0.0));
    assert!(!t.contains_point(Point::new(-9.0, -9.0), 0.0));
}

#[test]
fn test_line_render_and_hit_test_use_endpoints() {
    let line = DesignLine::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    let (x1, y1, x2, y2) = path_bounds(&line.render());
    assert!(x1.abs() < 1e-3 && y1.abs() < 1e-3 && (x2 - 10.0).abs() < 1e-3 && (y2 - 10.0).abs() < 1e-3);
    assert!(line.contains_point(Point::new(10.0, 10.0), 0.1));
    assert!(!line.contains_point(Point::new(10.0, 0.0), 0.1));
}

#[test]
fn test_gcode_no_duplicate_line_numbers_and_g01_after_arc() {
    let gen = ToolpathToGcode::with_line_numbers(Units::MM, 5.0, true);
    let mut tp = Toolpath::new(3.0, -1.0);
    tp.add_segment(ToolpathSegment::new(
        ToolpathSegmentType::RapidMove,
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        500.0,
        1000,
    ));
    tp.add_segment(ToolpathSegment::new_arc(
        ToolpathSegmentType::ArcCCW,
        Point::new(10.0, 0.0),
        Point::new(0.0, 10.0),
        Point::new(0.0, 0.0),
        500.0,
        1000,
    ));
    tp.add_segment(ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        Point::new(0.0, 10.0),
        Point::new(0.0, 20.0),
        500.0,
        1000,
    ));
    let body = gen.generate_body(&tp, 10);
    for line in body.lines() {
        assert_eq!(line.matches('N').count(), 1, "duplicated line number: {line}");
    }
    let lines: Vec<&str> = body.lines().collect();
    let arc_idx = lines.iter().position(|l| l.contains("G03")).expect("arc emitted");
    let next = lines[arc_idx + 1];
    assert!(next.contains("G01"), "linear move after arc must restate G01: {next}");
}

fn path_contour_has_arc(path: &DesignPath) -> bool {
    let mut gen = ToolpathGenerator::new();
    gen.set_feed_rate(100.0);
    gen.set_cut_depth(1.0);
    gen.generate_path_contour(path, 1.0)
        .iter()
        .flat_map(|tp| tp.segments.iter())
        .any(|s| {
            matches!(
                s.segment_type,
                ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW
            )
        })
}

#[test]
fn test_straight_cubic_is_not_converted_to_arc() {
    // A straight cubic (controls on the endpoints) must not become a semicircle.
    let mut b = lyon::path::Path::builder();
    b.begin(lyon::math::point(0.0, 0.0));
    b.cubic_bezier_to(
        lyon::math::point(0.0, 0.0),
        lyon::math::point(10.0, 0.0),
        lyon::math::point(10.0, 0.0),
    );
    b.end(false);
    assert!(!path_contour_has_arc(&DesignPath::from_lyon_path(&b.build())));

    // A symmetric quadratic is a parabola, not an arc.
    let mut b = lyon::path::Path::builder();
    b.begin(lyon::math::point(0.0, 0.0));
    b.quadratic_bezier_to(lyon::math::point(5.0, 5.0), lyon::math::point(10.0, 0.0));
    b.end(false);
    assert!(!path_contour_has_arc(&DesignPath::from_lyon_path(&b.build())));

    // A genuine quarter-circle cubic is still emitted as an arc.
    let k = 10.0 * 0.552_284_8;
    let mut b = lyon::path::Path::builder();
    b.begin(lyon::math::point(10.0, 0.0));
    b.cubic_bezier_to(
        lyon::math::point(10.0, k),
        lyon::math::point(k, 10.0),
        lyon::math::point(0.0, 10.0),
    );
    b.end(false);
    assert!(path_contour_has_arc(&DesignPath::from_lyon_path(&b.build())));
}

#[test]
fn test_shape_ids_not_reused_after_load() {
    let mut state = DesignerState::new();
    let a = state.canvas.add_rectangle(0.0, 0.0, 10.0, 10.0);
    let b = state.canvas.add_rectangle(20.0, 0.0, 10.0, 10.0);
    let objs: Vec<_> = state.canvas.shapes().cloned().collect();
    state.canvas.clear();
    for o in objs {
        state.canvas.restore_shape(o);
    }
    let c = state.canvas.add_rectangle(40.0, 0.0, 10.0, 10.0);
    assert!(c != a && c != b);
    assert_eq!(state.canvas.shapes().count(), 3);
}

#[test]
fn test_serialization_preserves_center_and_hole_radius() {
    let mut state = DesignerState::new();
    let pid = state.canvas.add_polygon(Point::new(30.0, 40.0), 10.0, 5);
    let mut gear = DesignGear::new(Point::new(0.0, 0.0), 2.0, 15);
    gear.hole_radius = 3.0;
    let gid = state.canvas.add_shape(Shape::Gear(gear));

    let p = DesignFile::from_drawing_object(state.canvas.get_shape(pid).unwrap());
    let restored = DesignFile::to_drawing_object(&p, pid as i32).unwrap();
    match restored.shape {
        Shape::Polygon(poly) => {
            assert!((poly.center.x - 30.0).abs() < 1e-9 && (poly.center.y - 40.0).abs() < 1e-9)
        }
        _ => panic!("expected polygon"),
    }

    let g = DesignFile::from_drawing_object(state.canvas.get_shape(gid).unwrap());
    let restored = DesignFile::to_drawing_object(&g, gid as i32).unwrap();
    match restored.shape {
        Shape::Gear(gear) => assert!((gear.hole_radius - 3.0).abs() < 1e-9),
        _ => panic!("expected gear"),
    }
}

#[test]
fn test_mirror_triangle_is_undoable() {
    let mut state = DesignerState::new();
    let id = state.canvas.add_triangle(Point::new(10.0, 10.0), 20.0, 10.0);
    state.canvas.select_shape(id, false);
    state.mirror_selected_x();
    let mirrored = match &state.canvas.get_shape(id).unwrap().shape {
        Shape::Triangle(t) => t.right_angle_corner,
        _ => unreachable!(),
    };
    // Horizontal flip swaps left/right: bottom-left (0) -> bottom-right (1).
    assert_eq!(mirrored, 1);
    state.undo();
    match &state.canvas.get_shape(id).unwrap().shape {
        Shape::Triangle(t) => assert_eq!(t.right_angle_corner, 0),
        _ => unreachable!(),
    }
}

#[test]
fn test_cnc_gcode_uses_tool_settings_feed() {
    let mut state = DesignerState::new();
    state.set_machine_mode(MachineMode::Cnc3D);
    state.canvas.add_rectangle(0.0, 0.0, 10.0, 10.0);
    let gcode = state.generate_gcode(None);
    assert!(!gcode.contains(" F0\n"), "feed must not be zero:\n{gcode}");
    assert!(gcode.contains("F2000"), "expected tool settings feed:\n{gcode}");
}

#[test]
fn test_svg_import_attribute_and_tag_matching() {
    let svg = r#"<svg width="100" height="100">
        <linearGradient id="g" x1="0" y1="0" x2="1" y2="1"></linearGradient>
        <rect rx="2" x="10" y="10" stroke-width="3" width="20" height="5"/>
        <path id="p1" d="M 0 0 L 10 0 L 10 10 Z"/>
    </svg>"#;
    let design = SvgImporter::new(1.0, 0.0, 0.0).import_string(svg).unwrap();
    // No phantom line from <linearGradient>; rect and path both imported.
    assert_eq!(design.shapes.len(), 2, "{:?}", design.shapes);
    let rect = design
        .shapes
        .iter()
        .find_map(|s| match s {
            Shape::Rectangle(r) => Some(r.clone()),
            _ => None,
        })
        .expect("rect imported");
    assert!((rect.width - 20.0).abs() < 1e-9);
    assert!(design.shapes.iter().any(|s| matches!(s, Shape::Path(_))));
}

#[test]
fn test_dxf_closed_lwpolyline_closing_bulge() {
    // Closed polyline (0,0)->(10,0) with a semicircular bulge on the closing segment.
    let dxf = "0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n8\n0\n90\n2\n70\n1\n10\n0.0\n20\n0.0\n10\n10.0\n20\n0.0\n42\n1.0\n0\nENDSEC\n0\nEOF\n";
    let design = DxfImporter::new(1.0, 0.0, 0.0).import_string(dxf).unwrap();
    assert_eq!(design.shapes.len(), 1);
    let (_, y1, _, y2) = design.shapes[0].bounds();
    // The closing arc has radius 5, so the shape spans 5 mm in Y (not a flat line).
    assert!((y2 - y1) > 4.9, "closing bulge ignored: y-span {}", y2 - y1);
}
