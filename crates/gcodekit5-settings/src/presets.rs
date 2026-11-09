//! # Machine Preset Configurations
//!
//! Predefined machine settings for common 6-axis CNC configurations.
//! Provides quick-start templates for various machine types.

use crate::Config;

/// Machine preset definition for common 6-axis configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachinePreset {
    /// Generic 3-axis mill
    Mill3Axis,
    /// Generic 4-axis mill (with A axis rotary table)
    Mill4Axis,
    /// Generic 5-axis mill (with A/B trunnion)
    Mill5Axis,
    /// Full 6-axis machine (A/B/C rotary)
    Mill6Axis,
    /// Small desktop 6-axis (e.g., PocketNC)
    Desktop6Axis,
    /// Large industrial 6-axis (e.g., DMG Mori)
    Industrial6Axis,
}

impl MachinePreset {
    /// Get human-readable name for this preset
    pub fn name(&self) -> &str {
        match self {
            Self::Mill3Axis => "3-Axis Mill",
            Self::Mill4Axis => "4-Axis Mill",
            Self::Mill5Axis => "5-Axis Mill",
            Self::Mill6Axis => "6-Axis Mill",
            Self::Desktop6Axis => "Desktop 6-Axis",
            Self::Industrial6Axis => "Industrial 6-Axis",
        }
    }

    /// Get description for this preset
    pub fn description(&self) -> &str {
        match self {
            Self::Mill3Axis => "Standard 3-axis milling machine (X/Y/Z)",
            Self::Mill4Axis => "4-axis mill with A-axis rotary table",
            Self::Mill5Axis => "5-axis mill with A/B trunnion table",
            Self::Mill6Axis => "Full 6-axis machine with A/B/C rotary",
            Self::Desktop6Axis => "Compact 6-axis desktop machine (e.g., PocketNC)",
            Self::Industrial6Axis => "Large industrial 6-axis machining center",
        }
    }

    /// Apply this preset to a config instance
    pub fn apply(&self, config: &mut Config) {
        match self {
            Self::Mill3Axis => apply_3axis_defaults(config),
            Self::Mill4Axis => apply_4axis_defaults(config),
            Self::Mill5Axis => apply_5axis_defaults(config),
            Self::Mill6Axis => apply_6axis_defaults(config),
            Self::Desktop6Axis => apply_desktop_6axis(config),
            Self::Industrial6Axis => apply_industrial_6axis(config),
        }
    }
}

/// Get all available machine presets
pub fn available_presets() -> Vec<MachinePreset> {
    vec![
        MachinePreset::Mill3Axis,
        MachinePreset::Mill4Axis,
        MachinePreset::Mill5Axis,
        MachinePreset::Mill6Axis,
        MachinePreset::Desktop6Axis,
        MachinePreset::Industrial6Axis,
    ]
}

/// Apply 3-axis mill defaults
fn apply_3axis_defaults(config: &mut Config) {
    config.machine.x_limit = 300.0;
    config.machine.y_limit = 200.0;
    config.machine.z_limit = 100.0;
    config.machine.a_limit = 0.0;
    config.machine.b_limit = 0.0;
    config.machine.c_limit = 0.0;
    config.machine.jog_increment = 1.0;
    config.machine.jog_increment_rotary = 0.0; // No rotary
}

/// Apply 4-axis mill defaults
fn apply_4axis_defaults(config: &mut Config) {
    config.machine.x_limit = 300.0;
    config.machine.y_limit = 200.0;
    config.machine.z_limit = 100.0;
    config.machine.a_limit = 360.0; // A axis enabled
    config.machine.b_limit = 0.0;
    config.machine.c_limit = 0.0;
    config.machine.jog_increment = 1.0;
    config.machine.jog_increment_rotary = 1.0;

    // Steps per degree for A axis
    config
        .machine
        .steps_per_degree
        .insert("A".to_string(), 80.0);
}

/// Apply 5-axis mill defaults
fn apply_5axis_defaults(config: &mut Config) {
    config.machine.x_limit = 300.0;
    config.machine.y_limit = 200.0;
    config.machine.z_limit = 100.0;
    config.machine.a_limit = 120.0; // Limited A for trunnion
    config.machine.b_limit = 360.0; // Full B rotation
    config.machine.c_limit = 0.0;
    config.machine.jog_increment = 1.0;
    config.machine.jog_increment_rotary = 1.0;

    // Steps per degree for A/B axes
    config
        .machine
        .steps_per_degree
        .insert("A".to_string(), 80.0);
    config
        .machine
        .steps_per_degree
        .insert("B".to_string(), 80.0);
}

