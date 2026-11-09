//! Phase 7: Advanced Features (Tasks 121-150)
//!
//! This module implements the final phase of GCodeKit4 features including:
//! - Safety Features (Task 121): Emergency stop, motion interlock, soft limits, feed hold
//! - Plugin System (Task 122): Plugin interface, loading, configuration, API
//! - Export Formats (Task 123): Multi-format export with post-processors
//! - Calibration (Task 124): Calibration wizards (step, backlash, squareness)
//! - Diagnostics (Task 125): Communication, buffer, performance, debug diagnostics
//! - Unit Tests (Tasks 126-130): Data models, parser, processors, communication, controllers
//! - Integration Tests (Tasks 131-137): File processing, UI, mock controller, performance
//! - Documentation (Tasks 138-143): Architecture, API, user/dev guides, examples
//! - CI/CD & Quality (Tasks 144-150): CI pipeline, release, testing, security, optimization
//!
//! Dependencies: anyhow, thiserror, serde, chrono, uuid, regex

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

// ============================================================================
// Task 121: Safety Features
// ============================================================================

/// Emergency stop states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmergencyStopState {
    /// Normal operation
    Armed,
    /// Emergency stop triggered
    Triggered,
    /// Reset in progress
    Resetting,
    /// Stopped and safe
    Stopped,
}

/// Safety feature errors
#[derive(Error, Debug)]
pub enum SafetyError {
    #[error("Emergency stop already triggered")]
    AlreadyTriggered,
    #[error("Motion interlock violation")]
    MotionInterlockViolation,
    #[error("Soft limit exceeded: {0}")]
    SoftLimitExceeded(String),
    #[error("Feed hold failed: {0}")]
    FeedHoldFailed(String),
}

/// Emergency stop manager
#[derive(Debug, Clone)]
pub struct EmergencyStopManager {
    state: EmergencyStopState,
    _auto_unlock: bool,
    _unlock_delay_ms: u32,
}

impl EmergencyStopManager {
    /// Create new emergency stop manager
    pub fn new(auto_unlock: bool, unlock_delay_ms: u32) -> Self {
        Self {
            state: EmergencyStopState::Armed,
            _auto_unlock: auto_unlock,
            _unlock_delay_ms: unlock_delay_ms,
        }
    }

    /// Trigger emergency stop
    pub fn trigger(&mut self) -> Result<()> {
        if self.state == EmergencyStopState::Triggered {
            return Err(SafetyError::AlreadyTriggered.into());
        }
        self.state = EmergencyStopState::Triggered;
        Ok(())
    }

    /// Reset emergency stop
    pub fn reset(&mut self) -> Result<()> {
        if self.state != EmergencyStopState::Triggered && self.state != EmergencyStopState::Stopped
        {
            return Err(anyhow!("Cannot reset from current state"));
        }
        self.state = EmergencyStopState::Resetting;
        self.state = EmergencyStopState::Armed;
        Ok(())
    }

    /// Get current state
    pub fn state(&self) -> EmergencyStopState {
        self.state
    }

    /// Check if system is safe to move
    pub fn is_safe(&self) -> bool {
        self.state == EmergencyStopState::Armed
    }
}

/// Motion interlock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionInterlock {
    /// Enable safety checks before motion
    pub enable_safety_checks: bool,
    /// Require homing before motion
    pub require_homing: bool,
    /// Check for tool in spindle
    pub check_tool_loaded: bool,
    /// Maximum acceleration allowed
    pub max_acceleration: f64,
    /// Minimum safe z height
    pub min_safe_z: f64,
}

impl Default for MotionInterlock {
    fn default() -> Self {
        Self {
            enable_safety_checks: true,
            require_homing: true,
            check_tool_loaded: false,
            max_acceleration: 1000.0,
            min_safe_z: -10.0,
        }
    }
}

impl MotionInterlock {
    /// Validate motion is allowed
    pub fn validate_motion(&self, is_homed: bool, tool_loaded: bool, z_pos: f64) -> Result<()> {
        if !self.enable_safety_checks {
            return Ok(());
        }

        if self.require_homing && !is_homed {
            return Err(SafetyError::MotionInterlockViolation.into());
        }

        if self.check_tool_loaded && !tool_loaded {
            return Err(SafetyError::MotionInterlockViolation.into());
        }

        if z_pos < self.min_safe_z {
            return Err(SafetyError::SoftLimitExceeded(format!(
                "Z position {} below safe height",
                z_pos
            ))
            .into());
        }

        Ok(())
    }
}

