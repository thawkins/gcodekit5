//! Controller State Panel (DRO) - Task 69
//!
//! Digital Readout (DRO) for displaying machine and work coordinates

/// Unit system for coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitSystem {
    /// Millimeters
    Millimeters,
    /// Inches
    Inches,
}

impl std::fmt::Display for UnitSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Millimeters => write!(f, "mm"),
            Self::Inches => write!(f, "in"),
        }
    }
}

impl UnitSystem {
    /// Convert millimeters to this unit
    pub fn from_mm(&self, mm: f32) -> f32 {
        match self {
            Self::Millimeters => mm,
            Self::Inches => mm / 25.4,
        }
    }

    /// Convert from this unit to millimeters
    pub fn to_mm(&self, value: f32) -> f32 {
        match self {
            Self::Millimeters => value,
            Self::Inches => value * 25.4,
        }
    }
}

/// Coordinate system (G54-G59)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateSystem {
    /// G54 - Work coordinate system 1
    G54,
    /// G55 - Work coordinate system 2
    G55,
    /// G56 - Work coordinate system 3
    G56,
    /// G57 - Work coordinate system 4
    G57,
    /// G58 - Work coordinate system 5
    G58,
    /// G59 - Work coordinate system 6
    G59,
}

impl std::fmt::Display for CoordinateSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::G54 => write!(f, "G54"),
            Self::G55 => write!(f, "G55"),
            Self::G56 => write!(f, "G56"),
            Self::G57 => write!(f, "G57"),
            Self::G58 => write!(f, "G58"),
            Self::G59 => write!(f, "G59"),
        }
    }
}

impl CoordinateSystem {
    /// Get all coordinate systems
    pub fn all() -> Vec<Self> {
        vec![
            Self::G54,
            Self::G55,
            Self::G56,
            Self::G57,
            Self::G58,
            Self::G59,
        ]
    }
}

/// Machine position (absolute coordinates)
#[derive(Debug, Clone, Copy, Default)]
pub struct MachinePosition {
    /// X axis position
    pub x: f32,
    /// Y axis position
    pub y: f32,
    /// Z axis position
    pub z: f32,
    /// A axis position (rotary around X)
    pub a: f32,
    /// B axis position (rotary around Y, 5th axis)
    pub b: f32,
    /// C axis position (rotary around Z, 6th axis)
    pub c: f32,
}

impl MachinePosition {
    /// Create new machine position
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, a: 0.0, b: 0.0, c: 0.0 }
    }

    /// Get formatted string in given units (X, Y, Z only)
    pub fn formatted(&self, units: UnitSystem) -> String {
        let x = units.from_mm(self.x);
        let y = units.from_mm(self.y);
        let z = units.from_mm(self.z);
        format!("X:{:.2} Y:{:.2} Z:{:.2}", x, y, z)
    }

    /// Get formatted string for 6-axis display with conditional axes
    /// axis_count: 3=XYZ, 4=XYZA, 5=XYZAB, 6=XYZABC
    pub fn formatted_6axis(&self, units: UnitSystem, axis_count: u8) -> String {
        let x = units.from_mm(self.x);
        let y = units.from_mm(self.y);
        let z = units.from_mm(self.z);
        
        match axis_count {
            3 => format!("X:{:.2} Y:{:.2} Z:{:.2}", x, y, z),
            4 => format!("X:{:.2} Y:{:.2} Z:{:.2} A:{:.2}°", x, y, z, self.a),
            5 => format!("X:{:.2} Y:{:.2} Z:{:.2} A:{:.2}° B:{:.2}°", x, y, z, self.a, self.b),
            _ => format!("X:{:.2} Y:{:.2} Z:{:.2} A:{:.2}° B:{:.2}° C:{:.2}°", x, y, z, self.a, self.b, self.c),
        }
    }
}

/// Work position (relative to zero/offset)
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkPosition {
    /// X axis offset
    pub x: f32,
    /// Y axis offset
    pub y: f32,
    /// Z axis offset
    pub z: f32,
    /// A axis offset (rotary around X)
    pub a: f32,
    /// B axis offset (rotary around Y, 5th axis)
    pub b: f32,
    /// C axis offset (rotary around Z, 6th axis)
    pub c: f32,
}

