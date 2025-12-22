// Integration tests for adaptive clearing strategy (Phase 4.4)

use gcodekit5_designer::{
    AdaptiveAlgorithm, AdaptiveClearing, DynamicStepover, LoadMonitor, MaterialProperties,
    MaterialType,
};

#[test]
fn test_material_aluminum_properties() {
    let mat = MaterialProperties::aluminum();
    assert_eq!(mat.material_type, MaterialType::Aluminum);
    assert_eq!(mat.max_feed_rate, 200.0);
    assert!(mat.is_valid());
}

#[test]
fn test_material_plastic_properties() {
    let mat = MaterialProperties::plastic();
    assert_eq!(mat.material_type, MaterialType::Plastic);
    assert!(mat.is_valid());
}

#[test]
fn test_material_wood_properties() {
    let mat = MaterialProperties::wood();
    assert_eq!(mat.material_type, MaterialType::Wood);
    assert!(mat.is_valid());
}

#[test]
fn test_material_brass_properties() {
    let mat = MaterialProperties::brass();
    assert_eq!(mat.material_type, MaterialType::Brass);
    assert!(mat.is_valid());
}

#[test]
fn test_material_steel_properties() {
    let mat = MaterialProperties::steel();
    assert_eq!(mat.material_type, MaterialType::Steel);
    assert!(mat.is_valid());
}

#[test]
fn test_material_stainless_steel_properties() {
    let mat = MaterialProperties::stainless_steel();
    assert_eq!(mat.material_type, MaterialType::StainlessSteel);
    assert!(mat.is_valid());
}

#[test]
fn test_load_monitor_basic_operation() {
    let monitor = LoadMonitor::new(0.75);
    assert_eq!(monitor.target_load, 0.75);
    assert_eq!(monitor.current_load, 0.0);
}

#[test]
fn test_load_monitor_sample_recording() {
    let mut monitor = LoadMonitor::new(0.75);

    monitor.record_sample(0.7);
    assert_eq!(monitor.current_load, 0.7);

    monitor.record_sample(0.8);
    assert_eq!(monitor.current_load, 0.8);
}

#[test]
fn test_load_monitor_invalid_samples() {
    let mut monitor = LoadMonitor::new(0.75);

    // Negative load should be ignored
    monitor.record_sample(-0.1);
    assert_eq!(monitor.current_load, 0.0);

    // Load > 1.0 should be ignored
    monitor.record_sample(1.5);
    assert_eq!(monitor.current_load, 0.0);
}

#[test]
fn test_load_monitor_averaging() {
    let mut monitor = LoadMonitor::new(0.75);

    monitor.record_sample(0.4);
    monitor.record_sample(0.6);
    monitor.record_sample(0.8);

    let expected_avg = (0.4 + 0.6 + 0.8) / 3.0;
    assert!((monitor.average_load - expected_avg).abs() < 0.01);
}

#[test]
fn test_load_monitor_adjustment_factor_under_load() {
    let mut monitor = LoadMonitor::new(1.0);
    monitor.record_sample(0.5);

    let factor = monitor.adjustment_factor();
    // Should increase load (factor = 1.0 / 0.5 = 2.0)
    assert!((factor - 2.0).abs() < 0.01);
}

#[test]
fn test_load_monitor_adjustment_factor_over_load() {
    let mut monitor = LoadMonitor::new(0.5);
    monitor.record_sample(1.0);

    let factor = monitor.adjustment_factor();
    // Should decrease load (factor = 0.5 / 1.0 = 0.5)
    assert!((factor - 0.5).abs() < 0.01);
}

#[test]
fn test_load_monitor_health_check_good() {
    let mut monitor = LoadMonitor::new(0.75);
    monitor.record_sample(0.75);

    assert!(monitor.is_load_healthy());
}

#[test]
fn test_load_monitor_health_check_over_high() {
    let mut monitor = LoadMonitor::new(0.75);
    monitor.record_sample(1.0); // 33% above target

    assert!(!monitor.is_load_healthy());
}

