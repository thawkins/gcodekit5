//! Bridge between Slint UI and EditorState backend

use gcodekit5_gcodeeditor::EditorState;
use slint::{Model, ModelRc, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

/// Slint-compatible text line structure
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

/// Bridge between EditorState and Slint UI
pub struct EditorBridge {
    editor: Rc<RefCell<EditorState>>,
    visible_lines: Rc<VecModel<SlintTextLine>>,
}

impl EditorBridge {
    /// Create a new editor bridge
    pub fn new(viewport_height: f32, line_height: f32) -> Self {
        let editor = Rc::new(RefCell::new(EditorState::new(viewport_height, line_height)));
        let visible_lines = Rc::new(VecModel::default());

        Self {
            editor,
            visible_lines,
        }
    }

    /// Update viewport dimensions (called when UI resizes)
    pub fn set_viewport_size(&self, viewport_height: f32, line_height: f32) {
        let mut editor = self.editor.borrow_mut();
        editor.set_viewport_size(viewport_height, line_height);
        drop(editor);
        self.update_visible_lines();
    }

    /// Load text into editor
    pub fn load_text(&self, text: &str) {
        let mut editor = self.editor.borrow_mut();
        editor.load_text(text);
        let _line_count = editor.line_count();
        drop(editor);
        self.update_visible_lines();
    }

    /// Get all text from editor
    pub fn get_text(&self) -> String {
        self.editor.borrow().get_text()
    }

    /// Insert text at cursor
    pub fn insert_text(&self, text: &str) {
        let mut editor = self.editor.borrow_mut();
        editor.insert_text(text);
        drop(editor);
        self.update_visible_lines();
    }

    /// Delete text forward (delete key)
    pub fn delete_forward(&self, count: usize) {
        let mut editor = self.editor.borrow_mut();
        editor.delete_forward(count);
        drop(editor);
        self.update_visible_lines();
    }

    /// Delete text backward (backspace key)
    pub fn delete_backward(&self, count: usize) {
        let mut editor = self.editor.borrow_mut();
        editor.delete_backward(count);
        drop(editor);
        self.update_visible_lines();
    }

    /// Undo last change
    pub fn undo(&self) -> bool {
        let mut editor = self.editor.borrow_mut();
        let result = editor.undo();
        drop(editor);
        if result {
            self.update_visible_lines();
        }
        result
    }

    /// Redo last undone change
    pub fn redo(&self) -> bool {
        let mut editor = self.editor.borrow_mut();
        let result = editor.redo();
        drop(editor);
        if result {
            self.update_visible_lines();
        }
        result
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.editor.borrow().can_undo()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.editor.borrow().can_redo()
    }

    /// Scroll viewport to specific line
    pub fn scroll_to_line(&self, line: usize) {
        let mut editor = self.editor.borrow_mut();
        let _total_lines = editor.line_count();
        // Use scroll_to_line for absolute positioning (not scroll_by which is relative)
        editor.scroll_to_line(line);
        let _viewport = editor.viewport();
        drop(editor);
        self.update_visible_lines();
    }

    /// Set cursor position
    pub fn set_cursor(&self, line: usize, column: usize) {
        let editor = self.editor.borrow();
        let char_pos = editor.line_col_to_char(line, column);
        drop(editor);
        
        let mut editor = self.editor.borrow_mut();
        editor.set_cursor(char_pos);
    }

    /// Get cursor line and column
    pub fn cursor_position(&self) -> (usize, usize) {
        self.editor.borrow().cursor_line_col()
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.editor.borrow().line_count()
    }

    /// Check if modified
    pub fn is_modified(&self) -> bool {
        self.editor.borrow().is_modified()
    }

    /// Mark as unmodified
    pub fn mark_unmodified(&self) {
        self.editor.borrow_mut().mark_unmodified();
    }

    /// Get visible lines as Slint model
    pub fn get_visible_lines_model(&self) -> ModelRc<SlintTextLine> {
        ModelRc::from(self.visible_lines.clone())
    }

    /// Get visible lines as raw data for constructing Slint types
    pub fn get_visible_lines_data(&self) -> Vec<(i32, String, bool)> {
        self.visible_lines
            .iter()
            .map(|line| (line.line_number, line.content.clone(), line.is_dirty))
            .collect()
    }

    /// Get line content at given line index (0-based)
    pub fn get_line_at(&self, line_idx: usize) -> Option<String> {
        let editor = self.editor.borrow();
        editor.get_line(line_idx)
    }

    /// Update visible lines from editor state
    fn update_visible_lines(&self) {
        let editor = self.editor.borrow();
        let (first_line_idx, lines) = editor.get_visible_lines();
        let _viewport = editor.viewport();
        let _total_lines = editor.line_count();

        // Clear and rebuild visible lines
        let mut new_lines = Vec::new();
        for (idx, content) in lines.iter().enumerate() {
            // Line numbers are 1-indexed for display, first_line_idx is 0-indexed
            let line_number = first_line_idx + idx + 1;
            new_lines.push(SlintTextLine::new(
                line_number,
                content.clone(),
                false, // Could check dirty state here
            ));
        }

        // Update model
        self.visible_lines.set_vec(new_lines);
    }

    /// Get viewport info
    pub fn viewport_range(&self) -> (usize, usize) {
        let editor = self.editor.borrow();
        let viewport = editor.viewport();
        (viewport.start_line, viewport.end_line)
    }
}

