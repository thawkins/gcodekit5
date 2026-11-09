//! GRBL Protocol Constants and Capabilities
//!
//! This module defines all GRBL-specific constants, capabilities, error codes,
//! alarm codes, and feature flags according to the GRBL protocol specification.

// GRBL Version Info
/// Minimum supported GRBL version (1.1.0)
pub const GRBL_MIN_VERSION: &str = "1.1.0";

/// GRBL 1.2.x version string pattern
pub const GRBL_VERSION_1_2: &str = "Grbl 1.2";

/// GRBL 1.1.x version string pattern
pub const GRBL_VERSION_1_1: &str = "Grbl 1.1";

/// GRBL 0.9.x version string pattern  
pub const GRBL_VERSION_0_9: &str = "Grbl 0.9";

// GRBL Capabilities
/// Maximum number of axes supported by GRBL
pub const GRBL_MAX_AXES: u8 = 6;

/// Default buffer size for GRBL
pub const GRBL_DEFAULT_BUFFER_SIZE: usize = 128;

/// GRBL block size (maximum command length)
pub const GRBL_MAX_BLOCK_SIZE: usize = 256;

/// Default serial communication speed (baud rate)
pub const GRBL_DEFAULT_BAUD_RATE: u32 = 115200;

/// Alternative baud rates supported
pub const GRBL_ALT_BAUD_RATES: &[u32] = &[9600, 19200, 38400, 57600];

// GRBL Real-Time Commands
/// Query status command (?)
pub const CMD_QUERY_STATUS: u8 = b'?';

/// Feed hold command (!)
pub const CMD_FEED_HOLD: u8 = b'!';

/// Cycle start/resume command (~)
pub const CMD_CYCLE_START: u8 = b'~';

/// Soft reset command (Ctrl+X = 0x18)
pub const CMD_SOFT_RESET: u8 = 0x18;

/// Door open alarm button (Ctrl+D = 0x04)
pub const CMD_DOOR_ALARM: u8 = 0x04;

/// Safety door toggle (Ctrl+Shift+D = 0x84)
pub const CMD_SAFETY_DOOR: u8 = 0x84;

// GRBL Status Report Codes
/// GRBL is IDLE
pub const STATUS_IDLE: &str = "Idle";

/// GRBL is RUNNING
pub const STATUS_RUNNING: &str = "Run";

/// GRBL is HOLDING (feed hold)
pub const STATUS_HOLD: &str = "Hold";

/// GRBL is in JOG mode
pub const STATUS_JOG: &str = "Jog";

/// GRBL has an ALARM
pub const STATUS_ALARM: &str = "Alarm";

/// GRBL is in CHECK mode
pub const STATUS_CHECK: &str = "Check";

/// GRBL DOOR state
pub const STATUS_DOOR: &str = "Door";

/// GRBL is SLEEPING
pub const STATUS_SLEEP: &str = "Sleep";

// GRBL System Settings ($)
/// Minimum baud rate setting
pub const SETTING_BAUD_RATE: u8 = 110;

/// Idle delay after laser off (ms)
pub const SETTING_IDLE_DELAY: u8 = 111;

/// Step idle delay (µs)
pub const SETTING_STEP_IDLE_DELAY: u8 = 112;

/// Step pulse duration (µs)
pub const SETTING_STEP_PULSE_DURATION: u8 = 113;

/// Direction invert mask
pub const SETTING_DIR_INVERT: u8 = 114;

/// Step enable invert flag
pub const SETTING_STEP_ENABLE_INVERT: u8 = 115;

/// Limit pins invert flag
pub const SETTING_LIMIT_PINS_INVERT: u8 = 116;

/// Probe pin invert flag
pub const SETTING_PROBE_PIN_INVERT: u8 = 117;

/// Status report mask
pub const SETTING_STATUS_REPORT_MASK: u8 = 118;

/// Junction deviation setting
pub const SETTING_JUNCTION_DEVIATION: u8 = 120;

/// Arc tolerance setting
pub const SETTING_ARC_TOLERANCE: u8 = 121;

/// Report inches setting
pub const SETTING_REPORT_INCHES: u8 = 122;

/// Control invert mask
pub const SETTING_CONTROL_INVERT_MASK: u8 = 123;

/// Limit invert mask
pub const SETTING_LIMIT_INVERT_MASK: u8 = 124;

/// Spindle invert mask
pub const SETTING_SPINDLE_INVERT_MASK: u8 = 125;

/// Control pull-up disable mask
pub const SETTING_CONTROL_PULL_UP_DISABLE: u8 = 126;

/// Limit pull-up disable mask
pub const SETTING_LIMIT_PULL_UP_DISABLE: u8 = 127;

/// Probe pull-up disable flag
pub const SETTING_PROBE_PULL_UP_DISABLE: u8 = 128;

// GRBL Error Codes (1-50)
/// Expected command letter
pub const ERROR_EXPECTED_COMMAND_LETTER: u8 = 1;

