//! Coordinate System Panel - Task 76
//!
//! Work coordinate system selection and offsets display

use std::collections::HashMap;

/// Coordinate system identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CoordinateSystemId {
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

impl CoordinateSystemId {
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

    /// Get G-code command
    pub fn gcode(&self) -> &str {
        match self {
            Self::G54 => "G54",
            Self::G55 => "G55",
            Self::G56 => "G56",
            Self::G57 => "G57",
            Self::G58 => "G58",
            Self::G59 => "G59",
        }
    }

    /// Get system number (1-6)
    pub fn number(&self) -> u8 {
        match self {
            Self::G54 => 1,
            Self::G55 => 2,
            Self::G56 => 3,
            Self::G57 => 4,
            Self::G58 => 5,
            Self::G59 => 6,
        }
    }
}

impl std::fmt::Display for CoordinateSystemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.gcode())
    }
}

/// Coordinate offset
#[derive(Debug, Clone, Copy)]
pub struct CoordinateOffset {
    /// X offset
    pub x: f32,
    /// Y offset
    pub y: f32,
    /// Z offset
    pub z: f32,
    /// A axis (4th axis) offset
    pub a: f32,
    /// B axis (5th axis) offset
    pub b: f32,
    /// C axis (6th axis) offset
    pub c: f32,
}

impl CoordinateOffset {
    /// Create new coordinate offset for 3 axes
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, a: 0.0, b: 0.0, c: 0.0 }
    }

    /// Create new coordinate offset for 6 axes
    pub fn new_6axis(x: f32, y: f32, z: f32, a: f32, b: f32, c: f32) -> Self {
        Self { x, y, z, a, b, c }
    }

    /// Get offset as tuple (x, y, z)
    pub fn as_tuple(&self) -> (f32, f32, f32) {
        (self.x, self.y, self.z)
    }

    /// Get full offset as 6-axis tuple
    pub fn as_tuple_6axis(&self) -> (f32, f32, f32, f32, f32, f32) {
        (self.x, self.y, self.z, self.a, self.b, self.c)
    }

    /// Get formatted string for 3 axes
    pub fn formatted(&self, units: &str) -> String {
        format!("X:{:.2} Y:{:.2} Z:{:.2} {}", self.x, self.y, self.z, units)
    }

    /// Get formatted string for 6 axes
    pub fn formatted_6axis(&self, units: &str) -> String {
        format!(
            "X:{:.2} Y:{:.2} Z:{:.2} A:{:.2}° B:{:.2}° C:{:.2}° {}",
            self.x, self.y, self.z, self.a, self.b, self.c, units
        )
    }

    /// Set offset value by axis
    pub fn set(&mut self, axis: char, value: f32) {
        match axis {
            'X' | 'x' => self.x = value,
            'Y' | 'y' => self.y = value,
            'Z' | 'z' => self.z = value,
            'A' | 'a' => self.a = value,
            'B' | 'b' => self.b = value,
            'C' | 'c' => self.c = value,
            _ => {}
        }
    }

    /// Get offset value by axis
    pub fn get(&self, axis: char) -> Option<f32> {
        match axis {
            'X' | 'x' => Some(self.x),
            'Y' | 'y' => Some(self.y),
            'Z' | 'z' => Some(self.z),
            'A' | 'a' => Some(self.a),
            'B' | 'b' => Some(self.b),
            'C' | 'c' => Some(self.c),
            _ => None,
        }
    }
}

impl Default for CoordinateOffset {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

/// Work coordinate system definition
#[derive(Debug, Clone)]
pub struct WorkCoordinateSystem {
    /// System identifier
    pub id: CoordinateSystemId,
    /// Coordinate offsets
    pub offset: CoordinateOffset,
    /// Description
    pub description: String,
}

impl WorkCoordinateSystem {
    /// Create new WCS
    pub fn new(id: CoordinateSystemId) -> Self {
        Self {
            id,
            offset: CoordinateOffset::default(),
            description: format!("Work Coordinate System {}", id.number()),
        }
    }

    /// Set offset for 3 axes (X, Y, Z)
    pub fn set_offset(&mut self, x: f32, y: f32, z: f32) {
        self.offset.x = x;
        self.offset.y = y;
        self.offset.z = z;
    }

    /// Set full 6-axis offset
    pub fn set_offset_6axis(&mut self, x: f32, y: f32, z: f32, a: f32, b: f32, c: f32) {
        self.offset = CoordinateOffset::new_6axis(x, y, z, a, b, c);
    }

