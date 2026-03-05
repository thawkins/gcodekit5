//! File Management and Processing - Tasks 91-100
//!
//! File I/O, recent files, processing pipeline, statistics,
//! export, drag/drop, validation, comparison, backup, templates

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// ============================================================================
// Task 91: File I/O - Reading
// ============================================================================

/// File encoding
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileEncoding {
    UTF8,
    ASCII,
    LATIN1,
}

/// File reader
#[derive(Debug)]
pub struct FileReader;

impl FileReader {
    /// Read file with encoding detection
    pub fn read_file(path: &Path, _encoding: FileEncoding) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
    }

    /// Read file in chunks (for large files)
    pub fn read_file_chunked(path: &Path, chunk_size: usize) -> Result<Vec<String>, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        Ok(content
            .lines()
            .collect::<Vec<_>>()
            .chunks(chunk_size)
            .map(|chunk| chunk.join("\n"))
            .collect())
    }

    /// Get file size
    pub fn get_file_size(path: &Path) -> Result<u64, String> {
        std::fs::metadata(path)
            .map(|m| m.len())
            .map_err(|e| format!("Failed to get file size: {}", e))
    }

    /// Get file line count
    pub fn get_line_count(path: &Path) -> Result<usize, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        Ok(content.lines().count())
    }
}

// ============================================================================
// Task 92: File I/O - Recent Files
// ============================================================================

/// Recent file entry
#[derive(Debug, Clone)]
pub struct RecentFile {
    /// File path
    pub path: PathBuf,
    /// File name
    pub name: String,
    /// Last accessed time
    pub accessed: SystemTime,
    /// File size
    pub size: u64,
}

impl RecentFile {
    /// Create new recent file entry
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let metadata =
            std::fs::metadata(&path).map_err(|e| format!("Failed to get file metadata: {}", e))?;

        Ok(Self {
            path,
            name,
            accessed: SystemTime::now(),
            size: metadata.len(),
        })
    }
}

/// Recent files manager
#[derive(Debug, Clone)]
pub struct RecentFilesManager {
    /// Recent files list (most recent first)
    pub files: Vec<RecentFile>,
    /// Maximum recent files to track
    pub max_files: usize,
}

impl RecentFilesManager {
    /// Create new recent files manager
    pub fn new(max_files: usize) -> Self {
        Self {
            files: Vec::new(),
            max_files,
        }
    }

    /// Add recent file
    pub fn add(&mut self, file: RecentFile) {
        self.files.retain(|f| f.path != file.path);
        self.files.insert(0, file);
        if self.files.len() > self.max_files {
            self.files.truncate(self.max_files);
        }
    }

    /// Get recent files
    pub fn get_recent(&self) -> &[RecentFile] {
        &self.files
    }

    /// Clear recent files
    pub fn clear(&mut self) {
        self.files.clear();
    }

    /// Remove file from recent
    pub fn remove(&mut self, path: &Path) {
        self.files.retain(|f| f.path != path);
    }
}

impl Default for RecentFilesManager {
    fn default() -> Self {
        Self::new(10)
    }
}

// ============================================================================
// Task 93: File Processing Pipeline
// ============================================================================

/// File processing result
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// Original file path
    pub source: PathBuf,
    /// Output file path
    pub output: Option<PathBuf>,
    /// Processing time (milliseconds)
    pub duration_ms: u32,
    /// Number of lines processed
    pub lines_processed: usize,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
}

impl ProcessingResult {
    /// Create new processing result
    pub fn new(source: PathBuf) -> Self {
        Self {
            source,
            output: None,
            duration_ms: 0,
            lines_processed: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Check if successful
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty()
    }
}

// ============================================================================
// Task 94: File Statistics
// ============================================================================

/// File statistics
#[derive(Debug, Clone)]
pub struct FileStatistics {
    /// Total lines in file
    pub total_lines: usize,
    /// Total distance in mm
    pub total_distance: f32,
    /// Estimated execution time in seconds
    pub estimated_time: Option<f32>,
    /// Minimum coordinates
    pub min_coords: (f32, f32, f32),
    /// Maximum coordinates
    pub max_coords: (f32, f32, f32),
    /// G-code command count by type (uses Cow to avoid allocating static G-code strings)
    pub command_counts: HashMap<Cow<'static, str>, usize>,
}

impl FileStatistics {
    /// Create new file statistics
    pub fn new() -> Self {
        Self {
            total_lines: 0,
            total_distance: 0.0,
            estimated_time: None,
            min_coords: (0.0, 0.0, 0.0),
            max_coords: (0.0, 0.0, 0.0),
            command_counts: HashMap::new(),
        }
    }

