// Tests for toolpath generation with various shape combinations and parameters

use gcodekit5_designer::model::{DesignGear, DesignPolygon, DesignSprocket, DesignTriangle};
use gcodekit5_designer::{Circle, Line, Point, Rectangle, ToolpathGenerator, ToolpathSegmentType};

/// Helper: create a configured toolpath generator
fn configured_gen() -> ToolpathGenerator {
    let mut gen = ToolpathGenerator::new();
    gen.set_feed_rate(200.0);
    gen.set_spindle_speed(5000);
    gen.set_tool_diameter(3.175);
    gen.set_cut_depth(-3.0);
    gen.set_start_depth(0.0);
    gen
}

// ── Contour generation for each shape type ────────────────────────────

#[test]
fn test_rectangle_contour_has_segments() {
    let gen = configured_gen();
    let rect = Rectangle::new(10.0, 10.0, 40.0, 20.0);
    let toolpaths = gen.generate_rectangle_contour(&rect, 1.0);
    assert!(!toolpaths.is_empty());
    assert!(!toolpaths[0].segments.is_empty());
}

#[test]
fn test_circle_contour_has_segments() {
    let gen = configured_gen();
    let circle = Circle::new(Point::new(50.0, 50.0), 15.0);
    let toolpaths = gen.generate_circle_contour(&circle, 1.0);
    assert!(!toolpaths.is_empty());
    assert!(!toolpaths[0].segments.is_empty());
}

#[test]
fn test_line_contour_has_segments() {
    let gen = configured_gen();
    let line = Line::new(Point::new(0.0, 0.0), Point::new(50.0, 30.0));
    let toolpaths = gen.generate_line_contour(&line, 1.0);
    assert!(!toolpaths.is_empty());
    assert!(!toolpaths[0].segments.is_empty());
}

#[test]
fn test_triangle_contour_has_segments() {
    let gen = configured_gen();
    let tri = DesignTriangle::new(Point::new(50.0, 50.0), 40.0, 30.0);
    let toolpaths = gen.generate_triangle_contour(&tri, 1.0);
    assert!(!toolpaths.is_empty());
    assert!(!toolpaths[0].segments.is_empty());
}

#[test]
fn test_polygon_contour_has_segments() {
    let gen = configured_gen();
    let poly = DesignPolygon::new(Point::new(50.0, 50.0), 20.0, 6);
    let toolpaths = gen.generate_polygon_contour(&poly, 1.0);
    assert!(!toolpaths.is_empty());
    assert!(!toolpaths[0].segments.is_empty());
}

#[test]
fn test_gear_contour_has_segments() {
    let gen = configured_gen();
    let gear = DesignGear::new(Point::new(50.0, 50.0), 2.0, 20);
    let toolpaths = gen.generate_gear_contour(&gear, 1.0);
    assert!(!toolpaths.is_empty());
    assert!(!toolpaths[0].segments.is_empty());
}

#[test]
fn test_sprocket_contour_has_segments() {
    let gen = configured_gen();
    let sprocket = DesignSprocket::new(Point::new(50.0, 50.0), 12.7, 15);
    let toolpaths = gen.generate_sprocket_contour(&sprocket, 1.0);
    assert!(!toolpaths.is_empty());
    assert!(!toolpaths[0].segments.is_empty());
}

// ── Polyline contour ──────────────────────────────────────────────────

#[test]
fn test_polyline_contour_square() {
    let gen = configured_gen();
    let vertices = vec![
        Point::new(0.0, 0.0),
        Point::new(40.0, 0.0),
        Point::new(40.0, 40.0),
        Point::new(0.0, 40.0),
    ];
    let toolpaths = gen.generate_polyline_contour(&vertices, &[], 1.0);
    assert!(!toolpaths.is_empty());
    assert!(!toolpaths[0].segments.is_empty());
}

