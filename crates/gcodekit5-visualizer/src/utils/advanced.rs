//! Advanced Features Module - Tasks 97-102
//!
//! Task 97: File Validation UI - Show validation results
//! Task 98: File Comparison - Compare original vs processed
//! Task 99: Backup and Recovery - Auto-save and recovery
//! Task 100: File Templates - Template system
//! Task 101: Probing - Basic - Z-axis probing
//! Task 102: Probing - Advanced - Multi-point probing

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// TASK 97: FILE VALIDATION UI
// ============================================================================

/// Validation error severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Error - critical issue
    Error,
    /// Warning - potential issue
    Warning,
    /// Info - informational message
    Info,
}

/// Validation issue with location and fix suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Issue line number
    pub line_number: u32,
    /// Issue severity
    pub severity: ValidationSeverity,
    /// Issue message
    pub message: String,
    /// Suggested fix
    pub suggestion: Option<String>,
}

impl ValidationIssue {
    /// Create new validation issue
    pub fn new(line_number: u32, severity: ValidationSeverity, message: impl Into<String>) -> Self {
        Self {
            line_number,
            severity,
            message: message.into(),
            suggestion: None,
        }
    }

    /// Add suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Validation results for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// All issues found
    pub issues: Vec<ValidationIssue>,
    /// Total error count
    pub error_count: u32,
    /// Total warning count
    pub warning_count: u32,
    /// Total info count
    pub info_count: u32,
}

impl ValidationResult {
    /// Create new validation result
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
        }
    }

    /// Add issue and update counts
    pub fn add_issue(&mut self, issue: ValidationIssue) {
        match issue.severity {
            ValidationSeverity::Error => self.error_count += 1,
            ValidationSeverity::Warning => self.warning_count += 1,
            ValidationSeverity::Info => self.info_count += 1,
        }
        self.issues.push(issue);
    }

    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        self.error_count == 0
    }

    /// Get formatted summary
    pub fn summary(&self) -> String {
        format!(
            "Errors: {} | Warnings: {} | Info: {}",
            self.error_count, self.warning_count, self.info_count
        )
    }

    /// Get issues by line number
    pub fn issues_at_line(&self, line_number: u32) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.line_number == line_number)
            .collect()
    }

    /// Get issues by severity
    pub fn issues_by_severity(&self, severity: ValidationSeverity) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == severity)
            .collect()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 98: FILE COMPARISON
// ============================================================================

/// Line comparison result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineChange {
    /// Line unchanged
    Unchanged,
    /// Line was added
    Added,
    /// Line was removed
    Removed,
    /// Line was modified
    Modified,
}

/// File comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileComparison {
    /// Original file content lines
    pub original_lines: Vec<String>,
    /// Processed file content lines
    pub processed_lines: Vec<String>,
    /// Changes per line
    pub line_changes: Vec<LineChange>,
    /// Added lines count
    pub added_count: u32,
    /// Removed lines count
    pub removed_count: u32,
    /// Modified lines count
    pub modified_count: u32,
}

impl FileComparison {
    /// Create new file comparison
    pub fn new(original: &str, processed: &str) -> Self {
        let original_lines: Vec<_> = original.lines().map(|s| s.to_string()).collect();
        let processed_lines: Vec<_> = processed.lines().map(|s| s.to_string()).collect();

        let mut result = Self {
            original_lines,
            processed_lines,
            line_changes: Vec::new(),
            added_count: 0,
            removed_count: 0,
            modified_count: 0,
        };

        result.compute_changes();
        result
    }

    /// Compute line changes
    fn compute_changes(&mut self) {
        let max_len = self.original_lines.len().max(self.processed_lines.len());

        for i in 0..max_len {
            let original = self.original_lines.get(i);
            let processed = self.processed_lines.get(i);

            let change = match (original, processed) {
                (Some(o), Some(p)) => {
                    if o == p {
                        LineChange::Unchanged
                    } else {
                        self.modified_count += 1;
                        LineChange::Modified
                    }
                }
                (Some(_), None) => {
                    self.removed_count += 1;
                    LineChange::Removed
                }
                (None, Some(_)) => {
                    self.added_count += 1;
                    LineChange::Added
                }
                (None, None) => unreachable!(),
            };

            self.line_changes.push(change);
        }
    }

