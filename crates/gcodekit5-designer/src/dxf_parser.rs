//! # DXF Import and Parsing Module
//!
//! Provides comprehensive DXF file parsing and entity extraction for the Designer tool.
//!
//! Supports:
//! - DXF R2000+ format parsing
//! - Entity extraction (lines, circles, arcs, polylines, ellipse, text)
//! - Layer and block handling
//! - Coordinate system transformation
//! - Unit conversion
//! - Color and linetype mapping

use crate::Point;
use anyhow::Result;
use std::collections::HashMap;

/// DXF entity types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxfEntityType {
    /// Line entity
    Line,
    /// Circle entity
    Circle,
    /// Arc entity
    Arc,
    /// Polyline entity
    Polyline,
    /// LwPolyline (light-weight polyline)
    LwPolyline,
    /// Text entity
    Text,
    /// Point entity
    Point,
    /// Spline entity
    Spline,
    /// Block reference (insert)
    BlockReference,
    /// Ellipse
    Ellipse,
    /// Other/unsupported entity
    Other,
}

/// DXF coordinate system units
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxfUnit {
    /// No units specified
    Unitless,
    /// Inches
    Inches,
    /// Feet
    Feet,
    /// Millimeters
    Millimeters,
    /// Centimeters
    Centimeters,
    /// Meters
    Meters,
    /// Kilometers
    Kilometers,
}

impl DxfUnit {
    /// Get conversion factor to millimeters
    pub fn to_mm_factor(&self) -> f64 {
        match self {
            DxfUnit::Unitless => 1.0,
            DxfUnit::Inches => 25.4,
            DxfUnit::Feet => 304.8,
            DxfUnit::Millimeters => 1.0,
            DxfUnit::Centimeters => 10.0,
            DxfUnit::Meters => 1000.0,
            DxfUnit::Kilometers => 1_000_000.0,
        }
    }
}

/// Represents a DXF line entity
#[derive(Debug, Clone)]
pub struct DxfLine {
    /// Start point
    pub start: Point,
    /// End point
    pub end: Point,
    /// Layer name
    pub layer: String,
    /// Color (ACI value or 256 for by-layer)
    pub color: u16,
}

/// Represents a DXF circle entity
#[derive(Debug, Clone)]
pub struct DxfCircle {
    /// Center point
    pub center: Point,
    /// Radius
    pub radius: f64,
    /// Layer name
    pub layer: String,
    /// Color
    pub color: u16,
}

/// Represents a DXF arc entity
#[derive(Debug, Clone)]
pub struct DxfArc {
    /// Center point
    pub center: Point,
    /// Radius
    pub radius: f64,
    /// Start angle in degrees
    pub start_angle: f64,
    /// End angle in degrees
    pub end_angle: f64,
    /// Layer name
    pub layer: String,
    /// Color
    pub color: u16,
}

/// Represents a DXF polyline entity
#[derive(Debug, Clone)]
pub struct DxfPolyline {
    /// Vertices
    pub vertices: Vec<Point>,
    /// Bulge factors para cada segmento (0 = recta, >0 = arco)
    pub bulges: Vec<f64>,
    /// Whether the polyline is closed
    pub closed: bool,
    /// Layer name
    pub layer: String,
    /// Color
    pub color: u16,
}

/// Represents a DXF text entity
#[derive(Debug, Clone)]
pub struct DxfText {
    /// Text content
    pub content: String,
    /// Insertion point
    pub position: Point,
    /// Text height
    pub height: f64,
    /// Rotation angle in degrees
    pub rotation: f64,
    /// Layer name
    pub layer: String,
    /// Color
    pub color: u16,
}

/// DXF entity wrapper
#[derive(Debug, Clone)]
pub enum DxfEntity {
    /// Line entity
    Line(DxfLine),
    /// Circle entity
    Circle(DxfCircle),
    /// Arc entity
    Arc(DxfArc),
    /// Polyline entity
    Polyline(DxfPolyline),
    /// Text entity
    Text(DxfText),
    /// Ellipse
    Ellipse(DxfEllipse),
}

