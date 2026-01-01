//! G-code generation from toolpaths.

use super::toolpath::{Toolpath, ToolpathSegmentType};
use gcodekit5_core::Units;

/// G-code generator for converting toolpaths to G-code commands.
pub struct ToolpathToGcode {
    _units: Units,
    /// Safe Z height for rapid moves between shapes
    pub safe_z: f64,
    line_numbers_enabled: bool,
}

impl ToolpathToGcode {
    /// Creates a new G-code generator.
    pub fn new(units: Units, safe_z: f64) -> Self {
        Self {
            _units: units,
            safe_z,
            line_numbers_enabled: false,
        }
    }

    /// Creates a new G-code generator with line numbers enabled.
    pub fn with_line_numbers(units: Units, safe_z: f64, enabled: bool) -> Self {
        Self {
            _units: units,
            safe_z,
            line_numbers_enabled: enabled,
        }
    }

    /// Generates G-code from a toolpath.
    pub fn generate(&self, toolpath: &Toolpath) -> String {
        let mut gcode = String::new();

        // Get spindle speed and feed rate from first segment (all should have same parameters)
        let spindle_speed = toolpath
            .segments
            .first()
            .map(|s| s.spindle_speed)
            .unwrap_or(1000);
        let feed_rate = toolpath
            .segments
            .first()
            .map(|s| s.feed_rate)
            .unwrap_or(100.0);

        gcode.push_str(&self.generate_header(
            spindle_speed,
            feed_rate,
            toolpath.tool_diameter,
            toolpath.depth,
            toolpath.total_length(),
        ));
        gcode.push_str(&self.generate_body(toolpath, 10));
        gcode.push_str(&self.generate_footer());

        gcode
    }

    /// Generates the G-code header.
    pub fn generate_header(
        &self,
        spindle_speed: u32,
        feed_rate: f64,
        tool_diameter: f64,
        depth: f64,
        total_length: f64,
    ) -> String {
        let mut gcode = String::new();
        gcode.push_str("; Generated G-code from Designer tool\n");
        gcode.push_str(&format!("; Tool diameter: {:.3}mm\n", tool_diameter));
        gcode.push_str(&format!("; Cut depth: {:.3}mm\n", depth));
        gcode.push_str(&format!("; Feed rate: {:.0} mm/min\n", feed_rate));
        gcode.push_str(&format!("; Spindle speed: {} RPM\n", spindle_speed));
        gcode.push_str(&format!("; Total path length: {:.3}mm\n", total_length));
        gcode.push('\n');

        // Setup
        gcode.push_str("G90         ; Absolute positioning\n");
        gcode.push_str("G21         ; Millimeter units\n");
        gcode.push_str("G17         ; XY plane\n");
        gcode.push_str(&format!(
            "M3 S{}      ; Spindle on at {} RPM\n",
            spindle_speed, spindle_speed
        ));
        gcode.push('\n');
        gcode
    }

    /// Generates the G-code body (moves) for a toolpath.
    pub fn generate_body(&self, toolpath: &Toolpath, start_line_number: u32) -> String {
        self.generate_body_continuing(toolpath, start_line_number, self.safe_z).0
    }

