use gcodekit5_designer::model::Point;
use gcodekit5_designer::toolpath::{Toolpath, ToolpathSegment, ToolpathSegmentType};
use gcodekit5_designer::toolpath_simulation::{
    MaterialRemovalInfo, SimulationState, ToolpathAnalyzer, ToolpathSimulator,
};

#[test]
fn test_simulation_state_names() {
    assert_eq!(SimulationState::Idle.name(), "Idle");
    assert_eq!(SimulationState::Running.name(), "Running");
    assert_eq!(SimulationState::Complete.name(), "Complete");
}

#[test]
fn test_toolpath_simulator_creation() {
    let toolpath = Toolpath::new(3.175, -5.0);
    let sim = ToolpathSimulator::new(toolpath);
    assert_eq!(sim.get_state(), SimulationState::Idle);
}

#[test]
fn test_toolpath_simulator_start_pause_resume() {
    let toolpath = Toolpath::new(3.175, -5.0);
    let mut sim = ToolpathSimulator::new(toolpath);

    sim.start();
    assert_eq!(sim.get_state(), SimulationState::Running);

    sim.pause();
    assert_eq!(sim.get_state(), SimulationState::Paused);

    sim.resume();
    assert_eq!(sim.get_state(), SimulationState::Running);
}

#[test]
fn test_toolpath_simulator_reset() {
    let toolpath = Toolpath::new(3.175, -5.0);
    let mut sim = ToolpathSimulator::new(toolpath);

    sim.start();
    sim.reset();
    assert_eq!(sim.get_state(), SimulationState::Idle);
    assert_eq!(sim.get_current_time(), 0.0);
}

#[test]
fn test_material_removal_info() {
    let mut info = MaterialRemovalInfo::new(100.0);
    assert_eq!(info.percentage_complete, 0.0);

    info.update(25.0);
    assert_eq!(info.percentage_complete, 25.0);

    info.update(150.0);
    assert_eq!(info.percentage_complete, 100.0);
}

#[test]
fn test_toolpath_analyzer_creation() {
    let toolpath = Toolpath::new(3.175, -5.0);
    let analyzer = ToolpathAnalyzer::new(toolpath);
    assert_eq!(analyzer.calculate_total_length(), 0.0);
}

#[test]
fn test_toolpath_analyzer_segment_counting() {
    let mut toolpath = Toolpath::new(3.175, -5.0);
    let segment = ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
        100.0,
        10000,
    );
    toolpath.add_segment(segment);

    let analyzer = ToolpathAnalyzer::new(toolpath);
    let (rapid, linear, arc) = analyzer.count_segments_by_type();
    assert_eq!(linear, 1);
    assert_eq!(rapid, 0);
    assert_eq!(arc, 0);
}
