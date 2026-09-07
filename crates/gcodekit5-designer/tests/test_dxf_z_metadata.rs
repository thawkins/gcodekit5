//! Regression tests for DXF Z metadata handling: ACIS solid detection and
//! elevation/thickness capture for flat 2D entities (Phase 1 of Z preservation).

use gcodekit5_designer::dxf_parser::DxfParser;
use gcodekit5_designer::import::DxfImporter;

/// ACIS-based solids (3DSOLID/BODY/REGION) cannot be parsed without a CAD
/// kernel; they must be tracked as unsupported and not crash the parser.
#[test]
fn test_dxf_acis_entities_are_tracked_and_discarded() {
    let content = "0\nSECTION\n2\nENTITIES\n\
0\nLINE\n8\n0\n10\n0.0\n20\n0.0\n11\n10.0\n21\n0.0\n\
0\n3DSOLID\n8\n0\n1\nsomeacisdata\n\
0\nBODY\n8\n0\n1\nsomeacisdata\n\
0\n3DSOLID\n8\n0\n1\nsomeacisdata\n\
0\nENDSEC\n0\nEOF\n";

    let file = DxfParser::parse(content).expect("parse should succeed");

    // The LINE entity is still parsed normally.
    assert_eq!(file.entity_count(), 1);

    assert_eq!(file.unsupported_entities.get("3DSOLID"), Some(&2));
    assert_eq!(file.unsupported_entities.get("BODY"), Some(&1));

    let importer = DxfImporter::new(1.0, 0.0, 0.0);
    let design = importer.import_string(content).expect("import should succeed");
    let mut unsupported = design.unsupported_entities.clone();
    unsupported.sort();
    assert_eq!(
        unsupported,
        vec![("3DSOLID".to_string(), 2), ("BODY".to_string(), 1)]
    );
}

/// LWPOLYLINE elevation (group 38) and thickness (group 39) must be read
/// instead of silently discarded.
#[test]
fn test_dxf_lwpolyline_elevation_and_thickness_are_captured() {
    let content = "0\nSECTION\n2\nENTITIES\n\
0\nLWPOLYLINE\n8\n0\n90\n2\n70\n0\n38\n5.0\n39\n2.5\n\
10\n0.0\n20\n0.0\n10\n10.0\n20\n0.0\n\
0\nENDSEC\n0\nEOF\n";

    let file = DxfParser::parse(content).expect("parse should succeed");
    assert_eq!(file.entity_count(), 1);

    match &file.entities[0] {
        gcodekit5_designer::dxf_parser::DxfEntity::Polyline(p) => {
            assert!((p.elevation - 5.0).abs() < 1e-9);
            assert!((p.thickness - 2.5).abs() < 1e-9);
        }
        other => panic!("expected Polyline entity, got {:?}", other),
    }

    let importer = DxfImporter::new(1.0, 0.0, 0.0);
    let design = importer.import_string(content).expect("import should succeed");
    assert_eq!(design.z_metadata.len(), design.shapes.len());
    assert_eq!(design.z_metadata[0], (5.0, 2.5));
}

/// CIRCLE elevation (group 30) and thickness (group 39) must be read.
#[test]
fn test_dxf_circle_elevation_and_thickness_are_captured() {
    let content = "0\nSECTION\n2\nENTITIES\n\
0\nCIRCLE\n8\n0\n10\n5.0\n20\n5.0\n30\n3.0\n40\n2.0\n39\n1.0\n\
0\nENDSEC\n0\nEOF\n";

    let file = DxfParser::parse(content).expect("parse should succeed");
    match &file.entities[0] {
        gcodekit5_designer::dxf_parser::DxfEntity::Circle(c) => {
            assert!((c.elevation - 3.0).abs() < 1e-9);
            assert!((c.thickness - 1.0).abs() < 1e-9);
        }
        other => panic!("expected Circle entity, got {:?}", other),
    }
}

/// Entities without elevation/thickness codes default to (0.0, 0.0), so
/// plain 2D DXF files keep behaving exactly as before this change.
#[test]
fn test_dxf_entities_without_z_metadata_default_to_zero() {
    let content = "0\nSECTION\n2\nENTITIES\n\
0\nLINE\n8\n0\n10\n0.0\n20\n0.0\n11\n10.0\n21\n0.0\n\
0\nENDSEC\n0\nEOF\n";

    let importer = DxfImporter::new(1.0, 0.0, 0.0);
    let design = importer.import_string(content).expect("import should succeed");
    assert_eq!(design.z_metadata, vec![(0.0, 0.0)]);
    assert!(design.unsupported_entities.is_empty());
}
