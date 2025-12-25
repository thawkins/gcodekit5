use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Parameters for the Drill Press CAMTool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrillPressParameters {
    /// Final diameter of the hole (mm)
    pub hole_diameter: f64,
    /// Diameter of the tool being used (mm)
    pub tool_diameter: f64,
    /// Z coordinate of the material surface (mm)
    pub top_z: f64,
    /// Final depth of the hole (mm)
    pub bottom_z: f64,
    /// Maximum depth of each plunge (mm). Set to 0 for no pecking.
    pub peck_depth: f64,
    /// Feed rate for vertical movement (mm/min)
    pub plunge_rate: f64,
    /// Feed rate for horizontal movement during helical cycles (mm/min)
    pub feed_rate: f64,
    /// Spindle speed (RPM)
    pub spindle_speed: f64,
    /// Height for safe travel between locations (mm)
    pub safe_z: f64,
    /// X coordinate of the hole center (mm)
    pub x: f64,
    /// Y coordinate of the hole center (mm)
    pub y: f64,
}

/// Generator for Drill Press G-Code
pub struct DrillPressGenerator {
    params: DrillPressParameters,
}

impl DrillPressGenerator {
    /// Create a new DrillPressGenerator with the given parameters
    pub fn new(params: DrillPressParameters) -> Self {
        Self { params }
    }

    /// Generate the G-Code for the drilling operation
    pub fn generate(&self) -> Result<String> {
        let mut gcode = String::new();
        let p = &self.params;

        // Header
        gcode.push_str("; Drill Press Toolpath\n");
        gcode.push_str(&format!("; Hole Diameter: {:.3} mm\n", p.hole_diameter));
        gcode.push_str(&format!("; Tool Diameter: {:.3} mm\n", p.tool_diameter));
        gcode.push_str(&format!(
            "; Depth: {:.3} to {:.3} mm\n",
            p.top_z, p.bottom_z
        ));
        gcode.push_str(&format!("; Center: X{:.3} Y{:.3}\n", p.x, p.y));

        // Initialization
        gcode.push_str("G21 ; Set units to millimeters\n");
        gcode.push_str("G90 ; Absolute positioning\n");
        gcode.push_str(&format!("M3 S{:.0} ; Start spindle\n", p.spindle_speed));
        gcode.push_str(&format!("G0 Z{:.3} ; Move to safe height\n", p.safe_z));
        gcode.push_str(&format!(
            "G0 X{:.3} Y{:.3} ; Move to hole center\n",
            p.x, p.y
        ));

        if p.tool_diameter >= p.hole_diameter {
            self.generate_drilling(&mut gcode)?;
        } else {
            self.generate_helical(&mut gcode)?;
        }

        // Retract and end
        gcode.push_str(&format!("G0 Z{:.3} ; Retract to safe height\n", p.safe_z));
        gcode.push_str("M5 ; Stop spindle\n");
        gcode.push_str("M30 ; End program\n");

        Ok(gcode)
    }

    /// Generate standard or peck drilling G-Code
    fn generate_drilling(&self, gcode: &mut String) -> Result<()> {
        let p = &self.params;
        let target_z = p.bottom_z;
        let start_z = p.top_z;

        if p.peck_depth <= 0.0 {
            // Simple drill
            gcode.push_str("; Simple drilling cycle\n");
            gcode.push_str(&format!("G1 Z{:.3} F{:.1}\n", target_z, p.plunge_rate));
        } else {
            // Peck drill
            gcode.push_str("; Peck drilling cycle\n");
            let mut current_z = start_z;
            while current_z > target_z {
                current_z -= p.peck_depth;
                if current_z < target_z {
                    current_z = target_z;
                }
                gcode.push_str(&format!("G1 Z{:.3} F{:.1}\n", current_z, p.plunge_rate));
                gcode.push_str(&format!("G0 Z{:.3} ; Retract to clear chips\n", start_z));
                if current_z > target_z {
                    // Rapid back to just above last cut (0.5mm clearance)
                    gcode.push_str(&format!("G0 Z{:.3}\n", current_z + 0.5));
                }
            }
        }
        Ok(())
    }

    /// Generate helical interpolation G-Code for holes larger than the tool
    fn generate_helical(&self, gcode: &mut String) -> Result<()> {
        let p = &self.params;
        let radius = (p.hole_diameter - p.tool_diameter) / 2.0;
        let target_z = p.bottom_z;
        let start_z = p.top_z;

        gcode.push_str("; Helical interpolation cycle\n");

        // Move to start of helix (X + radius)
        gcode.push_str(&format!("G0 X{:.3} Y{:.3}\n", p.x + radius, p.y));
        gcode.push_str(&format!("G1 Z{:.3} F{:.1}\n", start_z, p.plunge_rate));

        // Spiral down
        // Use peck_depth as the pitch for the spiral
        let pitch = if p.peck_depth > 0.0 {
            p.peck_depth
        } else {
            1.0
        };
        let mut current_z = start_z;

        while current_z > target_z {
            current_z -= pitch;
            if current_z < target_z {
                current_z = target_z;
            }
            // G2 helical move: I is relative to start point (X+radius, Y), so I = -radius
            gcode.push_str(&format!(
                "G2 X{:.3} Y{:.3} I{:.3} J0.0 Z{:.3} F{:.1}\n",
                p.x + radius,
                p.y,
                -radius,
                current_z,
                p.feed_rate
            ));
        }

        // Final full circle at bottom to ensure clean hole
        gcode.push_str(&format!(
            "G2 X{:.3} Y{:.3} I{:.3} J0.0 F{:.1}\n",
            p.x + radius,
            p.y,
            -radius,
            p.feed_rate
        ));

        // Move back to center before retracting
        gcode.push_str(&format!("G1 X{:.3} Y{:.3} F{:.1}\n", p.x, p.y, p.feed_rate));

        Ok(())
    }
}
