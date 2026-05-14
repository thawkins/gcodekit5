//! G-code generation from toolpaths.

use super::toolpath::{Toolpath, ToolpathSegment, ToolpathSegmentType};
use crate::model::Point;
use gcodekit5_core::Units;

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
    /// Minimum distance between points for laser mode (mm)
    pub min_point_distance: f64,
    /// Tolerance for curve simplification (Ramer-Douglas-Peucker)
    pub curve_simplification_tolerance: f64,
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
            min_point_distance: 0.1,
            curve_simplification_tolerance: 0.05,
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
            min_point_distance: 0.1,
            curve_simplification_tolerance: 0.05,
        }
    }

    /// Activate 2D laser mode
    pub fn with_laser_2d(mut self) -> Self {
        self.is_laser_2d = true;
        self
    }

    /// Set minimum distance between points for laser filtering
    pub fn with_min_point_distance(mut self, distance: f64) -> Self {
        self.min_point_distance = distance;
        self
    }

    /// Set tolerance for curve simplification (higher = more simplification)
    /// Recommended values: 0.05 (minimal), 0.10 (light), 0.15 (medium), 0.25 (aggressive)
    pub fn with_curve_tolerance(mut self, tolerance: f64) -> Self {
        self.curve_simplification_tolerance = tolerance;
        self
    }

    /// Generates G-code from a toolpath (standard version)
    pub fn generate(&self, toolpath: &Toolpath) -> String {
        let mut gcode = String::new();

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

    /// Optimiza un toolpath completo para láser (une colineales Y simplifica curvas)
    pub fn optimize_toolpath_for_laser(&self, toolpath: &Toolpath) -> Toolpath {
        if toolpath.segments.is_empty() {
            return toolpath.clone();
        }

        let mut optimized = Toolpath::new(toolpath.tool_diameter, toolpath.depth);

        let mut last_point: Option<Point> = None;
        let mut pending_segment: Option<ToolpathSegment> = None;

        for segment in &toolpath.segments {
            match segment.segment_type {
                ToolpathSegmentType::RapidMove => {
                    // Flush any pending segment
                    if let Some(pending) = pending_segment.take() {
                        optimized.segments.push(pending);
                    }
                    // Preserve RapidMove
                    optimized.segments.push(segment.clone());
                    last_point = Some(segment.end);
                }

                ToolpathSegmentType::LinearMove => {
                    // Filter: skip points that are too close
                    if let Some(last) = last_point {
                        let dist = last.distance_to(&segment.end);
                        if dist < self.min_point_distance {
                            continue;
                        }
                    }

                    // Try to merge with previous collinear segment
                    if let Some(prev) = &mut pending_segment {
                        if prev.segment_type == ToolpathSegmentType::LinearMove {
                            let dir1 = (prev.end.x - prev.start.x, prev.end.y - prev.start.y);
                            let dir2 = (segment.end.x - prev.end.x, segment.end.y - prev.end.y);
                            let len1 = (dir1.0 * dir1.0 + dir1.1 * dir1.1).sqrt();
                            let len2 = (dir2.0 * dir2.0 + dir2.1 * dir2.1).sqrt();

                            if len1 > 0.01 && len2 > 0.01 {
                                let dot = (dir1.0 * dir2.0 + dir1.1 * dir2.1) / (len1 * len2);
                                if dot > 0.996 {
                                    prev.end = segment.end;
                                    last_point = Some(segment.end);
                                    continue;
                                }
                            }
                            optimized.segments.push(prev.clone());
                        } else {
                            optimized.segments.push(prev.clone());
                        }
                    }
                    pending_segment = Some(segment.clone());
                    last_point = Some(segment.end);
                }

                ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                    if let Some(pending) = pending_segment.take() {
                        optimized.segments.push(pending);
                    }
                    optimized.segments.push(segment.clone());
                    last_point = Some(segment.end);
                }
            }
        }

        if let Some(pending) = pending_segment {
            optimized.segments.push(pending);
        }

        optimized
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
        gcode.push_str(&format!("; Tool diameter: {:.2}mm\n", tool_diameter));
        gcode.push_str(&format!("; Cut depth: {:.2}mm\n", depth));
        gcode.push_str(&format!("; Feed rate: {:.0} mm/min\n", feed_rate));
        gcode.push_str(&format!("; Spindle speed: {} RPM / Power\n", spindle_speed));
        gcode.push_str(&format!("; Total path length: {:.2}mm\n", total_length));
        gcode.push('\n');

        // Setup
        gcode.push_str("G90         ; Absolute positioning\n");
        gcode.push_str("G21         ; Millimeter units\n");
        gcode.push_str("G17         ; XY plane\n");
        gcode.push_str("M5          ; Laser OFF ->\n");

        gcode
    }

    /// Helper para formatear coordenadas a 2 decimales (centésimas)
    fn fmt_coord(&self, value: f64) -> String {
        format!("{:.2}", value)
    }

    /// Helper para obtener prefijo de línea
    fn get_line_prefix(&self, line_number: u32) -> String {
        if self.line_numbers_enabled {
            format!("N{} ", line_number)
        } else {
            String::new()
        }
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
        let laser_power = toolpath
            .segments
            .first()
            .map(|s| s.spindle_speed)
            .unwrap_or(1000);

        // State tracking to avoid redundant commands
        let mut laser_on = false; // Initially OFF
        let mut last_feed_rate: Option<f64> = None;
        let mut last_point: Option<(f64, f64)> = None;
        let mut line_g01 = true;

        let has_z = self.num_axes >= 3 && !self.is_laser_2d;

        for segment in &toolpath.segments {
            match segment.segment_type {
                ToolpathSegmentType::RapidMove => {
                    // Always turn off the laser before rapid movement
                    if self.is_laser_2d && laser_on {
                        let line_prefix = self.get_line_prefix(line_number);
                        gcode.push_str(&format!("{}M5          ; Laser OFF\n", line_prefix));
                        laser_on = false;
                        line_number += 10;
                    }

                    let line_prefix = self.get_line_prefix(line_number);
                    gcode.push_str(&format!(
                        "{}G00 X{} Y{}   ; Rapid move\n",
                        line_prefix,
                        self.fmt_coord(segment.end.x),
                        self.fmt_coord(segment.end.y)
                    ));

                    line_g01 = true;
                    last_point = None;
                    current_z = self.safe_z;
                    line_number += 10;
                }

                ToolpathSegmentType::LinearMove => {
                                    // Handle start Z plunge if needed
                                    if has_z {
                                        if let Some(sz) = segment.start_z {
                                            if (current_z - sz).abs() > 0.01 {
                                                let line_prefix = self.get_line_prefix(line_number);
                                                gcode.push_str(&format!(
                                                    "{}G01 Z{} F{:.0}\n",
                                                    line_prefix,
                                                    self.fmt_coord(sz),
                                                    segment.feed_rate
                                                ));
                                                line_number += 10;
                                                current_z = sz;
                                            }
                                        } else if segment.z_depth.is_none()
                                            && (current_z - toolpath.depth).abs() > 0.01
                                        {
                                            let line_prefix = self.get_line_prefix(line_number);
                                            gcode.push_str(&format!(
                                                "{}G01 Z{} F{:.0}\n",
                                                line_prefix,
                                                self.fmt_coord(toolpath.depth),
                                                segment.feed_rate
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

                    // Laser filtering: skip points that are too close
                    if self.is_laser_2d {
                        if let Some((last_x, last_y)) = last_point {
                            let dx = segment.end.x - last_x;
                            let dy = segment.end.y - last_y;
                            let dist_sq = dx * dx + dy * dy;

                            if dist_sq < self.min_point_distance * self.min_point_distance {
                                continue;
                            }
                        }
                        last_point = Some((segment.end.x, segment.end.y));
                    }

                    if self.is_laser_2d && !laser_on {
                        let line_prefix = self.get_line_prefix(line_number);
                        gcode.push_str(&format!(
                            "{}M4 S{}       ; Laser ON\n",
                            line_prefix, laser_power
                        ));
                        laser_on = true;
                        line_number += 10;
                        line_g01 = true;
                    }

                    // Determine whether to write "G01" or nothing.
                    let cmd = if line_g01 {
                        line_g01 = false; // Marked it as used
                        "G01 "
                    } else {
                        ""
                    };

                    // Only send feed rate if it changed
                    let feed_rate_cmd =
                        if last_feed_rate.is_none_or(|fr| (fr - segment.feed_rate).abs() > 0.1) {
                            last_feed_rate = Some(segment.feed_rate);
                            format!(" F{:.0}", segment.feed_rate)
                        } else {
                            String::new()
                        };

                    let coords = if has_z && (target_z - current_z).abs() > 0.001 {
                        current_z = target_z;
                        format!(
                            "X{} Y{} Z{}",
                            self.fmt_coord(segment.end.x),
                            self.fmt_coord(segment.end.y),
                            self.fmt_coord(target_z)
                        )
                    } else {
                        format!(
                            "X{} Y{}",
                            self.fmt_coord(segment.end.x),
                            self.fmt_coord(segment.end.y)
                        )
                    };

                    gcode.push_str(&format!(
                        "{}{}{}{}{}\n",
                        self.get_line_prefix(line_number),
                        line_prefix,
                        cmd,
                        coords,
                        feed_rate_cmd
                    ));

                    line_number += 10;
                }

                ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                    // Handle Z plunge if needed
                    if has_z {
                        if let Some(sz) = segment.start_z {
                            if (current_z - sz).abs() > 0.01 {
                                let line_prefix = self.get_line_prefix(line_number);
                                gcode.push_str(&format!(
                                    "{}G01 Z{} F{:.0}\n",
                                    line_prefix,
                                    self.fmt_coord(sz),
                                    segment.feed_rate
                                ));
                                line_number += 10;
                                current_z = sz;
                            }
                        } else if segment.z_depth.is_none()
                            && (current_z - toolpath.depth).abs() > 0.01
                        {
                            let line_prefix = self.get_line_prefix(line_number);
                            gcode.push_str(&format!(
                                "{}G01 Z{} F{:.0}\n",
                                line_prefix,
                                self.fmt_coord(toolpath.depth),
                                segment.feed_rate
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

                    let line_prefix = self.get_line_prefix(line_number);

                    // Reset last point for arcs
                    last_point = None;

                    // Turn on the laser
                    if self.is_laser_2d && !laser_on {
                        let line_prefix = self.get_line_prefix(line_number);
                        gcode.push_str(&format!(
                            "{}M4 S{}       ; Laser ON\n",
                            line_prefix, segment.spindle_speed
                        ));
                        laser_on = true;
                        line_number += 10;
                    }

                    let cmd = if segment.segment_type == ToolpathSegmentType::ArcCW {
                        "G02"
                    } else {
                        "G03"
                    };

                    // Only send feed rate if it changed
                    let feed_rate_cmd =
                        if last_feed_rate.is_none_or(|fr| (fr - segment.feed_rate).abs() > 0.1) {
                            last_feed_rate = Some(segment.feed_rate);
                            format!(" F{:.0}", segment.feed_rate)
                        } else {
                            String::new()
                        };

                    if let Some(center) = segment.center {
                        let i = center.x - segment.start.x;
                        let j = center.y - segment.start.y;

                        if has_z && (target_z - current_z).abs() > 0.001 {
                            gcode.push_str(&format!(
                                "{}{} X{} Y{} Z{} I{} J{}{}\n",
                                line_prefix,
                                cmd,
                                self.fmt_coord(segment.end.x),
                                self.fmt_coord(segment.end.y),
                                self.fmt_coord(target_z),
                                self.fmt_coord(i),
                                self.fmt_coord(j),
                                feed_rate_cmd
                            ));
                            current_z = target_z;
                        } else {
                            gcode.push_str(&format!(
                                "{}{} X{} Y{} I{} J{}{}\n",
                                line_prefix,
                                cmd,
                                self.fmt_coord(segment.end.x),
                                self.fmt_coord(segment.end.y),
                                self.fmt_coord(i),
                                self.fmt_coord(j),
                                feed_rate_cmd
                            ));
                        }
                    } else {
                        // Fallback to linear if no center provided
                        if has_z && (target_z - current_z).abs() > 0.001 {
                            gcode.push_str(&format!(
                                "{}G01 X{} Y{} Z{}{}\n",
                                line_prefix,
                                self.fmt_coord(segment.end.x),
                                self.fmt_coord(segment.end.y),
                                self.fmt_coord(target_z),
                                feed_rate_cmd
                            ));
                            current_z = target_z;
                        } else {
                            gcode.push_str(&format!(
                                "{}G01 X{} Y{}{}\n",
                                line_prefix,
                                self.fmt_coord(segment.end.x),
                                self.fmt_coord(segment.end.y),
                                feed_rate_cmd
                            ));
                        }
                    }

                    line_number += 10;
                }
            }
        }

        // Turn off laser at the end of the toolpath if it was on
        if self.is_laser_2d && laser_on {
            let line_prefix = self.get_line_prefix(line_number);
            gcode.push_str(&format!("{}M5          ; Laser OFF\n", line_prefix));
        }

        (gcode, current_z)
    }

    /// Generates the G-code footer.
    pub fn generate_footer(&self) -> String {
        let mut gcode = String::new();
        gcode.push('\n');

        // Turn off laser if not already off (safety)
        if self.is_laser_2d {
            gcode.push_str("M5          ; Laser OFF (safety)\n");
        }

        if self.num_axes >= 3 && !self.is_laser_2d {
            gcode.push_str(&format!(
                "G00 Z{:.2}   ; Raise tool to safe height\n",
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
