//! G-Code stream management for reading and tracking position
//!
//! This module provides:
//! - Stream reader trait for different input sources
//! - File-based stream reader for reading from disk
//! - String-based stream reader for in-memory G-Code
//! - Stream position tracking and pause/resume capabilities

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Trait for reading G-Code streams from various sources
pub trait GcodeStreamReader: Send + Sync {
    /// Read the next line from the stream
    ///
    /// Returns `Ok(Some(line))` if a line is available and `Ok(None)` at the
    /// end of the stream (or while a [`PausableStream`] is paused).
    ///
    /// # Errors
    /// Returns the underlying I/O error if the source cannot be read; a read
    /// error is never reported as end of stream.
    fn read_line(&mut self) -> std::io::Result<Option<String>>;

    /// Get the current line number (0-indexed)
    fn current_line_number(&self) -> usize;

    /// Get the total number of lines (if known)
    fn total_lines(&self) -> Option<usize>;

    /// Reset stream to beginning
    fn reset(&mut self) -> std::io::Result<()>;

    /// Seek to a specific line number
    fn seek_to_line(&mut self, line_number: usize) -> std::io::Result<()>;

    /// Check if stream is at end
    fn is_eof(&self) -> bool;
}

/// File-based G-Code stream reader
///
/// Reads G-Code from a file on disk, supporting position tracking
/// and pause/resume functionality.
pub struct FileStreamReader {
    reader: BufReader<File>,
    file_path: std::path::PathBuf,
    current_line: usize,
    total_lines: Option<usize>,
    is_eof: bool,
}

impl FileStreamReader {
    /// Create a new file stream reader
    ///
    /// # Arguments
    /// * `path` - Path to the G-Code file
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(&path)?;
        let reader = BufReader::new(file);

        // Count total lines (byte-based, so non-UTF-8 content is counted too).
        // A failure here only makes the total unknown; the error itself is
        // reported by `read_line` when the content is actually read.
        let file_for_count = File::open(&path)?;
        let count_reader = BufReader::new(file_for_count);
        let total_lines = count_reader
            .split(b'\n')
            .try_fold(0usize, |n, line| line.map(|_| n + 1))
            .ok();

        Ok(Self {
            reader,
            file_path: path.as_ref().to_path_buf(),
            current_line: 0,
            total_lines,
            is_eof: false,
        })
    }

    /// Get the file path
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Get the progress as a percentage (0-100)
    pub fn progress_percent(&self) -> f64 {
        if let Some(total) = self.total_lines {
            if total == 0 {
                100.0
            } else {
                (self.current_line as f64 / total as f64) * 100.0
            }
        } else {
            0.0
        }
    }
}

impl GcodeStreamReader for FileStreamReader {
    fn read_line(&mut self) -> std::io::Result<Option<String>> {
        if self.is_eof {
            return Ok(None);
        }

        // Read raw bytes so that a non-UTF-8 byte (e.g. a Latin-1 comment) is
        // decoded lossily instead of failing; genuine I/O errors propagate and
        // leave the reader positioned where it was.
        let mut buf = Vec::new();
        match self.reader.read_until(b'\n', &mut buf)? {
            0 => {
                self.is_eof = true;
                Ok(None)
            }
            _ => {
                self.current_line += 1;
                Ok(Some(String::from_utf8_lossy(&buf).into_owned()))
            }
        }
    }

    fn current_line_number(&self) -> usize {
        self.current_line
    }

    fn total_lines(&self) -> Option<usize> {
        self.total_lines
    }

    fn reset(&mut self) -> std::io::Result<()> {
        let file = File::open(&self.file_path)?;
        self.reader = BufReader::new(file);
        self.current_line = 0;
        self.is_eof = false;
        Ok(())
    }

    fn seek_to_line(&mut self, line_number: usize) -> std::io::Result<()> {
        self.reset()?;
        let mut current = 0;
        let mut line = Vec::new();

        while current < line_number && self.reader.read_until(b'\n', &mut line)? > 0 {
            current += 1;
            line.clear();
        }

        self.current_line = current;
        // EOF only if the file ended before the requested line was reached
        self.is_eof = current < line_number;
        Ok(())
    }

    fn is_eof(&self) -> bool {
        self.is_eof
    }
}

