//! Phase 6 Advanced Features - Tasks 103-120
//!
//! Task 103: Auto-leveling probe mesh
//! Task 104: Tool change management
//! Task 105: Tool length offset probing
//! Task 106: Work coordinate systems
//! Task 107: Soft limits
//! Task 108: Simulation mode
//! Task 109: Step-through execution
//! Task 110: Bookmarks/Breakpoints
//! Task 111: Program restart
//! Task 112: Performance monitoring
//! Task 113: Command history
//! Task 114: Custom scripts/macros
//! Task 115: Pendant support
//! Task 116: Custom buttons/actions
//! Task 117: Auto-connect
//! Task 118: Network/remote access
//! Task 119: Data logging
//! Task 120: Alarms and notifications

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// TASK 103: AUTO-LEVELING PROBE MESH
// ============================================================================

/// Height map data point
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HeightPoint {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Z height at this point
    pub z: f64,
}

/// Probe mesh for auto-leveling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeMesh {
    /// Grid points
    pub points: Vec<HeightPoint>,
    /// Grid X spacing
    pub x_spacing: f64,
    /// Grid Y spacing
    pub y_spacing: f64,
    /// Minimum Z found
    pub z_min: f64,
    /// Maximum Z found
    pub z_max: f64,
}

impl ProbeMesh {
    /// Create new probe mesh
    pub fn new(x_spacing: f64, y_spacing: f64) -> Self {
        Self {
            points: Vec::new(),
            x_spacing,
            y_spacing,
            z_min: f64::MAX,
            z_max: f64::MIN,
        }
    }

    /// Add probe point
    pub fn add_point(&mut self, point: HeightPoint) {
        self.z_min = self.z_min.min(point.z);
        self.z_max = self.z_max.max(point.z);
        self.points.push(point);
    }

    /// Get mesh statistics
    pub fn stats(&self) -> (usize, f64, f64) {
        (self.points.len(), self.z_min, self.z_max)
    }
}

// ============================================================================
// TASK 104: TOOL CHANGE MANAGEMENT
// ============================================================================

/// Tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool number
    pub number: u32,
    /// Tool name
    pub name: String,
    /// Tool diameter
    pub diameter: f64,
    /// Tool length
    pub length: f64,
    /// Tool material (HSS, Carbide, etc.)
    pub material: String,
}

impl ToolInfo {
    /// Create new tool
    pub fn new(number: u32, name: impl Into<String>, diameter: f64) -> Self {
        Self {
            number,
            name: name.into(),
            diameter,
            length: 0.0,
            material: "Unknown".to_string(),
        }
    }
}

/// Tool change event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolChangeType {
    /// Manual tool change
    Manual,
    /// Automatic tool change
    Automatic,
}

/// Tool library
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolLibrary {
    /// Tools by number
    tools: HashMap<u32, ToolInfo>,
    /// Current tool
    current_tool: Option<u32>,
}

impl ToolLibrary {
    /// Create new tool library
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            current_tool: None,
        }
    }

    /// Add tool
    pub fn add_tool(&mut self, tool: ToolInfo) {
        self.tools.insert(tool.number, tool);
    }

    /// Get tool
    pub fn get_tool(&self, number: u32) -> Option<&ToolInfo> {
        self.tools.get(&number)
    }

    /// Set current tool
    pub fn set_current_tool(&mut self, number: u32) {
        self.current_tool = Some(number);
    }

    /// Get current tool
    pub fn current_tool(&self) -> Option<&ToolInfo> {
        self.current_tool.and_then(|n| self.tools.get(&n))
    }

    /// List all tools
    pub fn list_tools(&self) -> Vec<&ToolInfo> {
        self.tools.values().collect()
    }
}

impl Default for ToolLibrary {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 105: TOOL LENGTH OFFSET
// ============================================================================

/// Tool offset entry
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ToolOffset {
    /// Tool number
    pub tool_number: u32,
    /// Length offset
    pub length_offset: f64,
    /// Wear offset
    pub wear_offset: f64,
}

impl ToolOffset {
    /// Create new tool offset
    pub fn new(tool_number: u32, length_offset: f64) -> Self {
        Self {
            tool_number,
            length_offset,
            wear_offset: 0.0,
        }
    }

