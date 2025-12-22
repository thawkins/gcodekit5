use gcodekit5_designer::model::Point;
use gcodekit5_designer::multipass::{DepthStrategy, MultiPassConfig, MultiPassToolpathGenerator};
use gcodekit5_designer::toolpath::{Toolpath, ToolpathSegment, ToolpathSegmentType};

#[test]
fn test_multipass_config_constant_strategy() {
    let config = MultiPassConfig::new(-30.0, 10.0);
    assert_eq!(config.calculate_passes(), 3);
    assert_eq!(config.calculate_pass_depth(1), -10.0);
    assert_eq!(config.calculate_pass_depth(2), -20.0);
    assert_eq!(config.calculate_pass_depth(3), -30.0);
}

#[test]
fn test_multipass_config_ramped_strategy() {
    let mut config = MultiPassConfig::new(-30.0, 10.0);
    config.set_strategy(DepthStrategy::Ramped);
    config.set_minimum_depth(2.0);
    config.set_ramp_start_depth(2.0);

    let depths = config.get_all_pass_depths();
    assert_eq!(depths.len(), 3);
    assert!(depths[0].abs() < 2.1);
    assert!(depths[2].abs() >= 8.0);
}

#[test]
fn test_multipass_config_adaptive_strategy() {
    let mut config = MultiPassConfig::new(-30.0, 10.0);
    config.set_strategy(DepthStrategy::Adaptive);

    let depths = config.get_all_pass_depths();
    assert_eq!(depths.len(), 3);
}

#[test]
fn test_depth_strategy_names() {
    assert_eq!(DepthStrategy::Constant.name(), "Constant");
    assert_eq!(DepthStrategy::Ramped.name(), "Ramped");
    assert_eq!(DepthStrategy::Adaptive.name(), "Adaptive");
}

#[test]
fn test_multipass_toolpath_generator() {
    let config = MultiPassConfig::new(-20.0, 10.0);
    let gen = MultiPassToolpathGenerator::new(config);

    let mut base_toolpath = Toolpath::new(3.175, -10.0);
    let segment = ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        Point::new(0.0, 0.0),
        Point::new(10.0, -10.0),
        100.0,
        10000,
    );
    base_toolpath.add_segment(segment);

    let multi_pass = gen.generate_multi_pass(&base_toolpath);
    assert!(multi_pass.segments.len() >= 2);
}

#[test]
fn test_spiral_ramp_generation() {
    let config = MultiPassConfig::new(-10.0, 10.0);
    let gen = MultiPassToolpathGenerator::new(config);

    let toolpath = gen.generate_spiral_ramp(Point::new(50.0, 50.0), 10.0, -10.0, 100.0);
    assert!(toolpath.segments.len() > 0);
}