impl WorkPosition {
    /// Create new work position
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, a: 0.0, b: 0.0, c: 0.0 }
    }

    /// Get formatted string in given units (X, Y, Z only)
    pub fn formatted(&self, units: UnitSystem) -> String {
        let x = units.from_mm(self.x);
        let y = units.from_mm(self.y);
        let z = units.from_mm(self.z);
        format!("X:{:.2} Y:{:.2} Z:{:.2}", x, y, z)
    }

    /// Get formatted string for 6-axis display with conditional axes
    /// axis_count: 3=XYZ, 4=XYZA, 5=XYZAB, 6=XYZABC
    pub fn formatted_6axis(&self, units: UnitSystem, axis_count: u8) -> String {
        let x = units.from_mm(self.x);
        let y = units.from_mm(self.y);
        let z = units.from_mm(self.z);
        
        match axis_count {
            3 => format!("X:{:.2} Y:{:.2} Z:{:.2}", x, y, z),
            4 => format!("X:{:.2} Y:{:.2} Z:{:.2} A:{:.2}°", x, y, z, self.a),
            5 => format!("X:{:.2} Y:{:.2} Z:{:.2} A:{:.2}° B:{:.2}°", x, y, z, self.a, self.b),
            _ => format!("X:{:.2} Y:{:.2} Z:{:.2} A:{:.2}° B:{:.2}° C:{:.2}°", x, y, z, self.a, self.b, self.c),
        }
    }

    /// Zero out all axes
    pub fn zero(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
        self.z = 0.0;
        self.a = 0.0;
        self.b = 0.0;
        self.c = 0.0;
    }

    /// Zero a specific axis
    pub fn zero_axis(&mut self, axis: char) {
        match axis {
            'X' | 'x' => self.x = 0.0,
            'Y' | 'y' => self.y = 0.0,
            'Z' | 'z' => self.z = 0.0,
            'A' | 'a' => self.a = 0.0,
            'B' | 'b' => self.b = 0.0,
            'C' | 'c' => self.c = 0.0,
            _ => {}
        }
    }
}

/// Controller state for DRO display
#[derive(Debug, Clone)]
pub struct ControllerDRO {
    /// Machine position
    pub machine_pos: MachinePosition,
    /// Work position
    pub work_pos: WorkPosition,
    /// Current coordinate system
    pub coordinate_system: CoordinateSystem,
    /// Unit system (mm or inches)
    pub units: UnitSystem,
    /// Feed rate (units/min)
    pub feed_rate: f32,
    /// Spindle speed (RPM)
    pub spindle_speed: u16,
    /// Is spindle running
    pub spindle_running: bool,
    /// Machine state
    pub machine_state: String,
}

impl ControllerDRO {
    /// Create new controller DRO
    pub fn new() -> Self {
        Self {
            machine_pos: MachinePosition::default(),
            work_pos: WorkPosition::default(),
            coordinate_system: CoordinateSystem::G54,
            units: UnitSystem::Millimeters,
            feed_rate: 0.0,
            spindle_speed: 0,
            spindle_running: false,
            machine_state: "Idle".to_string(),
        }
    }

    /// Update machine position
    pub fn update_position(&mut self, x: f32, y: f32, z: f32) {
        self.machine_pos = MachinePosition::new(x, y, z);
    }

    /// Update work position
    pub fn update_work_position(&mut self, x: f32, y: f32, z: f32) {
        self.work_pos = WorkPosition::new(x, y, z);
    }

    /// Update feed rate
    pub fn update_feed_rate(&mut self, rate: f32) {
        self.feed_rate = rate;
    }

    /// Update spindle speed
    pub fn update_spindle_speed(&mut self, speed: u16, running: bool) {
        self.spindle_speed = speed;
        self.spindle_running = running;
    }

    /// Set coordinate system
    pub fn set_coordinate_system(&mut self, cs: CoordinateSystem) {
        self.coordinate_system = cs;
    }

