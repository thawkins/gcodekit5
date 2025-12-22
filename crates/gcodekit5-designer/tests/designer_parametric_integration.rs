// Integration tests for parametric design system (Phase 4.6)

use gcodekit5_designer::parametric::{
    Parameter, ParameterConstraint, ParameterSet, ParameterType, ParametricGenerator,
    ParametricTemplate, TemplateLibrary,
};

#[test]
fn test_simple_box_template() {
    let mut template = ParametricTemplate::new(
        "simple_box".to_string(),
        "Simple Box".to_string(),
        "Create a simple rectangular box".to_string(),
    );

    let width_constraint = ParameterConstraint::new(10.0, 500.0, 100.0, 5.0);
    let width = Parameter::new(
        "width".to_string(),
        ParameterType::Distance,
        width_constraint,
        "Width of the box".to_string(),
    );

    let height_constraint = ParameterConstraint::new(10.0, 500.0, 100.0, 5.0);
    let height = Parameter::new(
        "height".to_string(),
        ParameterType::Distance,
        height_constraint,
        "Height of the box".to_string(),
    );

    template.add_parameter(width);
    template.add_parameter(height);

    assert_eq!(template.parameter_count(), 2);
}

#[test]
fn test_circle_template() {
    let mut template = ParametricTemplate::new(
        "circle".to_string(),
        "Circle".to_string(),
        "Create a circle".to_string(),
    );

    let radius_constraint = ParameterConstraint::new(1.0, 1000.0, 50.0, 1.0);
    let radius = Parameter::new(
        "radius".to_string(),
        ParameterType::Distance,
        radius_constraint,
        "Circle radius".to_string(),
    );

    template.add_parameter(radius);
    assert_eq!(template.parameter_count(), 1);
}

#[test]
fn test_gear_template() {
    let mut template = ParametricTemplate::new(
        "gear".to_string(),
        "Gear".to_string(),
        "Create a gear".to_string(),
    );

    // Pitch diameter
    let pitch_constraint = ParameterConstraint::new(10.0, 500.0, 50.0, 1.0);
    let pitch = Parameter::new(
        "pitch_diameter".to_string(),
        ParameterType::Distance,
        pitch_constraint,
        "Pitch diameter".to_string(),
    );

    // Number of teeth
    let teeth_constraint = ParameterConstraint::new(8.0, 100.0, 20.0, 1.0);
    let teeth = Parameter::new(
        "teeth".to_string(),
        ParameterType::Integer,
        teeth_constraint,
        "Number of teeth".to_string(),
    );

    // Pressure angle
    let angle_constraint = ParameterConstraint::new(14.5, 25.0, 20.0, 0.5);
    let angle = Parameter::new(
        "pressure_angle".to_string(),
        ParameterType::Angle,
        angle_constraint,
        "Pressure angle in degrees".to_string(),
    );

    template.add_parameter(pitch);
    template.add_parameter(teeth);
    template.add_parameter(angle);

    assert_eq!(template.parameter_count(), 3);
}

#[test]
fn test_parameter_set_with_template() {
    let mut template =
        ParametricTemplate::new("box".to_string(), "Box".to_string(), "A box".to_string());

    let constraint = ParameterConstraint::new(10.0, 100.0, 50.0, 1.0);
    let param = Parameter::new(
        "size".to_string(),
        ParameterType::Distance,
        constraint,
        "Box size".to_string(),
    );

    template.add_parameter(param);

    let mut param_set = ParameterSet::new("box".to_string());
    let _ = param_set.set("size", 75.0);

    assert!(ParametricGenerator::validate_all(&template, &param_set).is_ok());
}

