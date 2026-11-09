//! 3D Visualizer - Toolpath Rendering - Task 81
//!
//! Render G-code toolpath with color-coding by movement type,
//! current position indicator, and arc rendering support

use crate::visualizer::setup::{Color, Vector3};

const CARDINAL_ANGLES: [f32; 4] = [
    0.0,
    std::f32::consts::FRAC_PI_2,
    std::f32::consts::PI,
    std::f32::consts::FRAC_PI_2 * 3.0,
];
const ANGLE_EPSILON: f32 = 1e-4;

/// Movement type for toolpath segments
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovementType {
    /// Rapid movement (no cutting)
    Rapid,
    /// Feed movement (cutting)
    Feed,
    /// Arc movement clockwise
    ArcClockwise,
    /// Arc movement counter-clockwise
    ArcCounterClockwise,
}

/// Shared metadata for any movement segment
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementMeta {
    /// Segment classification
    pub movement_type: MovementType,
    /// Optional feed rate
    pub feed_rate: Option<f32>,
}

impl MovementMeta {
    pub fn new(movement_type: MovementType) -> Self {
        Self {
            movement_type,
            feed_rate: None,
        }
    }

    pub fn with_feed_rate(mut self, feed_rate: f32) -> Self {
        self.feed_rate = Some(feed_rate);
        self
    }
}

/// Line segment in toolpath
#[derive(Debug, Clone)]
pub struct LineSegment {
    /// Start point
    pub start: Vector3,
    /// End point
    pub end: Vector3,
    /// Movement meta
    pub meta: MovementMeta,
    /// Segment color (override default)
    pub color: Option<Color>,
}

impl LineSegment {
    /// Create new line segment
    pub fn new(start: Vector3, end: Vector3, movement_type: MovementType) -> Self {
        Self {
            start,
            end,
            meta: MovementMeta::new(movement_type),
            color: None,
        }
    }

    /// Set feed rate
    pub fn with_feed_rate(mut self, feed_rate: f32) -> Self {
        self.meta.feed_rate = Some(feed_rate);
        self
    }

    /// Set custom color
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Get segment length
    pub fn length(&self) -> f32 {
        self.start.subtract(self.end).magnitude()
    }

    /// Get segment color (using movement type if not customized)
    pub fn get_color(&self) -> Color {
        if let Some(color) = self.color {
            color
        } else {
            match self.meta.movement_type {
                MovementType::Rapid => Color::orange(),
                MovementType::Feed => Color::new(0.0, 1.0, 0.0),
                MovementType::ArcClockwise => Color::new(1.0, 0.0, 0.0),
                MovementType::ArcCounterClockwise => Color::new(1.0, 0.0, 0.0),
            }
        }
    }

    pub fn bounding_box(&self) -> (Vector3, Vector3) {
        let min = Vector3::new(
            self.start.x.min(self.end.x),
            self.start.y.min(self.end.y),
            self.start.z.min(self.end.z),
        );
        let max = Vector3::new(
            self.start.x.max(self.end.x),
            self.start.y.max(self.end.y),
            self.start.z.max(self.end.z),
        );
        (min, max)
    }
}

/// Arc segment in toolpath
#[derive(Debug, Clone)]
pub struct ArcSegment {
    /// Start point
    pub start: Vector3,
    /// End point
    pub end: Vector3,
    /// Arc center
    pub center: Vector3,
    /// Arc radius
    pub radius: f32,
    /// Movement metadata (direction + feed rate)
    pub meta: MovementMeta,
    /// Number of line segments to approximate arc
    pub segments: u32,
    /// Cached angular span for fast interpolation
    angles: ArcAngles,
}

impl ArcSegment {
    /// Create new arc segment
    pub fn new(start: Vector3, end: Vector3, center: Vector3, clockwise: bool) -> Self {
        let radius = start.subtract(center).magnitude();
        let movement_type = if clockwise {
            MovementType::ArcClockwise
        } else {
            MovementType::ArcCounterClockwise
        };
        let angles = ArcAngles::from_points(start, end, center, movement_type);
        Self {
            start,
            end,
            center,
            radius,
            meta: MovementMeta::new(movement_type),
            segments: 20,
            angles,
        }
    }

