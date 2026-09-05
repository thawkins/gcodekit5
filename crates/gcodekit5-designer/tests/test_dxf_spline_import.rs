//! Regression tests for SPLINE entity import.
//!
//! AutoCAD 2000+ DXF exports (AC1015 and later, covering the common
//! "2010"/"2018" DXF versions) frequently store curved geometry as SPLINE
//! entities instead of flattening it to POLYLINE segments the way older R12
//! (AC1009) exports do. Before SPLINE support was added, all such geometry
//! was silently dropped on import.

use gcodekit5_designer::dxf_parser::{DxfEntity, DxfParser};

/// A degree-1 (linear) clamped B-spline should evaluate to a straight line
/// between its two control points, regardless of the NURBS machinery used.
#[test]
fn test_dxf_spline_linear_degree1() {
    let content = "0\nSECTION\n2\nENTITIES\n\
0\nSPLINE\n8\n0\n70\n0\n71\n1\n\
40\n0.0\n40\n0.0\n40\n1.0\n40\n1.0\n\
10\n0.0\n20\n0.0\n\
10\n10.0\n20\n0.0\n\
0\nENDSEC\n0\nEOF\n";

    let file = DxfParser::parse(content).expect("parse should succeed");
    assert_eq!(file.entity_count(), 1);

    match &file.entities[0] {
        DxfEntity::Polyline(p) => {
            assert!(p.vertices.len() >= 2, "expected sampled points for spline");
            let first = p.vertices.first().unwrap();
            let last = p.vertices.last().unwrap();
            assert!((first.x - 0.0).abs() < 1e-6);
            assert!((last.x - 10.0).abs() < 1e-6);
            for v in &p.vertices {
                assert!(v.y.abs() < 1e-6, "linear spline should stay on Y=0");
            }
        }
        other => panic!("expected Polyline entity, got {:?}", other),
    }
}

/// A degree-3 clamped B-spline (the common AutoCAD case) should be sampled
/// into a smooth multi-point polyline rather than being dropped.
#[test]
fn test_dxf_spline_cubic_is_sampled() {
    let content = "0\nSECTION\n2\nENTITIES\n\
0\nSPLINE\n8\n0\n70\n0\n71\n3\n\
40\n0.0\n40\n0.0\n40\n0.0\n40\n0.0\n40\n1.0\n40\n1.0\n40\n1.0\n40\n1.0\n\
10\n0.0\n20\n0.0\n\
10\n0.0\n20\n10.0\n\
10\n10.0\n20\n10.0\n\
10\n10.0\n20\n0.0\n\
0\nENDSEC\n0\nEOF\n";

    let file = DxfParser::parse(content).expect("parse should succeed");
    assert_eq!(file.entity_count(), 1);

    match &file.entities[0] {
        DxfEntity::Polyline(p) => {
            assert!(
                p.vertices.len() >= 32,
                "expected a densely sampled curve, got {} points",
                p.vertices.len()
            );
            let first = p.vertices.first().unwrap();
            let last = p.vertices.last().unwrap();
            assert!((first.x - 0.0).abs() < 1e-6 && (first.y - 0.0).abs() < 1e-6);
            assert!((last.x - 10.0).abs() < 1e-6 && (last.y - 0.0).abs() < 1e-6);
        }
        other => panic!("expected Polyline entity, got {:?}", other),
    }
}

/// Real-world AutoCAD 2010 (AC1024) export that previously imported with all
/// SPLINE geometry silently missing.
#[test]
fn test_dxf_ac2010_splines_are_imported() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/dxf/7997964_889-Ac2010.dxf"
    );
    let content = std::fs::read_to_string(path).expect("test asset should exist");
    let file = DxfParser::parse(&content).expect("parse should succeed");

    let spline_polylines = file
        .entities
        .iter()
        .filter_map(|e| match e {
            DxfEntity::Polyline(p) if p.vertices.len() > 2 => Some(p.vertices.len()),
            _ => None,
        })
        .count();

    assert!(
        spline_polylines > 500,
        "expected hundreds of sampled spline curves, got {}",
        spline_polylines
    );
}

/// Real-world AutoCAD 2018 (AC1032) export, same check as the 2010 case.
#[test]
fn test_dxf_ac2018_splines_are_imported() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/dxf/7997964_889-Ac2018.dxf"
    );
    let content = std::fs::read_to_string(path).expect("test asset should exist");
    let file = DxfParser::parse(&content).expect("parse should succeed");

    let spline_polylines = file
        .entities
        .iter()
        .filter_map(|e| match e {
            DxfEntity::Polyline(p) if p.vertices.len() > 2 => Some(p.vertices.len()),
            _ => None,
        })
        .count();

    assert!(
        spline_polylines > 500,
        "expected hundreds of sampled spline curves, got {}",
        spline_polylines
    );
}
