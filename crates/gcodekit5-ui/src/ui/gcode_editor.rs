//! G-Code Editor Module
//!
//! Provides a full-featured G-Code editor with:
//! - Syntax highlighting for G-Code commands
//! - Line number support
//! - Current line tracking during execution
//! - File management (open, save, save as)
//! - Search and replace functionality
//! - Real-time validation
//!
//! This module merges functionality from both gcode_editor and gcode_viewer
//! into a unified, comprehensive editor implementation.
//!
//! File operations use the `rfd` crate for cross-platform file dialogs.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use gcodekit5_core::{thread_safe, ThreadSafe};

/// Token types for G-Code syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    /// G-Code command (G00, G01, etc.)
    GCommand,
    /// M-Code command (M03, M04, etc.)
    MCommand,
    /// Coordinate (X, Y, Z, A, B, C)
    Coordinate,
    /// Parameter (F, S, T, etc.)
    Parameter,
    /// Comment
    Comment,
    /// Whitespace/normal text
    Normal,
}

/// A token in the G-Code file
#[derive(Debug, Clone)]
pub struct Token {
    /// Token type
    pub token_type: TokenType,
    /// Token text
    pub text: String,
    /// Starting position in line
    pub start: usize,
    /// Ending position in line
    pub end: usize,
}

/// Represents a single line in the G-Code file
#[derive(Debug, Clone)]
pub struct GcodeLine {
    /// Line number (1-indexed)
    pub line_number: usize,
    /// Raw text of the line
    pub text: String,
    /// Parsed tokens
    pub tokens: Vec<Token>,
    /// Whether this line has been executed
    pub executed: bool,
    /// Whether this is the current line
    pub is_current: bool,
}

impl GcodeLine {
    /// Create a new G-Code line
    pub fn new(line_number: usize, text: String) -> Self {
        let tokens = Self::tokenize(&text);
        Self {
            line_number,
            text,
            tokens,
            executed: false,
            is_current: false,
        }
    }

    /// Tokenize the line for syntax highlighting
    fn tokenize(text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut start_pos = 0;
        let mut in_comment = false;

        for (i, ch) in text.chars().enumerate() {
            if in_comment {
                current_token.push(ch);
            } else if ch == ';' {
                // Start of comment
                if !current_token.is_empty() {
                    tokens.push(Token {
                        token_type: Self::classify_token(&current_token),
                        text: current_token.clone(),
                        start: start_pos,
                        end: i,
                    });
                    current_token.clear();
                }
                in_comment = true;
                start_pos = i;
                current_token.push(ch);
            } else if ch.is_whitespace() {
                if !current_token.is_empty() {
                    tokens.push(Token {
                        token_type: Self::classify_token(&current_token),
                        text: current_token.clone(),
                        start: start_pos,
                        end: i,
                    });
                    current_token.clear();
                }
                start_pos = i + 1;
            } else {
                if current_token.is_empty() {
                    start_pos = i;
                }
                current_token.push(ch);
            }
        }

        if !current_token.is_empty() {
            tokens.push(Token {
                token_type: if in_comment {
                    TokenType::Comment
                } else {
                    Self::classify_token(&current_token)
                },
                text: current_token,
                start: start_pos,
                end: text.len(),
            });
        }

        tokens
    }

    /// Classify a token based on its content
    fn classify_token(text: &str) -> TokenType {
        if text.is_empty() {
            return TokenType::Normal;
        }

        let upper = text.to_uppercase();

        if upper.starts_with('G') {
            TokenType::GCommand
        } else if upper.starts_with('M') {
            TokenType::MCommand
        } else if matches!(
            upper.chars().next(),
            Some('X' | 'Y' | 'Z' | 'A' | 'B' | 'C')
        ) {
            TokenType::Coordinate
        } else if matches!(upper.chars().next(), Some('F' | 'S' | 'T' | 'H' | 'P')) {
            TokenType::Parameter
        } else if text.starts_with(';') {
            TokenType::Comment
        } else {
            TokenType::Normal
        }
    }
}

/// G-Code file content manager
pub struct GcodeFile {
    /// File path
    pub path: Option<String>,
    /// All lines in the file
    lines: Vec<GcodeLine>,
    /// Current line number (0-indexed)
    current_line: usize,
}

