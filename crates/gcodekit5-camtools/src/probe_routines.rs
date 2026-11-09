//! # Probe Routines Engine
//!
//! Generates G-code sequences for CNC touch-probe operations and computes
//! work coordinate offsets from probe trigger results.
//!
//! ## Supported Routines
//!
//! - **Z-Touch**: Surface probing with optional slow re-probe for accuracy
//! - **Edge Find**: Single-axis edge detection with backoff and re-probe
//! - **Corner Find**: Two-axis corner location (X-min/Y-min, X-max/Y-max, etc.)
//! - **Bore Center**: 4-point internal diameter center calculation
//! - **Boss Center**: 4-point external diameter center calculation
//! - **Tool Length**: Tool length measurement using a setter plate
//!
//! ## Example Usage
//!
//! ```rust
//! use gcodekit5_core::{ProbeRoutine, Position};
//! use gcodekit5_camtools::probe_routines::{ProbeRoutineEngine, ProbeRoutineOutput};
//!
//! let routine = ProbeRoutine::ZTouch {
//!     safe_height: 5.0,
//!     max_depth: 20.0,
//!     fast_feed: 100.0,
//!     slow_feed: 25.0,
//!     backoff: 2.0,
//! };
//!
//! let engine = ProbeRoutineEngine::new(routine);
//! let output = engine.generate(Position::new(10.0, 20.0, 5.0))
//!     .expect("Valid parameters");
//!
//! println!("Generated G-code:\n{}", output.gcode);
//! ```

use anyhow::{anyhow, Result};
use gcodekit5_core::{Corner, Position, ProbeReport, ProbeRoutine};
use gcodekit5_core::{ProbeAxis as Axis, ProbeDirection as Direction};

/// Output from a probe routine generation containing G-code and metadata.
#[derive(Debug, Clone)]
pub struct ProbeRoutineOutput {
    /// The generated G-code sequence.
    pub gcode: String,
    /// Description of the routine for UI display.
    pub description: String,
    /// Expected number of probe triggers.
    pub expected_triggers: usize,
    /// Current machine position when routine was generated.
    pub start_position: Position,
}

/// Engine for generating probe routine G-code and computing results.
pub struct ProbeRoutineEngine {
    routine: ProbeRoutine,
}

impl ProbeRoutineEngine {
    /// Create a new probe routine engine with the given routine.
    pub fn new(routine: ProbeRoutine) -> Self {
        Self { routine }
    }

    /// Generate the G-code sequence for this routine.
    ///
    /// # Arguments
    /// * `current_pos` - The current machine position (used for relative calculations)
    ///
    /// # Returns
    /// The generated G-code and metadata, or an error if parameters are invalid.
    pub fn generate(&self, current_pos: Position) -> Result<ProbeRoutineOutput> {
        match &self.routine {
            ProbeRoutine::ZTouch { .. } => self.generate_z_touch(current_pos),
            ProbeRoutine::EdgeFind { .. } => self.generate_edge_find(current_pos),
            ProbeRoutine::CornerFind { .. } => self.generate_corner_find(current_pos),
            ProbeRoutine::BoreCenter { .. } => self.generate_bore_center(current_pos),
            ProbeRoutine::BossCenter { .. } => self.generate_boss_center(current_pos),
            ProbeRoutine::ToolLength { .. } => self.generate_tool_length(current_pos),
        }
    }

    /// Compute the final result from a probe report after triggers are collected.
    ///
    /// # Arguments
    /// * `report` - The probe report containing trigger results
    ///
    /// # Returns
    /// The computed position (center, edge, or offset) or an error if computation fails.
    pub fn compute_result(report: &ProbeReport) -> Result<Position> {
        match &report.routine {
            ProbeRoutine::ZTouch { backoff, .. } => Self::compute_z_touch_result(report, *backoff),
            ProbeRoutine::EdgeFind { .. } => Self::compute_edge_find_result(report),
            ProbeRoutine::CornerFind { .. } => Self::compute_corner_find_result(report),
            ProbeRoutine::BoreCenter { .. } => Self::compute_bore_center_result(report),
            ProbeRoutine::BossCenter { .. } => Self::compute_boss_center_result(report),
            ProbeRoutine::ToolLength { plate_z, .. } => {
                Self::compute_tool_length_result(report, *plate_z)
            }
        }
    }