    /// Get total offset
    pub fn total_offset(&self) -> f64 {
        self.length_offset + self.wear_offset
    }
}

/// Tool offset manager
pub struct ToolOffsetManager {
    /// Offsets by tool number
    offsets: HashMap<u32, ToolOffset>,
}

impl ToolOffsetManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            offsets: HashMap::new(),
        }
    }

    /// Set offset
    pub fn set_offset(&mut self, offset: ToolOffset) {
        self.offsets.insert(offset.tool_number, offset);
    }

    /// Get offset
    pub fn get_offset(&self, tool_number: u32) -> Option<&ToolOffset> {
        self.offsets.get(&tool_number)
    }

    /// Get total offset
    pub fn get_total_offset(&self, tool_number: u32) -> f64 {
        self.offsets
            .get(&tool_number)
            .map(|o| o.total_offset())
            .unwrap_or(0.0)
    }

    /// Adjust wear offset
    pub fn adjust_wear(&mut self, tool_number: u32, adjustment: f64) {
        if let Some(offset) = self.offsets.get_mut(&tool_number) {
            offset.wear_offset += adjustment;
        }
    }
}

impl Default for ToolOffsetManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 106: WORK COORDINATE SYSTEMS
// ============================================================================

/// Work coordinate system offset
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorkOffset {
    /// X offset
    pub x: f64,
    /// Y offset
    pub y: f64,
    /// Z offset
    pub z: f64,
}

impl WorkOffset {
    /// Create new offset
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Zero offset
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

/// Work coordinate systems manager
pub struct WorkCoordinateSystem {
    /// WCS 1-6 (G54-G59)
    systems: HashMap<u32, WorkOffset>,
    /// Current system (1-6)
    current_system: u32,
}

impl WorkCoordinateSystem {
    /// Create new WCS manager
    pub fn new() -> Self {
        let mut systems = HashMap::new();
        for i in 1..=9 {
            systems.insert(i, WorkOffset::zero());
        }

        Self {
            systems,
            current_system: 1,
        }
    }

    /// Set offset for system
    pub fn set_offset(&mut self, system: u32, offset: WorkOffset) {
        self.systems.insert(system, offset);
    }

    /// Get offset
    pub fn get_offset(&self, system: u32) -> Option<WorkOffset> {
        self.systems.get(&system).copied()
    }

    /// Select system
    pub fn select_system(&mut self, system: u32) -> Result<()> {
        if !(1..=9).contains(&system) {
            return Err(anyhow::anyhow!("Invalid WCS system: {}", system));
        }
        self.current_system = system;
        Ok(())
    }

    /// Get current system
    pub fn current_system(&self) -> u32 {
        self.current_system
    }

    /// Get current offset
    pub fn current_offset(&self) -> WorkOffset {
        self.systems
            .get(&self.current_system)
            .copied()
            .unwrap_or(WorkOffset::zero())
    }
}

impl Default for WorkCoordinateSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 107: SOFT LIMITS
// ============================================================================

/// Machine limits
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SoftLimits {
    /// X minimum
    pub x_min: f64,
    /// X maximum
    pub x_max: f64,
    /// Y minimum
    pub y_min: f64,
    /// Y maximum
    pub y_max: f64,
    /// Z minimum
    pub z_min: f64,
    /// Z maximum
    pub z_max: f64,
    /// Enabled
    pub enabled: bool,
}

impl SoftLimits {
    /// Create new soft limits
    pub fn new() -> Self {
        Self {
            x_min: -100.0,
            x_max: 100.0,
            y_min: -100.0,
            y_max: 100.0,
            z_min: -100.0,
            z_max: 100.0,
            enabled: true,
        }
    }

    /// Check if position is within limits
    pub fn check(&self, x: f64, y: f64, z: f64) -> bool {
        if !self.enabled {
            return true;
        }

        x >= self.x_min
            && x <= self.x_max
            && y >= self.y_min
            && y <= self.y_max
            && z >= self.z_min
            && z <= self.z_max
    }

