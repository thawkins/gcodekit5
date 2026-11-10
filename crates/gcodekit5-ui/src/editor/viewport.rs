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
        self.update_visible_range();
    }

    /// Update viewport dimensions
    pub fn set_viewport_size(&mut self, height: f32, line_height: f32) {
        self.viewport_height = height;
        self.line_height = line_height;
        self.visible_lines = (height / line_height).ceil() as usize;
        self.update_visible_range();
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
            let offset = line.saturating_sub(self.visible_lines - 1);
            self.set_scroll_offset(offset);
        }
    }

    /// Update visible line range based on scroll offset
    fn update_visible_range(&mut self) {
        self.start_line = self.scroll_offset;
        self.end_line = (self.scroll_offset + self.visible_lines).min(self.total_lines);
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

    /// Get overscan range (lines to pre-render above/below viewport)
    pub fn overscan_range(&self, overscan_lines: usize) -> Range<usize> {
        let start = self.start_line.saturating_sub(overscan_lines);
        let end = (self.end_line + overscan_lines).min(self.total_lines);
        start..end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_viewport() {
        let viewport = Viewport::new(400.0, 20.0);
        assert_eq!(viewport.visible_lines, 20);
        assert_eq!(viewport.start_line, 0);
        assert_eq!(viewport.end_line, 20);
    }

    #[test]
    fn test_set_total_lines() {
        let mut viewport = Viewport::new(400.0, 20.0);
        viewport.set_total_lines(100);
        assert_eq!(viewport.total_lines, 100);
        assert_eq!(viewport.end_line, 20);
    }

    #[test]
    fn test_scroll_by() {
        let mut viewport = Viewport::new(400.0, 20.0);
        viewport.set_total_lines(100);

        viewport.scroll_by(10);
        assert_eq!(viewport.start_line, 10);
        assert_eq!(viewport.end_line, 30);

        viewport.scroll_by(-5);
        assert_eq!(viewport.start_line, 5);
        assert_eq!(viewport.end_line, 25);
    }

    #[test]
    fn test_scroll_to_line() {
        let mut viewport = Viewport::new(400.0, 20.0);
        viewport.set_total_lines(100);

        viewport.scroll_to_line(50);
        assert!(viewport.is_line_visible(50));

        viewport.scroll_to_line(5);
        assert!(viewport.is_line_visible(5));
    }

    #[test]
    fn test_scroll_position() {
        let mut viewport = Viewport::new(400.0, 20.0);
        viewport.set_total_lines(100);

        viewport.set_scroll_offset(40); // Middle
        let pos = viewport.scroll_position();
        assert!((pos - 0.5).abs() < 0.01);

        viewport.set_scroll_position(0.0);
        assert_eq!(viewport.scroll_offset, 0);

        viewport.set_scroll_position(1.0);
        assert_eq!(viewport.scroll_offset, 80); // 100 - 20
    }

    #[test]
    fn test_scrollbar_ratio() {
        let mut viewport = Viewport::new(400.0, 20.0);
        viewport.set_total_lines(100);

        let ratio = viewport.scrollbar_ratio();
        assert_eq!(ratio, 0.2); // 20 visible / 100 total
    }

    #[test]
    fn test_overscan_range() {
        let mut viewport = Viewport::new(400.0, 20.0);
        viewport.set_total_lines(100);
        viewport.set_scroll_offset(40);

        let range = viewport.overscan_range(5);
        assert_eq!(range.start, 35);
        assert_eq!(range.end, 65);
    }
}
