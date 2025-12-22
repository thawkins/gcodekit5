// Designer Template Management Tests
// Tests for the design template system including persistence,
// search, filtering, and library management.

use gcodekit5_designer::templates::{
    DesignTemplate, DesignTemplateLibrary, TemplateCategory, TemplateManager, TemplatePersistence,
};
use tempfile::TempDir;

#[test]
fn test_design_template_creation() {
    let template = DesignTemplate::new(
        "gear-box-1".to_string(),
        "Precision Gear Box".to_string(),
        "A precision gear assembly for mechanical systems".to_string(),
        TemplateCategory::Mechanical,
        "John Smith".to_string(),
        r#"{"version": "1.0", "shapes": []}"#.to_string(),
    );

    assert_eq!(template.id, "gear-box-1");
    assert_eq!(template.name, "Precision Gear Box");
    assert_eq!(template.category, TemplateCategory::Mechanical);
    assert_eq!(template.author, "John Smith");
    assert_eq!(template.version, "1.0.0");
    assert!(!template.is_favorite);
}

#[test]
fn test_design_template_tags() {
    let mut template = DesignTemplate::new(
        "test-1".to_string(),
        "Test".to_string(),
        "Description".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    assert_eq!(template.tags.len(), 0);

    template.add_tag("metal".to_string());
    template.add_tag("gear".to_string());
    assert_eq!(template.tags.len(), 2);

    // Adding duplicate tag should not increase count
    template.add_tag("metal".to_string());
    assert_eq!(template.tags.len(), 2);

    // Removing existing tag
    assert!(template.remove_tag("metal"));
    assert_eq!(template.tags.len(), 1);

    // Removing non-existent tag
    assert!(!template.remove_tag("nonexistent"));
}

#[test]
fn test_design_template_metadata() {
    let mut template = DesignTemplate::new(
        "test-1".to_string(),
        "Test".to_string(),
        "Description".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    template.set_version("2.0.0".to_string());
    assert_eq!(template.version, "2.0.0");

    template.set_thumbnail("base64-encoded-image".to_string());
    assert!(template.thumbnail.is_some());
    assert_eq!(template.thumbnail.unwrap(), "base64-encoded-image");

    template
        .metadata
        .insert("brand".to_string(), "Precision Tools".to_string());
    assert_eq!(template.metadata.get("brand").unwrap(), "Precision Tools");
}

#[test]
fn test_design_template_favorite() {
    let mut template = DesignTemplate::new(
        "test-1".to_string(),
        "Test".to_string(),
        "Description".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    assert!(!template.is_favorite);

    template.toggle_favorite();
    assert!(template.is_favorite);

    template.toggle_favorite();
    assert!(!template.is_favorite);

    template.set_favorite(true);
    assert!(template.is_favorite);

    template.set_favorite(false);
    assert!(!template.is_favorite);
}

#[test]
fn test_design_template_search() {
    let mut template = DesignTemplate::new(
        "gear-box-1".to_string(),
        "Precision Gear Box".to_string(),
        "A precision gear assembly for CNC machines".to_string(),
        TemplateCategory::Mechanical,
        "John Smith".to_string(),
        "{}".to_string(),
    );

    template.add_tag("gear".to_string());
    template.add_tag("mechanical".to_string());

    // Test name search
    assert!(template.matches_search("gear"));
    assert!(template.matches_search("precision"));

    // Test description search
    assert!(template.matches_search("assembly"));

    // Test author search
    assert!(template.matches_search("john"));

    // Test tag search
    assert!(template.matches_search("mechanical"));

    // Case-insensitive search
    assert!(template.matches_search("PRECISION"));
    assert!(template.matches_search("GEAR"));

    // Non-matching search
    assert!(!template.matches_search("hydraulic"));
}

#[test]
fn test_design_template_library_add() {
    let mut library = DesignTemplateLibrary::new();
    assert_eq!(library.count(), 0);

    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template 1".to_string(),
        "First template".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    assert!(library.add_template(template).is_ok());
    assert_eq!(library.count(), 1);
}

#[test]
fn test_design_template_library_duplicate_id() {
    let mut library = DesignTemplateLibrary::new();

    let template1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template 1".to_string(),
        "First".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    let template2 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template 2".to_string(),
        "Second".to_string(),
        TemplateCategory::Decorative,
        "Author".to_string(),
        "{}".to_string(),
    );

    assert!(library.add_template(template1).is_ok());
    let result = library.add_template(template2);
    assert!(result.is_err());
    assert_eq!(library.count(), 1);
}

#[test]
fn test_design_template_library_remove() {
    let mut library = DesignTemplateLibrary::new();

    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template 1".to_string(),
        "First".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    library.add_template(template).ok();
    assert_eq!(library.count(), 1);

    let removed = library.remove_template("tmpl-1");
    assert!(removed.is_some());
    assert_eq!(library.count(), 0);

    // Removing non-existent template
    let removed = library.remove_template("non-existent");
    assert!(removed.is_none());
}

#[test]
fn test_design_template_library_get() {
    let mut library = DesignTemplateLibrary::new();

    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template 1".to_string(),
        "First".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    library.add_template(template).ok();

    let retrieved = library.get_template("tmpl-1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Template 1");

    let not_found = library.get_template("non-existent");
    assert!(not_found.is_none());
}

#[test]
fn test_design_template_library_list_by_category() {
    let mut library = DesignTemplateLibrary::new();

    let t1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Gear Box".to_string(),
        "Mechanical".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    let t2 = DesignTemplate::new(
        "tmpl-2".to_string(),
        "Flower".to_string(),
        "Decorative".to_string(),
        TemplateCategory::Decorative,
        "Author".to_string(),
        "{}".to_string(),
    );

    let t3 = DesignTemplate::new(
        "tmpl-3".to_string(),
        "Another Gear".to_string(),
        "Mechanical".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    library.add_template(t1).ok();
    library.add_template(t2).ok();
    library.add_template(t3).ok();

    let mechanical = library.list_by_category(TemplateCategory::Mechanical);
    assert_eq!(mechanical.len(), 2);

    let decorative = library.list_by_category(TemplateCategory::Decorative);
    assert_eq!(decorative.len(), 1);

    let signage = library.list_by_category(TemplateCategory::Signage);
    assert_eq!(signage.len(), 0);
}

#[test]
fn test_design_template_library_favorites() {
    let mut library = DesignTemplateLibrary::new();

    let mut t1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Favorite".to_string(),
        "First".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    let t2 = DesignTemplate::new(
        "tmpl-2".to_string(),
        "Not Favorite".to_string(),
        "Second".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    t1.set_favorite(true);

    library.add_template(t1).ok();
    library.add_template(t2).ok();

    let favorites = library.list_favorites();
    assert_eq!(favorites.len(), 1);
    assert_eq!(favorites[0].name, "Favorite");
}

#[test]
fn test_design_template_library_search() {
    let mut library = DesignTemplateLibrary::new();

    let mut t1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Gear Box".to_string(),
        "Metal part".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );
    t1.add_tag("metal".to_string());

    let t2 = DesignTemplate::new(
        "tmpl-2".to_string(),
        "Flower".to_string(),
        "Decorative".to_string(),
        TemplateCategory::Decorative,
        "Author".to_string(),
        "{}".to_string(),
    );

    library.add_template(t1).ok();
    library.add_template(t2).ok();

    let results = library.search("gear");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "tmpl-1");

    let results = library.search("metal");
    assert_eq!(results.len(), 1);

    let results = library.search("nonexistent");
    assert_eq!(results.len(), 0);
}

#[test]
fn test_design_template_library_advanced_search() {
    let mut library = DesignTemplateLibrary::new();

    let mut t1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Gear".to_string(),
        "Metal part".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );
    t1.add_tag("metal".to_string());

    let mut t2 = DesignTemplate::new(
        "tmpl-2".to_string(),
        "Flower".to_string(),
        "Decorative".to_string(),
        TemplateCategory::Decorative,
        "Author".to_string(),
        "{}".to_string(),
    );
    t2.set_favorite(true);

    library.add_template(t1).ok();
    library.add_template(t2).ok();

    // Search by category only
    let results = library.search_advanced(None, Some(TemplateCategory::Mechanical), None, false);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "tmpl-1");

    // Search by favorites only
    let results = library.search_advanced(None, None, None, true);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "tmpl-2");

    // Search by query and category
    let results = library.search_advanced(
        Some("metal"),
        Some(TemplateCategory::Mechanical),
        None,
        false,
    );
    assert_eq!(results.len(), 1);

    // Search with no matches
    let results = library.search_advanced(
        Some("nonexistent"),
        Some(TemplateCategory::Mechanical),
        None,
        false,
    );
    assert_eq!(results.len(), 0);
}

