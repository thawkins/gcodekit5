//! # GCodeKit5 G-Code Editor
//!
//! This crate provides a high-performance text editor specifically optimized for
//! G-code files with efficient text manipulation, undo/redo support, and GTK4 UI integration.
//!
//! ## Architecture Boundary
//!
//! The editor functionality is split across two crates:
//!
//! ### 1. This Crate (`gcodekit5-gcodeeditor`) - Backend
//! - **Purpose**: Text buffer management, syntax highlighting, undo/redo, line handling
//! - **Dependencies**: No GTK dependencies (uses ropey, thiserror)
//! - **Key Types**: `EditorBridge`, `TextBuffer`, `UndoManager`, `Viewport`
//! - **Location**: `crates/gcodekit5-gcodeeditor/src/`
//!
//! ### 2. UI Crate (`gcodekit5-ui`) - Frontend
//! - **Purpose**: GTK4-based text editor widget using SourceView5
//! - **Dependencies**: GTK4, sourceview5, libadwaita
//! - **Key Types**: `GcodeEditor` (in `ui/gtk/editor.rs`)
//! - **Location**: `crates/gcodekit5-ui/src/ui/gtk/editor.rs`
//!
//! ## Rationale
//!
//! This split allows:
//! 1. **Testability**: Backend logic can be tested without GTK initialization
//! 2. **Reusability**: Text editing logic could be used with other UI frameworks
//! 3. **Separation of Concerns**: Clear boundary between text logic and presentation
//!
//! ## Maintenance Guidelines
//!
//! - **Text manipulation logic** → Belongs in this crate
//! - **GTK widget code** → Belongs in `gcodekit5-ui`
//! - **File I/O** → Can be in either, but prefer this crate for core operations
//! - **Settings/UI integration** → Belongs in `gcodekit5-ui`
//!
//! ## Core Components
//!
//! ### Editor State
//! - **EditorState**: Complete editor state managing text buffer, undo/redo history, and viewport
//! - Handles cursor positioning, text editing operations, and scroll management
//! - Tracks document modifications for save state
//!
//! ### Text Management
//! - **TextBuffer**: Rope-based text storage for efficient large file handling
//! - Character-indexed operations with line/column mapping
//! - Efficient slicing and range operations
//!
//! ### Undo/Redo
//! - **UndoManager**: Full undo/redo history with changeset tracking
//! - Supports insertion, deletion, and complex text transformations
//! - Cursor position preserved across undo/redo operations
//!
//! ### Viewport
//! - **Viewport**: Camera control for navigating large files
//! - Overscan mechanism for smooth scrolling
//! - Efficient visible line range calculation
//!
//! ### Error Handling
//! - **EditorError**: Structured error types for text buffer operations
//! - **EditorResult**: Result type alias with EditorError

mod editor_bridge;
mod error;
mod text_buffer;
mod undo_manager;
mod viewport;

pub use editor_bridge::EditorBridgeBackend;
pub use error::{EditorError, EditorResult};
pub use text_buffer::TextBuffer;
pub use undo_manager::{TextChange, UndoManager};
pub use viewport::Viewport;

/// Editor state combining buffer, undo, and viewport
pub struct EditorState {
    /// The text buffer containing the document
    buffer: TextBuffer,
    /// Undo/redo history manager
    undo_manager: UndoManager,
    /// Viewport for scroll/camera control
    viewport: Viewport,
    /// Track if document has been modified
    modified: bool,
    /// Cursor position (character index)
    cursor: usize,
}

impl EditorState {
    /// Create a new editor state with the given dimensions
    pub fn new(viewport_height: f32, line_height: f32) -> Self {
        Self {
            buffer: TextBuffer::new(),
            undo_manager: UndoManager::new(),
            viewport: Viewport::new(viewport_height, line_height),
            modified: false,
            cursor: 0,
        }
    }