/// String-based G-Code stream reader
///
/// Reads G-Code from an in-memory string, supporting position tracking
/// and pause/resume functionality.
pub struct StringStreamReader {
    lines: Vec<String>,
    current_index: usize,
}

impl StringStreamReader {
    /// Create a new string stream reader
    ///
    /// # Arguments
    /// * `content` - The G-Code content as a string
    pub fn new(content: &str) -> Self {
        let lines = content
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();

        let _total = lines.len();

        Self {
            lines,
            current_index: 0,
        }
    }

    /// Get the progress as a percentage (0-100)
    pub fn progress_percent(&self) -> f64 {
        if self.lines.is_empty() {
            100.0
        } else {
            (self.current_index as f64 / self.lines.len() as f64) * 100.0
        }
    }
}

impl GcodeStreamReader for StringStreamReader {
    fn read_line(&mut self) -> std::io::Result<Option<String>> {
        if self.current_index < self.lines.len() {
            let line = self.lines[self.current_index].clone();
            self.current_index += 1;
            Ok(Some(line))
        } else {
            Ok(None)
        }
    }

    fn current_line_number(&self) -> usize {
        self.current_index
    }

    fn total_lines(&self) -> Option<usize> {
        Some(self.lines.len())
    }

    fn reset(&mut self) -> std::io::Result<()> {
        self.current_index = 0;
        Ok(())
    }

    fn seek_to_line(&mut self, line_number: usize) -> std::io::Result<()> {
        if line_number <= self.lines.len() {
            self.current_index = line_number;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Line number {} exceeds total lines {}",
                    line_number,
                    self.lines.len()
                ),
            ))
        }
    }

    fn is_eof(&self) -> bool {
        self.current_index >= self.lines.len()
    }
}

/// Pausable G-Code stream wrapper
///
/// Wraps any stream reader and adds pause/resume functionality.
pub struct PausableStream {
    inner: Box<dyn GcodeStreamReader>,
    is_paused: Arc<AtomicUsize>, // 0 = running, 1 = paused
    pause_line: Arc<AtomicUsize>,
}

impl PausableStream {
    /// Create a new pausable stream wrapper
    pub fn new(inner: Box<dyn GcodeStreamReader>) -> Self {
        Self {
            inner,
            is_paused: Arc::new(AtomicUsize::new(0)),
            pause_line: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Pause the stream
    pub fn pause(&self) {
        self.is_paused.store(1, Ordering::SeqCst);
        let line = self.inner.current_line_number();
        self.pause_line.store(line, Ordering::SeqCst);
    }

    /// Resume the stream
    pub fn resume(&self) {
        self.is_paused.store(0, Ordering::SeqCst);
    }

    /// Check if stream is paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst) == 1
    }

    /// Get the pause line number
    pub fn pause_line(&self) -> usize {
        self.pause_line.load(Ordering::SeqCst)
    }

    /// Get current line number
    pub fn current_line(&self) -> usize {
        self.inner.current_line_number()
    }

    /// Get total lines if known
    pub fn total_lines(&self) -> Option<usize> {
        self.inner.total_lines()
    }

    /// Check if at end of stream
    pub fn is_eof(&self) -> bool {
        self.inner.is_eof()
    }
}

impl GcodeStreamReader for PausableStream {
    fn read_line(&mut self) -> std::io::Result<Option<String>> {
        // Don't read while paused
        if self.is_paused.load(Ordering::SeqCst) == 1 {
            return Ok(None);
        }
        self.inner.read_line()
    }

    fn current_line_number(&self) -> usize {
        self.inner.current_line_number()
    }

    fn total_lines(&self) -> Option<usize> {
        self.inner.total_lines()
    }

    fn reset(&mut self) -> std::io::Result<()> {
        self.is_paused.store(0, Ordering::SeqCst);
        self.pause_line.store(0, Ordering::SeqCst);
        self.inner.reset()
    }

    fn seek_to_line(&mut self, line_number: usize) -> std::io::Result<()> {
        self.is_paused.store(0, Ordering::SeqCst);
        self.pause_line.store(0, Ordering::SeqCst);
        self.inner.seek_to_line(line_number)
    }

    fn is_eof(&self) -> bool {
        self.inner.is_eof()
    }
}