impl GcodeFile {
    /// Create a new empty G-Code file
    pub fn new() -> Self {
        Self {
            path: None,
            lines: Vec::new(),
            current_line: 0,
        }
    }

    /// Load content from a string
    pub fn load_content(&mut self, content: &str) {
        self.lines.clear();
        for (idx, line) in content.lines().enumerate() {
            self.lines.push(GcodeLine::new(idx + 1, line.to_string()));
        }
    }

    /// Get all lines
    pub fn get_lines(&self) -> &[GcodeLine] {
        &self.lines
    }

    /// Get a mutable reference to all lines
    pub fn get_lines_mut(&mut self) -> &mut [GcodeLine] {
        &mut self.lines
    }

    /// Get a specific line
    pub fn get_line(&self, line_number: usize) -> Option<&GcodeLine> {
        if line_number > 0 && line_number <= self.lines.len() {
            Some(&self.lines[line_number - 1])
        } else {
            None
        }
    }

    /// Mark a line as executed
    pub fn mark_executed(&mut self, line_number: usize) {
        if line_number > 0 && line_number <= self.lines.len() {
            self.lines[line_number - 1].executed = true;
        }
    }

    /// Set the current line
    pub fn set_current_line(&mut self, line_number: usize) {
        // Clear previous current line
        if self.current_line > 0 && self.current_line <= self.lines.len() {
            self.lines[self.current_line - 1].is_current = false;
        }

        if line_number > 0 && line_number <= self.lines.len() {
            self.lines[line_number - 1].is_current = true;
            self.current_line = line_number;
        }
    }

    /// Get the current line number
    pub fn get_current_line(&self) -> usize {
        self.current_line
    }

    /// Clear all execution state
    pub fn clear_execution_state(&mut self) {
        for line in &mut self.lines {
            line.executed = false;
            line.is_current = false;
        }
        self.current_line = 0;
    }

    /// Get content as a formatted string with line numbers and styling
    pub fn get_formatted_content(&self) -> String {
        let mut output = String::new();
        let max_line_num_width = self.lines.len().to_string().len();

        for line in &self.lines {
            // Format line number
            output.push_str(&format!(
                "{:>width$} | ",
                line.line_number,
                width = max_line_num_width
            ));

            // Add execution markers
            if line.is_current {
                output.push_str("▶ ");
            } else if line.executed {
                output.push_str("✓ ");
            } else {
                output.push_str("  ");
            }

            output.push_str(&line.text);
            output.push('\n');
        }

        output
    }

    /// Get plain text content (without line numbers or markers)
    pub fn get_plain_content(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for GcodeFile {
    fn default() -> Self {
        Self::new()
    }
}

/// G-Code Editor Manager
pub struct GcodeEditor {
    /// Current file being edited
    file: ThreadSafe<GcodeFile>,
    /// Whether the editor is editable
    editable: ThreadSafe<bool>,
    // ---
    /// Sender to Visualizer
    toolpath_sender: mpsc::Sender<Vec<Toolpath>>,
    // ---
}

impl GcodeEditor {
    /// Create a new G-Code editor
    pub fn new() -> Self {
        Self {
            file: thread_safe(GcodeFile::new()),
            editable: thread_safe(true),
        }
    }

    /// Acquire file lock with poisoned lock recovery
    fn lock_file(&self) -> std::sync::MutexGuard<'_, GcodeFile> {
        self.file
            .lock()
    }

    /// Acquire editable lock with poisoned lock recovery
    fn lock_editable(&self) -> std::sync::MutexGuard<'_, bool> {
        self.editable
            .lock()
    }

    /// Load a file from content
    pub fn load_content(&self, content: &str) -> anyhow::Result<()> {
        let mut file = self.lock_file();
        file.load_content(content);
        Ok(())
    }

    /// Get formatted content with line numbers and execution state
    pub fn get_display_content(&self) -> String {
        let file = self.lock_file();
        file.get_formatted_content()
    }

    /// Get plain content without formatting
    pub fn get_plain_content(&self) -> String {
        let file = self.lock_file();
        file.get_plain_content()
    }

    /// Mark a line as executed
    pub fn mark_line_executed(&self, line_number: usize) {
        let mut file = self.lock_file();
        file.mark_executed(line_number);
    }

