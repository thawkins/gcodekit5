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

use crate::gcode::words::{code_number, parse_line};

/// Maximum number of recent files to keep in history
const DEFAULT_MAX_RECENT: usize = 20;

/// Buffer size for reading large files (256 KB)
const READ_BUFFER_SIZE: usize = 256 * 1024;

/// Supported file encodings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEncoding {
    /// UTF-8 encoding
    Utf8,
    /// ASCII encoding (7-bit). Valid ASCII is also valid UTF-8, so
    /// [`FileEncoding::detect`] reports it as `Utf8`; kept for compatibility.
    Ascii,
    /// Single-byte Latin-1 / ISO-8859-1, used for data that is not valid UTF-8
    /// (e.g. comments written by legacy Windows CAM tools).
    Latin1,
}

impl FileEncoding {
    /// Detect encoding from file bytes
    ///
    /// # Arguments
    /// * `data` - Raw bytes (a line or a chunk of the file).
    ///
    /// # Returns
    /// `Utf8` for a UTF-8 BOM or valid UTF-8, otherwise `Latin1`.
    pub fn detect(data: &[u8]) -> Self {
        // Check for UTF-8 BOM
        if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return FileEncoding::Utf8;
        }

        // Try to validate as UTF-8
        if std::str::from_utf8(data).is_ok() {
            return FileEncoding::Utf8;
        }

        // Not UTF-8: decode as a single-byte encoding, which never fails
        FileEncoding::Latin1
    }

    /// Decode raw bytes into UTF-8 text using this encoding.
    ///
    /// `Utf8`/`Ascii` decode lossily (invalid sequences become U+FFFD);
    /// `Latin1` maps every byte to the code point of the same value.
    ///
    /// # Arguments
    /// * `data` - Raw bytes to decode.
    ///
    /// # Returns
    /// The decoded text.
    pub fn decode(self, data: &[u8]) -> String {
        match self {
            FileEncoding::Utf8 | FileEncoding::Ascii => String::from_utf8_lossy(data).into_owned(),
            FileEncoding::Latin1 => data.iter().map(|&b| b as char).collect(),
        }
    }
}

/// Decode a whole file's bytes, detecting the encoding.
///
/// A leading UTF-8 BOM is removed. Valid UTF-8 is returned as-is; anything
/// else is decoded as Latin-1 so no byte is lost and decoding never fails.
///
/// # Arguments
/// * `data` - Raw file bytes.
///
/// # Returns
/// `(text, detected_encoding)`.
pub fn decode_bytes(data: &[u8]) -> (String, FileEncoding) {
    let body = data.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(data);
    let encoding = FileEncoding::detect(body);
    (encoding.decode(body), encoding)
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

        let bytes = fs::read(&self.path).map_err(|e| anyhow!("Failed to read file: {}", e))?;
        let (text, encoding) = decode_bytes(&bytes);
        if encoding != FileEncoding::Utf8 {
            tracing::debug!(path = %self.path.display(), ?encoding, "decoded non-UTF-8 file");
        }
        Ok(text)
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
        let mut reader = BufReader::with_capacity(READ_BUFFER_SIZE, file);

        let mut lines_read = 0u64;
        let mut bytes_read = 0u64;
        // Becomes `Latin1` (and stays so) once a line is found that is not
        // valid UTF-8; every later line is then decoded the same way.
        let mut encoding = FileEncoding::Utf8;
        let mut buf: Vec<u8> = Vec::new();

        loop {
            buf.clear();
            let n = reader.read_until(b'\n', &mut buf)?;
            if n == 0 {
                break;
            }
            bytes_read += n as u64;

            let mut raw: &[u8] = &buf;
            if lines_read == 0 {
                raw = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
            }
            raw = raw.strip_suffix(b"\n").unwrap_or(raw);
            raw = raw.strip_suffix(b"\r").unwrap_or(raw);

            if encoding == FileEncoding::Utf8 && std::str::from_utf8(raw).is_err() {
                encoding = FileEncoding::Latin1;
            }
            let line = encoding.decode(raw);

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

    /// Detect the encoding of the file from its first 64 KiB.
    ///
    /// # Errors
    /// Returns error if the file cannot be read
    pub fn detect_encoding(&self) -> Result<FileEncoding> {
        use std::io::Read;
        let mut head = Vec::new();
        File::open(&self.path)?
            .take(64 * 1024)
            .read_to_end(&mut head)?;
        // A chunk boundary may split a UTF-8 sequence; ignore an incomplete tail.
        let valid = match std::str::from_utf8(&head) {
            Ok(_) => true,
            Err(e) => e.error_len().is_none(),
        };
        Ok(if valid {
            FileEncoding::Utf8
        } else {
            FileEncoding::Latin1
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
        let encoding = self.detect_encoding()?;
        let file_size = self.file_size;

        self.read_lines(|line| {
            if lines.len() < max_lines {
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

            let words = parse_line(trimmed);

            // Check for motion commands
            if words.iter().any(|w| w.letter == 'G' || w.letter == 'M') {
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

            // Count explicit motion words (exact G0/G1/G2/G3, so G10, G21, G91 etc.
            // are not mistaken for motion commands)
            for word in words.iter().filter(|w| w.letter == 'G') {
                match code_number(word.value) {
                    (0, 0) => validation.rapid_moves += 1,
                    (1, 0) => validation.linear_moves += 1,
                    (2, 0) | (3, 0) => validation.arc_moves += 1,
                    _ => {}
                }
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
    use tempfile::TempDir;

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
        let tmp = TempDir::new().expect("create temp dir");
        let test_file = tmp.path().join("test_recent.nc");
        let _ = fs::write(&test_file, "G0 X10");

        let entry = RecentFileEntry::new(&test_file, 1024).expect("test entry");
        assert_eq!(entry.name, "test_recent.nc");
        assert_eq!(entry.file_size, 1024);
        assert!(entry.formatted_size().contains("KB"));
    }

    #[test]
    fn test_recent_files_manager() {
        let manager = RecentFilesManager::new(5);
        assert_eq!(manager.count(), 0);
        assert_eq!(manager.max_files, 5);
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
        let tmp = TempDir::new().expect("create temp dir");
        let test_file = tmp.path().join("size_test.nc");
        let _ = fs::write(&test_file, "G0 X10");

        let entry = RecentFileEntry::new(&test_file, 2048).expect("test entry");
        assert!(entry.formatted_size().contains("KB"));

        let entry2 = RecentFileEntry::new(&test_file, 1024 * 1024 * 2).expect("test entry");
        assert!(entry2.formatted_size().contains("MB"));
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

        let tmp = TempDir::new().expect("create temp dir");
        let file1 = tmp.path().join("file1.nc");
        let file2 = tmp.path().join("file2.nc");
        let file3 = tmp.path().join("file3.nc");

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
    }
}
