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
    });

    design.save_to_file(&file_path).unwrap();
    let loaded = DesignFile::load_from_file(&file_path).unwrap();

    assert_eq!(loaded.shapes.len(), 1);
    assert_eq!(loaded.shapes[0].width, 100.0);
    assert_eq!(loaded.shapes[0].name, "My Rect");

    std::fs::remove_file(&file_path).ok();
}
