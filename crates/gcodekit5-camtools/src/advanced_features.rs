//! Advanced Features - Tasks 101-125
//!
//! Probing, tool management, work coordinates, soft limits, simulation,
//! step-through, bookmarks, program restart, performance monitoring

use std::collections::HashMap;

// ============================================================================
// Tasks 101-103: Probing System
// ============================================================================

/// Probe result
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// Probed Z position
    pub z_position: f32,
    /// Probe successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl ProbeResult {
    /// Create successful probe result
    pub fn success(z_position: f32) -> Self {
        Self {
            z_position,
            success: true,
            error: None,
        }
    }

    /// Create failed probe result
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            z_position: 0.0,
            success: false,
            error: Some(error.into()),
        }
    }
}

/// Probing system
#[derive(Debug, Clone)]
pub struct ProbingSystem {
    /// Probe tip diameter
    pub tip_diameter: f32,
    /// Probe feed rate
    pub feed_rate: f32,
    /// Probe results
    pub results: Vec<ProbeResult>,
}

impl ProbingSystem {
    /// Create new probing system
    pub fn new() -> Self {
        Self {
            tip_diameter: 3.0,
            feed_rate: 50.0,
            results: Vec::new(),
        }
    }

    /// Add probe result
    pub fn add_result(&mut self, result: ProbeResult) {
        self.results.push(result);
    }

    /// Get average probed height
    pub fn average_height(&self) -> Option<f32> {
        let successful: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.z_position)
            .collect();

        if successful.is_empty() {
            None
        } else {
            Some(successful.iter().sum::<f32>() / successful.len() as f32)
        }
    }

    /// Generate probe mesh
    pub fn generate_mesh(&self, grid_x: usize, grid_y: usize) -> Vec<Vec<f32>> {
        let mut mesh = vec![vec![0.0; grid_x]; grid_y];
        let mut idx = 0;

        for row in mesh.iter_mut() {
            for cell in row.iter_mut() {
                if idx < self.results.len() && self.results[idx].success {
                    *cell = self.results[idx].z_position;
                }
                idx += 1;
            }
        }

        mesh
    }
}

impl Default for ProbingSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tasks 104-105: Tool Management
// ============================================================================

/// Tool definition
#[derive(Debug, Clone)]
pub struct Tool {
    /// Tool number
    pub number: u32,
    /// Tool name/description
    pub name: String,
    /// Tool length offset
    pub length_offset: f32,
    /// Tool diameter
    pub diameter: f32,
    /// Maximum RPM
    pub max_rpm: u32,
}

impl Tool {
    /// Create new tool
    pub fn new(number: u32, name: impl Into<String>) -> Self {
        Self {
            number,
            name: name.into(),
            length_offset: 0.0,
            diameter: 0.0,
            max_rpm: 12000,
        }
    }

    /// Set length offset
    pub fn with_length_offset(mut self, offset: f32) -> Self {
        self.length_offset = offset;
        self
    }

    /// Set diameter
    pub fn with_diameter(mut self, diameter: f32) -> Self {
        self.diameter = diameter;
        self
    }
}

/// Tool library
#[derive(Debug, Clone)]
pub struct ToolLibrary {
    /// Tools by number
    pub tools: HashMap<u32, Tool>,
    /// Current tool number
    pub current_tool: Option<u32>,
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
    pub fn add_tool(&mut self, tool: Tool) {
        self.tools.insert(tool.number, tool);
    }

    /// Get tool
    pub fn get_tool(&self, number: u32) -> Option<&Tool> {
        self.tools.get(&number)
    }

    /// Select tool
    pub fn select_tool(&mut self, number: u32) -> bool {
        if self.tools.contains_key(&number) {
            self.current_tool = Some(number);
            true
        } else {
            false
        }
    }

    /// Get current tool
    pub fn current(&self) -> Option<&Tool> {
        self.current_tool.and_then(|n| self.tools.get(&n))
    }
}

impl Default for ToolLibrary {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tasks 106: Work Coordinate Systems
// ============================================================================

/// Work coordinate offset
#[derive(Debug, Clone)]
pub struct CoordinateOffset {
    /// X offset
    pub x: f32,
    /// Y offset
    pub y: f32,
    /// Z offset
    pub z: f32,
}

impl CoordinateOffset {
    /// Create new offset
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Get as tuple
    pub fn as_tuple(&self) -> (f32, f32, f32) {
        (self.x, self.y, self.z)
    }
}

/// Work coordinate system manager
#[derive(Debug, Clone)]
pub struct WorkCoordinateManager {
    /// WCS offsets (G54-G59)
    pub offsets: HashMap<u32, CoordinateOffset>,
    /// Current WCS number
    pub current_wcs: u32,
}

impl WorkCoordinateManager {
    /// Create new WCS manager
    pub fn new() -> Self {
        let mut offsets = HashMap::new();
        for i in 1..=6 {
            offsets.insert(i, CoordinateOffset::new(0.0, 0.0, 0.0));
        }

        Self {
            offsets,
            current_wcs: 1,
        }
    }

