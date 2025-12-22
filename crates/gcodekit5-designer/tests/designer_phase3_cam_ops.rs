// Integration tests for Phase 3 CAM operations.

use gcodekit5_designer::pocket_operations::PocketStrategy;
use gcodekit5_designer::{
    DepthStrategy, DrillOperation, DrillingPattern, DrillingPatternGenerator, MultiPassConfig,
    MultiPassToolpathGenerator, PatternType, PocketGenerator, PocketOperation, Point, Rectangle,
    SimulationState, ToolLibrary, ToolType, Toolpath, ToolpathAnalyzer, ToolpathSegment,
    ToolpathSegmentType, ToolpathSimulator,
};

#[test]
fn test_phase3_tool_library_creation() {
    let library = ToolLibrary::with_defaults();
    assert!(library.get_tool("em_125").is_some());
    assert!(library.get_tool("drill_32").is_some());
    assert!(library.get_default_tool().is_some());
}

#[test]
fn test_phase3_tool_library_tool_selection() {
    let library = ToolLibrary::with_defaults();
    let end_mills = library.list_tools_by_type(ToolType::EndMill);
    assert!(end_mills.len() > 0);
}

#[test]
fn test_phase3_pocket_rectangular() {
    let op = PocketOperation::new("pocket1".to_string(), -10.0, 3.175);
    let gen = PocketGenerator::new(op);
    let rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);

    let toolpaths = gen.generate_rectangular_pocket(&rect, 10.0);
    assert!(toolpaths.len() > 0);
    let toolpath = &toolpaths[0];
    assert!(toolpath.segments.len() > 0);
    assert_eq!(toolpath.tool_diameter, 3.175);
    assert_eq!(toolpath.depth, -10.0);
}

#[test]
fn test_phase3_pocket_with_islands() {
    let op = PocketOperation::new("pocket1".to_string(), -10.0, 3.175);
    let mut gen = PocketGenerator::new(op);
    gen.add_circular_island(Point::new(50.0, 50.0), 15.0);

    let rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
    let toolpaths = gen.generate_rectangular_pocket(&rect, 10.0);
    assert!(toolpaths.len() > 0);
    assert!(toolpaths[0].segments.len() > 0);
}

#[test]
fn test_phase3_pocket_offset_paths() {
    let op = PocketOperation::new("pocket1".to_string(), -10.0, 3.175);
    let gen = PocketGenerator::new(op);
    let rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);

    let paths = gen.generate_offset_paths(&rect, 5);
    assert!(paths.len() > 0);
}

#[test]
fn test_phase3_drilling_linear_pattern() {
    let op = DrillOperation::new("drill1".to_string(), 6.35, 6.35, -15.0);
    let gen = DrillingPatternGenerator::new(op);

    let toolpath = gen.generate_linear_pattern(Point::new(0.0, 0.0), Point::new(100.0, 0.0), 5);
    assert_eq!(toolpath.segments.len(), 5);
}

#[test]
fn test_phase3_drilling_circular_pattern() {
    let op = DrillOperation::new("drill1".to_string(), 6.35, 6.35, -15.0);
    let gen = DrillingPatternGenerator::new(op);

    let toolpath = gen.generate_circular_pattern(Point::new(50.0, 50.0), 25.0, 8);
    assert_eq!(toolpath.segments.len(), 8);
}

#[test]
fn test_phase3_drilling_grid_pattern() {
    let op = DrillOperation::new("drill1".to_string(), 6.35, 6.35, -15.0);
    let gen = DrillingPatternGenerator::new(op);

    let toolpath = gen.generate_grid_pattern(Point::new(0.0, 0.0), 10.0, 10.0, 5, 3);
    assert_eq!(toolpath.segments.len(), 15);
}

#[test]
fn test_phase3_drilling_peck() {
    let mut op = DrillOperation::new("drill1".to_string(), 6.35, 6.35, -15.0);
    op.set_peck_drilling(5.0);

    let pecks = op.calculate_pecks();
    assert_eq!(pecks, 3);
}

#[test]
fn test_phase3_multipass_constant() {
    let config = MultiPassConfig::new(-30.0, 10.0);
    assert_eq!(config.calculate_passes(), 3);

    let depths = config.get_all_pass_depths();
    assert_eq!(depths[0], -10.0);
    assert_eq!(depths[1], -20.0);
    assert_eq!(depths[2], -30.0);
}

#[test]
fn test_phase3_multipass_ramped() {
    let mut config = MultiPassConfig::new(-30.0, 10.0);
    config.set_strategy(DepthStrategy::Ramped);
    config.set_minimum_depth(2.0);

    let depths = config.get_all_pass_depths();
    assert!(depths[0].abs() < depths[2].abs());
}

#[test]
fn test_phase3_multipass_adaptive() {
    let mut config = MultiPassConfig::new(-30.0, 10.0);
    config.set_strategy(DepthStrategy::Adaptive);

    let depths = config.get_all_pass_depths();
    assert_eq!(depths.len(), 3);
}

