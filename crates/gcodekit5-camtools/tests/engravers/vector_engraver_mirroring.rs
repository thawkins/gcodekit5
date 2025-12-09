use gcodekit5_camtools::vector_engraver::{VectorEngraver, VectorEngravingParameters};
use std::io::Write;
// removed unused import of lyon::math::point

#[test]
fn test_svg_mirroring() {
    // Create a temporary SVG file
    let file_name = "temp_mirror_test.svg";
    let mut file = std::fs::File::create(file_name).unwrap();
    
    // SVG with two paths at top and bottom
    // Path 1: (10, 10) to (20, 10) -> Y=10
    // Path 2: (10, 90) to (20, 90) -> Y=90
    // Bounding box Y: 10 to 90. Center Y = 50.
    // Expected after mirror:
    // Path 1 -> Y = 2*50 - 10 = 90
    // Path 2 -> Y = 2*50 - 90 = 10
    
    file.write_all(br#"
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <path d="M 10 10 L 20 10" />
  <path d="M 10 90 L 20 90" />
</svg>
"#).unwrap();

    let params = VectorEngravingParameters::default();
    let engraver = VectorEngraver::from_file(file_name, params).unwrap();
    
    // Clean up
    std::fs::remove_file(file_name).unwrap();
    
    assert_eq!(engraver.paths.len(), 2);
    
    // Check coordinates of first path
    // Original: (10, 10) -> (20, 10)
    // Expected: (10, 90) -> (20, 90)
    let path1 = &engraver.paths[0];
    let mut p1_y = 0.0;
    for event in path1.iter() {
        match event {
            lyon::path::Event::Begin { at } => {
                p1_y = at.y;
            }
            _ => {}
        }
    }
    
    // Check coordinates of second path
    // Original: (10, 90) -> (20, 90)
    // Expected: (10, 10) -> (20, 10)
    let path2 = &engraver.paths[1];
    let mut p2_y = 0.0;
    for event in path2.iter() {
        match event {
            lyon::path::Event::Begin { at } => {
                p2_y = at.y;
            }
            _ => {}
        }
    }
    
    // Allow small float error
    assert!((p1_y - 90.0).abs() < 0.1, "Path 1 Y should be 90.0, got {}", p1_y);
    assert!((p2_y - 10.0).abs() < 0.1, "Path 2 Y should be 10.0, got {}", p2_y);
}