#[test]
fn test_template_default_parameters() {
    let mut template = ParametricTemplate::new(
        "simple".to_string(),
        "Simple".to_string(),
        "Simple template".to_string(),
    );

    let constraint1 = ParameterConstraint::new(10.0, 100.0, 25.0, 1.0);
    let param1 = Parameter::new(
        "param1".to_string(),
        ParameterType::Distance,
        constraint1,
        "Parameter 1".to_string(),
    );

    let constraint2 = ParameterConstraint::new(0.0, 360.0, 180.0, 5.0);
    let param2 = Parameter::new(
        "param2".to_string(),
        ParameterType::Angle,
        constraint2,
        "Parameter 2".to_string(),
    );

    template.add_parameter(param1);
    template.add_parameter(param2);

    let defaults = template.create_default_parameters();
    assert_eq!(defaults.get("param1"), Some(25.0));
    assert_eq!(defaults.get("param2"), Some(180.0));
}

#[test]
fn test_template_library_organization() {
    let mut library = TemplateLibrary::new("cad_templates".to_string());

    let box_template = ParametricTemplate::new(
        "box".to_string(),
        "Box".to_string(),
        "Create a box".to_string(),
    );

    let circle_template = ParametricTemplate::new(
        "circle".to_string(),
        "Circle".to_string(),
        "Create a circle".to_string(),
    );

    let gear_template = ParametricTemplate::new(
        "gear".to_string(),
        "Gear".to_string(),
        "Create a gear".to_string(),
    );

    let _ = library.add_template(box_template);
    let _ = library.add_template(circle_template);
    let _ = library.add_template(gear_template);

    assert_eq!(library.template_count(), 3);
    let templates = library.template_ids();
    assert!(templates.contains(&"box"));
    assert!(templates.contains(&"circle"));
    assert!(templates.contains(&"gear"));
}

#[test]
fn test_template_library_list() {
    let mut library = TemplateLibrary::new("shapes".to_string());

    for i in 0..3 {
        let template = ParametricTemplate::new(
            format!("template_{}", i),
            format!("Template {}", i),
            format!("Template description {}", i),
        );
        let _ = library.add_template(template);
    }

    let list = library.list_templates();
    assert_eq!(list.len(), 3);
}

#[test]
fn test_parameter_types() {
    let number_constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 1.0);
    let number_param = Parameter::new(
        "number".to_string(),
        ParameterType::Number,
        number_constraint,
        "A number".to_string(),
    );
    assert_eq!(number_param.param_type, ParameterType::Number);

    let integer_constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 1.0);
    let integer_param = Parameter::new(
        "integer".to_string(),
        ParameterType::Integer,
        integer_constraint,
        "An integer".to_string(),
    );
    assert_eq!(integer_param.param_type, ParameterType::Integer);

    let angle_constraint = ParameterConstraint::new(0.0, 360.0, 180.0, 1.0);
    let angle_param = Parameter::new(
        "angle".to_string(),
        ParameterType::Angle,
        angle_constraint,
        "An angle".to_string(),
    );
    assert_eq!(angle_param.param_type, ParameterType::Angle);

    let distance_constraint = ParameterConstraint::new(0.0, 1000.0, 100.0, 1.0);
    let distance_param = Parameter::new(
        "distance".to_string(),
        ParameterType::Distance,
        distance_constraint,
        "A distance".to_string(),
    );
    assert_eq!(distance_param.param_type, ParameterType::Distance);
}

#[test]
fn test_complex_template_multi_parameter() {
    let mut template = ParametricTemplate::new(
        "complex".to_string(),
        "Complex Shape".to_string(),
        "A complex multi-parameter shape".to_string(),
    );

    template.version = "2.0".to_string();
    template.author = "Designer Team".to_string();

    // Add multiple parameters
    for i in 0..5 {
        let constraint = ParameterConstraint::new(1.0, 100.0, 50.0, 1.0);
        let param = Parameter::new(
            format!("dimension_{}", i),
            ParameterType::Distance,
            constraint,
            format!("Dimension {}", i),
        );
        template.add_parameter(param);
    }

    assert_eq!(template.parameter_count(), 5);
    assert_eq!(template.version, "2.0");
}