#[test]
fn test_polyline_contour_star() {
    let gen = configured_gen();
    let vertices: Vec<Point> = (0..10)
        .map(|i| {
            let angle = std::f64::consts::PI * 2.0 * (i as f64) / 10.0;
            let r = if i % 2 == 0 { 30.0 } else { 15.0 };
            Point::new(50.0 + r * angle.cos(), 50.0 + r * angle.sin())
        })
        .collect();
    let toolpaths = gen.generate_polyline_contour(&vertices, &[], 1.0);
    assert!(!toolpaths.is_empty());
}

// ── Pocket generation ─────────────────────────────────────────────────

#[test]
fn test_rectangle_pocket_has_segments() {
    let gen = configured_gen();
    let rect = Rectangle::new(10.0, 10.0, 40.0, 20.0);
    let toolpaths = gen.generate_rectangle_pocket(&rect, 3.0, 1.0, 1.5);
    assert!(!toolpaths.is_empty());
    for tp in &toolpaths {
        assert!(!tp.segments.is_empty());
    }
}

#[test]
fn test_circle_pocket_has_segments() {
    let gen = configured_gen();
    let circle = Circle::new(Point::new(50.0, 50.0), 15.0);
    let toolpaths = gen.generate_circle_pocket(&circle, 3.0, 1.0, 1.5);
    assert!(!toolpaths.is_empty());
}

#[test]
fn test_triangle_pocket_has_segments() {
    let gen = configured_gen();
    let tri = DesignTriangle::new(Point::new(50.0, 50.0), 40.0, 30.0);
    let toolpaths = gen.generate_triangle_pocket(&tri, 3.0, 1.0, 1.5);
    assert!(!toolpaths.is_empty());
}

#[test]
fn test_polygon_pocket_has_segments() {
    let gen = configured_gen();
    let poly = DesignPolygon::new(Point::new(50.0, 50.0), 20.0, 6);
    let toolpaths = gen.generate_polygon_pocket(&poly, 3.0, 1.0, 1.5);
    assert!(!toolpaths.is_empty());
}

// ── Tool parameter variations ─────────────────────────────────────────

#[test]
fn test_different_tool_diameters() {
    let rect = Rectangle::new(10.0, 10.0, 40.0, 20.0);
    for diameter in [1.0, 3.175, 6.35, 12.7] {
        let mut gen = configured_gen();
        gen.set_tool_diameter(diameter);
        let toolpaths = gen.generate_rectangle_contour(&rect, 1.0);
        assert!(!toolpaths.is_empty(), "Failed for diameter {}", diameter);
        assert_eq!(toolpaths[0].tool_diameter, diameter);
    }
}

#[test]
fn test_different_feed_rates() {
    let rect = Rectangle::new(10.0, 10.0, 40.0, 20.0);
    for feed_rate in [50.0, 150.0, 300.0, 1000.0] {
        let mut gen = configured_gen();
        gen.set_feed_rate(feed_rate);
        let toolpaths = gen.generate_rectangle_contour(&rect, 1.0);
        assert!(!toolpaths.is_empty(), "Failed for feed_rate {}", feed_rate);
    }
}

#[test]
fn test_different_cut_depths() {
    let rect = Rectangle::new(10.0, 10.0, 40.0, 20.0);
    for depth in [-1.0, -3.0, -5.0, -10.0] {
        let mut gen = configured_gen();
        gen.set_cut_depth(depth);
        let toolpaths = gen.generate_rectangle_contour(&rect, 0.0);
        assert!(!toolpaths.is_empty(), "Failed for depth {}", depth);
        // The final pass should reach the target depth
        let last = toolpaths.last().unwrap();
        assert!(
            (last.depth - depth).abs() < 0.01,
            "Final pass depth {} != target {} for cut_depth {}",
            last.depth,
            depth,
            depth
        );
    }
}

// ── Step-down multipass ───────────────────────────────────────────────

#[test]
fn test_multipass_step_down() {
    let gen = configured_gen();
    let rect = Rectangle::new(10.0, 10.0, 40.0, 20.0);
    let toolpaths = gen.generate_rectangle_contour(&rect, 1.0);
    // With 3mm depth and 1mm step_down, we should get multiple passes
    assert!(
        toolpaths.len() >= 1,
        "Should generate at least one toolpath pass"
    );
}

