use gcodekit5_designer::parametric::{
    Parameter, ParameterConstraint, ParameterSet, ParameterType, ParametricGenerator,
    ParametricTemplate, TemplateLibrary,
};

#[test]
fn test_parameter_constraint_creation() {
    let constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 1.0);
    assert_eq!(constraint.min, 0.0);
    assert_eq!(constraint.max, 100.0);
    assert_eq!(constraint.default, 50.0);
}

#[test]
fn test_parameter_constraint_validation() {
    let constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 1.0);

    assert!(constraint.validate(50.0));
    assert!(constraint.validate(0.0));
    assert!(constraint.validate(100.0));
    assert!(!constraint.validate(101.0));
    assert!(!constraint.validate(-1.0));
}

#[test]
fn test_parameter_constraint_clamping() {
    let constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 1.0);

    assert_eq!(constraint.clamp(50.0), 50.0);
    assert_eq!(constraint.clamp(150.0), 100.0);
    assert_eq!(constraint.clamp(-50.0), 0.0);
}

#[test]
fn test_parameter_creation() {
    let constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 1.0);
    let param = Parameter::new(
        "width".to_string(),
        ParameterType::Distance,
        constraint,
        "Box width".to_string(),
    );

    assert_eq!(param.name, "width");
    assert_eq!(param.param_type, ParameterType::Distance);
    assert_eq!(param.default_value(), 50.0);
}

#[test]
fn test_parameter_set_creation() {
    let set = ParameterSet::new("box_template".to_string());
    assert_eq!(set.template_id, "box_template");
    assert_eq!(set.param_count(), 0);
}

#[test]
fn test_parameter_set_values() {
    let mut set = ParameterSet::new("template1".to_string());

    let _ = set.set("width", 100.0);
    let _ = set.set("height", 50.0);

    assert_eq!(set.get("width"), Some(100.0));
    assert_eq!(set.get("height"), Some(50.0));
    assert_eq!(set.param_count(), 2);
}

#[test]
fn test_parametric_template_creation() {
    let template = ParametricTemplate::new(
        "box".to_string(),
        "Box Template".to_string(),
        "Create a simple box".to_string(),
    );

    assert_eq!(template.id, "box");
    assert_eq!(template.parameter_count(), 0);
}

#[test]
fn test_parametric_template_add_parameter() {
    let mut template = ParametricTemplate::new(
        "box".to_string(),
        "Box Template".to_string(),
        "Create a simple box".to_string(),
    );

    let constraint = ParameterConstraint::new(1.0, 1000.0, 100.0, 1.0);
    let param = Parameter::new(
        "width".to_string(),
        ParameterType::Distance,
        constraint,
        "Box width".to_string(),
    );

    template.add_parameter(param);
    assert_eq!(template.parameter_count(), 1);
}

#[test]
fn test_parametric_template_get_parameter() {
    let mut template = ParametricTemplate::new(
        "box".to_string(),
        "Box Template".to_string(),
        "Create a simple box".to_string(),
    );

    let constraint = ParameterConstraint::new(1.0, 1000.0, 100.0, 1.0);
    let param = Parameter::new(
        "width".to_string(),
        ParameterType::Distance,
        constraint,
        "Box width".to_string(),
    );

    template.add_parameter(param);
    let retrieved = template.get_parameter("width");
    assert!(retrieved.is_some());
}

#[test]
fn test_template_library_creation() {
    let library = TemplateLibrary::new("design_templates".to_string());
    assert_eq!(library.template_count(), 0);
}

#[test]
fn test_template_library_add_template() {
    let mut library = TemplateLibrary::new("templates".to_string());
    let template = ParametricTemplate::new(
        "box".to_string(),
        "Box".to_string(),
        "Box template".to_string(),
    );

    let result = library.add_template(template);
    assert!(result.is_ok());
    assert_eq!(library.template_count(), 1);
}

#[test]
fn test_template_library_duplicate_prevention() {
    let mut library = TemplateLibrary::new("templates".to_string());
    let template1 = ParametricTemplate::new(
        "box".to_string(),
        "Box".to_string(),
        "Box template".to_string(),
    );
    let template2 = ParametricTemplate::new(
        "box".to_string(),
        "Box".to_string(),
        "Box template".to_string(),
    );

    let _ = library.add_template(template1);
    let result = library.add_template(template2);
    assert!(result.is_err());
}

#[test]
fn test_template_library_get_template() {
    let mut library = TemplateLibrary::new("templates".to_string());
    let template = ParametricTemplate::new(
        "circle".to_string(),
        "Circle".to_string(),
        "Circle template".to_string(),
    );

    let _ = library.add_template(template);
    let retrieved = library.get_template("circle");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Circle");
}

#[test]
fn test_template_library_remove_template() {
    let mut library = TemplateLibrary::new("templates".to_string());
    let template = ParametricTemplate::new(
        "square".to_string(),
        "Square".to_string(),
        "Square template".to_string(),
    );

    let _ = library.add_template(template);
    assert_eq!(library.template_count(), 1);

    let removed = library.remove_template("square");
    assert!(removed.is_some());
    assert_eq!(library.template_count(), 0);
}

#[test]
fn test_parametric_generator_validate() {
    let mut template = ParametricTemplate::new(
        "test".to_string(),
        "Test".to_string(),
        "Test template".to_string(),
    );

    let constraint = ParameterConstraint::new(0.0, 100.0, 50.0, 1.0);
    let param = Parameter::new(
        "value".to_string(),
        ParameterType::Number,
        constraint,
        "A value".to_string(),
    );
    template.add_parameter(param);

    let mut params = ParameterSet::new("test".to_string());
    let _ = params.set("value", 50.0);

    let result = ParametricGenerator::validate_all(&template, &params);
    assert!(result.is_ok());
}

#[test]
fn test_parametric_template_default_parameters() {
    let mut template = ParametricTemplate::new(
        "box".to_string(),
        "Box".to_string(),
        "Box template".to_string(),
    );

    let constraint = ParameterConstraint::new(1.0, 100.0, 50.0, 1.0);
    let param = Parameter::new(
        "width".to_string(),
        ParameterType::Distance,
        constraint,
        "Width".to_string(),
    );
    template.add_parameter(param);

    let defaults = template.create_default_parameters();
    assert_eq!(defaults.get("width"), Some(50.0));
}