    /// Generate G-code for Z-touch (surface probe) routine.
    fn generate_z_touch(&self, current_pos: Position) -> Result<ProbeRoutineOutput> {
        let ProbeRoutine::ZTouch {
            safe_height,
            max_depth,
            fast_feed,
            slow_feed,
            backoff,
        } = &self.routine
        else {
            return Err(anyhow!("Expected ZTouch routine"));
        };

        // Validate parameters
        if *safe_height <= 0.0 {
            return Err(anyhow!("Safe height must be positive"));
        }
        if *max_depth <= 0.0 {
            return Err(anyhow!("Max probe depth must be positive"));
        }
        if *fast_feed <= 0.0 || *slow_feed <= 0.0 {
            return Err(anyhow!("Feed rates must be positive"));
        }
        if *backoff < 0.0 {
            return Err(anyhow!("Backoff distance must be non-negative"));
        }

        let mut gcode = String::new();
        let target_z = current_pos.z - *max_depth as f32;

        // Header
        gcode.push_str("; Z-Touch Probe Routine\n");
        gcode.push_str(&format!(
            "; Start: X{:.3} Y{:.3} Z{:.3}\n",
            current_pos.x, current_pos.y, current_pos.z
        ));
        gcode.push_str(&format!("; Safe height: {:.2} mm\n", safe_height));
        gcode.push_str(&format!("; Max depth: {:.2} mm\n", max_depth));
        gcode.push('\n');

        // Move to safe height
        gcode.push_str(&format!("G0 Z{:.3} ; Move to safe height\n", safe_height));

        // Fast probe
        gcode.push_str(&format!(
            "G38.2 Z{:.3} F{:.1} ; Fast probe down\n",
            target_z, fast_feed
        ));

        // If backoff is specified, do slow re-probe for accuracy
        if *backoff > 0.0 {
            gcode.push_str("; Backoff and slow re-probe\n");
            gcode.push_str(&format!(
                "G0 Z[PRB_Z+{:.3}] ; Back off by {:.2} mm\n",
                backoff, backoff
            ));
            gcode.push_str(&format!(
                "G38.2 Z[PRB_Z-{:.3}] F{:.1} ; Slow re-probe\n",
                backoff + 0.5,
                slow_feed
            ));
        }

        // Final retract
        gcode.push_str(&format!(
            "G0 Z{:.3} ; Retract to safe height\n",
            safe_height
        ));

        Ok(ProbeRoutineOutput {
            gcode,
            description: format!(
                "Z-touch probe at X{:.2} Y{:.2}",
                current_pos.x, current_pos.y
            ),
            expected_triggers: if *backoff > 0.0 { 2 } else { 1 },
            start_position: current_pos,
        })
    }