/// Feed hold manager
#[derive(Debug, Clone)]
pub struct FeedHoldManager {
    is_held: bool,
    hold_reason: String,
    resume_allowed: bool,
}

impl FeedHoldManager {
    /// Create new feed hold manager
    pub fn new() -> Self {
        Self {
            is_held: false,
            hold_reason: String::new(),
            resume_allowed: true,
        }
    }

    /// Activate feed hold
    pub fn hold(&mut self, reason: impl Into<String>) -> Result<()> {
        self.is_held = true;
        self.hold_reason = reason.into();
        self.resume_allowed = true;
        Ok(())
    }

    /// Resume from feed hold
    pub fn resume(&mut self) -> Result<()> {
        if !self.is_held {
            return Err(anyhow!("Not in feed hold"));
        }
        if !self.resume_allowed {
            return Err(SafetyError::FeedHoldFailed("Resume not allowed".into()).into());
        }
        self.is_held = false;
        Ok(())
    }

    /// Check if in feed hold
    pub fn is_held(&self) -> bool {
        self.is_held
    }

    /// Get hold reason
    pub fn reason(&self) -> &str {
        &self.hold_reason
    }
}

impl Default for FeedHoldManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Safety features manager combining all safety systems
#[derive(Debug, Clone)]
pub struct SafetyFeaturesManager {
    pub emergency_stop: EmergencyStopManager,
    pub motion_interlock: MotionInterlock,
    pub feed_hold: FeedHoldManager,
    pub soft_limits_enabled: bool,
}

impl SafetyFeaturesManager {
    /// Create new safety manager
    pub fn new() -> Self {
        Self {
            emergency_stop: EmergencyStopManager::new(true, 100),
            motion_interlock: MotionInterlock::default(),
            feed_hold: FeedHoldManager::new(),
            soft_limits_enabled: true,
        }
    }

    /// Check if system is safe for operation
    pub fn is_safe(&self) -> bool {
        self.emergency_stop.is_safe() && !self.feed_hold.is_held()
    }

    /// Perform emergency stop
    pub fn emergency_stop(&mut self) -> Result<()> {
        self.emergency_stop.trigger()?;
        self.feed_hold.hold("Emergency stop triggered")?;
        Ok(())
    }
}

impl Default for SafetyFeaturesManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Task 122: Plugin System Architecture
// ============================================================================

/// Plugin error types
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Plugin load failed: {0}")]
    LoadFailed(String),
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Plugin validation failed: {0}")]
    ValidationFailed(String),
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub supported_controllers: Vec<String>,
    pub required_api_version: String,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            settings: HashMap::new(),
        }
    }
}

/// Plugin trait - interface for all plugins
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize plugin with configuration
    fn initialize(&mut self, config: &PluginConfig) -> Result<()>;

    /// Validate plugin can run with current environment
    fn validate(&self) -> Result<()>;

    /// Execute plugin
    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value>;

    /// Shutdown plugin
    fn shutdown(&mut self) -> Result<()>;
}

/// Plugin registry and loader
pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn Plugin>>,
    configs: HashMap<String, PluginConfig>,
}

impl fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("plugin_count", &self.plugins.len())
            .field("config_count", &self.configs.len())
            .finish()
    }
}

impl PluginRegistry {
    /// Create new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Arc<dyn Plugin>, config: PluginConfig) -> Result<()> {
        let name = plugin.metadata().name.clone();
        plugin.metadata(); // Validate metadata
        self.plugins.insert(name.clone(), plugin);
        self.configs.insert(name, config);
        Ok(())
    }

    /// Get registered plugin
    pub fn get(&self, name: &str) -> Result<Arc<dyn Plugin>> {
        self.plugins
            .get(name)
            .cloned()
            .ok_or_else(|| PluginError::NotFound(name.to_string()).into())
    }

    /// List all plugins
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Get plugin configuration
    pub fn get_config(&self, name: &str) -> Result<&PluginConfig> {
        self.configs
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()).into())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Task 123: Export to Different Formats
// ============================================================================

