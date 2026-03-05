//! # G-code Editor Bridge
//!
//! Bridges the G-code editor backend (text buffer, undo manager, viewport)
//! with the GTK4 UI layer. Handles text editing events, syntax state,
//! and cursor management.

use crate::EditorState;
use std::cell::RefCell;
use std::rc::Rc;

/// A minimal EditorBridge for non-UI consumers. This mirrors the Slint 'EditorBridge'
/// API used by tests and is intended for GTK porting later.
pub struct EditorBridgeBackend {
    state: Rc<RefCell<EditorState>>,
}

impl EditorBridgeBackend {
    /// Create a new EditorBridge instance.
    pub fn new(viewport_height: f32, line_height: f32) -> Self {
        Self {
            state: Rc::new(RefCell::new(EditorState::new(viewport_height, line_height))),
        }
    }

    pub fn load_text(&self, text: &str) {
        self.state.borrow_mut().load_text(text);
    }

    pub fn get_text(&self) -> String {
        self.state.borrow().get_text()
    }

    pub fn insert_text(&self, text: &str) {
        self.state.borrow_mut().insert_text(text);
    }

    pub fn can_undo(&self) -> bool {
        self.state.borrow().can_undo()
    }

    pub fn undo(&self) -> bool {
        self.state.borrow_mut().undo()
    }

    pub fn can_redo(&self) -> bool {
        self.state.borrow().can_redo()
    }

    pub fn redo(&self) -> bool {
        self.state.borrow_mut().redo()
    }

    pub fn line_count(&self) -> usize {
        self.state.borrow().line_count()
    }

    pub fn is_modified(&self) -> bool {
        self.state.borrow().is_modified()
    }

    pub fn mark_unmodified(&self) {
        self.state.borrow_mut().mark_unmodified();
    }

    /// Get a single line at index (0-based)
    pub fn get_line_at(&self, idx: usize) -> Option<String> {
        self.state.borrow().get_line(idx)
    }

    /// Get viewport range (start_line, end_line)
    pub fn viewport_range(&self) -> (usize, usize) {
        let s = self.state.borrow();
        let viewport = s.viewport();
        (viewport.start_line, viewport.end_line)
    }

    /// Get visible lines data for UI consumption
    pub fn get_visible_lines_data(&self) -> Vec<(i32, String, bool)> {
        let (first, lines) = self.state.borrow().get_visible_lines();
        lines
            .into_iter()
            .enumerate()
            .map(|(i, content)| ((first + i + 1) as i32, content, false))
            .collect()
    }
}