    /// Generate G-code for edge-find routine.
    fn generate_edge_find(&self, current_pos: Position) -> Result<ProbeRoutineOutput> {
        let ProbeRoutine::EdgeFind {
            axis,
            direction,
            safe_height,
            probe_distance,
            fast_feed,
            slow_feed,
            backoff,
        } = &self.routine
        else {
            return Err(anyhow!("Expected EdgeFind routine"));
        };

        // Validate parameters
        if *safe_height <= 0.0 {
            return Err(anyhow!("Safe height must be positive"));
        }
        if *probe_distance <= 0.0 {
            return Err(anyhow!("Probe distance must be positive"));
        }
        if *fast_feed <= 0.0 || *slow_feed <= 0.0 {
            return Err(anyhow!("Feed rates must be positive"));
        }
        if *backoff < 0.0 {
            return Err(anyhow!("Backoff distance must be non-negative"));
        }

        let axis_name = match axis {
            Axis::X => "X",
            Axis::Y => "Y",
            Axis::Z => "Z",
        };

        let dir_multiplier = match direction {
            Direction::Positive => 1.0,
            Direction::Negative => -1.0,
        };

        let mut gcode = String::new();
        let probe_delta = *probe_distance * dir_multiplier;

        // Header
        gcode.push_str(&format!(
            "; Edge Find: {}-axis {}\n",
            axis_name,
            if *direction == Direction::Positive {
                "positive"
            } else {
                "negative"
            }
        ));
        gcode.push_str(&format!("; Probe distance: {:.2} mm\n", probe_distance));
        gcode.push_str(&format!("; Backoff: {:.2} mm\n", backoff));
        gcode.push('\n');

        // Move to safe height
        gcode.push_str(&format!("G0 Z{:.3} ; Move to safe height\n", safe_height));

        // Fast probe
        match axis {
            Axis::X => {
                gcode.push_str(&format!(
                    "G38.2 X{:.3} F{:.1} ; Fast probe {}\n",
                    current_pos.x + probe_delta as f32,
                    fast_feed,
                    if dir_multiplier > 0.0 {
                        "right"
                    } else {
                        "left"
                    }
                ));
            }
            Axis::Y => {
                gcode.push_str(&format!(
                    "G38.2 Y{:.3} F{:.1} ; Fast probe {}\n",
                    current_pos.y + probe_delta as f32,
                    fast_feed,
                    if dir_multiplier > 0.0 {
                        "back"
                    } else {
                        "front"
                    }
                ));
            }
            Axis::Z => {
                gcode.push_str(&format!(
                    "G38.2 Z{:.3} F{:.1} ; Fast probe down\n",
                    current_pos.z + probe_delta as f32,
                    fast_feed
                ));
            }
        }

        // Slow re-probe if backoff specified
        if *backoff > 0.0 {
            gcode.push_str("; Backoff and slow re-probe\n");
            match axis {
                Axis::X => gcode.push_str(&format!(
                    "G0 X[PRB_X{}{:.3}]\n",
                    if dir_multiplier > 0.0 { "-" } else { "+" },
                    backoff
                )),
                Axis::Y => gcode.push_str(&format!(
                    "G0 Y[PRB_Y{}{:.3}]\n",
                    if dir_multiplier > 0.0 { "-" } else { "+" },
                    backoff
                )),
                Axis::Z => gcode.push_str(&format!("G0 Z[PRB_Z+{:.3}]\n", backoff)),
            }

            match axis {
                Axis::X => gcode.push_str(&format!(
                    "G38.2 X[PRB_X{}{:.3}] F{:.1} ; Slow re-probe\n",
                    if dir_multiplier > 0.0 { "+" } else { "-" },
                    0.5,
                    slow_feed
                )),
                Axis::Y => gcode.push_str(&format!(
                    "G38.2 Y[PRB_Y{}{:.3}] F{:.1} ; Slow re-probe\n",
                    if dir_multiplier > 0.0 { "+" } else { "-" },
                    0.5,
                    slow_feed
                )),
                Axis::Z => gcode.push_str(&format!(
                    "G38.2 Z[PRB_Z-0.5] F{:.1} ; Slow re-probe\n",
                    slow_feed
                )),
            }
        }

        // Retract
        gcode.push_str(&format!(
            "G0 Z{:.3} ; Retract to safe height\n",
            safe_height
        ));

        Ok(ProbeRoutineOutput {
            gcode,
            description: format!(
                "{}-axis edge find {} from {:.2}",
                axis_name,
                if dir_multiplier > 0.0 {
                    "positive"
                } else {
                    "negative"
                },
                match axis {
                    Axis::X => current_pos.x,
                    Axis::Y => current_pos.y,
                    Axis::Z => current_pos.z,
                }
            ),
            expected_triggers: if *backoff > 0.0 { 2 } else { 1 },
            start_position: current_pos,
        })
    }

