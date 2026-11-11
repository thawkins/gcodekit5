//! Tests for `gcodekit5_ui::ui::materials_manager_backend` (moved from the inline test module).

use gcodekit5_core::data::materials::{Material, MaterialCategory, MaterialId};
use gcodekit5_ui::ui::materials_manager_backend::*;

/// A storage file of this test's own under `target/temp`, so no test reads or
/// overwrites the user's `custom_materials.json` (PR #99 review). Each test and process
/// gets a fresh directory.
fn test_storage(test: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/temp/materials_manager_backend")
        .join(format!("{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create test storage dir");
    dir.join("custom_materials.json")
}

#[test]
fn test_backend_creation() {
    let storage = test_storage("test_backend_creation");
    let backend = MaterialsManagerBackend::with_storage_path(storage.clone());
    assert!(!backend.get_all_materials().is_empty());
}

#[test]
fn test_category_conversion() {
    assert_eq!(string_to_category("Wood"), Some(MaterialCategory::Wood));
    assert_eq!(
        string_to_category("Non-Ferrous Metal"),
        Some(MaterialCategory::NonFerrousMetal)
    );
}

#[test]
fn test_search_materials() {
    let storage = test_storage("test_search_materials");
    let backend = MaterialsManagerBackend::with_storage_path(storage.clone());
    let results = backend.search_materials("oak");
    assert!(!results.is_empty());
}

#[test]
fn test_persistence() {
    let storage = test_storage("test_persistence");
    // Create a test material
    let test_material = Material::new(
        MaterialId("test_persist".to_string()),
        "Test Persist Material".to_string(),
        MaterialCategory::Wood,
        "Test".to_string(),
    );

    // Add and save
    {
        let mut backend = MaterialsManagerBackend::with_storage_path(storage.clone());
        let mut material = test_material.clone();
        material.custom = true;
        backend.add_material(material);
    }

    assert!(
        storage.exists(),
        "custom entries are saved to the test file"
    );

    // Create new backend and verify material was loaded
    {
        let backend = MaterialsManagerBackend::with_storage_path(storage.clone());
        let loaded = backend.get_material(&MaterialId("test_persist".to_string()));
        assert!(loaded.is_some());
        assert_eq!(loaded.expect("load failed").name, "Test Persist Material");
    }

    // Cleanup
    {
        let mut backend = MaterialsManagerBackend::with_storage_path(storage.clone());
        backend.remove_material(&MaterialId("test_persist".to_string()));
    }
}
