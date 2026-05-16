//! Jog Controller Panel - Task 70
//!
//! Manual machine control with jog buttons and step size selection

/// Jog direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JogDirection {
    /// Move X positive
    XPos,
    /// Move X negative
    XNeg,
    /// Move Y positive
    YPos,
    /// Move Y negative
    YNeg,
    /// Move Z positive
    ZPos,
    /// Move Z negative
    ZNeg,
    /// Rotate A positive (4th axis - around X)
    APos,
    /// Rotate A negative (4th axis - around X)
    ANeg,
    /// Rotate B positive (5th axis - around Y)
    BPos,
    /// Rotate B negative (5th axis - around Y)
    BNeg,
    /// Rotate C positive (6th axis - around Z)
    CPos,
    /// Rotate C negative (6th axis - around Z)
    CNeg,
}

impl JogDirection {
    /// Get axis character
    pub fn axis(&self) -> char {
        match self {
            Self::XPos | Self::XNeg => 'X',
            Self::YPos | Self::YNeg => 'Y',
            Self::ZPos | Self::ZNeg => 'Z',
            Self::APos | Self::ANeg => 'A',
            Self::BPos | Self::BNeg => 'B',
            Self::CPos | Self::CNeg => 'C',
        }
    }

    /// Get direction multiplier
    pub fn multiplier(&self) -> f64 {
        match self {
            Self::XNeg | Self::YNeg | Self::ZNeg | Self::ANeg | Self::BNeg | Self::CNeg => -1.0,
            Self::XPos | Self::YPos | Self::ZPos | Self::APos | Self::BPos | Self::CPos => 1.0,
        }
    }

    /// Get direction name
    pub fn name(&self) -> &str {
        match self {
            Self::XPos => "X+",
            Self::XNeg => "X-",
            Self::YPos => "Y+",
            Self::YNeg => "Y-",
            Self::ZPos => "Z+",
            Self::ZNeg => "Z-",
            Self::APos => "A+",
            Self::ANeg => "A-",
            Self::BPos => "B+",
            Self::BNeg => "B-",
            Self::CPos => "C+",
            Self::CNeg => "C-",
        }
    }

    /// Check if this is a rotary axis direction
    pub fn is_rotary(&self) -> bool {
        matches!(self, Self::APos | Self::ANeg | Self::BPos | Self::BNeg | Self::CPos | Self::CNeg)
    }
}

/// Jog step size for linear axes (mm/inches)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum JogStepSize {
    /// 0.001 mm/in
    Micro = 0,
    /// 0.01 mm/in
    Tiny = 1,
    /// 0.1 mm/in
    Small = 2,
    /// 1.0 mm/in
    Medium = 3,
    /// 10.0 mm/in
    Large = 4,
    /// 100.0 mm/in
    Huge = 5,
}

impl JogStepSize {
    /// Get step size value in mm/inches
    pub fn value(&self) -> f64 {
        match self {
            Self::Micro => 0.001,
            Self::Tiny => 0.01,
            Self::Small => 0.1,
            Self::Medium => 1.0,
            Self::Large => 10.0,
            Self::Huge => 100.0,
        }
    }

    /// Get all step sizes
    pub fn all() -> Vec<Self> {
        vec![
            Self::Micro,
            Self::Tiny,
            Self::Small,
            Self::Medium,
            Self::Large,
            Self::Huge,
        ]
    }

    /// Get description
    pub fn description(&self) -> &str {
        match self {
            Self::Micro => "0.001",
            Self::Tiny => "0.01",
            Self::Small => "0.1",
            Self::Medium => "1.0",
            Self::Large => "10.0",
            Self::Huge => "100.0",
        }
    }

    /// Get unit label
    pub fn unit_label(&self) -> &str {
        "mm"
    }
}

/// Rotary jog step size (degrees)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum RotaryJogStepSize {
    /// 0.1 degrees - fine adjustment
    Fine = 0,
    /// 0.5 degrees - small moves
    Small = 1,
    /// 1.0 degrees - standard
    Medium = 2,
    /// 5.0 degrees - moderate
    Large = 3,
    /// 10.0 degrees - coarse
    Coarse = 4,
    /// 45.0 degrees - large moves
    XLarge = 5,
    /// 90.0 degrees - quadrant moves
    Huge = 6,
}