    /// Generates the G-code body continuing from a given Z position.
    /// Returns (gcode_string, final_z_position) to allow chaining toolpaths without unnecessary retracts.
    pub fn generate_body_continuing(
        &self,
        toolpath: &Toolpath,
        start_line_number: u32,
        initial_z: f64,
    ) -> (String, f64) {
        let mut gcode = String::new();
        let mut line_number = start_line_number;
        let mut current_z = initial_z;

        for segment in &toolpath.segments {
            match segment.segment_type {
                ToolpathSegmentType::RapidMove => {
                    // Retract to safe Z before changing XY to avoid diagonal plunges
                    if (current_z - self.safe_z).abs() > 0.001 {
                        let line_prefix = if self.line_numbers_enabled {
                            format!("N{} ", line_number)
                        } else {
                            String::new()
                        };
                        gcode.push_str(&format!("{}G00 Z{:.3}\n", line_prefix, self.safe_z));
                        line_number += 10;
                    }

                    let line_prefix = if self.line_numbers_enabled {
                        format!("N{} ", line_number)
                    } else {
                        String::new()
                    };
                    gcode.push_str(&format!(
                        "{}G00 X{:.3} Y{:.3} Z{:.3}\n",
                        line_prefix, segment.end.x, segment.end.y, self.safe_z
                    ));
                    current_z = self.safe_z;
                }
                ToolpathSegmentType::LinearMove => {
                    // Handle start Z plunge if needed
                    if let Some(sz) = segment.start_z {
                        if (current_z - sz).abs() > 0.01 {
                            let line_prefix = if self.line_numbers_enabled {
                                format!("N{} ", line_number)
                            } else {
                                String::new()
                            };
                            gcode.push_str(&format!(
                                "{}G01 Z{:.3} F{:.0}\n",
                                line_prefix, sz, segment.feed_rate
                            ));
                            line_number += 10;
                            current_z = sz;
                        }
                    } else if segment.z_depth.is_none() {
                        // Plunge to cutting depth once per cutting section.
                        if (current_z - toolpath.depth).abs() > 0.01 {
                            let line_prefix = if self.line_numbers_enabled {
                                format!("N{} ", line_number)
                            } else {
                                String::new()
                            };
                            gcode.push_str(&format!(
                                "{}G01 Z{:.3} F{:.0}\n",
                                line_prefix, toolpath.depth, segment.feed_rate
                            ));
                            line_number += 10;
                            current_z = toolpath.depth;
                        }
                    }

                    let target_z = segment.z_depth.unwrap_or(if segment.start_z.is_some() {
                        current_z
                    } else {
                        toolpath.depth
                    });

                    // Linear move (G01)
                    let line_prefix = if self.line_numbers_enabled {
                        format!("N{} ", line_number)
                    } else {
                        String::new()
                    };

                    if (target_z - current_z).abs() > 0.001 {
                        gcode.push_str(&format!(
                            "{}G01 X{:.3} Y{:.3} Z{:.3} F{:.0}\n",
                            line_prefix, segment.end.x, segment.end.y, target_z, segment.feed_rate
                        ));
                        current_z = target_z;
                    } else {
                        gcode.push_str(&format!(
                            "{}G01 X{:.3} Y{:.3} F{:.0}\n",
                            line_prefix, segment.end.x, segment.end.y, segment.feed_rate
                        ));
                    }
                }
                ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                    // Handle start Z plunge if needed
                    if let Some(sz) = segment.start_z {
                        if (current_z - sz).abs() > 0.01 {
                            let line_prefix = if self.line_numbers_enabled {
                                format!("N{} ", line_number)
                            } else {
                                String::new()
                            };
                            gcode.push_str(&format!(
                                "{}G01 Z{:.3} F{:.0}\n",
                                line_prefix, sz, segment.feed_rate
                            ));
                            line_number += 10;
                            current_z = sz;
                        }
                    } else if segment.z_depth.is_none() {
                        // Plunge to cutting depth once per cutting section.
                        if (current_z - toolpath.depth).abs() > 0.01 {
                            let line_prefix = if self.line_numbers_enabled {
                                format!("N{} ", line_number)
                            } else {
                                String::new()
                            };
                            gcode.push_str(&format!(
                                "{}G01 Z{:.3} F{:.0}\n",
                                line_prefix, toolpath.depth, segment.feed_rate
                            ));
                            line_number += 10;
                            current_z = toolpath.depth;
                        }
                    }

                    let target_z = segment.z_depth.unwrap_or(if segment.start_z.is_some() {
                        current_z
                    } else {
                        toolpath.depth
                    });

                    let line_prefix = if self.line_numbers_enabled {
                        format!("N{} ", line_number)
                    } else {
                        String::new()
                    };

                    let cmd = if segment.segment_type == ToolpathSegmentType::ArcCW {
                        "G02"
                    } else {
                        "G03"
                    };

                    if let Some(center) = segment.center {
                        let i = center.x - segment.start.x;
                        let j = center.y - segment.start.y;

                        if (target_z - current_z).abs() > 0.001 {
                            gcode.push_str(&format!(
                                "{}{} X{:.3} Y{:.3} Z{:.3} I{:.3} J{:.3} F{:.0}\n",
                                line_prefix,
                                cmd,
                                segment.end.x,
                                segment.end.y,
                                target_z,
                                i,
                                j,
                                segment.feed_rate
                            ));
                            current_z = target_z;
                        } else {
                            gcode.push_str(&format!(
                                "{}{} X{:.3} Y{:.3} I{:.3} J{:.3} F{:.0}\n",
                                line_prefix,
                                cmd,
                                segment.end.x,
                                segment.end.y,
                                i,
                                j,
                                segment.feed_rate
                            ));
                        }
                    } else {
                        // Fallback to linear if no center provided (should not happen for arcs)
                        if (target_z - current_z).abs() > 0.001 {
                            gcode.push_str(&format!(
                                "{}G01 X{:.3} Y{:.3} Z{:.3} F{:.0}\n",
                                line_prefix,
                                segment.end.x,
                                segment.end.y,
                                target_z,
                                segment.feed_rate
                            ));
                            current_z = target_z;
                        } else {
                            gcode.push_str(&format!(
                                "{}G01 X{:.3} Y{:.3} F{:.0}\n",
                                line_prefix, segment.end.x, segment.end.y, segment.feed_rate
                            ));
                        }
                    }
                }
            }

            line_number += 10;
        }
        (gcode, current_z)
    }

    /// Generates the G-code footer.
    pub fn generate_footer(&self) -> String {
        let mut gcode = String::new();
        gcode.push('\n');
        gcode.push_str("M5          ; Spindle off\n");
        gcode.push_str(&format!("G00 Z{:.3}   ; Raise tool to safe height\n", self.safe_z));
        gcode.push_str("G00 X0 Y0   ; Return to origin\n");
        gcode.push_str("M30         ; End program\n");
        gcode
    }
}

impl Default for ToolpathToGcode {
    fn default() -> Self {
        Self::new(Units::MM, 10.0)
    }
}
