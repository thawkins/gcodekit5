use gcodekit5_designer::templates::{DesignTemplate, DesignTemplateLibrary, TemplateCategory};

#[test]
fn test_template_category_conversion() {
    assert_eq!(TemplateCategory::Mechanical.as_str(), "mechanical");
    assert_eq!(
        TemplateCategory::from_str("mechanical"),
        Some(TemplateCategory::Mechanical)
    );
    assert_eq!(TemplateCategory::from_str("invalid"), None);
}

#[test]
fn test_create_design_template() {
    let template = DesignTemplate::new(
        "test-1".to_string(),
        "Test Template".to_string(),
        "A test template".to_string(),
        TemplateCategory::Mechanical,
        "Test Author".to_string(),
        "{}".to_string(),
    );

    assert_eq!(template.id, "test-1");
    assert_eq!(template.name, "Test Template");
    assert_eq!(template.version, "1.0.0");
    assert!(!template.is_favorite);
}

#[test]
fn test_template_tags() {
    let mut template = DesignTemplate::new(
        "test-1".to_string(),
        "Test".to_string(),
        "Test".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    template.add_tag("gear".to_string());
    template.add_tag("metal".to_string());
    assert_eq!(template.tags.len(), 2);

    let removed = template.remove_tag("gear");
    assert!(removed);
    assert_eq!(template.tags.len(), 1);
}

#[test]
fn test_template_favorite() {
    let mut template = DesignTemplate::new(
        "test-1".to_string(),
        "Test".to_string(),
        "Test".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    assert!(!template.is_favorite);
    template.toggle_favorite();
    assert!(template.is_favorite);
    template.set_favorite(false);
    assert!(!template.is_favorite);
}

#[test]
fn test_template_search() {
    let template = DesignTemplate::new(
        "test-1".to_string(),
        "Gear Box".to_string(),
        "A precision gear assembly".to_string(),
        TemplateCategory::Mechanical,
        "John Doe".to_string(),
        "{}".to_string(),
    );

    assert!(template.matches_search("gear"));
    assert!(template.matches_search("assembly"));
    assert!(template.matches_search("john"));
    assert!(!template.matches_search("invalid"));
}

#[test]
fn test_template_library_add_remove() {
    let mut library = DesignTemplateLibrary::new();

    let template1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Template 1".to_string(),
        "First template".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );

    assert!(library.add_template(template1).is_ok());
    assert_eq!(library.count(), 1);

    let template2 = DesignTemplate::new(
        "tmpl-2".to_string(),
        "Template 2".to_string(),
        "Second template".to_string(),
        TemplateCategory::Decorative,
        "Author".to_string(),
        "{}".to_string(),
    );

    assert!(library.add_template(template2).is_ok());
    assert_eq!(library.count(), 2);

    let removed = library.remove_template("tmpl-1");
    assert!(removed.is_some());
    assert_eq!(library.count(), 1);
}

#[test]
fn test_template_library_duplicate_id() {
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
    assert!(library.add_template(template2).is_err());
}

#[test]
fn test_template_library_search() {
    let mut library = DesignTemplateLibrary::new();

    let mut t1 = DesignTemplate::new(
        "tmpl-1".to_string(),
        "Gear Box".to_string(),
        "Mechanical".to_string(),
        TemplateCategory::Mechanical,
        "Author".to_string(),
        "{}".to_string(),
    );
    t1.add_tag("metal".to_string());

    let t2 = DesignTemplate::new(
        "tmpl-2".to_string(),
        "Decoration".to_string(),
        "Artistic".to_string(),
        TemplateCategory::Decorative,
        "Author".to_string(),
        "{}".to_string(),
    );

    library.add_template(t1).ok();
    library.add_template(t2).ok();

    let results = library.search("gear");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "tmpl-1");
}

#[test]
fn test_template_library_category_filter() {
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

    library.add_template(t1).ok();
    library.add_template(t2).ok();

    let mechanical = library.list_by_category(TemplateCategory::Mechanical);
    assert_eq!(mechanical.len(), 1);
    assert_eq!(mechanical[0].id, "tmpl-1");

    let decorative = library.list_by_category(TemplateCategory::Decorative);
    assert_eq!(decorative.len(), 1);
    assert_eq!(decorative[0].id, "tmpl-2");
}

#[test]
fn test_template_library_favorites() {
    let mut library = DesignTemplateLibrary::new();

    let mut t1 = DesignTemplate::new(
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

    t1.set_favorite(true);

    library.add_template(t1).ok();
    library.add_template(t2).ok();

    let favorites = library.list_favorites();
    assert_eq!(favorites.len(), 1);
    assert_eq!(favorites[0].id, "tmpl-1");
}

#[test]
fn test_template_library_advanced_search() {
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

    let results = library.search_advanced(None, Some(TemplateCategory::Mechanical), None, false);
    assert_eq!(results.len(), 1);

    let results = library.search_advanced(None, None, None, true);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "tmpl-2");
}