#[test]
fn test_dynamic_stepover_creation() {
    let stepover = DynamicStepover::new(2.0, 1.0);

    assert_eq!(stepover.base_stepover, 2.0);
    assert_eq!(stepover.base_stepdown, 1.0);
    assert_eq!(stepover.current_stepover, 2.0);
    assert_eq!(stepover.current_stepdown, 1.0);
}

#[test]
fn test_dynamic_stepover_bounds() {
    let stepover = DynamicStepover::new(2.0, 1.0);

    // Min should be 30% of base
    assert_eq!(stepover.min_stepover, 0.6);
    // Max should be 150% of base
    assert_eq!(stepover.max_stepover, 3.0);
}

#[test]
fn test_dynamic_stepover_reduce() {
    let mut stepover = DynamicStepover::new(2.0, 1.0);
    stepover.apply_adjustment(0.5);

    assert_eq!(stepover.current_stepover, 1.0);
    assert_eq!(stepover.current_stepdown, 0.5);
}

#[test]
fn test_dynamic_stepover_increase() {
    let mut stepover = DynamicStepover::new(2.0, 1.0);
    stepover.apply_adjustment(1.5);

    assert_eq!(stepover.current_stepover, 3.0);
    assert_eq!(stepover.current_stepdown, 1.5);
}

#[test]
fn test_dynamic_stepover_clamp_max() {
    let mut stepover = DynamicStepover::new(2.0, 1.0);
    stepover.apply_adjustment(2.0); // Try to double

    // Should be clamped to max 150% = 3.0
    assert_eq!(stepover.current_stepover, 3.0);
}

#[test]
fn test_dynamic_stepover_clamp_min() {
    let mut stepover = DynamicStepover::new(2.0, 1.0);
    stepover.apply_adjustment(0.1); // Try to reduce significantly

    // Should be clamped to min 30% = 0.6
    assert_eq!(stepover.current_stepover, 0.6);
}

#[test]
fn test_dynamic_stepover_efficiency() {
    let mut stepover = DynamicStepover::new(2.0, 1.0);
    stepover.apply_adjustment(1.0);

    let efficiency = stepover.efficiency_ratio();
    assert!((efficiency - 1.0).abs() < 0.01); // Should be neutral (1.0)
}

#[test]
fn test_adaptive_clearing_aluminum() {
    let mat = MaterialProperties::aluminum();
    let clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    assert!(clearing.is_valid());
    assert_eq!(clearing.tool_condition, 1.0);
}

#[test]
fn test_adaptive_clearing_different_materials() {
    let aluminum = MaterialProperties::aluminum();
    let steel = MaterialProperties::steel();

    let clear_al = AdaptiveClearing::new(aluminum, 2.0, 1.0, 0.7);
    let clear_steel = AdaptiveClearing::new(steel, 2.0, 1.0, 0.7);

    // Materials should have different max feed rates
    assert_ne!(
        clear_al.material.max_feed_rate,
        clear_steel.material.max_feed_rate
    );
}

#[test]
fn test_adaptive_clearing_update_load() {
    let mat = MaterialProperties::aluminum();
    let mut clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    clearing.update(0.6);
    assert_eq!(clearing.load_monitor.current_load, 0.6);
    assert!(clearing.load_monitor.average_load > 0.0);
}

#[test]
fn test_adaptive_clearing_wear_compensation() {
    let mat = MaterialProperties::aluminum();
    let mut clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    let original_stepover = clearing.stepover.current_stepover;

    clearing.tool_condition = 0.8;
    clearing.apply_wear_compensation();

    // Should reduce stepover due to tool wear
    assert!(clearing.stepover.current_stepover <= original_stepover);
}