    /// Get violations
    pub fn get_violations(&self, x: f64, y: f64, z: f64) -> Vec<String> {
        let mut violations = Vec::new();

        if !self.enabled {
            return violations;
        }

        if x < self.x_min {
            violations.push(format!("X too low: {} < {}", x, self.x_min));
        }
        if x > self.x_max {
            violations.push(format!("X too high: {} > {}", x, self.x_max));
        }
        if y < self.y_min {
            violations.push(format!("Y too low: {} < {}", y, self.y_min));
        }
        if y > self.y_max {
            violations.push(format!("Y too high: {} > {}", y, self.y_max));
        }
        if z < self.z_min {
            violations.push(format!("Z too low: {} < {}", z, self.z_min));
        }
        if z > self.z_max {
            violations.push(format!("Z too high: {} > {}", z, self.z_max));
        }

        violations
    }
}

impl Default for SoftLimits {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 108: SIMULATION MODE
// ============================================================================

/// Simulation state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SimulationPosition {
    /// X position
    pub x: f64,
    /// Y position
    pub y: f64,
    /// Z position
    pub z: f64,
}

/// Dry-run simulator
pub struct Simulator {
    /// Current position
    pub position: SimulationPosition,
    /// Commands executed
    pub commands_executed: u32,
    /// Simulation active
    pub active: bool,
}

impl Simulator {
    /// Create new simulator
    pub fn new() -> Self {
        Self {
            position: SimulationPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            commands_executed: 0,
            active: false,
        }
    }

    /// Start simulation
    pub fn start(&mut self) {
        self.active = true;
        self.commands_executed = 0;
    }

    /// Stop simulation
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Execute move command
    pub fn move_to(&mut self, x: f64, y: f64, z: f64) {
        if self.active {
            self.position.x = x;
            self.position.y = y;
            self.position.z = z;
            self.commands_executed += 1;
        }
    }

    /// Get distance traveled
    pub fn distance_traveled(&self) -> f64 {
        0.0 // Would be accumulated from moves
    }
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 109: STEP-THROUGH EXECUTION
// ============================================================================

/// Step execution mode
pub struct Stepper {
    /// Current line
    pub current_line: u32,
    /// Total lines
    pub total_lines: u32,
    /// Paused
    pub paused: bool,
}

impl Stepper {
    /// Create new stepper
    pub fn new(total_lines: u32) -> Self {
        Self {
            current_line: 0,
            total_lines,
            paused: true,
        }
    }

    /// Step forward
    pub fn step_forward(&mut self) -> bool {
        if self.current_line < self.total_lines {
            self.current_line += 1;
            true
        } else {
            false
        }
    }

    /// Step backward
    pub fn step_backward(&mut self) -> bool {
        if self.current_line > 0 {
            self.current_line -= 1;
            true
        } else {
            false
        }
    }

    /// Get current line
    pub fn current_line(&self) -> u32 {
        self.current_line
    }

    /// Resume
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Pause
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Is paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

// ============================================================================
// TASK 110: BOOKMARKS/BREAKPOINTS
// ============================================================================

/// Bookmark/breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    /// Line number
    pub line: u32,
    /// Name/description
    pub name: String,
    /// Is breakpoint
    pub is_breakpoint: bool,
}

/// Bookmark manager
pub struct BookmarkManager {
    /// Bookmarks by line
    bookmarks: HashMap<u32, Bookmark>,
}

impl BookmarkManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            bookmarks: HashMap::new(),
        }
    }

    /// Add bookmark
    pub fn add_bookmark(&mut self, line: u32, name: impl Into<String>) {
        self.bookmarks.insert(
            line,
            Bookmark {
                line,
                name: name.into(),
                is_breakpoint: false,
            },
        );
    }

    /// Add breakpoint
    pub fn add_breakpoint(&mut self, line: u32, name: impl Into<String>) {
        self.bookmarks.insert(
            line,
            Bookmark {
                line,
                name: name.into(),
                is_breakpoint: true,
            },
        );
    }

    /// Remove bookmark
    pub fn remove_bookmark(&mut self, line: u32) -> bool {
        self.bookmarks.remove(&line).is_some()
    }

    /// Get bookmark
    pub fn get_bookmark(&self, line: u32) -> Option<&Bookmark> {
        self.bookmarks.get(&line)
    }

    /// List all bookmarks
    pub fn list_bookmarks(&self) -> Vec<&Bookmark> {
        self.bookmarks.values().collect()
    }

    /// List breakpoints
    pub fn list_breakpoints(&self) -> Vec<&Bookmark> {
        self.bookmarks
            .values()
            .filter(|b| b.is_breakpoint)
            .collect()
    }
}

