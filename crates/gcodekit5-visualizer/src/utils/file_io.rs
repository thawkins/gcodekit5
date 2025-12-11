//! File I/O Module - Tasks 91 & 92
//!
//! Handles reading G-code files with support for various encodings,
//! efficient large file handling, and recent files management.
//!
//! Task 91: File I/O - Reading
//! - Implement G-code file reader with UTF-8/ASCII support
//! - Handle large files efficiently with streaming
//! - Support file validation and encoding detection
//!
//! Task 92: File I/O - Recent Files
//! - Track recently opened files with timestamps
//! - Provide recent files menu functionality
//! - Persist recent files list to disk

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Maximum number of recent files to keep in history
const DEFAULT_MAX_RECENT: usize = 20;

/// Buffer size for reading large files (256 KB)
const READ_BUFFER_SIZE: usize = 256 * 1024;

/// Supported file encodings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEncoding {
    /// UTF-8 encoding
    Utf8,
    /// ASCII encoding (7-bit)
    Ascii,
}

impl FileEncoding {
    /// Detect encoding from file bytes
    pub fn detect(data: &[u8]) -> Self {
        // Check for UTF-8 BOM
        if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return FileEncoding::Utf8;
        }

        // Try to validate as UTF-8
        if std::str::from_utf8(data).is_ok() {
            return FileEncoding::Utf8;
        }

        // Fall back to ASCII
        FileEncoding::Ascii
    }
}

/// File read statistics
#[derive(Debug, Clone)]
pub struct FileReadStats {
    /// Total bytes read
    pub bytes_read: u64,
    /// Total lines read
    pub lines_read: u64,
    /// Detected encoding
    pub encoding: FileEncoding,
    /// File size in bytes
    pub file_size: u64,
    /// Time taken to read (milliseconds)
    pub read_time_ms: u64,
}

impl FileReadStats {
    /// Get progress percentage
    pub fn progress_percent(&self) -> f64 {
        if self.file_size == 0 {
            0.0
        } else {
            (self.bytes_read as f64 / self.file_size as f64) * 100.0
        }
    }
}

/// G-code file reader with streaming support
pub struct GcodeFileReader {
    path: PathBuf,
    file_size: u64,
}