#[test]
fn test_design_template_library_categories() {
    let mut library = DesignTemplateLibrary::new();

    let t1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "T1".to_string(),
        "D".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    let t2 = DesignTemplate::new(
        "tmpl-2".to_string(),
        "T2".to_string(),
        "D".to_string(),
        TemplateCategory::Decorative,
        "Author".to_string(),
        "{}".to_string(),
    );

    let t3 = DesignTemplate::new(
        "tmpl-3".to_string(),
        "T3".to_string(),
        "D".to_string(),
        TemplateCategory::Signage,
        "Author".to_string(),
        "{}".to_string(),
    );

    library.add_template(t1).ok();
    library.add_template(t2).ok();
    library.add_template(t3).ok();

    let categories = library.get_categories();
    assert_eq!(categories.len(), 3);
}

#[test]
fn test_design_template_library_tags() {
    let mut library = DesignTemplateLibrary::new();

    let mut t1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "T1".to_string(),
        "D".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );
    t1.add_tag("metal".to_string());
    t1.add_tag("gear".to_string());

    let mut t2 = DesignTemplate::new(
        "tmpl-2".to_string(),
        "T2".to_string(),
        "D".to_string(),
        TemplateCategory::Decorative,
        "Author".to_string(),
        "{}".to_string(),
    );
    t2.add_tag("wood".to_string());
    t2.add_tag("gear".to_string());

    library.add_template(t1).ok();
    library.add_template(t2).ok();

    let tags = library.get_all_tags();
    assert_eq!(tags.len(), 3);
    assert!(tags.contains(&"metal".to_string()));
    assert!(tags.contains(&"gear".to_string()));
    assert!(tags.contains(&"wood".to_string()));
}