impl Default for BookmarkManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 111: PROGRAM RESTART
// ============================================================================

/// Program state for restart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramState {
    /// Current line
    pub current_line: u32,
    /// Position X
    pub x: f64,
    /// Position Y
    pub y: f64,
    /// Position Z
    pub z: f64,
    /// Current tool
    pub tool: Option<u32>,
    /// Current speed
    pub speed: f64,
    /// Current feed rate
    pub feed_rate: f64,
}

impl ProgramState {
    /// Create new state
    pub fn new() -> Self {
        Self {
            current_line: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            tool: None,
            speed: 1000.0,
            feed_rate: 100.0,
        }
    }
}

impl Default for ProgramState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 112: PERFORMANCE MONITORING
// ============================================================================

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Commands sent
    pub commands_sent: u32,
    /// Commands per second
    pub commands_per_second: f64,
    /// Buffer usage percent
    pub buffer_usage: f64,
    /// Total distance traveled
    pub distance_traveled: f64,
    /// Execution time in seconds
    pub execution_time: f64,
}

impl PerformanceMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            commands_sent: 0,
            commands_per_second: 0.0,
            buffer_usage: 0.0,
            distance_traveled: 0.0,
            execution_time: 0.0,
        }
    }

    /// Get throughput estimate
    pub fn throughput(&self) -> f64 {
        if self.execution_time > 0.0 {
            self.commands_sent as f64 / self.execution_time
        } else {
            0.0
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 113: COMMAND HISTORY
// ============================================================================

/// Command history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Command text
    pub command: String,
    /// Timestamp
    pub timestamp: u64,
    /// Success
    pub success: bool,
}

/// Command history manager
pub struct CommandHistory {
    /// History entries
    entries: VecDeque<HistoryEntry>,
    /// Max entries
    max_entries: usize,
}

impl CommandHistory {
    /// Create new history
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }

    /// Add entry
    pub fn add(&mut self, command: impl Into<String>, success: bool) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.entries.push_back(HistoryEntry {
            command: command.into(),
            timestamp,
            success,
        });

        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    /// Get history
    pub fn get_history(&self) -> Vec<&HistoryEntry> {
        self.entries.iter().collect()
    }

    /// Get last command
    pub fn get_last(&self) -> Option<&HistoryEntry> {
        self.entries.back()
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new(1000)
    }
}

// ============================================================================
// TASK 114: CUSTOM SCRIPTS/MACROS
// ============================================================================

/// Custom macro
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMacro {
    /// Macro name
    pub name: String,
    /// Macro code
    pub code: String,
    /// Variables
    pub variables: HashMap<String, String>,
}

impl CustomMacro {
    /// Create new macro
    pub fn new(name: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            code: code.into(),
            variables: HashMap::new(),
        }
    }

    /// Set variable
    pub fn set_variable(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(name.into(), value.into());
    }

    /// Expand macro
    pub fn expand(&self) -> String {
        let mut result = self.code.clone();
        for (name, value) in &self.variables {
            let placeholder = format!("${{{}}}", name);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

// ============================================================================
// TASK 115: PENDANT SUPPORT
// ============================================================================

/// Pendant button type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendantButton {
    /// Start
    Start,
    /// Stop
    Stop,
    /// Pause/Resume
    PauseResume,
    /// Feed override +
    FeedUp,
    /// Feed override -
    FeedDown,
    /// Speed override +
    SpeedUp,
    /// Speed override -
    SpeedDown,
}

/// Pendant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendantConfig {
    /// Device path
    pub device_path: String,
    /// Button mappings
    pub button_mappings: HashMap<u32, PendantButton>,
    /// Enabled
    pub enabled: bool,
}

