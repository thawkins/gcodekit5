//! Error types for the G-code editor crate.
//!
//! This module provides structured error types for text buffer operations,
//! editor state management, and undo/redo handling.

use thiserror::Error;

/// Errors that can occur during editor operations.
#[derive(Error, Debug)]
pub enum EditorError {
    /// The requested position is out of bounds.
    #[error("Position out of bounds: {position} (max: {max})")]
    PositionOutOfBounds { position: usize, max: usize },

    /// The requested line does not exist.
    #[error("Line out of bounds: {line} (total: {total})")]
    LineOutOfBounds { line: usize, total: usize },

    /// The requested range is invalid.
    #[error("Invalid range: {start}..{end} (max: {max})")]
    InvalidRange {
        start: usize,
        end: usize,
        max: usize,
    },

    /// A buffer operation failed.
    #[error("Buffer error: {0}")]
    Buffer(#[from] BufferError),

    /// The undo stack is empty.
    #[error("Nothing to undo")]
    NothingToUndo,

    /// The redo stack is empty.
    #[error("Nothing to redo")]
    NothingToRedo,

    /// The editor state is invalid.
    #[error("Invalid editor state: {0}")]
    InvalidState(String),
}

/// Errors related to text buffer operations.
#[derive(Error, Debug)]
pub enum BufferError {
    /// Character index is out of bounds.
    #[error("Character index out of bounds: {index} (length: {length})")]
    CharIndexOutOfBounds { index: usize, length: usize },

    /// Line index is out of bounds.
    #[error("Line index out of bounds: {index} (lines: {total})")]
    LineIndexOutOfBounds { index: usize, total: usize },

    /// The text slice range is invalid.
    #[error("Invalid slice range: {start}..{end}")]
    InvalidSlice { start: usize, end: usize },

    /// The text is not valid UTF-8.
    #[error("Invalid UTF-8 text")]
    InvalidUtf8,

    /// The operation would create an empty buffer in an invalid state.
    #[error("Buffer operation would create invalid state: {0}")]
    InvalidOperation(String),
}

/// Result type alias for editor operations.
pub type EditorResult<T> = Result<T, EditorError>;

/// Result type alias for buffer operations.
pub type BufferResult<T> = Result<T, BufferError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_error_display() {
        let err = EditorError::PositionOutOfBounds {
            position: 150,
            max: 100,
        };
        assert_eq!(err.to_string(), "Position out of bounds: 150 (max: 100)");

        let err = EditorError::LineOutOfBounds {
            line: 50,
            total: 30,
        };
        assert_eq!(err.to_string(), "Line out of bounds: 50 (total: 30)");

        let err = EditorError::NothingToUndo;
        assert_eq!(err.to_string(), "Nothing to undo");
    }

    #[test]
    fn test_buffer_error_display() {
        let err = BufferError::CharIndexOutOfBounds {
            index: 200,
            length: 150,
        };
        assert_eq!(
            err.to_string(),
            "Character index out of bounds: 200 (length: 150)"
        );

        let err = BufferError::InvalidSlice { start: 50, end: 10 };
        assert_eq!(err.to_string(), "Invalid slice range: 50..10");

        let err = BufferError::InvalidUtf8;
        assert_eq!(err.to_string(), "Invalid UTF-8 text");
    }

    #[test]
    fn test_error_conversion() {
        let buf_err = BufferError::InvalidUtf8;
        let ed_err: EditorError = buf_err.into();
        assert!(matches!(ed_err, EditorError::Buffer(_)));
    }
}