    /// Get G-code for WCS
    pub fn get_gcode(&self, wcs: u32) -> String {
        match wcs {
            1..=6 => format!("G{}", 53 + wcs),
            _ => "Unknown".to_string(),
        }
    }

    /// Set WCS offset
    pub fn set_offset(&mut self, wcs: u32, offset: CoordinateOffset) {
        self.offsets.insert(wcs, offset);
    }

    /// Get current offset
    pub fn current_offset(&self) -> &CoordinateOffset {
        static DEFAULT: CoordinateOffset = CoordinateOffset {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        self.offsets.get(&self.current_wcs).unwrap_or(&DEFAULT)
    }

    /// Select WCS
    pub fn select(&mut self, wcs: u32) -> bool {
        if self.offsets.contains_key(&wcs) {
            self.current_wcs = wcs;
            true
        } else {
            false
        }
    }
}

impl Default for WorkCoordinateManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Task 107: Soft Limits
// ============================================================================

/// Soft limits configuration
#[derive(Debug, Clone)]
pub struct SoftLimits {
    /// X minimum
    pub x_min: f32,
    /// X maximum
    pub x_max: f32,
    /// Y minimum
    pub y_min: f32,
    /// Y maximum
    pub y_max: f32,
    /// Z minimum
    pub z_min: f32,
    /// Z maximum
    pub z_max: f32,
    /// Limits enabled
    pub enabled: bool,
}

impl SoftLimits {
    /// Create new soft limits
    pub fn new() -> Self {
        Self {
            x_min: 0.0,
            x_max: 100.0,
            y_min: 0.0,
            y_max: 100.0,
            z_min: -100.0,
            z_max: 0.0,
            enabled: true,
        }
    }

