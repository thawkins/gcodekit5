//! Arc Expander - Task 51
//!
//! Converts G2/G3 arc commands to linear segments for controllers without arc support.

use std::f64::consts::PI;

/// Arc expansion configuration
#[derive(Debug, Clone)]
pub struct ArcExpanderConfig {
    /// Maximum segment length
    pub segment_length: f64,
    /// Number of segments
    pub num_segments: usize,
}

impl Default for ArcExpanderConfig {
    fn default() -> Self {
        Self {
            segment_length: 0.5,
            num_segments: 20,
        }
    }
}

/// Converts arc commands to line segments
#[derive(Debug)]
pub struct ArcExpander {
    config: ArcExpanderConfig,
}

impl ArcExpander {
    /// Create a new arc expander
    pub fn new(config: ArcExpanderConfig) -> Self {
        Self { config }
    }

    /// Expand an arc into line segments
    #[allow(clippy::too_many_arguments)]
    pub fn expand_arc(
        &self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        center_x: f64,
        center_y: f64,
        is_clockwise: bool,
    ) -> Vec<(f64, f64)> {
        let mut segments = Vec::new();

        // Calculate radius
        let radius_x = start_x - center_x;
        let radius_y = start_y - center_y;
        let radius = (radius_x * radius_x + radius_y * radius_y).sqrt();

        // Calculate angles
        let start_angle = radius_y.atan2(radius_x);
        let end_angle = (end_y - center_y).atan2(end_x - center_x);

        // Calculate angle delta
        let mut angle_delta = end_angle - start_angle;
        if is_clockwise && angle_delta > 0.0 {
            angle_delta -= 2.0 * PI;
        } else if !is_clockwise && angle_delta < 0.0 {
            angle_delta += 2.0 * PI;
        }

        // Generate segments
        for i in 1..=self.config.num_segments {
            let fraction = i as f64 / self.config.num_segments as f64;
            let angle = start_angle + angle_delta * fraction;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            segments.push((x, y));
        }

        segments
    }
}

impl Default for ArcExpander {
    fn default() -> Self {
        Self::new(ArcExpanderConfig::default())
    }
}