#[test]
fn test_zero_step_down_single_pass() {
    let gen = configured_gen();
    let rect = Rectangle::new(10.0, 10.0, 40.0, 20.0);
    let toolpaths = gen.generate_rectangle_contour(&rect, 0.0);
    // With step_down=0 should get at least one pass
    assert!(!toolpaths.is_empty());
}

// ── Segment type verification ─────────────────────────────────────────

#[test]
fn test_contour_contains_linear_moves() {
    let gen = configured_gen();
    let rect = Rectangle::new(10.0, 10.0, 40.0, 20.0);
    let toolpaths = gen.generate_rectangle_contour(&rect, 0.0);
    let has_linear = toolpaths[0]
        .segments
        .iter()
        .any(|s| s.segment_type == ToolpathSegmentType::LinearMove);
    assert!(has_linear, "Rectangle contour should contain linear moves");
}

#[test]
fn test_circle_contour_has_arc_or_linear() {
    let gen = configured_gen();
    let circle = Circle::new(Point::new(50.0, 50.0), 15.0);
    let toolpaths = gen.generate_circle_contour(&circle, 0.0);
    let has_motion = toolpaths[0].segments.iter().any(|s| {
        s.segment_type == ToolpathSegmentType::LinearMove
            || s.segment_type == ToolpathSegmentType::ArcCW
            || s.segment_type == ToolpathSegmentType::ArcCCW
    });
    assert!(has_motion, "Circle contour should have motion segments");
}

// ── Toolpath length sanity checks ─────────────────────────────────────

#[test]
fn test_rectangle_contour_length_reasonable() {
    let gen = configured_gen();
    let rect = Rectangle::new(0.0, 0.0, 40.0, 20.0);
    let toolpaths = gen.generate_rectangle_contour(&rect, 0.0);
    let length = toolpaths[0].total_length();
    // Perimeter of 40x20 is 120mm, toolpath should be at least that
    assert!(
        length >= 100.0,
        "Rectangle contour length {} too short",
        length
    );
}

#[test]
fn test_circle_contour_length_reasonable() {
    let gen = configured_gen();
    let circle = Circle::new(Point::new(0.0, 0.0), 20.0);
    let toolpaths = gen.generate_circle_contour(&circle, 0.0);
    let length = toolpaths[0].total_length();
    // Circumference of r=20 is ~125.6mm
    assert!(
        length >= 100.0,
        "Circle contour length {} too short",
        length
    );
}

// ── Rounded rectangle ─────────────────────────────────────────────────

#[test]
fn test_rounded_rectangle_contour() {
    let gen = configured_gen();
    let mut rect = Rectangle::new(10.0, 10.0, 40.0, 20.0);
    rect.corner_radius = 5.0;
    let toolpaths = gen.generate_rectangle_contour(&rect, 0.0);
    assert!(!toolpaths.is_empty());
    assert!(!toolpaths[0].segments.is_empty());
}

// ── Different polygon sides ───────────────────────────────────────────

#[test]
fn test_polygon_various_sides() {
    let gen = configured_gen();
    for sides in [3, 4, 5, 6, 8, 12] {
        let poly = DesignPolygon::new(Point::new(50.0, 50.0), 20.0, sides);
        let toolpaths = gen.generate_polygon_contour(&poly, 0.0);
        assert!(
            !toolpaths.is_empty(),
            "Failed for polygon with {} sides",
            sides
        );
        assert!(!toolpaths[0].segments.is_empty());
    }
}

// ── Gear parameter variations ─────────────────────────────────────────

#[test]
fn test_gear_various_teeth() {
    let gen = configured_gen();
    for teeth in [8, 12, 20, 32] {
        let gear = DesignGear::new(Point::new(50.0, 50.0), 2.0, teeth);
        let toolpaths = gen.generate_gear_contour(&gear, 0.0);
        assert!(
            !toolpaths.is_empty(),
            "Failed for gear with {} teeth",
            teeth
        );
    }
}