/// Bad number format
pub const ERROR_BAD_NUMBER_FORMAT: u8 = 2;

/// Invalid statement
pub const ERROR_INVALID_STATEMENT: u8 = 3;

/// Negative value
pub const ERROR_NEGATIVE_VALUE: u8 = 4;

/// Setting disabled
pub const ERROR_SETTING_DISABLED: u8 = 5;

/// Failed to execute startup block
pub const ERROR_STARTUP_BLOCK_FAILED: u8 = 23;

/// EEPROM read failed (GRBL 1.1 error:7)
pub const ERROR_EEPROM_READ_FAILED: u8 = 7;

/// Unsupported or invalid g-code command
pub const ERROR_INVALID_GCODE_ID: u8 = 20;

/// Modal group violation
pub const ERROR_MODAL_GROUP_VIOLATION: u8 = 21;

/// Undefined feed rate
pub const ERROR_UNDEFINED_FEED_RATE: u8 = 22;

// GRBL Alarm Codes (1-10)
/// Hard limit triggered
pub const ALARM_HARD_LIMIT: u8 = 1;

/// Soft limit exceeded
pub const ALARM_SOFT_LIMIT: u8 = 2;

/// Abort during cycle
pub const ALARM_ABORT_CYCLE: u8 = 3;

/// Probe fail
pub const ALARM_PROBE_FAIL: u8 = 4;

/// Probe not triggered
pub const ALARM_PROBE_NOT_TRIGGERED: u8 = 5;

/// Homing fail
pub const ALARM_HOMING_FAIL: u8 = 6;

/// Homing fail pulloff (GRBL 1.1 ALARM:8)
pub const ALARM_HOMING_FAIL_PULLOFF: u8 = 8;

/// Spindle control failure
pub const ALARM_SPINDLE_CONTROL: u8 = 8;

/// Cooling mist control failure
pub const ALARM_COOLING_MIST_CONTROL: u8 = 9;

// Coordinate Systems (G54-G59)
/// G54 Work Coordinate System
pub const COORD_SYS_G54: &str = "G54";

/// G55 Work Coordinate System
pub const COORD_SYS_G55: &str = "G55";

/// G56 Work Coordinate System
pub const COORD_SYS_G56: &str = "G56";

/// G57 Work Coordinate System
pub const COORD_SYS_G57: &str = "G57";

/// G58 Work Coordinate System
pub const COORD_SYS_G58: &str = "G58";

/// G59 Work Coordinate System
pub const COORD_SYS_G59: &str = "G59";

/// G59.1 Extended Work Coordinate System
pub const COORD_SYS_G59_1: &str = "G59.1";

/// G59.2 Extended Work Coordinate System
pub const COORD_SYS_G59_2: &str = "G59.2";

/// G59.3 Extended Work Coordinate System
pub const COORD_SYS_G59_3: &str = "G59.3";

// G-Code Groups
/// Motion Mode (G0, G1, G2, G3, G80)
pub const GCODE_GROUP_MOTION: u8 = 1;

/// Coordinate System (G54-G59, G59.1-G59.3)
pub const GCODE_GROUP_COORD_SYSTEM: u8 = 12;

/// Plane Selection (G17, G18, G19)
pub const GCODE_GROUP_PLANE: u8 = 2;

/// Distance Mode (G90, G91)
pub const GCODE_GROUP_DISTANCE: u8 = 3;

/// Feed Rate Mode (G93, G94)
pub const GCODE_GROUP_FEED_RATE: u8 = 5;

/// Units (G20, G21)
pub const GCODE_GROUP_UNITS: u8 = 6;

/// Tool Offset (G43, G49)
pub const GCODE_GROUP_TOOL_OFFSET: u8 = 7;

/// Cutter Compensation (G40, G41, G42)
pub const GCODE_GROUP_COMPENSATION: u8 = 8;

// Feature Flags (bit positions)
/// Supports EEPROM settings
pub const FEATURE_EEPROM: u32 = 1 << 0;

/// Supports spindle control
pub const FEATURE_SPINDLE: u32 = 1 << 1;

/// Supports cool ant control
pub const FEATURE_COOLANT: u32 = 1 << 2;

/// Supports SD card (FluidNC)
pub const FEATURE_SD_CARD: u32 = 1 << 3;

/// Supports safety door
pub const FEATURE_SAFETY_DOOR: u32 = 1 << 4;

/// Supports homing force origin
pub const FEATURE_FORCE_ORIGIN: u32 = 1 << 5;

/// Supports DRO position display
pub const FEATURE_DRO: u32 = 1 << 6;

/// Supports ASCII character feedback
pub const FEATURE_ASCII_FEEDBACK: u32 = 1 << 7;

/// Supports new startup blocks
pub const FEATURE_STARTUP_BLOCK: u32 = 1 << 8;

/// Supports build info command
pub const FEATURE_BUILD_INFO: u32 = 1 << 9;

/// Supports restoring alarm lock
pub const FEATURE_RESTORE_ALARM_LOCK: u32 = 1 << 10;
