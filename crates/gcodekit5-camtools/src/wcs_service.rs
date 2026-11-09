//! # WCS Offset Service
//!
//! Provides automatic work coordinate system (WCS) updates from probe results.
//! Subscribes to probe report events and generates appropriate G-code commands
//! to update G54-G59 work offsets, G92 temporary offsets, or G43.1 tool-length offsets.
//!
//! ## Features
//!
//! - Auto-update WCS (G54-G59) from probe results
//! - Generate G10 L2 P<n> commands for persistent offsets
//! - Generate G92 commands for temporary offsets (needs the current machine position, see
//!   [`WcsUpdateService::set_machine_position`])
//! - Generate G43.1 commands for tool-length offsets
//! - Preview mode for user confirmation
//! - Persistent storage of last probe results per WCS
//!
//! ## Example
//!
//! ```rust
//! use gcodekit5_camtools::wcs_service::{WcsUpdateService, WcsUpdateConfig};
//!
//! let config = WcsUpdateConfig {
//!     auto_update_wcs: false, // Require user confirmation
//!     target_wcs: 54, // G54
//!     ..Default::default()
//! };
//!
//! let service = WcsUpdateService::new(config);
//! ```

use crate::probe_routines::{ProbeOptions, ProbeRoutineEngine};
use anyhow::{anyhow, Result};
use gcodekit5_core::{Position, ProbeReport, ProbeRoutine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for WCS update behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WcsUpdateConfig {
    /// Automatically update WCS without user confirmation.
    pub auto_update_wcs: bool,
    /// Target WCS number (54-59 for G54-G59, 0 for G92).
    pub target_wcs: u8,
    /// Use G92 temporary offset instead of G10 persistent offset.
    ///
    /// `G92 X<v>` declares that the *current* position is X=v, so the value depends on where
    /// the machine is when the command runs. The service therefore needs the current machine
    /// position ([`WcsUpdateService::set_machine_position`]) to make the probed point the
    /// work zero.
    pub use_temporary_offset: bool,
    /// Apply tool-length offset via G43.1 for ToolLength routines.
    pub apply_tool_length: bool,
    /// Safe height to retract after probe (mm).
    pub safe_height: f64,
    /// Preview changes before applying (when auto_update_wcs is false).
    pub preview_before_apply: bool,
    /// Probe depth, tip radius and plate thickness used when the routine was generated.
    #[serde(default)]
    pub probe_options: ProbeOptions,
}

impl Default for WcsUpdateConfig {
    fn default() -> Self {
        Self {
            auto_update_wcs: false,
            target_wcs: 54, // G54 is most common
            use_temporary_offset: false,
            apply_tool_length: true,
            safe_height: 5.0,
            preview_before_apply: true,
            probe_options: ProbeOptions::default(),
        }
    }
}

/// Result of a WCS update operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WcsUpdateResult {
    /// Whether the update was applied.
    pub applied: bool,
    /// The G-code command that was (or would be) sent.
    pub command: String,
    /// Human-readable description of the update.
    pub description: String,
    /// Target WCS number.
    pub target_wcs: u8,
    /// Computed offset position.
    pub computed_position: Option<Position>,
    /// Previous offset (if known).
    pub previous_offset: Option<Position>,
}

/// Service for updating work coordinate systems from probe results.
pub struct WcsUpdateService {
    config: WcsUpdateConfig,
    /// Last probe results stored per WCS (54-59).
    last_results: HashMap<u8, ProbeReport>,
    /// Last probe results stored per routine type.
    last_results_by_routine: HashMap<String, ProbeReport>,
    /// Current machine position, needed to express a probed point as a G92 offset.
    machine_position: Option<Position>,
}

impl WcsUpdateService {
    /// Create a new WCS update service with the given configuration.
    pub fn new(config: WcsUpdateConfig) -> Self {
        Self {
            config,
            last_results: HashMap::new(),
            last_results_by_routine: HashMap::new(),
            machine_position: None,
        }
    }

    /// Record the machine position (machine coordinates, mm) at which the update command
    /// will be sent. Required for G92 temporary offsets.
    ///
    /// # Arguments
    /// * `position` - Current machine position as reported by the controller.
    pub fn set_machine_position(&mut self, position: Position) {
        self.machine_position = Some(position);
    }

    /// Get the current configuration.
    pub fn config(&self) -> &WcsUpdateConfig {
        &self.config
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: WcsUpdateConfig) {
        self.config = config;
    }