#[test]
fn test_parameter_validation_bounds() {
    let constraint = ParameterConstraint::new(10.0, 100.0, 50.0, 5.0);

    let param = Parameter::new(
        "width".to_string(),
        ParameterType::Distance,
        constraint,
        "Width".to_string(),
    );

    // Valid values
    assert!(param.validate(50.0));
    assert!(param.validate(10.0));
    assert!(param.validate(100.0));

    // Invalid values
    assert!(!param.validate(5.0));
    assert!(!param.validate(150.0));
}

#[test]
fn test_template_get_parameter_by_name() {
    let mut template = ParametricTemplate::new(
        "template".to_string(),
        "Template".to_string(),
        "Test template".to_string(),
    );

    let constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 1.0);
    let param = Parameter::new(
        "test_param".to_string(),
        ParameterType::Number,
        constraint,
        "Test parameter".to_string(),
    );

    template.add_parameter(param);

    let retrieved = template.get_parameter("test_param");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "test_param");

    let not_found = template.get_parameter("nonexistent");
    assert!(not_found.is_none());
}

#[test]
fn test_parameter_set_multiple_values() {
    let mut param_set = ParameterSet::new("template_id".to_string());

    let _ = param_set.set("width", 100.0);
    let _ = param_set.set("height", 50.0);
    let _ = param_set.set("depth", 30.0);

    assert_eq!(param_set.param_count(), 3);
    assert_eq!(param_set.get("width"), Some(100.0));
    assert_eq!(param_set.get("height"), Some(50.0));
    assert_eq!(param_set.get("depth"), Some(30.0));
}

#[test]
fn test_parameter_constraint_step() {
    let constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 5.0);
    assert_eq!(constraint.step, 5.0);

    let fine_constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 0.1);
    assert_eq!(fine_constraint.step, 0.1);
}

#[test]
fn test_library_category() {
    let library = TemplateLibrary::new("mechanical".to_string());
    assert_eq!(library.category, "mechanical");

    let library2 = TemplateLibrary::new("structural".to_string());
    assert_eq!(library2.category, "structural");
}

#[test]
fn test_parametric_design_workflow() {
    // Create template
    let mut template = ParametricTemplate::new(
        "box".to_string(),
        "Box".to_string(),
        "Create a box".to_string(),
    );

    // Add parameters
    let width_constraint = ParameterConstraint::new(10.0, 200.0, 100.0, 1.0);
    let width_param = Parameter::new(
        "width".to_string(),
        ParameterType::Distance,
        width_constraint,
        "Width".to_string(),
    );

    template.add_parameter(width_param);

    // Create parameter set
    let mut params = ParameterSet::new("box".to_string());
    let _ = params.set("width", 150.0);

    // Validate
    let validation_result = ParametricGenerator::validate_all(&template, &params);
    assert!(validation_result.is_ok());

    // Check complexity
    let complexity = ParametricGenerator::estimate_complexity(&params);
    assert!(complexity > 0);
}

#[test]
fn test_parameter_set_clear() {
    let mut param_set = ParameterSet::new("template".to_string());

    let _ = param_set.set("param1", 100.0);
    let _ = param_set.set("param2", 200.0);

    assert_eq!(param_set.param_count(), 2);

    param_set.clear();

    assert_eq!(param_set.param_count(), 0);
    assert_eq!(param_set.get("param1"), None);
}

#[test]
fn test_template_library_update() {
    let mut library = TemplateLibrary::new("templates".to_string());

    let template = ParametricTemplate::new(
        "box".to_string(),
        "Box v1".to_string(),
        "Original box".to_string(),
    );

    let _ = library.add_template(template);

    // Retrieve and modify
    if let Some(retrieved) = library.get_template_mut("box") {
        retrieved.name = "Box v2".to_string();
        retrieved.version = "2.0".to_string();
    }

    // Verify changes
    let updated = library.get_template("box");
    assert_eq!(updated.unwrap().name, "Box v2");
    assert_eq!(updated.unwrap().version, "2.0");
}
