use gcodekit5_designer::tool_library::{Tool, ToolLibrary, ToolType};

#[test]
fn test_tool_creation() {
    let tool = Tool::new(
        "test1".to_string(),
        "Test Tool".to_string(),
        ToolType::EndMill,
        3.175,
        2,
        "HSS".to_string(),
    );
    assert_eq!(tool.diameter, 3.175);
    assert_eq!(tool.tool_type, ToolType::EndMill);
}

#[test]
fn test_tool_calculate_passes() {
    let tool = Tool::new(
        "test1".to_string(),
        "Test Tool".to_string(),
        ToolType::EndMill,
        3.175,
        2,
        "HSS".to_string(),
    );
    let passes = tool.calculate_passes(15.0);
    assert_eq!(passes, 3);
}

#[test]
fn test_tool_library_add_and_retrieve() {
    let mut library = ToolLibrary::new();
    let tool = Tool::new(
        "test1".to_string(),
        "Test Tool".to_string(),
        ToolType::EndMill,
        3.175,
        2,
        "HSS".to_string(),
    );
    library.add_tool(tool.clone());

    assert!(library.get_tool("test1").is_some());
    assert_eq!(library.get_tool("test1").unwrap().id, "test1");
}

#[test]
fn test_tool_library_default_tool() {
    let mut library = ToolLibrary::new();
    let tool = Tool::new(
        "test1".to_string(),
        "Test Tool".to_string(),
        ToolType::EndMill,
        3.175,
        2,
        "HSS".to_string(),
    );
    library.add_tool(tool);

    assert!(library.get_default_tool().is_some());
    assert_eq!(library.get_default_tool().unwrap().id, "test1");
}

#[test]
fn test_tool_library_with_defaults() {
    let library = ToolLibrary::with_defaults();
    assert!(library.get_tool("em_125").is_some());
    assert!(library.get_tool("bn_125").is_some());
    assert!(library.get_default_tool().is_some());
}

#[test]
fn test_tool_type_names() {
    assert_eq!(ToolType::EndMill.name(), "End Mill");
    assert_eq!(ToolType::VBit.name(), "V-Bit");
    assert_eq!(ToolType::Drill.name(), "Drill");
}
