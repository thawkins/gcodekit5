//! # Probe Routines Engine
//!
//! Generates G-code sequences for CNC touch-probe operations and computes
//! work coordinate offsets from probe trigger results.
//!
//! ## Supported Routines
//!
//! - **Z-Touch**: Surface probing with optional slow re-probe and touch-plate compensation
//! - **Edge Find**: Single-axis edge detection with backoff and re-probe
//! - **Corner Find**: Two-axis inside-corner location (X-min/Y-min, X-max/Y-max, etc.)
//! - **Bore Center**: 4-point internal diameter center calculation
//! - **Boss Center**: 4-point external diameter center calculation
//! - **Tool Length**: Tool length measurement using a setter plate
//!
//! ## Relative moves
//!
//! All routines (Z-touch, edge, corner, bore, boss, tool length) are emitted in incremental
//! mode (`G91`)
//! from wherever the probe is when the program starts, and restore `G90` at the end. The
//! generated G-code therefore does not depend on the start position passed to
//! [`ProbeRoutineEngine::generate`]; that position is only written into the header comment.
//! Position the probe before running:
//!
//! - **Z-touch**: above the surface (or touch plate), within `max_depth`.
//! - **Edge / corner**: beside the edge (inside the corner), at a safe height above the work.
//!   The probe is lowered by [`ProbeOptions::probe_depth`] with a protected `G38.3` move
//!   before probing sideways, and raised back to the start height afterwards.
//! - **Bore**: above the approximate bore centre.
//! - **Boss**: above the approximate boss centre, clear of the top of the boss.
//! - **Tool length**: with the tool tip above the tool setter, within `max_depth`. The
//!   routine does not travel to the setter itself; `plate_xy` is only written into the
//!   header as a reminder of where the setter is.
//!
//! ### Tool-setter reference
//!
//! For tool length, `plate_z` is the reference: the **machine** Z at which the reference
//! tool (the one the work offsets were set with) triggers the setter. The probe report is
//! in machine coordinates, so the tool-length offset is `trigger Z - plate_z`, ready for
//! `G43.1 Z<offset>` (positive for a tool longer than the reference). Re-measure the
//! reference tool to update `plate_z` whenever the setter is moved.
//!
//! ## Compensation
//!
//! [`ProbeOptions::tip_radius`] compensates edge and corner results (and measured bore/boss
//! diameters) for the stylus ball radius. [`ProbeOptions::plate_thickness`] compensates the
//! Z-touch result for a touch plate laid on the work. Bore and boss centres are not affected
//! by the tip radius because the probe touches both sides symmetrically.
//!
//! ## Example Usage
//!
//! ```rust
//! use gcodekit5_core::{ProbeRoutine, Position};
//! use gcodekit5_camtools::probe_routines::{ProbeOptions, ProbeRoutineEngine};
//!
//! let routine = ProbeRoutine::ZTouch {
//!     safe_height: 5.0,
//!     max_depth: 20.0,
//!     fast_feed: 100.0,
//!     slow_feed: 25.0,
//!     backoff: 2.0,
//! };
//!
//! let options = ProbeOptions { plate_thickness: 1.5, ..ProbeOptions::default() };
//! let engine = ProbeRoutineEngine::with_options(routine, options);
//! let output = engine.generate(Position::new(10.0, 20.0, 5.0))
//!     .expect("Valid parameters");
//! assert!(output.gcode.contains("G91"));
//! ```
//!
//! Depends on `gcodekit5-core` for the routine, report and position types.

use anyhow::{Result, anyhow};
use gcodekit5_core::{Corner, Position, ProbeReport, ProbeResult, ProbeRoutine};
use gcodekit5_core::{ProbeAxis as Axis, ProbeDirection as Direction};
use serde::{Deserialize, Serialize};

/// Output from a probe routine generation containing G-code and metadata.
#[derive(Debug, Clone)]
pub struct ProbeRoutineOutput {
    /// The generated G-code sequence.
    pub gcode: String,
    /// Description of the routine for UI display.
    pub description: String,
    /// Number of probe reports (`[PRB:...]`) the controller sends while running the
    /// G-code: one per `G38.x` cycle, including the protected `G38.3` descents.
    pub expected_triggers: usize,
    /// Start position given to [`ProbeRoutineEngine::generate`] (informational only).
    pub start_position: Position,
    /// The probe cycles in the order they are executed (one per `G38.x` line).
    pub cycles: Vec<ProbeCycle>,
}