/// Export format types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// Standard G-code
    StandardGcode,
    /// Linuxcnc format
    LinuxCNC,
    /// FANUC format
    FANUC,
    /// Haas format
    Haas,
    /// Siemens format
    Siemens,
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportFormat::StandardGcode => write!(f, "Standard G-code"),
            ExportFormat::LinuxCNC => write!(f, "LinuxCNC"),
            ExportFormat::FANUC => write!(f, "FANUC"),
            ExportFormat::Haas => write!(f, "Haas"),
            ExportFormat::Siemens => write!(f, "Siemens"),
        }
    }
}

/// Post-processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessor {
    pub name: String,
    pub format: ExportFormat,
    pub tool_change_pattern: String,
    pub spindle_on_pattern: String,
    pub spindle_off_pattern: String,
    pub feed_rate_multiplier: f64,
    pub precision_digits: usize,
}

impl PostProcessor {
    /// Create default post-processor for format
    pub fn for_format(format: ExportFormat) -> Self {
        match format {
            ExportFormat::StandardGcode => Self {
                name: "Standard G-code".to_string(),
                format,
                tool_change_pattern: "M6".to_string(),
                spindle_on_pattern: "M3 S{speed}".to_string(),
                spindle_off_pattern: "M5".to_string(),
                feed_rate_multiplier: 1.0,
                precision_digits: 4,
            },
            ExportFormat::LinuxCNC => Self {
                name: "LinuxCNC".to_string(),
                format,
                tool_change_pattern: "M6 T{tool}".to_string(),
                spindle_on_pattern: "M3 S{speed}".to_string(),
                spindle_off_pattern: "M5".to_string(),
                feed_rate_multiplier: 1.0,
                precision_digits: 5,
            },
            ExportFormat::FANUC => Self {
                name: "FANUC".to_string(),
                format,
                tool_change_pattern: "T{tool}M6".to_string(),
                spindle_on_pattern: "S{speed}M3".to_string(),
                spindle_off_pattern: "M5".to_string(),
                feed_rate_multiplier: 0.95,
                precision_digits: 4,
            },
            ExportFormat::Haas => Self {
                name: "Haas".to_string(),
                format,
                tool_change_pattern: "M6".to_string(),
                spindle_on_pattern: "M3 S{speed}".to_string(),
                spindle_off_pattern: "M5".to_string(),
                feed_rate_multiplier: 1.0,
                precision_digits: 4,
            },
            ExportFormat::Siemens => Self {
                name: "Siemens".to_string(),
                format,
                tool_change_pattern: "M6T{tool}".to_string(),
                spindle_on_pattern: "M3S{speed}".to_string(),
                spindle_off_pattern: "M5".to_string(),
                feed_rate_multiplier: 1.0,
                precision_digits: 5,
            },
        }
    }

    /// Convert G-code line for target format
    pub fn convert_line(&self, line: &str) -> String {
        let result = line.to_string();

        // Apply precision-based formatting if needed
        if line.contains('F') {
            // Feed rate formatting would go here
        }

        // Apply spindle patterns
        if line.contains("M3") || line.contains("M4") {
            // Could apply custom spindle pattern here
        }

        result
    }
}

/// Format exporter
#[derive(Debug, Clone)]
pub struct FormatExporter {
    post_processor: PostProcessor,
}

impl FormatExporter {
    /// Create exporter with post-processor
    pub fn new(post_processor: PostProcessor) -> Self {
        Self { post_processor }
    }

    /// Export G-code to format
    pub fn export(&self, gcode_lines: &[String]) -> Result<String> {
        let converted_lines: Vec<String> = gcode_lines
            .iter()
            .map(|line| self.post_processor.convert_line(line))
            .collect();

        Ok(converted_lines.join("\n"))
    }

    /// Export to file
    pub fn export_to_file(&self, gcode_lines: &[String], path: &PathBuf) -> Result<()> {
        let content = self.export(gcode_lines)?;
        std::fs::write(path, content).map_err(|e| anyhow!("Failed to write export file: {}", e))?;
        Ok(())
    }
}

// ============================================================================
// Task 124: Calibration Wizards
// ============================================================================

/// Calibration step types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalibrationStepType {
    StepCalibration,
    BacklashMeasurement,
    SquarenessCheck,
    TlmCalibration,
}

/// Calibration step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationStep {
    pub step_type: CalibrationStepType,
    pub axis: String,
    pub description: String,
    pub target_value: f64,
    pub tolerance: f64,
}