#[test]
fn test_phase3_multipass_generation() {
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
fn test_phase3_spiral_ramp() {
    let config = MultiPassConfig::new(-10.0, 10.0);
    let gen = MultiPassToolpathGenerator::new(config);

    let toolpath = gen.generate_spiral_ramp(Point::new(50.0, 50.0), 10.0, -10.0, 100.0);
    assert!(toolpath.segments.len() > 0);
}

#[test]
fn test_phase3_toolpath_simulation_lifecycle() {
    let toolpath = Toolpath::new(3.175, -5.0);
    let mut sim = ToolpathSimulator::new(toolpath);

    assert_eq!(sim.get_state(), SimulationState::Idle);

    sim.start();
    assert_eq!(sim.get_state(), SimulationState::Running);

    sim.pause();
    assert_eq!(sim.get_state(), SimulationState::Paused);

    sim.reset();
    assert_eq!(sim.get_state(), SimulationState::Idle);
}

#[test]
fn test_phase3_toolpath_simulator_progress() {
    let mut toolpath = Toolpath::new(3.175, -5.0);
    let segment = ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
        100.0,
        10000,
    );
    toolpath.add_segment(segment);

    let mut sim = ToolpathSimulator::new(toolpath);
    sim.simulate_all();

    assert!(sim.get_progress_percentage() > 0.0);
}

#[test]
fn test_phase3_toolpath_analyzer_length() {
    let mut toolpath = Toolpath::new(3.175, -5.0);
    let segment = ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        100.0,
        10000,
    );
    toolpath.add_segment(segment);

    let analyzer = ToolpathAnalyzer::new(toolpath);
    assert_eq!(analyzer.calculate_total_length(), 10.0);
}

#[test]
fn test_phase3_toolpath_analyzer_machining_time() {
    let mut toolpath = Toolpath::new(3.175, -5.0);
    let segment = ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        100.0,
        10000,
    );
    toolpath.add_segment(segment);

    let analyzer = ToolpathAnalyzer::new(toolpath);
    let time = analyzer.calculate_machining_time();
    assert!(time > 0.0);
}

#[test]
fn test_phase3_toolpath_analyzer_volume() {
    let mut toolpath = Toolpath::new(3.175, -10.0);
    let segment = ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        100.0,
        10000,
    );
    toolpath.add_segment(segment);

    let analyzer = ToolpathAnalyzer::new(toolpath);
    let volume = analyzer.estimate_volume_removed();
    assert!(volume > 0.0);
}

#[test]
fn test_phase3_toolpath_analyzer_surface_finish() {
    let mut toolpath = Toolpath::new(3.175, -5.0);
    let segment = ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        50.0,
        10000,
    );
    toolpath.add_segment(segment);

    let analyzer = ToolpathAnalyzer::new(toolpath);
    let finish = analyzer.analyze_surface_finish();
    assert!(!finish.is_empty());
}

#[test]
fn test_phase3_drilling_pattern_types() {
    let linear = DrillingPattern::linear(Point::new(0.0, 0.0), Point::new(100.0, 0.0), 5);
    assert_eq!(linear.pattern_type, PatternType::Linear);

    let circular = DrillingPattern::circular(Point::new(50.0, 50.0), 25.0, 8);
    assert_eq!(circular.pattern_type, PatternType::Circular);

    let grid = DrillingPattern::grid(Point::new(0.0, 0.0), 10.0, 10.0, 5, 3);
    assert_eq!(grid.pattern_type, PatternType::Grid);
}

#[test]
fn test_phase3_combined_workflow() {
    // Create tool library
    let library = ToolLibrary::with_defaults();
    let tool = library.get_tool("em_125").unwrap();

    // Create pocket operation
    let pocket_op = PocketOperation::new("pocket1".to_string(), -10.0, tool.diameter);
    let pocket_gen = PocketGenerator::new(pocket_op);

    // Generate pocket toolpath
    let rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
    let pocket_toolpaths = pocket_gen.generate_rectangular_pocket(&rect, 10.0);
    let pocket_toolpath = &pocket_toolpaths[0];

    // Configure multi-pass for deep cut
    let multipass_config = MultiPassConfig::new(-20.0, 10.0);
    let multipass_gen = MultiPassToolpathGenerator::new(multipass_config);
    let multipass_toolpath = multipass_gen.generate_multi_pass(pocket_toolpath);

    // Simulate the toolpath
    let mut sim = ToolpathSimulator::new(multipass_toolpath.clone());
    sim.simulate_all();

    // Analyze the results
    let analyzer = ToolpathAnalyzer::new(multipass_toolpath);
    let total_time = analyzer.calculate_machining_time();
    let volume = analyzer.estimate_volume_removed();

    assert!(total_time > 0.0);
    assert!(volume > 0.0);
}

#[test]
fn test_phase3_pocket_with_duplicate_vertices() {
    let op = PocketOperation::new("pocket_dup".to_string(), -5.0, 3.175);
    let gen = PocketGenerator::new(op);

    // Create a polygon with duplicate vertices
    let vertices = vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 0.0), // Duplicate
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
        Point::new(0.0, 0.0), // Closing point (same as first)
    ];

    // This should not panic
    let toolpaths = gen.generate_polygon_pocket(&vertices, 5.0);
    assert!(toolpaths.len() > 0);
}

#[test]
fn test_phase3_pocket_adaptive() {
    let mut op = PocketOperation::new("pocket_adaptive".to_string(), -5.0, 3.175);
    op.set_strategy(PocketStrategy::Adaptive);
    let gen = PocketGenerator::new(op);

    let rect = Rectangle::new(0.0, 0.0, 50.0, 50.0);
    let toolpaths = gen.generate_rectangular_pocket(&rect, 5.0);

    assert!(toolpaths.len() > 0);
    let toolpath = &toolpaths[0];

    // Check if we have segments
    assert!(toolpath.segments.len() > 0);

    // Check if we have a helical entry (Rapid to start, then Linear moves)
    // The current implementation just does Rapid to start.
    // But we should verify it generates paths.
}