    /// Generate G-code for corner-find routine.
    fn generate_corner_find(&self, current_pos: Position) -> Result<ProbeRoutineOutput> {
        let ProbeRoutine::CornerFind {
            corner,
            safe_height,
            probe_distance,
            fast_feed,
            slow_feed,
            backoff,
        } = &self.routine
        else {
            return Err(anyhow!("Expected CornerFind routine"));
        };

        // Validate parameters
        if *safe_height <= 0.0 {
            return Err(anyhow!("Safe height must be positive"));
        }
        if *probe_distance <= 0.0 {
            return Err(anyhow!("Probe distance must be positive"));
        }
        if *fast_feed <= 0.0 || *slow_feed <= 0.0 {
            return Err(anyhow!("Feed rates must be positive"));
        }
        if *backoff < 0.0 {
            return Err(anyhow!("Backoff distance must be non-negative"));
        }

        let (x_dir, y_dir) = match corner {
            Corner::XminYmin => (-1.0, -1.0),
            Corner::XmaxYmin => (1.0, -1.0),
            Corner::XminYmax => (-1.0, 1.0),
            Corner::XmaxYmax => (1.0, 1.0),
        };

        let corner_name = match corner {
            Corner::XminYmin => "X-min/Y-min",
            Corner::XmaxYmin => "X-max/Y-min",
            Corner::XminYmax => "X-min/Y-max",
            Corner::XmaxYmax => "X-max/Y-max",
        };

        let mut gcode = String::new();

        // Header
        gcode.push_str(&format!("; Corner Find: {}\n", corner_name));
        gcode.push_str(&format!(
            "; Probe distance: {:.2} mm per edge\n",
            probe_distance
        ));
        gcode.push('\n');

        // Move to safe height
        gcode.push_str(&format!("G0 Z{:.3} ; Move to safe height\n", safe_height));

        // Probe X edge
        gcode.push_str("; Probe X edge\n");
        gcode.push_str(&format!(
            "G38.2 X{:.3} F{:.1}\n",
            current_pos.x + (*probe_distance * x_dir) as f32,
            fast_feed
        ));

        if *backoff > 0.0 {
            gcode.push_str(&format!(
                "G0 X[PRB_X{}{:.3}]\n",
                if x_dir > 0.0 { "-" } else { "+" },
                backoff
            ));
            gcode.push_str(&format!(
                "G38.2 X[PRB_X{}{:.3}] F{:.1}\n",
                if x_dir > 0.0 { "+" } else { "-" },
                0.5,
                slow_feed
            ));
        }

        // Move to probe Y edge (retract first)
        gcode.push_str(&format!("G0 Z{:.3} ; Retract\n", safe_height));
        gcode.push_str(&format!(
            "G0 Y{:.3} ; Move to Y probe position\n",
            current_pos.y + (*probe_distance * y_dir) as f32
        ));

        // Probe Y edge
        gcode.push_str("; Probe Y edge\n");
        gcode.push_str(&format!(
            "G38.2 Y{:.3} F{:.1}\n",
            current_pos.y + (*probe_distance * y_dir) as f32,
            fast_feed
        ));

        if *backoff > 0.0 {
            gcode.push_str(&format!(
                "G0 Y[PRB_Y{}{:.3}]\n",
                if y_dir > 0.0 { "-" } else { "+" },
                backoff
            ));
            gcode.push_str(&format!(
                "G38.2 Y[PRB_Y{}{:.3}] F{:.1}\n",
                if y_dir > 0.0 { "+" } else { "-" },
                0.5,
                slow_feed
            ));
        }

        // Final retract
        gcode.push_str(&format!("G0 Z{:.3} ; Final retract\n", safe_height));

        Ok(ProbeRoutineOutput {
            gcode,
            description: format!("Corner find at {} corner", corner_name),
            expected_triggers: if *backoff > 0.0 { 4 } else { 2 },
            start_position: current_pos,
        })
    }