/// Apply 6-axis mill defaults
fn apply_6axis_defaults(config: &mut Config) {
    config.machine.x_limit = 300.0;
    config.machine.y_limit = 200.0;
    config.machine.z_limit = 100.0;
    config.machine.a_limit = 360.0; // Full A rotation
    config.machine.b_limit = 360.0; // Full B rotation
    config.machine.c_limit = 360.0; // Full C rotation
    config.machine.jog_increment = 1.0;
    config.machine.jog_increment_rotary = 1.0;

    // Steps per degree for all rotary axes
    config
        .machine
        .steps_per_degree
        .insert("A".to_string(), 80.0);
    config
        .machine
        .steps_per_degree
        .insert("B".to_string(), 80.0);
    config
        .machine
        .steps_per_degree
        .insert("C".to_string(), 80.0);
}

/// Apply desktop 6-axis defaults (e.g., PocketNC)
fn apply_desktop_6axis(config: &mut Config) {
    config.machine.x_limit = 115.0; // 115mm X travel
    config.machine.y_limit = 100.0; // 100mm Y travel
    config.machine.z_limit = 90.0; // 90mm Z travel
    config.machine.a_limit = 9999.0; // Continuous A rotation
    config.machine.b_limit = 9999.0; // Continuous B rotation
    config.machine.c_limit = 9999.0; // Continuous C rotation
    config.machine.jog_increment = 0.5;
    config.machine.jog_increment_rotary = 0.5;
    config.machine.jog_feed_rate = 500.0; // Lower feed for desktop

    // Steps per degree (typically higher ratio for desktop)
    config
        .machine
        .steps_per_degree
        .insert("A".to_string(), 200.0);
    config
        .machine
        .steps_per_degree
        .insert("B".to_string(), 200.0);
    config
        .machine
        .steps_per_degree
        .insert("C".to_string(), 200.0);
}

/// Apply industrial 6-axis defaults
fn apply_industrial_6axis(config: &mut Config) {
    config.machine.x_limit = 800.0; // Large X travel
    config.machine.y_limit = 600.0; // Large Y travel
    config.machine.z_limit = 500.0; // Large Z travel
    config.machine.a_limit = 360.0; // Full A rotation
    config.machine.b_limit = 120.0; // Limited B for industrial
    config.machine.c_limit = 360.0; // Full C rotation
    config.machine.jog_increment = 10.0;
    config.machine.jog_increment_rotary = 5.0;
    config.machine.jog_feed_rate = 5000.0; // Higher feed for industrial

    // Steps per degree (typically lower ratio for industrial)
    config
        .machine
        .steps_per_degree
        .insert("A".to_string(), 40.0);
    config
        .machine
        .steps_per_degree
        .insert("B".to_string(), 40.0);
    config
        .machine
        .steps_per_degree
        .insert("C".to_string(), 40.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_names() {
        assert_eq!(MachinePreset::Mill3Axis.name(), "3-Axis Mill");
        assert_eq!(MachinePreset::Mill6Axis.name(), "6-Axis Mill");
    }

    #[test]
    fn test_preset_descriptions() {
        assert!(MachinePreset::Mill5Axis.description().contains("5-axis"));
    }

    #[test]
    fn test_available_presets() {
        let presets = available_presets();
        assert_eq!(presets.len(), 6);
        assert!(presets.contains(&MachinePreset::Mill6Axis));
    }

    #[test]
    fn test_apply_6axis_preset() {
        let mut config = Config::new();
        MachinePreset::Mill6Axis.apply(&mut config);

        assert_eq!(config.machine.a_limit, 360.0);
        assert_eq!(config.machine.b_limit, 360.0);
        assert_eq!(config.machine.c_limit, 360.0);
        assert!(config.machine.steps_per_degree.contains_key("A"));
        assert!(config.machine.steps_per_degree.contains_key("B"));
        assert!(config.machine.steps_per_degree.contains_key("C"));
    }

    #[test]
    fn test_apply_desktop_preset() {
        let mut config = Config::new();
        MachinePreset::Desktop6Axis.apply(&mut config);

        assert_eq!(config.machine.x_limit, 115.0);
        assert_eq!(config.machine.jog_feed_rate, 500.0);
    }
}
