//! Tests for `gcodekit5_ui::ui::tools_manager_backend` (moved from the inline test module).

use gcodekit5_core::data::tools::{Tool, ToolId, ToolType};
use gcodekit5_ui::ui::tools_manager_backend::*;

/// A storage file of this test's own under `target/temp`, so no test reads or
/// overwrites the user's `custom_tools.json` (PR #99 review). Each test and process
/// gets a fresh directory.
fn test_storage(test: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/temp/tools_manager_backend")
        .join(format!("{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create test storage dir");
    dir.join("custom_tools.json")
}

#[test]
fn test_backend_creation() {
    let storage = test_storage("test_backend_creation");
    let backend = ToolsManagerBackend::with_storage_path(storage.clone());
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
    let storage = test_storage("test_search_tools");
    let backend = ToolsManagerBackend::with_storage_path(storage.clone());
    let results = backend.search_tools("end");
    assert!(!results.is_empty());
}

#[test]
fn test_persistence() {
    let storage = test_storage("test_persistence");
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
        let mut backend = ToolsManagerBackend::with_storage_path(storage.clone());
        let mut tool = test_tool.clone();
        tool.custom = true;
        backend.add_tool(tool);
    }

    assert!(
        storage.exists(),
        "custom entries are saved to the test file"
    );

    // Create new backend and verify tool was loaded
    {
        let backend = ToolsManagerBackend::with_storage_path(storage.clone());
        let loaded = backend.get_tool(&ToolId("test_persist_tool".to_string()));
        assert!(loaded.is_some());
        assert_eq!(loaded.expect("load failed").name, "Test Persist Tool");
    }

    // Cleanup
    {
        let mut backend = ToolsManagerBackend::with_storage_path(storage.clone());
        backend.remove_tool(&ToolId("test_persist_tool".to_string()));
    }
}
