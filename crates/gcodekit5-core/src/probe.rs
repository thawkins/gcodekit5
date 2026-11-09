//! Probe types and result structures for CNC touch-probe operations.
//!
//! Provides the data models for probe hardware types, probe routines,
//! probe trigger results, and computed probe reports. These types are
//! used across all crates: core definitions, communication parsers,
//! CAM tool G-code generation, and UI state.

use serde::{Deserialize, Serialize};

use crate::{Position, Units};

/// Type of physical probe device connected to the controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeType {
    /// Spring-loaded stylus that closes a circuit on contact.
    TouchProbe,
    /// Fixed plate on the machine bed used to measure tool length.
    ToolLengthSetter,
}

impl std::fmt::Display for ProbeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeType::TouchProbe => write!(f, "Touch Probe"),
            ProbeType::ToolLengthSetter => write!(f, "Tool Length Setter"),
        }
    }
}

/// Result of a single `G38.x` probe cycle.
///
/// GRBL 1.1 responds with `[PRB:x,y,z:flag]` where `flag` is `1` on success
/// and `0` on failure (for `G38.3`). This struct captures the parsed
/// trigger position and whether the probe made contact.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProbeResult {
    /// Position where the probe triggered (machine coordinates).
    pub position: Position,
    /// `true` if the probe made contact, `false` if the target was reached
    /// without triggering (only possible with `G38.3` / `G38.5`).
    pub success: bool,
    /// The coordinate unit used for the reported position.
    pub unit: Units,
}

impl ProbeResult {
    /// Create a new probe result.
    pub fn new(position: Position, success: bool, unit: Units) -> Self {
        Self {
            position,
            success,
            unit,
        }
    }
}

/// Describes a multi-step probing routine.
///
/// Routines are built by the CAM tool layer and executed by the
/// communication layer. Each routine generates a sequence of `G38.x`
/// commands and produces a [`ProbeReport`] on completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProbeRoutine {
    /// Touch off the Z surface at the current XY location.
    ZTouch {
        /// Safe height to rapid above the surface (mm).
        safe_height: f64,
        /// Maximum depth to probe before giving up (mm).
        max_depth: f64,
        /// Fast feed rate for initial probe (units/min).
        fast_feed: f64,
        /// Slow feed rate for accuracy re-probe (units/min).
        slow_feed: f64,
        /// Distance to back off after initial trigger before re-probing (mm).
        backoff: f64,
    },
    /// Find an edge along a single axis.
    EdgeFind {
        /// Axis to probe (X or Y).
        axis: Axis,
        /// Direction to probe.
        direction: Direction,
        /// Safe height to rapid above the surface (mm).
        safe_height: f64,
        /// Distance to probe from the start position (mm).
        probe_distance: f64,
        /// Fast feed rate for initial probe (units/min).
        fast_feed: f64,
        /// Slow feed rate for accuracy re-probe (units/min).
        slow_feed: f64,
        /// Distance to back off after initial trigger (mm).
        backoff: f64,
    },
    /// Find an inside corner by probing two adjacent edges.
    CornerFind {
        /// Which corner to locate (e.g. X-min/Y-min).
        corner: Corner,
        /// Safe height to rapid above the surface (mm).
        safe_height: f64,
        /// Distance to probe along each edge (mm).
        probe_distance: f64,
        /// Fast feed rate for initial probe (units/min).
        fast_feed: f64,
        /// Slow feed rate for accuracy re-probe (units/min).
        slow_feed: f64,
        /// Distance to back off after initial trigger (mm).
        backoff: f64,
    },
    /// Find the centre of a circular bore (internal diameter).
    BoreCenter {
        /// Approximate bore diameter (mm) — used to calculate probe start positions.
        diameter: f64,
        /// Safe height to rapid above the surface (mm).
        safe_height: f64,
        /// Fast feed rate (units/min).
        fast_feed: f64,
        /// Slow feed rate for accuracy (units/min).
        slow_feed: f64,
    },
    /// Find the centre of a circular boss or pin (external diameter).
    BossCenter {
        /// Approximate boss diameter (mm).
        diameter: f64,
        /// Safe height to rapid above the surface (mm).
        safe_height: f64,
        /// Fast feed rate (units/min).
        fast_feed: f64,
        /// Slow feed rate for accuracy (units/min).
        slow_feed: f64,
    },
    /// Measure tool length using a fixed setter plate.
    ToolLength {
        /// XY position of the setter plate.
        plate_xy: (f64, f64),
        /// Known Z height of the setter plate surface (machine coordinates).
        plate_z: f64,
        /// Safe height to rapid above the plate (mm).
        safe_height: f64,
        /// Maximum depth to probe below plate_z (mm).
        max_depth: f64,
        /// Feed rate for the probe move (units/min).
        feed_rate: f64,
    },
}