    /// Process a probe report and generate WCS update commands.
    ///
    /// This method analyzes the probe report, computes the appropriate
    /// offset/center position, and generates the G-code command to apply it.
    ///
    /// # Arguments
    /// * `report` - The probe report containing trigger results and routine info
    ///
    /// # Returns
    /// The WCS update result containing the generated command and metadata.
    pub fn process_probe_report(&mut self, report: &ProbeReport) -> Result<WcsUpdateResult> {
        // Compute the result position from the report
        let computed_pos = self.compute_position(report)?;

        // Store the report for persistence
        self.store_probe_result(report, computed_pos);

        // Build the appropriate G-code command
        let (command, description) = self.build_command(report, computed_pos)?;

        // Determine if we should auto-apply
        let applied = self.config.auto_update_wcs;

        Ok(WcsUpdateResult {
            applied,
            command: command.clone(),
            description,
            target_wcs: self.config.target_wcs,
            computed_position: Some(computed_pos),
            previous_offset: None, // Would be populated from machine state
        })
    }

    /// Generate an offset preview for user confirmation.
    ///
    /// # Arguments
    /// * `report` - The probe report to preview
    ///
    /// # Returns
    /// A human-readable preview string describing the proposed offset.
    pub fn generate_preview(&self, report: &ProbeReport) -> Result<String> {
        let computed_pos = self.compute_position(report)?;
        let wcs_name = self.wcs_name(self.config.target_wcs);

        let preview = match &report.routine {
            ProbeRoutine::ZTouch { .. } => {
                format!(
                    "Probe found Z surface at {:.3} mm.\n\
                     Set {} Z = 0 at this position?",
                    computed_pos.z, wcs_name
                )
            }
            ProbeRoutine::EdgeFind {
                axis, direction, ..
            } => {
                let axis_name = format!("{:?}", axis);
                let dir_name = if *direction == gcodekit5_core::ProbeDirection::Positive {
                    "positive"
                } else {
                    "negative"
                };
                format!(
                    "Probe found {}-axis {} edge at {:.3} mm.\n\
                                       Set {} {} = 0?",
                    axis_name,
                    dir_name,
                    match axis {
                        gcodekit5_core::ProbeAxis::X => computed_pos.x,
                        gcodekit5_core::ProbeAxis::Y => computed_pos.y,
                        gcodekit5_core::ProbeAxis::Z => computed_pos.z,
                    },
                    wcs_name,
                    axis_name
                )
            }
            ProbeRoutine::CornerFind { .. } => {
                format!(
                    "Probe found corner at ({:.3}, {:.3}).\n\
                     Set {} X={:.3}, Y={:.3}?",
                    computed_pos.x, computed_pos.y, wcs_name, computed_pos.x, computed_pos.y
                )
            }
            ProbeRoutine::BoreCenter { diameter, .. } => {
                format!(
                    "Probe found bore center at ({:.3}, {:.3}).\n\
                     Diameter: {:.2} mm.\n\
                     Set {} X={:.3}, Y={:.3}?",
                    computed_pos.x,
                    computed_pos.y,
                    diameter,
                    wcs_name,
                    computed_pos.x,
                    computed_pos.y
                )
            }
            ProbeRoutine::BossCenter { diameter, .. } => {
                format!(
                    "Probe found boss center at ({:.3}, {:.3}).\n\
                     Diameter: {:.2} mm.\n\
                     Set {} X={:.3}, Y={:.3}?",
                    computed_pos.x,
                    computed_pos.y,
                    diameter,
                    wcs_name,
                    computed_pos.x,
                    computed_pos.y
                )
            }
            ProbeRoutine::ToolLength { plate_z, .. } => {
                let tool_length = computed_pos.z;
                format!(
                    "Tool length measured: {:.3} mm (plate Z={:.3}).\n\
                     Apply G43.1 Z{:.3}?",
                    tool_length, plate_z, tool_length
                )
            }
        };

        Ok(preview)
    }

