use gcodekit5_core::data::materials::*;
// Removed unused HashMap import

#[test]
fn test_material_id_display() {
    let id = MaterialId("test_material".to_string());
    assert_eq!(id.to_string(), "test_material");
}

#[test]
fn test_material_creation() {
    let material = Material::new(
        MaterialId("test".to_string()),
        "Test Material".to_string(),
        MaterialCategory::Wood,
        "Test".to_string(),
    );

    assert_eq!(material.id.0, "test");
    assert_eq!(material.name, "Test Material");
    assert_eq!(material.category, MaterialCategory::Wood);
}

#[test]
fn test_machinability_descriptions() {
    let mut material = Material::new(
        MaterialId("test".to_string()),
        "Test".to_string(),
        MaterialCategory::Wood,
        "Test".to_string(),
    );

    material.machinability_rating = 1;
    assert_eq!(material.machinability_desc(), "Very Difficult");

    material.machinability_rating = 5;
    assert_eq!(material.machinability_desc(), "Moderate");

    material.machinability_rating = 9;
    assert_eq!(material.machinability_desc(), "Very Easy");
}

#[test]
fn test_cutting_parameters_default() {
    let params = CuttingParameters::default();
    assert_eq!(params.rpm_range, (12000, 18000));
    assert_eq!(params.plunge_rate_percent, 50.0);
}

#[test]
fn test_material_library_add_and_get() {
    let mut library = MaterialLibrary::new();
    let material = Material::new(
        MaterialId("test".to_string()),
        "Test".to_string(),
        MaterialCategory::Wood,
        "Test".to_string(),
    );

    library.add_material(material);
    assert_eq!(library.len(), 1);

    let retrieved = library.get_material(&MaterialId("test".to_string()));
    assert!(retrieved.is_some());
}

#[test]
fn test_material_library_search() {
    let library = init_standard_library();
    let results = library.search_by_name("oak");
    assert!(!results.is_empty());
    assert!(results.iter().any(|m| m.name.contains("Oak")));
}

#[test]
fn test_material_library_category_filter() {
    let library = init_standard_library();
    let wood_materials = library.get_materials_by_category(MaterialCategory::Wood);
    assert!(!wood_materials.is_empty());

    let metal_materials = library.get_materials_by_category(MaterialCategory::NonFerrousMetal);
    assert!(!metal_materials.is_empty());
}

#[test]
fn test_standard_library_initialization() {
    let library = init_standard_library();
    assert!(library.len() >= 3);

    // Check that common materials exist
    assert!(library
        .get_material(&MaterialId("wood_oak_red".to_string()))
        .is_some());
    assert!(library
        .get_material(&MaterialId("metal_al_6061".to_string()))
        .is_some());
    assert!(library
        .get_material(&MaterialId("plastic_acrylic".to_string()))
        .is_some());
}

#[test]
fn test_cutting_parameters_storage() {
    let mut material = Material::new(
        MaterialId("test".to_string()),
        "Test".to_string(),
        MaterialCategory::Wood,
        "Test".to_string(),
    );

    let params = CuttingParameters::default();
    material.set_cutting_params("endmill_flat".to_string(), params);

    let retrieved = material.get_cutting_params("endmill_flat");
    assert!(retrieved.is_some());
}
