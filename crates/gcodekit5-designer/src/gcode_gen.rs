//! G-code generation from toolpaths.

use super::toolpath::{Toolpath, ToolpathSegmentType};
use gcodekit5_core::Units;
// use crate::model::Point;

/// G-code generator for converting toolpaths to G-code commands.
pub struct ToolpathToGcode {
    _units: Units,
    /// Safe Z height for rapid moves between shapes
    pub safe_z: f64,
    line_numbers_enabled: bool,
    /// Number of axes on the target device (default 3).
    pub num_axes: u8,
    /// Laser Mode 2D (without Z axis)
    pub is_laser_2d: bool,
}

impl ToolpathToGcode {
    /// Creates a new G-code generator.
    pub fn new(units: Units, safe_z: f64) -> Self {
        Self {
            _units: units,
            safe_z,
            line_numbers_enabled: false,
            num_axes: 3,
            is_laser_2d: false,
        }
    }

    /// Creates a new G-code generator with line numbers enabled.
    pub fn with_line_numbers(units: Units, safe_z: f64, enabled: bool) -> Self {
        Self {
            _units: units,
            safe_z,
            line_numbers_enabled: enabled,
            num_axes: 3,
            is_laser_2d: false,
        }
    }

    /// Activate 2D laser mode
    pub fn with_laser_2d(mut self) -> Self {
        self.is_laser_2d = true;
        self
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

        // KEY CHANGE: Use M4 for dynamic laser mode
        // To remove burn marks on 1-2mm curves
        gcode.push_str(&format!("M4 S{}      ; Laser Dynamic Mode\n", spindle_speed));

        gcode.push('\n');
        gcode
    }

    /// Generates the G-code body (moves) for a toolpath.
    pub fn generate_body(&self, toolpath: &Toolpath, start_line_number: u32) -> String {
        self.generate_body_continuing(toolpath, start_line_number, self.safe_z)
        .0
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
        // We save the last recorded position to calculate the distance
        //        let mut last_recorded_pos: Option<Point> = None;
        let mut last_x: Option<f64> = None;
        let mut last_y: Option<f64> = None;

        let has_z = self.num_axes >= 3 && !self.is_laser_2d;
        let mut first_cut_move = true;

        for segment in &toolpath.segments {

            if self.is_laser_2d {
                if segment.segment_type == ToolpathSegmentType::RapidMove {
                    // Si es un salto G0, reseteamos el filtro para no perder precisión
                    last_x = None;
                    last_y = None;
                } else if segment.segment_type == ToolpathSegmentType::LinearMove {
                    if let (Some(lx), Some(ly)) = (last_x, last_y) {
                        let dx = segment.end.x - lx;
                        let dy = segment.end.y - ly;
                        // Filtro de 0.25mm (0.25 * 0.25 = 0.0625)
                        if (dx * dx + dy * dy) < 0.0625 {
                            continue; // Saltamos este micro-punto
                        }
                    }
                    // Guardamos posición actual
                    last_x = Some(segment.end.x);
                    last_y = Some(segment.end.y);
                }
            }


            match segment.segment_type {
                ToolpathSegmentType::RapidMove => {
                    let line_prefix = if self.line_numbers_enabled {
                        format!("N{} ", line_number)
                    } else {
                        String::new()
                    };

                    // GENERATE RAPID MOVEMENT (without laser)
                    if self.is_laser_2d {
                        gcode.push_str(&format!(
                            "{}G00 X{:.3} Y{:.3}   ; Posicionar\n",
                            line_prefix, segment.end.x, segment.end.y
                        ));
                    }

                    current_z = self.safe_z;
                    line_number += 10;
                }

                ToolpathSegmentType::LinearMove => {
                    // Handle start Z plunge if needed
                    if has_z {
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

                    // In laser mode, turn on just before the first cutting movement
                    if self.is_laser_2d && first_cut_move {
                        let speed_value = if self.is_laser_2d {
                            (segment.spindle_speed as f64) as u32
                        } else {
                            segment.spindle_speed
                        };

                        gcode.push_str(&format!(
                            "{}M4 S{}      ; Laser Dynamic ON\n",
                            line_prefix, speed_value
                        ));
                        first_cut_move = false;
                    }

                    if has_z && (target_z - current_z).abs() > 0.001 {
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
                    if has_z {
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

                    // In laser mode, turn on just before the first cutting movement
                    if self.is_laser_2d && first_cut_move {
                        // Apply factor ×10 only for laser
                        let speed_value = if self.is_laser_2d {
                            (segment.spindle_speed as f64) as u32
                        } else {
                            segment.spindle_speed
                        };

                        gcode.push_str(&format!(
                            "{}M4 S{}      ; Laser Dynamic ON\n\n",
                            line_prefix, speed_value
                        ));
                        first_cut_move = false;
                    }

                    let cmd = if segment.segment_type == ToolpathSegmentType::ArcCW {
                        "G02"
                    } else {
                        "G03"
                    };

                    if let Some(center) = segment.center {
                        let i = center.x - segment.start.x;
                        let j = center.y - segment.start.y;

                        if has_z && (target_z - current_z).abs() > 0.001 {
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
                        // Fallback to linear if no center provided
                        if has_z && (target_z - current_z).abs() > 0.001 {
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
        } // End for segment in &toolpath.segments

        // At the end of the toolpath, if it's laser mode, turn off
        if self.is_laser_2d {
            let line_prefix = if self.line_numbers_enabled {
                format!("N{} ", line_number)
            } else {
                String::new()
            };
            gcode.push_str(&format!("{}M5          ; Laser off\n", line_prefix));

        }

        (gcode, current_z)
    }

    /// Generates the G-code footer.
    pub fn generate_footer(&self) -> String {
        let mut gcode = String::new();
        gcode.push('\n');
        gcode.push_str("M5          ; Laser off\n");
        if self.num_axes >= 3 && !self.is_laser_2d {
            gcode.push_str(&format!(
                "G00 Z{:.3}   ; Raise tool to safe height\n",
                self.safe_z
            ));
        }

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

