//! Regression test for spline-fit POLYLINE entities (header flag 70 bit 2).
//!
//! These interleave the original control-frame vertices (vertex flag 16) with
//! the generated curve vertices (vertex flag 8). Rendering the control frame
//! produced a zig-zag/triangle artifact connecting curve endpoints back to the
//! control points; only the flag-8 vertices should be kept.

use gcodekit5_designer::dxf_parser::{DxfEntity, DxfParser};

#[test]
fn test_spline_fit_polyline_uses_only_curve_vertices() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/dxf/object_with_elevation.dxf"
    );
    let content = std::fs::read_to_string(path).expect("test asset should exist");

    let file = DxfParser::parse(&content).expect("parse should succeed");

    let polylines: Vec<_> = file
        .entities
        .iter()
        .filter_map(|e| match e {
            DxfEntity::Polyline(p) => Some(p),
            _ => None,
        })
        .collect();

    // LWPOLYLINE (closed rectangle, 4 vertices) + spline-fit POLYLINE.
    assert_eq!(polylines.len(), 2);

    let rectangle = polylines[0];
    assert_eq!(rectangle.vertices.len(), 4);

    // The spline-fit polyline has 4 control-frame vertices (flag 16) and 9
    // curve vertices (flag 8) in the source file; only the 9 curve vertices
    // should survive, so the control frame never gets drawn as stray lines.
    let spline_fit = polylines[1];
    assert_eq!(spline_fit.vertices.len(), 9);
}
