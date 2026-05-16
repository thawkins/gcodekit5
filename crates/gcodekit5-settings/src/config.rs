//! Configuration and settings management for GCodeKit4
//!
//! Provides configuration file handling, settings management, and validation.
//! Supports JSON and TOML file formats stored in platform-specific directories.
//!
//! Configuration is organized into logical sections:
//! - Connection settings (ports, timeouts, protocols)
//! - UI preferences (theme, layout, fonts)
//! - File processing defaults (preprocessors, arc settings)
//! - Machine preferences (limits, jog settings)
//! - Firmware-specific settings

pub use gcodekit5_core::units::{FeedRateUnits, MeasurementSystem};
use gcodekit5_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Connection protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionType {
    /// Serial/USB connection
    Serial,
    /// TCP/IP connection
    Tcp,
    /// WebSocket connection
    WebSocket,
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serial => write!(f, "serial"),
            Self::Tcp => write!(f, "tcp"),
            Self::WebSocket => write!(f, "websocket"),
        }
    }
}

/// Connection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    /// Last used connection type
    pub connection_type: ConnectionType,
    /// Last used port (serial) or hostname (TCP/WebSocket)
    pub port: String,
    /// Baud rate for serial connections
    pub baud_rate: u32,
    /// TCP port for network connections
    pub tcp_port: u16,
    /// Connection timeout in milliseconds
    pub timeout_ms: u64,
    /// Auto-reconnect on connection loss
    pub auto_reconnect: bool,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            connection_type: ConnectionType::Serial,
            port: "Auto".to_string(),
            baud_rate: 115200,
            tcp_port: 8888,
            timeout_ms: 5000,
            auto_reconnect: true,
        }
    }
}

/// Theme selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow system preference
    #[default]
    System,
    /// Force light theme
    Light,
    /// Force dark theme
    Dark,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "System"),
            Self::Light => write!(f, "Light"),
            Self::Dark => write!(f, "Dark"),
        }
    }
}

// Measurement system selection
// Removed local definition, imported from gcodekit5_core::units

// Feed rate units selection
// Removed local definition, imported from gcodekit5_core::units

/// Startup tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StartupTab {
    #[default]
    Machine,
    Console,
    Editor,
    Visualizer,
    CamTools,
    Designer,
    DeviceInfo,
    Config,
    Devices,
    Tools,
    Materials,
}

impl std::fmt::Display for StartupTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Machine => write!(f, "Machine Control"),
            Self::Console => write!(f, "Device Console"),
            Self::Editor => write!(f, "G-Code Editor"),
            Self::Visualizer => write!(f, "Visualizer"),
            Self::CamTools => write!(f, "CAM Tools"),
            Self::Designer => write!(f, "Designer"),
            Self::DeviceInfo => write!(f, "Device Info"),
            Self::Config => write!(f, "Device Config"),
            Self::Devices => write!(f, "Device Manager"),
            Self::Tools => write!(f, "CNC Tools"),
            Self::Materials => write!(f, "Materials"),
        }
    }
}

/// UI preference settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    /// Window width
    pub window_width: u32,
    /// Window height
    pub window_height: u32,
    /// Whether panels are visible (by name)
    pub panel_visibility: HashMap<String, bool>,
    /// Selected theme (light/dark/system)
    #[serde(default)]
    pub theme: Theme,
    /// Font size in points
    pub font_size: u8,
    /// UI language code (e.g., "en", "es", "fr")
    pub language: String,
    /// Show keyboard shortcuts in menus
    pub show_menu_shortcuts: bool,
    /// Measurement system (Metric or Imperial)
    #[serde(default)]
    pub measurement_system: MeasurementSystem,
    /// Feed rate units preference
    #[serde(default)]
    pub feed_rate_units: FeedRateUnits,
    /// Startup tab
    #[serde(default)]
    pub startup_tab: StartupTab,

    /// Show the About dialog on startup (auto-closes after a short delay)
    #[serde(default)]
    pub show_about_on_startup: bool,

    /// Enable 3D stock removal visualization (experimental)
    #[serde(default)]
    pub enable_stock_removal_3d: bool,

    /// Enable STL file import with shadow projection (experimental)
    #[serde(default)]
    pub enable_stl_import: bool,
    /// Visualizer sidebar splitter position (Paned position in pixels)
    #[serde(default)]
    pub visualizer_sidebar_position: Option<i32>,

    /// Device Config info sidebar splitter position (Paned position in pixels)
    #[serde(default)]
    pub device_config_sidebar_position: Option<i32>,

    /// Tools tab sidebar splitter position (Paned position in pixels)
    #[serde(default)]
    pub tools_sidebar_position: Option<i32>,

    /// Tools tab search text
    #[serde(default)]
    pub tools_manager_search: String,

    /// Tools tab type filter active id
    #[serde(default)]
    pub tools_manager_type_filter: String,

    /// Tools tab material filter active id
    #[serde(default)]
    pub tools_manager_material_filter: String,

    /// Tools tab diameter min filter (mm)
    #[serde(default)]
    pub tools_manager_dia_min: Option<f32>,

    /// Tools tab diameter max filter (mm)
    #[serde(default)]
    pub tools_manager_dia_max: Option<f32>,

    /// Tools tab last selected tool id
    #[serde(default)]
    pub tools_manager_selected_tool: Option<String>,

    /// Grid major line width in pixels (coarse grid lines)
    #[serde(default = "default_grid_major_line_width")]
    pub grid_major_line_width: f64,

    /// Grid minor line width in pixels (fine grid lines)
    #[serde(default = "default_grid_minor_line_width")]
    pub grid_minor_line_width: f64,
}

