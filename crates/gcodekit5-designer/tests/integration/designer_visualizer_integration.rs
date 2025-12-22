use gcodekit5_designer::designer_visualizer_integration::{
    DesignVisualization, DesignerVisualizerIntegration, MaterialSettings, SimulationState,
    ToolpathViewSettings, VisualizationBounds,
};

#[test]
fn test_visualization_bounds_creation() {
    let bounds = VisualizationBounds::new(0.0, 0.0, 0.0, 100.0, 100.0, 100.0);
    let (dx, dy, dz) = bounds.dimensions();
    assert_eq!(dx, 100.0);
    assert_eq!(dy, 100.0);
    assert_eq!(dz, 100.0);
}

#[test]
fn test_visualization_bounds_center() {
    let bounds = VisualizationBounds::new(0.0, 0.0, 0.0, 100.0, 100.0, 100.0);
    let (cx, cy, cz) = bounds.center();
    assert_eq!(cx, 50.0);
    assert_eq!(cy, 50.0);
    assert_eq!(cz, 50.0);
}

#[test]
fn test_design_visualization_creation() {
    let bounds = VisualizationBounds::default();
    let viz = DesignVisualization::new("Test Design".to_string(), bounds);
    assert_eq!(viz.name, "Test Design");
    assert!(viz.show_toolpath);
    assert!(viz.show_shapes);
}

#[test]
fn test_material_settings_default() {
    let settings = MaterialSettings::default();
    assert!(settings.show_material_removal);
    assert_eq!(settings.opacity, 0.9);
    assert!(settings.solid_view);
}

#[test]
fn test_toolpath_view_settings_default() {
    let settings = ToolpathViewSettings::default();
    assert!(settings.show_toolpath);
    assert_eq!(settings.line_thickness, 2.0);
    assert!(settings.show_tool_marker);
}

#[test]
fn test_integration_load_design() {
    let mut integration = DesignerVisualizerIntegration::new();
    let bounds = VisualizationBounds::default();
    let viz = DesignVisualization::new("Test".to_string(), bounds);

    integration.load_design(viz);
    assert!(integration.current_visualization().is_some());
}

#[test]
fn test_integration_simulation_state() {
    let mut integration = DesignerVisualizerIntegration::new();
    let bounds = VisualizationBounds::default();
    let viz = DesignVisualization::new("Test".to_string(), bounds);

    integration.load_design(viz);
    assert!(integration.start_simulation());
    assert_eq!(integration.simulation_state, SimulationState::Running);

    assert!(integration.pause_simulation());
    assert_eq!(integration.simulation_state, SimulationState::Paused);

    assert!(integration.resume_simulation());
    assert_eq!(integration.simulation_state, SimulationState::Running);

    integration.stop_simulation();
    assert_eq!(integration.simulation_state, SimulationState::Idle);
}

#[test]
fn test_integration_toggle_visibility() {
    let mut integration = DesignerVisualizerIntegration::new();
    let bounds = VisualizationBounds::default();
    let viz = DesignVisualization::new("Test".to_string(), bounds);

    integration.load_design(viz);

    let toolpath_visible = integration.toggle_toolpath();
    assert!(!toolpath_visible);

    let shapes_visible = integration.toggle_shapes();
    assert!(!shapes_visible);
}

#[test]
fn test_integration_clear() {
    let mut integration = DesignerVisualizerIntegration::new();
    let bounds = VisualizationBounds::default();
    let viz = DesignVisualization::new("Test".to_string(), bounds);

    integration.load_design(viz);
    assert!(integration.current_visualization().is_some());

    integration.clear();
    assert!(integration.current_visualization().is_none());
}

#[test]
fn test_integration_stats() {
    let mut integration = DesignerVisualizerIntegration::new();
    assert!(!integration.stats().has_active_design);

    let bounds = VisualizationBounds::default();
    let viz = DesignVisualization::new("Test".to_string(), bounds);
    integration.load_design(viz);

    let stats = integration.stats();
    assert!(stats.has_active_design);
}

#[test]
fn test_integration_realtime_updates() {
    let mut integration = DesignerVisualizerIntegration::new();
    let bounds = VisualizationBounds::default();
    let viz = DesignVisualization::new("Test".to_string(), bounds);

    integration.load_design(viz);
    integration.enable_realtime_updates(true);

    assert!(
        integration
            .current_visualization()
            .unwrap()
            .real_time_updates
    );
}
