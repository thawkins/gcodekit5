//! Tests for `gcodekit5_ui::ui::tools_manager_backend` (moved from the inline test module).

use gcodekit5_core::data::tools::{Tool, ToolId, ToolType};
use gcodekit5_ui::ui::tools_manager_backend::*;

#[test]
fn test_backend_creation() {
    let backend = ToolsManagerBackend::new();
    assert!(!backend.get_all_tools().is_empty());
}

#[test]
fn test_tool_type_conversion() {
    assert_eq!(
        string_to_tool_type("Flat End Mill"),
        Some(ToolType::EndMillFlat)
    );
    assert_eq!(string_to_tool_type("Drill Bit"), Some(ToolType::DrillBit));
}

#[test]
fn test_search_tools() {
    let backend = ToolsManagerBackend::new();
    let results = backend.search_tools("end");
    assert!(!results.is_empty());
}

#[test]
fn test_persistence() {
    // Create a test tool
    let test_tool = Tool::new(
        ToolId("test_persist_tool".to_string()),
        999, // tool number
        "Test Persist Tool".to_string(),
        ToolType::EndMillFlat,
        6.35, // diameter
        38.0, // length
    );

    // Add and save
    {
        let mut backend = ToolsManagerBackend::new();
        let mut tool = test_tool.clone();
        tool.custom = true;
        backend.add_tool(tool);
    }

    // Create new backend and verify tool was loaded
    {
        let backend = ToolsManagerBackend::new();
        let loaded = backend.get_tool(&ToolId("test_persist_tool".to_string()));
        assert!(loaded.is_some());
        assert_eq!(loaded.expect("load failed").name, "Test Persist Tool");
    }

    // Cleanup
    {
        let mut backend = ToolsManagerBackend::new();
        backend.remove_tool(&ToolId("test_persist_tool".to_string()));
    }
}