    /// Set the current executing line
    pub fn set_current_line(&self, line_number: usize) {
        let mut file = self.lock_file();
        file.set_current_line(line_number);
    }

    /// Get the current line number
    pub fn get_current_line(&self) -> usize {
        let file = self.lock_file();
        file.get_current_line()
    }

    /// Clear all execution state
    pub fn clear_execution_state(&self) {
        let mut file = self.lock_file();
        file.clear_execution_state();
    }

    /// Get line count
    pub fn get_line_count(&self) -> usize {
        let file = self.lock_file();
        file.lines.len()
    }

    /// Set whether the editor is editable
    pub fn set_editable(&self, editable: bool) {
        *self.lock_editable() = editable;
    }

    /// Check if the editor is editable
    pub fn is_editable(&self) -> bool {
        *self.lock_editable()
    }

    /// Search for text in all lines
    pub fn search(&self, query: &str) -> Vec<(usize, usize)> {
        let file = self.lock_file();
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for line in &file.lines {
            let text_lower = line.text.to_lowercase();
            let mut pos = 0;
            while let Some(index) = text_lower[pos..].find(&query_lower) {
                let actual_pos = pos + index;
                results.push((line.line_number, actual_pos));
                pos = actual_pos + 1;
            }
        }

        results
    }

    /// Replace all occurrences of text
    pub fn replace_all(&self, old: &str, new: &str) -> usize {
        let mut file = self.lock_file();
        let mut count = 0;

        for line in &mut file.lines {
            let before = line.text.clone();
            line.text = line.text.replace(old, new);
            if line.text != before {
                count += 1;
                line.tokens = GcodeLine::tokenize(&line.text);
            }
        }

        count
    }

    /// Replace occurrence at specific line and position
    pub fn replace_at(&self, line_number: usize, position: usize, old: &str, new: &str) -> bool {
        let mut file = self.lock_file();

        if line_number > 0 && line_number <= file.lines.len() {
            let line = &mut file.lines[line_number - 1];
            let end = position + old.len();

            if end <= line.text.len() && &line.text[position..end] == old {
                let mut new_text = line.text.clone();
                new_text.replace_range(position..end, new);
                line.text = new_text;
                line.tokens = GcodeLine::tokenize(&line.text);
                return true;
            }
        }
        false
    }

    /// Insert text at specific position
    pub fn insert_text(&self, line_number: usize, position: usize, text: &str) -> bool {
        if !self.is_editable() {
            return false;
        }

        let mut file = self.lock_file();

        if line_number > 0 && line_number <= file.lines.len() {
            let line = &mut file.lines[line_number - 1];
            if position <= line.text.len() {
                line.text.insert_str(position, text);
                line.tokens = GcodeLine::tokenize(&line.text);
                return true;
            }
        }
        false
    }

    /// Delete character at specific position
    pub fn delete_char(&self, line_number: usize, position: usize) -> bool {
        if !self.is_editable() {
            return false;
        }

        let mut file = self.lock_file();

        if line_number > 0 && line_number <= file.lines.len() {
            let line = &mut file.lines[line_number - 1];
            if position < line.text.len() {
                line.text.remove(position);
                line.tokens = GcodeLine::tokenize(&line.text);
                return true;
            }
        }
        false
    }

    /// Get a specific line's text
    pub fn get_line_text(&self, line_number: usize) -> Option<String> {
        let file = self.lock_file();

        if line_number > 0 && line_number <= file.lines.len() {
            Some(file.lines[line_number - 1].text.clone())
        } else {
            None
        }
    }

    /// Get all line texts
    pub fn get_all_lines(&self) -> Vec<String> {
        let file = self.lock_file();
        file.lines.iter().map(|l| l.text.clone()).collect()
    }

    /// Set read-only mode (inverse of editable)
    pub fn set_read_only(&self, read_only: bool) {
        self.set_editable(!read_only)
    }

    /// Check if editor is in read-only mode
    pub fn is_read_only(&self) -> bool {
        !self.is_editable()
    }

    /// Load a file from disk using a file dialog
    pub fn open_file_dialog(&self) -> anyhow::Result<PathBuf> {
        let file = rfd::FileDialog::new()
            .add_filter("G-Code files", &["gcode", "gc", "ngc", "tap"])
            .add_filter("Text files", &["txt"])
            .add_filter("All files", &["*"])
            .pick_file()
        .ok_or_else(|| anyhow::anyhow!("File dialog cancelled"))?;

        Ok(file)
    }