    /// Get current position with offset (3 axes)
    pub fn apply_offset(&self, x: f32, y: f32, z: f32) -> (f32, f32, f32) {
        (x + self.offset.x, y + self.offset.y, z + self.offset.z)
    }

    /// Get current position with full 6-axis offset
    pub fn apply_offset_6axis(&self, x: f32, y: f32, z: f32, a: f32, b: f32, c: f32) -> (f32, f32, f32, f32, f32, f32) {
        (
            x + self.offset.x,
            y + self.offset.y,
            z + self.offset.z,
            a + self.offset.a,
            b + self.offset.b,
            c + self.offset.c,
        )
    }

    /// Remove offset from position (3 axes)
    pub fn remove_offset(&self, x: f32, y: f32, z: f32) -> (f32, f32, f32) {
        (x - self.offset.x, y - self.offset.y, z - self.offset.z)
    }

    /// Remove offset from position (6 axes)
    pub fn remove_offset_6axis(&self, x: f32, y: f32, z: f32, a: f32, b: f32, c: f32) -> (f32, f32, f32, f32, f32, f32) {
        (
            x - self.offset.x,
            y - self.offset.y,
            z - self.offset.z,
            a - self.offset.a,
            b - self.offset.b,
            c - self.offset.c,
        )
    }
}

/// Zero operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroType {
    /// Zero X axis only
    X,
    /// Zero Y axis only
    Y,
    /// Zero Z axis only
    Z,
    /// Zero A axis only (4th axis)
    A,
    /// Zero B axis only (5th axis)
    B,
    /// Zero C axis only (6th axis)
    C,
    /// Zero all axes
    All,
}

impl std::fmt::Display for ZeroType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X => write!(f, "Zero X"),
            Self::Y => write!(f, "Zero Y"),
            Self::Z => write!(f, "Zero Z"),
            Self::A => write!(f, "Zero A"),
            Self::B => write!(f, "Zero B"),
            Self::C => write!(f, "Zero C"),
            Self::All => write!(f, "Zero All"),
        }
    }
}

impl ZeroType {
    /// Get G-code command for setting this axis to zero (G92)
    pub fn to_g92_command(&self) -> &'static str {
        match self {
            Self::X => "G92 X0",
            Self::Y => "G92 Y0",
            Self::Z => "G92 Z0",
            Self::A => "G92 A0",
            Self::B => "G92 B0",
            Self::C => "G92 C0",
            Self::All => "G92 X0 Y0 Z0 A0 B0 C0",
        }
    }

    /// Get G-code command for setting this axis to zero (G10)
    pub fn to_g10_command(&self) -> &'static str {
        match self {
            Self::X => "G10 L20 P0 X0",
            Self::Y => "G10 L20 P0 Y0",
            Self::Z => "G10 L20 P0 Z0",
            Self::A => "G10 L20 P0 A0",
            Self::B => "G10 L20 P0 B0",
            Self::C => "G10 L20 P0 C0",
            Self::All => "G10 L20 P0 X0 Y0 Z0 A0 B0 C0",
        }
    }

    /// Get axis character if single axis
    pub fn axis_char(&self) -> Option<char> {
        match self {
            Self::X => Some('X'),
            Self::Y => Some('Y'),
            Self::Z => Some('Z'),
            Self::A => Some('A'),
            Self::B => Some('B'),
            Self::C => Some('C'),
            Self::All => None,
        }
    }
}

/// Coordinate system panel
#[derive(Debug)]
pub struct CoordinateSystemPanel {
    /// All work coordinate systems
    pub systems: HashMap<CoordinateSystemId, WorkCoordinateSystem>,
    /// Active/current WCS
    pub active_system: CoordinateSystemId,
    /// Current machine position (x, y, z)
    pub current_position: (f32, f32, f32),
    /// Current machine position (a, b, c) for rotary axes
    pub current_rotary_position: (f32, f32, f32),
    /// Unit system (mm or in)
    pub units: String,
    /// Axis count (3, 4, 5, or 6)
    pub axis_count: u8,
}

impl CoordinateSystemPanel {
    /// Create new coordinate system panel
    pub fn new() -> Self {
        let mut systems = HashMap::new();

        for id in CoordinateSystemId::all() {
            systems.insert(id, WorkCoordinateSystem::new(id));
        }

        Self {
            systems,
            active_system: CoordinateSystemId::G54,
            current_position: (0.0, 0.0, 0.0),
            current_rotary_position: (0.0, 0.0, 0.0),
            units: "mm".to_string(),
            axis_count: 3,
        }
    }

