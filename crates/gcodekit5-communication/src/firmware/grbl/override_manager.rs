//! GRBL Override Manager
//!
//! Provides real-time override management for GRBL firmware,
//! including feed rate, rapid, and spindle speed overrides.

/// GRBL real-time override commands
/// According to GRBL protocol specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealTimeOverrideCommand {
    /// Feed hold (0x21 = !)
    FeedHold = 0x21,
    /// Cycle start/resume (0x7E = ~)
    CycleStart = 0x7E,
    /// Reset (0x18 = Ctrl+X)
    Reset = 0x18,
    /// Feed rate: 10% decrease (0x92)
    FeedDecrease10 = 0x92,
    /// Feed rate: 1% decrease (0x94)
    FeedDecrease1 = 0x94,
    /// Feed rate: 1% increase (0x93)
    FeedIncrease1 = 0x93,
    /// Feed rate: 10% increase (0x91)
    FeedIncrease10 = 0x91,
    /// Rapid: 25% (0x97)
    RapidOv25 = 0x97,
    /// Rapid: 50% (0x96)
    RapidOv50 = 0x96,
    /// Rapid: 100% (0x95)
    RapidOv100 = 0x95,
    /// Spindle: 100% (0x99)
    SpindleReset = 0x99,
    /// Spindle: 10% increase (0x9A)
    SpindleIncrease10 = 0x9A,
    /// Spindle: 10% decrease (0x9B)
    SpindleDecrease10 = 0x9B,
    /// Spindle: 1% increase (0x9C)
    SpindleIncrease1 = 0x9C,
    /// Spindle: 1% decrease (0x9D)
    SpindleDecrease1 = 0x9D,
    /// Spindle: Stop (0x9E)
    SpindleStop = 0x9E,
}

impl RealTimeOverrideCommand {
    /// Get the real-time byte value for this command
    pub fn as_byte(&self) -> u8 {
        *self as u8
    }
}

/// GRBL Override Manager
///
/// Manages real-time overrides for GRBL including feed rate, rapid, and spindle speed.
#[derive(Debug, Clone)]
pub struct OverrideManager {
    /// Current feed rate override (0-200%)
    feed_override: u16,
    /// Current rapid override (25, 50, 100)
    rapid_override: u8,
    /// Current spindle override (0-200%)
    spindle_override: u16,
    /// Track previous feed override for increment/decrement
    previous_feed: u16,
    /// Track previous spindle override for increment/decrement
    previous_spindle: u16,
}

impl OverrideManager {
    /// Create a new override manager with default values
    pub fn new() -> Self {
        Self {
            feed_override: 100,
            rapid_override: 100,
            spindle_override: 100,
            previous_feed: 100,
            previous_spindle: 100,
        }
    }

    /// Get the current feed rate override percentage
    pub fn get_feed_override(&self) -> u16 {
        self.feed_override
    }

    /// Get the current rapid override percentage
    pub fn get_rapid_override(&self) -> u8 {
        self.rapid_override
    }

    /// Get the current spindle override percentage
    pub fn get_spindle_override(&self) -> u16 {
        self.spindle_override
    }

    /// Set feed rate override
    ///
    /// # Arguments
    /// * `percentage` - Feed rate override percentage (0-200%)
    ///
    /// # Errors
    /// Returns error if percentage is outside valid range
    pub fn set_feed_override(&mut self, percentage: u16) -> anyhow::Result<()> {
        if percentage > 200 {
            return Err(anyhow::anyhow!(
                "Feed override must be 0-200%, got {}",
                percentage
            ));
        }

        self.previous_feed = self.feed_override;
        self.feed_override = percentage;

        Ok(())
    }

    /// Increase feed rate override by 1%
    pub fn increase_feed_1(&mut self) -> anyhow::Result<()> {
        let new_value = std::cmp::min(self.feed_override + 1, 200);
        self.set_feed_override(new_value)?;
        Ok(())
    }

    /// Decrease feed rate override by 1%
    pub fn decrease_feed_1(&mut self) -> anyhow::Result<()> {
        let new_value = if self.feed_override > 0 {
            self.feed_override - 1
        } else {
            0
        };
        self.set_feed_override(new_value)?;
        Ok(())
    }

    /// Increase feed rate override by 10%
    pub fn increase_feed_10(&mut self) -> anyhow::Result<()> {
        let new_value = std::cmp::min(self.feed_override + 10, 200);
        self.set_feed_override(new_value)?;
        Ok(())
    }

    /// Decrease feed rate override by 10%
    pub fn decrease_feed_10(&mut self) -> anyhow::Result<()> {
        let new_value = self.feed_override.saturating_sub(10);
        self.set_feed_override(new_value)?;
        Ok(())
    }