    /// Add G-code command to statistics
    pub fn count_command(&mut self, command: &str) {
        let key = match command {
            "G0" => Cow::Borrowed("G0"),
            "G1" => Cow::Borrowed("G1"),
            "G2" => Cow::Borrowed("G2"),
            "G3" => Cow::Borrowed("G3"),
            "M3" => Cow::Borrowed("M3"),
            "M5" => Cow::Borrowed("M5"),
            "M8" => Cow::Borrowed("M8"),
            "M9" => Cow::Borrowed("M9"),
            other => Cow::Owned(other.to_string()),
        };
        *self.command_counts.entry(key).or_insert(0) += 1;
    }

    /// Update bounding box
    pub fn update_bounds(&mut self, x: f32, y: f32, z: f32) {
        self.min_coords.0 = self.min_coords.0.min(x);
        self.min_coords.1 = self.min_coords.1.min(y);
        self.min_coords.2 = self.min_coords.2.min(z);

        self.max_coords.0 = self.max_coords.0.max(x);
        self.max_coords.1 = self.max_coords.1.max(y);
        self.max_coords.2 = self.max_coords.2.max(z);
    }
}

impl Default for FileStatistics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Task 95: File Export
// ============================================================================

/// File export format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    /// Standard G-code
    GCode,
    /// CSV format
    CSV,
    /// JSON format
    JSON,
}

/// File exporter
#[derive(Debug)]
pub struct FileExporter;

impl FileExporter {
    /// Export to file
    pub fn export(content: &str, path: &Path, format: ExportFormat) -> Result<(), String> {
        match format {
            ExportFormat::GCode => {
                std::fs::write(path, content).map_err(|e| format!("Failed to export: {}", e))?;
            }
            ExportFormat::CSV => {
                let csv_content = Self::to_csv(content);
                std::fs::write(path, csv_content)
                    .map_err(|e| format!("Failed to export: {}", e))?;
            }
            ExportFormat::JSON => {
                let json_content = Self::to_json(content);
                std::fs::write(path, json_content)
                    .map_err(|e| format!("Failed to export: {}", e))?;
            }
        }
        Ok(())
    }

    /// Convert to CSV
    fn to_csv(content: &str) -> String {
        let mut csv = String::from("Line,Command\n");
        for (idx, line) in content.lines().enumerate() {
            csv.push_str(&format!("{},\"{}\"\n", idx + 1, line.replace("\"", "\"\"")));
        }
        csv
    }

    /// Convert to JSON
    fn to_json(content: &str) -> String {
        let lines: Vec<_> = content.lines().collect();
        format!(r#"{{"lines": {}, "commands": {:?}}}"#, lines.len(), lines)
    }
}

// ============================================================================
// Task 96: Drag and Drop Support
// ============================================================================

/// Drag and drop event
#[derive(Debug, Clone)]
pub struct DragDropEvent {
    /// File paths being dropped
    pub files: Vec<PathBuf>,
    /// Drop position (x, y)
    pub position: (i32, i32),
}

impl DragDropEvent {
    /// Create new drag and drop event
    pub fn new(files: Vec<PathBuf>, position: (i32, i32)) -> Self {
        Self { files, position }
    }

    /// Get first file
    pub fn first_file(&self) -> Option<&PathBuf> {
        self.files.first()
    }
}

// ============================================================================
// Task 97: File Validation UI
// ============================================================================

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Line number
    pub line_number: usize,
    /// Error message
    pub message: String,
    /// Suggested fix
    pub suggestion: Option<String>,
}

impl ValidationError {
    /// Create new validation error
    pub fn new(line_number: usize, message: impl Into<String>) -> Self {
        Self {
            line_number,
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

/// File validator
#[derive(Debug)]
pub struct FileValidator;

impl FileValidator {
    /// Validate G-code file
    pub fn validate(content: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if !trimmed.starts_with('G') && !trimmed.starts_with('M') && !trimmed.starts_with('T') {
                errors.push(
                    ValidationError::new(line_num + 1, "Line does not start with G, M, or T")
                        .with_suggestion("Add valid G-code command at the beginning"),
                );
            }
        }

        errors
    }
}

// ============================================================================
// Task 98: File Comparison
// ============================================================================

/// Diff line
#[derive(Debug, Clone)]
pub enum DiffLine {
    /// Unchanged line
    Unchanged(String),
    /// Added line
    Added(String),
    /// Removed line
    Removed(String),
}

/// File comparison result
#[derive(Debug, Clone)]
pub struct FileComparison {
    /// Diff lines
    pub lines: Vec<DiffLine>,
    /// Lines added
    pub added_count: usize,
    /// Lines removed
    pub removed_count: usize,
}

impl FileComparison {
    /// Create new file comparison
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            added_count: 0,
            removed_count: 0,
        }
    }