    /// Load file content from disk
    pub fn load_file(&self, path: &Path) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.load_content(&content)?;

        let mut file = self.lock_file();
        file.path = Some(path.to_string_lossy().to_string());
        Ok(())
    }

    /// Load file content and extract the path from dialog
    pub fn open_and_load_file(&self) -> anyhow::Result<PathBuf> {
        let path = self.open_file_dialog()?;
        self.load_file(&path)?;
        Ok(path)
    }

    /// Save file to disk with given content
    pub fn save_file_with_content(&self, content: &str) -> anyhow::Result<()> {
        let file = self.lock_file();

        if let Some(path) = &file.path {
            std::fs::write(path, content)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No file path set. Use save_as instead."))
        }
    }

    /// Save file to disk (uses internal content)
    pub fn save_file(&self) -> anyhow::Result<()> {
        let file = self.lock_file();

        if let Some(path) = &file.path {
            let content = file.get_plain_content();
            std::fs::write(path, content)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No file path set. Use save_as instead."))
        }
    }

    /// Save file with a new name using file dialog
    pub fn save_as_dialog(&self) -> anyhow::Result<PathBuf> {
        let file = rfd::FileDialog::new()
            .add_filter("G-Code files", &["gcode", "gc", "ngc", "tap"])
            .add_filter("Text files", &["txt"])
            .add_filter("All files", &["*"])
            .set_file_name("untitled.gcode")
            .save_file()
        .ok_or_else(|| anyhow::anyhow!("Save dialog cancelled"))?;

        Ok(file)
    }

    /// Save file with a new name
    pub fn save_as(&self, path: &Path) -> anyhow::Result<()> {
        let content = {
            let file = self.lock_file();
            file.get_plain_content()
        };

        std::fs::write(path, content)?;

        let mut file = self.lock_file();
        file.path = Some(path.to_string_lossy().to_string());
        Ok(())
    }

    /// Save with a new name and provided content
    pub fn save_as_with_content(&self, path: &Path, content: &str) -> anyhow::Result<()> {
        std::fs::write(path, content)?;

        let mut file = self.lock_file();
        file.path = Some(path.to_string_lossy().to_string());
        Ok(())
    }

    /// Save file with new name using dialog
    pub fn save_as_with_dialog(&self) -> anyhow::Result<PathBuf> {
        let path = self.save_as_dialog()?;
        self.save_as(&path)?;
        Ok(path)
    }

    /// Save file with new name using dialog and provided content
    pub fn save_as_with_dialog_and_content(&self, content: &str) -> anyhow::Result<PathBuf> {
        let path = self.save_as_dialog()?;
        self.save_as_with_content(&path, content)?;
        Ok(path)
    }

    /// Get the current file path
    pub fn get_file_path(&self) -> Option<String> {
        let file = self.lock_file();
        file.path.clone()
    }

    /// Set the file path (useful for tracking)
    pub fn set_file_path(&self, path: Option<String>) {
        let mut file = self.lock_file();
        file.path = path;
    }

    /// Export content to a specific file
    pub fn export_to(&self, path: &Path) -> anyhow::Result<()> {
        let content = self.get_plain_content();
        std::fs::write(path, content)?;
        Ok(())
    }

    // ---
pub fn on_gcode_loaded(&mut self, gcode: String) {
        match parse_gcode_to_toolpaths(&gcode) {
            Ok(toolpaths) => {
                let _ = self.toolpath_sender.send(toolpaths);
            }
            Err(e) => eprintln!("Error parsing G-code: {}", e),
        }
    }
    // ---
}

