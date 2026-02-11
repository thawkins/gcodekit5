use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SpoilboardSurfacingParameters {
    pub width: f64,
    pub height: f64,
    pub tool_diameter: f64,
    pub feed_rate: f64,
    pub spindle_speed: f64,
    pub cut_depth: f64,
    pub stepover_percent: f64,
    pub safe_z: f64,
}

pub struct SpoilboardSurfacingGenerator {
    params: SpoilboardSurfacingParameters,
}

impl SpoilboardSurfacingGenerator {
    pub fn new(params: SpoilboardSurfacingParameters) -> Self {
        Self { params }
    }

    pub fn generate(&self) -> Result<String> {
        let mut gcode = String::new();
        let p = &self.params;

        // Header
        gcode.push_str("; Spoilboard Surfacing Toolpath\n");
        gcode.push_str(&format!(
            "; Dimensions: {:.1} x {:.1} mm\n",
            p.width, p.height
        ));
        gcode.push_str(&format!("; Tool Diameter: {:.1} mm\n", p.tool_diameter));
        gcode.push_str(&format!("; Cut Depth: {:.1} mm\n", p.cut_depth));

        // Initialization sequence
        gcode.push_str("G21 ; Set units to millimeters\n");
        gcode.push_str("G90 ; Absolute positioning\n");
        gcode.push_str("G17 ; XY plane selection\n\n");

        gcode.push_str("; Home and set work coordinate system\n");
        gcode.push_str("$H ; Home all axes (bottom-left corner)\n");
        gcode.push_str("G10 L2 P1 X0 Y0 Z0 ; Clear G54 offset\n");
        gcode.push_str("G54 ; Select work coordinate system 1\n");
        gcode.push_str("G0 X0.0 Y0.0 ; Move to work origin\n");
        gcode.push_str("G10 L20 P1 X0 Y0 Z0 ; Set current position as work zero\n");
        gcode.push_str(&format!("G0 Z{:.3} F500 ; Move to safe height\n", p.safe_z));

        // Spindle start
        gcode.push_str(&format!("M3 S{:.0}\n", p.spindle_speed));

        // Move to Start Position (X0 Y0)
        gcode.push_str("G0 X0 Y0\n");

        // Plunge to Cut Depth
        let target_z = -p.cut_depth.abs();
        gcode.push_str(&format!("G1 Z{:.3} F{:.1}\n", target_z, p.feed_rate / 2.0));

        let step_dist = p.tool_diameter * (p.stepover_percent / 100.0);

        let mut current_y = 0.0;
        let mut going_right = true;

        while current_y <= p.height {
            let target_x = if going_right { p.width } else { 0.0 };
            gcode.push_str(&format!("G1 X{:.3} F{:.1}\n", target_x, p.feed_rate));

            if current_y < p.height {
                current_y += step_dist;
                if current_y > p.height {
                    current_y = p.height;
                }
                gcode.push_str(&format!("G1 Y{:.3} F{:.1}\n", current_y, p.feed_rate));
                going_right = !going_right;
            } else {
                break;
            }
        }

        gcode.push_str(&format!("G0 Z{:.3}\n", p.safe_z));

        // Stop spindle and end program
        gcode.push_str("M5\n");
        gcode.push_str("M30\n");

        Ok(gcode)
    }
}