    /// Set feed rate
    pub fn with_feed_rate(mut self, feed_rate: f32) -> Self {
        self.meta.feed_rate = Some(feed_rate);
        self
    }

    /// Set number of segments for arc approximation
    pub fn with_segments(mut self, segments: u32) -> Self {
        self.segments = segments.max(2);
        self
    }

    /// Calculate arc length
    pub fn length(&self) -> f32 {
        let angle_diff = self.angles.delta;

        // Arc length = radius * angle (in radians)
        // Also account for helical movement (Z change)
        let arc_len = self.radius * angle_diff.abs();
        let z_diff = (self.end.z - self.start.z).abs();

        (arc_len.powi(2) + z_diff.powi(2)).sqrt()
    }

    /// Convert arc to line segments
    pub fn to_line_segments(&self) -> Vec<LineSegment> {
        self.line_iter().collect()
    }

    /// Iterate line segments lazily without allocations
    pub fn line_iter(&self) -> ArcLineIterator<'_> {
        ArcLineIterator::new(self)
    }

    /// Compute bounding box analytically without discretization
    pub fn bounding_box(&self) -> (Vector3, Vector3) {
        let mut min_x = self.start.x.min(self.end.x);
        let mut min_y = self.start.y.min(self.end.y);
        let mut max_x = self.start.x.max(self.end.x);
        let mut max_y = self.start.y.max(self.end.y);
        let min_z = self.start.z.min(self.end.z);
        let max_z = self.start.z.max(self.end.z);

        for angle in CARDINAL_ANGLES {
            if let Some(point) = self.point_on_arc_at_angle(angle) {
                min_x = min_x.min(point.x);
                min_y = min_y.min(point.y);
                max_x = max_x.max(point.x);
                max_y = max_y.max(point.y);
            }
        }

        (
            Vector3::new(min_x, min_y, min_z),
            Vector3::new(max_x, max_y, max_z),
        )
    }

    /// Interpolate point on arc at parameter t (0.0 to 1.0)
    pub fn interpolate_arc(&self, t: f32) -> Vector3 {
        let angle = self.calculate_arc_angle(t);
        Vector3::new(
            self.center.x + self.radius * angle.cos(),
            self.center.y + self.radius * angle.sin(),
            self.start.z + (self.end.z - self.start.z) * t,
        )
    }

    fn point_on_arc_at_angle(&self, angle: f32) -> Option<Vector3> {
        self.angles.param_for_angle(angle).map(|t| {
            Vector3::new(
                self.center.x + self.radius * angle.cos(),
                self.center.y + self.radius * angle.sin(),
                self.start.z + (self.end.z - self.start.z) * t,
            )
        })
    }

    fn calculate_arc_angle(&self, t: f32) -> f32 {
        self.angles.angle_at(t)
    }
}

#[derive(Debug, Clone, Copy)]
struct ArcAngles {
    start: f32,
    delta: f32,
}

impl ArcAngles {
    fn from_points(
        start: Vector3,
        end: Vector3,
        center: Vector3,
        movement_type: MovementType,
    ) -> Self {
        let start_dir = (start - center).normalize();
        let end_dir = (end - center).normalize();

        let angle_start = start_dir.y.atan2(start_dir.x);
        let angle_end = end_dir.y.atan2(end_dir.x);

        // Coincident start/end points denote a full circle (G-code semantics),
        // so a zero difference must become a full turn in the arc direction.
        const FULL_CIRCLE_EPSILON: f32 = 1e-6;
        let clockwise = movement_type == MovementType::ArcClockwise;
        let mut angle_diff = angle_end - angle_start;
        if clockwise && angle_diff >= -FULL_CIRCLE_EPSILON {
            angle_diff -= std::f32::consts::TAU;
        } else if !clockwise && angle_diff <= FULL_CIRCLE_EPSILON {
            angle_diff += std::f32::consts::TAU;
        }

        Self {
            start: angle_start,
            delta: angle_diff,
        }
    }