impl PendantConfig {
    /// Create new config
    pub fn new(device_path: impl Into<String>) -> Self {
        Self {
            device_path: device_path.into(),
            button_mappings: HashMap::new(),
            enabled: false,
        }
    }
}

// ============================================================================
// TASK 116: CUSTOM BUTTONS/ACTIONS
// ============================================================================

/// Custom action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAction {
    /// Action name
    pub name: String,
    /// Commands to execute
    pub commands: Vec<String>,
    /// Description
    pub description: String,
}

impl CustomAction {
    /// Create new action
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            commands: Vec::new(),
            description: String::new(),
        }
    }

    /// Add command
    pub fn add_command(&mut self, command: impl Into<String>) {
        self.commands.push(command.into());
    }
}

// ============================================================================
// TASK 117: AUTO-CONNECT
// ============================================================================

/// Auto-connect configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConnectConfig {
    /// Enabled
    pub enabled: bool,
    /// Last connection string
    pub last_connection: Option<String>,
    /// Auto-detect firmware
    pub auto_detect_firmware: bool,
}

impl AutoConnectConfig {
    /// Create new config
    pub fn new() -> Self {
        Self {
            enabled: true,
            last_connection: None,
            auto_detect_firmware: true,
        }
    }
}

impl Default for AutoConnectConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 118: NETWORK/REMOTE ACCESS
// ============================================================================

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// WebSocket enabled
    pub websocket_enabled: bool,
    /// WebSocket port
    pub websocket_port: u16,
    /// REST API enabled
    pub rest_enabled: bool,
    /// REST port
    pub rest_port: u16,
    /// Remote monitoring enabled
    pub remote_monitoring: bool,
    /// Remote control enabled
    pub remote_control: bool,
}

impl NetworkConfig {
    /// Create new config
    pub fn new() -> Self {
        Self {
            websocket_enabled: false,
            websocket_port: 8080,
            rest_enabled: false,
            rest_port: 8081,
            remote_monitoring: false,
            remote_control: false,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 119: DATA LOGGING
// ============================================================================

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp
    pub timestamp: u64,
    /// Log level (INFO, WARN, ERROR)
    pub level: String,
    /// Message
    pub message: String,
}

/// Data logger
pub struct DataLogger {
    /// Log entries
    entries: Vec<LogEntry>,
    /// Logging enabled
    enabled: bool,
}

impl DataLogger {
    /// Create new logger
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            enabled: true,
        }
    }

    /// Log message
    pub fn log(&mut self, level: impl Into<String>, message: impl Into<String>) {
        if self.enabled {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            self.entries.push(LogEntry {
                timestamp,
                level: level.into(),
                message: message.into(),
            });
        }
    }

    /// Get logs
    pub fn get_logs(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Clear logs
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Export logs
    pub fn export(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

impl Default for DataLogger {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TASK 120: ALARMS AND NOTIFICATIONS
// ============================================================================

/// Alarm type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlarmType {
    /// Info
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
}

/// Alarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alarm {
    /// Alarm type
    pub alarm_type: AlarmType,
    /// Message
    pub message: String,
    /// Timestamp
    pub timestamp: u64,
    /// Acknowledged
    pub acknowledged: bool,
}

impl Alarm {
    /// Create new alarm
    pub fn new(alarm_type: AlarmType, message: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            alarm_type,
            message: message.into(),
            timestamp,
            acknowledged: false,
        }
    }
}

/// Alarm manager
pub struct AlarmManager {
    /// Active alarms
    alarms: Vec<Alarm>,
    /// Enable sound
    _sound_enabled: bool,
    /// Enable visual
    _visual_enabled: bool,
    /// Enable system tray
    _system_tray_enabled: bool,
}

impl AlarmManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            alarms: Vec::new(),
            _sound_enabled: true,
            _visual_enabled: true,
            _system_tray_enabled: true,
        }
    }

    /// Add alarm
    pub fn add_alarm(&mut self, alarm: Alarm) {
        self.alarms.push(alarm);
    }

    /// Get alarms
    pub fn get_alarms(&self) -> &[Alarm] {
        &self.alarms
    }

    /// Acknowledge alarm
    pub fn acknowledge(&mut self, index: usize) -> bool {
        if index < self.alarms.len() {
            self.alarms[index].acknowledged = true;
            true
        } else {
            false
        }
    }

    /// Clear alarms
    pub fn clear(&mut self) {
        self.alarms.clear();
    }

    /// Get unacknowledged alarms
    pub fn unacknowledged(&self) -> Vec<&Alarm> {
        self.alarms.iter().filter(|a| !a.acknowledged).collect()
    }
}