impl RotaryJogStepSize {
    /// Get step size value in degrees
    pub fn value(&self) -> f64 {
        match self {
            Self::Fine => 0.1,
            Self::Small => 0.5,
            Self::Medium => 1.0,
            Self::Large => 5.0,
            Self::Coarse => 10.0,
            Self::XLarge => 45.0,
            Self::Huge => 90.0,
        }
    }

    /// Get all rotary step sizes
    pub fn all() -> Vec<Self> {
        vec![
            Self::Fine,
            Self::Small,
            Self::Medium,
            Self::Large,
            Self::Coarse,
            Self::XLarge,
            Self::Huge,
        ]
    }

    /// Get description
    pub fn description(&self) -> &str {
        match self {
            Self::Fine => "0.1°",
            Self::Small => "0.5°",
            Self::Medium => "1.0°",
            Self::Large => "5.0°",
            Self::Coarse => "10.0°",
            Self::XLarge => "45.0°",
            Self::Huge => "90.0°",
        }
    }

    /// Get unit label
    pub fn unit_label(&self) -> &str {
        "°"
    }
}

impl std::fmt::Display for JogStepSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Jog button definition
#[derive(Debug, Clone)]
pub struct JogButton {
    /// Button direction
    pub direction: JogDirection,
    /// Button label
    pub label: String,
    /// Keyboard shortcut
    pub shortcut: Option<char>,
    /// Is pressed/held
    pub pressed: bool,
}

impl JogButton {
    /// Create new jog button
    pub fn new(direction: JogDirection) -> Self {
        Self {
            direction,
            label: direction.name().to_string(),
            shortcut: None,
            pressed: false,
        }
    }

    /// Set keyboard shortcut
    pub fn with_shortcut(mut self, key: char) -> Self {
        self.shortcut = Some(key);
        self
    }
}

/// Keyboard shortcut map
#[derive(Debug, Clone)]
pub struct ShortcutMap {
    /// Mapping of character to jog direction
    pub shortcuts: std::collections::HashMap<char, JogDirection>,
}

impl ShortcutMap {
    /// Create default keyboard shortcuts
    pub fn default_shortcuts() -> Self {
        let mut map = std::collections::HashMap::new();
        // Linear axis shortcuts (numpad-style)
        map.insert('8', JogDirection::YPos); // Up
        map.insert('2', JogDirection::YNeg); // Down
        map.insert('4', JogDirection::XNeg); // Left
        map.insert('6', JogDirection::XPos); // Right
        map.insert('9', JogDirection::ZPos); // Page Up
        map.insert('3', JogDirection::ZNeg); // Page Down
        // Rotary axis shortcuts (letter keys)
        map.insert('q', JogDirection::ANeg); // Q = A-
        map.insert('w', JogDirection::APos); // W = A+
        map.insert('a', JogDirection::BNeg); // A = B-
        map.insert('s', JogDirection::BPos); // S = B+
        map.insert('z', JogDirection::CNeg); // Z = C-
        map.insert('x', JogDirection::CPos); // X = C+

        Self { shortcuts: map }
    }

    /// Create new shortcut map
    pub fn new() -> Self {
        Self::default_shortcuts()
    }

    /// Add shortcut
    pub fn add(&mut self, key: char, direction: JogDirection) {
        self.shortcuts.insert(key, direction);
    }

    /// Get direction for key
    pub fn get(&self, key: char) -> Option<JogDirection> {
        self.shortcuts.get(&key).copied()
    }

    /// Get all rotary axis shortcuts
    pub fn rotary_shortcuts(&self) -> Vec<(char, JogDirection)> {
        self.shortcuts
            .iter()
            .filter(|(_, d)| d.is_rotary())
            .map(|(&k, &v)| (k, v))
            .collect()
    }
}

impl Default for ShortcutMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Jog controller panel
#[derive(Debug)]
pub struct JogControllerPanel {
    /// Current step size
    pub step_size: JogStepSize,
    /// Jog buttons
    pub buttons: Vec<JogButton>,
    /// Keyboard shortcuts
    pub shortcuts: ShortcutMap,
    /// Feed rate for jogging (units/min)
    pub jog_feed_rate: f64,
    /// Continuous jog enabled
    pub continuous_jog: bool,
    /// Pending jog commands
    pub pending_jogs: Vec<(JogDirection, f64)>,
}

impl JogControllerPanel {
    /// Create new jog controller panel
    pub fn new() -> Self {
        Self {
            step_size: JogStepSize::Medium,
            buttons: Self::create_buttons(),
            shortcuts: ShortcutMap::new(),
            jog_feed_rate: 500.0,
            continuous_jog: false,
            pending_jogs: Vec::new(),
        }
    }

