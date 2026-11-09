//! Smoothieware command creator
//!
//! Generates G-code and M-code commands specific to Smoothieware.

use gcodekit5_core::Position;

/// Smoothieware command creator
#[derive(Debug, Clone)]
pub struct SmoothiewareCommandCreator;

impl SmoothiewareCommandCreator {
    /// Create a new command creator
    pub fn new() -> Self {
        Self
    }

    /// Create a jog command for Smoothieware
    pub fn jog_command(&self, axis: char, distance: f64, feed_rate: f64) -> String {
        format!(
            "G1 {}:{:.2} F{:.0}",
            axis.to_ascii_uppercase(),
            distance,
            feed_rate
        )
    }

    /// Create a move command
    pub fn move_command(&self, position: &Position, feed_rate: f64) -> String {
        let mut cmd = String::from("G1");
        cmd.push_str(&format!(" X{:.2}", position.x));
        cmd.push_str(&format!(" Y{:.2}", position.y));
        cmd.push_str(&format!(" Z{:.2}", position.z));
        if let Some(a) = position.a {
            cmd.push_str(&format!(" A{:.2}", a));
        }
        cmd.push_str(&format!(" F{:.0}", feed_rate));
        cmd
    }

    /// Create a rapid move command
    pub fn rapid_move_command(&self, position: &Position) -> String {
        let mut cmd = String::from("G0");
        cmd.push_str(&format!(" X{:.2}", position.x));
        cmd.push_str(&format!(" Y{:.2}", position.y));
        cmd.push_str(&format!(" Z{:.2}", position.z));
        if let Some(a) = position.a {
            cmd.push_str(&format!(" A{:.2}", a));
        }
        cmd
    }

    /// Create a home command
    pub fn home_command(&self, axes: Option<&str>) -> String {
        match axes {
            Some(a) => format!("G28.2 {}", a),
            None => "G28.2".to_string(),
        }
    }

    /// Create a spindle on command (clockwise)
    pub fn spindle_on_cw(&self, speed: u16) -> String {
        format!("M3 S{}", speed)
    }

    /// Create a spindle on command (counter-clockwise)
    pub fn spindle_on_ccw(&self, speed: u16) -> String {
        format!("M4 S{}", speed)
    }

    /// Create a spindle off command
    pub fn spindle_off(&self) -> String {
        "M5".to_string()
    }

    /// Create a dwell command
    pub fn dwell(&self, seconds: f64) -> String {
        // Smoothieware interprets G4 P as milliseconds (unless grbl_mode is on);
        // the S word is seconds in every mode.
        format!("G4 S{:.3}", seconds)
    }

    /// Create a status request command
    pub fn status_request(&self) -> String {
        "M114".to_string()
    }

    /// Create a reset command
    pub fn reset(&self) -> String {
        "M999".to_string()
    }

    /// Create a probe command
    pub fn probe_command(
        &self,
        axis: char,
        distance: f64,
        feed_rate: f64,
        pull_off: f64,
    ) -> String {
        format!(
            "G38.2 {}:{:.2} F{:.0}; G1 {}:{:.2}",
            axis.to_ascii_uppercase(),
            distance,
            feed_rate,
            axis.to_ascii_uppercase(),
            pull_off
        )
    }

    /// Create an arc command (clockwise)
    pub fn arc_cw(&self, end: &Position, center_x: f64, center_y: f64, feed_rate: f64) -> String {
        let mut cmd = String::from("G2");
        cmd.push_str(&format!(" X{:.2}", end.x));
        cmd.push_str(&format!(" Y{:.2}", end.y));
        cmd.push_str(&format!(" I{:.2} J{:.2}", center_x, center_y));
        cmd.push_str(&format!(" F{:.0}", feed_rate));
        cmd
    }

    /// Create an arc command (counter-clockwise)
    pub fn arc_ccw(&self, end: &Position, center_x: f64, center_y: f64, feed_rate: f64) -> String {
        let mut cmd = String::from("G3");
        cmd.push_str(&format!(" X{:.2}", end.x));
        cmd.push_str(&format!(" Y{:.2}", end.y));
        cmd.push_str(&format!(" I{:.2} J{:.2}", center_x, center_y));
        cmd.push_str(&format!(" F{:.0}", feed_rate));
        cmd
    }
}

impl Default for SmoothiewareCommandCreator {
    fn default() -> Self {
        Self::new()
    }
}
