// crates/gcodekit5-core/src/gcode_parser.rs

use crate::Point;

#[derive(Debug, Clone)]
pub struct GCodeToolpath {
    pub segments: Vec<GCodeSegment>,
}

#[derive(Debug, Clone)]
pub struct GCodeSegment {
    pub typ: GCodeSegmentType,
    pub start: Point,
    pub end: Point,
    pub center: Option<Point>,
    pub feed_rate: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GCodeSegmentType {
    Rapid,
    Linear,
    ArcCW,
    ArcCCW,
}

pub fn parse_gcode_to_toolpaths(gcode: &str) -> Result<Vec<GCodeToolpath>, String> {
    let mut toolpaths = Vec::new();
    let mut current_toolpath = GCodeToolpath { segments: Vec::new() };
    let mut current_pos = Point::new(0.0, 0.0);
    let mut absolute_mode = true;

    for line in gcode.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        // Parsear comandos G
        if line.starts_with('G') {
            // ... lógica de parseo ...
        }
    }

    Ok(toolpaths)
}
