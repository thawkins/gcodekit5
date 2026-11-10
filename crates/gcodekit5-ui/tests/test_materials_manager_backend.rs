//! Tests for `gcodekit5_ui::ui::materials_manager_backend` (moved from the inline test module).

use gcodekit5_core::data::materials::{Material, MaterialCategory, MaterialId};
use gcodekit5_ui::ui::materials_manager_backend::*;

#[test]
fn test_backend_creation() {
    let backend = MaterialsManagerBackend::new();
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
    let backend = MaterialsManagerBackend::new();
    let results = backend.search_materials("oak");
    assert!(!results.is_empty());
}

#[test]
fn test_persistence() {
    // Create a test material
    let test_material = Material::new(
        MaterialId("test_persist".to_string()),
        "Test Persist Material".to_string(),
        MaterialCategory::Wood,
        "Test".to_string(),
    );

    // Add and save
    {
        let mut backend = MaterialsManagerBackend::new();
        let mut material = test_material.clone();
        material.custom = true;
        backend.add_material(material);
    }

    // Create new backend and verify material was loaded
    {
        let backend = MaterialsManagerBackend::new();
        let loaded = backend.get_material(&MaterialId("test_persist".to_string()));
        assert!(loaded.is_some());
        assert_eq!(loaded.expect("load failed").name, "Test Persist Material");
    }

    // Cleanup
    {
        let mut backend = MaterialsManagerBackend::new();
        backend.remove_material(&MaterialId("test_persist".to_string()));
    }
}