    /// Load text into the buffer
    pub fn load_text(&mut self, text: &str) {
        self.buffer = TextBuffer::new();
        self.buffer.insert(0, text);
        self.viewport.set_total_lines(self.buffer.len_lines());
        self.undo_manager.clear();
        self.modified = false;
        self.cursor = 0;
    }

    /// Current cursor position (character index)
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Move the cursor
    ///
    /// # Arguments
    /// * `char_idx` - New cursor position; clamped to the document length.
    pub fn set_cursor(&mut self, char_idx: usize) {
        self.cursor = char_idx.min(self.buffer.len_chars());
    }

    /// Get the full text content
    pub fn get_text(&self) -> String {
        self.buffer.slice(0, self.buffer.len_chars())
    }

    /// Insert text at the cursor and record it as one undo step
    ///
    /// The cursor moves to the end of the inserted text.
    ///
    /// # Arguments
    /// * `text` - Text to insert.
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let pos = self.cursor.min(self.buffer.len_chars());
        self.buffer.insert(pos, text);
        let new_cursor = pos + text.chars().count();
        self.undo_manager.record(TextChange::new(
            pos..pos,
            String::new(),
            text.to_string(),
            pos,
            new_cursor,
        ));
        self.cursor = new_cursor;
        self.viewport.set_total_lines(self.buffer.len_lines());
        self.modified = true;
    }

    /// Apply one change: replace `char_range` with `new_text`.
    fn apply_change(&mut self, change: &TextChange) {
        self.buffer.delete(change.char_range.clone());
        if !change.new_text.is_empty() {
            self.buffer
                .insert(change.char_range.start, &change.new_text);
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.undo_manager.can_undo()
    }

    /// Perform undo
    pub fn undo(&mut self) -> bool {
        // `UndoManager::undo` already returns the inverses of the recorded
        // changes, in application order; inverting them again would re-apply
        // the original edit.
        let Some(inverses) = self.undo_manager.undo() else {
            return false;
        };
        for inv in &inverses {
            self.apply_change(inv);
        }
        if let Some(last) = inverses.last() {
            self.cursor = last.new_cursor.min(self.buffer.len_chars());
        }
        self.viewport.set_total_lines(self.buffer.len_lines());
        self.modified = true;
        true
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.undo_manager.can_redo()
    }

    /// Perform redo
    pub fn redo(&mut self) -> bool {
        let Some(changes) = self.undo_manager.redo() else {
            return false;
        };
        for change in &changes {
            self.apply_change(change);
        }
        if let Some(last) = changes.last() {
            self.cursor = last.new_cursor.min(self.buffer.len_chars());
        }
        self.viewport.set_total_lines(self.buffer.len_lines());
        self.modified = true;
        true
    }

    /// Get the total number of lines
    pub fn line_count(&self) -> usize {
        self.buffer.len_lines()
    }

    /// Check if the document has been modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Mark the document as unmodified
    pub fn mark_unmodified(&mut self) {
        self.modified = false;
    }

    /// Get a specific line by index
    pub fn get_line(&self, idx: usize) -> Option<String> {
        self.buffer.line(idx)
    }

    /// Get visible lines with content
    pub fn get_visible_lines(&self) -> (usize, Vec<String>) {
        let range = self.viewport.visible_range();
        let lines = self.buffer.lines_in_range(range.clone());
        (range.start, lines)
    }

    /// Get a reference to the viewport
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Get a mutable reference to the viewport
    pub fn viewport_mut(&mut self) -> &mut Viewport {
        &mut self.viewport
    }

    /// Get a reference to the text buffer
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    /// Get a mutable reference to the text buffer
    pub fn buffer_mut(&mut self) -> &mut TextBuffer {
        &mut self.buffer
    }

    /// Get a reference to the undo manager
    pub fn undo_manager(&self) -> &UndoManager {
        &self.undo_manager
    }

    /// Get a mutable reference to the undo manager
    pub fn undo_manager_mut(&mut self) -> &mut UndoManager {
        &mut self.undo_manager
    }
}