    fn angle_at(&self, t: f32) -> f32 {
        self.start + self.delta * t
    }

    fn param_for_angle(&self, angle: f32) -> Option<f32> {
        if self.delta.abs() < f32::EPSILON {
            return None;
        }

        if self.delta > 0.0 {
            let rel = normalize_positive(angle - self.start);
            if rel <= self.delta + ANGLE_EPSILON {
                Some((rel / self.delta).clamp(0.0, 1.0))
            } else {
                None
            }
        } else {
            let rel = normalize_negative(angle - self.start);
            if rel >= self.delta - ANGLE_EPSILON {
                Some((rel / self.delta).clamp(0.0, 1.0))
            } else {
                None
            }
        }
    }
}

fn normalize_positive(angle: f32) -> f32 {
    let mut value = angle % std::f32::consts::TAU;
    if value < 0.0 {
        value += std::f32::consts::TAU;
    }
    value
}

fn normalize_negative(angle: f32) -> f32 {
    let mut value = angle % std::f32::consts::TAU;
    if value > 0.0 {
        value -= std::f32::consts::TAU;
    }
    value
}

/// Iterator that lazily emits discretized line segments for an arc
pub struct ArcLineIterator<'a> {
    arc: &'a ArcSegment,
    current_point: Vector3,
    step: f32,
    index: u32,
}

impl<'a> ArcLineIterator<'a> {
    fn new(arc: &'a ArcSegment) -> Self {
        Self {
            arc,
            current_point: arc.start,
            step: 1.0 / arc.segments as f32,
            index: 0,
        }
    }
}

impl<'a> Iterator for ArcLineIterator<'a> {
    type Item = LineSegment;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.arc.segments {
            return None;
        }

        self.index += 1;
        let t = self.step * self.index as f32;
        let next_point = self.arc.interpolate_arc(t);

        let mut segment =
            LineSegment::new(self.current_point, next_point, self.arc.meta.movement_type);
        if let Some(feed_rate) = self.arc.meta.feed_rate {
            segment = segment.with_feed_rate(feed_rate);
        }

        self.current_point = next_point;
        Some(segment)
    }
}

/// Unified path segment representation
#[derive(Debug, Clone)]
pub enum PathSegment {
    /// Linear movement segment
    Line(LineSegment),
    /// Arc movement segment
    Arc(ArcSegment),
}

impl PathSegment {
    pub fn start(&self) -> Vector3 {
        match self {
            PathSegment::Line(line) => line.start,
            PathSegment::Arc(arc) => arc.start,
        }
    }

    pub fn end(&self) -> Vector3 {
        match self {
            PathSegment::Line(line) => line.end,
            PathSegment::Arc(arc) => arc.end,
        }
    }

    pub fn length(&self) -> f32 {
        match self {
            PathSegment::Line(line) => line.length(),
            PathSegment::Arc(arc) => arc.length(),
        }
    }

    pub fn movement_type(&self) -> MovementType {
        match self {
            PathSegment::Line(line) => line.meta.movement_type,
            PathSegment::Arc(arc) => arc.meta.movement_type,
        }
    }

    pub fn bounding_box(&self) -> (Vector3, Vector3) {
        match self {
            PathSegment::Line(line) => line.bounding_box(),
            PathSegment::Arc(arc) => arc.bounding_box(),
        }
    }

    pub fn as_line_segments(&self) -> Vec<LineSegment> {
        match self {
            PathSegment::Line(line) => vec![line.clone()],
            PathSegment::Arc(arc) => arc.line_iter().collect(),
        }
    }
}

/// Toolpath visualization data
#[derive(Debug, Clone)]
pub struct Toolpath {
    /// Ordered path segments (line or arc)
    pub segments: Vec<PathSegment>,
    /// Current tool position
    pub current_position: Vector3,
    /// Start position
    pub start_position: Vector3,
    /// Total path length
    pub total_length: f32,
    /// Estimated execution time (seconds)
    pub estimated_time: Option<f32>,
}