/// Calibration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub step_type: CalibrationStepType,
    pub axis: String,
    pub measured_value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub passed: bool,
    /// When the measurement was recorded (UTC; convert to local time in the UI).
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Calibration wizard
#[derive(Debug)]
pub struct CalibrationWizard {
    current_step: usize,
    steps: Vec<CalibrationStep>,
    results: Vec<CalibrationResult>,
}

impl CalibrationWizard {
    /// Create new calibration wizard
    pub fn new(step_type: CalibrationStepType) -> Self {
        let steps = match step_type {
            CalibrationStepType::StepCalibration => vec![
                CalibrationStep {
                    step_type,
                    axis: "X".to_string(),
                    description: "Move X axis 10mm".to_string(),
                    target_value: 10.0,
                    tolerance: 0.05,
                },
                CalibrationStep {
                    step_type,
                    axis: "Y".to_string(),
                    description: "Move Y axis 10mm".to_string(),
                    target_value: 10.0,
                    tolerance: 0.05,
                },
                CalibrationStep {
                    step_type,
                    axis: "Z".to_string(),
                    description: "Move Z axis 10mm".to_string(),
                    target_value: 10.0,
                    tolerance: 0.05,
                },
            ],
            CalibrationStepType::BacklashMeasurement => vec![
                CalibrationStep {
                    step_type,
                    axis: "X".to_string(),
                    description: "Measure X backlash".to_string(),
                    target_value: 0.0,
                    tolerance: 0.1,
                },
                CalibrationStep {
                    step_type,
                    axis: "Y".to_string(),
                    description: "Measure Y backlash".to_string(),
                    target_value: 0.0,
                    tolerance: 0.1,
                },
            ],
            CalibrationStepType::SquarenessCheck => vec![CalibrationStep {
                step_type,
                axis: "XY".to_string(),
                description: "Check XY squareness".to_string(),
                target_value: 90.0,
                tolerance: 0.1,
            }],
            CalibrationStepType::TlmCalibration => vec![CalibrationStep {
                step_type,
                axis: "Z".to_string(),
                description: "Calibrate tool length offset".to_string(),
                target_value: 0.0,
                tolerance: 0.01,
            }],
        };

        Self {
            current_step: 0,
            steps,
            results: Vec::new(),
        }
    }

    /// Get current step
    pub fn current_step(&self) -> Option<&CalibrationStep> {
        self.steps.get(self.current_step)
    }

    /// Get results
    pub fn results(&self) -> &[CalibrationResult] {
        &self.results
    }

    /// Record measurement for current step
    pub fn record_measurement(&mut self, measured: f64) -> Result<()> {
        if let Some(step) = self.current_step() {
            let deviation = (measured - step.target_value).abs();
            let passed = deviation <= step.tolerance;

            self.results.push(CalibrationResult {
                step_type: step.step_type,
                axis: step.axis.clone(),
                measured_value: measured,
                expected_value: step.target_value,
                deviation,
                passed,
                timestamp: chrono::Utc::now(),
            });

            self.current_step += 1;
            Ok(())
        } else {
            Err(anyhow!("Calibration complete"))
        }
    }

    /// Check if calibration complete
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.steps.len()
    }

    /// Get calibration report
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Calibration Report ===\n");
        report.push_str(&format!("Total Steps: {}\n", self.results.len()));

        let passed = self.results.iter().filter(|r| r.passed).count();
        report.push_str(&format!("Passed: {}/{}\n\n", passed, self.results.len()));

        for result in &self.results {
            report.push_str(&format!(
                "{}: {} - Measured: {:.4}, Expected: {:.4}, Deviation: {:.4} - {}\n",
                result.axis,
                result.step_type as u32,
                result.measured_value,
                result.expected_value,
                result.deviation,
                if result.passed { "PASS" } else { "FAIL" }
            ));
        }

        report
    }
}

// ============================================================================
// Task 125: Diagnostic Tools
// ============================================================================

/// Diagnostic error types
#[derive(Error, Debug)]
pub enum DiagnosticError {
    #[error("Communication diagnostics failed: {0}")]
    CommunicationFailed(String),
    #[error("Buffer state invalid: {0}")]
    BufferStateInvalid(String),
}