impl Default for GcodeEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_gcommand() {
        let tokens = GcodeLine::tokenize("G00 X10 Y20 Z5");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::GCommand));
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Coordinate));
    }

    #[test]
    fn test_tokenize_comment() {
        let tokens = GcodeLine::tokenize("; This is a comment");
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    #[test]
    fn test_gcode_file_load() {
        let mut file = GcodeFile::new();
        file.load_content("G00 X10\nG01 Y20\nG00 Z5");
        assert_eq!(file.lines.len(), 3);
        assert_eq!(file.lines[0].line_number, 1);
    }

    #[test]
    fn test_mark_executed() {
        let mut file = GcodeFile::new();
        file.load_content("G00 X10\nG01 Y20\nG00 Z5");
        file.mark_executed(1);
        assert!(file.lines[0].executed);
        assert!(!file.lines[1].executed);
    }

    #[test]
    fn test_set_current_line() {
        let mut file = GcodeFile::new();
        file.load_content("G00 X10\nG01 Y20\nG00 Z5");
        file.set_current_line(2);
        assert!(!file.lines[0].is_current);
        assert!(file.lines[1].is_current);
        assert_eq!(file.current_line, 2);
    }

    #[test]
    fn test_editor_content() {
        let editor = GcodeEditor::new();
        editor.load_content("G00 X10\nG01 Y20").expect("load failed");
        editor.set_current_line(1);
        let content = editor.get_display_content();
        assert!(content.contains("▶")); // Current line marker shows for current line

        // Now test executed marker on a different line
        editor.set_current_line(2);
        editor.mark_line_executed(1);
        let content = editor.get_display_content();
        assert!(content.contains("✓")); // Executed marker shows for executed line
    }

    #[test]
    fn test_search() {
        let editor = GcodeEditor::new();
        editor.load_content("G00 X10\nG01 Y20\nG00 X30").expect("load failed");
        let results = editor.search("G00");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_case_insensitive() {
        let editor = GcodeEditor::new();
        editor.load_content("g00 X10\nG01 Y20").expect("load failed");
        let results = editor.search("G00");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_replace_all() {
        let editor = GcodeEditor::new();
        editor.load_content("X10 Y10\nX20 Y10").expect("load failed");
        let count = editor.replace_all("Y10", "Y30");
        assert_eq!(count, 2);

        let content = editor.get_plain_content();
        assert!(content.contains("Y30"));
        assert!(!content.contains("Y10"));
    }

    #[test]
    fn test_insert_text() {
        let editor = GcodeEditor::new();
        editor.load_content("G00 X10").expect("load failed");
        editor.set_editable(true);

        let result = editor.insert_text(1, 4, " Y20");
        assert!(result);

        let text = editor.get_line_text(1);
        assert_eq!(text, Some("G00  Y20X10".to_string()));
    }

    #[test]
    fn test_delete_char() {
        let editor = GcodeEditor::new();
        editor.load_content("G00 X10").expect("load failed");
        editor.set_editable(true);

        let result = editor.delete_char(1, 3);
        assert!(result);

        let text = editor.get_line_text(1);
        assert_eq!(text, Some("G00X10".to_string()));
    }

    #[test]
    fn test_read_only_mode() {
        let editor = GcodeEditor::new();
        editor.load_content("G00 X10").expect("load failed");
        editor.set_read_only(true);

        let result = editor.insert_text(1, 0, "test");
        assert!(!result);

        let text = editor.get_line_text(1);
        assert_eq!(text, Some("G00 X10".to_string()));
    }

    #[test]
    fn test_file_path_tracking() {
        let editor = GcodeEditor::new();
        editor.load_content("G00 X10").expect("load failed");

        let path = Some("/path/to/file.gcode".to_string());
        editor.set_file_path(path.clone());

        assert_eq!(editor.get_file_path(), path);
    }

    #[test]
    fn test_export_content() {
        use tempfile::NamedTempFile;

        let editor = GcodeEditor::new();
        editor.load_content("G00 X10\nG01 Y20").expect("load failed");

        let temp_file = NamedTempFile::new().expect("temp file failed");
        let path = temp_file.path();

        let result = editor.export_to(path);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(path).expect("read failed");
        assert_eq!(content, "G00 X10\nG01 Y20");
    }

    #[test]
    fn test_save_as_file() {
        use tempfile::NamedTempFile;

        let editor = GcodeEditor::new();
        editor.load_content("G00 X10").expect("load failed");

        let temp_file = NamedTempFile::new().expect("temp file failed");
        let path = temp_file.path();

        let result = editor.save_as(path);
        assert!(result.is_ok());
        assert_eq!(
            editor.get_file_path(),
            Some(path.to_string_lossy().to_string())
        );

        let content = std::fs::read_to_string(path).expect("read failed");
        assert_eq!(content, "G00 X10");
    }
}