impl DxfEntity {
    /// Get entity type
    pub fn entity_type(&self) -> DxfEntityType {
        match self {
            DxfEntity::Line(_) => DxfEntityType::Line,
            DxfEntity::Circle(_) => DxfEntityType::Circle,
            DxfEntity::Arc(_) => DxfEntityType::Arc,
            DxfEntity::Polyline(_) => DxfEntityType::Polyline,
            DxfEntity::Text(_) => DxfEntityType::Text,
            DxfEntity::Ellipse(_) => DxfEntityType::Ellipse,
        }
    }

    /// Get layer name
    pub fn layer(&self) -> &str {
        match self {
            DxfEntity::Line(l) => &l.layer,
            DxfEntity::Circle(c) => &c.layer,
            DxfEntity::Arc(a) => &a.layer,
            DxfEntity::Polyline(p) => &p.layer,
            DxfEntity::Text(t) => &t.layer,
            DxfEntity::Ellipse(e) => &e.layer,
        }
    }

    /// Get color
    pub fn color(&self) -> u16 {
        match self {
            DxfEntity::Line(l) => l.color,
            DxfEntity::Circle(c) => c.color,
            DxfEntity::Arc(a) => a.color,
            DxfEntity::Polyline(p) => p.color,
            DxfEntity::Text(t) => t.color,
            DxfEntity::Ellipse(e) => e.color,
        }
    }
}

/// DXF file header containing document properties
#[derive(Debug, Clone)]
pub struct DxfHeader {
    /// DXF version (e.g., "AC1021" for R2000)
    pub version: String,
    /// Unit system used in drawing
    pub unit: DxfUnit,
    /// Drawing extents - minimum point
    pub extents_min: Point,
    /// Drawing extents - maximum point
    pub extents_max: Point,
}

impl Default for DxfHeader {
    fn default() -> Self {
        Self {
            version: "AC1021".to_string(),
            unit: DxfUnit::Millimeters,
            extents_min: Point::new(0.0, 0.0),
            extents_max: Point::new(100.0, 100.0),
        }
    }
}

/// Parsed DXF file
#[derive(Debug, Clone)]
pub struct DxfFile {
    /// Header information
    pub header: DxfHeader,
    /// All entities in file
    pub entities: Vec<DxfEntity>,
    /// Entities organized by layer
    pub layers: HashMap<String, Vec<DxfEntity>>,
}

#[derive(Debug, Clone)]
pub struct DxfEllipse {
    /// Center point
    pub center: Point,
    /// Major axis endpoint (relative to center)
    pub major_axis: Point,
    /// Minor to major ratio
    pub ratio: f64,
    /// Start angle in radians
    pub start_angle: f64,
    /// End angle in radians
    pub end_angle: f64,
    /// Layer name
    pub layer: String,
    /// Color
    pub color: u16,
}

impl DxfFile {
    /// Create new DXF file
    pub fn new() -> Self {
        Self {
            header: DxfHeader::default(),
            entities: Vec::new(),
            layers: HashMap::new(),
        }
    }

    /// Add entity to file
    pub fn add_entity(&mut self, entity: DxfEntity) {
        let layer = entity.layer().to_string();
        self.entities.push(entity.clone());
        self.layers.entry(layer).or_default().push(entity);
    }

    /// Get all entities in a layer
    pub fn get_layer_entities(&self, layer: &str) -> Option<&Vec<DxfEntity>> {
        self.layers.get(layer)
    }

    /// Get layer names
    pub fn layer_names(&self) -> Vec<&str> {
        self.layers.keys().map(|s| s.as_str()).collect()
    }

    /// Get number of entities
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get bounding box of all entities
    pub fn bounds(&self) -> (Point, Point) {
        (self.header.extents_min, self.header.extents_max)
    }

