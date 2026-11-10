//! Custom text editor module with undo/redo and optimized rendering
//!
//! This module provides a high-performance text editor implementation optimized
//! for large G-code files with efficient text manipulation and rendering.

#[cfg(feature = "slint_legacy_tests")]
mod slint_bridge;
mod text_buffer;
mod undo_manager;
mod viewport;

#[cfg(feature = "slint_legacy_tests")]
pub use slint_bridge::{EditorBridge, SlintTextLine};

// When the slint legacy feature is disabled, provide a lightweight backend alias
// so other parts of the UI can import `gcodekit5_ui::EditorBridge` without requiring Slint.
#[cfg(not(feature = "slint_legacy_tests"))]
mod slint_bridge_stub {
    pub use gcodekit5_gcodeeditor::EditorBridgeBackend as EditorBridge;

    #[derive(Clone, Debug)]
    pub struct SlintTextLine {
        pub line_number: i32,
        pub content: String,
        pub is_dirty: bool,
    }
    impl SlintTextLine {
        pub fn new(line_number: usize, content: String, is_dirty: bool) -> Self {
            Self {
                line_number: line_number as i32,
                content,
                is_dirty,
            }
        }
    }
}

#[cfg(not(feature = "slint_legacy_tests"))]
pub use slint_bridge_stub::{EditorBridge, SlintTextLine};
pub use text_buffer::TextBuffer;
pub use undo_manager::{TextChange, UndoManager};
pub use viewport::Viewport;

// Re-export for Slint UI
#[derive(Clone, Debug)]
pub struct TextLine {
    pub line_number: i32,
    pub content: String,
    pub is_dirty: bool,
}

/// Complete editor state managing buffer, undo/redo, and viewport
pub struct EditorState {
    buffer: TextBuffer,
    undo_manager: UndoManager,
    viewport: Viewport,
    cursor_pos: usize,
    selection: Option<(usize, usize)>,
    modified: bool,
}

impl EditorState {
    /// Create a new editor state
    pub fn new(viewport_height: f32, line_height: f32) -> Self {
        let mut viewport = Viewport::new(viewport_height, line_height);
        let buffer = TextBuffer::new();
        viewport.set_total_lines(buffer.len_lines());

        Self {
            buffer,
            undo_manager: UndoManager::new(),
            viewport,
            cursor_pos: 0,
            selection: None,
            modified: false,
        }
    }

    /// Load text from string
    pub fn load_text(&mut self, text: &str) {
        self.buffer = TextBuffer::from_str(text);
        self.viewport.set_total_lines(self.buffer.len_lines());
        self.cursor_pos = 0;
        self.selection = None;
        self.undo_manager.clear();
        self.modified = false;
    }

    /// Get all text
    pub fn get_text(&self) -> String {
        self.buffer.to_string()
    }

    /// Insert text at cursor
    pub fn insert_text(&mut self, text: &str) {
        let old_text = String::new();
        let old_cursor = self.cursor_pos;

        self.buffer.insert(self.cursor_pos, text);
        let new_cursor = self.cursor_pos + text.len();

        let change = TextChange::new(
            self.cursor_pos..self.cursor_pos,
            old_text,
            text.to_string(),
            old_cursor,
            new_cursor,
        );

        self.undo_manager.record(change);
        self.cursor_pos = new_cursor;
        self.viewport.set_total_lines(self.buffer.len_lines());
        self.modified = true;
    }

    /// Delete text before cursor (backspace key)
    pub fn delete_backward(&mut self, count: usize) {
        if let Some((_start, _end)) = self.selection {
            self.delete_selection();
        } else if self.cursor_pos > 0 {
            let start = self.cursor_pos.saturating_sub(count);
            let old_text = self.buffer.slice(start, self.cursor_pos);

            self.buffer.delete(start..self.cursor_pos);

            let change = TextChange::new(
                start..self.cursor_pos,
                old_text,
                String::new(),
                self.cursor_pos,
                start,
            );

            self.undo_manager.record(change);
            self.cursor_pos = start;
            self.viewport.set_total_lines(self.buffer.len_lines());
            self.modified = true;
        }
    }