    /// Build the G-code command to apply the computed offset.
    fn build_command(
        &self,
        report: &ProbeReport,
        computed_pos: Position,
    ) -> Result<(String, String)> {
        let wcs = self.config.target_wcs;

        match &report.routine {
            ProbeRoutine::ToolLength { .. } if self.config.apply_tool_length => {
                // Tool length offset: G43.1 Z<length>
                let command = format!("G43.1 Z{:.3}", computed_pos.z);
                let description = format!("Apply tool-length offset: {:.3} mm", computed_pos.z);
                Ok((command, description))
            }
            ProbeRoutine::ToolLength { .. } => Err(anyhow!(
                "A tool-length result can only be applied as a G43.1 tool-length offset"
            )),
            _ if self.config.use_temporary_offset => {
                // `G92 X<v>` makes the *current* position read X=v. For the probed point p to
                // read 0, the current position must read (current - p); this is independent
                // of the active G54..G59 offset and of any earlier G92 offset.
                let current = self.machine_position.ok_or_else(|| {
                    anyhow!("G92 needs the current machine position (set_machine_position)")
                })?;
                let relative = Position::new(
                    current.x - computed_pos.x,
                    current.y - computed_pos.y,
                    current.z - computed_pos.z,
                );
                let command = format!("G92 {}", Self::axis_words(&report.routine, relative));
                let description = format!(
                    "Set temporary origin (G92) at the probed point ({:.3}, {:.3}, {:.3})",
                    computed_pos.x, computed_pos.y, computed_pos.z
                );
                Ok((command, description))
            }
            _ => {
                // Persistent WCS offset: G10 L2 P<n> with only the axes this routine measured.
                // GRBL/LinuxCNC number coordinate systems P1..P6 for G54..G59 (P0 = active).
                let p_number = match wcs {
                    54..=59 => wcs - 53,
                    other => other,
                };
                let command = format!(
                    "G10 L2 P{} {}",
                    p_number,
                    Self::axis_words(&report.routine, computed_pos)
                );
                let description = format!(
                    "Set {} offset to ({:.3}, {:.3}, {:.3})",
                    self.wcs_name(wcs),
                    computed_pos.x,
                    computed_pos.y,
                    computed_pos.z
                );
                Ok((command, description))
            }
        }
    }

    /// Axis words for the axes a routine actually measures.
    ///
    /// Writing unmeasured axes would clobber existing offsets (e.g. a Z-touch must not
    /// move the X/Y origin, and an X edge find must not touch Y or Z).
    fn axis_words(routine: &ProbeRoutine, pos: Position) -> String {
        match routine {
            ProbeRoutine::ZTouch { .. } | ProbeRoutine::ToolLength { .. } => {
                format!("Z{:.3}", pos.z)
            }
            ProbeRoutine::EdgeFind { axis, .. } => match axis {
                gcodekit5_core::ProbeAxis::X => format!("X{:.3}", pos.x),
                gcodekit5_core::ProbeAxis::Y => format!("Y{:.3}", pos.y),
                gcodekit5_core::ProbeAxis::Z => format!("Z{:.3}", pos.z),
            },
            ProbeRoutine::CornerFind { .. }
            | ProbeRoutine::BoreCenter { .. }
            | ProbeRoutine::BossCenter { .. } => format!("X{:.3} Y{:.3}", pos.x, pos.y),
        }
    }

    /// Compute the position from a probe report (machine coordinates, compensated for the
    /// probe tip radius and touch-plate thickness from the configuration).
    fn compute_position(&self, report: &ProbeReport) -> Result<Position> {
        ProbeRoutineEngine::compute_result_with(report, &self.config.probe_options)
    }

    /// Store a probe result for persistence.
    fn store_probe_result(&mut self, report: &ProbeReport, _computed_pos: Position) {
        let wcs = self.config.target_wcs;
        self.last_results.insert(wcs, report.clone());

        // Also store by routine type for easier retrieval
        let routine_key = self.routine_key(&report.routine);
        self.last_results_by_routine
            .insert(routine_key, report.clone());
    }

    /// Get the last probe result for a specific WCS.
    pub fn get_last_result_for_wcs(&self, wcs: u8) -> Option<&ProbeReport> {
        self.last_results.get(&wcs)
    }

    /// Get the last probe result for a specific routine type.
    pub fn get_last_result_for_routine(&self, routine: &ProbeRoutine) -> Option<&ProbeReport> {
        let key = self.routine_key(routine);
        self.last_results_by_routine.get(&key)
    }

    /// Get all stored probe results.
    pub fn get_all_results(&self) -> &HashMap<u8, ProbeReport> {
        &self.last_results
    }

    /// Clear all stored results.
    pub fn clear_results(&mut self) {
        self.last_results.clear();
        self.last_results_by_routine.clear();
    }

    /// Generate a key for a routine type.
    fn routine_key(&self, routine: &ProbeRoutine) -> String {
        match routine {
            ProbeRoutine::ZTouch { .. } => "ztouch".to_string(),
            ProbeRoutine::EdgeFind {
                axis, direction, ..
            } => format!("edge_{:?}_{:?}", axis, direction).to_lowercase(),
            ProbeRoutine::CornerFind { corner, .. } => {
                format!("corner_{:?}", corner).to_lowercase()
            }
            ProbeRoutine::BoreCenter { .. } => "bore_center".to_string(),
            ProbeRoutine::BossCenter { .. } => "boss_center".to_string(),
            ProbeRoutine::ToolLength { .. } => "tool_length".to_string(),
        }
    }