/// One `G38.x` probe cycle emitted by a routine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProbeCycle {
    /// Protected `G38.3` descent before probing sideways. Its report is not a measurement
    /// (normally it does not touch anything).
    Descent,
    /// A measuring `G38.2` probe.
    Measure {
        /// Axis being probed.
        axis: Axis,
        /// `true` when probing toward the positive end of the axis.
        positive: bool,
    },
}

/// Settings that complement a [`ProbeRoutine`].
///
/// [`ProbeRoutine`] is defined in `gcodekit5-core` and has no fields for these, so they are
/// kept here and passed alongside it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProbeOptions {
    /// How far below the start height to lower the probe (with a protected `G38.3` move)
    /// before probing sideways, in mm. Used by X/Y edge, corner, bore and boss routines.
    pub probe_depth: f64,
    /// Radius of the probe tip (stylus ball), in mm. Edge and corner results are moved by
    /// this amount in the probing direction to give the surface position.
    pub tip_radius: f64,
    /// Thickness of a touch plate placed on the work for Z-touch, in mm. The work surface
    /// is this far below the trigger point.
    pub plate_thickness: f64,
    /// When set to 54..=59, the generated G-code writes the probed result into that work
    /// coordinate system with `G10 L20` immediately after the final probe. Only routines whose
    /// result needs no arithmetic on probe reports (Z-touch, edge, corner) do this; bore and
    /// boss centres must be computed from the reports with
    /// [`ProbeRoutineEngine::compute_result_with`].
    pub set_wcs: Option<u8>,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            probe_depth: defaults::PROBE_DEPTH,
            tip_radius: 0.0,
            plate_thickness: 0.0,
            set_wcs: None,
        }
    }
}

/// Engine for generating probe routine G-code and computing results.
pub struct ProbeRoutineEngine {
    routine: ProbeRoutine,
    options: ProbeOptions,
}

/// Incremental G-code builder that records each probe cycle it emits.
struct ProbeProgram {
    gcode: String,
    cycles: Vec<ProbeCycle>,
}

impl ProbeProgram {
    fn new() -> Self {
        Self {
            gcode: String::new(),
            cycles: Vec::new(),
        }
    }

    fn line(&mut self, text: &str) {
        self.gcode.push_str(text);
        self.gcode.push('\n');
    }

    /// Rapid incremental move, e.g. `rapid("X-2.000", "Back off")`.
    fn rapid(&mut self, words: &str, comment: &str) {
        self.line(&format!("G0 {} ; {}", words, comment));
    }

    /// Protected `G38.3` descent: stops without an alarm if it touches something.
    fn descend(&mut self, depth: f64, feed: f64, comment: &str) {
        if depth > 0.0 {
            self.line(&format!("G38.3 Z{:.3} F{:.1} ; {}", -depth, feed, comment));
            self.cycles.push(ProbeCycle::Descent);
        }
    }

    /// Raise back up after [`Self::descend`].
    fn raise(&mut self, depth: f64, comment: &str) {
        if depth > 0.0 {
            self.rapid(&format!("Z{:.3}", depth), comment);
        }
    }

    /// Measuring `G38.2` probe by `delta` along `axis` (sign gives the direction).
    fn probe(&mut self, axis: Axis, delta: f64, feed: f64, comment: &str) {
        self.line(&format!(
            "G38.2 {}{:.3} F{:.1} ; {}",
            axis_letter(axis),
            delta,
            feed,
            comment
        ));
        self.cycles.push(ProbeCycle::Measure {
            axis,
            positive: delta >= 0.0,
        });
    }

    /// Back off from the trigger point and re-probe slowly (incremental mode).
    fn backoff_and_reprobe(&mut self, axis: Axis, positive: bool, backoff: f64, slow_feed: f64) {
        if backoff <= 0.0 {
            return;
        }
        let dir = if positive { 1.0 } else { -1.0 };
        let letter = axis_letter(axis);
        self.rapid(
            &format!("{}{:.3}", letter, -dir * backoff),
            &format!("Back off by {:.2} mm", backoff),
        );
        self.probe(axis, dir * (backoff + 0.5), slow_feed, "Slow re-probe");
    }
}