    /// Switch units
    pub fn toggle_units(&mut self) {
        self.units = match self.units {
            UnitSystem::Millimeters => UnitSystem::Inches,
            UnitSystem::Inches => UnitSystem::Millimeters,
        };
    }

    /// Zero work coordinates
    pub fn zero_work_coordinates(&mut self) {
        self.work_pos.zero();
    }

    /// Zero single work axis
    pub fn zero_work_axis(&mut self, axis: char) {
        self.work_pos.zero_axis(axis);
    }

    /// Update machine state
    pub fn update_machine_state(&mut self, state: impl Into<String>) {
        self.machine_state = state.into();
    }

    /// Get DRO display string
    pub fn display_string(&self) -> String {
        format!(
            "[{}] Machine: {} | Work: {} | Feed: {:.1} | Spindle: {} ({})",
            self.coordinate_system,
            self.machine_pos.formatted(self.units),
            self.work_pos.formatted(self.units),
            self.feed_rate,
            if self.spindle_running { "ON" } else { "OFF" },
            self.spindle_speed
        )
    }
}

impl Default for ControllerDRO {
    fn default() -> Self {
        Self::new()
    }
}

/// DRO Panel UI component
#[derive(Debug)]
pub struct DROPanel {
    /// Controller DRO data
    pub dro: ControllerDRO,
    /// Show machine position
    pub show_machine_pos: bool,
    /// Show work position
    pub show_work_pos: bool,
    /// Large display mode
    pub large_display: bool,
}

impl DROPanel {
    /// Create new DRO panel
    pub fn new() -> Self {
        Self {
            dro: ControllerDRO::new(),
            show_machine_pos: true,
            show_work_pos: true,
            large_display: false,
        }
    }

    /// Toggle position display mode
    pub fn toggle_position_mode(&mut self) {
        self.show_work_pos = !self.show_work_pos;
    }

    /// Toggle large display
    pub fn toggle_large_display(&mut self) {
        self.large_display = !self.large_display;
    }

    /// Get display format for current settings
    pub fn get_display(&self) -> (String, String, String) {
        let x_display = if self.show_work_pos {
            format!("X: {:.2}", self.dro.units.from_mm(self.dro.work_pos.x))
        } else {
            format!("X: {:.2}", self.dro.units.from_mm(self.dro.machine_pos.x))
        };

        let y_display = if self.show_work_pos {
            format!("Y: {:.2}", self.dro.units.from_mm(self.dro.work_pos.y))
        } else {
            format!("Y: {:.2}", self.dro.units.from_mm(self.dro.machine_pos.y))
        };

        let z_display = if self.show_work_pos {
            format!("Z: {:.2}", self.dro.units.from_mm(self.dro.work_pos.z))
        } else {
            format!("Z: {:.2}", self.dro.units.from_mm(self.dro.machine_pos.z))
        };

        (x_display, y_display, z_display)
    }
}

impl Default for DROPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_conversion() {
        assert_eq!(UnitSystem::Millimeters.from_mm(25.4), 25.4);
        assert!(UnitSystem::Inches.from_mm(25.4) - 1.0 < 0.01);
    }

    #[test]
    fn test_machine_position() {
        let pos = MachinePosition::new(10.0, 20.0, 5.0);
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
        assert_eq!(pos.z, 5.0);
    }

    #[test]
    fn test_work_position_zero() {
        let mut pos = WorkPosition::new(10.0, 20.0, 5.0);
        pos.zero_axis('X');
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 20.0);
    }

    #[test]
    fn test_controller_dro() {
        let mut dro = ControllerDRO::new();
        dro.update_position(100.0, 200.0, 50.0);
        assert_eq!(dro.machine_pos.x, 100.0);
    }

    #[test]
    fn test_coordinate_systems() {
        let systems = CoordinateSystem::all();
        assert_eq!(systems.len(), 6);
    }

    #[test]
    fn test_dro_panel() {
        let mut panel = DROPanel::new();
        panel.dro.update_position(100.0, 200.0, 50.0);
        assert_eq!(panel.dro.machine_pos.x, 100.0);
    }
}