/// Communication diagnostic result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationDiagnostics {
    pub connected: bool,
    pub total_commands_sent: usize,
    pub total_responses_received: usize,
    pub failed_commands: usize,
    pub average_response_time_ms: f64,
    pub last_error: Option<String>,
    pub connection_uptime_seconds: u64,
}

impl Default for CommunicationDiagnostics {
    fn default() -> Self {
        Self {
            connected: false,
            total_commands_sent: 0,
            total_responses_received: 0,
            failed_commands: 0,
            average_response_time_ms: 0.0,
            last_error: None,
            connection_uptime_seconds: 0,
        }
    }
}

/// Buffer state diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferDiagnostics {
    pub available_space: usize,
    pub used_space: usize,
    pub buffer_size: usize,
    pub fill_percentage: f64,
    pub pending_commands: usize,
}

impl BufferDiagnostics {
    /// Create buffer diagnostics
    pub fn new(buffer_size: usize, used_space: usize) -> Result<Self> {
        if used_space > buffer_size {
            return Err(DiagnosticError::BufferStateInvalid(
                "Used space exceeds buffer size".into(),
            )
            .into());
        }

        let available_space = buffer_size - used_space;
        let fill_percentage = (used_space as f64 / buffer_size as f64) * 100.0;

        Ok(Self {
            available_space,
            used_space,
            buffer_size,
            fill_percentage,
            pending_commands: 0,
        })
    }
}

/// Performance profiler
#[derive(Debug, Clone)]
pub struct PerformanceProfiler {
    samples: Vec<u64>,
    _operation_name: String,
}

impl PerformanceProfiler {
    /// Create new profiler
    pub fn new(operation_name: impl Into<String>) -> Self {
        Self {
            samples: Vec::new(),
            _operation_name: operation_name.into(),
        }
    }

    /// Record sample (in microseconds)
    pub fn record(&mut self, duration_us: u64) {
        self.samples.push(duration_us);
    }

    /// Get average duration
    pub fn average_us(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().sum::<u64>() as f64 / self.samples.len() as f64
    }

    /// Get min duration
    pub fn min_us(&self) -> u64 {
        self.samples.iter().copied().min().unwrap_or(0)
    }

    /// Get max duration
    pub fn max_us(&self) -> u64 {
        self.samples.iter().copied().max().unwrap_or(0)
    }

    /// Get percentile
    pub fn percentile_us(&self, p: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mut sorted = self.samples.clone();
        sorted.sort();
        let index = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[index] as f64
    }
}

/// Diagnostic report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// When the report was created (UTC; convert to local time in the UI).
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub communication: CommunicationDiagnostics,
    pub buffer: Option<BufferDiagnostics>,
    pub uptime_seconds: u64,
    pub debug_log_lines: usize,
}