    /// Get total changes
    pub fn total_changes(&self) -> u32 {
        self.added_count + self.removed_count + self.modified_count
    }

    /// Get change percentage
    pub fn change_percentage(&self) -> f64 {
        let total = self.original_lines.len().max(self.processed_lines.len()) as f64;
        if total == 0.0 {
            0.0
        } else {
            (self.total_changes() as f64 / total) * 100.0
        }
    }

    /// Get diff summary
    pub fn summary(&self) -> String {
        format!(
            "Added: {} | Modified: {} | Removed: {} | Change: {:.1}%",
            self.added_count,
            self.modified_count,
            self.removed_count,
            self.change_percentage()
        )
    }
}

// ============================================================================
// TASK 99: BACKUP AND RECOVERY
// ============================================================================

/// Backup entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    /// Backup ID
    pub id: String,
    /// Backup timestamp
    pub timestamp: u64,
    /// Backup file path
    pub backup_path: PathBuf,
    /// Original file path
    pub original_path: PathBuf,
    /// Backup size
    pub size: u64,
    /// Backup description
    pub description: String,
}

/// Backup and recovery manager
pub struct BackupManager {
    /// Backup directory
    backup_dir: PathBuf,
    /// Max backup age in seconds
    max_age: u64,
    /// Max backups to keep
    max_backups: usize,
}

impl BackupManager {
    /// Create new backup manager
    pub fn new(backup_dir: impl AsRef<Path>) -> Self {
        Self {
            backup_dir: backup_dir.as_ref().to_path_buf(),
            max_age: 86400 * 7, // 7 days
            max_backups: 10,
        }
    }

    /// Set max backup age
    pub fn set_max_age(&mut self, seconds: u64) {
        self.max_age = seconds;
    }

    /// Set max backups to keep
    pub fn set_max_backups(&mut self, count: usize) {
        self.max_backups = count;
    }

    /// Create backup
    pub fn backup(&self, source: impl AsRef<Path>, description: &str) -> Result<BackupEntry> {
        let source = source.as_ref();

        // Create backup directory
        fs::create_dir_all(&self.backup_dir)?;

        // Generate backup ID
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let id = format!("{}", timestamp);

        // Create backup
        let backup_path = self.backup_dir.join(&id);
        fs::copy(source, &backup_path)?;

        let size = fs::metadata(&backup_path)?.len();

        Ok(BackupEntry {
            id,
            timestamp,
            backup_path,
            original_path: source.to_path_buf(),
            size,
            description: description.to_string(),
        })
    }

    /// Restore from backup
    pub fn restore(&self, backup: &BackupEntry, dest: impl AsRef<Path>) -> Result<()> {
        let dest = dest.as_ref();
        fs::copy(&backup.backup_path, dest)?;
        Ok(())
    }

    /// List backups
    pub fn list_backups(&self) -> Result<Vec<BackupEntry>> {
        let mut backups = Vec::new();

        if self.backup_dir.exists() {
            for entry in fs::read_dir(&self.backup_dir)? {
                let entry = entry?;
                let path = entry.path();

                if let Some(id) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(timestamp) = id.parse::<u64>() {
                        if let Ok(metadata) = fs::metadata(&path) {
                            backups.push(BackupEntry {
                                id: id.to_string(),
                                timestamp,
                                backup_path: path,
                                original_path: PathBuf::new(),
                                size: metadata.len(),
                                description: String::new(),
                            });
                        }
                    }
                }
            }
        }

        backups.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        Ok(backups)
    }

    /// Clean old backups
    pub fn cleanup(&self) -> Result<()> {
        let mut backups = self.list_backups()?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // Remove old backups
        backups.retain(|b| now - b.timestamp <= self.max_age);

        // Keep only max backups
        if backups.len() > self.max_backups {
            let to_remove = backups.len() - self.max_backups;
            for backup in &backups[..to_remove] {
                fs::remove_file(&backup.backup_path).ok();
            }
        }

        Ok(())
    }
}

// ============================================================================
// TASK 100: FILE TEMPLATES
// ============================================================================

