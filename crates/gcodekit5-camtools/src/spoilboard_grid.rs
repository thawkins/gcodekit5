//! # Spoilboard Grid Generator
//!
//! Generates G-code for laser-engraved alignment grids on CNC spoilboards.
//! Produces a rectangular grid pattern at configurable spacing with
//! laser power and feed rate parameters.

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SpoilboardGridParameters {
    pub width: f64,
    pub height: f64,
    pub grid_spacing: f64,
    pub feed_rate: f64,
    pub laser_power: f64,
    pub laser_mode: String, // "M3" or "M4"
}

pub struct SpoilboardGridGenerator {
    params: SpoilboardGridParameters,
}

impl SpoilboardGridGenerator {
    pub fn new(params: SpoilboardGridParameters) -> Self {
        Self { params }
    }

    pub fn generate(&self) -> Result<String> {
        let mut gcode = String::new();
        let p = &self.params;

        // Header
        gcode.push_str("; Spoilboard Grid Toolpath\n");
        gcode.push_str(&format!(
            "; Dimensions: {:.1} x {:.1} mm\n",
            p.width, p.height
        ));
        gcode.push_str(&format!("; Grid Spacing: {:.1} mm\n", p.grid_spacing));
        gcode.push_str(&format!("; Laser Power: {:.1}\n", p.laser_power));

        // Initialization sequence
        gcode.push_str("G21 ; Set units to millimeters\n");
        gcode.push_str("G90 ; Absolute positioning\n");
        gcode.push_str("G17 ; XY plane selection\n\n");

        gcode.push_str("; Home and set work coordinate system\n");
        gcode.push_str("$H ; Home all axes (bottom-left corner)\n");
        gcode.push_str("G10 L2 P1 X0 Y0 ; Clear G54 offset\n");
        gcode.push_str("G54 ; Select work coordinate system 1\n");
        gcode.push_str("G0 X0.0 Y0.0 ; Move to work origin\n");
        gcode.push_str("G10 L20 P1 X0 Y0 ; Set current position as work zero\n");

        // Enable Laser Mode (GRBL specific, but good practice)
        gcode.push_str("$32=1 ; Enable Laser Mode\n");

        // Vertical Lines (moving along Y)
        gcode.push_str("\n; Vertical Lines\n");
        let mut going_up = true;
        let mut current_x = 0.0;
        // Use epsilon for float comparison
        while current_x <= p.width + 0.001 {
            let start_y = if going_up { 0.0 } else { p.height };
            let end_y = if going_up { p.height } else { 0.0 };

            // Rapid to start
            gcode.push_str("M5\n");
            gcode.push_str(&format!("G0 X{:.3} Y{:.3}\n", current_x, start_y));

            // Cut
            gcode.push_str(&format!("{} S{:.0}\n", p.laser_mode, p.laser_power));
            gcode.push_str(&format!("G1 Y{:.3} F{:.1}\n", end_y, p.feed_rate));

            current_x += p.grid_spacing;
            going_up = !going_up;
        }

        // Horizontal Lines (moving along X)
        gcode.push_str("\n; Horizontal Lines\n");
        let mut going_right = true;
        let mut current_y = 0.0;
        while current_y <= p.height + 0.001 {
            let start_x = if going_right { 0.0 } else { p.width };
            let end_x = if going_right { p.width } else { 0.0 };

            // Rapid to start
            gcode.push_str("M5\n");
            gcode.push_str(&format!("G0 X{:.3} Y{:.3}\n", start_x, current_y));

            // Cut
            gcode.push_str(&format!("{} S{:.0}\n", p.laser_mode, p.laser_power));
            gcode.push_str(&format!("G1 X{:.3} F{:.1}\n", end_x, p.feed_rate));

            current_y += p.grid_spacing;
            going_right = !going_right;
        }

        // Finish
        gcode.push_str("\nM5 ; Laser Off\n");
        gcode.push_str("G0 X0 Y0 ; Return to origin\n");
        gcode.push_str("M30 ; End program\n");

        Ok(gcode)
    }
}