impl Default for UiSettings {
    fn default() -> Self {
        let mut visibility = HashMap::new();
        visibility.insert("connection".to_string(), true);
        visibility.insert("dro".to_string(), true);
        visibility.insert("jog".to_string(), true);
        visibility.insert("console".to_string(), true);
        visibility.insert("visualizer_sidebar".to_string(), true);
        visibility.insert("device_config_sidebar".to_string(), true);

        Self {
            window_width: 1400,
            window_height: 900,
            panel_visibility: visibility,
            theme: Theme::default(),
            font_size: 12,
            language: "en".to_string(),
            show_menu_shortcuts: true,
            measurement_system: MeasurementSystem::default(),
            feed_rate_units: FeedRateUnits::default(),
            startup_tab: StartupTab::default(),
            show_about_on_startup: false,
            enable_stock_removal_3d: false,
            enable_stl_import: false,
            visualizer_sidebar_position: None,
            device_config_sidebar_position: None,
            tools_sidebar_position: None,
            tools_manager_search: String::new(),
            tools_manager_type_filter: String::new(),
            tools_manager_material_filter: String::new(),
            tools_manager_dia_min: None,
            tools_manager_dia_max: None,
            tools_manager_selected_tool: None,
            grid_major_line_width: 2.0,
            grid_minor_line_width: 1.0,
        }
    }
}

/// Default value for grid major line width
fn default_grid_major_line_width() -> f64 {
    2.0
}

/// Default value for grid minor line width
fn default_grid_minor_line_width() -> f64 {
    1.0
}

/// File processing settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProcessingSettings {
    /// Default preprocessors to enable
    pub enabled_preprocessors: Vec<String>,
    /// Arc segment length in mm
    pub arc_segment_length: f64,
    /// Maximum line length in characters
    pub max_line_length: u32,
    /// Whether to preserve comments
    pub preserve_comments: bool,
    /// Default output directory
    pub output_directory: PathBuf,
    /// Number of recent files to track
    pub recent_files_count: usize,
    /// Whether to add N[nnn] line numbers in generated G-code
    pub line_numbers_enabled: bool,
}

impl Default for FileProcessingSettings {
    fn default() -> Self {
        Self {
            enabled_preprocessors: vec![
                "comment_remover".to_string(),
                "whitespace_cleaner".to_string(),
            ],
            arc_segment_length: 0.5,
            max_line_length: 256,
            preserve_comments: false,
            output_directory: dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")),
            recent_files_count: 10,
            line_numbers_enabled: false,
        }
    }
}

/// Machine preference settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineSettings {
    /// Default jog increment in mm (linear axes)
    pub jog_increment: f64,
    /// Default jog increment in degrees (rotary axes)
    pub jog_increment_rotary: f64,
    /// Default jog feed rate in units/min
    pub jog_feed_rate: f64,
    /// Machine X limit (max)
    pub x_limit: f64,
    /// Machine Y limit (max)
    pub y_limit: f64,
    /// Machine Z limit (max)
    pub z_limit: f64,
    /// Machine A limit (max degrees)
    pub a_limit: f64,
    /// Machine B limit (max degrees)
    pub b_limit: f64,
    /// Machine C limit (max degrees)
    pub c_limit: f64,
    /// Default unit (mm or in)
    pub default_unit: String,
    /// Homing direction per axis (true = negative, false = positive)
    pub homing_direction: HashMap<String, bool>,
    /// Direction inversion per axis (true = inverted)
    pub axis_direction_invert: HashMap<String, bool>,
    /// Steps per degree for rotary axes (A, B, C)
    pub steps_per_degree: HashMap<String, f64>,
    /// Rotary axis calibration offsets (degrees)
    pub rotary_calibration: HashMap<String, f64>,
    /// Rotary axis backlash compensation (degrees)
    pub rotary_backlash: HashMap<String, f64>,
}

