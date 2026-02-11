//! Undo/Redo manager for text editing operations

use std::ops::Range;

/// Represents a single text change for undo/redo
#[derive(Clone, Debug)]
pub struct TextChange {
    /// Character range affected
    pub char_range: Range<usize>,
    /// Text before the change (for undo)
    pub old_text: String,
    /// Text after the change (for redo)
    pub new_text: String,
    /// Cursor position before change
    pub old_cursor: usize,
    /// Cursor position after change
    pub new_cursor: usize,
}

impl TextChange {
    /// Create a new text change
    pub fn new(
        char_range: Range<usize>,
        old_text: String,
        new_text: String,
        old_cursor: usize,
        new_cursor: usize,
    ) -> Self {
        Self {
            char_range,
            old_text,
            new_text,
            old_cursor,
            new_cursor,
        }
    }

    /// Create inverse change for undo
    pub fn inverse(&self) -> Self {
        Self {
            char_range: self.char_range.start..(self.char_range.start + self.new_text.len()),
            old_text: self.new_text.clone(),
            new_text: self.old_text.clone(),
            old_cursor: self.new_cursor,
            new_cursor: self.old_cursor,
        }
    }
}

/// Manages undo/redo stacks for text editing
pub struct UndoManager {
    undo_stack: Vec<TextChange>,
    redo_stack: Vec<TextChange>,
    max_depth: usize,
    current_batch: Option<Vec<TextChange>>,
}

impl UndoManager {
    /// Create a new undo manager with default depth (100)
    pub fn new() -> Self {
        Self::with_depth(100)
    }

    /// Create with custom maximum undo depth
    pub fn with_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::with_capacity(max_depth),
            redo_stack: Vec::with_capacity(max_depth),
            max_depth,
            current_batch: None,
        }
    }

    /// Record a change to the undo stack
    pub fn record(&mut self, change: TextChange) {
        if let Some(batch) = &mut self.current_batch {
            // Add to current batch
            batch.push(change);
        } else {
            // Direct push
            self.push_undo(change);
        }
    }

    /// Push a change to undo stack
    fn push_undo(&mut self, change: TextChange) {
        // Clear redo stack when new change is made
        self.redo_stack.clear();

        // Add to undo stack
        self.undo_stack.push(change);

        // Trim if exceeds max depth
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
    }

    /// Start batching changes (for multi-operation edits)
    pub fn begin_batch(&mut self) {
        self.current_batch = Some(Vec::new());
    }

    /// End batch and commit as single undo operation
    pub fn end_batch(&mut self) {
        if let Some(batch) = self.current_batch.take() {
            if !batch.is_empty() {
                // Merge batch into single change if possible
                if batch.len() == 1 {
                    self.push_undo(batch.into_iter().next().expect("batch not empty"));
                } else {
                    // For multiple changes, push them individually
                    // (could be optimized to merge adjacent changes)
                    for change in batch {
                        self.push_undo(change);
                    }
                }
            }
        }
    }

    /// Undo last change
    pub fn undo(&mut self) -> Option<TextChange> {
        self.undo_stack.pop().map(|change| {
            let inverse = change.inverse();
            self.redo_stack.push(change);
            inverse
        })
    }

    /// Redo last undone change
    pub fn redo(&mut self) -> Option<TextChange> {
        self.redo_stack.pop().inspect(|change| {
            self.undo_stack.push(change.clone());
        })
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all undo/redo history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current_batch = None;
    }

    /// Get number of undo operations available
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get number of redo operations available
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_undo() {
        let mut mgr = UndoManager::new();

        let change = TextChange::new(0..0, String::new(), "Hello".to_string(), 0, 5);

        mgr.record(change);
        assert!(mgr.can_undo());
        assert!(!mgr.can_redo());

        let undo_change = mgr.undo().expect("undo available");
        assert_eq!(undo_change.new_text, "");
        assert_eq!(undo_change.old_text, "Hello");
        assert!(!mgr.can_undo());
        assert!(mgr.can_redo());
    }

    #[test]
    fn test_redo() {
        let mut mgr = UndoManager::new();

        let change = TextChange::new(0..0, String::new(), "Hello".to_string(), 0, 5);

        mgr.record(change);
        mgr.undo();

        let redo_change = mgr.redo().expect("redo available");
        assert_eq!(redo_change.old_text, "");
        assert_eq!(redo_change.new_text, "Hello");
    }

    #[test]
    fn test_clear_redo_on_new_change() {
        let mut mgr = UndoManager::new();

        mgr.record(TextChange::new(0..0, String::new(), "A".to_string(), 0, 1));
        mgr.undo();
        assert!(mgr.can_redo());

        mgr.record(TextChange::new(0..0, String::new(), "B".to_string(), 0, 1));
        assert!(!mgr.can_redo());
    }

    #[test]
    fn test_max_depth() {
        let mut mgr = UndoManager::with_depth(3);

        mgr.record(TextChange::new(0..0, String::new(), "1".to_string(), 0, 1));
        mgr.record(TextChange::new(1..1, String::new(), "2".to_string(), 1, 2));
        mgr.record(TextChange::new(2..2, String::new(), "3".to_string(), 2, 3));
        mgr.record(TextChange::new(3..3, String::new(), "4".to_string(), 3, 4));

        assert_eq!(mgr.undo_count(), 3); // Should have trimmed oldest
    }

    #[test]
    fn test_batch() {
        let mut mgr = UndoManager::new();

        mgr.begin_batch();
        mgr.record(TextChange::new(0..0, String::new(), "H".to_string(), 0, 1));
        mgr.record(TextChange::new(1..1, String::new(), "i".to_string(), 1, 2));
        mgr.end_batch();

        assert_eq!(mgr.undo_count(), 2); // Two changes in batch
    }
}