/// Axis identifier for probe routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    /// X-axis (typically left-right).
    X,
    /// Y-axis (typically front-back).
    Y,
    /// Z-axis (typically up-down).
    Z,
}

/// Direction for probe moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Positive direction (toward max limit).
    Positive,
    /// Negative direction (toward min limit).
    Negative,
}

/// Corner identifier for corner-finding routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Corner {
    /// X-min, Y-min corner (front-left).
    XminYmin,
    /// X-max, Y-min corner (front-right).
    XmaxYmin,
    /// X-min, Y-max corner (back-left).
    XminYmax,
    /// X-max, Y-max corner (back-right).
    XmaxYmax,
}

/// Comprehensive report produced after completing a probe routine.
///
/// Contains raw trigger positions, computed offsets or centre coordinates,
/// and the suggested G-code commands to apply the results to the active
/// work coordinate system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbeReport {
    /// The routine that was executed.
    pub routine: ProbeRoutine,
    /// Individual probe trigger results (one per `G38.x` cycle).
    pub triggers: Vec<ProbeResult>,
    /// Computed offset or centre position derived from the triggers.
    pub computed: Option<Position>,
    /// Suggested `G10` command to update the active WCS.
    pub suggested_g10: Option<String>,
    /// Suggested `G92` command to set temporary origin.
    pub suggested_g92: Option<String>,
    /// Suggested `G43.1` command for tool-length offset (ToolLength only).
    pub suggested_g43_1: Option<String>,
}

impl ProbeReport {
    /// Create a new empty probe report for a given routine.
    pub fn new(routine: ProbeRoutine) -> Self {
        Self {
            routine,
            triggers: Vec::new(),
            computed: None,
            suggested_g10: None,
            suggested_g92: None,
            suggested_g43_1: None,
        }
    }

    /// Add a trigger result to the report.
    pub fn add_trigger(&mut self, result: ProbeResult) {
        self.triggers.push(result);
    }

    /// Set the computed position.
    pub fn set_computed(&mut self, pos: Position) {
        self.computed = Some(pos);
    }

    /// Build the suggested G-code commands from the computed position and routine.
    ///
    /// * `wcs` — target work coordinate system number (54-59 for G54-G59).
    pub fn build_commands(&mut self, wcs: u8) {
        let Some(pos) = self.computed else {
            return;
        };

        match &self.routine {
            ProbeRoutine::ToolLength { .. } => {
                // Tool length offset: G43.1 Z<length>
                self.suggested_g43_1 = Some(format!("G43.1 Z{:.3}", pos.z));
            }
            _ => {
                // Work coordinate offset: G10 L2 P<n> X<x> Y<y> Z<z>
                self.suggested_g10 = Some(format!(
                    "G10 L2 P{} X{:.3} Y{:.3} Z{:.3}",
                    wcs, pos.x, pos.y, pos.z
                ));
                // Temporary offset alternative
                self.suggested_g92 = Some(format!("G92 X{:.3} Y{:.3} Z{:.3}", pos.x, pos.y, pos.z));
            }
        }
    }
}

/// Pin-level probe state extracted from real-time status reports.
///
/// GRBL 1.1 includes `Pn:P` in status reports when the probe input pin is
/// active (closed). This enum tracks transitions for UI indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbePinState {
    /// Probe circuit is open (not touching).
    Open,
    /// Probe circuit is closed (touching / triggered).
    Closed,
    /// Pin state is unknown (controller doesn't report it).
    Unknown,
}

impl ProbePinState {
    /// Returns `true` if the probe pin is actively closed (touching).
    pub fn is_triggered(&self) -> bool {
        matches!(self, ProbePinState::Closed)
    }
}