    /// Check if position is within limits
    pub fn check(&self, x: f32, y: f32, z: f32) -> bool {
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

    /// Get limit violations
    pub fn get_violations(&self, x: f32, y: f32, z: f32) -> Vec<String> {
        let mut violations = Vec::new();

        if !self.enabled {
            return violations;
        }

        if x < self.x_min {
            violations.push(format!("X below minimum: {} < {}", x, self.x_min));
        }
        if x > self.x_max {
            violations.push(format!("X above maximum: {} > {}", x, self.x_max));
        }
        if y < self.y_min {
            violations.push(format!("Y below minimum: {} < {}", y, self.y_min));
        }
        if y > self.y_max {
            violations.push(format!("Y above maximum: {} > {}", y, self.y_max));
        }
        if z < self.z_min {
            violations.push(format!("Z below minimum: {} < {}", z, self.z_min));
        }
        if z > self.z_max {
            violations.push(format!("Z above maximum: {} > {}", z, self.z_max));
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
// Task 108: Simulation Mode
// ============================================================================

/// Simulation mode
#[derive(Debug, Clone)]
pub struct SimulationMode {
    /// Is simulation active
    pub active: bool,
    /// Simulated position
    pub position: (f32, f32, f32),
    /// Commands executed in simulation
    pub commands_executed: usize,
    /// Total movement distance simulated
    pub distance_simulated: f32,
}

impl SimulationMode {
    /// Create new simulation mode
    pub fn new() -> Self {
        Self {
            active: false,
            position: (0.0, 0.0, 0.0),
            commands_executed: 0,
            distance_simulated: 0.0,
        }
    }

    /// Start simulation
    pub fn start(&mut self) {
        self.active = true;
        self.commands_executed = 0;
        self.distance_simulated = 0.0;
        self.position = (0.0, 0.0, 0.0);
    }

    /// Stop simulation
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Execute simulated move
    pub fn execute_move(&mut self, x: f32, y: f32, z: f32) {
        if self.active {
            let dx = x - self.position.0;
            let dy = y - self.position.1;
            let dz = z - self.position.2;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            self.distance_simulated += distance;
            self.position = (x, y, z);
            self.commands_executed += 1;
        }
    }

    /// Get simulation report
    pub fn get_report(&self) -> String {
        format!(
            "Simulation: {} commands, {:.2}mm moved, position ({:.2}, {:.2}, {:.2})",
            self.commands_executed,
            self.distance_simulated,
            self.position.0,
            self.position.1,
            self.position.2
        )
    }
}

impl Default for SimulationMode {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tasks 109-110: Step-Through and Breakpoints
// ============================================================================

/// Breakpoint
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Line number
    pub line_number: usize,
    /// Enabled
    pub enabled: bool,
}

impl Breakpoint {
    /// Create new breakpoint
    pub fn new(line_number: usize) -> Self {
        Self {
            line_number,
            enabled: true,
        }
    }
}

/// Step-through execution mode
#[derive(Debug, Clone)]
pub struct StepThroughMode {
    /// Current line
    pub current_line: usize,
    /// Total lines
    pub total_lines: usize,
    /// Paused at breakpoint
    pub paused: bool,
    /// Breakpoints
    pub breakpoints: Vec<Breakpoint>,
}

impl StepThroughMode {
    /// Create new step-through mode
    pub fn new(total_lines: usize) -> Self {
        Self {
            current_line: 0,
            total_lines,
            paused: false,
            breakpoints: Vec::new(),
        }
    }

    /// Add breakpoint
    pub fn add_breakpoint(&mut self, line: usize) {
        if line <= self.total_lines {
            self.breakpoints.push(Breakpoint::new(line));
        }
    }

    /// Remove breakpoint
    pub fn remove_breakpoint(&mut self, line: usize) {
        self.breakpoints.retain(|b| b.line_number != line);
    }

    /// Check for breakpoint
    pub fn check_breakpoint(&self, line: usize) -> bool {
        self.breakpoints
            .iter()
            .any(|b| b.line_number == line && b.enabled)
    }

    /// Step forward
    pub fn step_forward(&mut self) -> bool {
        if self.current_line < self.total_lines {
            self.current_line += 1;
            self.paused = self.check_breakpoint(self.current_line);
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
}

// ============================================================================
// Task 111: Program Restart
// ============================================================================

/// Program restart state
#[derive(Debug, Clone)]
pub struct ProgramRestartState {
    /// Line to restart from
    pub restart_line: usize,
    /// Saved modal state
    pub modal_state: HashMap<String, String>,
    /// Saved position
    pub position: (f32, f32, f32),
}

impl ProgramRestartState {
    /// Create new restart state
    pub fn new(restart_line: usize) -> Self {
        Self {
            restart_line,
            modal_state: HashMap::new(),
            position: (0.0, 0.0, 0.0),
        }
    }

    /// Save modal state
    pub fn save_modal(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.modal_state.insert(key.into(), value.into());
    }

    /// Set saved position
    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.position = (x, y, z);
    }
}

// ============================================================================
// Task 112: Performance Monitoring
// ============================================================================

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Commands per second
    pub commands_per_second: f32,
    /// Buffer usage percentage
    pub buffer_usage: f32,
    /// Peak buffer usage
    pub peak_buffer: f32,
    /// Average latency (ms)
    pub avg_latency: f32,
}

impl PerformanceMetrics {
    /// Create new performance metrics
    pub fn new() -> Self {
        Self {
            commands_per_second: 0.0,
            buffer_usage: 0.0,
            peak_buffer: 0.0,
            avg_latency: 0.0,
        }
    }

    /// Update metrics
    pub fn update(&mut self, cps: f32, buffer: f32, latency: f32) {
        self.commands_per_second = cps;
        self.buffer_usage = buffer;
        self.peak_buffer = self.peak_buffer.max(buffer);
        self.avg_latency = latency;
    }

    /// Get performance report
    pub fn get_report(&self) -> String {
        format!(
            "Performance: {:.1} cmd/s, Buffer: {:.1}% (peak {:.1}%), Latency: {:.1}ms",
            self.commands_per_second, self.buffer_usage, self.peak_buffer, self.avg_latency
        )
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Task 113: Command History
// ============================================================================

/// Command history entry
#[derive(Debug, Clone)]
pub struct CommandHistoryEntry {
    /// Command text
    pub command: String,
    /// Result (ok, error, etc)
    pub result: String,
    /// Execution time (ms)
    pub execution_time: f32,
}

impl CommandHistoryEntry {
    /// Create new history entry
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            result: String::new(),
            execution_time: 0.0,
        }
    }

    /// Set result
    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = result.into();
        self
    }
}

/// Command history manager
#[derive(Debug, Clone)]
pub struct CommandHistory {
    /// History entries
    pub entries: Vec<CommandHistoryEntry>,
    /// Maximum history size
    pub max_size: usize,
}

impl CommandHistory {
    /// Create new command history
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_size,
        }
    }

    /// Add to history
    pub fn add(&mut self, entry: CommandHistoryEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_size {
            self.entries.remove(0);
        }
    }

    /// Get history
    pub fn get_history(&self) -> &[CommandHistoryEntry] {
        &self.entries
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Resend command from history
    pub fn resend(&self, index: usize) -> Option<String> {
        self.entries.get(index).map(|e| e.command.clone())
    }
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new(1000)
    }
}
