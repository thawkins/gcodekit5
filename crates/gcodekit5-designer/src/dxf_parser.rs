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
    /// Miles
    Miles,
    /// Microinches
    Microinches,
    /// Mils (thousandths of an inch)
    Mils,
    /// Yards
    Yards,
    /// Decimeters
    Decimeters,
    /// Micrometers (microns)
    Micrometers,
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
            DxfUnit::Miles => 1_609_344.0,
            DxfUnit::Microinches => 25.4e-6,
            DxfUnit::Mils => 0.0254,
            DxfUnit::Yards => 914.4,
            DxfUnit::Decimeters => 100.0,
            DxfUnit::Micrometers => 0.001,
        }
    }

    /// Maps a `$INSUNITS` header value to a unit.
    ///
    /// # Arguments
    /// * `code` - The `$INSUNITS` value (0 = unitless, 1 = inches, 2 = feet,
    ///   3 = miles, 4 = mm, 5 = cm, 6 = m, 7 = km, 8 = microinches, 9 = mils,
    ///   10 = yards, 13 = micrometers, 14 = decimeters).
    ///
    /// # Returns
    /// The unit, or `None` for codes with no supported length conversion
    /// (e.g. astronomical units, light years, parsecs).
    pub fn from_insunits(code: i32) -> Option<Self> {
        Some(match code {
            0 => DxfUnit::Unitless,
            1 => DxfUnit::Inches,
            2 => DxfUnit::Feet,
            3 => DxfUnit::Miles,
            4 => DxfUnit::Millimeters,
            5 => DxfUnit::Centimeters,
            6 => DxfUnit::Meters,
            7 => DxfUnit::Kilometers,
            8 => DxfUnit::Microinches,
            9 => DxfUnit::Mils,
            10 => DxfUnit::Yards,
            13 => DxfUnit::Micrometers,
            14 => DxfUnit::Decimeters,
            _ => return None,
        })
    }
}

/// Returns true when an entity's extrusion direction (group code 230, the Z of
/// its OCS normal) points down the world Z axis. By the DXF arbitrary-axis
/// algorithm an OCS with normal (0, 0, -1) has its X axis along world -X and its
/// Y axis along world +Y, so OCS geometry is mirrored in X.
fn extrusion_is_negative_z(extrusion_z: f64) -> bool {
    extrusion_z < 0.0
}

/// Mirrors an arc's start/end angles (degrees) for a negative-Z extrusion: the
/// OCS point at angle `a` lies at world angle `180 - a`, and the sweep direction
/// reverses, so the world arc runs counter-clockwise from `180 - end` to
/// `180 - start`.
fn mirror_arc_angles(start: f64, end: f64) -> (f64, f64) {
    let wrap = |a: f64| a.rem_euclid(360.0);
    (wrap(180.0 - end), wrap(180.0 - start))
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
    /// Z elevation of the entity's plane (group code 30)
    pub elevation: f64,
    /// Extrusion thickness along Z (group code 39)
    pub thickness: f64,
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
    /// Z elevation of the entity's plane (group code 30)
    pub elevation: f64,
    /// Extrusion thickness along Z (group code 39)
    pub thickness: f64,
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
    /// Z elevation of the entity's plane (group code 30)
    pub elevation: f64,
    /// Extrusion thickness along Z (group code 39)
    pub thickness: f64,
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
    /// Z elevation of the entity's plane (group code 38 for LWPOLYLINE, or first vertex Z for POLYLINE)
    pub elevation: f64,
    /// Extrusion thickness along Z (group code 39)
    pub thickness: f64,
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
    /// Unsupported entity types encountered (e.g. ACIS solids), mapped to occurrence count
    pub unsupported_entities: HashMap<String, usize>,
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
    /// Z elevation of the entity's plane (group code 30)
    pub elevation: f64,
    /// Extrusion thickness along Z (group code 39)
    pub thickness: f64,
}

