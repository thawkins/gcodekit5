use gcodekit5_designer::adaptive::{
    AdaptiveAlgorithm, AdaptiveClearing, DynamicStepover, LoadMonitor, MaterialProperties,
    MaterialType,
};

#[test]
fn test_material_aluminum() {
    let mat = MaterialProperties::aluminum();
    assert_eq!(mat.material_type, MaterialType::Aluminum);
    assert!(mat.is_valid());
}

#[test]
fn test_material_steel() {
    let mat = MaterialProperties::steel();
    assert_eq!(mat.material_type, MaterialType::Steel);
    assert!(mat.is_valid());
}

#[test]
fn test_load_monitor_creation() {
    let monitor = LoadMonitor::new(0.75);
    assert_eq!(monitor.current_load, 0.0);
    assert_eq!(monitor.target_load, 0.75);
}

#[test]
fn test_load_monitor_recording() {
    let mut monitor = LoadMonitor::new(0.75);
    monitor.record_sample(0.7);
    assert_eq!(monitor.current_load, 0.7);
    assert_eq!(monitor.average_load, 0.7);
}

#[test]
fn test_load_monitor_average() {
    let mut monitor = LoadMonitor::new(0.75);
    monitor.record_sample(0.5);
    monitor.record_sample(0.6);
    monitor.record_sample(0.7);

    let avg = (0.5 + 0.6 + 0.7) / 3.0;
    assert!((monitor.average_load - avg).abs() < 0.01);
}

#[test]
fn test_load_monitor_adjustment_factor() {
    let mut monitor = LoadMonitor::new(1.0);
    monitor.record_sample(0.5);

    let factor = monitor.adjustment_factor();
    assert!((factor - 2.0).abs() < 0.01);
}

#[test]
fn test_load_monitor_health_check() {
    let mut monitor = LoadMonitor::new(0.75);
    monitor.record_sample(0.75);

    assert!(monitor.is_load_healthy());
}

#[test]
fn test_dynamic_stepover_creation() {
    let stepover = DynamicStepover::new(2.0, 1.0);
    assert_eq!(stepover.base_stepover, 2.0);
    assert_eq!(stepover.current_stepover, 2.0);
}

#[test]
fn test_dynamic_stepover_adjustment() {
    let mut stepover = DynamicStepover::new(2.0, 1.0);
    stepover.apply_adjustment(0.5);

    assert_eq!(stepover.current_stepover, 1.0);
    assert_eq!(stepover.current_stepdown, 0.5);
}

#[test]
fn test_dynamic_stepover_clamping() {
    let mut stepover = DynamicStepover::new(2.0, 1.0);
    stepover.apply_adjustment(3.0); // Try to increase beyond max

    assert!(stepover.current_stepover <= stepover.max_stepover);
}

#[test]
fn test_adaptive_clearing_creation() {
    let mat = MaterialProperties::aluminum();
    let clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    assert!(clearing.is_valid());
    assert_eq!(clearing.aggressiveness, 0.7);
}

#[test]
fn test_adaptive_clearing_update() {
    let mat = MaterialProperties::aluminum();
    let mut clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    clearing.update(0.6);
    assert!(clearing.load_monitor.average_load > 0.0);
}

#[test]
fn test_adaptive_wear_compensation() {
    let mat = MaterialProperties::aluminum();
    let mut clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    clearing.tool_condition = 0.8;
    let before = clearing.stepover.current_stepover;
    clearing.apply_wear_compensation();
    let after = clearing.stepover.current_stepover;

    assert!(after <= before);
}

#[test]
fn test_adaptive_simulate_wear() {
    let mat = MaterialProperties::aluminum();
    let mut clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    assert_eq!(clearing.tool_condition, 1.0);
    clearing.simulate_wear(0.3);
    assert_eq!(clearing.tool_condition, 0.7);
}

#[test]
fn test_estimate_load() {
    let mat = MaterialProperties::aluminum();
    let result = AdaptiveAlgorithm::estimate_load(3.175, 100.0, 2.0, 10000, &mat);

    assert!(result.is_ok());
    let load = result.unwrap();
    assert!(load > 0.0 && load <= 1.0);
}

#[test]
fn test_generate_passes() {
    let mat = MaterialProperties::aluminum();
    let clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    let result = AdaptiveAlgorithm::generate_passes(&clearing, 100.0, 5);
    assert!(result.is_ok());

    let passes = result.unwrap();
    assert_eq!(passes.len(), 5);
}

#[test]
fn test_optimize_feed_rate() {
    let mat = MaterialProperties::aluminum();
    let result = AdaptiveAlgorithm::optimize_feed_rate(&mat, 2, 12000, 1.0);

    assert!(result.is_ok());
    let feed = result.unwrap();
    assert!(feed > 0.0 && feed <= mat.max_feed_rate);
}

#[test]
fn test_time_reduction_estimate() {
    let mat = MaterialProperties::aluminum();
    let clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.8);

    let reduction = clearing.time_reduction_estimate();
    assert!(reduction > -50.0 && reduction < 50.0);
}
