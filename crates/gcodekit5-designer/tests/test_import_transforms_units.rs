//! Regression tests for SVG transforms / viewBox scaling and DXF units / extrusion.

use gcodekit5_designer::model::{DesignerShape, Shape};
use gcodekit5_designer::{DxfImporter, SvgImporter};

const EPS: f64 = 1e-3;

fn assert_bounds(actual: (f64, f64, f64, f64), expected: (f64, f64, f64, f64)) {
    let ok = (actual.0 - expected.0).abs() < EPS
        && (actual.1 - expected.1).abs() < EPS
        && (actual.2 - expected.2).abs() < EPS
        && (actual.3 - expected.3).abs() < EPS;
    assert!(ok, "bounds {actual:?}, expected {expected:?}");
}

fn import_svg(svg: &str) -> Vec<Shape> {
    SvgImporter::new(1.0, 0.0, 0.0)
        .import_string(svg)
        .expect("svg import should succeed")
        .shapes
}

fn import_dxf(dxf: &str) -> Vec<Shape> {
    DxfImporter::new(1.0, 0.0, 0.0)
        .import_string(dxf)
        .expect("dxf import should succeed")
        .shapes
}

#[test]
fn test_svg_nested_group_translate_and_scale() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <g transform="translate(10,20)">
            <g transform="scale(2)">
                <rect x="1" y="1" width="5" height="3"/>
            </g>
        </g>
    </svg>"#;
    let shapes = import_svg(svg);
    assert_eq!(shapes.len(), 1);
    // (1,1)-(6,4) scaled by 2 then shifted by (10,20). A single shape is flipped
    // about its own centre, so its bounds are unchanged.
    assert_bounds(shapes[0].bounds(), (12.0, 22.0, 22.0, 28.0));
}

#[test]
fn test_svg_element_transforms_apply_to_every_element_type() {
    let svg = r#"<svg width="100" height="100">
        <g transform="translate(50 0)">
            <circle cx="0" cy="10" r="2" transform="translate(5,0)"/>
            <ellipse cx="0" cy="10" rx="2" ry="1"/>
            <line x1="0" y1="10" x2="4" y2="10"/>
            <polyline points="0,10 4,10 4,12"/>
            <polygon points="0,10 4,10 4,12"/>
            <path d="M 0 10 L 4 10"/>
        </g>
    </svg>"#;
    let shapes = import_svg(svg);
    assert_eq!(shapes.len(), 6);
    for s in &shapes {
        let (x1, _, _, _) = s.bounds();
        assert!(x1 >= 47.9, "group translate not applied to {s:?}");
    }
    let (cx1, _, cx2, _) = shapes[0].bounds();
    assert!(
        ((cx1 + cx2) / 2.0 - 55.0).abs() < EPS,
        "circle own transform"
    );
}

#[test]
fn test_svg_rotated_rect() {
    let svg = r#"<svg width="100" height="100">
        <rect x="0" y="0" width="10" height="2" transform="rotate(90)"/>
    </svg>"#;
    let shapes = import_svg(svg);
    assert_eq!(shapes.len(), 1);
    // rotate(90) maps (x, y) to (-y, x).
    assert_bounds(shapes[0].bounds(), (-2.0, 0.0, 0.0, 10.0));
    match &shapes[0] {
        Shape::Rectangle(r) => {
            assert!((r.width - 10.0).abs() < EPS && (r.height - 2.0).abs() < EPS);
            assert!(
                (r.rotation.abs() - 90.0).abs() < EPS,
                "rotation {}",
                r.rotation
            );
        }
        other => panic!("expected a rectangle, got {other:?}"),
    }
}

#[test]
fn test_svg_skewed_rect_becomes_path() {
    let svg = r#"<svg width="100" height="100">
        <rect x="0" y="0" width="10" height="10" transform="skewX(45)"/>
    </svg>"#;
    let shapes = import_svg(svg);
    assert_eq!(shapes.len(), 1);
    assert!(matches!(shapes[0], Shape::Path(_)));
    // skewX(45): x' = x + y, so the 10 mm square spans 20 mm in X.
    assert_bounds(shapes[0].bounds(), (0.0, 0.0, 20.0, 10.0));
}

#[test]
fn test_svg_viewbox_with_mm_width_scales_to_mm() {
    let svg = r#"<svg width="50mm" height="25mm" viewBox="0 0 100 50">
        <rect x="0" y="0" width="100" height="50"/>
    </svg>"#;
    let design = SvgImporter::new(1.0, 0.0, 0.0)
        .import_string(svg)
        .expect("svg import should succeed");
    assert_eq!(design.shapes.len(), 1);
    assert_bounds(design.shapes[0].bounds(), (0.0, 0.0, 50.0, 25.0));
    assert!((design.dimensions.0 - 50.0).abs() < EPS);
    assert!((design.dimensions.1 - 25.0).abs() < EPS);
}