impl Toolpath {
    /// Create new toolpath
    pub fn new(start_position: Vector3) -> Self {
        Self {
            segments: Vec::new(),
            current_position: start_position,
            start_position,
            total_length: 0.0,
            estimated_time: None,
        }
    }

    /// Add line segment
    pub fn add_line_segment(&mut self, segment: LineSegment) {
        self.total_length += segment.length();
        self.current_position = segment.end;
        self.segments.push(PathSegment::Line(segment));
    }

    /// Add arc segment
    pub fn add_arc_segment(&mut self, segment: ArcSegment) {
        self.total_length += segment.length();
        self.current_position = segment.end;
        self.segments.push(PathSegment::Arc(segment));
    }

    /// Update current position
    pub fn update_current_position(&mut self, position: Vector3) {
        self.current_position = position;
    }

    /// Get all segments as line segments (arcs converted)
    pub fn get_all_line_segments(&self) -> Vec<LineSegment> {
        let mut all_segments = Vec::new();
        self.visit_line_segments(|segment| all_segments.push(segment));
        all_segments
    }

    /// Visit every line segment (arcs discretized on the fly)
    pub fn visit_line_segments<F>(&self, mut visitor: F)
    where
        F: FnMut(LineSegment),
    {
        for segment in &self.segments {
            match segment {
                PathSegment::Line(line) => visitor(line.clone()),
                PathSegment::Arc(arc) => {
                    for line in arc.line_iter() {
                        visitor(line);
                    }
                }
            }
        }
    }

    /// Calculate bounding box
    pub fn get_bounding_box(&self) -> Option<(Vector3, Vector3)> {
        let mut min: Option<Vector3> = None;
        let mut max: Option<Vector3> = None;

        for segment in &self.segments {
            let (seg_min, seg_max) = segment.bounding_box();
            min = Some(match min {
                Some(current) => Vector3::new(
                    current.x.min(seg_min.x),
                    current.y.min(seg_min.y),
                    current.z.min(seg_min.z),
                ),
                None => seg_min,
            });

            max = Some(match max {
                Some(current) => Vector3::new(
                    current.x.max(seg_max.x),
                    current.y.max(seg_max.y),
                    current.z.max(seg_max.z),
                ),
                None => seg_max,
            });
        }

        match (min, max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    /// Get center of bounding box
    pub fn get_center(&self) -> Option<Vector3> {
        self.get_bounding_box().map(|(min, max)| {
            Vector3::new(
                (min.x + max.x) / 2.0,
                (min.y + max.y) / 2.0,
                (min.z + max.z) / 2.0,
            )
        })
    }

    /// Set estimated execution time
    pub fn set_estimated_time(&mut self, time_seconds: f32) {
        self.estimated_time = Some(time_seconds.max(0.0));
    }

    /// Clear all segments
    pub fn clear(&mut self) {
        self.segments.clear();
        self.current_position = self.start_position;
        self.total_length = 0.0;
        self.estimated_time = None;
    }

    /// Get statistics
    pub fn get_statistics(&self) -> ToolpathStats {
        let mut totals = SegmentTotals::default();

        self.visit_line_segments(|segment| {
            totals.total += 1;
            match segment.meta.movement_type {
                MovementType::Rapid => totals.rapid += 1,
                MovementType::Feed => totals.feed += 1,
                _ => {}
            }
        });

        ToolpathStats {
            total_segments: totals.total,
            rapid_segments: totals.rapid,
            feed_segments: totals.feed,
            total_length: self.total_length,
            estimated_time: self.estimated_time,
        }
    }
}

#[derive(Default)]
struct SegmentTotals {
    total: usize,
    rapid: usize,
    feed: usize,
}

impl Default for Toolpath {
    fn default() -> Self {
        Self::new(Vector3::zero())
    }
}

/// Toolpath statistics
#[derive(Debug, Clone)]
pub struct ToolpathStats {
    /// Total number of segments
    pub total_segments: usize,
    /// Number of rapid segments
    pub rapid_segments: usize,
    /// Number of feed segments
    pub feed_segments: usize,
    /// Total path length
    pub total_length: f32,
    /// Estimated execution time
    pub estimated_time: Option<f32>,
}