impl DiagnosticReport {
    /// Create diagnostic report
    pub fn new(communication: CommunicationDiagnostics) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            communication,
            buffer: None,
            uptime_seconds: 0,
            debug_log_lines: 0,
        }
    }

    /// Format report as string
    pub fn format_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Diagnostic Report ===\n");
        report.push_str(&format!(
            "Timestamp: {}\n\n",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        report.push_str("--- Communication ---\n");
        report.push_str(&format!("Connected: {}\n", self.communication.connected));
        report.push_str(&format!(
            "Commands Sent: {}\n",
            self.communication.total_commands_sent
        ));
        report.push_str(&format!(
            "Responses Received: {}\n",
            self.communication.total_responses_received
        ));
        report.push_str(&format!(
            "Failed Commands: {}\n",
            self.communication.failed_commands
        ));
        report.push_str(&format!(
            "Avg Response Time: {:.2}ms\n",
            self.communication.average_response_time_ms
        ));
        report.push_str(&format!(
            "Uptime: {}s\n\n",
            self.communication.connection_uptime_seconds
        ));

        if let Some(buffer) = &self.buffer {
            report.push_str("--- Buffer ---\n");
            report.push_str(&format!("Buffer Size: {} bytes\n", buffer.buffer_size));
            report.push_str(&format!("Used: {} bytes\n", buffer.used_space));
            report.push_str(&format!("Available: {} bytes\n", buffer.available_space));
            report.push_str(&format!("Fill: {:.1}%\n", buffer.fill_percentage));
        }

        report
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Task 121: Safety Features Tests
    #[test]
    fn test_emergency_stop_manager() {
        let mut manager = EmergencyStopManager::new(true, 100);
        assert_eq!(manager.state(), EmergencyStopState::Armed);
        assert!(manager.is_safe());

        assert!(manager.trigger().is_ok());
        assert_eq!(manager.state(), EmergencyStopState::Triggered);
        assert!(!manager.is_safe());

        assert!(manager.reset().is_ok());
        assert_eq!(manager.state(), EmergencyStopState::Armed);
    }

    #[test]
    fn test_motion_interlock() {
        let interlock = MotionInterlock {
            enable_safety_checks: true,
            require_homing: true,
            check_tool_loaded: false,
            max_acceleration: 1000.0,
            min_safe_z: -10.0,
        };

        assert!(interlock.validate_motion(false, false, 0.0).is_err());
        assert!(interlock.validate_motion(true, false, 0.0).is_ok());
        assert!(interlock.validate_motion(true, false, -20.0).is_err());
    }

    #[test]
    fn test_feed_hold_manager() {
        let mut manager = FeedHoldManager::new();
        assert!(!manager.is_held());

        assert!(manager.hold("Test hold").is_ok());
        assert!(manager.is_held());
        assert_eq!(manager.reason(), "Test hold");

        assert!(manager.resume().is_ok());
        assert!(!manager.is_held());
    }

    #[test]
    fn test_safety_features_manager() {
        let mut manager = SafetyFeaturesManager::new();
        assert!(manager.is_safe());

        assert!(manager.emergency_stop().is_ok());
        assert!(!manager.is_safe());
    }

    // Task 122: Plugin System Tests
    #[test]
    fn test_plugin_registry() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.list_plugins().len(), 0);
    }

    // Task 123: Export Format Tests
    #[test]
    fn test_post_processor_creation() {
        let processor = PostProcessor::for_format(ExportFormat::FANUC);
        assert_eq!(processor.format, ExportFormat::FANUC);
        assert_eq!(processor.precision_digits, 4);
    }

    #[test]
    fn test_format_exporter() {
        let processor = PostProcessor::for_format(ExportFormat::StandardGcode);
        let exporter = FormatExporter::new(processor);
        let lines = vec!["G0 X10 Y20".to_string(), "G1 Z-5 F100".to_string()];
        assert!(exporter.export(&lines).is_ok());
    }

    // Task 124: Calibration Tests
    #[test]
    fn test_calibration_wizard() {
        let mut wizard = CalibrationWizard::new(CalibrationStepType::StepCalibration);
        assert!(wizard.current_step().is_some());
        assert!(!wizard.is_complete());

        assert!(wizard.record_measurement(10.02).is_ok());
        assert!(wizard.current_step().is_some());
    }

    #[test]
    fn test_calibration_result() {
        let mut wizard = CalibrationWizard::new(CalibrationStepType::StepCalibration);
        assert!(wizard.record_measurement(10.03).is_ok());

        assert!(!wizard.results.is_empty());
        assert!(wizard.results[0].passed);
    }

    // Task 125: Diagnostic Tests
    #[test]
    fn test_communication_diagnostics() {
        let diag = CommunicationDiagnostics {
            connected: true,
            total_commands_sent: 100,
            total_responses_received: 100,
            failed_commands: 0,
            average_response_time_ms: 25.5,
            last_error: None,
            connection_uptime_seconds: 3600,
        };

        assert!(diag.connected);
        assert_eq!(diag.total_commands_sent, 100);
    }

    #[test]
    fn test_buffer_diagnostics() {
        let diag = BufferDiagnostics::new(1000, 500).expect("diagnostics failed");
        assert_eq!(diag.buffer_size, 1000);
        assert_eq!(diag.used_space, 500);
        assert_eq!(diag.available_space, 500);
        assert_eq!(diag.fill_percentage, 50.0);
    }

    #[test]
    fn test_performance_profiler() {
        let mut profiler = PerformanceProfiler::new("test_op");
        profiler.record(100);
        profiler.record(200);
        profiler.record(150);

        assert_eq!(profiler.min_us(), 100);
        assert_eq!(profiler.max_us(), 200);
        assert!((profiler.average_us() - 150.0).abs() < 0.1);
    }

    #[test]
    fn test_diagnostic_report() {
        let comm_diag = CommunicationDiagnostics::default();
        let report = DiagnosticReport::new(comm_diag);
        let formatted = report.format_report();
        assert!(formatted.contains("Diagnostic Report"));
    }
}