    /// Scale all coordinates by a factor
    pub fn scale(&mut self, factor: f64) {
        for entity in &mut self.entities {
            match entity {
                DxfEntity::Line(l) => {
                    l.start = Point::new(l.start.x * factor, l.start.y * factor);
                    l.end = Point::new(l.end.x * factor, l.end.y * factor);
                }
                DxfEntity::Circle(c) => {
                    c.center = Point::new(c.center.x * factor, c.center.y * factor);
                    c.radius *= factor;
                }
                DxfEntity::Arc(a) => {
                    a.center = Point::new(a.center.x * factor, a.center.y * factor);
                    a.radius *= factor;
                }
                DxfEntity::Polyline(p) => {
                    for vertex in &mut p.vertices {
                        *vertex = Point::new(vertex.x * factor, vertex.y * factor);
                    }
                }
                DxfEntity::Text(t) => {
                    t.position = Point::new(t.position.x * factor, t.position.y * factor);
                    t.height *= factor;
                }
                DxfEntity::Ellipse(e) => {
                    e.center = Point::new(e.center.x * factor, e.center.y * factor);
                    e.major_axis = Point::new(e.major_axis.x * factor, e.major_axis.y * factor);
                }
            }
        }

        for layer_entities in self.layers.values_mut() {
            for entity in layer_entities {
                match entity {
                    DxfEntity::Line(l) => {
                        l.start = Point::new(l.start.x * factor, l.start.y * factor);
                        l.end = Point::new(l.end.x * factor, l.end.y * factor);
                    }
                    DxfEntity::Circle(c) => {
                        c.center = Point::new(c.center.x * factor, c.center.y * factor);
                        c.radius *= factor;
                    }
                    DxfEntity::Arc(a) => {
                        a.center = Point::new(a.center.x * factor, a.center.y * factor);
                        a.radius *= factor;
                    }
                    DxfEntity::Polyline(p) => {
                        for vertex in &mut p.vertices {
                            *vertex = Point::new(vertex.x * factor, vertex.y * factor);
                        }
                    }
                    DxfEntity::Text(t) => {
                        t.position = Point::new(t.position.x * factor, t.position.y * factor);
                        t.height *= factor;
                    }
                    DxfEntity::Ellipse(e) => {
                        e.center = Point::new(e.center.x * factor, e.center.y * factor);
                        e.major_axis = Point::new(e.major_axis.x * factor, e.major_axis.y * factor);
                    }
                }
            }
        }

        self.header.extents_min = Point::new(
            self.header.extents_min.x * factor,
            self.header.extents_min.y * factor,
        );
        self.header.extents_max = Point::new(
            self.header.extents_max.x * factor,
            self.header.extents_max.y * factor,
        );
    }

    /// Apply unit conversion
    pub fn convert_units(&mut self, from_unit: DxfUnit, to_unit: DxfUnit) {
        let conversion_factor = from_unit.to_mm_factor() / to_unit.to_mm_factor();
        self.scale(conversion_factor);
    }
}

impl Default for DxfFile {
    fn default() -> Self {
        Self::new()
    }
}

/// DXF parser for parsing DXF file contents
pub struct DxfParser;

