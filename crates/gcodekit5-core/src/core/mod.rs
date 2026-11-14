//! Core controller management and state machine
//!
//! Controller override state and console messages.

pub mod message;

/// Override state for controller operations.
///
/// Tracks real-time override percentages for feed rate, rapids, and spindle
/// speed. These values are applied on top of programmed values by the
/// controller firmware.
///
/// # Example
/// ```
/// use gcodekit5_core::core::OverrideState;
///
/// let overrides = OverrideState::default();
/// assert_eq!(overrides.feed_override, 100);
/// assert_eq!(overrides.rapid_override, 100);
/// assert_eq!(overrides.spindle_override, 100);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverrideState {
    /// Feed rate override percentage (0-200%)
    pub feed_override: u16,
    /// Rapid override percentage (25%, 50%, 100%)
    pub rapid_override: u8,
    /// Spindle override percentage (0-200%)
    pub spindle_override: u16,
}

impl Default for OverrideState {
    fn default() -> Self {
        Self {
            feed_override: 100,
            rapid_override: 100,
            spindle_override: 100,
        }
    }
}
