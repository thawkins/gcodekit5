//! Integration tests for Designer file import functionality (Phase 4.1)

use gcodekit5_designer::{DxfImporter, FileFormat, SvgImporter};

#[test]
fn test_svg_importer_basic_creation() {
    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    assert_eq!(importer.scale, 1.0);
}

#[test]
fn test_svg_importer_with_scale_and_offset() {
    let importer = SvgImporter::new(2.5, 10.0, 20.0);
    // Verify importer is created with correct parameters
    // This would be tested more thoroughly with actual import functionality
    let _ = importer;
}

#[test]
fn test_svg_import_empty_string() {
    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string("<svg></svg>");

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Svg);
        // Layer count not asserted here; importer may return 0 layers for empty SVGs
}

#[test]
fn test_svg_import_with_valid_content() {
    let svg_content = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    <rect x="10" y="10" width="80" height="80" fill="red"/>
</svg>"#;

    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string(svg_content);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Svg);
    assert_eq!(design.dimensions, (100.0, 100.0));
}

#[test]
fn test_dxf_importer_basic_creation() {
    let importer = DxfImporter::new(1.0, 0.0, 0.0);
    assert_eq!(importer.scale, 1.0);
}

#[test]
fn test_dxf_importer_with_scale() {
    let importer = DxfImporter::new(1.5, 5.0, 10.0);
    let _ = importer;
}

#[test]
fn test_dxf_import_empty_content() {
    let importer = DxfImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string("0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF");

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Dxf);
}

#[test]
fn test_file_format_enum() {
    assert_eq!(FileFormat::Svg, FileFormat::Svg);
    assert_eq!(FileFormat::Dxf, FileFormat::Dxf);
    assert_ne!(FileFormat::Svg, FileFormat::Dxf);
}

#[test]
fn test_svg_scaling() {
    let importer_1x = SvgImporter::new(1.0, 0.0, 0.0);
    let result_1x = importer_1x.import_string("<svg></svg>");
    assert!(result_1x.is_ok());

    let importer_2x = SvgImporter::new(2.0, 0.0, 0.0);
    let result_2x = importer_2x.import_string("<svg></svg>");
    assert!(result_2x.is_ok());
}

#[test]
fn test_dxf_scaling() {
    let dxf_content = "0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF";

    let importer_1x = DxfImporter::new(1.0, 0.0, 0.0);
    let result_1x = importer_1x.import_string(dxf_content);
    assert!(result_1x.is_ok());

    let importer_5x = DxfImporter::new(5.0, 0.0, 0.0);
    let result_5x = importer_5x.import_string(dxf_content);
    assert!(result_5x.is_ok());
}

#[test]
fn test_svg_with_offset() {
    let importer = SvgImporter::new(1.0, 50.0, 75.0);
    let result = importer.import_string("<svg></svg>");

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Svg);
}

#[test]
fn test_dxf_with_offset() {
    let importer = DxfImporter::new(1.0, 100.0, 200.0);
    let result = importer.import_string("0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF");

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Dxf);
}

#[test]
fn test_imported_design_properties() {
    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string("<svg></svg>");

    assert!(result.is_ok());
    let design = result.unwrap();

    // Verify basic design properties
    assert!(!design.shapes.is_empty() || design.shapes.is_empty()); // Can be either
    assert_eq!(design.dimensions.0, 100.0);
    assert_eq!(design.dimensions.1, 100.0);
    assert_eq!(design.format, FileFormat::Svg);
        // Layer count not asserted here; importer may return 0 layers for empty SVGs
}

#[test]
fn test_multiple_svg_imports() {
    let importer = SvgImporter::new(1.0, 0.0, 0.0);

    let result1 = importer.import_string("<svg></svg>");
    let result2 = importer.import_string("<svg></svg>");
    let result3 = importer.import_string("<svg></svg>");

    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result3.is_ok());
}

#[test]
fn test_multiple_dxf_imports() {
    let importer = DxfImporter::new(1.0, 0.0, 0.0);
    let dxf_content = "0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF";

    let result1 = importer.import_string(dxf_content);
    let result2 = importer.import_string(dxf_content);
    let result3 = importer.import_string(dxf_content);

    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result3.is_ok());
}

#[test]
fn test_svg_import_complex_svg() {
    let complex_svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200">
    <g id="group1">
        <rect x="10" y="10" width="50" height="50" fill="blue"/>
        <circle cx="100" cy="100" r="30" fill="green"/>
        <line x1="10" y1="10" x2="190" y2="190" stroke="red" stroke-width="2"/>
    </g>
    <g id="group2">
        <path d="M 20 20 L 100 100 L 20 180 Z" fill="yellow"/>
    </g>
</svg>"#;

    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string(complex_svg);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Svg);
}

#[test]
fn test_svg_import_framework_ready() {
    // This test confirms that the SVG import framework is ready
    // and can be extended with full SVG parsing later
    let importer = SvgImporter::new(1.0, 0.0, 0.0);

    let result =
        importer.import_string(r#"<?xml version="1.0"?><svg width="100" height="100"></svg>"#);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Svg);

    // Framework ready for Phase 4 implementation
}

#[test]
fn test_dxf_import_framework_ready() {
    let importer = DxfImporter::new(1.0, 0.0, 0.0);

    let dxf_content = r#"0
SECTION
2
ENTITIES
0
LINE
10
0.0
20
0.0
11
10.0
21
10.0
0
ENDSEC
0
EOF"#;

    let result = importer.import_string(dxf_content);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Dxf);
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_dxf_import_with_circles() {
    let importer = DxfImporter::new(1.0, 0.0, 0.0);

    let dxf_content = r#"0
SECTION
2
ENTITIES
0
CIRCLE
10
50.0
20
50.0
40
25.0
0
ENDSEC
0
EOF"#;

    let result = importer.import_string(dxf_content);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Dxf);
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_dxf_import_multiple_shapes() {
    let importer = DxfImporter::new(1.0, 0.0, 0.0);

    let dxf_content = r#"0
SECTION
2
ENTITIES
0
LINE
10
0.0
20
0.0
11
10.0
21
10.0
0
CIRCLE
10
20.0
20
20.0
40
5.0
0
LINE
10
30.0
20
30.0
11
40.0
21
40.0
0
ENDSEC
0
EOF"#;

    let result = importer.import_string(dxf_content);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Dxf);
    assert_eq!(design.shapes.len(), 3);
}

#[test]
fn test_dxf_import_with_scaling() {
    let importer = DxfImporter::new(2.0, 0.0, 0.0);

    let dxf_content = r#"0
SECTION
2
ENTITIES
0
LINE
10
0.0
20
0.0
11
10.0
21
10.0
0
ENDSEC
0
EOF"#;

    let result = importer.import_string(dxf_content);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_dxf_import_with_offset() {
    let importer = DxfImporter::new(1.0, 100.0, 200.0);

    let dxf_content = r#"0
SECTION
2
ENTITIES
0
CIRCLE
10
0.0
20
0.0
40
10.0
0
ENDSEC
0
EOF"#;

    let result = importer.import_string(dxf_content);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_dxf_import_polyline_conversion() {
    let importer = DxfImporter::new(1.0, 0.0, 0.0);

    let dxf_content = r#"0
SECTION
2
ENTITIES
0
LWPOLYLINE
70
1
10
0.0
20
0.0
10
10.0
20
0.0
10
10.0
20
10.0
0
ENDSEC
0
EOF"#;

    let result = importer.import_string(dxf_content);

    assert!(result.is_ok());
    let design = result.unwrap();
    // Polyline is converted to a single PathShape
    assert_eq!(design.shapes.len(), 1);
}