impl Default for MachineSettings {
    fn default() -> Self {
        let mut homing = HashMap::new();
        homing.insert("X".to_string(), true);
        homing.insert("Y".to_string(), true);
        homing.insert("Z".to_string(), true);
        homing.insert("A".to_string(), true);
        homing.insert("B".to_string(), true);
        homing.insert("C".to_string(), true);

        let mut direction_invert = HashMap::new();
        direction_invert.insert("X".to_string(), false);
        direction_invert.insert("Y".to_string(), false);
        direction_invert.insert("Z".to_string(), false);
        direction_invert.insert("A".to_string(), false);
        direction_invert.insert("B".to_string(), false);
        direction_invert.insert("C".to_string(), false);

        let mut steps_per_deg = HashMap::new();
        steps_per_deg.insert("A".to_string(), 80.0); // Common NEMA 23 stepper with 8:1 gearbox
        steps_per_deg.insert("B".to_string(), 80.0);
        steps_per_deg.insert("C".to_string(), 80.0);

        let mut rotary_cal = HashMap::new();
        rotary_cal.insert("A".to_string(), 0.0);
        rotary_cal.insert("B".to_string(), 0.0);
        rotary_cal.insert("C".to_string(), 0.0);

        let mut rotary_back = HashMap::new();
        rotary_back.insert("A".to_string(), 0.0);
        rotary_back.insert("B".to_string(), 0.0);
        rotary_back.insert("C".to_string(), 0.0);

        Self {
            jog_increment: 1.0,
            jog_increment_rotary: 1.0,
            jog_feed_rate: 1000.0,
            x_limit: 200.0,
            y_limit: 200.0,
            z_limit: 100.0,
            a_limit: 360.0,
            b_limit: 360.0,
            c_limit: 360.0,
            default_unit: "mm".to_string(),
            homing_direction: homing,
            axis_direction_invert: direction_invert,
            steps_per_degree: steps_per_deg,
            rotary_calibration: rotary_cal,
            rotary_backlash: rotary_back,
        }
    }
}

/// Probe settings for WCS auto-update and persistent storage.
///
/// Stores probe default parameters and last probe results for each WCS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSettings {
    /// Default safe height for probe operations (mm).
    pub default_safe_height: f64,
    /// Default fast feed rate for probing (units/min).
    pub default_fast_feed: f64,
    /// Default slow feed rate for accuracy re-probe (units/min).
    pub default_slow_feed: f64,
    /// Default backoff distance after initial probe (mm).
    pub default_backoff: f64,
    /// Default maximum probe depth (mm).
    pub default_max_depth: f64,
    /// Auto-update WCS after successful probe.
    pub auto_update_wcs: bool,
    /// Target WCS for probe results (54-59 for G54-G59, 0 for G92).
    pub target_wcs: u8,
    /// Use temporary G92 offset instead of persistent G10.
    pub use_temporary_offset: bool,
    /// Apply tool-length offset via G43.1 for tool length probes.
    pub apply_tool_length: bool,
    /// Preview probe results before applying WCS update.
    pub preview_before_apply: bool,
    /// Setter plate X position (machine coordinates).
    pub setter_plate_x: f64,
    /// Setter plate Y position (machine coordinates).
    pub setter_plate_y: f64,
    /// Setter plate Z position (machine coordinates).
    pub setter_plate_z: f64,
}

impl ProbeSettings {
    /// Create new probe settings with defaults.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ProbeSettings {
    fn default() -> Self {
        Self {
            default_safe_height: 10.0,
            default_fast_feed: 100.0,
            default_slow_feed: 20.0,
            default_backoff: 2.0,
            default_max_depth: -10.0,
            auto_update_wcs: true,
            target_wcs: 54,
            use_temporary_offset: false,
            apply_tool_length: false,
            preview_before_apply: true,
            setter_plate_x: 0.0,
            setter_plate_y: 0.0,
            setter_plate_z: 0.0,
        }
    }
}

/// Firmware-specific settings trait
///
/// Allows firmware implementations to define their own settings structure.
pub trait FirmwareSettings: Serialize {
    /// Get the firmware name
    fn firmware_name(&self) -> &str;

    /// Validate the settings
    fn validate(&self) -> Result<()>;

    /// Get setting by key
    fn get_setting(&self, key: &str) -> Option<String>;

    /// Set setting by key
    fn set_setting(&mut self, key: &str, value: String) -> Result<()>;
}

/// Complete application configuration
///
/// Aggregates all settings sections and provides file I/O operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Connection settings
    pub connection: ConnectionSettings,
    /// UI preferences
    pub ui: UiSettings,
    /// File processing settings
    pub file_processing: FileProcessingSettings,
    /// Machine preferences
    pub machine: MachineSettings,
    /// Probe settings and persistent results
    #[serde(default)]
    pub probe: ProbeSettings,
    /// Recent files list
    pub recent_files: Vec<PathBuf>,
    /// Last Path
    pub last_opened_path: Option<String>,
}