    /// Generate G-code for bore-center (internal diameter) routine.
    fn generate_bore_center(&self, current_pos: Position) -> Result<ProbeRoutineOutput> {
        let ProbeRoutine::BoreCenter {
            diameter,
            safe_height,
            fast_feed,
            slow_feed,
        } = &self.routine
        else {
            return Err(anyhow!("Expected BoreCenter routine"));
        };

        // Validate parameters
        if *diameter <= 0.0 {
            return Err(anyhow!("Bore diameter must be positive"));
        }
        if *safe_height <= 0.0 {
            return Err(anyhow!("Safe height must be positive"));
        }
        if *fast_feed <= 0.0 || *slow_feed <= 0.0 {
            return Err(anyhow!("Feed rates must be positive"));
        }

        let radius = diameter / 2.0;
        let probe_offset = (radius * 0.8) as f32; // Probe at 80% of radius

        let mut gcode = String::new();

        // Header
        gcode.push_str(&format!("; Bore Center: {:.2} mm diameter\n", diameter));
        gcode.push_str(&format!(
            "; Approximate center: X{:.3} Y{:.3}\n",
            current_pos.x, current_pos.y
        ));
        gcode.push_str("; Probing 4 points: left, right, front, back\n");
        gcode.push('\n');

        // Move to safe height
        gcode.push_str(&format!("G0 Z{:.3} ; Move to safe height\n", safe_height));

        // 4-point probe sequence: left (X-), right (X+), front (Y-), back (Y+)
        let probe_points = [
            (current_pos.x - probe_offset, current_pos.y, "left"),
            (current_pos.x + probe_offset, current_pos.y, "right"),
            (current_pos.x, current_pos.y - probe_offset, "front"),
            (current_pos.x, current_pos.y + probe_offset, "back"),
        ];

        for (i, (x, y, name)) in probe_points.iter().enumerate() {
            if i > 0 {
                gcode.push_str(&format!("G0 Z{:.3} ; Retract\n", safe_height));
            }
            gcode.push_str(&format!("; Probe {} side\n", name));
            gcode.push_str(&format!(
                "G0 X{:.3} Y{:.3} ; Move to {} probe position\n",
                x, y, name
            ));

            if *name == "left" || *name == "right" {
                let target_x = if *name == "left" {
                    current_pos.x + probe_offset * 2.0
                } else {
                    current_pos.x - probe_offset * 2.0
                };
                gcode.push_str(&format!(
                    "G38.2 X{:.3} F{:.1} ; Probe toward center\n",
                    target_x, fast_feed
                ));
            } else {
                let target_y = if *name == "front" {
                    current_pos.y + probe_offset * 2.0
                } else {
                    current_pos.y - probe_offset * 2.0
                };
                gcode.push_str(&format!(
                    "G38.2 Y{:.3} F{:.1} ; Probe toward center\n",
                    target_y, fast_feed
                ));
            }
        }

        // Final retract
        gcode.push_str(&format!("G0 Z{:.3} ; Final retract\n", safe_height));

        Ok(ProbeRoutineOutput {
            gcode,
            description: format!("Bore center: {:.2} mm diameter", diameter),
            expected_triggers: 4,
            start_position: current_pos,
        })
    }

    /// Generate G-code for boss-center (external diameter) routine.
    fn generate_boss_center(&self, current_pos: Position) -> Result<ProbeRoutineOutput> {
        let ProbeRoutine::BossCenter {
            diameter,
            safe_height,
            fast_feed,
            slow_feed,
        } = &self.routine
        else {
            return Err(anyhow!("Expected BossCenter routine"));
        };

        // Validate parameters
        if *diameter <= 0.0 {
            return Err(anyhow!("Boss diameter must be positive"));
        }
        if *safe_height <= 0.0 {
            return Err(anyhow!("Safe height must be positive"));
        }
        if *fast_feed <= 0.0 || *slow_feed <= 0.0 {
            return Err(anyhow!("Feed rates must be positive"));
        }

        let radius = diameter / 2.0;
        let probe_offset = (radius * 1.2) as f32; // Start outside the boss

        let mut gcode = String::new();

        // Header
        gcode.push_str(&format!("; Boss Center: {:.2} mm diameter\n", diameter));
        gcode.push_str(&format!(
            "; Approximate center: X{:.3} Y{:.3}\n",
            current_pos.x, current_pos.y
        ));
        gcode.push_str("; Probing 4 points from outside: left, right, front, back\n");
        gcode.push('\n');

        // Move to safe height
        gcode.push_str(&format!("G0 Z{:.3} ; Move to safe height\n", safe_height));

        // 4-point probe sequence from outside
        let probe_points = [
            (current_pos.x - probe_offset, current_pos.y, "left"),
            (current_pos.x + probe_offset, current_pos.y, "right"),
            (current_pos.x, current_pos.y - probe_offset, "front"),
            (current_pos.x, current_pos.y + probe_offset, "back"),
        ];

        for (i, (x, y, name)) in probe_points.iter().enumerate() {
            if i > 0 {
                gcode.push_str(&format!("G0 Z{:.3} ; Retract\n", safe_height));
            }
            gcode.push_str(&format!("; Probe {} side\n", name));
            gcode.push_str(&format!(
                "G0 X{:.3} Y{:.3} ; Move to {} probe position\n",
                x, y, name
            ));

            if *name == "left" || *name == "right" {
                let target_x = if *name == "left" {
                    current_pos.x - probe_offset * 2.0
                } else {
                    current_pos.x + probe_offset * 2.0
                };
                gcode.push_str(&format!(
                    "G38.2 X{:.3} F{:.1} ; Probe outward\n",
                    target_x, fast_feed
                ));
            } else {
                let target_y = if *name == "front" {
                    current_pos.y - probe_offset * 2.0
                } else {
                    current_pos.y + probe_offset * 2.0
                };
                gcode.push_str(&format!(
                    "G38.2 Y{:.3} F{:.1} ; Probe outward\n",
                    target_y, fast_feed
                ));
            }
        }

        // Final retract
        gcode.push_str(&format!("G0 Z{:.3} ; Final retract\n", safe_height));

        Ok(ProbeRoutineOutput {
            gcode,
            description: format!("Boss center: {:.2} mm diameter", diameter),
            expected_triggers: 4,
            start_position: current_pos,
        })
    }