    /// Set axis count and enable/disable rotary axis support
    pub fn set_axis_count(&mut self, count: u8) {
        self.axis_count = count.min(6).max(3);
    }

    /// Check if rotary axes are available
    pub fn has_rotary_axes(&self) -> bool {
        self.axis_count >= 4
    }

    /// Select coordinate system
    pub fn select_system(&mut self, id: CoordinateSystemId) -> Option<String> {
        if self.systems.contains_key(&id) {
            self.active_system = id;
            Some(id.gcode().to_string())
        } else {
            None
        }
    }

    /// Get active system
    pub fn get_active_system(&self) -> Option<&WorkCoordinateSystem> {
        self.systems.get(&self.active_system)
    }

    /// Get mutable active system
    pub fn get_active_system_mut(&mut self) -> Option<&mut WorkCoordinateSystem> {
        self.systems.get_mut(&self.active_system)
    }

    /// Set offset in active system (3 axes)
    pub fn set_active_offset(&mut self, x: f32, y: f32, z: f32) {
        if let Some(system) = self.get_active_system_mut() {
            system.set_offset(x, y, z);
        }
    }

    /// Set full 6-axis offset in active system
    pub fn set_active_offset_6axis(&mut self, x: f32, y: f32, z: f32, a: f32, b: f32, c: f32) {
        if let Some(system) = self.get_active_system_mut() {
            system.set_offset_6axis(x, y, z, a, b, c);
        }
    }

    /// Zero axis in active system
    pub fn zero_axis(&mut self, axis: char) -> bool {
        if let Some(system) = self.get_active_system_mut() {
            system.offset.set(axis, 0.0);
            true
        } else {
            false
        }
    }

    /// Zero all axes in active system
    pub fn zero_all_axes(&mut self) -> bool {
        if let Some(system) = self.get_active_system_mut() {
            system.set_offset_6axis(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            true
        } else {
            false
        }
    }

    /// Zero specific axes by ZeroType
    pub fn zero_by_type(&mut self, zero_type: ZeroType) -> bool {
        match zero_type {
            ZeroType::All => self.zero_all_axes(),
            _ => {
                if let Some(axis) = zero_type.axis_char() {
                    self.zero_axis(axis)
                } else {
                    false
                }
            }
        }
    }

    /// Get G-code command for zero operation
    pub fn get_zero_gcode(&self, zero_type: ZeroType) -> Option<&'static str> {
        match zero_type {
            ZeroType::All => Some("G92 X0 Y0 Z0 A0 B0 C0"),
            ZeroType::X => Some("G92 X0"),
            ZeroType::Y => Some("G92 Y0"),
            ZeroType::Z => Some("G92 Z0"),
            ZeroType::A if self.axis_count >= 4 => Some("G92 A0"),
            ZeroType::B if self.axis_count >= 5 => Some("G92 B0"),
            ZeroType::C if self.axis_count >= 6 => Some("G92 C0"),
            _ => None,
        }
    }

    /// Set work position (sets offset based on current position)
    pub fn set_work_position(&mut self, x: f32, y: f32, z: f32) -> bool {
        let (cx, cy, cz) = self.current_position;
        if let Some(system) = self.get_active_system_mut() {
            system.set_offset(x - cx, y - cy, z - cz);
            true
        } else {
            false
        }
    }

    /// Set 6-axis work position
    pub fn set_work_position_6axis(
        &mut self,
        x: f32, y: f32, z: f32,
        a: f32, b: f32, c: f32
    ) -> bool {
        let (cx, cy, cz) = self.current_position;
        let (ca, cb, cc) = self.current_rotary_position;
        if let Some(system) = self.get_active_system_mut() {
            system.set_offset_6axis(
                x - cx, y - cy, z - cz,
                a - ca, b - cb, c - cc
            );
            true
        } else {
            false
        }
    }

