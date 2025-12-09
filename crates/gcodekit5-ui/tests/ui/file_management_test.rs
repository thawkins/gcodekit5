#![cfg(feature = "slint_legacy_tests")]
use gcodekit5_ui::ui::file_management::{RecentFilesManager, RecentFile, FileStatistics, FileComparison, FileTemplate};
use std::path::PathBuf;

#[test]
fn test_recent_files_manager() {
    let mut manager = RecentFilesManager::new(5);
    let file = RecentFile::new(PathBuf::from("/tmp/test.gcode"));
    if let Ok(f) = file {
        manager.add(f);
        assert_eq!(manager.files.len(), 1);
    }
}

#[test]
fn test_file_statistics() {
    let mut stats = FileStatistics::new();
    stats.count_command("G0");
    stats.update_bounds(10.0, 20.0, 30.0);
    assert_eq!(stats.command_counts.get("G0"), Some(&1));
}

#[test]
fn test_file_comparison() {
    let original = "G0 X10\nG1 X20";
    let modified = "G0 X10\nG1 X25";
    let comparison = FileComparison::compare(original, modified);
    assert!(comparison.added_count > 0);
}

#[test]
fn test_file_template() {
    let mut template = FileTemplate::new("test", "Test Template", "G0 X{{x}} Y{{y}}");
    template.set_variable("x", "10");
    template.set_variable("y", "20");
    let expanded = template.expand();
    assert!(expanded.contains("10"));
    assert!(expanded.contains("20"));
}