/// G-code letter for an axis.
fn axis_letter(axis: Axis) -> &'static str {
    match axis {
        Axis::X => "X",
        Axis::Y => "Y",
        Axis::Z => "Z",
    }
}

/// G10 L20 P-number for a WCS given as 54..=59.
fn wcs_p_number(wcs: u8) -> Option<u8> {
    match wcs {
        54..=59 => Some(wcs - 53),
        _ => None,
    }
}

impl ProbeRoutineEngine {
    /// Create a new probe routine engine with default [`ProbeOptions`].
    ///
    /// # Arguments
    /// * `routine` - The routine to generate.
    pub fn new(routine: ProbeRoutine) -> Self {
        Self::with_options(routine, ProbeOptions::default())
    }

    /// Create a probe routine engine with explicit options (probe depth, compensation).
    ///
    /// # Arguments
    /// * `routine` - The routine to generate.
    /// * `options` - Probe depth, tip radius, plate thickness and optional WCS target.
    pub fn with_options(routine: ProbeRoutine, options: ProbeOptions) -> Self {
        Self { routine, options }
    }

    /// Generate the G-code sequence for this routine.
    ///
    /// # Arguments
    /// * `current_pos` - The current machine position. Only written into the header comment:
    ///   all moves except the tool-length routine are relative to wherever the probe is
    ///   when the program runs.
    ///
    /// # Returns
    /// The generated G-code and metadata, or an error if parameters are invalid.
    pub fn generate(&self, current_pos: Position) -> Result<ProbeRoutineOutput> {
        self.validate_options()?;
        let (program, description) = match &self.routine {
            ProbeRoutine::ZTouch { .. } => self.generate_z_touch(current_pos)?,
            ProbeRoutine::EdgeFind { .. } => self.generate_edge_find(current_pos)?,
            ProbeRoutine::CornerFind { .. } => self.generate_corner_find(current_pos)?,
            ProbeRoutine::BoreCenter { .. } => self.generate_bore_center(current_pos)?,
            ProbeRoutine::BossCenter { .. } => self.generate_boss_center(current_pos)?,
            ProbeRoutine::ToolLength { .. } => self.generate_tool_length(current_pos)?,
        };
        Ok(ProbeRoutineOutput {
            gcode: program.gcode,
            description,
            expected_triggers: program.cycles.len(),
            start_position: current_pos,
            cycles: program.cycles,
        })
    }

    /// Compute the final result from a probe report using default [`ProbeOptions`]
    /// (no tip-radius or plate compensation).
    ///
    /// # Arguments
    /// * `report` - The probe report containing trigger results
    ///
    /// # Returns
    /// The computed position (center, edge, or offset) or an error if computation fails.
    pub fn compute_result(report: &ProbeReport) -> Result<Position> {
        Self::compute_result_with(report, &ProbeOptions::default())
    }

    /// Compute the final result from a probe report, applying tip-radius and plate-thickness
    /// compensation.
    ///
    /// The report may contain one entry per `G38.x` cycle of the generated program
    /// (including protected descents), or only the measuring probes.
    ///
    /// # Arguments
    /// * `report` - The probe report containing trigger results (machine coordinates).
    /// * `options` - The options the routine was generated with.
    ///
    /// # Returns
    /// - Z-touch: X/Y of the trigger, Z of the work surface.
    /// - Edge: the trigger position with the probed axis moved to the surface.
    /// - Corner: X and Y of the two surfaces, Z of the X trigger.
    /// - Bore/boss: the centre (mean Z of the four triggers).
    /// - Tool length: X/Y of the trigger, Z = trigger Z - plate Z.
    pub fn compute_result_with(report: &ProbeReport, options: &ProbeOptions) -> Result<Position> {
        let measures = Self::measurements(report, options)?;
        let r = options.tip_radius as f32;
        let sign = |positive: bool| if positive { 1.0f32 } else { -1.0 };

        match &report.routine {
            ProbeRoutine::ZTouch { .. } => {
                let (_, t) = Self::last_on_axis(&measures, Axis::Z)?;
                Ok(Position::new(
                    t.position.x,
                    t.position.y,
                    t.position.z - options.plate_thickness as f32,
                ))
            }
            ProbeRoutine::EdgeFind { axis, .. } => {
                let (positive, t) = Self::last_on_axis(&measures, *axis)?;
                let mut pos = t.position;
                match axis {
                    Axis::X => pos.x += sign(positive) * r,
                    Axis::Y => pos.y += sign(positive) * r,
                    // A downward Z edge is a surface touch: the plate thickness applies.
                    Axis::Z if !positive => pos.z -= options.plate_thickness as f32,
                    Axis::Z => {}
                }
                Ok(pos)
            }
            ProbeRoutine::CornerFind { .. } => {
                let (x_pos, xt) = Self::last_on_axis(&measures, Axis::X)?;
                let (y_pos, yt) = Self::last_on_axis(&measures, Axis::Y)?;
                Ok(Position::new(
                    xt.position.x + sign(x_pos) * r,
                    yt.position.y + sign(y_pos) * r,
                    xt.position.z,
                ))
            }
            ProbeRoutine::BoreCenter { .. } | ProbeRoutine::BossCenter { .. } => {
                let [left, right, front, back] = Self::four_sides(&measures)?;
                Ok(Position::new(
                    (left.x + right.x) / 2.0,
                    (front.y + back.y) / 2.0,
                    (left.z + right.z + front.z + back.z) / 4.0,
                ))
            }
            ProbeRoutine::ToolLength { plate_z, .. } => {
                let (_, t) = measures
                    .first()
                    .copied()
                    .ok_or_else(|| anyhow!("No trigger results available"))?;
                Ok(Position::new(
                    t.position.x,
                    t.position.y,
                    t.position.z - *plate_z as f32,
                ))
            }
        }
    }