    /// Go to zero (return to work coordinate zero)
    pub fn go_to_zero(&self) -> (f32, f32, f32) {
        if let Some(system) = self.get_active_system() {
            let (x, y, z) = system.offset.as_tuple();
            (-x, -y, -z)
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    /// Go to zero (6-axis)
    pub fn go_to_zero_6axis(&self) -> (f32, f32, f32, f32, f32, f32) {
        if let Some(system) = self.get_active_system() {
            let (x, y, z, a, b, c) = system.offset.as_tuple_6axis();
            (-x, -y, -z, -a, -b, -c)
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        }
    }

    /// Update current machine position (3 axes)
    pub fn update_position(&mut self, x: f32, y: f32, z: f32) {
        self.current_position = (x, y, z);
    }

    /// Update current machine position (6 axes)
    pub fn update_position_6axis(
        &mut self,
        x: f32, y: f32, z: f32,
        a: f32, b: f32, c: f32
    ) {
        self.current_position = (x, y, z);
        self.current_rotary_position = (a, b, c);
    }

    /// Get work position from machine position (3 axes)
    pub fn get_work_position(&self) -> (f32, f32, f32) {
        if let Some(system) = self.get_active_system() {
            system.remove_offset(
                self.current_position.0,
                self.current_position.1,
                self.current_position.2,
            )
        } else {
            self.current_position
        }
    }

    /// Get work position from machine position (6 axes)
    pub fn get_work_position_6axis(&self) -> (f32, f32, f32, f32, f32, f32) {
        if let Some(system) = self.get_active_system() {
            system.remove_offset_6axis(
                self.current_position.0,
                self.current_position.1,
                self.current_position.2,
                self.current_rotary_position.0,
                self.current_rotary_position.1,
                self.current_rotary_position.2,
            )
        } else {
            (
                self.current_position.0,
                self.current_position.1,
                self.current_position.2,
                self.current_rotary_position.0,
                self.current_rotary_position.1,
                self.current_rotary_position.2,
            )
        }
    }

    /// Get all offsets
    pub fn get_all_offsets(&self) -> HashMap<CoordinateSystemId, (f32, f32, f32)> {
        self.systems
            .iter()
            .map(|(id, sys)| (*id, sys.offset.as_tuple()))
            .collect()
    }

    /// Get offset for specific system
    pub fn get_offset(&self, id: CoordinateSystemId) -> Option<(f32, f32, f32)> {
        self.systems.get(&id).map(|s| s.offset.as_tuple())
    }

    /// Get system description
    pub fn get_description(&self, id: CoordinateSystemId) -> Option<String> {
        self.systems.get(&id).map(|s| s.description.clone())
    }

    /// Set system description
    pub fn set_description(
        &mut self,
        id: CoordinateSystemId,
        description: impl Into<String>,
    ) -> bool {
        if let Some(system) = self.systems.get_mut(&id) {
            system.description = description.into();
            true
        } else {
            false
        }
    }

    /// Get offset summary for active system
    pub fn active_offset_summary(&self) -> String {
        if let Some(system) = self.get_active_system() {
            format!("{}: {}", system.id, system.offset.formatted(&self.units))
        } else {
            "No active system".to_string()
        }
    }

    /// Get all systems list
    pub fn get_systems_list(&self) -> Vec<(CoordinateSystemId, String)> {
        CoordinateSystemId::all()
            .iter()
            .map(|id| {
                (
                    *id,
                    self.systems
                        .get(id)
                        .map(|s| s.description.clone())
                        .unwrap_or_else(|| format!("WCS {}", id.number())),
                )
            })
            .collect()
    }
}

impl Default for CoordinateSystemPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_system_id_all() {
        let all = CoordinateSystemId::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_coordinate_system_id_gcode() {
        assert_eq!(CoordinateSystemId::G54.gcode(), "G54");
        assert_eq!(CoordinateSystemId::G59.gcode(), "G59");
    }

    #[test]
    fn test_coordinate_system_id_number() {
        assert_eq!(CoordinateSystemId::G54.number(), 1);
        assert_eq!(CoordinateSystemId::G59.number(), 6);
    }

    #[test]
    fn test_coordinate_offset() {
        let offset = CoordinateOffset::new(10.5, 20.0, -5.5);
        assert_eq!(offset.x, 10.5);
        let (x, y, z) = offset.as_tuple();
        assert_eq!(x, 10.5);
    }

    #[test]
    fn test_coordinate_offset_set() {
        let mut offset = CoordinateOffset::default();
        offset.set('X', 5.0);
        assert_eq!(offset.x, 5.0);
    }

    #[test]
    fn test_coordinate_offset_get() {
        let offset = CoordinateOffset::new(5.0, 10.0, 15.0);
        assert_eq!(offset.get('X'), Some(5.0));
        assert_eq!(offset.get('Y'), Some(10.0));
    }

    #[test]
    fn test_wcs_creation() {
        let wcs = WorkCoordinateSystem::new(CoordinateSystemId::G54);
        assert_eq!(wcs.id, CoordinateSystemId::G54);
        assert_eq!(wcs.offset.x, 0.0);
    }

    #[test]
    fn test_wcs_apply_offset() {
        let mut wcs = WorkCoordinateSystem::new(CoordinateSystemId::G54);
        wcs.set_offset(10.0, 20.0, 5.0);
        let (x, y, z) = wcs.apply_offset(0.0, 0.0, 0.0);
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
        assert_eq!(z, 5.0);
    }

    #[test]
    fn test_wcs_remove_offset() {
        let mut wcs = WorkCoordinateSystem::new(CoordinateSystemId::G54);
        wcs.set_offset(10.0, 20.0, 5.0);
        let (x, y, z) = wcs.remove_offset(15.0, 25.0, 10.0);
        assert_eq!(x, 5.0);
        assert_eq!(y, 5.0);
        assert_eq!(z, 5.0);
    }

    #[test]
    fn test_panel_creation() {
        let panel = CoordinateSystemPanel::new();
        assert_eq!(panel.systems.len(), 6);
        assert_eq!(panel.active_system, CoordinateSystemId::G54);
    }

    #[test]
    fn test_panel_select_system() {
        let mut panel = CoordinateSystemPanel::new();
        let result = panel.select_system(CoordinateSystemId::G55);
        assert!(result.is_some());
        assert_eq!(panel.active_system, CoordinateSystemId::G55);
    }

    #[test]
    fn test_panel_zero_axis() {
        let mut panel = CoordinateSystemPanel::new();
        panel.set_active_offset(10.0, 20.0, 5.0);
        panel.zero_axis('X');
        let offset = panel.get_active_system().expect("no active system").offset;
        assert_eq!(offset.x, 0.0);
    }

    #[test]
    fn test_panel_zero_all() {
        let mut panel = CoordinateSystemPanel::new();
        panel.set_active_offset(10.0, 20.0, 5.0);
        panel.zero_all_axes();
        let offset = panel.get_active_system().expect("no active system").offset;
        assert_eq!(offset.x, 0.0);
        assert_eq!(offset.y, 0.0);
        assert_eq!(offset.z, 0.0);
    }

    #[test]
    fn test_panel_set_work_position() {
        let mut panel = CoordinateSystemPanel::new();
        panel.update_position(100.0, 200.0, 50.0);
        panel.set_work_position(10.0, 20.0, 5.0);
        let offset = panel.get_active_system().expect("no active system").offset;
        assert_eq!(offset.x, -90.0);
        assert_eq!(offset.y, -180.0);
        assert_eq!(offset.z, -45.0);
    }

    #[test]
    fn test_panel_go_to_zero() {
        let mut panel = CoordinateSystemPanel::new();
        panel.set_active_offset(10.0, 20.0, 5.0);
        let (x, y, z) = panel.go_to_zero();
        assert_eq!(x, -10.0);
        assert_eq!(y, -20.0);
        assert_eq!(z, -5.0);
    }

    #[test]
    fn test_panel_work_position() {
        let mut panel = CoordinateSystemPanel::new();
        panel.set_active_offset(10.0, 20.0, 5.0);
        panel.update_position(50.0, 60.0, 30.0);
        let (x, y, z) = panel.get_work_position();
        assert_eq!(x, 40.0);
        assert_eq!(y, 40.0);
        assert_eq!(z, 25.0);
    }

    #[test]
    fn test_panel_get_all_offsets() {
        let panel = CoordinateSystemPanel::new();
        let offsets = panel.get_all_offsets();
        assert_eq!(offsets.len(), 6);
    }

    #[test]
    fn test_panel_offset_summary() {
        let mut panel = CoordinateSystemPanel::new();
        panel.set_active_offset(5.0, 10.0, 15.0);
        let summary = panel.active_offset_summary();
        assert!(summary.contains("G54"));
    }

    #[test]
    fn test_panel_systems_list() {
        let panel = CoordinateSystemPanel::new();
        let list = panel.get_systems_list();
        assert_eq!(list.len(), 6);
    }

    #[test]
    fn test_panel_description() {
        let mut panel = CoordinateSystemPanel::new();
        panel.set_description(CoordinateSystemId::G54, "Main Setup");
        let desc = panel.get_description(CoordinateSystemId::G54);
        assert_eq!(desc, Some("Main Setup".to_string()));
    }
}