impl Default for AlarmManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_mesh() {
        let mut mesh = ProbeMesh::new(1.0, 1.0);
        mesh.add_point(HeightPoint {
            x: 0.0,
            y: 0.0,
            z: 0.5,
        });
        mesh.add_point(HeightPoint {
            x: 1.0,
            y: 1.0,
            z: 0.7,
        });

        let (count, min, max) = mesh.stats();
        assert_eq!(count, 2);
        assert!(min < 1.0 && max > 0.0);
    }

    #[test]
    fn test_tool_library() {
        let mut lib = ToolLibrary::new();
        let tool = ToolInfo::new(1, "End Mill", 3.175);
        lib.add_tool(tool);
        lib.set_current_tool(1);

        assert!(lib.current_tool().is_some());
    }

    #[test]
    fn test_work_coordinate_system() {
        let mut wcs = WorkCoordinateSystem::new();
        wcs.set_offset(1, WorkOffset::new(10.0, 20.0, 30.0));

        let offset = wcs.get_offset(1).expect("offset not found");
        assert_eq!(offset.x, 10.0);
    }

    #[test]
    fn test_soft_limits() {
        let limits = SoftLimits::new();
        assert!(limits.check(0.0, 0.0, 0.0));
        assert!(!limits.check(200.0, 0.0, 0.0));
    }

    #[test]
    fn test_simulator() {
        let mut sim = Simulator::new();
        sim.start();
        sim.move_to(10.0, 20.0, 30.0);

        assert_eq!(sim.position.x, 10.0);
        assert_eq!(sim.commands_executed, 1);
    }

    #[test]
    fn test_stepper() {
        let mut stepper = Stepper::new(100);
        assert!(stepper.step_forward());
        assert_eq!(stepper.current_line, 1);
    }

    #[test]
    fn test_bookmark_manager() {
        let mut manager = BookmarkManager::new();
        manager.add_bookmark(10, "Start cutting");
        manager.add_breakpoint(50, "Check dimension");

        assert!(manager.get_bookmark(10).is_some());
        assert_eq!(manager.list_bookmarks().len(), 2);
        assert_eq!(manager.list_breakpoints().len(), 1);
    }

    #[test]
    fn test_command_history() {
        let mut history = CommandHistory::new(100);
        history.add("G0 X10", true);
        history.add("G1 Y20", false);

        assert_eq!(history.get_history().len(), 2);
    }

    #[test]
    fn test_custom_macro() {
        let mut macro_obj = CustomMacro::new("move", "G0 X${X} Y${Y}");
        macro_obj.set_variable("X", "10");
        macro_obj.set_variable("Y", "20");

        let expanded = macro_obj.expand();
        assert!(expanded.contains("X10"));
        assert!(expanded.contains("Y20"));
    }

    #[test]
    fn test_custom_action() {
        let mut action = CustomAction::new("Drill");
        action.add_command("G0 Z5");
        action.add_command("G1 Z-5 F100");

        assert_eq!(action.commands.len(), 2);
    }

    #[test]
    fn test_alarm_manager() {
        let mut manager = AlarmManager::new();
        let alarm = Alarm::new(AlarmType::Warning, "Low coolant");
        manager.add_alarm(alarm);

        assert_eq!(manager.get_alarms().len(), 1);
        assert_eq!(manager.unacknowledged().len(), 1);

        manager.acknowledge(0);
        assert_eq!(manager.unacknowledged().len(), 0);
    }

    #[test]
    fn test_data_logger() {
        let mut logger = DataLogger::new();
        logger.log("INFO", "Starting job");
        logger.log("ERROR", "Command failed");

        assert_eq!(logger.get_logs().len(), 2);
    }
}
