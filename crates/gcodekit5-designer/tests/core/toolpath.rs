use gcodekit5_designer::toolpath::ToolpathGenerator;
use gcodekit5_designer::shapes::Rectangle;

#[test]
fn test_toolpath_generator_rectangle() {
    let gen = ToolpathGenerator::new();
    let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
    let toolpaths = gen.generate_rectangle_contour(&rect, 0.0);
    let toolpath = &toolpaths[0];

    assert!(toolpath.segments.len() > 0);
    assert_eq!(toolpath.tool_diameter, 3.175);
    assert_eq!(toolpath.depth, -5.0);
}

#[test]
fn test_toolpath_total_length() {
    let gen = ToolpathGenerator::new();
    let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
    let toolpaths = gen.generate_rectangle_contour(&rect, 0.0);
    let toolpath = &toolpaths[0];

    let length = toolpath.total_length();
    assert!(length > 0.0);
}