    /// Get the human-readable name for a WCS.
    fn wcs_name(&self, wcs: u8) -> String {
        match wcs {
            0 => "G92 (Temporary)".to_string(),
            54..=59 => format!("G{}", wcs),
            _ => format!("WCS{}", wcs),
        }
    }

    /// Build a complete G-code snippet for the probe operation including
    /// the update command and any necessary setup.
    ///
    /// # Arguments
    /// * `report` - The probe report
    /// * `include_setup` - Whether to include G21/G90 setup commands
    ///
    /// # Returns
    /// Complete G-code string ready to send to the controller.
    pub fn build_complete_gcode(
        &self,
        report: &ProbeReport,
        include_setup: bool,
    ) -> Result<String> {
        let computed_pos = self.compute_position(report)?;
        let (update_cmd, _) = self.build_command(report, computed_pos)?;

        let mut gcode = String::new();

        if include_setup {
            gcode.push_str("; Probe Result Application\n");
            gcode.push_str("G21 ; Metric mode\n");
            gcode.push_str("G90 ; Absolute positioning\n");
        }

        let update_line = format!(
            "{} ; {}\n",
            update_cmd,
            self.routine_description(&report.routine)
        );
        let retract_line = format!(
            "G0 Z{:.3} ; Retract to safe height\n",
            self.config.safe_height
        );
        let is_g92 = update_cmd.starts_with("G92");
        if is_g92 {
            // G92 is relative to the position where it executes (the recorded machine
            // position), so it must run before any move.
            gcode.push_str(&update_line);
            gcode.push_str(&retract_line);
        } else {
            gcode.push_str(&retract_line);
            gcode.push_str(&update_line);
        }

        // For tool length, also suggest resetting
        if matches!(report.routine, ProbeRoutine::ToolLength { .. })
            && self.config.apply_tool_length
        {
            gcode.push_str("; To reset tool length: G49\n");
        }

        Ok(gcode)
    }

    /// Get a description of the routine type.
    fn routine_description(&self, routine: &ProbeRoutine) -> String {
        match routine {
            ProbeRoutine::ZTouch { .. } => "Set Z surface".to_string(),
            ProbeRoutine::EdgeFind { axis, .. } => format!("Set {:?} edge", axis),
            ProbeRoutine::CornerFind { corner, .. } => format!("Set {:?} corner", corner),
            ProbeRoutine::BoreCenter { .. } => "Set bore center".to_string(),
            ProbeRoutine::BossCenter { .. } => "Set boss center".to_string(),
            ProbeRoutine::ToolLength { .. } => "Set tool length".to_string(),
        }
    }
}

/// Persistent storage for probe results across sessions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistentProbeResults {
    /// Results stored by WCS number (54-59).
    pub results_by_wcs: HashMap<u8, PersistedProbeReport>,
    /// Default configuration for WCS updates.
    pub default_config: WcsUpdateConfig,
    /// Last used WCS number.
    pub last_wcs: u8,
}

/// Serializable version of ProbeReport for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedProbeReport {
    /// Routine type name.
    pub routine_type: String,
    /// Number of trigger results.
    pub trigger_count: usize,
    /// Computed position X.
    pub computed_x: f32,
    /// Computed position Y.
    pub computed_y: f32,
    /// Computed position Z.
    pub computed_z: f32,
    /// Suggested G10 command.
    pub g10_command: Option<String>,
    /// Suggested G92 command.
    pub g92_command: Option<String>,
    /// Suggested G43.1 command.
    pub g43_1_command: Option<String>,
    /// Timestamp of the probe.
    pub timestamp: String,
}

impl PersistentProbeResults {
    /// Create empty persistent results.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a probe report for persistence.
    pub fn store_report(&mut self, wcs: u8, report: &ProbeReport) {
        let persisted = PersistedProbeReport {
            routine_type: format!("{:?}", report.routine),
            trigger_count: report.triggers.len(),
            computed_x: report.computed.map(|p| p.x).unwrap_or(0.0),
            computed_y: report.computed.map(|p| p.y).unwrap_or(0.0),
            computed_z: report.computed.map(|p| p.z).unwrap_or(0.0),
            g10_command: report.suggested_g10.clone(),
            g92_command: report.suggested_g92.clone(),
            g43_1_command: report.suggested_g43_1.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.results_by_wcs.insert(wcs, persisted);
        self.last_wcs = wcs;
    }

    /// Get a stored report for a WCS.
    pub fn get_report(&self, wcs: u8) -> Option<&PersistedProbeReport> {
        self.results_by_wcs.get(&wcs)
    }

    /// Clear all stored results.
    pub fn clear(&mut self) {
        self.results_by_wcs.clear();
    }
}
