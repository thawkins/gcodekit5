use gcodekit5_designer::import::SvgImporter;
use gcodekit5_designer::Rectangle;

#[test]
fn test_svg_import_mirroring() {
    // Create SVG with two rectangles
    // Rect 1: y=10, h=10 (10-20)
    // Rect 2: y=30, h=10 (30-40)
    // BBox: 10-40. Center = 25.
    // Expected after mirror:
    // Rect 1: 30-40
    // Rect 2: 10-20

    let svg_content = r#"
        <svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <rect x="0" y="10" width="10" height="10" />
            <rect x="0" y="30" width="10" height="10" />
        </svg>
    "#;

    let importer = SvgImporter::new(1.0, 0.0, 0.0);
    let design = importer
        .import_string(svg_content)
        .expect("Failed to import SVG");

    assert_eq!(design.shapes.len(), 2);

    // We don't know the order, so we check bounds
    let mut found_rect1_moved = false;
    let mut found_rect2_moved = false;

    for shape in design.shapes {
        if let Some(rect) = shape.as_any().downcast_ref::<Rectangle>() {
            // Mirror is around y=25; centers should be at 35 and 15 after mirroring
            let center_y = rect.center.y;
            if (center_y - 35.0).abs() < 0.001 && (rect.height - 10.0).abs() < 0.001 {
                found_rect1_moved = true;
            }
            if (center_y - 15.0).abs() < 0.001 && (rect.height - 10.0).abs() < 0.001 {
                found_rect2_moved = true;
            }
        }
    }

    assert!(found_rect1_moved, "Did not find Rect 1 moved to y=30");
    assert!(found_rect2_moved, "Did not find Rect 2 moved to y=10");
}
