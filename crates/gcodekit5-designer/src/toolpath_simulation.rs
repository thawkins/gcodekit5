//! Toolpath simulation and visualization for CAM operations.
//!
//! Provides simulation capabilities for previewing toolpath execution,
//! estimating machining time, and detecting potential collisions.

use super::toolpath::{Toolpath, ToolpathSegmentType};
use crate::model::Point;

/// Simulation state of a toolpath.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationState {
    Idle,
    Running,
    Paused,
    Complete,
    Error,
}

impl SimulationState {
    /// Returns the name of the state.
    pub fn name(&self) -> &'static str {
        match self {
            SimulationState::Idle => "Idle",
            SimulationState::Running => "Running",
            SimulationState::Paused => "Paused",
            SimulationState::Complete => "Complete",
            SimulationState::Error => "Error",
        }
    }
}

/// Represents a simulated tool position during execution.
#[derive(Debug, Clone)]
pub struct ToolPosition {
    pub position: Point,
    pub depth: f64,
    pub spindle_speed: u32,
    pub feed_rate: f64,
    pub timestamp: f64,
}

impl ToolPosition {
    /// Creates a new tool position.
    pub fn new(
        position: Point,
        depth: f64,
        spindle_speed: u32,
        feed_rate: f64,
        timestamp: f64,
    ) -> Self {
        Self {
            position,
            depth,
            spindle_speed,
            feed_rate,
            timestamp,
        }
    }
}

/// Material removal visualization data.
#[derive(Debug, Clone)]
pub struct MaterialRemovalInfo {
    pub total_volume: f64,
    pub volume_removed: f64,
    pub percentage_complete: f64,
}

impl MaterialRemovalInfo {
    /// Creates new material removal info.
    pub fn new(total_volume: f64) -> Self {
        Self {
            total_volume,
            volume_removed: 0.0,
            percentage_complete: 0.0,
        }
    }

    /// Updates the removed volume.
    pub fn update(&mut self, volume: f64) {
        self.volume_removed = volume.min(self.total_volume);
        self.percentage_complete = (self.volume_removed / self.total_volume * 100.0).min(100.0);
    }
}

/// Toolpath simulator for visualization and analysis.
pub struct ToolpathSimulator {
    toolpath: Toolpath,
    simulation_state: SimulationState,
    current_segment: usize,
    current_time: f64,
    tool_positions: Vec<ToolPosition>,
}

impl ToolpathSimulator {
    /// Creates a new toolpath simulator.
    pub fn new(toolpath: Toolpath) -> Self {
        Self {
            toolpath,
            simulation_state: SimulationState::Idle,
            current_segment: 0,
            current_time: 0.0,
            tool_positions: Vec::new(),
        }
    }

    /// Starts the simulation.
    pub fn start(&mut self) {
        self.simulation_state = SimulationState::Running;
        self.current_segment = 0;
        self.current_time = 0.0;
    }

    /// Pauses the simulation.
    pub fn pause(&mut self) {
        self.simulation_state = SimulationState::Paused;
    }

    /// Resumes the simulation.
    pub fn resume(&mut self) {
        if self.simulation_state == SimulationState::Paused {
            self.simulation_state = SimulationState::Running;
        }
    }

    /// Resets the simulation.
    pub fn reset(&mut self) {
        self.simulation_state = SimulationState::Idle;
        self.current_segment = 0;
        self.current_time = 0.0;
        self.tool_positions.clear();
    }

    /// Steps through the simulation.
    pub fn step(&mut self) {
        if self.simulation_state != SimulationState::Running {
            return;
        }

        if self.current_segment >= self.toolpath.segments.len() {
            self.simulation_state = SimulationState::Complete;
            return;
        }

        let segment = &self.toolpath.segments[self.current_segment];
        let distance = segment.start.distance_to(&segment.end);
        let time_seconds = distance / segment.feed_rate * 60.0;

        let tool_pos = ToolPosition::new(
            segment.end,
            segment.end.y,
            segment.spindle_speed,
            segment.feed_rate,
            self.current_time + time_seconds,
        );
        self.tool_positions.push(tool_pos);

        self.current_time += time_seconds;
        self.current_segment += 1;
    }

    /// Simulates the entire toolpath.
    pub fn simulate_all(&mut self) {
        self.reset();
        self.start();

        while self.simulation_state == SimulationState::Running {
            self.step();
        }
    }