/// Template variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// Variable name
    pub name: String,
    /// Variable description
    pub description: String,
    /// Default value
    pub default_value: String,
}

/// G-code template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcodeTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template content
    pub content: String,
    /// Variables in template
    pub variables: Vec<TemplateVariable>,
}

impl GcodeTemplate {
    /// Create new template
    pub fn new(id: impl Into<String>, name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            content: content.into(),
            variables: Vec::new(),
        }
    }

    /// Expand template with variables
    pub fn expand(&self, values: &HashMap<String, String>) -> String {
        let mut result = self.content.clone();

        for var in &self.variables {
            let value = values
                .get(&var.name)
                .map(|s| s.as_str())
                .unwrap_or(&var.default_value);
            let placeholder = format!("{{{{{}}}}}", var.name);
            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Add variable
    pub fn add_variable(&mut self, variable: TemplateVariable) {
        self.variables.push(variable);
    }
}

/// Template library
pub struct TemplateLibrary {
    /// Templates by ID
    templates: HashMap<String, GcodeTemplate>,
}

impl TemplateLibrary {
    /// Create new template library
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Add template
    pub fn add(&mut self, template: GcodeTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    /// Get template
    pub fn get(&self, id: &str) -> Option<&GcodeTemplate> {
        self.templates.get(id)
    }

    /// List all templates
    pub fn list(&self) -> Vec<&GcodeTemplate> {
        self.templates.values().collect()
    }

    /// Remove template
    pub fn remove(&mut self, id: &str) -> Option<GcodeTemplate> {
        self.templates.remove(id)
    }
}

impl Default for TemplateLibrary {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 101: PROBING - BASIC
// ============================================================================

/// Probe result
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProbeResult {
    /// Probed Z position
    pub z: f64,
    /// Probe status (success/failure)
    pub success: bool,
}

/// Z-axis probing
pub struct BasicProber {
    /// Current Z position
    pub current_z: f64,
    /// Probe offset (tool length)
    pub probe_offset: f64,
    /// Feed rate for probing
    pub probe_feed_rate: f64,
}

impl BasicProber {
    /// Create new basic prober
    pub fn new() -> Self {
        Self {
            current_z: 0.0,
            probe_offset: 0.0,
            probe_feed_rate: 50.0,
        }
    }

    /// Generate probe command
    pub fn generate_probe_command(&self, target_z: f64) -> String {
        format!(
            "; Probe to Z: {} at F{}\nG38.2 Z{} F{}\n",
            target_z, self.probe_feed_rate, target_z, self.probe_feed_rate
        )
    }

    /// Calculate work offset from probe
    pub fn calculate_offset(&self, probed_z: f64) -> f64 {
        probed_z - self.probe_offset
    }

    /// Generate offset command
    pub fn generate_offset_command(&self, work_offset: f64) -> String {
        format!("; Set work offset\nG92 Z{}\n", work_offset)
    }
}

impl Default for BasicProber {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 102: PROBING - ADVANCED
// ============================================================================

/// Probe point
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProbePoint {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Probed Z coordinate
    pub z: f64,
}

impl ProbePoint {
    /// Create new probe point
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// Advanced probing with multiple points
pub struct AdvancedProber {
    /// Base prober
    base: BasicProber,
    /// Probe points
    probe_points: Vec<ProbePoint>,
}

impl AdvancedProber {
    /// Create new advanced prober
    pub fn new() -> Self {
        Self {
            base: BasicProber::new(),
            probe_points: Vec::new(),
        }
    }

    /// Add probe point
    pub fn add_probe_point(&mut self, x: f64, y: f64) {
        self.probe_points.push(ProbePoint::new(x, y, 0.0));
    }

    /// Generate multi-point probe sequence
    pub fn generate_probe_sequence(&self) -> String {
        let mut sequence = String::new();

        sequence.push_str("; Multi-point probing sequence\n");

        for (i, point) in self.probe_points.iter().enumerate() {
            sequence.push_str(&format!(
                "; Probe point {}\nG0 X{} Y{}\n",
                i + 1,
                point.x,
                point.y
            ));
            sequence.push_str(&self.base.generate_probe_command(-50.0)); // Probe down
        }

        sequence.push_str("; Return to start\nG0 Z10\n");
        sequence
    }

    /// Generate corner finding sequence
    pub fn generate_corner_finding(&self, corner_1: (f64, f64), corner_2: (f64, f64)) -> String {
        let mut sequence = String::new();
        sequence.push_str("; Corner finding sequence\n");
        sequence.push_str(&format!(
            "G0 X{} Y{}\n; Probe corner 1\n",
            corner_1.0, corner_1.1
        ));
        sequence.push_str(&self.base.generate_probe_command(-50.0));
        sequence.push_str(&format!(
            "G0 X{} Y{}\n; Probe corner 2\n",
            corner_2.0, corner_2.1
        ));
        sequence.push_str(&self.base.generate_probe_command(-50.0));
        sequence.push_str("; Return\nG0 Z10\n");
        sequence
    }

    /// Generate center finding on circular feature
    pub fn generate_center_finding(&self, center: (f64, f64), radius: f64) -> String {
        let mut sequence = String::new();
        sequence.push_str("; Center finding on circular feature\n");

        let points = 4;
        for i in 0..points {
            let angle = (i as f64 / points as f64) * std::f64::consts::TAU;
            let x = center.0 + radius * angle.cos();
            let y = center.1 + radius * angle.sin();

            sequence.push_str(&format!(
                "G0 X{:.3} Y{:.3}\n; Probe point {}\n",
                x,
                y,
                i + 1
            ));
            sequence.push_str(&self.base.generate_probe_command(-50.0));
        }

        sequence.push_str("; Return\nG0 Z10\n");
        sequence
    }

    /// Get probe points
    pub fn probe_points(&self) -> &[ProbePoint] {
        &self.probe_points
    }

    /// Clear probe points
    pub fn clear_probe_points(&mut self) {
        self.probe_points.clear();
    }
}

impl Default for AdvancedProber {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_issue_creation() {
        let issue = ValidationIssue::new(1, ValidationSeverity::Error, "Test error");
        assert_eq!(issue.line_number, 1);
        assert_eq!(issue.severity, ValidationSeverity::Error);
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        result.add_issue(ValidationIssue::new(1, ValidationSeverity::Error, "Error"));
        result.add_issue(ValidationIssue::new(
            2,
            ValidationSeverity::Warning,
            "Warning",
        ));

        assert_eq!(result.error_count, 1);
        assert_eq!(result.warning_count, 1);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_file_comparison() {
        let original = "G0 X10\nG1 Y20\n";
        let processed = "G0 X10\nG1 Y25\n";

        let comparison = FileComparison::new(original, processed);
        assert_eq!(comparison.original_lines.len(), 2);
        assert_eq!(comparison.modified_count, 1);
    }

    #[test]
    fn test_template_expansion() {
        let mut template = GcodeTemplate::new("move", "Move Template", "G0 X{{X}} Y{{Y}}");
        template.add_variable(TemplateVariable {
            name: "X".to_string(),
            description: "X position".to_string(),
            default_value: "0".to_string(),
        });
        template.add_variable(TemplateVariable {
            name: "Y".to_string(),
            description: "Y position".to_string(),
            default_value: "0".to_string(),
        });

        let mut values = HashMap::new();
        values.insert("X".to_string(), "10".to_string());
        values.insert("Y".to_string(), "20".to_string());

        let expanded = template.expand(&values);
        assert!(expanded.contains("X10"));
        assert!(expanded.contains("Y20"));
    }

    #[test]
    fn test_basic_prober() {
        let prober = BasicProber::new();
        let command = prober.generate_probe_command(-10.0);
        assert!(command.contains("G38.2"));
    }

    #[test]
    fn test_advanced_prober() {
        let mut prober = AdvancedProber::new();
        prober.add_probe_point(0.0, 0.0);
        prober.add_probe_point(10.0, 10.0);

        assert_eq!(prober.probe_points().len(), 2);

        let sequence = prober.generate_probe_sequence();
        assert!(sequence.contains("Multi-point"));
    }

    #[test]
    fn test_template_library() {
        let mut library = TemplateLibrary::new();
        let template = GcodeTemplate::new("move", "Move", "G0 X10");
        library.add(template);

        assert!(library.get("move").is_some());
        assert_eq!(library.list().len(), 1);
    }
}