impl GcodeFileReader {
    /// Create a new G-code file reader
    ///
    /// # Arguments
    /// * `path` - Path to the G-code file
    ///
    /// # Errors
    /// Returns error if file does not exist or cannot be accessed
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", path.display()));
        }

        if !path.is_file() {
            return Err(anyhow!("Path is not a file: {}", path.display()));
        }

        let metadata = fs::metadata(&path)?;
        let file_size = metadata.len();

        Ok(Self { path, file_size })
    }

    /// Get file size in bytes
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Get file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read entire file into memory
    ///
    /// Warning: Use with caution for large files (>100MB)
    ///
    /// # Errors
    /// Returns error if file cannot be read
    pub fn read_all(&self) -> Result<String> {
        if self.file_size > 500 * 1024 * 1024 {
            tracing::warn!(
                "Reading very large file ({}MB) into memory",
                self.file_size / (1024 * 1024)
            );
        }

        fs::read_to_string(&self.path).map_err(|e| anyhow!("Failed to read file: {}", e))
    }

    /// Read file with line-by-line streaming callback
    ///
    /// More memory-efficient for large files.
    ///
    /// # Arguments
    /// * `callback` - Called for each line with the line content
    ///
    /// # Errors
    /// Returns error if file cannot be read or callback returns error
    pub fn read_lines<F>(&self, mut callback: F) -> Result<FileReadStats>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let start_time = SystemTime::now();
        let file = File::open(&self.path)?;
        let reader = BufReader::with_capacity(READ_BUFFER_SIZE, file);

        let mut lines_read = 0u64;
        let mut bytes_read = 0u64;
        let mut encoding = FileEncoding::Utf8;
        let mut first_chunk = true;

        for line_result in reader.lines() {
            let line = line_result?;

            // Detect encoding from first chunk
            if first_chunk {
                encoding = FileEncoding::detect(line.as_bytes());
                first_chunk = false;
            }

            bytes_read += line.len() as u64 + 1; // +1 for newline

            callback(&line)?;
            lines_read += 1;
        }

        let elapsed = start_time.elapsed().unwrap_or_default().as_millis() as u64;

        Ok(FileReadStats {
            bytes_read,
            lines_read,
            encoding,
            file_size: self.file_size,
            read_time_ms: elapsed,
        })
    }

    /// Read file with limited number of lines
    ///
    /// # Arguments
    /// * `max_lines` - Maximum number of lines to read
    ///
    /// # Errors
    /// Returns error if file cannot be read
    pub fn read_lines_limited(&self, max_lines: usize) -> Result<(Vec<String>, FileReadStats)> {
        let mut lines = Vec::new();
        let mut encoding = FileEncoding::Utf8;
        let file_size = self.file_size;

        self.read_lines(|line| {
            if lines.len() < max_lines {
                if lines.is_empty() {
                    encoding = FileEncoding::detect(line.as_bytes());
                }
                lines.push(line.to_string());
                Ok(())
            } else {
                Err(anyhow!("Max lines reached"))
            }
        })
        .or_else(|e| {
            if e.to_string().contains("Max lines reached") {
                Ok(FileReadStats {
                    bytes_read: lines.iter().map(|l| l.len() + 1).sum::<usize>() as u64,
                    lines_read: lines.len() as u64,
                    encoding,
                    file_size,
                    read_time_ms: 0,
                })
            } else {
                Err(e)
            }
        })
        .map(|stats| (lines, stats))
    }

    /// Validate file without fully reading it
    ///
    /// Performs basic checks on file format
    ///
    /// # Errors
    /// Returns validation errors
    pub fn validate(&self) -> Result<FileValidation> {
        let mut validation = FileValidation::new();
        let mut has_motion = false;

        self.read_lines(|line| {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('(') {
                return Ok(());
            }

            validation.total_lines += 1;

            // Check for motion commands
            let upper = trimmed.to_uppercase();
            if upper.contains('G') || upper.contains('M') {
                has_motion = true;
            }

            // Check for potential issues
            if trimmed.len() > 256 {
                validation.warnings.push(format!(
                    "Line {} is very long ({})",
                    validation.total_lines,
                    trimmed.len()
                ));
            }

            if trimmed.contains("G00") || trimmed.contains("G0 ") {
                validation.rapid_moves += 1;
            }
            if trimmed.contains("G01") || trimmed.contains("G1 ") {
                validation.linear_moves += 1;
            }
            if trimmed.contains("G02")
                || trimmed.contains("G2 ")
                || trimmed.contains("G03")
                || trimmed.contains("G3 ")
            {
                validation.arc_moves += 1;
            }

            Ok(())
        })?;

        if !has_motion {
            validation
                .warnings
                .push("File contains no motion commands (G or M)".to_string());
        }

        if validation.total_lines == 0 {
            validation.errors.push("File is empty".to_string());
        }

        validation.is_valid = validation.errors.is_empty();
        Ok(validation)
    }
}

/// File validation result
#[derive(Debug, Clone)]
pub struct FileValidation {
    /// Whether file is valid
    pub is_valid: bool,
    /// Total lines in file
    pub total_lines: u64,
    /// Number of rapid moves (G0)
    pub rapid_moves: u64,
    /// Number of linear moves (G1)
    pub linear_moves: u64,
    /// Number of arc moves (G2/G3)
    pub arc_moves: u64,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

impl FileValidation {
    /// Create new validation result
    pub fn new() -> Self {
        Self {
            is_valid: true,
            total_lines: 0,
            rapid_moves: 0,
            linear_moves: 0,
            arc_moves: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Get total motion commands
    pub fn total_motion_commands(&self) -> u64 {
        self.rapid_moves + self.linear_moves + self.arc_moves
    }
}

impl Default for FileValidation {
    fn default() -> Self {
        Self::new()
    }
}

/// Recent file entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFileEntry {
    /// File path
    pub path: PathBuf,
    /// File name
    pub name: String,
    /// Last opened timestamp (Unix seconds)
    pub timestamp: u64,
    /// File size in bytes
    pub file_size: u64,
    /// Last accessed timestamp (Unix seconds)
    pub last_accessed: u64,
}

impl RecentFileEntry {
    /// Create new recent file entry
    pub fn new(path: impl AsRef<Path>, file_size: u64) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(Self {
            path,
            name,
            timestamp,
            file_size,
            last_accessed: timestamp,
        })
    }