#[test]
fn test_svg_viewbox_inch_and_px_units() {
    let inch = import_svg(
        r#"<svg width="2in" height="2in" viewBox="0 0 100 100">
            <line x1="0" y1="0" x2="100" y2="0"/></svg>"#,
    );
    let (x1, _, x2, _) = inch[0].bounds();
    assert!(
        (x2 - x1 - 50.8).abs() < EPS,
        "2 in = 50.8 mm, got {}",
        x2 - x1
    );

    let px = import_svg(
        r#"<svg width="96px" height="96px" viewBox="0 0 96 96">
            <line x1="0" y1="0" x2="96" y2="0"/></svg>"#,
    );
    let (x1, _, x2, _) = px[0].bounds();
    assert!(
        (x2 - x1 - 25.4).abs() < EPS,
        "96 px = 25.4 mm, got {}",
        x2 - x1
    );

    let cm = import_svg(
        r#"<svg width="10cm" height="10cm" viewBox="-50 0 100 100">
            <line x1="-50" y1="0" x2="50" y2="0"/></svg>"#,
    );
    let (x1, _, x2, _) = cm[0].bounds();
    assert!(x1.abs() < EPS && (x2 - 100.0).abs() < EPS, "{x1}..{x2}");
}

#[test]
fn test_svg_defs_content_is_not_imported() {
    let svg = r#"<svg width="100" height="100">
        <defs><rect id="r" x="0" y="0" width="5" height="5"/></defs>
        <circle cx="10" cy="10" r="3"/>
    </svg>"#;
    let shapes = import_svg(svg);
    assert_eq!(shapes.len(), 1);
    assert!(matches!(shapes[0], Shape::Circle(_)));
}

fn dxf_with_units(insunits: i32, entities: &str) -> String {
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n{insunits}\n0\nENDSEC\n\
         0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n"
    )
}

const UNIT_LINE: &str = "0\nLINE\n8\n0\n10\n0.0\n20\n0.0\n11\n1.0\n21\n0.0\n";

fn line_length(dxf: &str) -> f64 {
    let shapes = import_dxf(dxf);
    assert_eq!(shapes.len(), 1);
    let (x1, _, x2, _) = shapes[0].bounds();
    x2 - x1
}

#[test]
fn test_dxf_insunits_scale_to_mm() {
    for (code, mm) in [
        (1, 25.4),
        (2, 304.8),
        (4, 1.0),
        (5, 10.0),
        (6, 1000.0),
        (0, 1.0),
    ] {
        let len = line_length(&dxf_with_units(code, UNIT_LINE));
        // Geometry is stored in single precision, so compare relatively.
        assert!(
            (len - mm).abs() < mm * 1e-6 + 1e-6,
            "$INSUNITS {code}: {len} mm"
        );
    }
}

#[test]
fn test_dxf_without_header_is_millimetres() {
    let dxf = format!("0\nSECTION\n2\nENTITIES\n{UNIT_LINE}0\nENDSEC\n0\nEOF\n");
    assert!((line_length(&dxf) - 1.0).abs() < 1e-6);
}

#[test]
fn test_dxf_negative_extrusion_mirrors_circle_and_arc() {
    let circle =
        "0\nCIRCLE\n8\n0\n10\n10.0\n20\n5.0\n30\n0.0\n40\n2.0\n210\n0.0\n220\n0.0\n230\n-1.0\n";
    let shapes = import_dxf(&dxf_with_units(4, circle));
    let (x1, y1, x2, y2) = shapes[0].bounds();
    assert!(
        ((x1 + x2) / 2.0 + 10.0).abs() < 0.01,
        "centre x {}",
        (x1 + x2) / 2.0
    );
    assert!(((y1 + y2) / 2.0 - 5.0).abs() < 0.01);

    // OCS arc 0..90 degrees around the origin lies in world quadrant II after mirroring.
    let arc = "0\nARC\n8\n0\n10\n0.0\n20\n0.0\n30\n0.0\n40\n10.0\n50\n0.0\n51\n90.0\n230\n-1.0\n";
    let shapes = import_dxf(&dxf_with_units(4, arc));
    assert_bounds(shapes[0].bounds(), (-10.0, 0.0, 0.0, 10.0));
}

#[test]
fn test_dxf_negative_extrusion_mirrors_lwpolyline() {
    let poly = "0\nLWPOLYLINE\n8\n0\n90\n3\n70\n0\n10\n1.0\n20\n0.0\n10\n5.0\n20\n0.0\n10\n5.0\n20\n3.0\n210\n0.0\n220\n0.0\n230\n-1.0\n";
    let shapes = import_dxf(&dxf_with_units(4, poly));
    assert_bounds(shapes[0].bounds(), (-5.0, 0.0, -1.0, 3.0));
}

#[test]
fn test_dxf_positive_extrusion_is_unchanged() {
    let circle = "0\nCIRCLE\n8\n0\n10\n10.0\n20\n5.0\n40\n2.0\n230\n1.0\n";
    let shapes = import_dxf(&dxf_with_units(4, circle));
    let (x1, _, x2, _) = shapes[0].bounds();
    assert!(((x1 + x2) / 2.0 - 10.0).abs() < 0.01);
}
