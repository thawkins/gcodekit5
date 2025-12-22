use gcodekit5_designer::{FileFormat, SvgImporter};

use std::path::PathBuf;

#[test]
fn test_svg_import_comprehensive() {
    let svg_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="300" height="300" xmlns="http://www.w3.org/2000/svg">
  <rect x="10" y="10" width="50" height="30"/>
  <circle cx="150" cy="30" r="15"/>
  <line x1="10" y1="80" x2="80" y2="80"/>
  <ellipse cx="150" cy="100" rx="40" ry="20"/>
  <polyline points="10,150 30,140 50,160 70,145"/>
  <polygon points="120,140 150,170 90,170"/>
  <path d="M 10 200 L 40 200 L 40 230 L 10 230 Z"/>
</svg>"#;

    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string(svg_content);

    assert!(result.is_ok());
    let design = result.unwrap();

    // Verify format
    assert_eq!(design.format, FileFormat::Svg);

    // Verify dimensions
    assert_eq!(design.dimensions.0, 300.0);
    assert_eq!(design.dimensions.1, 300.0);

    // Verify we imported all shapes
    // rect(1) + circle(1) + line(1) + ellipse(1) + polyline(1) + polygon(1) + path(1) = 7
    // Note: path is imported as a single PathShape
    assert_eq!(design.shapes.len(), 7);
}

#[test]
fn test_svg_import_with_scaling() {
    let svg_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
  <rect x="10" y="10" width="20" height="20"/>
</svg>"#;

    let importer = SvgImporter::new(2.0, 0.0, 0.0);
    let result = importer.import_string(svg_content);

    assert!(result.is_ok());
    let design = result.unwrap();

    // Dimensions should be scaled
    assert_eq!(design.dimensions.0, 200.0);
    assert_eq!(design.dimensions.1, 200.0);

    // Should have one rectangle
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_svg_import_with_offset() {
    let svg_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <circle cx="0" cy="0" r="10"/>
</svg>"#;

    let importer = SvgImporter::new(1.0, 100.0, 50.0);
    let result = importer.import_string(svg_content);

    assert!(result.is_ok());
    let design = result.unwrap();

    // Should have one circle with offset applied
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_svg_import_empty_file() {
    let svg_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"></svg>"#;

    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string(svg_content);

    assert!(result.is_ok());
    let design = result.unwrap();

    // Should succeed but have no shapes
    assert_eq!(design.shapes.len(), 0);
}

#[test]
fn test_svg_import_invalid_xml() {
    let svg_content = "not valid xml";

    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string(svg_content);

    // Should fail with parse error
    assert!(result.is_err());
}

#[test]
fn test_svg_import_path_commands() {
    let svg_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <path d="M 10 10 L 20 10 H 30 V 20 Z"/>
</svg>"#;

    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string(svg_content);

    assert!(result.is_ok());
    let design = result.unwrap();

    // Path with M, L, H, V, Z should create 1 PathShape
    assert_eq!(design.shapes.len(), 1);
}

#[test]
fn test_svg_import_nested_groups() {
    let svg_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <g id="outer">
    <rect x="10" y="10" width="20" height="20"/>
    <g id="inner">
      <circle cx="50" cy="50" r="10"/>
    </g>
  </g>
</svg>"#;

    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string(svg_content);

    assert!(result.is_ok());
    let design = result.unwrap();

    // Should flatten hierarchy and import both shapes
    assert_eq!(design.shapes.len(), 2);
}

#[test]
fn test_svg_import_tigershead_asset_has_shapes() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let svg_path = manifest_dir.join("../../assets/svg/tigershead.svg");
    let svg_content = std::fs::read_to_string(svg_path).expect("read tigershead.svg");

    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let result = importer.import_string(&svg_content);

    assert!(result.is_ok());
    let design = result.unwrap();
    assert!(
        !design.shapes.is_empty(),
        "expected at least one imported shape"
    );
}