impl DxfParser {
    /// Parse DXF content from a string
    pub fn parse(content: &str) -> Result<DxfFile> {
        let mut file = DxfFile::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut i = 0;
        let mut in_entities = false;

        while i < lines.len() {
            let line = lines[i].trim();

            // Look for ENTITIES section
            if line == "ENTITIES" {
                in_entities = true;
                i += 1;
                continue;
            }

            // Exit entities section
            if in_entities && line == "ENDSEC" {
                in_entities = false;
                i += 1;
                continue;
            }

            // Parse entity data - look for entity type markers after "0" code
            if in_entities && line == "0" {
                i += 1;
                if i < lines.len() {
                    let entity_type = lines[i].trim();
                    match entity_type {
                        "LINE" => {
                            i += 1;
                            if let Ok(line_entity) = Self::parse_line(&lines, &mut i) {
                                file.add_entity(DxfEntity::Line(line_entity));
                            }
                        }
                        "CIRCLE" => {
                            i += 1;
                            if let Ok(circle_entity) = Self::parse_circle(&lines, &mut i) {
                                file.add_entity(DxfEntity::Circle(circle_entity));
                            }
                        }
                        "ARC" => {
                            i += 1;
                            if let Ok(arc_entity) = Self::parse_arc(&lines, &mut i) {
                                file.add_entity(DxfEntity::Arc(arc_entity));
                            }
                        }
                        "LWPOLYLINE" => {
                            i += 1;
                            if let Ok(polyline_entity) = Self::parse_lwpolyline(&lines, &mut i) {
                                file.add_entity(DxfEntity::Polyline(polyline_entity));
                            }
                        }
                        "POLYLINE" => {
                            i += 1;
                            if let Ok(polyline_entity) = Self::parse_polyline(&lines, &mut i) {
                                file.add_entity(DxfEntity::Polyline(polyline_entity));
                            }
                        }
                        "TEXT" => {
                            i += 1;
                            if let Ok(text_entity) = Self::parse_text(&lines, &mut i) {
                                file.add_entity(DxfEntity::Text(text_entity));
                            }
                        }
                        "ELLIPSE" => {
                            i += 1;
                            if let Ok(ellipse_entity) = Self::parse_ellipse(&lines, &mut i) {
                                file.add_entity(DxfEntity::Ellipse(ellipse_entity));
                            }
                        }
                        _ => i += 1,
                    }
                    continue;
                }
            }

            i += 1;
        }

        Ok(file)
    }

    /// Parse a LINE entity
    fn parse_line(lines: &[&str], index: &mut usize) -> Result<DxfLine> {
        let mut start = Point::new(0.0, 0.0);
        let mut end = Point::new(0.0, 0.0);
        let mut layer = "0".to_string();
        let mut color = 256u16;

        while *index < lines.len() {
            let code = lines[*index].trim();
            *index += 1;

            if *index >= lines.len() {
                break;
            }

            let value = lines[*index].trim();

            if code == "0" && !value.is_empty() {
                *index -= 1;
                break;
            }

            match code {
                "8" => layer = value.to_string(),
                "62" => color = value.parse().unwrap_or(256),
                "10" => start.x = value.parse().unwrap_or(0.0),
                "20" => start.y = value.parse().unwrap_or(0.0),
                "11" => end.x = value.parse().unwrap_or(0.0),
                "21" => end.y = value.parse().unwrap_or(0.0),
                _ => {}
            }

            *index += 1;
        }

        Ok(DxfLine {
            start,
            end,
            layer,
            color,
        })
    }

    /// Parse a CIRCLE entity
    fn parse_circle(lines: &[&str], index: &mut usize) -> Result<DxfCircle> {
        let mut center = Point::new(0.0, 0.0);
        let mut radius = 0.0;
        let mut layer = "0".to_string();
        let mut color = 256u16;

        while *index < lines.len() {
            let code = lines[*index].trim();
            *index += 1;

            if *index >= lines.len() {
                break;
            }

            let value = lines[*index].trim();

            if code == "0" && !value.is_empty() {
                *index -= 1;
                break;
            }

            match code {
                "8" => layer = value.to_string(),
                "62" => color = value.parse().unwrap_or(256),
                "10" => center.x = value.parse().unwrap_or(0.0),
                "20" => center.y = value.parse().unwrap_or(0.0),
                "40" => radius = value.parse().unwrap_or(0.0),
                _ => {}
            }

            *index += 1;
        }

        Ok(DxfCircle {
            center,
            radius,
            layer,
            color,
        })
    }