    /// Delete current selection
    fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection {
            let old_text = self.buffer.slice(start, end);
            self.buffer.delete(start..end);

            let change =
                TextChange::new(start..end, old_text, String::new(), self.cursor_pos, start);

            self.undo_manager.record(change);
            self.cursor_pos = start;
            self.selection = None;
            self.viewport.set_total_lines(self.buffer.len_lines());
            self.modified = true;
        }
    }

    /// Undo last change
    pub fn undo(&mut self) -> bool {
        if let Some(change) = self.undo_manager.undo() {
            self.buffer
                .replace(change.char_range.clone(), &change.new_text);
            self.cursor_pos = change.new_cursor;
            self.viewport.set_total_lines(self.buffer.len_lines());
            self.modified = true;
            true
        } else {
            false
        }
    }

    /// Redo last undone change
    pub fn redo(&mut self) -> bool {
        if let Some(change) = self.undo_manager.redo() {
            self.buffer
                .replace(change.char_range.clone(), &change.new_text);
            self.cursor_pos = change.new_cursor;
            self.viewport.set_total_lines(self.buffer.len_lines());
            self.modified = true;
            true
        } else {
            false
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.undo_manager.can_undo()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.undo_manager.can_redo()
    }

    /// Get visible lines for rendering with overscan for smooth scrolling
    /// Returns `(start_line, Vec<String>)` where start_line is the 0-indexed line number of the first line
    pub fn get_visible_lines(&self) -> (usize, Vec<String>) {
        // Use small overscan (5 lines) to reduce lag while maintaining smooth scrolling
        let range = self.viewport.overscan_range(5);
        let start_line = range.start;
        let mut lines = self.buffer.lines_in_range(range);

        // If editor is empty, always provide at least one line with a space so cursor is visible
        if lines.is_empty() {
            lines.push(" ".to_string());
        }

        (start_line, lines)
    }

    /// Scroll viewport by delta lines
    pub fn scroll_by(&mut self, delta: i32) {
        self.viewport.scroll_by(delta);
    }

    /// Scroll viewport to absolute line number (sets scroll offset to show that line at top)
    pub fn scroll_to_line(&mut self, line: usize) {
        self.viewport.set_scroll_offset(line);
    }

    /// Get viewport info
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Set cursor position
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor_pos = pos.min(self.buffer.len_chars());

        // Scroll to cursor if needed
        let (line, _) = self.buffer.char_to_line_col(self.cursor_pos);
        self.viewport.scroll_to_line(line);
    }

    /// Get cursor position
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Check if document is modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Mark as unmodified (after save)
    pub fn mark_unmodified(&mut self) {
        self.modified = false;
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.buffer.len_lines()
    }

    /// Update viewport size (when UI resizes)
    pub fn set_viewport_size(&mut self, viewport_height: f32, line_height: f32) {
        self.viewport
            .set_viewport_size(viewport_height, line_height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_insert() {
        let mut editor = EditorState::new(400.0, 20.0);
        editor.insert_text("Hello");
        assert_eq!(editor.get_text(), "Hello");
        assert_eq!(editor.cursor_pos(), 5);
        assert!(editor.is_modified());
    }

    #[test]
    fn test_editor_undo_redo() {
        let mut editor = EditorState::new(400.0, 20.0);
        editor.insert_text("Hello");

        assert!(editor.can_undo());
        editor.undo();
        assert_eq!(editor.get_text(), "");

        assert!(editor.can_redo());
        editor.redo();
        assert_eq!(editor.get_text(), "Hello");
    }

    #[test]
    fn test_editor_delete() {
        let mut editor = EditorState::new(400.0, 20.0);
        editor.insert_text("Hello");
        editor.delete_backward(2);
        assert_eq!(editor.get_text(), "Hel");
    }
}
