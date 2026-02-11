//! Override manager framework
//!
//! Provides traits and implementations for managing feed rate, rapid, and spindle overrides.

/// Override state
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverrideState {
    /// Feed rate override percentage (0-200%)
    pub feed_rate_override: f64,
    /// Rapid override level (0=0%, 1=25%, 2=50%, 3=100%)
    pub rapid_override: RapidOverrideLevel,
    /// Spindle override percentage (0-200%)
    pub spindle_override: f64,
}

/// Rapid override levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RapidOverrideLevel {
    /// Rapid movements are disabled (0%)
    Off = 0,
    /// Slow rapid (25%)
    Slow = 1,
    /// Medium rapid (50%)
    Medium = 2,
    /// Full rapid (100%)
    Full = 3,
}

impl Default for OverrideState {
    fn default() -> Self {
        Self {
            feed_rate_override: 100.0,
            rapid_override: RapidOverrideLevel::Full,
            spindle_override: 100.0,
        }
    }
}

/// Trait for override management
pub trait OverrideManagerTrait: Send + Sync {
    /// Set feed rate override
    fn set_feed_rate_override(&mut self, percentage: f64) -> anyhow::Result<()>;

    /// Get current feed rate override
    fn get_feed_rate_override(&self) -> f64;

    /// Set rapid override level
    fn set_rapid_override(&mut self, level: RapidOverrideLevel) -> anyhow::Result<()>;

    /// Get current rapid override level
    fn get_rapid_override(&self) -> RapidOverrideLevel;

    /// Set spindle override
    fn set_spindle_override(&mut self, percentage: f64) -> anyhow::Result<()>;

    /// Get current spindle override
    fn get_spindle_override(&self) -> f64;

    /// Get complete override state
    fn get_state(&self) -> OverrideState;

    /// Increase feed rate override by increment
    fn increase_feed_rate(&mut self, increment: f64) -> anyhow::Result<()> {
        let new_value = (self.get_feed_rate_override() + increment).clamp(0.0, 200.0);
        self.set_feed_rate_override(new_value)
    }

    /// Decrease feed rate override by decrement
    fn decrease_feed_rate(&mut self, decrement: f64) -> anyhow::Result<()> {
        let new_value = (self.get_feed_rate_override() - decrement).clamp(0.0, 200.0);
        self.set_feed_rate_override(new_value)
    }

    /// Increase spindle override by increment
    fn increase_spindle(&mut self, increment: f64) -> anyhow::Result<()> {
        let new_value = (self.get_spindle_override() + increment).clamp(0.0, 200.0);
        self.set_spindle_override(new_value)
    }

    /// Decrease spindle override by decrement
    fn decrease_spindle(&mut self, decrement: f64) -> anyhow::Result<()> {
        let new_value = (self.get_spindle_override() - decrement).clamp(0.0, 200.0);
        self.set_spindle_override(new_value)
    }
}

/// Default implementation of override manager
#[derive(Debug, Clone)]
pub struct DefaultOverrideManager {
    state: OverrideState,
}

impl DefaultOverrideManager {
    /// Create a new override manager
    pub fn new() -> Self {
        Self {
            state: OverrideState::default(),
        }
    }
}

impl Default for DefaultOverrideManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OverrideManagerTrait for DefaultOverrideManager {
    fn set_feed_rate_override(&mut self, percentage: f64) -> anyhow::Result<()> {
        if !(0.0..=200.0).contains(&percentage) {
            return Err(anyhow::anyhow!(
                "Feed rate override must be between 0 and 200%, got {}",
                percentage
            ));
        }
        self.state.feed_rate_override = percentage;
        Ok(())
    }

    fn get_feed_rate_override(&self) -> f64 {
        self.state.feed_rate_override
    }

    fn set_rapid_override(&mut self, level: RapidOverrideLevel) -> anyhow::Result<()> {
        self.state.rapid_override = level;
        Ok(())
    }

    fn get_rapid_override(&self) -> RapidOverrideLevel {
        self.state.rapid_override
    }

    fn set_spindle_override(&mut self, percentage: f64) -> anyhow::Result<()> {
        if !(0.0..=200.0).contains(&percentage) {
            return Err(anyhow::anyhow!(
                "Spindle override must be between 0 and 200%, got {}",
                percentage
            ));
        }
        self.state.spindle_override = percentage;
        Ok(())
    }

    fn get_spindle_override(&self) -> f64 {
        self.state.spindle_override
    }

    fn get_state(&self) -> OverrideState {
        self.state
    }
}