impl DxfFile {
    /// Create new DXF file
    pub fn new() -> Self {
        Self {
            header: DxfHeader::default(),
            entities: Vec::new(),
            layers: HashMap::new(),
            unsupported_entities: HashMap::new(),
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
                    l.elevation *= factor;
                    l.thickness *= factor;
                }
                DxfEntity::Circle(c) => {
                    c.center = Point::new(c.center.x * factor, c.center.y * factor);
                    c.radius *= factor;
                    c.elevation *= factor;
                    c.thickness *= factor;
                }
                DxfEntity::Arc(a) => {
                    a.center = Point::new(a.center.x * factor, a.center.y * factor);
                    a.radius *= factor;
                    a.elevation *= factor;
                    a.thickness *= factor;
                }
                DxfEntity::Polyline(p) => {
                    for vertex in &mut p.vertices {
                        *vertex = Point::new(vertex.x * factor, vertex.y * factor);
                    }
                    p.elevation *= factor;
                    p.thickness *= factor;
                }
                DxfEntity::Text(t) => {
                    t.position = Point::new(t.position.x * factor, t.position.y * factor);
                    t.height *= factor;
                }
                DxfEntity::Ellipse(e) => {
                    e.center = Point::new(e.center.x * factor, e.center.y * factor);
                    e.major_axis = Point::new(e.major_axis.x * factor, e.major_axis.y * factor);
                    e.elevation *= factor;
                    e.thickness *= factor;
                }
            }
        }

        for layer_entities in self.layers.values_mut() {
            for entity in layer_entities {
                match entity {
                    DxfEntity::Line(l) => {
                        l.start = Point::new(l.start.x * factor, l.start.y * factor);
                        l.end = Point::new(l.end.x * factor, l.end.y * factor);
                        l.elevation *= factor;
                        l.thickness *= factor;
                    }
                    DxfEntity::Circle(c) => {
                        c.center = Point::new(c.center.x * factor, c.center.y * factor);
                        c.radius *= factor;
                        c.elevation *= factor;
                        c.thickness *= factor;
                    }
                    DxfEntity::Arc(a) => {
                        a.center = Point::new(a.center.x * factor, a.center.y * factor);
                        a.radius *= factor;
                        a.elevation *= factor;
                        a.thickness *= factor;
                    }
                    DxfEntity::Polyline(p) => {
                        for vertex in &mut p.vertices {
                            *vertex = Point::new(vertex.x * factor, vertex.y * factor);
                        }
                        p.elevation *= factor;
                        p.thickness *= factor;
                    }
                    DxfEntity::Text(t) => {
                        t.position = Point::new(t.position.x * factor, t.position.y * factor);
                        t.height *= factor;
                    }
                    DxfEntity::Ellipse(e) => {
                        e.center = Point::new(e.center.x * factor, e.center.y * factor);
                        e.major_axis = Point::new(e.major_axis.x * factor, e.major_axis.y * factor);
                        e.elevation *= factor;
                        e.thickness *= factor;
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

thread_local! {
    /// Values in the file being parsed that were not valid numbers.
    static BAD_NUMBERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// A number type read from a DXF group value.
trait DxfNumber: std::str::FromStr {
    /// Whether a parsed value is usable (floats must be finite).
    fn is_usable(&self) -> bool {
        true
    }
}
impl DxfNumber for f64 {
    fn is_usable(&self) -> bool {
        self.is_finite()
    }
}
impl DxfNumber for i16 {}
impl DxfNumber for i32 {}
impl DxfNumber for i64 {}
impl DxfNumber for u8 {}
impl DxfNumber for u16 {}
impl DxfNumber for u32 {}
impl DxfNumber for usize {}

/// Parses a DXF number, counting a malformed or non-finite value (which is
/// replaced by `default`) so the import can warn about it instead of
/// silently moving a vertex to 0.
fn parse_or<T: DxfNumber>(value: &str, default: T) -> T {
    parse_opt(value).unwrap_or(default)
}

/// Parses a DXF number, counting a malformed or non-finite value as `None`.
fn parse_opt<T: DxfNumber>(value: &str) -> Option<T> {
    match value.trim().parse::<T>() {
        Ok(v) if v.is_usable() => Some(v),
        _ => {
            BAD_NUMBERS.with(|n| n.set(n.get() + 1));
            None
        }
    }
}

/// DXF parser for parsing DXF file contents
pub struct DxfParser;

impl DxfParser {
    /// Parse DXF content from a string
    pub fn parse(content: &str) -> Result<DxfFile> {
        BAD_NUMBERS.with(|n| n.set(0));
        let mut file = DxfFile::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut i = 0;
        let mut in_entities = false;

        while i < lines.len() {
            let line = lines[i].trim();

            // Drawing units from the HEADER section: `9 / $INSUNITS / 70 / <code>`.
            if !in_entities
                && line == "$INSUNITS"
                && lines.get(i + 1).map(|c| c.trim()) == Some("70")
            {
                let code = lines
                    .get(i + 2)
                    .and_then(|v| v.trim().parse::<i32>().ok())
                    .unwrap_or(0);
                match DxfUnit::from_insunits(code) {
                    Some(unit) => file.header.unit = unit,
                    None => tracing::warn!(
                        insunits = code,
                        "Unsupported DXF $INSUNITS value; assuming millimetres"
                    ),
                }
                i += 3;
                continue;
            }

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
                        "SPLINE" => {
                            i += 1;
                            if let Ok(spline_entity) = Self::parse_spline(&lines, &mut i) {
                                if !spline_entity.vertices.is_empty() {
                                    file.add_entity(DxfEntity::Polyline(spline_entity));
                                }
                            }
                        }
                        // ACIS-based solids (3DSOLID/BODY/REGION) are not parseable without a CAD kernel; track and discard.
                        "3DSOLID" | "BODY" | "REGION" => {
                            *file
                                .unsupported_entities
                                .entry(entity_type.to_string())
                                .or_insert(0) += 1;
                            i += 1;
                        }
                        _ => i += 1,
                    }
                    continue;
                }
            }

            i += 1;
        }

        // Values that were not numbers were replaced by defaults; report them
        // with the unsupported entities so the import shows a warning.
        let bad = BAD_NUMBERS.with(std::cell::Cell::get);
        if bad > 0 {
            tracing::warn!(count = bad, "DXF values that are not numbers were replaced");
            file.unsupported_entities
                .insert("values that are not numbers".to_string(), bad);
        }
        Ok(file)
    }

    /// Parse a LINE entity
    fn parse_line(lines: &[&str], index: &mut usize) -> Result<DxfLine> {
        let mut start = Point::new(0.0, 0.0);
        let mut end = Point::new(0.0, 0.0);
        let mut layer = "0".to_string();
        let mut color = 256u16;
        let mut elevation = 0.0;
        let mut thickness = 0.0;

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
                "62" => color = parse_or(value, 256),
                "10" => start.x = parse_or(value, 0.0),
                "20" => start.y = parse_or(value, 0.0),
                "11" => end.x = parse_or(value, 0.0),
                "21" => end.y = parse_or(value, 0.0),
                "30" => elevation = parse_or(value, 0.0),
                "39" => thickness = parse_or(value, 0.0),
                _ => {}
            }

            *index += 1;
        }

        Ok(DxfLine {
            start,
            end,
            layer,
            color,
            elevation,
            thickness,
        })
    }

    /// Parse a CIRCLE entity
    fn parse_circle(lines: &[&str], index: &mut usize) -> Result<DxfCircle> {
        let mut center = Point::new(0.0, 0.0);
        let mut radius = 0.0;
        let mut layer = "0".to_string();
        let mut color = 256u16;
        let mut elevation = 0.0;
        let mut thickness = 0.0;
        let mut extrusion_z = 1.0;

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
                "62" => color = parse_or(value, 256),
                "10" => center.x = parse_or(value, 0.0),
                "20" => center.y = parse_or(value, 0.0),
                "40" => radius = parse_or(value, 0.0),
                "30" => elevation = parse_or(value, 0.0),
                "39" => thickness = parse_or(value, 0.0),
                "230" => extrusion_z = parse_or(value, 1.0),
                _ => {}
            }

            *index += 1;
        }

        if extrusion_is_negative_z(extrusion_z) {
            center.x = -center.x;
            elevation = -elevation;
        }

        Ok(DxfCircle {
            center,
            radius,
            layer,
            color,
            elevation,
            thickness,
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
        let mut elevation = 0.0;
        let mut thickness = 0.0;
        let mut extrusion_z = 1.0;

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
                "62" => color = parse_or(value, 256),
                "10" => center.x = parse_or(value, 0.0),
                "20" => center.y = parse_or(value, 0.0),
                "40" => radius = parse_or(value, 0.0),
                "50" => start_angle = parse_or(value, 0.0),
                "51" => end_angle = parse_or(value, 0.0),
                "30" => elevation = parse_or(value, 0.0),
                "39" => thickness = parse_or(value, 0.0),
                "230" => extrusion_z = parse_or(value, 1.0),
                _ => {}
            }

            *index += 1;
        }

        if extrusion_is_negative_z(extrusion_z) {
            center.x = -center.x;
            elevation = -elevation;
            (start_angle, end_angle) = mirror_arc_angles(start_angle, end_angle);
        }

        Ok(DxfArc {
            center,
            radius,
            start_angle,
            end_angle,
            layer,
            color,
            elevation,
            thickness,
        })
    }

    fn parse_lwpolyline(lines: &[&str], index: &mut usize) -> Result<DxfPolyline> {
        let mut vertices = Vec::new();
        let mut bulges = Vec::new();
        let mut closed = false;
        let mut layer = "0".to_string();
        let mut color = 256u16;
        let mut current_x: Option<f64> = None;
        let mut elevation = 0.0;
        let mut thickness = 0.0;
        let mut extrusion_z = 1.0;

        while *index < lines.len() {
            let code = lines[*index].trim();
            let value = lines.get(*index + 1).map(|v| v.trim()).unwrap_or("");

            if code == "0" {
                break;
            } // End entity

            match code {
                "8" => layer = value.to_string(),
                "62" => color = parse_or(value, 256),
                "70" => {
                    let flags = parse_or::<i32>(value, 0);
                    closed = (flags & 1) != 0;
                }
                "10" => current_x = parse_opt(value),
                "20" => {
                    if let Some(x) = current_x {
                        let y = parse_or(value, 0.0);
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
                "38" => elevation = parse_or(value, 0.0),
                "39" => thickness = parse_or(value, 0.0),
                "230" => extrusion_z = parse_or(value, 1.0),
                "90" => { /* Optional: You could use it to do Vec::with_capacity(n) */ }
                _ => {}
            }
            *index += 2;
        }

        if extrusion_is_negative_z(extrusion_z) {
            // Mirroring reverses the turning direction of every bulge arc.
            for v in &mut vertices {
                v.x = -v.x;
            }
            for b in &mut bulges {
                *b = -*b;
            }
            elevation = -elevation;
        }

        Ok(DxfPolyline {
            vertices,
            bulges,
            closed,
            layer,
            color,
            elevation,
            thickness,
        })
    }

    /// Parse a POLYLINE entity
    fn parse_polyline(lines: &[&str], index: &mut usize) -> Result<DxfPolyline> {
        let mut raw_vertices: Vec<(Point, i32)> = Vec::new();
        let mut closed = false;
        let mut layer = "0".to_string();
        let mut color = 256u16;
        let mut thickness = 0.0;
        let mut elevation: Option<f64> = None;
        let mut header_flags: i32 = 0;
        let mut extrusion_z = 1.0;

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
                "62" => color = parse_or(value, 256),
                "70" => {
                    header_flags = parse_or(value, 0);
                    closed = (header_flags & 1) != 0;
                }
                "39" => thickness = parse_or(value, 0.0),
                "230" => extrusion_z = parse_or(value, 1.0),
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
                let mut vx: Option<f64> = None;
                let mut vy: Option<f64> = None;
                let mut vertex_flags: i32 = 0;

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
                        "10" => vx = parse_opt(v_value),
                        "20" => vy = parse_opt(v_value),
                        // The plane elevation is taken from the first vertex's Z; true
                        // per-vertex 3D polylines are not yet supported (see 3DFACE/MESH).
                        "30" => {
                            if elevation.is_none() {
                                elevation = parse_opt(v_value);
                            }
                        }
                        "70" => vertex_flags = parse_or(v_value, 0),
                        _ => {}
                    }
                }

                if let (Some(x), Some(y)) = (vx, vy) {
                    raw_vertices.push((Point::new(x, y), vertex_flags));
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

        // Spline-fit polylines (header flag bit 2) interleave the original control-frame
        // vertices (vertex flag bit 16) with the generated curve vertices (vertex flag bit
        // 8, "AcDbSplineVertex"). Rendering the control frame produces zig-zag artifacts,
        // so only the curve-fit vertices are kept for these polylines.
        let is_spline_fit = header_flags & 4 != 0;
        let mut vertices: Vec<Point> = if is_spline_fit {
            let curve_vertices: Vec<Point> = raw_vertices
                .iter()
                .filter(|(_, flags)| flags & 8 != 0)
                .map(|(p, _)| *p)
                .collect();
            if curve_vertices.is_empty() {
                raw_vertices.into_iter().map(|(p, _)| p).collect()
            } else {
                curve_vertices
            }
        } else {
            raw_vertices.into_iter().map(|(p, _)| p).collect()
        };

        // 3D polylines (flag 8) and polyface meshes (flag 64) use world coordinates;
        // 2D polylines are in the OCS.
        let is_ocs = header_flags & (8 | 16 | 64) == 0;
        let mut elevation = elevation.unwrap_or(0.0);
        if is_ocs && extrusion_is_negative_z(extrusion_z) {
            for v in &mut vertices {
                v.x = -v.x;
            }
            elevation = -elevation;
        }

        Ok(DxfPolyline {
            vertices,
            bulges: vec![],
            closed,
            layer,
            color,
            elevation,
            thickness,
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
        let mut extrusion_z = 1.0;

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
                "62" => color = parse_or(value, 256),
                "10" => position.x = parse_or(value, 0.0),
                "20" => position.y = parse_or(value, 0.0),
                "40" => height = parse_or(value, 2.5),
                "50" => rotation = parse_or(value, 0.0),
                "230" => extrusion_z = parse_or(value, 1.0),
                _ => {}
            }

            *index += 1;
        }

        if extrusion_is_negative_z(extrusion_z) {
            position.x = -position.x;
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
        let mut elevation = 0.0;
        let mut thickness = 0.0;
        let mut extrusion_z = 1.0;

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
                "62" => color = parse_or(value, 256),
                "10" => center.x = parse_or(value, 0.0),
                "20" => center.y = parse_or(value, 0.0),
                "11" => major_axis.x = parse_or(value, 0.0),
                "21" => major_axis.y = parse_or(value, 0.0),
                "40" => ratio = parse_or(value, 0.0),
                "41" => start_angle = parse_or(value, 0.0),
                "42" => end_angle = parse_or(value, 2.0 * std::f64::consts::PI),
                "30" => elevation = parse_or(value, 0.0),
                "39" => thickness = parse_or(value, 0.0),
                "230" => extrusion_z = parse_or(value, 1.0),
                _ => {}
            }

            *index += 1;
        }

        // ELLIPSE centre and major axis are in world coordinates, but the minor axis is
        // normal x major, so with a -Z normal the curve runs clockwise: the point at
        // parameter t equals the +Z-normal point at -t.
        if extrusion_is_negative_z(extrusion_z) {
            (start_angle, end_angle) = (-end_angle, -start_angle);
        }

        Ok(DxfEllipse {
            center,
            major_axis,
            ratio,
            start_angle,
            end_angle,
            layer,
            color,
            elevation,
            thickness,
        })
    }

    /// Parse a SPLINE entity, approximating the NURBS curve as a polyline.
    ///
    /// AutoCAD 2000+ exports (AC1015 and later, e.g. the "2010"/"2018" DXF
    /// versions) commonly use SPLINE for organic/curved geometry instead of
    /// flattening it to POLYLINE segments the way older R12 (AC1009) exports
    /// do. Without this, all spline geometry was silently dropped on import.
    fn parse_spline(lines: &[&str], index: &mut usize) -> Result<DxfPolyline> {
        let mut layer = "0".to_string();
        let mut color = 256u16;
        let mut flags: i32 = 0;
        let mut degree: usize = 3;
        let mut knots: Vec<f64> = Vec::new();
        let mut weights: Vec<f64> = Vec::new();
        let mut control_points: Vec<Point> = Vec::new();
        let mut fit_points: Vec<Point> = Vec::new();
        let mut pending_ctrl_x: Option<f64> = None;
        let mut pending_fit_x: Option<f64> = None;

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
                "62" => color = parse_or(value, 256),
                "70" => flags = parse_or(value, 0),
                "71" => degree = parse_or::<i64>(value, 3).max(1) as usize,
                "40" => knots.push(parse_or(value, 0.0)),
                "41" => weights.push(parse_or(value, 1.0)),
                "10" => pending_ctrl_x = parse_opt(value),
                "20" => {
                    if let Some(x) = pending_ctrl_x.take() {
                        control_points.push(Point::new(x, parse_or(value, 0.0)));
                    }
                }
                "11" => pending_fit_x = parse_opt(value),
                "21" => {
                    if let Some(x) = pending_fit_x.take() {
                        fit_points.push(Point::new(x, parse_or(value, 0.0)));
                    }
                }
                _ => {}
            }

            *index += 1;
        }

        let closed = flags & 1 != 0;
        let vertices = Self::sample_spline(&control_points, &knots, &weights, degree)
            .filter(|v| v.len() >= 2)
            .unwrap_or(fit_points);
        let vertex_count = vertices.len();

        Ok(DxfPolyline {
            vertices,
            bulges: vec![0.0; vertex_count],
            closed,
            layer,
            color,
            elevation: 0.0,
            thickness: 0.0,
        })
    }

    /// Sample a (possibly rational) B-spline curve into a polyline approximation.
    ///
    /// Returns `None` when the control point / knot vector data is insufficient
    /// or malformed, so the caller can fall back to fit points instead of
    /// producing a corrupt/empty curve.
    fn sample_spline(
        control_points: &[Point],
        knots: &[f64],
        weights: &[f64],
        degree: usize,
    ) -> Option<Vec<Point>> {
        let num_ctrl = control_points.len();
        if degree == 0 || num_ctrl < degree + 1 || knots.len() < num_ctrl + degree + 1 {
            return None;
        }

        let t_min = *knots.get(degree)?;
        let t_max = *knots.get(num_ctrl)?;
        if t_max.partial_cmp(&t_min) != Some(std::cmp::Ordering::Greater) {
            return None;
        }

        // Scale sample density so chord segments are ~1mm long. The control
        // polygon length is an upper bound on the actual curve length, so it
        // gives a safe (never too coarse) sample count estimate.
        const TARGET_SEGMENT_LEN: f64 = 1.0;
        let control_length: f64 = control_points
            .windows(2)
            .map(|w| w[0].distance_to(&w[1]))
            .sum();
        let length_based = (control_length / TARGET_SEGMENT_LEN).ceil() as usize;
        let samples = length_based.max(num_ctrl * 8).clamp(32, 2000);
        let mut points = Vec::with_capacity(samples + 1);

        for step in 0..=samples {
            let t = t_min + (t_max - t_min) * (step as f64 / samples as f64);
            if let Some(p) = Self::bspline_point(t, degree, control_points, weights, knots) {
                points.push(p);
            }
        }

        Some(points)
    }

    /// Evaluate a rational B-spline curve at parameter `t` using the rational
    /// de Boor algorithm (homogeneous coordinates), defaulting weights to 1.0
    /// for non-rational splines.
    fn bspline_point(
        t: f64,
        degree: usize,
        control_points: &[Point],
        weights: &[f64],
        knots: &[f64],
    ) -> Option<Point> {
        let n = control_points.len().checked_sub(1)?;
        let p = degree;

        // Find the knot span k such that knots[k] <= t < knots[k+1], clamped to [p, n].
        let mut k = p;
        for i in p..=n {
            if *knots.get(i)? <= t {
                k = i;
            } else {
                break;
            }
        }
        k = k.min(n);

        // Homogeneous coordinates (x*w, y*w, w) so rational weights fall out
        // naturally after the recurrence.
        let mut d: Vec<(f64, f64, f64)> = Vec::with_capacity(p + 1);
        for j in 0..=p {
            let idx = k.checked_sub(p)?.checked_add(j)?;
            let cp = *control_points.get(idx)?;
            let w = weights.get(idx).copied().unwrap_or(1.0);
            d.push((cp.x * w, cp.y * w, w));
        }

        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = k.checked_sub(p)?.checked_add(j)?;
                let left = *knots.get(i)?;
                let right = *knots.get(i + p - r + 1)?;
                let denom = right - left;
                let alpha = if denom.abs() < 1e-12 {
                    0.0
                } else {
                    (t - left) / denom
                };
                d[j].0 = (1.0 - alpha) * d[j - 1].0 + alpha * d[j].0;
                d[j].1 = (1.0 - alpha) * d[j - 1].1 + alpha * d[j].1;
                d[j].2 = (1.0 - alpha) * d[j - 1].2 + alpha * d[j].2;
            }
        }

        let (x, y, w) = d[p];
        if w.abs() > 1e-9 {
            Some(Point::new(x / w, y / w))
        } else {
            None
        }
    }

    /// Validate DXF file format
    pub fn validate_header(content: &str) -> Result<()> {
        if !content.contains("SECTION") || !content.contains("ENDSEC") {
            return Err(anyhow::anyhow!("Invalid DXF file format"));
        }
        Ok(())
    }
}
