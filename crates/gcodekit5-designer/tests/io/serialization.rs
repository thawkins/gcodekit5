use gcodekit5_designer::pocket_operations::PocketStrategy;
use gcodekit5_designer::serialization::{DesignFile, ShapeData};

#[test]
fn test_create_new_design() {
    let design = DesignFile::new("Test Design");
    assert_eq!(design.version, "1.0");
    assert_eq!(design.metadata.name, "Test Design");
    assert_eq!(design.shapes.len(), 0);
}

#[test]
fn test_save_and_load() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_design.gck4");

    let mut design = DesignFile::new("Test");
    design.shapes.push(ShapeData {
        id: 1,
        shape_type: "rectangle".to_string(),
        name: "My Rect".to_string(),
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 50.0,
        points: Vec::new(),
        selected: false,
        use_custom_values: false,
        operation_type: "profile".to_string(),
        pocket_depth: 0.0,
        step_down: 0.0,
        step_in: 0.0,
        start_depth: 0.0,
        text_content: String::new(),
        font_size: 0.0,
        font_family: String::new(),
        font_bold: false,
        font_italic: false,
        path_data: String::new(),
        group_id: None,
        corner_radius: 0.0,
        is_slot: false,
        rotation: 0.0,
        ramp_angle: 0.0,
        pocket_strategy: PocketStrategy::ContourParallel,
        raster_fill_ratio: 0.5,
        sides: 0,
        teeth: 0,
        module: 0.0,
        pressure_angle: 0.0,
        pitch: 0.0,
        roller_diameter: 0.0,
        thickness: 0.0,
        depth: 0.0,
        tab_size: 0.0,
        offset: 0.0,
        fillet: 0.0,
        chamfer: 0.0,
        lock_aspect_ratio: true,
    });

    design.save_to_file(&file_path).expect("save failed");
    let loaded = DesignFile::load_from_file(&file_path).expect("load failed");

    assert_eq!(loaded.shapes.len(), 1);
    assert_eq!(loaded.shapes[0].width, 100.0);
    assert_eq!(loaded.shapes[0].name, "My Rect");

    std::fs::remove_file(&file_path).ok();
}

#[test]
fn test_round_trip_all_shape_types() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_all_shapes.gck4");

    let mut design = DesignFile::new("All Shapes Test");

    // Add various shape types
    let shape_types = vec![
        ("rectangle", 1),
        ("circle", 2),
        ("ellipse", 3),
        ("line", 4),
        ("triangle", 5),
        ("polygon", 6),
    ];

    for (shape_type, id) in shape_types {
        design.shapes.push(create_test_shape(id, shape_type));
    }

    design.save_to_file(&file_path).expect("save failed");
    let loaded = DesignFile::load_from_file(&file_path).expect("load failed");

    assert_eq!(loaded.shapes.len(), 6);
    assert_eq!(loaded.shapes[0].shape_type, "rectangle");
    assert_eq!(loaded.shapes[1].shape_type, "circle");
    assert_eq!(loaded.shapes[2].shape_type, "ellipse");
    assert_eq!(loaded.shapes[3].shape_type, "line");
    assert_eq!(loaded.shapes[4].shape_type, "triangle");
    assert_eq!(loaded.shapes[5].shape_type, "polygon");

    std::fs::remove_file(&file_path).ok();
}

#[test]
fn test_round_trip_toolpath_params() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_toolpath_params.gck4");

    let mut design = DesignFile::new("Toolpath Test");
    design.toolpath_params.feed_rate = 2000.0;
    design.toolpath_params.spindle_speed = 15000.0;
    design.toolpath_params.tool_diameter = 6.0;
    design.toolpath_params.cut_depth = -10.0;
    design.toolpath_params.stock_width = 300.0;
    design.toolpath_params.stock_height = 200.0;
    design.toolpath_params.stock_thickness = 20.0;
    design.toolpath_params.safe_z_height = 15.0;

    design.save_to_file(&file_path).expect("save failed");
    let loaded = DesignFile::load_from_file(&file_path).expect("load failed");

    assert!((loaded.toolpath_params.feed_rate - 2000.0).abs() < 0.001);
    assert!((loaded.toolpath_params.spindle_speed - 15000.0).abs() < 0.001);
    assert!((loaded.toolpath_params.tool_diameter - 6.0).abs() < 0.001);
    assert!((loaded.toolpath_params.cut_depth - (-10.0)).abs() < 0.001);
    assert!((loaded.toolpath_params.stock_width - 300.0).abs() < 0.001);
    assert!((loaded.toolpath_params.stock_height - 200.0).abs() < 0.001);
    assert!((loaded.toolpath_params.stock_thickness - 20.0).abs() < 0.001);
    assert!((loaded.toolpath_params.safe_z_height - 15.0).abs() < 0.001);

    std::fs::remove_file(&file_path).ok();
}

#[test]
fn test_round_trip_viewport() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_viewport.gck4");

    let mut design = DesignFile::new("Viewport Test");
    design.viewport.zoom = 2.5;
    design.viewport.pan_x = 150.0;
    design.viewport.pan_y = -75.0;

    design.save_to_file(&file_path).expect("save failed");
    let loaded = DesignFile::load_from_file(&file_path).expect("load failed");

    assert!((loaded.viewport.zoom - 2.5).abs() < 0.001);
    assert!((loaded.viewport.pan_x - 150.0).abs() < 0.001);
    assert!((loaded.viewport.pan_y - (-75.0)).abs() < 0.001);

    std::fs::remove_file(&file_path).ok();
}

#[test]
fn test_load_nonexistent_file() {
    let result = DesignFile::load_from_file("/nonexistent/path/file.gck4");
    assert!(result.is_err());
}

#[test]
fn test_load_invalid_json() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_invalid.gck4");

    std::fs::write(&file_path, "{ invalid json }").expect("write failed");

    let result = DesignFile::load_from_file(&file_path);
    assert!(result.is_err());

    std::fs::remove_file(&file_path).ok();
}

fn create_test_shape(id: i32, shape_type: &str) -> ShapeData {
    ShapeData {
        id,
        shape_type: shape_type.to_string(),
        name: format!("Test {}", shape_type),
        x: 10.0 * id as f64,
        y: 10.0 * id as f64,
        width: 50.0,
        height: 50.0,
        points: Vec::new(),
        selected: false,
        use_custom_values: false,
        operation_type: "profile".to_string(),
        pocket_depth: 0.0,
        step_down: 0.0,
        step_in: 0.0,
        start_depth: 0.0,
        text_content: String::new(),
        font_size: 0.0,
        font_family: String::new(),
        font_bold: false,
        font_italic: false,
        path_data: String::new(),
        group_id: None,
        corner_radius: 0.0,
        is_slot: false,
        rotation: 0.0,
        ramp_angle: 0.0,
        pocket_strategy: PocketStrategy::ContourParallel,
        raster_fill_ratio: 0.5,
        sides: if shape_type == "polygon" { 6 } else { 0 },
        teeth: 0,
        module: 0.0,
        pressure_angle: 0.0,
        pitch: 0.0,
        roller_diameter: 0.0,
        thickness: 0.0,
        depth: 0.0,
        tab_size: 0.0,
        offset: 0.0,
        fillet: 0.0,
        chamfer: 0.0,
        lock_aspect_ratio: true,
    }
}
