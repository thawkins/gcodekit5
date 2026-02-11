//! Text buffer implementation using rope data structure for efficient text manipulation

use ropey::Rope;
use std::fmt;
use std::ops::Range;

/// Efficient text buffer using rope data structure
/// Optimized for large files with incremental updates
#[derive(Clone)]
pub struct TextBuffer {
    rope: Rope,
    dirty_lines: Vec<usize>,
}

impl TextBuffer {
    /// Create a new empty text buffer
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            dirty_lines: Vec::new(),
        }
    }

    /// Get the total length in bytes
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Get the total length in chars
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Get the total number of lines
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.rope.len_bytes() == 0
    }

    /// Get a slice of text as a String
    pub fn slice(&self, start: usize, end: usize) -> String {
        self.rope.slice(start..end).to_string()
    }

    /// Get a line of text
    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx < self.len_lines() {
            Some(self.rope.line(line_idx).to_string())
        } else {
            None
        }
    }

    /// Get lines in a range for viewport rendering
    pub fn lines_in_range(&self, range: Range<usize>) -> Vec<String> {
        let start = range.start.min(self.len_lines());
        let end = range.end.min(self.len_lines());

        (start..end)
            .map(|idx| self.rope.line(idx).to_string())
            .collect()
    }

    /// Insert text at character position
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        let char_idx = char_idx.min(self.len_chars());
        self.rope.insert(char_idx, text);
        self.mark_dirty_at_char(char_idx);
    }

    /// Delete text in character range
    pub fn delete(&mut self, char_range: Range<usize>) {
        let start = char_range.start.min(self.len_chars());
        let end = char_range.end.min(self.len_chars());

        if start < end {
            self.rope.remove(start..end);
            self.mark_dirty_at_char(start);
        }
    }

    /// Replace text in character range
    pub fn replace(&mut self, char_range: Range<usize>, text: &str) {
        self.delete(char_range.clone());
        self.insert(char_range.start, text);
    }

    /// Append text to end (optimized for streaming)
    pub fn append(&mut self, text: &str) {
        let len = self.len_chars();
        self.rope.insert(len, text);
        self.mark_dirty_at_char(len);
    }

    /// Clear all text
    pub fn clear(&mut self) {
        self.rope = Rope::new();
        self.dirty_lines.clear();
    }

    /// Convert char index to line/column
    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let char_idx = char_idx.min(self.len_chars());
        let line_idx = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line_idx);
        let col = char_idx - line_start;
        (line_idx, col)
    }

    /// Convert line/column to char index
    pub fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        let line = line.min(self.len_lines().saturating_sub(1));
        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();
        line_start + col.min(line_len)
    }

    /// Mark lines as dirty for incremental rendering
    fn mark_dirty_at_char(&mut self, char_idx: usize) {
        if char_idx < self.len_chars() {
            let line_idx = self.rope.char_to_line(char_idx);
            if !self.dirty_lines.contains(&line_idx) {
                self.dirty_lines.push(line_idx);
            }
        }
    }

    /// Get and clear dirty lines
    pub fn take_dirty_lines(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.dirty_lines)
    }

    /// Check if a line is dirty
    pub fn is_line_dirty(&self, line_idx: usize) -> bool {
        self.dirty_lines.contains(&line_idx)
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&str> for TextBuffer {
    fn from(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            dirty_lines: Vec::new(),
        }
    }
}

impl fmt::Display for TextBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rope)
    }
}
