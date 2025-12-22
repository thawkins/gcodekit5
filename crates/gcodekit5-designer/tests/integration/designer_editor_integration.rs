use gcodekit5_designer::designer_editor_integration::{
    DesignEditorIntegration, DesignExport, ExportParameters,
};

#[test]
fn test_design_export_creation() {
    let params = ExportParameters::default();
    let export = DesignExport::new("Test Design".to_string(), "G00 X0 Y0\n".to_string(), params);

    assert_eq!(export.name, "Test Design");
    assert_eq!(export.gcode_lines(), 1);
}

#[test]
fn test_integration_export_design() {
    let mut integration = DesignEditorIntegration::new();
    let params = ExportParameters::default();
    let export = DesignExport::new("Test".to_string(), "G-code here".to_string(), params);

    let export_id = integration.export_design(None, export);
    assert!(!export_id.is_empty());
    assert!(integration.get_export(&export_id).is_some());
}

#[test]
fn test_integration_recent_exports() {
    let mut integration = DesignEditorIntegration::new();
    let params = ExportParameters::default();

    for i in 0..5 {
        let export = DesignExport::new(
            format!("Design {}", i),
            "G-code".to_string(),
            params.clone(),
        );
        integration.export_design(None, export);
    }

    let recent = integration.get_recent_exports();
    assert_eq!(recent.len(), 5);
}

#[test]
fn test_integration_max_recent() {
    let mut integration = DesignEditorIntegration::new();
    // integration.max_recent = 3; // Private field, using default 10

    let params = ExportParameters::default();

    for i in 0..15 {
        let export = DesignExport::new(
            format!("Design {}", i),
            "G-code".to_string(),
            params.clone(),
        );
        integration.export_design(None, export);
    }

    let recent = integration.get_recent_exports();
    assert_eq!(recent.len(), 10);
}

#[test]
fn test_integration_delete_export() {
    let mut integration = DesignEditorIntegration::new();
    let params = ExportParameters::default();
    let export = DesignExport::new("Test".to_string(), "G-code".to_string(), params);

    let export_id = integration.export_design(None, export);
    assert!(integration.get_export(&export_id).is_some());

    assert!(integration.delete_export(&export_id));
    assert!(integration.get_export(&export_id).is_none());
}

#[test]
fn test_integration_stats() {
    let mut integration = DesignEditorIntegration::new();
    let params = ExportParameters::default();

    let export1 = DesignExport::new(
        "Design 1".to_string(),
        "G00 X0 Y0\nG01 X10 Y10\n".to_string(),
        params.clone(),
    );
    let export2 = DesignExport::new("Design 2".to_string(), "G00 X0 Y0\n".to_string(), params);

    integration.export_design(None, export1);
    integration.export_design(None, export2);

    let stats = integration.stats();
    assert_eq!(stats.total_exports, 2);
    assert_eq!(stats.total_gcode_lines, 3);
}

#[test]
fn test_integration_design_tracking() {
    let mut integration = DesignEditorIntegration::new();
    let design_id = Some("design_123".to_string());
    let params = ExportParameters::default();
    let export = DesignExport::new("Test".to_string(), "G-code".to_string(), params);

    integration.export_design(design_id.clone(), export);

    let exports = integration.get_design_exports(design_id.as_ref().unwrap());
    assert_eq!(exports.len(), 1);
}

#[test]
fn test_export_parameters_default() {
    let params = ExportParameters::default();
    assert_eq!(params.tool_diameter, 3.0);
    assert_eq!(params.cut_depth, 5.0);
    assert_eq!(params.feed_rate, 500.0);
    assert_eq!(params.spindle_speed, 12000);
    assert_eq!(params.safe_z, 10.0);
}

#[test]
fn test_gcode_size() {
    let params = ExportParameters::default();
    let gcode = "G00 X0 Y0\nG01 X10 Y10\nM30\n".to_string();
    let export = DesignExport::new("Test".to_string(), gcode.clone(), params);

    assert_eq!(export.gcode_size(), gcode.len());
}

#[test]
fn test_clear_old_exports() {
    let mut integration = DesignEditorIntegration::new();
    let params = ExportParameters::default();

    for i in 0..10 {
        let export = DesignExport::new(
            format!("Design {}", i),
            "G-code".to_string(),
            params.clone(),
        );
        integration.export_design(None, export);
    }

    assert_eq!(integration.stats().total_exports, 10);

    integration.clear_old_exports(5);
    assert_eq!(integration.stats().total_exports, 5);
}
