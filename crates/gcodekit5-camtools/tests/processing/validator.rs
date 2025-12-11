use gcodekit5_camtools::validator::GCodeValidator;

#[test]
fn test_coordinate_validation() {
    let validator = GCodeValidator::default();
    let lines = vec!["G0 X2001 Y10".to_string()];
    let result = validator.validate(&lines);
    assert!(result.is_err());
}

#[test]
fn test_valid_line() {
    let validator = GCodeValidator::default();
    let lines = vec!["G0 X10 Y20 Z0".to_string()];
    let result = validator.validate(&lines);
    assert!(result.is_ok());
}