    /// Generate G-code for tool-length setter routine.
    fn generate_tool_length(&self, _current_pos: Position) -> Result<ProbeRoutineOutput> {
        let ProbeRoutine::ToolLength {
            plate_xy,
            plate_z,
            safe_height,
            max_depth,
            feed_rate,
        } = &self.routine
        else {
            return Err(anyhow!("Expected ToolLength routine"));
        };

        // Validate parameters
        if *safe_height <= 0.0 {
            return Err(anyhow!("Safe height must be positive"));
        }
        if *max_depth <= 0.0 {
            return Err(anyhow!("Max probe depth must be positive"));
        }
        if *feed_rate <= 0.0 {
            return Err(anyhow!("Feed rate must be positive"));
        }

        let target_z = plate_z - max_depth;

        let mut gcode = String::new();

        // Header
        gcode.push_str("; Tool Length Measurement\n");
        gcode.push_str(&format!(
            "; Setter plate position: X{:.3} Y{:.3}\n",
            plate_xy.0, plate_xy.1
        ));
        gcode.push_str(&format!("; Setter plate Z height: {:.3} mm\n", plate_z));
        gcode.push_str(&format!("; Safe height: {:.2} mm\n", safe_height));
        gcode.push('\n');

        // Move to setter plate
        gcode.push_str(&format!("G0 Z{:.3} ; Move to safe height\n", safe_height));
        gcode.push_str(&format!(
            "G0 X{:.3} Y{:.3} ; Move over setter plate\n",
            plate_xy.0, plate_xy.1
        ));

        // Probe down
        gcode.push_str(&format!(
            "G38.2 Z{:.3} F{:.1} ; Probe toward setter plate\n",
            target_z, feed_rate
        ));

        // Retract
        gcode.push_str(&format!(
            "G0 Z{:.3} ; Retract to safe height\n",
            safe_height
        ));

        Ok(ProbeRoutineOutput {
            gcode,
            description: "Tool length measurement at setter plate".to_string(),
            expected_triggers: 1,
            start_position: Position::new(
                plate_xy.0 as f32,
                plate_xy.1 as f32,
                *safe_height as f32,
            ),
        })
    }

    // Result computation methods

    /// Compute Z-touch result from trigger positions.
    fn compute_z_touch_result(report: &ProbeReport, backoff: f64) -> Result<Position> {
        let triggers = &report.triggers;

        if triggers.is_empty() {
            return Err(anyhow!("No trigger results available"));
        }

        // Use slow re-probe result if available (last trigger), otherwise first trigger
        let trigger_idx = if triggers.len() >= 2 && backoff > 0.0 {
            triggers.len() - 1 // Use last (slow) probe
        } else {
            0 // Use first (only) probe
        };

        let trigger = &triggers[trigger_idx];
        if !trigger.success {
            return Err(anyhow!("Probe did not trigger successfully"));
        }

        // Z-touch sets the surface position (Z=0 at trigger point)
        Ok(Position::new(
            trigger.position.x,
            trigger.position.y,
            0.0, // Z is set to surface
        ))
    }

    /// Compute edge-find result from trigger positions.
    fn compute_edge_find_result(report: &ProbeReport) -> Result<Position> {
        let triggers = &report.triggers;

        if triggers.is_empty() {
            return Err(anyhow!("No trigger results available"));
        }

        // Use last trigger (slow re-probe if available)
        let trigger = triggers.last().unwrap();
        if !trigger.success {
            return Err(anyhow!("Probe did not trigger successfully"));
        }

        // Return the trigger position as the edge
        Ok(trigger.position)
    }