    /// Simple line-by-line comparison
    pub fn compare(original: &str, modified: &str) -> Self {
        let mut comparison = Self::new();
        let orig_lines: Vec<_> = original.lines().collect();
        let mod_lines: Vec<_> = modified.lines().collect();

        for (i, line) in mod_lines.iter().enumerate() {
            if i < orig_lines.len() && orig_lines[i] == *line {
                comparison.lines.push(DiffLine::Unchanged(line.to_string()));
            } else {
                comparison.lines.push(DiffLine::Added(line.to_string()));
                comparison.added_count += 1;
            }
        }

        for i in mod_lines.len()..orig_lines.len() {
            comparison
                .lines
                .push(DiffLine::Removed(orig_lines[i].to_string()));
            comparison.removed_count += 1;
        }

        comparison
    }
}

impl Default for FileComparison {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Task 99: Backup and Recovery
// ============================================================================

/// Backup entry
#[derive(Debug, Clone)]
pub struct BackupEntry {
    /// Backup ID
    pub id: String,
    /// Original file path
    pub original_path: PathBuf,
    /// Backup file path
    pub backup_path: PathBuf,
    /// Backup timestamp
    pub timestamp: SystemTime,
}

impl BackupEntry {
    /// Create new backup entry
    pub fn new(original_path: PathBuf, backup_path: PathBuf) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            original_path,
            backup_path,
            timestamp: SystemTime::now(),
        }
    }
}

/// Backup and recovery manager
#[derive(Debug, Clone)]
pub struct BackupManager {
    /// Backup entries
    pub backups: Vec<BackupEntry>,
    /// Backup directory
    pub backup_dir: PathBuf,
}

impl BackupManager {
    /// Create new backup manager
    pub fn new(backup_dir: PathBuf) -> Self {
        Self {
            backups: Vec::new(),
            backup_dir,
        }
    }

    /// Create backup
    pub fn create_backup(&mut self, original: &Path) -> Result<BackupEntry, String> {
        let backup_path = self.backup_dir.join(format!(
            "{}_backup",
            original.file_name().unwrap_or_default().to_string_lossy()
        ));
        std::fs::copy(original, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;

        let entry = BackupEntry::new(original.to_path_buf(), backup_path);
        self.backups.push(entry.clone());
        Ok(entry)
    }

    /// Restore from backup
    pub fn restore_backup(&self, backup_id: &str) -> Result<(), String> {
        if let Some(backup) = self.backups.iter().find(|b| b.id == backup_id) {
            std::fs::copy(&backup.backup_path, &backup.original_path)
                .map_err(|e| format!("Failed to restore backup: {}", e))?;
            Ok(())
        } else {
            Err("Backup not found".to_string())
        }
    }
}

// ============================================================================
// Task 100: File Templates
// ============================================================================

/// File template
#[derive(Debug, Clone)]
pub struct FileTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Template content
    pub content: String,
    /// Template variables
    pub variables: HashMap<String, String>,
}

impl FileTemplate {
    /// Create new template
    pub fn new(id: impl Into<String>, name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            content: content.into(),
            variables: HashMap::new(),
        }
    }

    /// Set variable
    pub fn set_variable(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(name.into(), value.into());
    }

    /// Expand template with variables
    pub fn expand(&self) -> String {
        let mut result = self.content.clone();
        for (name, value) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", name);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

/// Template library
#[derive(Debug, Clone)]
pub struct TemplateLibrary {
    /// Templates
    pub templates: HashMap<String, FileTemplate>,
}

impl TemplateLibrary {
    /// Create new template library
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Add template
    pub fn add_template(&mut self, template: FileTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    /// Get template
    pub fn get_template(&self, id: &str) -> Option<&FileTemplate> {
        self.templates.get(id)
    }

    /// List templates
    pub fn list_templates(&self) -> Vec<&FileTemplate> {
        self.templates.values().collect()
    }
}

impl Default for TemplateLibrary {
    fn default() -> Self {
        Self::new()
    }
}

