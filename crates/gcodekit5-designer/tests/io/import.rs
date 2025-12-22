use gcodekit5_designer::import::{DxfImporter, FileFormat, SvgImporter};

#[test]
fn test_svg_importer_creation() {
    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    assert_eq!(importer.scale, 1.0);
}

#[test]
fn test_dxf_importer_creation() {
    let importer = DxfImporter::new(1.0, 0.0, 0.0);
    assert_eq!(importer.scale, 1.0);
}

#[test]
fn test_svg_import_basic() {
    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let svg = r#"<svg width="100" height="100"></svg>"#;
    let result = importer.import_string(svg);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Svg);
    assert_eq!(design.dimensions.0, 100.0);
    assert_eq!(design.dimensions.1, 100.0);
}

#[test]
fn test_svg_import_rectangle() {
    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let svg = r#"<svg><rect x="10" y="20" width="30" height="40"/></svg>"#;
    let result = importer.import_string(svg);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_svg_import_circle() {
    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let svg = r#"<svg><circle cx="50" cy="50" r="25"/></svg>"#;
    let result = importer.import_string(svg);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_svg_import_line() {
    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let svg = r#"<svg><line x1="0" y1="0" x2="100" y2="100"/></svg>"#;
    let result = importer.import_string(svg);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_svg_import_with_scale() {
    let importer = SvgImporter::new(2.0, 0.0, 0.0);
    let svg = r#"<svg width="100" height="100"></svg>"#;
    let result = importer.import_string(svg);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.dimensions.0, 200.0);
    assert_eq!(design.dimensions.1, 200.0);
}

#[test]
fn test_dxf_import_framework() {
    let importer = DxfImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string("0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF");

    assert!(result.is_ok());
    let design = result.unwrap();
    assert_eq!(design.format, FileFormat::Dxf);
}
