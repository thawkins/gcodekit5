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
        backend
            .remove_tool(&ToolId("test_persist_tool".to_string()))
            .expect("remove");
    }
}

fn custom_tool(id: &str, name: &str) -> Tool {
    let mut tool = Tool::new(
        ToolId(id.to_string()),
        998,
        name.to_string(),
        ToolType::EndMillFlat,
        3.175,
        38.0,
    );
    tool.custom = true;
    tool
}

#[test]
fn test_save_tool_writes_and_replaces() {
    let storage = test_storage("test_save_tool_writes_and_replaces");
    let mut backend = ToolsManagerBackend::with_storage_path(storage.clone());
    backend
        .save_tool(custom_tool("save_tool_ok", "First"))
        .expect("save");
    backend
        .save_tool(custom_tool("save_tool_ok", "Second"))
        .expect("replace");
    let reloaded = ToolsManagerBackend::with_storage_path(storage);
    let tool = reloaded
        .get_tool(&ToolId("save_tool_ok".to_string()))
        .expect("tool on disk");
    assert_eq!(tool.name, "Second");
}

#[test]
fn test_save_tool_reports_write_failure_and_rolls_back() {
    // PR #102 review: a failed write was only logged, so Save looked successful
    // and the edit was lost on restart.
    let storage = test_storage("test_save_tool_reports_write_failure_and_rolls_back");
    let mut backend = ToolsManagerBackend::with_storage_path(storage.clone());
    backend
        .save_tool(custom_tool("save_tool_fail", "Original"))
        .expect("first save");

    // A directory where the file should be makes every write fail.
    std::fs::remove_file(&storage).expect("remove file");
    std::fs::create_dir(&storage).expect("block path");

    let result = backend.save_tool(custom_tool("save_tool_fail", "Edited"));
    assert!(result.is_err(), "write failure must be reported");
    let tool = backend
        .get_tool(&ToolId("save_tool_fail".to_string()))
        .expect("tool kept");
    assert_eq!(
        tool.name, "Original",
        "memory rolled back to the saved tool"
    );

    let result = backend.save_tool(custom_tool("save_tool_new", "New"));
    assert!(result.is_err());
    assert!(
        backend
            .get_tool(&ToolId("save_tool_new".to_string()))
            .is_none(),
        "an unsaved new tool is not kept"
    );
}

#[test]
fn test_remove_tool_reports_write_failure_and_keeps_tool() {
    // PR #102 review: a failed write on delete was only logged, so the tool
    // vanished from the list but came back on restart.
    let storage = test_storage("test_remove_tool_reports_write_failure_and_keeps_tool");
    let mut backend = ToolsManagerBackend::with_storage_path(storage.clone());
    let id = ToolId("remove_tool_fail".to_string());
    backend
        .save_tool(custom_tool("remove_tool_fail", "Keep me"))
        .expect("save");

    std::fs::remove_file(&storage).expect("remove file");
    std::fs::create_dir(&storage).expect("block path");

    assert!(backend.remove_tool(&id).is_err(), "write failure reported");
    assert_eq!(
        backend.get_tool(&id).map(|t| t.name.as_str()),
        Some("Keep me"),
        "tool put back after the failed write"
    );

    // Once the file can be written again, the delete goes through.
    std::fs::remove_dir(&storage).expect("unblock path");
    let removed = backend.remove_tool(&id).expect("remove");
    assert_eq!(removed.map(|t| t.name), Some("Keep me".to_string()));
    assert!(backend.get_tool(&id).is_none());
}