    /// Compute corner-find result from trigger positions.
    fn compute_corner_find_result(report: &ProbeReport) -> Result<Position> {
        let triggers = &report.triggers;

        if triggers.len() < 2 {
            return Err(anyhow!("Need at least 2 triggers for corner computation"));
        }

        // Corner routine pattern: X probes first, then Y probes
        // With backoff: X fast, X slow, Y fast, Y slow (4 triggers)
        // Without backoff: X fast, Y fast (2 triggers)

        let x_idx = if triggers.len() >= 2 { 1 } else { 0 }; // Last X probe (slow if available)
        let y_idx = if triggers.len() >= 4 {
            3 // Last Y probe (slow)
        } else if triggers.len() >= 2 {
            1 // Only Y probe available
        } else {
            0
        };

        let x_trigger = &triggers[x_idx];
        let y_trigger = &triggers[y_idx];

        if !x_trigger.success || !y_trigger.success {
            return Err(anyhow!("One or more probes did not trigger successfully"));
        }

        // Corner is intersection of the two edges
        Ok(Position::new(
            x_trigger.position.x,
            y_trigger.position.y,
            x_trigger.position.z,
        ))
    }

    /// Compute bore center from 4 trigger points.
    fn compute_bore_center_result(report: &ProbeReport) -> Result<Position> {
        let triggers = &report.triggers;

        if triggers.len() < 4 {
            return Err(anyhow!("Need 4 trigger points for bore center computation"));
        }

        // Check all triggers succeeded
        for (i, trigger) in triggers.iter().enumerate() {
            if !trigger.success {
                return Err(anyhow!("Probe {} did not trigger successfully", i + 1));
            }
        }

        // Extract left/right (X) and front/back (Y) probe points
        // Assuming order: left, right, front, back
        let left = &triggers[0].position;
        let right = &triggers[1].position;
        let front = &triggers[2].position;
        let back = &triggers[3].position;

        // Compute center via chord midpoints
        let x_center = (left.x + right.x) / 2.0;
        let y_center = (front.y + back.y) / 2.0;
        let z_center = (left.z + right.z + front.z + back.z) / 4.0;

        Ok(Position::new(x_center, y_center, z_center))
    }

    /// Compute boss center from 4 trigger points.
    fn compute_boss_center_result(report: &ProbeReport) -> Result<Position> {
        // Same math as bore center - we're finding the center of the circle
        Self::compute_bore_center_result(report)
    }

    /// Compute tool length from trigger position.
    fn compute_tool_length_result(report: &ProbeReport, plate_z: f64) -> Result<Position> {
        let triggers = &report.triggers;

        if triggers.is_empty() {
            return Err(anyhow!("No trigger results available"));
        }

        let trigger = &triggers[0];
        if !trigger.success {
            return Err(anyhow!("Probe did not trigger successfully"));
        }

        // Tool length = trigger Z - plate Z
        let tool_length = trigger.position.z - plate_z as f32;

        // Return the tool length as the Z component
        Ok(Position::new(
            trigger.position.x,
            trigger.position.y,
            tool_length,
        ))
    }
}

/// Default parameters for probe routines.
pub mod defaults {
    /// Default safe height above workpiece (mm)
    pub const SAFE_HEIGHT: f64 = 5.0;
    /// Default maximum probe depth (mm)
    pub const MAX_PROBE_DEPTH: f64 = 20.0;
    /// Default fast feed rate (mm/min)
    pub const FAST_FEED: f64 = 100.0;
    /// Default slow feed rate for accuracy (mm/min)
    pub const SLOW_FEED: f64 = 25.0;
    /// Default backoff distance after initial probe (mm)
    pub const BACKOFF: f64 = 2.0;
    /// Default probe distance for edge finding (mm)
    pub const PROBE_DISTANCE: f64 = 10.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use gcodekit5_core::{ProbeResult, Units};

    #[test]
    fn test_z_touch_generation() {
        let routine = ProbeRoutine::ZTouch {
            safe_height: 5.0,
            max_depth: 20.0,
            fast_feed: 100.0,
            slow_feed: 25.0,
            backoff: 2.0,
        };

        let engine = ProbeRoutineEngine::new(routine);
        let current_pos = Position::new(10.0, 20.0, 5.0);
        let output = engine.generate(current_pos).unwrap();

        assert!(output.gcode.contains("G38.2"));
        assert!(output.gcode.contains("Z"));
        assert_eq!(output.expected_triggers, 2);
    }

