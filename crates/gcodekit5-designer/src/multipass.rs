//! Multiple pass depth control for deep cuts.
//!
//! Implements depth ramping and stepping for multi-pass cutting operations,
//! enabling deep cuts while maintaining tool safety and surface finish quality.

use super::toolpath::{Toolpath, ToolpathSegment, ToolpathSegmentType};
use crate::Point;

/// Depth cutting strategy for multiple passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthStrategy {
    /// Equal depth per pass
    Constant,
    /// Ramped depth increasing from shallow to deep
    Ramped,
    /// Adaptive depth based on material removal rate
    Adaptive,
}

impl DepthStrategy {
    /// Returns the name of the strategy.
    pub fn name(&self) -> &'static str {
        match self {
            DepthStrategy::Constant => "Constant",
            DepthStrategy::Ramped => "Ramped",
            DepthStrategy::Adaptive => "Adaptive",
        }
    }
}

/// Configuration for multi-pass depth control.
#[derive(Debug, Clone)]
pub struct MultiPassConfig {
    pub total_depth: f64,
    pub max_depth_per_pass: f64,
    pub strategy: DepthStrategy,
    pub minimum_depth: f64,
    pub ramp_start_depth: f64,
}

impl MultiPassConfig {
    /// Creates a new multi-pass configuration.
    pub fn new(total_depth: f64, max_depth_per_pass: f64) -> Self {
        debug_assert!(
            max_depth_per_pass.is_finite() && max_depth_per_pass > 0.0,
            "max_depth_per_pass must be positive and finite, got {max_depth_per_pass}"
        );
        debug_assert!(
            total_depth.is_finite(),
            "total_depth must be finite, got {total_depth}"
        );
        Self {
            total_depth,
            max_depth_per_pass: if max_depth_per_pass > 0.0 {
                max_depth_per_pass
            } else {
                0.5
            },
            strategy: DepthStrategy::Constant,
            minimum_depth: 0.5,
            ramp_start_depth: 2.0,
        }
    }

    /// Sets the depth strategy.
    pub fn set_strategy(&mut self, strategy: DepthStrategy) {
        self.strategy = strategy;
    }

    /// Sets the minimum depth per pass (for ramped strategy).
    pub fn set_minimum_depth(&mut self, depth: f64) {
        debug_assert!(
            depth.is_finite() && depth > 0.0,
            "minimum_depth must be positive and finite"
        );
        self.minimum_depth = if depth > 0.0 { depth } else { 0.1 };
    }

    /// Sets the depth at which ramping starts.
    pub fn set_ramp_start_depth(&mut self, depth: f64) {
        debug_assert!(
            depth.is_finite() && depth >= 0.0,
            "ramp_start_depth must be non-negative and finite"
        );
        self.ramp_start_depth = if depth >= 0.0 { depth } else { 0.0 };
    }

    /// Calculates the number of passes needed.
    pub fn calculate_passes(&self) -> u32 {
        if self.max_depth_per_pass <= 0.0 {
            return 1;
        }
        ((self.total_depth.abs() / self.max_depth_per_pass).ceil()).max(1.0) as u32
    }

    /// Calculates the depth for a specific pass.
    pub fn calculate_pass_depth(&self, pass: u32) -> f64 {
        match self.strategy {
            DepthStrategy::Constant => {
                self.total_depth * pass as f64 / self.calculate_passes() as f64
            }
            DepthStrategy::Ramped => self.calculate_ramped_depth(pass),
            DepthStrategy::Adaptive => self.calculate_adaptive_depth(pass),
        }
    }

    /// Calculates ramped depth for a pass.
    fn calculate_ramped_depth(&self, pass: u32) -> f64 {
        let passes = self.calculate_passes() as f64;
        let pass_f = pass as f64;
        let progress = pass_f / passes;

        let start_depth = -self.minimum_depth;
        let end_depth = -self.max_depth_per_pass;

        let ramp_progress =
            (progress * passes - (passes - self.ramp_start_depth)) / self.ramp_start_depth;
        let clamped = ramp_progress.clamp(0.0, 1.0);

        start_depth + (end_depth - start_depth) * clamped
    }

