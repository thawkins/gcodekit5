//! Toolpath generation from design shapes.

mod generator;
mod segment;

use super::pocket_operations::{PocketGenerator, PocketOperation, PocketStrategy};
use crate::font_manager;
use crate::model::{
    rotate_point, DesignCircle as Circle, DesignGear, DesignLine as Line, DesignPath as PathShape,
    DesignPolygon as Polygon, DesignRectangle as Rectangle, DesignSprocket,
    DesignText as TextShape, DesignTriangle as Triangle, DesignerShape, Point,
};

pub use generator::ToolpathGenerator;
pub use segment::{ToolpathSegment, ToolpathSegmentType};

/// A complete toolpath made up of multiple segments.
#[derive(Debug, Clone)]
pub struct Toolpath {
    pub segments: Vec<ToolpathSegment>,
    pub tool_diameter: f64,
    pub depth: f64,
}

impl Toolpath {
    /// Creates a new empty toolpath.
    pub fn new(tool_diameter: f64, depth: f64) -> Self {
        debug_assert!(
            tool_diameter.is_finite() && tool_diameter > 0.0,
            "tool_diameter must be positive and finite, got {tool_diameter}"
        );
        debug_assert!(depth.is_finite(), "depth must be finite, got {depth}");
        Self {
            segments: Vec::new(),
            tool_diameter,
            depth,
        }
    }

    /// Adds a segment to the toolpath.
    pub fn add_segment(&mut self, segment: ToolpathSegment) {
        self.segments.push(segment);
    }

    /// Gets the total length of the toolpath.
    pub fn total_length(&self) -> f64 {
        self.segments
            .iter()
            .map(|seg| seg.start.distance_to(&seg.end))
            .sum()
    }
}