    /// Parse an ARC entity
    fn parse_arc(lines: &[&str], index: &mut usize) -> Result<DxfArc> {
        let mut center = Point::new(0.0, 0.0);
        let mut radius = 0.0;
        let mut start_angle = 0.0;
        let mut end_angle = 0.0;
        let mut layer = "0".to_string();
        let mut color = 256u16;

        while *index < lines.len() {
            let code = lines[*index].trim();
            *index += 1;

            if *index >= lines.len() {
                break;
            }

            let value = lines[*index].trim();

            if code == "0" && !value.is_empty() {
                *index -= 1;
                break;
            }

            match code {
                "8" => layer = value.to_string(),
                "62" => color = value.parse().unwrap_or(256),
                "10" => center.x = value.parse().unwrap_or(0.0),
                "20" => center.y = value.parse().unwrap_or(0.0),
                "40" => radius = value.parse().unwrap_or(0.0),
                "50" => start_angle = value.parse().unwrap_or(0.0),
                "51" => end_angle = value.parse().unwrap_or(0.0),
                _ => {}
            }

            *index += 1;
        }

        Ok(DxfArc {
            center,
            radius,
            start_angle,
            end_angle,
            layer,
            color,
        })
    }

    fn parse_lwpolyline(lines: &[&str], index: &mut usize) -> Result<DxfPolyline> {
        let mut vertices = Vec::new();
        let mut bulges = Vec::new();
        let mut closed = false;
        let mut layer = "0".to_string();
        let mut color = 256u16;
        let mut current_x: Option<f64> = None;

        while *index < lines.len() {
            let code = lines[*index].trim();
            let value = lines.get(*index + 1).map(|v| v.trim()).unwrap_or("");

            if code == "0" {
                break;
            } // End entity

            match code {
                "8" => layer = value.to_string(),
                "62" => color = value.parse().unwrap_or(256),
                "70" => {
                    let flags = value.parse::<i32>().unwrap_or(0);
                    closed = (flags & 1) != 0;
                }
                "10" => current_x = value.parse().ok(),
                "20" => {
                    if let Some(x) = current_x {
                        let y = value.parse().unwrap_or(0.0);
                        vertices.push(Point::new(x, y));

                        bulges.push(0.0);
                        current_x = None;
                    }
                }
                "42" => {
                    if let Ok(bulge_val) = value.parse::<f64>() {
                        if let Some(last_bulge) = bulges.last_mut() {
                            *last_bulge = bulge_val;
                        }
                    }
                }
                "90" => { /* Optional: You could use it to do Vec::with_capacity(n) */ }
                _ => {}
            }
            *index += 2;
        }

        Ok(DxfPolyline {
            vertices,
            bulges,
            closed,
            layer,
            color,
        })
    }