    /// Calculates adaptive depth for a pass.
    fn calculate_adaptive_depth(&self, pass: u32) -> f64 {
        let passes = self.calculate_passes() as f64;
        let pass_f = pass as f64;
        let progress = pass_f / passes;

        let depth_min = -self.minimum_depth;
        let depth_max = -self.max_depth_per_pass;

        if progress < 0.3 {
            depth_min
        } else if progress < 0.7 {
            let local_progress = (progress - 0.3) / 0.4;
            depth_min + (depth_max - depth_min) * (local_progress * local_progress)
        } else {
            depth_max
        }
    }

    /// Gets all pass depths as a vector.
    pub fn get_all_pass_depths(&self) -> Vec<f64> {
        let passes = self.calculate_passes();
        (1..=passes)
            .map(|pass| self.calculate_pass_depth(pass))
            .collect()
    }
}

/// Manages multi-pass toolpath generation.
pub struct MultiPassToolpathGenerator {
    config: MultiPassConfig,
}

impl MultiPassToolpathGenerator {
    /// Creates a new multi-pass toolpath generator.
    pub fn new(config: MultiPassConfig) -> Self {
        Self { config }
    }

    /// Generates a multi-pass toolpath from a single-pass toolpath.
    pub fn generate_multi_pass(&self, base_toolpath: &Toolpath) -> Toolpath {
        let passes = self.config.calculate_passes();
        let mut multi_pass = Toolpath::new(base_toolpath.tool_diameter, self.config.total_depth);

        for pass in 1..=passes {
            let pass_depth = self.config.calculate_pass_depth(pass);

            for segment in &base_toolpath.segments {
                let adjusted_segment = self.adjust_segment_depth(segment, pass_depth);
                multi_pass.add_segment(adjusted_segment);
            }
        }

        multi_pass
    }

    /// Adjusts a toolpath segment to the specified depth.
    fn adjust_segment_depth(&self, segment: &ToolpathSegment, depth: f64) -> ToolpathSegment {
        let mut adjusted = segment.clone();
        adjusted.end.y = segment.end.y + depth - segment.end.y;
        adjusted
    }

    /// Generates depth ramp-down segments from current Z to target depth.
    pub fn generate_ramp_down(
        &self,
        start: Point,
        end_xy: Point,
        target_depth: f64,
    ) -> Vec<ToolpathSegment> {
        let mut segments = Vec::new();
        let ramp_steps = 20;

        for step in 1..=ramp_steps {
            let progress = step as f64 / ramp_steps as f64;
            let _current_depth = target_depth * progress;

            let current_point = Point::new(
                start.x + (end_xy.x - start.x) * progress,
                start.y + (end_xy.y - start.y) * progress,
            );

            let next_point = Point::new(
                start.x + (end_xy.x - start.x) * ((step + 1) as f64 / ramp_steps as f64),
                start.y + (end_xy.y - start.y) * ((step + 1) as f64 / ramp_steps as f64),
            );

            let segment = ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                current_point,
                next_point,
                100.0,
                10000,
            );
            segments.push(segment);
        }

        segments
    }

    /// Generates a continuous spiral ramp for entry to depth.
    pub fn generate_spiral_ramp(
        &self,
        center: Point,
        start_radius: f64,
        target_depth: f64,
        feed_rate: f64,
    ) -> Toolpath {
        let mut toolpath = Toolpath::new(3.175, target_depth);
        let spiral_turns = 5;
        let steps_per_turn = 36;
        let total_steps = spiral_turns * steps_per_turn;

        for step in 1..=total_steps {
            let progress = step as f64 / total_steps as f64;
            let angle = progress * spiral_turns as f64 * 2.0 * std::f64::consts::PI;
            let radius = start_radius * (1.0 - progress);
            let depth = target_depth * progress;

            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            let point = Point::new(x, y + depth);

            if step > 1 {
                let prev_step = step - 1;
                let prev_progress = prev_step as f64 / total_steps as f64;
                let prev_angle = prev_progress * spiral_turns as f64 * 2.0 * std::f64::consts::PI;
                let prev_radius = start_radius * (1.0 - prev_progress);
                let prev_depth = target_depth * prev_progress;

                let prev_x = center.x + prev_radius * prev_angle.cos();
                let prev_y = center.y + prev_radius * prev_angle.sin();
                let prev_point = Point::new(prev_x, prev_y + prev_depth);

                let segment = ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    prev_point,
                    point,
                    feed_rate,
                    10000,
                );
                toolpath.add_segment(segment);
            }
        }

        toolpath
    }
}