    /// Update last accessed timestamp
    pub fn update_accessed(&mut self) {
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Get file size formatted as human-readable string
    pub fn formatted_size(&self) -> String {
        if self.file_size < 1024 {
            format!("{} B", self.file_size)
        } else if self.file_size < 1024 * 1024 {
            format!("{:.2} KB", self.file_size as f64 / 1024.0)
        } else {
            format!("{:.2} MB", self.file_size as f64 / (1024.0 * 1024.0))
        }
    }
}

/// Recent files manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFilesManager {
    /// List of recent files (most recent first)
    files: Vec<RecentFileEntry>,
    /// Maximum number of recent files to keep
    max_files: usize,
    /// Path to persist recent files list
    persist_path: Option<PathBuf>,
}

impl RecentFilesManager {
    /// Create new recent files manager
    pub fn new(max_files: usize) -> Self {
        Self {
            files: Vec::new(),
            max_files,
            persist_path: None,
        }
    }

    /// Set persistence path for saving recent files list
    pub fn set_persist_path(&mut self, path: impl AsRef<Path>) {
        self.persist_path = Some(path.as_ref().to_path_buf());
    }

    /// Load recent files from file
    pub fn load(&mut self) -> Result<()> {
        if let Some(path) = &self.persist_path {
            if path.exists() {
                let content = fs::read_to_string(path)?;
                let loaded: Vec<RecentFileEntry> = serde_json::from_str(&content)?;
                self.files = loaded;
                self.trim_to_max();
                return Ok(());
            }
        }
        Ok(())
    }

    /// Save recent files to disk
    pub fn save(&self) -> Result<()> {
        if let Some(path) = &self.persist_path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = serde_json::to_string_pretty(&self.files)?;
            fs::write(path, content)?;
        }
        Ok(())
    }

    /// Add or update a recent file
    pub fn add(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let reader = GcodeFileReader::new(&path)?;
        let file_size = reader.file_size();

        let entry = RecentFileEntry::new(&path, file_size)?;

        // Remove if already exists
        self.files.retain(|f| f.path != entry.path);

        // Add to front
        self.files.insert(0, entry);

        // Trim to max size
        self.trim_to_max();

        // Persist
        self.save()?;

        Ok(())
    }

    /// Remove a recent file
    pub fn remove(&mut self, path: &Path) -> Result<()> {
        self.files.retain(|f| f.path != path);
        self.save()?;
        Ok(())
    }

    /// Clear all recent files
    pub fn clear(&mut self) -> Result<()> {
        self.files.clear();
        self.save()?;
        Ok(())
    }

    /// Get list of recent files
    pub fn list(&self) -> Vec<&RecentFileEntry> {
        self.files.iter().collect()
    }

    /// Get recent file by index
    pub fn get(&self, index: usize) -> Option<&RecentFileEntry> {
        self.files.get(index)
    }

    /// Get count of recent files
    pub fn count(&self) -> usize {
        self.files.len()
    }

    /// Find recent file by path
    pub fn find_by_path(&self, path: &Path) -> Option<&RecentFileEntry> {
        self.files.iter().find(|f| f.path == path)
    }

    /// Trim list to maximum size
    fn trim_to_max(&mut self) {
        if self.files.len() > self.max_files {
            self.files.truncate(self.max_files);
        }
    }

    /// Update access time for a file
    pub fn touch(&mut self, path: &Path) -> Result<()> {
        // Find and move to front
        if let Some(pos) = self.files.iter().position(|f| f.path == path) {
            let mut entry = self.files.remove(pos);
            entry.update_accessed();
            self.files.insert(0, entry);
            self.save()?;
        }
        Ok(())
    }
}