impl Config {
    /// Create new config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Load config from file (JSON or TOML)
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::other(format!("Failed to read config file: {}", e)))?;

        let config: Self = if path.extension().is_some_and(|ext| ext == "json") {
            serde_json::from_str(&content)
                .map_err(|e| Error::other(format!("Invalid JSON config: {}", e)))?
        } else if path.extension().is_some_and(|ext| ext == "toml") {
            toml::from_str(&content)
                .map_err(|e| Error::other(format!("Invalid TOML config: {}", e)))?
        } else {
            return Err(Error::other(
                "Config file must be .json or .toml".to_string(),
            ));
        };

        config.validate()?;
        Ok(config)
    }

    /// Save config to file (JSON or TOML)
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        self.validate()?;

        let content = if path.extension().is_some_and(|ext| ext == "json") {
            serde_json::to_string_pretty(self)
                .map_err(|e| Error::other(format!("Failed to serialize config: {}", e)))?
        } else if path.extension().is_some_and(|ext| ext == "toml") {
            toml::to_string_pretty(self)
                .map_err(|e| Error::other(format!("Failed to serialize config: {}", e)))?
        } else {
            return Err(Error::other(
                "Config file must be .json or .toml".to_string(),
            ));
        };

        std::fs::write(path, content)
            .map_err(|e| Error::other(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate connection settings
        if self.connection.timeout_ms == 0 {
            return Err(Error::other("Connection timeout must be > 0".to_string()));
        }

        if self.connection.baud_rate == 0 {
            return Err(Error::other("Baud rate must be > 0".to_string()));
        }

        // Validate UI settings
        if self.ui.window_width == 0 || self.ui.window_height == 0 {
            return Err(Error::other("Window dimensions must be > 0".to_string()));
        }

        if self.ui.font_size == 0 {
            return Err(Error::other("Font size must be > 0".to_string()));
        }

        // Validate file processing
        if self.file_processing.arc_segment_length <= 0.0 {
            return Err(Error::other("Arc segment length must be > 0".to_string()));
        }

        if self.file_processing.max_line_length == 0 {
            return Err(Error::other("Max line length must be > 0".to_string()));
        }

        // Validate machine settings
        if self.machine.jog_feed_rate <= 0.0 {
            return Err(Error::other("Jog feed rate must be > 0".to_string()));
        }

        if self.machine.x_limit <= 0.0 || self.machine.y_limit <= 0.0 || self.machine.z_limit <= 0.0
        {
            return Err(Error::other("Machine limits must be > 0".to_string()));
        }

        // Validate rotary axis limits (can be 0 if axis is not used)
        if self.machine.a_limit < 0.0 || self.machine.b_limit < 0.0 || self.machine.c_limit < 0.0 {
            return Err(Error::other("Rotary axis limits must be >= 0".to_string()));
        }

        // Validate steps per degree
        for (axis, steps) in &self.machine.steps_per_degree {
            if *steps <= 0.0 {
                return Err(Error::other(format!(
                    "Steps per degree for axis {} must be > 0",
                    axis
                )));
            }
        }
        // Validate probe settings
        if self.probe.default_fast_feed <= 0.0 {
            return Err(Error::other("Probe fast feed rate must be > 0".to_string()));
        }

        if self.probe.default_slow_feed <= 0.0 {
            return Err(Error::other("Probe slow feed rate must be > 0".to_string()));
        }

        if self.probe.default_safe_height < 0.0 {
            return Err(Error::other("Probe safe height must be >= 0".to_string()));
        }

        Ok(())
    }

    /// Add file to recent files list
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Remove if already in list
        self.recent_files.retain(|f| f != &path);

        // Add to front
        self.recent_files.insert(0, path);

        // Trim to max size
        self.recent_files
            .truncate(self.file_processing.recent_files_count);
    }

    /// Merge another config into this one (preserves existing values for missing sections)
    pub fn merge(&mut self, other: &Config) {
        // Only merge non-zero/non-default values
        if other.connection.timeout_ms > 0 {
            self.connection = other.connection.clone();
        }
        // Theme is an enum, so we can't check for empty string.
        // We assume if it's not default (System), we might want to merge it,
        // but merging logic is tricky with enums.
        // For now, let's assume if the other config has a specific theme set (not System), we take it.
        if other.ui.theme != Theme::System {
            self.ui = other.ui.clone();
        }
        if other.file_processing.arc_segment_length > 0.0 {
            self.file_processing = other.file_processing.clone();
        }
        if other.machine.jog_feed_rate > 0.0 {
            self.machine = other.machine.clone();
        }
    }
}
