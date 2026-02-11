//! Toolpath segment types and data structures.

use super::*;

/// Types of toolpath segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolpathSegmentType {
    RapidMove,
    LinearMove,
    ArcCW,
    ArcCCW,
}

/// A single segment of a toolpath.
#[derive(Debug, Clone)]
pub struct ToolpathSegment {
    pub segment_type: ToolpathSegmentType,
    pub start: Point,
    pub end: Point,
    pub center: Option<Point>,
    pub feed_rate: f64,
    pub spindle_speed: u32,
    /// Z depth for this segment (negative = below stock top)
    pub z_depth: Option<f64>,
    /// Start Z depth for this segment (if different from current Z)
    pub start_z: Option<f64>,
}

impl ToolpathSegment {
    /// Creates a new toolpath segment.
    pub fn new(
        segment_type: ToolpathSegmentType,
        start: Point,
        end: Point,
        feed_rate: f64,
        spindle_speed: u32,
    ) -> Self {
        Self {
            segment_type,
            start,
            end,
            center: None,
            feed_rate,
            spindle_speed,
            z_depth: None,
            start_z: None,
        }
    }

    /// Creates a new arc segment.
    pub fn new_arc(
        segment_type: ToolpathSegmentType,
        start: Point,
        end: Point,
        center: Point,
        feed_rate: f64,
        spindle_speed: u32,
    ) -> Self {
        Self {
            segment_type,
            start,
            end,
            center: Some(center),
            feed_rate,
            spindle_speed,
            z_depth: None,
            start_z: None,
        }
    }

    /// Set the Z depth for this segment
    pub fn with_z_depth(mut self, z: f64) -> Self {
        self.z_depth = Some(z);
        self
    }
}