    /// Measured diameter of a bore or boss, compensated for the probe tip radius.
    ///
    /// # Arguments
    /// * `report` - Bore-center or boss-center probe report.
    /// * `options` - The options the routine was generated with.
    ///
    /// # Returns
    /// The mean of the X and Y diameters in mm, or an error for other routines.
    pub fn compute_diameter(report: &ProbeReport, options: &ProbeOptions) -> Result<f64> {
        let measures = Self::measurements(report, options)?;
        let [left, right, front, back] = Self::four_sides(&measures)?;
        let cx = (left.x + right.x) as f64 / 2.0;
        let cy = (front.y + back.y) as f64 / 2.0;
        // Each chord may be off the centre line (the second chord starts from a nominal
        // centre), so use the chord's distance from the centre: r^2 = (c/2)^2 + d^2.
        let half_x = (right.x - left.x) as f64 / 2.0;
        let half_y = (back.y - front.y) as f64 / 2.0;
        let off_x = (left.y + right.y) as f64 / 2.0 - cy;
        let off_y = (front.x + back.x) as f64 / 2.0 - cx;
        let tip_path_radius = ((half_x * half_x + off_x * off_x).sqrt()
            + (half_y * half_y + off_y * off_y).sqrt())
            / 2.0;
        match report.routine {
            // Inside a bore the tip centre stops one radius short of each wall.
            ProbeRoutine::BoreCenter { .. } => Ok(2.0 * (tip_path_radius + options.tip_radius)),
            // Around a boss the tip centre stops one radius outside each wall.
            ProbeRoutine::BossCenter { .. } => Ok(2.0 * (tip_path_radius - options.tip_radius)),
            _ => Err(anyhow!(
                "Diameter is only defined for bore and boss routines"
            )),
        }
    }