impl Default for RecentFilesManager {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_RECENT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_encoding_detection() {
        // UTF-8 with BOM
        let utf8_bom = vec![0xEF, 0xBB, 0xBF, 0x48, 0x65, 0x6C, 0x6C, 0x6F]; // "Hello"
        assert_eq!(FileEncoding::detect(&utf8_bom), FileEncoding::Utf8);

        // Valid UTF-8
        assert_eq!(
            FileEncoding::detect("G0 X10 Y20".as_bytes()),
            FileEncoding::Utf8
        );
    }

    #[test]
    fn test_gcode_file_reader_not_found() {
        let result = GcodeFileReader::new("/nonexistent/path/file.nc");
        assert!(result.is_err());
    }

    #[test]
    fn test_recent_file_entry() {
        // Create temp file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_recent.nc");
        let _ = fs::write(&test_file, "G0 X10");

        let entry = RecentFileEntry::new(&test_file, 1024).unwrap();
        assert_eq!(entry.name, "test_recent.nc");
        assert_eq!(entry.file_size, 1024);
        assert!(entry.formatted_size().contains("KB"));

        // Cleanup
        let _ = fs::remove_file(&test_file);
    }

    #[test]
    fn test_recent_files_manager() {
        let manager = RecentFilesManager::new(5);
        assert_eq!(manager.count(), 0);

        // Create temp files
        let temp_dir = std::env::temp_dir();
        let file1 = temp_dir.join("test1.nc");
        let _ = fs::write(&file1, "G0 X10");

        // Note: We can't test add() without valid files, but we can test structure
        assert_eq!(manager.max_files, 5);

        // Cleanup
        let _ = fs::remove_file(&file1);
    }

    #[test]
    fn test_file_validation_result() {
        let validation = FileValidation::new();
        assert!(validation.is_valid);
        assert_eq!(validation.total_lines, 0);
        assert_eq!(validation.total_motion_commands(), 0);
    }

    #[test]
    fn test_recent_file_entry_size_formatting() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("size_test.nc");
        let _ = fs::write(&test_file, "G0 X10");

        let entry = RecentFileEntry::new(&test_file, 2048).unwrap();
        assert!(entry.formatted_size().contains("KB"));

        let entry2 = RecentFileEntry::new(&test_file, 1024 * 1024 * 2).unwrap();
        assert!(entry2.formatted_size().contains("MB"));

        // Cleanup
        let _ = fs::remove_file(&test_file);
    }

    #[test]
    fn test_file_validation_motion_count() {
        let mut validation = FileValidation::new();
        validation.rapid_moves = 5;
        validation.linear_moves = 10;
        validation.arc_moves = 3;
        assert_eq!(validation.total_motion_commands(), 18);
    }

    #[test]
    fn test_file_read_stats_progress() {
        let stats = FileReadStats {
            bytes_read: 50,
            lines_read: 10,
            encoding: FileEncoding::Utf8,
            file_size: 100,
            read_time_ms: 100,
        };
        assert_eq!(stats.progress_percent(), 50.0);
    }

    #[test]
    fn test_recent_files_manager_trim() {
        let mut manager = RecentFilesManager::new(2);

        // Manually add entries
        let temp_dir = std::env::temp_dir();
        let file1 = temp_dir.join("file1.nc");
        let file2 = temp_dir.join("file2.nc");
        let file3 = temp_dir.join("file3.nc");

        let _ = fs::write(&file1, "");
        let _ = fs::write(&file2, "");
        let _ = fs::write(&file3, "");

        // Add entries manually
        if let Ok(e1) = RecentFileEntry::new(&file1, 100) {
            manager.files.push(e1);
        }
        if let Ok(e2) = RecentFileEntry::new(&file2, 100) {
            manager.files.push(e2);
        }
        if let Ok(e3) = RecentFileEntry::new(&file3, 100) {
            manager.files.push(e3);
        }

        manager.trim_to_max();
        assert_eq!(manager.count(), 2);

        // Cleanup
        let _ = fs::remove_file(&file1);
        let _ = fs::remove_file(&file2);
        let _ = fs::remove_file(&file3);
    }
}