#[test]
fn test_template_persistence_save_and_load() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lib_path = temp_dir.path().join("templates.json");

    // Create and save library
    let mut library = DesignTemplateLibrary::new();
    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Test Template".to_string(),
        "Description".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    library.add_template(template).ok();

    assert!(TemplatePersistence::save(&library, &lib_path).is_ok());
    assert!(lib_path.exists());

    // Load library
    let loaded = TemplatePersistence::load(&lib_path);
    assert!(loaded.is_ok());

    let loaded_lib = loaded.unwrap();
    assert_eq!(loaded_lib.count(), 1);
    assert!(loaded_lib.get_template("tmpl-1").is_some());
}

#[test]
fn test_template_persistence_single_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let template_path = temp_dir.path().join("template.json");

    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Test Template".to_string(),
        "Description".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        r#"{"version": "1.0"}"#.to_string(),
    );

    // Save single template
    assert!(TemplatePersistence::save_template(&template, &template_path).is_ok());
    assert!(template_path.exists());

    // Load single template
    let loaded = TemplatePersistence::load_template(&template_path);
    assert!(loaded.is_ok());

    let loaded_template = loaded.unwrap();
    assert_eq!(loaded_template.id, "tmpl-1");
    assert_eq!(loaded_template.name, "Test Template");
}

#[test]
fn test_template_manager_add_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lib_path = temp_dir.path().join("templates.json");

    let mut manager = TemplateManager::new(lib_path.clone()).expect("Failed to create manager");
    assert_eq!(manager.count(), 0);

    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template".to_string(),
        "Description".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    assert!(manager.add_template(template).is_ok());
    assert_eq!(manager.count(), 1);

    // Verify persistence
    let manager2 = TemplateManager::new(lib_path).expect("Failed to create manager");
    assert_eq!(manager2.count(), 1);
}

#[test]
fn test_template_manager_remove_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lib_path = temp_dir.path().join("templates.json");

    let mut manager = TemplateManager::new(lib_path.clone()).expect("Failed to create manager");

    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template".to_string(),
        "Description".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    manager.add_template(template).ok();
    assert_eq!(manager.count(), 1);

    assert!(manager.remove_template("tmpl-1").is_ok());
    assert_eq!(manager.count(), 0);
}

#[test]
fn test_template_manager_favorite() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lib_path = temp_dir.path().join("templates.json");

    let mut manager = TemplateManager::new(lib_path.clone()).expect("Failed to create manager");

    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template".to_string(),
        "Description".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    manager.add_template(template).ok();

    assert_eq!(manager.list_favorites().len(), 0);

    assert!(manager.toggle_favorite("tmpl-1").is_ok());
    assert_eq!(manager.list_favorites().len(), 1);

    assert!(manager.toggle_favorite("tmpl-1").is_ok());
    assert_eq!(manager.list_favorites().len(), 0);
}

#[test]
fn test_template_manager_search() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lib_path = temp_dir.path().join("templates.json");

    let mut manager = TemplateManager::new(lib_path.clone()).expect("Failed to create manager");

    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Gear Box".to_string(),
        "Metal part".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    manager.add_template(template).ok();

    let results = manager.search("gear");
    assert_eq!(results.len(), 1);

    let results = manager.search("nonexistent");
    assert_eq!(results.len(), 0);
}

#[test]
fn test_template_manager_list_by_category() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lib_path = temp_dir.path().join("templates.json");

    let mut manager = TemplateManager::new(lib_path.clone()).expect("Failed to create manager");

    let t1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "T1".to_string(),
        "D".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    let t2 = DesignTemplate::new(
        "tmpl-2".to_string(),
        "T2".to_string(),
        "D".to_string(),
        TemplateCategory::Decorative,
        "Author".to_string(),
        "{}".to_string(),
    );

    manager.add_template(t1).ok();
    manager.add_template(t2).ok();

    let mechanical = manager.list_by_category(TemplateCategory::Mechanical);
    assert_eq!(mechanical.len(), 1);

    let decorative = manager.list_by_category(TemplateCategory::Decorative);
    assert_eq!(decorative.len(), 1);
}

#[test]
fn test_template_manager_reload() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lib_path = temp_dir.path().join("templates.json");

    let mut manager1 = TemplateManager::new(lib_path.clone()).expect("Failed to create manager");

    let template = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template".to_string(),
        "Description".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    manager1.add_template(template).ok();

    // Create new manager and reload
    let mut manager2 = TemplateManager::new(lib_path).expect("Failed to create manager");
    assert!(manager2.reload().is_ok());
    assert_eq!(manager2.count(), 1);
}
