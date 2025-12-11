//! Converter utilities for toolpath segments
//!
//! This module provides utilities to convert coordinates and create toolpath segments
//! for stock removal simulation.

use crate::toolpath::{ToolpathSegment, ToolpathSegmentType};
use crate::shapes::Point;

/// Convert 3D point to 2D designer Point
#[inline]
pub fn point_to_2d(x: f32, y: f32) -> Point {
    Point {
        x: x as f64,
        y: y as f64,
    }
}

/// Create a linear toolpath segment
pub fn create_linear_segment(
    from_x: f32,
    from_y: f32,
    from_z: f32,
    to_x: f32,
    to_y: f32,
    to_z: f32,
    rapid: bool,
    feed_rate: f64,
    spindle_speed: u32,
) -> ToolpathSegment {
    let start = point_to_2d(from_x, from_y);
    let end = point_to_2d(to_x, to_y);
    
    let segment_type = if rapid {
        ToolpathSegmentType::RapidMove
    } else {
        ToolpathSegmentType::LinearMove
    };
    
    let mut segment = ToolpathSegment::new(
        segment_type,
        start,
        end,
        feed_rate,
        spindle_speed,
    );
    segment.z_depth = Some(to_z as f64);
    segment
}

/// Create an arc toolpath segment
pub fn create_arc_segment(
    from_x: f32,
    from_y: f32,
    from_z: f32,
    to_x: f32,
    to_y: f32,
    to_z: f32,
    center_x: f32,
    center_y: f32,
    clockwise: bool,
    feed_rate: f64,
    spindle_speed: u32,
) -> ToolpathSegment {
    let start = point_to_2d(from_x, from_y);
    let end = point_to_2d(to_x, to_y);
    let arc_center = point_to_2d(center_x, center_y);
    
    let segment_type = if clockwise {
        ToolpathSegmentType::ArcCW
    } else {
        ToolpathSegmentType::ArcCCW
    };
    
    let mut segment = ToolpathSegment::new_arc(
        segment_type,
        start,
        end,
        arc_center,
        feed_rate,
        spindle_speed,
    );
    segment.z_depth = Some(to_z as f64);
    segment
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_point_conversion() {
        let p2d = point_to_2d(10.5, 20.3);
        assert_eq!(p2d.x, 10.5);
        assert_eq!(p2d.y, 20.3);
    }
    
    #[test]
    fn test_linear_segment_creation() {
        let segment = create_linear_segment(
            0.0, 0.0, 0.0,
            10.0, 10.0, -5.0,
            false,
            100.0,
            3000,
        );
        
        assert_eq!(segment.segment_type, ToolpathSegmentType::LinearMove);
        assert_eq!(segment.z_depth, Some(-5.0));
        assert_eq!(segment.end.x, 10.0);
        assert_eq!(segment.end.y, 10.0);
    }
    
    #[test]
    fn test_rapid_segment_creation() {
        let segment = create_linear_segment(
            0.0, 0.0, 5.0,
            10.0, 10.0, 5.0,
            true,
            100.0,
            3000,
        );
        
        assert_eq!(segment.segment_type, ToolpathSegmentType::RapidMove);
    }
    
    #[test]
    fn test_arc_segment_creation() {
        let segment = create_arc_segment(
            0.0, 0.0, -5.0,
            10.0, 0.0, -5.0,
            5.0, 0.0,
            true,
            100.0,
            3000,
        );
        
        assert_eq!(segment.segment_type, ToolpathSegmentType::ArcCW);
        assert!(segment.center.is_some());
        assert_eq!(segment.z_depth, Some(-5.0));
    }
}