    /// Create all jog buttons
    fn create_buttons() -> Vec<JogButton> {
        vec![
            JogButton::new(JogDirection::XPos),
            JogButton::new(JogDirection::XNeg),
            JogButton::new(JogDirection::YPos),
            JogButton::new(JogDirection::YNeg),
            JogButton::new(JogDirection::ZPos),
            JogButton::new(JogDirection::ZNeg),
        ]
    }

    /// Set step size
    pub fn set_step_size(&mut self, size: JogStepSize) {
        self.step_size = size;
    }

    /// Set feed rate
    pub fn set_feed_rate(&mut self, rate: f64) {
        self.jog_feed_rate = rate.max(1.0);
    }

    /// Handle jog button press
    pub fn button_press(&mut self, direction: JogDirection) {
        let increment = self.step_size.value();
        self.pending_jogs.push((direction, increment));

        if let Some(button) = self.buttons.iter_mut().find(|b| b.direction == direction) {
            button.pressed = true;
        }
    }

    /// Handle jog button release
    pub fn button_release(&mut self, direction: JogDirection) {
        if let Some(button) = self.buttons.iter_mut().find(|b| b.direction == direction) {
            button.pressed = false;
        }
    }

    /// Handle keyboard input
    pub fn keyboard_input(&mut self, key: char) -> bool {
        if let Some(direction) = self.shortcuts.get(key) {
            self.button_press(direction);
            true
        } else {
            false
        }
    }

    /// Toggle continuous jog
    pub fn toggle_continuous_jog(&mut self) {
        self.continuous_jog = !self.continuous_jog;
    }

    /// Get next pending jog command
    pub fn next_jog_command(&mut self) -> Option<(char, f64, f64)> {
        self.pending_jogs.pop().map(|(direction, increment)| {
            (
                direction.axis(),
                increment * direction.multiplier(),
                self.jog_feed_rate,
            )
        })
    }

    /// Clear pending jogs
    pub fn clear_pending_jogs(&mut self) {
        self.pending_jogs.clear();
    }

    /// Get active buttons (currently pressed)
    pub fn active_buttons(&self) -> Vec<&JogButton> {
        self.buttons.iter().filter(|b| b.pressed).collect()
    }
}

impl Default for JogControllerPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jog_direction() {
        assert_eq!(JogDirection::XPos.axis(), 'X');
        assert_eq!(JogDirection::YNeg.axis(), 'Y');
        assert_eq!(JogDirection::ZPos.multiplier(), 1.0);
        assert_eq!(JogDirection::XNeg.multiplier(), -1.0);
    }

    #[test]
    fn test_step_sizes() {
        let sizes = JogStepSize::all();
        assert_eq!(sizes.len(), 6);
        assert_eq!(sizes[3].value(), 1.0);
    }

    #[test]
    fn test_keyboard_shortcuts() {
        let shortcuts = ShortcutMap::new();
        assert_eq!(shortcuts.get('8'), Some(JogDirection::YPos));
        assert_eq!(shortcuts.get('x'), None);
    }

    #[test]
    fn test_jog_button() {
        let btn = JogButton::new(JogDirection::XPos);
        assert_eq!(btn.label, "X+");
        assert!(!btn.pressed);
    }

    #[test]
    fn test_jog_controller() {
        let mut jog = JogControllerPanel::new();
        assert_eq!(jog.step_size, JogStepSize::Medium);

        jog.button_press(JogDirection::XPos);
        assert_eq!(jog.pending_jogs.len(), 1);
    }

    #[test]
    fn test_jog_next_command() {
        let mut jog = JogControllerPanel::new();
        jog.button_press(JogDirection::YNeg);
        let cmd = jog.next_jog_command();
        assert!(cmd.is_some());
        let (axis, increment, _) = cmd.expect("no command");
        assert_eq!(axis, 'Y');
        assert!(increment < 0.0);
    }

    #[test]
    fn test_continuous_jog() {
        let mut jog = JogControllerPanel::new();
        assert!(!jog.continuous_jog);
        jog.toggle_continuous_jog();
        assert!(jog.continuous_jog);
    }

    #[test]
    fn test_active_buttons() {
        let mut jog = JogControllerPanel::new();
        jog.button_press(JogDirection::XPos);
        jog.button_press(JogDirection::YPos);
        assert_eq!(jog.active_buttons().len(), 2);
    }
}
