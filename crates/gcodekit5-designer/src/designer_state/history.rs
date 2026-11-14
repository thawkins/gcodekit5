//! History management (undo/redo) for designer state.

use super::DesignerState;
use crate::commands::DesignerCommand;

impl DesignerState {
    /// Pushes a command to the undo stack and executes it.
    pub fn push_command(&mut self, mut cmd: DesignerCommand) {
        cmd.apply(&mut self.canvas);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        // Limit stack size to 50
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.is_modified = true;
        self.gcode_generated = false;
    }

    /// Records a command that has already been applied.
    /// Use this when the action has already happened and you just want to record it for undo.
    pub fn record_command(&mut self, cmd: DesignerCommand) {
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        // Limit stack size to 50
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.is_modified = true;
        self.gcode_generated = false;
    }

    /// Undo last change.
    pub fn undo(&mut self) {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(&mut self.canvas);
            self.redo_stack.push(cmd);
            self.gcode_generated = false;
            self.is_modified = true;
        }
    }

    /// Redo last undo.
    pub fn redo(&mut self) {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.apply(&mut self.canvas);
            self.undo_stack.push(cmd);
            self.gcode_generated = false;
            self.is_modified = true;
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear history stacks.
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