    /// Set rapid override
    ///
    /// # Arguments
    /// * `percentage` - Rapid override percentage (must be 25, 50, or 100)
    ///
    /// # Errors
    /// Returns error if percentage is not one of the valid options
    pub fn set_rapid_override(&mut self, percentage: u8) -> anyhow::Result<()> {
        if ![25, 50, 100].contains(&percentage) {
            return Err(anyhow::anyhow!(
                "Rapid override must be 25, 50, or 100%, got {}",
                percentage
            ));
        }

        self.rapid_override = percentage;

        Ok(())
    }

    /// Get the real-time command for the current rapid override
    pub fn get_rapid_override_command(&self) -> RealTimeOverrideCommand {
        match self.rapid_override {
            25 => RealTimeOverrideCommand::RapidOv25,
            50 => RealTimeOverrideCommand::RapidOv50,
            _ => RealTimeOverrideCommand::RapidOv100,
        }
    }

    /// Set spindle override
    ///
    /// # Arguments
    /// * `percentage` - Spindle override percentage (0-200%)
    ///
    /// # Errors
    /// Returns error if percentage is outside valid range
    pub fn set_spindle_override(&mut self, percentage: u16) -> anyhow::Result<()> {
        if percentage > 200 {
            return Err(anyhow::anyhow!(
                "Spindle override must be 0-200%, got {}",
                percentage
            ));
        }

        self.previous_spindle = self.spindle_override;
        self.spindle_override = percentage;

        Ok(())
    }

    /// Increase spindle override by 1%
    pub fn increase_spindle_1(&mut self) -> anyhow::Result<()> {
        let new_value = std::cmp::min(self.spindle_override + 1, 200);
        self.set_spindle_override(new_value)?;
        Ok(())
    }

    /// Decrease spindle override by 1%
    pub fn decrease_spindle_1(&mut self) -> anyhow::Result<()> {
        let new_value = if self.spindle_override > 0 {
            self.spindle_override - 1
        } else {
            0
        };
        self.set_spindle_override(new_value)?;
        Ok(())
    }

    /// Increase spindle override by 10%
    pub fn increase_spindle_10(&mut self) -> anyhow::Result<()> {
        let new_value = std::cmp::min(self.spindle_override + 10, 200);
        self.set_spindle_override(new_value)?;
        Ok(())
    }

    /// Decrease spindle override by 10%
    pub fn decrease_spindle_10(&mut self) -> anyhow::Result<()> {
        let new_value = self.spindle_override.saturating_sub(10);
        self.set_spindle_override(new_value)?;
        Ok(())
    }

    /// Stop spindle
    pub fn stop_spindle(&mut self) -> anyhow::Result<()> {
        self.previous_spindle = self.spindle_override;
        self.spindle_override = 0;
        Ok(())
    }

    /// Reset all overrides to 100%
    pub fn reset_all(&mut self) {
        self.feed_override = 100;
        self.rapid_override = 100;
        self.spindle_override = 100;
    }

    /// Check if any override is different from default (100%)
    pub fn is_overridden(&self) -> bool {
        self.feed_override != 100 || self.rapid_override != 100 || self.spindle_override != 100
    }

    /// Get the real-time command to apply the current feed override
    /// Returns the command that would take us to the current override level
    pub fn get_feed_override_command(&self) -> Option<RealTimeOverrideCommand> {
        if self.feed_override == self.previous_feed {
            return None;
        }

        if self.feed_override > self.previous_feed {
            // Determine if it's a 1% or 10% increase
            let diff = self.feed_override - self.previous_feed;
            if diff >= 10 {
                Some(RealTimeOverrideCommand::FeedIncrease10)
            } else {
                Some(RealTimeOverrideCommand::FeedIncrease1)
            }
        } else {
            // Determine if it's a 1% or 10% decrease
            let diff = self.previous_feed - self.feed_override;
            if diff >= 10 {
                Some(RealTimeOverrideCommand::FeedDecrease10)
            } else {
                Some(RealTimeOverrideCommand::FeedDecrease1)
            }
        }
    }

    /// Get the real-time command to apply the current spindle override
    pub fn get_spindle_override_command(&self) -> Option<RealTimeOverrideCommand> {
        if self.spindle_override == self.previous_spindle {
            return None;
        }

        if self.spindle_override == 0 {
            return Some(RealTimeOverrideCommand::SpindleStop);
        }

        if self.spindle_override > self.previous_spindle {
            let diff = self.spindle_override - self.previous_spindle;
            if diff >= 10 {
                Some(RealTimeOverrideCommand::SpindleIncrease10)
            } else {
                Some(RealTimeOverrideCommand::SpindleIncrease1)
            }
        } else {
            let diff = self.previous_spindle - self.spindle_override;
            if diff >= 10 {
                Some(RealTimeOverrideCommand::SpindleDecrease10)
            } else {
                Some(RealTimeOverrideCommand::SpindleDecrease1)
            }
        }
    }
}

impl Default for OverrideManager {
    fn default() -> Self {
        Self::new()
    }
}