#[test]
fn test_adaptive_clearing_simulate_wear() {
    let mat = MaterialProperties::aluminum();
    let mut clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    assert_eq!(clearing.tool_condition, 1.0);

    clearing.simulate_wear(0.2);
    assert!((clearing.tool_condition - 0.8).abs() < 0.01);

    clearing.simulate_wear(0.2);
    assert!((clearing.tool_condition - 0.6).abs() < 0.01);
}

#[test]
fn test_adaptive_clearing_wear_floor() {
    let mat = MaterialProperties::aluminum();
    let mut clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    clearing.simulate_wear(1.5); // More than 1.0
    assert_eq!(clearing.tool_condition, 0.0);
}

#[test]
fn test_adaptive_aggressiveness_levels() {
    let mat = MaterialProperties::aluminum();

    let conservative = AdaptiveClearing::new(mat, 2.0, 1.0, 0.3);
    let moderate = AdaptiveClearing::new(mat, 2.0, 1.0, 0.6);
    let aggressive = AdaptiveClearing::new(mat, 2.0, 1.0, 0.9);

    // Higher aggressiveness should have higher target load
    assert!(conservative.load_monitor.target_load < moderate.load_monitor.target_load);
    assert!(moderate.load_monitor.target_load < aggressive.load_monitor.target_load);
}

#[test]
fn test_adaptive_algorithm_estimate_load() {
    let mat = MaterialProperties::aluminum();

    let result = AdaptiveAlgorithm::estimate_load(3.175, 100.0, 2.0, 10000, &mat);
    assert!(result.is_ok());

    let load = result.unwrap();
    assert!(load > 0.0 && load <= 1.0);
}

#[test]
fn test_adaptive_algorithm_estimate_load_invalid() {
    let mat = MaterialProperties::aluminum();

    let result = AdaptiveAlgorithm::estimate_load(0.0, 100.0, 2.0, 10000, &mat);
    assert!(result.is_err());
}

#[test]
fn test_adaptive_algorithm_generate_passes() {
    let mat = MaterialProperties::aluminum();
    let clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.7);

    let result = AdaptiveAlgorithm::generate_passes(&clearing, 100.0, 5);
    assert!(result.is_ok());

    let passes = result.unwrap();
    assert_eq!(passes.len(), 5);
}

#[test]
fn test_adaptive_algorithm_optimize_feed_rate() {
    let mat = MaterialProperties::aluminum();

    let result = AdaptiveAlgorithm::optimize_feed_rate(&mat, 2, 12000, 1.0);
    assert!(result.is_ok());

    let feed = result.unwrap();
    assert!(feed > 0.0 && feed <= mat.max_feed_rate);
}

#[test]
fn test_adaptive_algorithm_feed_rate_tool_wear() {
    let mat = MaterialProperties::aluminum();

    let new_tool = AdaptiveAlgorithm::optimize_feed_rate(&mat, 2, 12000, 1.0).unwrap();
    let worn_tool = AdaptiveAlgorithm::optimize_feed_rate(&mat, 2, 12000, 0.5).unwrap();

    // Worn tool should have lower or equal feed rate
    assert!(new_tool >= worn_tool);
}

#[test]
fn test_adaptive_time_reduction() {
    let mat = MaterialProperties::aluminum();
    let clearing = AdaptiveClearing::new(mat, 2.0, 1.0, 0.8);

    let reduction = clearing.time_reduction_estimate();
    // Should be a reasonable estimate (not extreme)
    assert!(reduction > -100.0 && reduction < 100.0);
}

#[test]
fn test_adaptive_complex_scenario() {
    let mat = MaterialProperties::steel();
    let mut clearing = AdaptiveClearing::new(mat, 1.5, 0.5, 0.6);

    // Simulate machining with load monitoring
    clearing.update(0.65); // Good load
    clearing.update(0.68); // Still good
    clearing.update(0.72); // Slightly high

    assert!(clearing.load_monitor.is_load_healthy());

    // Simulate tool wear
    clearing.simulate_wear(0.15);
    clearing.apply_wear_compensation();

    // Should still be valid
    assert!(clearing.is_valid());
}