    /// Pair each measuring cycle with its probe result.
    fn measurements<'a>(
        report: &'a ProbeReport,
        options: &ProbeOptions,
    ) -> Result<Vec<(ProbeCycle, &'a ProbeResult)>> {
        let output =
            Self::with_options(report.routine.clone(), *options).generate(Position::default())?;
        let cycles = output.cycles;
        let triggers = &report.triggers;
        let measure_count = cycles
            .iter()
            .filter(|c| matches!(c, ProbeCycle::Measure { .. }))
            .count();

        let pairs: Vec<(ProbeCycle, &ProbeResult)> = if triggers.len() >= cycles.len() {
            // One report per G38.x cycle, descents included.
            cycles
                .iter()
                .copied()
                .zip(triggers.iter())
                .filter(|(c, _)| matches!(c, ProbeCycle::Measure { .. }))
                .collect()
        } else if triggers.len() >= measure_count {
            // Only the measuring probes were recorded.
            cycles
                .iter()
                .copied()
                .filter(|c| matches!(c, ProbeCycle::Measure { .. }))
                .zip(triggers.iter())
                .collect()
        } else if triggers.is_empty() {
            return Err(anyhow!("No trigger results available"));
        } else {
            return Err(anyhow!(
                "Expected {} probe results, got {}",
                measure_count,
                triggers.len()
            ));
        };

        if let Some(i) = pairs.iter().position(|(_, t)| !t.success) {
            return Err(anyhow!("Probe {} did not trigger successfully", i + 1));
        }
        Ok(pairs)
    }

    /// Last measurement along `axis`, with its probing direction.
    fn last_on_axis<'a>(
        measures: &[(ProbeCycle, &'a ProbeResult)],
        axis: Axis,
    ) -> Result<(bool, &'a ProbeResult)> {
        measures
            .iter()
            .rev()
            .find_map(|(c, t)| match c {
                ProbeCycle::Measure { axis: a, positive } if *a == axis => Some((*positive, *t)),
                _ => None,
            })
            .ok_or_else(|| anyhow!("No {}-axis probe result", axis_letter(axis)))
    }

    /// Left, right, front and back trigger positions of a bore/boss routine.
    fn four_sides(measures: &[(ProbeCycle, &ProbeResult)]) -> Result<[Position; 4]> {
        if measures.len() < 4 {
            return Err(anyhow!("Need 4 trigger points for center computation"));
        }
        Ok([
            measures[0].1.position,
            measures[1].1.position,
            measures[2].1.position,
            measures[3].1.position,
        ])
    }

    fn validate_options(&self) -> Result<()> {
        let o = &self.options;
        if !(o.probe_depth.is_finite() && o.probe_depth >= 0.0) {
            return Err(anyhow!("Probe depth must be zero or positive"));
        }
        if !(o.tip_radius.is_finite() && o.tip_radius >= 0.0) {
            return Err(anyhow!("Probe tip radius must be zero or positive"));
        }
        if !(o.plate_thickness.is_finite() && o.plate_thickness >= 0.0) {
            return Err(anyhow!("Plate thickness must be zero or positive"));
        }
        if let Some(wcs) = o.set_wcs
            && wcs_p_number(wcs).is_none()
        {
            return Err(anyhow!("Target WCS must be G54..G59"));
        }
        Ok(())
    }

    fn validate_common(safe_height: f64, fast_feed: f64, slow_feed: f64) -> Result<()> {
        if safe_height <= 0.0 {
            return Err(anyhow!("Safe height must be positive"));
        }
        if fast_feed <= 0.0 || slow_feed <= 0.0 {
            return Err(anyhow!("Feed rates must be positive"));
        }
        Ok(())
    }

    /// Header shared by the relative routines.
    fn header(program: &mut ProbeProgram, title: &str, current_pos: Position) {
        program.line(&format!("; {}", title));
        program.line(&format!(
            "; Start (informational): X{:.3} Y{:.3} Z{:.3}",
            current_pos.x, current_pos.y, current_pos.z
        ));
        program.line("; All moves are relative to the probe position when this program starts");
        program.line("");
        program.line("G21 ; Millimetres");
        program.line("G91 ; Incremental moves from the current position");
    }

    /// `G10 L20` line writing `words` into the configured WCS, if any.
    fn set_wcs_line(&self, program: &mut ProbeProgram, words: &str, comment: &str) {
        if let Some(p) = self.options.set_wcs.and_then(wcs_p_number) {
            program.line(&format!("G10 L20 P{} {} ; {}", p, words, comment));
        }
    }

    /// Distance to move away from a touched wall before the next move.
    fn clearance(backoff: f64) -> f64 {
        backoff.max(defaults::CLEARANCE)
    }

    /// Generate G-code for Z-touch (surface probe) routine.
    fn generate_z_touch(&self, current_pos: Position) -> Result<(ProbeProgram, String)> {
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

        Self::validate_common(*safe_height, *fast_feed, *slow_feed)?;
        if *max_depth <= 0.0 {
            return Err(anyhow!("Max probe depth must be positive"));
        }
        if *backoff < 0.0 {
            return Err(anyhow!("Backoff distance must be non-negative"));
        }

        let mut p = ProbeProgram::new();
        Self::header(&mut p, "Z-Touch Probe Routine", current_pos);
        p.line(&format!(
            "; Max depth: {:.2} mm, plate thickness: {:.2} mm",
            max_depth, self.options.plate_thickness
        ));
        p.probe(Axis::Z, -*max_depth, *fast_feed, "Fast probe down");
        if *backoff > 0.0 {
            p.backoff_and_reprobe(Axis::Z, false, *backoff, *slow_feed);
        }
        self.set_wcs_line(
            &mut p,
            &format!("Z{:.3}", self.options.plate_thickness),
            "Work Z0 at the surface under the plate",
        );
        p.rapid(
            &format!("Z{:.3}", safe_height),
            "Retract to safe height above the surface",
        );
        p.line("G90 ; Restore absolute moves");

        Ok((
            p,
            format!(
                "Z-touch probe at X{:.2} Y{:.2}",
                current_pos.x, current_pos.y
            ),
        ))
    }

    /// Generate G-code for edge-find routine.
    fn generate_edge_find(&self, current_pos: Position) -> Result<(ProbeProgram, String)> {
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

        Self::validate_common(*safe_height, *fast_feed, *slow_feed)?;
        if *probe_distance <= 0.0 {
            return Err(anyhow!("Probe distance must be positive"));
        }
        if *backoff < 0.0 {
            return Err(anyhow!("Backoff distance must be non-negative"));
        }

        let positive = *direction == Direction::Positive;
        let dir = if positive { 1.0 } else { -1.0 };
        let letter = axis_letter(*axis);
        let dir_name = if positive { "positive" } else { "negative" };
        let sideways = *axis != Axis::Z;
        let depth = if sideways {
            self.options.probe_depth
        } else {
            0.0
        };

        let mut p = ProbeProgram::new();
        Self::header(
            &mut p,
            &format!("Edge Find: {}-axis {}", letter, dir_name),
            current_pos,
        );
        p.line(&format!(
            "; Probe distance: {:.2} mm, backoff: {:.2} mm, probe depth: {:.2} mm",
            probe_distance, backoff, depth
        ));
        p.descend(
            depth,
            *fast_feed,
            "Lower beside the edge (stops on contact)",
        );
        p.probe(*axis, dir * *probe_distance, *fast_feed, "Fast probe");
        p.backoff_and_reprobe(*axis, positive, *backoff, *slow_feed);

        let wcs_value = match axis {
            Axis::Z if !positive => self.options.plate_thickness,
            Axis::Z => 0.0,
            _ => -dir * self.options.tip_radius,
        };
        self.set_wcs_line(
            &mut p,
            &format!("{}{:.3}", letter, wcs_value),
            "Work zero at the edge (tip compensated)",
        );
        p.rapid(
            &format!("{}{:.3}", letter, -dir * Self::clearance(*backoff)),
            "Move clear of the edge",
        );
        p.raise(depth, "Raise back to start height");
        p.line("G90 ; Restore absolute moves");

        Ok((
            p,
            format!(
                "{}-axis edge find {} from {:.2}",
                letter,
                dir_name,
                match axis {
                    Axis::X => current_pos.x,
                    Axis::Y => current_pos.y,
                    Axis::Z => current_pos.z,
                }
            ),
        ))
    }

    /// Generate G-code for corner-find routine (inside corner).
    fn generate_corner_find(&self, current_pos: Position) -> Result<(ProbeProgram, String)> {
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

        Self::validate_common(*safe_height, *fast_feed, *slow_feed)?;
        if *probe_distance <= 0.0 {
            return Err(anyhow!("Probe distance must be positive"));
        }
        if *backoff < 0.0 {
            return Err(anyhow!("Backoff distance must be non-negative"));
        }

        let (x_dir, y_dir): (f64, f64) = match corner {
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
        let depth = self.options.probe_depth;
        let clear = Self::clearance(*backoff);
        let r = self.options.tip_radius;

        let mut p = ProbeProgram::new();
        Self::header(
            &mut p,
            &format!("Corner Find: {}", corner_name),
            current_pos,
        );
        p.line(&format!(
            "; Probe distance: {:.2} mm per edge, probe depth: {:.2} mm",
            probe_distance, depth
        ));
        p.descend(
            depth,
            *fast_feed,
            "Lower inside the corner (stops on contact)",
        );

        p.line("; Probe X edge");
        p.probe(Axis::X, x_dir * *probe_distance, *fast_feed, "Fast probe");
        p.backoff_and_reprobe(Axis::X, x_dir > 0.0, *backoff, *slow_feed);
        self.set_wcs_line(
            &mut p,
            &format!("X{:.3}", -x_dir * r),
            "Work X0 at the X edge",
        );
        // Step off the X wall; the Y probe runs from here toward the Y wall.
        p.rapid(
            &format!("X{:.3}", -x_dir * clear),
            "Move clear of the X edge",
        );

        p.line("; Probe Y edge");
        p.probe(Axis::Y, y_dir * *probe_distance, *fast_feed, "Fast probe");
        p.backoff_and_reprobe(Axis::Y, y_dir > 0.0, *backoff, *slow_feed);
        self.set_wcs_line(
            &mut p,
            &format!("Y{:.3}", -y_dir * r),
            "Work Y0 at the Y edge",
        );
        p.rapid(
            &format!("Y{:.3}", -y_dir * clear),
            "Move clear of the Y edge",
        );

        p.raise(depth, "Raise back to start height");
        p.line("G90 ; Restore absolute moves");

        Ok((p, format!("Corner find at {} corner", corner_name)))
    }

    /// Generate G-code for bore-center (internal diameter) routine.
    fn generate_bore_center(&self, current_pos: Position) -> Result<(ProbeProgram, String)> {
        let ProbeRoutine::BoreCenter {
            diameter,
            safe_height,
            fast_feed,
            slow_feed,
        } = &self.routine
        else {
            return Err(anyhow!("Expected BoreCenter routine"));
        };

        if *diameter <= 0.0 {
            return Err(anyhow!("Bore diameter must be positive"));
        }
        Self::validate_common(*safe_height, *fast_feed, *slow_feed)?;

        let radius = diameter / 2.0;
        let r = self.options.tip_radius;
        if radius <= r {
            return Err(anyhow!("Bore diameter must be larger than the probe tip"));
        }
        // Allow for the start being off-centre by up to a quarter radius (at least 2 mm).
        let reach = radius + (radius * 0.25).max(2.0);
        // From a touched wall, the nominal centre is (radius - tip radius) away.
        let to_center = radius - r;
        let depth = self.options.probe_depth;

        let mut p = ProbeProgram::new();
        Self::header(
            &mut p,
            &format!("Bore Center: {:.2} mm diameter", diameter),
            current_pos,
        );
        p.line("; Start above the approximate bore centre");
        p.line("; Probing 4 points: left, right, front, back");
        p.descend(depth, *fast_feed, "Lower into the bore (stops on contact)");
        p.probe(Axis::X, -reach, *fast_feed, "Probe left wall");
        p.rapid(&format!("X{:.3}", to_center), "Back to nominal centre");
        p.probe(Axis::X, reach, *fast_feed, "Probe right wall");
        p.rapid(&format!("X{:.3}", -to_center), "Back to nominal centre");
        p.probe(Axis::Y, -reach, *fast_feed, "Probe front wall");
        p.rapid(&format!("Y{:.3}", to_center), "Back to nominal centre");
        p.probe(Axis::Y, reach, *fast_feed, "Probe back wall");
        p.rapid(&format!("Y{:.3}", -to_center), "Back to nominal centre");
        p.raise(depth, "Raise back to start height");
        p.line("G90 ; Restore absolute moves");
        if self.options.set_wcs.is_some() {
            p.line("; The centre is computed from the probe reports; it is not set here");
        }

        Ok((p, format!("Bore center: {:.2} mm diameter", diameter)))
    }

    /// Generate G-code for boss-center (external diameter) routine.
    fn generate_boss_center(&self, current_pos: Position) -> Result<(ProbeProgram, String)> {
        let ProbeRoutine::BossCenter {
            diameter,
            safe_height,
            fast_feed,
            slow_feed,
        } = &self.routine
        else {
            return Err(anyhow!("Expected BossCenter routine"));
        };

        if *diameter <= 0.0 {
            return Err(anyhow!("Boss diameter must be positive"));
        }
        Self::validate_common(*safe_height, *fast_feed, *slow_feed)?;
        let depth = self.options.probe_depth;
        if depth <= 0.0 {
            return Err(anyhow!(
                "Boss probing needs a probe depth: the probe must go below the top of the boss"
            ));
        }

        let radius = diameter / 2.0;
        let r = self.options.tip_radius;
        let clear = defaults::CLEARANCE;
        // Start position (tip centre) outside the boss: radius + tip + a gap for an
        // off-centre start. Probing inward by the same distance stops at the nominal centre.
        let approach = radius + r + (radius * 0.2).max(2.0);
        // From the clearance point beside one wall to the start point on the opposite side.
        let cross = approach + radius + r + clear;
        // From the clearance point beside a wall back to the nominal centre line.
        let to_center = radius + r + clear;

        let mut p = ProbeProgram::new();
        Self::header(
            &mut p,
            &format!("Boss Center: {:.2} mm diameter", diameter),
            current_pos,
        );
        p.line("; Start above the approximate boss centre, clear of its top");
        p.line("; Probing 4 points from outside toward center: left, right, front, back");

        p.rapid(&format!("X{:.3}", -approach), "Move to left probe position");
        p.descend(
            depth,
            *fast_feed,
            "Lower beside the boss (stops on contact)",
        );
        p.probe(
            Axis::X,
            approach,
            *fast_feed,
            "Probe left side toward centre",
        );
        p.rapid(&format!("X{:.3}", -clear), "Move clear of the boss");
        p.raise(depth, "Raise above the boss");

        p.rapid(&format!("X{:.3}", cross), "Move to right probe position");
        p.descend(
            depth,
            *fast_feed,
            "Lower beside the boss (stops on contact)",
        );
        p.probe(
            Axis::X,
            -approach,
            *fast_feed,
            "Probe right side toward centre",
        );
        p.rapid(&format!("X{:.3}", clear), "Move clear of the boss");
        p.raise(depth, "Raise above the boss");

        p.rapid(
            &format!("X{:.3} Y{:.3}", -to_center, -approach),
            "Move to front probe position",
        );
        p.descend(
            depth,
            *fast_feed,
            "Lower beside the boss (stops on contact)",
        );
        p.probe(
            Axis::Y,
            approach,
            *fast_feed,
            "Probe front side toward centre",
        );
        p.rapid(&format!("Y{:.3}", -clear), "Move clear of the boss");
        p.raise(depth, "Raise above the boss");

        p.rapid(&format!("Y{:.3}", cross), "Move to back probe position");
        p.descend(
            depth,
            *fast_feed,
            "Lower beside the boss (stops on contact)",
        );
        p.probe(
            Axis::Y,
            -approach,
            *fast_feed,
            "Probe back side toward centre",
        );
        p.rapid(&format!("Y{:.3}", clear), "Move clear of the boss");
        p.raise(depth, "Raise above the boss");

        p.rapid(&format!("Y{:.3}", -to_center), "Return to nominal centre");
        p.line("G90 ; Restore absolute moves");
        if self.options.set_wcs.is_some() {
            p.line("; The centre is computed from the probe reports; it is not set here");
        }

        Ok((p, format!("Boss center: {:.2} mm diameter", diameter)))
    }

    /// Generate G-code for the tool-length routine (relative, from above the setter).
    fn generate_tool_length(&self, current_pos: Position) -> Result<(ProbeProgram, String)> {
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

        if *safe_height <= 0.0 {
            return Err(anyhow!("Safe height must be positive"));
        }
        if *max_depth <= 0.0 {
            return Err(anyhow!("Max probe depth must be positive"));
        }
        if *feed_rate <= 0.0 {
            return Err(anyhow!("Feed rate must be positive"));
        }

        let mut p = ProbeProgram::new();
        Self::header(&mut p, "Tool Length Measurement", current_pos);
        p.line(&format!(
            "; Position the tool tip above the tool setter (at X{:.3} Y{:.3}) before running",
            plate_xy.0, plate_xy.1
        ));
        p.line(&format!(
            "; Reference: machine Z{:.3} = trigger height of the reference tool; \
             tool length offset = trigger Z - reference",
            plate_z
        ));
        p.probe(
            Axis::Z,
            -*max_depth,
            *feed_rate,
            "Probe down onto the setter",
        );
        p.rapid(
            &format!("Z{:.3}", safe_height),
            "Retract to safe height above the setter",
        );
        p.line("G90 ; Restore absolute moves");

        Ok((p, "Tool length measurement at setter plate".to_string()))
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
    /// Default depth below the start height for sideways probing (mm)
    pub const PROBE_DEPTH: f64 = 3.0;
    /// Minimum distance to move off a touched wall before the next move (mm)
    pub const CLEARANCE: f64 = 1.0;
}
