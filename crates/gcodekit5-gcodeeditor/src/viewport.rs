//! Viewport manager for virtual scrolling and efficient rendering

use std::ops::Range;

/// Manages the visible viewport for virtual scrolling
/// Only renders lines that are currently visible
#[derive(Clone, Debug)]
pub struct Viewport {
    /// First visible line index
    pub start_line: usize,
    /// Last visible line index (exclusive)
    pub end_line: usize,
    /// Number of lines visible in viewport
    pub visible_lines: usize,
    /// Total number of lines in document
    pub total_lines: usize,
    /// Vertical scroll offset (in lines)
    pub scroll_offset: usize,
    /// Line height in pixels
    pub line_height: f32,
    /// Viewport height in pixels
    pub viewport_height: f32,
}

impl Viewport {
    /// Create a new viewport
    pub fn new(viewport_height: f32, line_height: f32) -> Self {
        let visible_lines = (viewport_height / line_height).ceil() as usize;
        Self {
            start_line: 0,
            end_line: visible_lines,
            visible_lines,
            total_lines: 0,
            scroll_offset: 0,
            line_height,
            viewport_height,
        }
    }

    /// Update total line count
    pub fn set_total_lines(&mut self, total: usize) {
        self.total_lines = total;
        // Re-clamp the scroll offset: the document may have shrunk below it
        self.set_scroll_offset(self.scroll_offset);
    }

    /// Update viewport dimensions
    pub fn set_viewport_size(&mut self, height: f32, line_height: f32) {
        self.viewport_height = height;
        self.line_height = line_height;
        self.visible_lines = (height / line_height).ceil() as usize;
        self.set_scroll_offset(self.scroll_offset);
    }

    /// Scroll by number of lines (positive = down, negative = up)
    pub fn scroll_by(&mut self, delta: i32) {
        let new_offset = (self.scroll_offset as i32 + delta).max(0) as usize;
        self.set_scroll_offset(new_offset);
    }

    /// Set absolute scroll offset
    pub fn set_scroll_offset(&mut self, offset: usize) {
        let max_scroll = self.total_lines.saturating_sub(self.visible_lines);
        self.scroll_offset = offset.min(max_scroll);
        self.update_visible_range();
    }

    /// Scroll to make a specific line visible
    pub fn scroll_to_line(&mut self, line: usize) {
        if line < self.start_line {
            // Scroll up to show line at top
            self.set_scroll_offset(line);
        } else if line >= self.end_line {
            // Scroll down to show line at bottom
            let offset = line.saturating_sub(self.visible_lines.saturating_sub(1));
            self.set_scroll_offset(offset);
        }
    }

    /// Update visible line range based on scroll offset
    fn update_visible_range(&mut self) {
        self.start_line = self.scroll_offset;
        self.end_line = self
            .scroll_offset
            .saturating_add(self.visible_lines)
            .min(self.total_lines);
    }

    /// Get the range of visible lines
    pub fn visible_range(&self) -> Range<usize> {
        self.start_line..self.end_line
    }

    /// Check if a line is currently visible
    pub fn is_line_visible(&self, line: usize) -> bool {
        line >= self.start_line && line < self.end_line
    }

    /// Get normalized scroll position (0.0 to 1.0)
    pub fn scroll_position(&self) -> f32 {
        if self.total_lines <= self.visible_lines {
            0.0
        } else {
            let max_scroll = self.total_lines - self.visible_lines;
            self.scroll_offset as f32 / max_scroll as f32
        }
    }

    /// Set scroll position (0.0 to 1.0)
    pub fn set_scroll_position(&mut self, pos: f32) {
        let pos = pos.clamp(0.0, 1.0);
        let max_scroll = self.total_lines.saturating_sub(self.visible_lines);
        let offset = (pos * max_scroll as f32) as usize;
        self.set_scroll_offset(offset);
    }

    /// Calculate scroll bar height ratio
    pub fn scrollbar_ratio(&self) -> f32 {
        if self.total_lines == 0 {
            1.0
        } else {
            (self.visible_lines as f32 / self.total_lines as f32).min(1.0)
        }
    }

    /// Check if content is scrollable
    pub fn is_scrollable(&self) -> bool {
        self.total_lines > self.visible_lines
    }

    /// Get overscan range (lines to pre-render above/below viewport)
    pub fn overscan_range(&self, overscan_lines: usize) -> Range<usize> {
        let start = self.start_line.saturating_sub(overscan_lines);
        let end = self
            .end_line
            .saturating_add(overscan_lines)
            .min(self.total_lines);
        start..end
    }
}