    #[test]
    fn test_edge_find_generation() {
        let routine = ProbeRoutine::EdgeFind {
            axis: Axis::X,
            direction: Direction::Negative,
            safe_height: 5.0,
            probe_distance: 10.0,
            fast_feed: 100.0,
            slow_feed: 25.0,
            backoff: 2.0,
        };

        let engine = ProbeRoutineEngine::new(routine);
        let current_pos = Position::new(10.0, 20.0, 5.0);
        let output = engine.generate(current_pos).unwrap();

        assert!(output.gcode.contains("G38.2"));
        assert!(output.gcode.contains("X"));
        assert_eq!(output.expected_triggers, 2);
    }

    #[test]
    fn test_bore_center_computation() {
        // Create a simulated bore center probe report
        let routine = ProbeRoutine::BoreCenter {
            diameter: 20.0,
            safe_height: 10.0,
            fast_feed: 100.0,
            slow_feed: 25.0,
        };

        let mut report = ProbeReport::new(routine);

        // Simulate probing a 20mm bore centered at (50, 50, -5)
        // Left edge at X=40, Right edge at X=60
        // Front edge at Y=40, Back edge at Y=60
        report.add_trigger(ProbeResult::new(
            Position::new(40.0, 50.0, -5.0),
            true,
            Units::MM,
        ));
        report.add_trigger(ProbeResult::new(
            Position::new(60.0, 50.0, -5.0),
            true,
            Units::MM,
        ));
        report.add_trigger(ProbeResult::new(
            Position::new(50.0, 40.0, -5.0),
            true,
            Units::MM,
        ));
        report.add_trigger(ProbeResult::new(
            Position::new(50.0, 60.0, -5.0),
            true,
            Units::MM,
        ));

        let center = ProbeRoutineEngine::compute_result(&report).unwrap();

        assert!((center.x - 50.0).abs() < 0.001);
        assert!((center.y - 50.0).abs() < 0.001);
        assert!((center.z - (-5.0)).abs() < 0.001);
    }

    #[test]
    fn test_corner_find_computation() {
        let routine = ProbeRoutine::CornerFind {
            corner: Corner::XminYmin,
            safe_height: 10.0,
            probe_distance: 10.0,
            fast_feed: 100.0,
            slow_feed: 25.0,
            backoff: 2.0,
        };

        let mut report = ProbeReport::new(routine);

        // Simulate finding a corner at (10, 20)
        // X edge found at X=10, Y edge found at Y=20
        report.add_trigger(ProbeResult::new(
            Position::new(10.0, 25.0, -5.0),
            true,
            Units::MM,
        ));
        report.add_trigger(ProbeResult::new(
            Position::new(10.0, 25.0, -5.0),
            true,
            Units::MM,
        )); // backoff re-probe
        report.add_trigger(ProbeResult::new(
            Position::new(15.0, 20.0, -5.0),
            true,
            Units::MM,
        ));
        report.add_trigger(ProbeResult::new(
            Position::new(15.0, 20.0, -5.0),
            true,
            Units::MM,
        )); // backoff re-probe

        let corner = ProbeRoutineEngine::compute_result(&report).unwrap();

        assert!((corner.x - 10.0).abs() < 0.001);
        assert!((corner.y - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_tool_length_computation() {
        let routine = ProbeRoutine::ToolLength {
            plate_xy: (100.0, 100.0),
            plate_z: -50.0,
            safe_height: 10.0,
            max_depth: 20.0,
            feed_rate: 100.0,
        };

        let mut report = ProbeReport::new(routine);

        // Tool triggered at Z=-65 (15mm below plate)
        report.add_trigger(ProbeResult::new(
            Position::new(100.0, 100.0, -65.0),
            true,
            Units::MM,
        ));

        let result = ProbeRoutineEngine::compute_result(&report).unwrap();

        // Tool length = trigger_z - plate_z = -65 - (-50) = -15
        // But we store as positive value
        assert!((result.z - (-15.0)).abs() < 0.001);
    }

    #[test]
    fn test_parameter_validation() {
        let routine = ProbeRoutine::ZTouch {
            safe_height: 0.0, // Invalid
            max_depth: 20.0,
            fast_feed: 100.0,
            slow_feed: 25.0,
            backoff: 2.0,
        };

        let engine = ProbeRoutineEngine::new(routine);
        let result = engine.generate(Position::new(0.0, 0.0, 0.0));

        assert!(result.is_err());
    }
}