    /// Gets the current simulation state.
    pub fn get_state(&self) -> SimulationState {
        self.simulation_state
    }

    /// Gets the current time in seconds.
    pub fn get_current_time(&self) -> f64 {
        self.current_time
    }

    /// Gets the estimated total time in seconds.
    pub fn get_estimated_time(&self) -> f64 {
        let mut total_time = 0.0;
        for segment in &self.toolpath.segments {
            let distance = segment.start.distance_to(&segment.end);
            total_time += distance / segment.feed_rate * 60.0;
        }
        total_time
    }

    /// Gets all recorded tool positions.
    pub fn get_tool_positions(&self) -> &[ToolPosition] {
        &self.tool_positions
    }

    /// Gets the percentage of simulation complete.
    pub fn get_progress_percentage(&self) -> f64 {
        if self.toolpath.segments.is_empty() {
            return 100.0;
        }
        (self.current_segment as f64 / self.toolpath.segments.len() as f64) * 100.0
    }
}

/// Analyzes toolpath for optimization opportunities.
pub struct ToolpathAnalyzer {
    toolpath: Toolpath,
}

impl ToolpathAnalyzer {
    /// Creates a new toolpath analyzer.
    pub fn new(toolpath: Toolpath) -> Self {
        Self { toolpath }
    }

    /// Calculates the total toolpath length.
    pub fn calculate_total_length(&self) -> f64 {
        self.toolpath
            .segments
            .iter()
            .map(|seg| seg.start.distance_to(&seg.end))
            .sum()
    }

    /// Calculates the total machining time.
    pub fn calculate_machining_time(&self) -> f64 {
        self.toolpath
            .segments
            .iter()
            .map(|seg| {
                let distance = seg.start.distance_to(&seg.end);
                distance / seg.feed_rate * 60.0
            })
            .sum()
    }

    /// Counts segments by type.
    pub fn count_segments_by_type(&self) -> (u32, u32, u32) {
        let mut rapid = 0;
        let mut linear = 0;
        let mut arc = 0;

        for segment in &self.toolpath.segments {
            match segment.segment_type {
                ToolpathSegmentType::RapidMove => rapid += 1,
                ToolpathSegmentType::LinearMove => linear += 1,
                ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => arc += 1,
            }
        }

        (rapid, linear, arc)
    }

    /// Estimates material removal volume.
    pub fn estimate_volume_removed(&self) -> f64 {
        let area = std::f64::consts::PI * (self.toolpath.tool_diameter / 2.0).powi(2);
        let length = self.calculate_total_length();
        let depth = self.toolpath.depth.abs();
        area * length * depth
    }

    /// Detects rapid speed inefficiencies.
    pub fn detect_rapid_inefficiencies(&self) -> Vec<(usize, f64)> {
        let mut inefficiencies = Vec::new();

        for (i, segment) in self.toolpath.segments.iter().enumerate() {
            if segment.segment_type == ToolpathSegmentType::RapidMove {
                let distance = segment.start.distance_to(&segment.end);
                let time = distance / 5000.0;
                if time > 0.1 {
                    inefficiencies.push((i, time));
                }
            }
        }

        inefficiencies
    }

    /// Calculates tool wear estimate based on cutting time.
    pub fn estimate_tool_wear(&self, tool_life_hours: f64) -> f64 {
        let cutting_time = self.calculate_machining_time() / 3600.0;
        ((cutting_time / tool_life_hours) * 100.0).min(100.0)
    }

    /// Analyzes surface finish quality based on feed rate and spindle speed.
    pub fn analyze_surface_finish(&self) -> String {
        let avg_feed_rate = self.calculate_average_feed_rate();

        if avg_feed_rate < 50.0 {
            "Excellent - Very smooth finish expected".to_string()
        } else if avg_feed_rate < 150.0 {
            "Good - Smooth finish expected".to_string()
        } else if avg_feed_rate < 300.0 {
            "Fair - Acceptable finish".to_string()
        } else {
            "Rough - Surface finishing may be needed".to_string()
        }
    }

    /// Calculates average feed rate across the toolpath.
    fn calculate_average_feed_rate(&self) -> f64 {
        if self.toolpath.segments.is_empty() {
            return 0.0;
        }

        let total: f64 = self.toolpath.segments.iter().map(|s| s.feed_rate).sum();
        total / self.toolpath.segments.len() as f64
    }
}