    /// Parse a POLYLINE entity
    fn parse_polyline(lines: &[&str], index: &mut usize) -> Result<DxfPolyline> {
        let mut vertices = Vec::new();
        let mut closed = false;
        let mut layer = "0".to_string();
        let mut color = 256u16;

        // Parse POLYLINE header
        while *index < lines.len() {
            let code = lines[*index].trim();
            *index += 1;

            if *index >= lines.len() {
                break;
            }

            let value = lines[*index].trim();

            if code == "0" {
                *index -= 1; // Backtrack to handle VERTEX or SEQEND
                break;
            }

            match code {
                "8" => layer = value.to_string(),
                "62" => color = value.parse().unwrap_or(256),
                "70" => {
                    if let Ok(flags) = value.parse::<i32>() {
                        closed = (flags & 1) != 0;
                    }
                }
                // Ignore 10, 20, 30 in POLYLINE header
                _ => {}
            }

            *index += 1;
        }

        // Parse VERTEX entities
        loop {
            if *index >= lines.len() {
                break;
            }

            let code = lines[*index].trim();
            *index += 1;

            if *index >= lines.len() {
                break;
            }

            let value = lines[*index].trim();
            *index += 1;

            if code != "0" {
                continue;
            }

            if value == "SEQEND" {
                break;
            }

            if value == "VERTEX" {
                let mut current_x: Option<f64> = None;

                // Parse VERTEX body
                while *index < lines.len() {
                    let v_code = lines[*index].trim();

                    if v_code == "0" {
                        break; // End of VERTEX
                    }

                    *index += 1;
                    if *index >= lines.len() {
                        break;
                    }

                    let v_value = lines[*index].trim();
                    *index += 1;

                    match v_code {
                        "10" => current_x = v_value.parse().ok(),
                        "20" => {
                            if let Some(x) = current_x {
                                let y = v_value.parse().unwrap_or(0.0);
                                vertices.push(Point::new(x, y));
                                current_x = None;
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                // Found unexpected entity type inside POLYLINE sequence
                // This shouldn't happen in valid DXF, but if it does, we should probably stop
                // and let the main loop handle it.
                // Backtrack and break
                *index -= 2;
                break;
            }
        }

        Ok(DxfPolyline {
            vertices,
            bulges: vec![],
            closed,
            layer,
            color,
        })
    }

    /// Parse a TEXT entity
    fn parse_text(lines: &[&str], index: &mut usize) -> Result<DxfText> {
        let mut content = String::new();
        let mut position = Point::new(0.0, 0.0);
        let mut height = 2.5;
        let mut rotation = 0.0;
        let mut layer = "0".to_string();
        let mut color = 256u16;

        while *index < lines.len() {
            let code = lines[*index].trim();
            *index += 1;

            if *index >= lines.len() {
                break;
            }

            let value = lines[*index].trim();

            if code == "0" && !value.is_empty() {
                *index -= 1;
                break;
            }

            match code {
                "1" => content = value.to_string(),
                "8" => layer = value.to_string(),
                "62" => color = value.parse().unwrap_or(256),
                "10" => position.x = value.parse().unwrap_or(0.0),
                "20" => position.y = value.parse().unwrap_or(0.0),
                "40" => height = value.parse().unwrap_or(2.5),
                "50" => rotation = value.parse().unwrap_or(0.0),
                _ => {}
            }

            *index += 1;
        }

        Ok(DxfText {
            content,
            position,
            height,
            rotation,
            layer,
            color,
        })
    }

    fn parse_ellipse(lines: &[&str], index: &mut usize) -> Result<DxfEllipse> {
        let mut center = Point::new(0.0, 0.0);
        let mut major_axis = Point::new(0.0, 0.0);
        let mut ratio = 0.0;
        let mut start_angle = 0.0;
        let mut end_angle = 2.0 * std::f64::consts::PI;
        let mut layer = "0".to_string();
        let mut color = 256u16;

        while *index < lines.len() {
            let code = lines[*index].trim();
            *index += 1;

            if *index >= lines.len() {
                break;
            }

            let value = lines[*index].trim();

            if code == "0" && !value.is_empty() {
                *index -= 1;
                break;
            }

            match code {
                "8" => layer = value.to_string(),
                "62" => color = value.parse().unwrap_or(256),
                "10" => center.x = value.parse().unwrap_or(0.0),
                "20" => center.y = value.parse().unwrap_or(0.0),
                "11" => major_axis.x = value.parse().unwrap_or(0.0),
                "21" => major_axis.y = value.parse().unwrap_or(0.0),
                "40" => ratio = value.parse().unwrap_or(0.0),
                "41" => start_angle = value.parse().unwrap_or(0.0),
                "42" => end_angle = value.parse().unwrap_or(2.0 * std::f64::consts::PI),
                _ => {}
            }

            *index += 1;
        }

        Ok(DxfEllipse {
            center,
            major_axis,
            ratio,
            start_angle,
            end_angle,
            layer,
            color,
        })
    }

    /// Validate DXF file format
    pub fn validate_header(content: &str) -> Result<()> {
        if !content.contains("SECTION") || !content.contains("ENDSEC") {
            return Err(anyhow::anyhow!("Invalid DXF file format"));
        }
        Ok(())
    }
}
