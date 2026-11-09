//! Undo/Redo manager for text editing operations

use std::collections::VecDeque;
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
            // `char_range` is in characters, so measure the inserted text in chars
            // (not bytes) or multi-byte text (e.g. "°") corrupts the undo range.
            char_range: self.char_range.start
                ..(self.char_range.start + self.new_text.chars().count()),
            old_text: self.new_text.clone(),
            new_text: self.old_text.clone(),
            old_cursor: self.new_cursor,
            new_cursor: self.old_cursor,
        }
    }
}

/// Manages undo/redo stacks for text editing
///
/// Every undo step is a group of one or more [`TextChange`]s. Changes recorded
/// between [`UndoManager::begin_batch`] and [`UndoManager::end_batch`] form a
/// single group, so they are undone and redone atomically.
pub struct UndoManager {
    undo_stack: VecDeque<Vec<TextChange>>,
    redo_stack: Vec<Vec<TextChange>>,
    max_depth: usize,
    current_batch: Option<Vec<TextChange>>,
}

impl UndoManager {
    /// Create a new undo manager with default depth (100)
    pub fn new() -> Self {
        Self::with_depth(100)
    }

    /// Create with custom maximum undo depth
    ///
    /// # Arguments
    /// * `max_depth` - Maximum number of undo steps (groups) kept.
    pub fn with_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_depth.min(1024)),
            redo_stack: Vec::new(),
            max_depth,
            current_batch: None,
        }
    }

    /// Record a change to the undo stack
    ///
    /// While a batch is open the change joins the batch; otherwise it becomes
    /// its own undo step.
    pub fn record(&mut self, change: TextChange) {
        if let Some(batch) = &mut self.current_batch {
            batch.push(change);
        } else {
            self.push_undo(vec![change]);
        }
    }

    /// Push a group of changes to the undo stack as one step
    fn push_undo(&mut self, group: Vec<TextChange>) {
        if group.is_empty() {
            return;
        }
        // Clear redo stack when new change is made
        self.redo_stack.clear();

        self.undo_stack.push_back(group);

        // Trim if exceeds max depth (O(1) with a VecDeque)
        while self.undo_stack.len() > self.max_depth {
            self.undo_stack.pop_front();
        }
    }

    /// Start batching changes (for multi-operation edits)
    ///
    /// If a batch is already open it stays open, so nested begin/end pairs
    /// still produce a single undo step.
    pub fn begin_batch(&mut self) {
        if self.current_batch.is_none() {
            self.current_batch = Some(Vec::new());
        }
    }

    /// End batch and commit all its changes as a single, atomic undo step
    pub fn end_batch(&mut self) {
        if let Some(batch) = self.current_batch.take() {
            self.push_undo(batch);
        }
    }

    /// Undo the last step
    ///
    /// # Returns
    /// The inverse changes of the step, in the order they must be applied
    /// (last change first), or `None` when there is nothing to undo.
    pub fn undo(&mut self) -> Option<Vec<TextChange>> {
        let group = self.undo_stack.pop_back()?;
        let inverses = group.iter().rev().map(TextChange::inverse).collect();
        self.redo_stack.push(group);
        Some(inverses)
    }

    /// Redo the last undone step
    ///
    /// # Returns
    /// The changes of the step, in the order they must be applied, or `None`
    /// when there is nothing to redo.
    pub fn redo(&mut self) -> Option<Vec<TextChange>> {
        let group = self.redo_stack.pop()?;
        self.undo_stack.push_back(group.clone());
        Some(group)
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

    /// Get number of undo steps available
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get number of redo steps available
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}
