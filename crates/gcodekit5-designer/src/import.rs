//! # File Import Module
//!
//! Provides functionality to import design files (SVG, DXF) into Designer shapes.
//!
//! This module provides importers for converting external file formats into Designer shapes.
//! Includes full SVG path parsing and DXF entity conversion.
//!
//! Supports:
//! - File format detection and validation
//! - SVG path parsing (lines, circles, rectangles, ellipses, paths)
//! - DXF entity conversion (lines, circles, arcs, polylines)
//! - Coordinate system transformation
//! - Scale and offset adjustment

use crate::dxf_parser::{DxfEntity, DxfFile, DxfParser};
use crate::model::{
    DesignCircle as Circle, DesignEllipse as Ellipse, DesignLine as Line, DesignPath as PathShape,  DesignRectangle as Rectangle, DesignerShape, Point, Shape,
};
use crate::model3d::{Mesh3D, Model3DImporter};
use anyhow::{anyhow, Result};
use lyon::math::{point, vector};
use lyon::path::Path;


/// Represents an imported design from a file
#[derive(Debug)]
pub struct ImportedDesign {
    /// Imported shapes as trait objects
    pub shapes: Vec<Shape>,
    /// Original file dimensions (width, height)
    pub dimensions: (f64, f64),
    /// Source file format
    pub format: FileFormat,
    /// Number of layers imported
    pub layer_count: usize,
    /// Optional 3D mesh for 3D models
    pub mesh_3d: Option<Mesh3D>,
}

/// Supported import file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// SVG (Scalable Vector Graphics)
    Svg,
    /// DXF (Drawing Exchange Format)
    Dxf,
    /// STL (STereoLithography) - 3D model format
    Stl,
}

/// SVG importer for converting SVG files to Designer shapes
///
/// Currently provides basic framework for SVG import.
/// Full implementation requires SVG parsing library integration.
pub struct SvgImporter {
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

enum ImportedShape {
    Rect(Rectangle),
    Circle(Circle),
    Line(Line),
    Ellipse(Ellipse),
    Path(PathShape),
}

impl ImportedShape {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        match self {
            Self::Rect(s) => s.bounds(),
            Self::Circle(s) => s.bounds(),
            Self::Line(s) => s.bounds(),
            Self::Ellipse(s) => s.bounds(),
            Self::Path(s) => s.bounds(),
        }
    }

    fn convert(self, center_y: f64, offset_x: f64, offset_y: f64) -> Shape {
        match self {
            Self::Rect(r) => {
                // y' = -y + 2c
                // New min_y is -(old_max_y) + 2c = -(r.y + r.height) + 2c
                let new_y = -(r.center.y + r.height / 2.0) + 2.0 * center_y + offset_y;
                let new_x = r.center.x - r.width / 2.0 + offset_x;
                Shape::Rectangle(Rectangle::new(new_x, new_y, r.width, r.height))
            }
            Self::Circle(c) => {
                let new_y = -c.center.y + 2.0 * center_y + offset_y;
                let new_x = c.center.x + offset_x;
                Shape::Circle(Circle::new(Point::new(new_x, new_y), c.radius))
            }
            Self::Line(l) => {
                let start_y = -l.start.y + 2.0 * center_y + offset_y;
                let start_x = l.start.x + offset_x;
                let end_y = -l.end.y + 2.0 * center_y + offset_y;
                let end_x = l.end.x + offset_x;
                Shape::Line(Line::new(
                    Point::new(start_x, start_y),
                                      Point::new(end_x, end_y),
                ))
            }
            Self::Ellipse(e) => {
                let new_y = -e.center.y + 2.0 * center_y + offset_y;
                let new_x = e.center.x + offset_x;
                Shape::Ellipse(Ellipse::new(Point::new(new_x, new_y), e.rx, e.ry))
            }
            Self::Path(p) => {
                // Transform: Translate(0, -c) -> Scale(1, -1) -> Translate(0, c) -> Translate(off_x, off_y)
                // y' = -y + 2c + off_y
                // x' = x + off_x

                let transform = lyon::math::Transform::new(
                    1.0,
                    0.0,
                    0.0,
                    -1.0,
                    offset_x as f32,
                    (2.0 * center_y + offset_y) as f32,
                );
                let mut new_p = p.clone();
                new_p.transform(&transform);
                Shape::Path(new_p)
            }
        }
    }
}

impl SvgImporter {
    /// Create a new SVG importer with optional scaling
    pub fn new(scale: f64, offset_x: f64, offset_y: f64) -> Self {
        Self {
            scale,
            offset_x,
            offset_y,
        }
    }

    /// Import SVG from string content
    pub fn import_string(&self, svg_content: &str) -> Result<ImportedDesign> {
        // Validate SVG structure by checking for basic tags
        if !svg_content.contains("<svg") {
            anyhow::bail!("Invalid SVG: missing <svg> element");
        }

        let mut imported_shapes: Vec<ImportedShape> = Vec::new();
        let mut viewbox_width = 100.0f64;
        let mut _viewbox_height = 100.0f64;

        // Parse width and height from SVG element
        if let Some(svg_start) = svg_content.find("<svg") {
            if let Some(svg_end) = svg_content[svg_start..].find('>') {
                let svg_tag = &svg_content[svg_start..svg_start + svg_end];

                if let Some(w) = Self::extract_attr_f64(svg_tag, "width") {
                    viewbox_width = w;
                }
                if let Some(h) = Self::extract_attr_f64(svg_tag, "height") {
                    _viewbox_height = h;
                }
            }
        }

        // Parse viewBox from SVG element (overrides width/height for logical dimensions if present)
        if let Some(viewbox_start) = svg_content.find("viewBox=\"") {
            if let Some(viewbox_end) = svg_content[viewbox_start + 9..].find('"') {
                let viewbox_str = &svg_content[viewbox_start + 9..viewbox_start + 9 + viewbox_end];
                let parts: Vec<&str> = viewbox_str.split_whitespace().collect();
                if parts.len() >= 4 {
                    viewbox_width = parts[2].parse().unwrap_or(100.0);
                    _viewbox_height = parts[3].parse().unwrap_or(100.0);
                }
            }
        }

        // Extract group transform matrix
        let mut group_transform = None;
        if let Some(g_start) = svg_content.find("<g") {
            if let Some(g_end) = svg_content[g_start..].find('>') {
                let g_tag = &svg_content[g_start..g_start + g_end];
                if let Some(transform_start) = g_tag.find("transform=\"") {
                    if let Some(transform_end) = g_tag[transform_start + 11..].find('"') {
                        let transform_str =
                        &g_tag[transform_start + 11..transform_start + 11 + transform_end];
                        group_transform = Self::parse_matrix_transform(transform_str);
                    }
                }
            }
        }

        // Extract all <rect .../> elements
        let mut search_pos = 0;
        while let Some(tag_start) = svg_content[search_pos..].find("<rect") {
            let abs_tag_start = search_pos + tag_start;
            if let Some(tag_end) = svg_content[abs_tag_start..].find('>') {
                let tag_content = &svg_content[abs_tag_start..abs_tag_start + tag_end];

                let x = Self::extract_attr_f64(tag_content, "x").unwrap_or(0.0);
                let y = Self::extract_attr_f64(tag_content, "y").unwrap_or(0.0);
                let width = Self::extract_attr_f64(tag_content, "width").unwrap_or(0.0);
                let height = Self::extract_attr_f64(tag_content, "height").unwrap_or(0.0);

                if width > 0.0 && height > 0.0 {
                    let rect = Rectangle::new(
                        x * self.scale,
                        y * self.scale,
                        width * self.scale,
                        height * self.scale,
                    );
                    imported_shapes.push(ImportedShape::Rect(rect));
                }
                search_pos = abs_tag_start + tag_end + 1;
            } else {
                break;
            }
        }

        // Extract all <circle .../> elements
        let mut search_pos = 0;
        while let Some(tag_start) = svg_content[search_pos..].find("<circle") {
            let abs_tag_start = search_pos + tag_start;
            if let Some(tag_end) = svg_content[abs_tag_start..].find('>') {
                let tag_content = &svg_content[abs_tag_start..abs_tag_start + tag_end];

                let cx = Self::extract_attr_f64(tag_content, "cx").unwrap_or(0.0);
                let cy = Self::extract_attr_f64(tag_content, "cy").unwrap_or(0.0);
                let r = Self::extract_attr_f64(tag_content, "r").unwrap_or(0.0);

                if r > 0.0 {
                    let circle =
                    Circle::new(Point::new(cx * self.scale, cy * self.scale), r * self.scale);
                    imported_shapes.push(ImportedShape::Circle(circle));
                }
                search_pos = abs_tag_start + tag_end + 1;
            } else {
                break;
            }
        }

        // Extract all <line .../> elements
        let mut search_pos = 0;
        while let Some(tag_start) = svg_content[search_pos..].find("<line") {
            let abs_tag_start = search_pos + tag_start;
            if let Some(tag_end) = svg_content[abs_tag_start..].find('>') {
                let tag_content = &svg_content[abs_tag_start..abs_tag_start + tag_end];

                let x1 = Self::extract_attr_f64(tag_content, "x1").unwrap_or(0.0);
                let y1 = Self::extract_attr_f64(tag_content, "y1").unwrap_or(0.0);
                let x2 = Self::extract_attr_f64(tag_content, "x2").unwrap_or(0.0);
                let y2 = Self::extract_attr_f64(tag_content, "y2").unwrap_or(0.0);

                let line = Line::new(
                    Point::new(x1 * self.scale, y1 * self.scale),
                                     Point::new(x2 * self.scale, y2 * self.scale),
                );
                imported_shapes.push(ImportedShape::Line(line));

                search_pos = abs_tag_start + tag_end + 1;
            } else {
                break;
            }
        }

        // Extract all <ellipse .../> elements
        let mut search_pos = 0;
        while let Some(tag_start) = svg_content[search_pos..].find("<ellipse") {
            let abs_tag_start = search_pos + tag_start;
            if let Some(tag_end) = svg_content[abs_tag_start..].find('>') {
                let tag_content = &svg_content[abs_tag_start..abs_tag_start + tag_end];

                let cx = Self::extract_attr_f64(tag_content, "cx").unwrap_or(0.0);
                let cy = Self::extract_attr_f64(tag_content, "cy").unwrap_or(0.0);
                let rx = Self::extract_attr_f64(tag_content, "rx").unwrap_or(0.0);
                let ry = Self::extract_attr_f64(tag_content, "ry").unwrap_or(0.0);

                if rx > 0.0 && ry > 0.0 {
                    let ellipse = Ellipse::new(
                        Point::new(cx * self.scale, cy * self.scale),
                                               rx * self.scale,
                                               ry * self.scale,
                    );
                    imported_shapes.push(ImportedShape::Ellipse(ellipse));
                }
                search_pos = abs_tag_start + tag_end + 1;
            } else {
                break;
            }
        }

        // Extract all <polyline .../> elements
        let mut search_pos = 0;
        while let Some(tag_start) = svg_content[search_pos..].find("<polyline") {
            let abs_tag_start = search_pos + tag_start;
            if let Some(tag_end) = svg_content[abs_tag_start..].find('>') {
                let tag_content = &svg_content[abs_tag_start..abs_tag_start + tag_end];

                if let Some(points_str) = Self::extract_attr_str(tag_content, "points") {
                    let points: Vec<Point> = points_str
                    .split([' ', ','])
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<&str>>()
                    .chunks(2)
                    .filter_map(|chunk| {
                        if chunk.len() == 2 {
                            let x = chunk[0].parse::<f64>().ok()?;
                            let y = chunk[1].parse::<f64>().ok()?;
                            Some(Point::new(x * self.scale, y * self.scale))
                        } else {
                            None
                        }
                    })
                    .collect();

                    if !points.is_empty() {
                        imported_shapes
                        .push(ImportedShape::Path(PathShape::from_points(&points, false)));
                    }
                }
                search_pos = abs_tag_start + tag_end + 1;
            } else {
                break;
            }
        }

        // Extract all <polygon .../> elements
        let mut search_pos = 0;
        while let Some(tag_start) = svg_content[search_pos..].find("<polygon") {
            let abs_tag_start = search_pos + tag_start;
            if let Some(tag_end) = svg_content[abs_tag_start..].find('>') {
                let tag_content = &svg_content[abs_tag_start..abs_tag_start + tag_end];

                if let Some(points_str) = Self::extract_attr_str(tag_content, "points") {
                    let points: Vec<Point> = points_str
                    .split([' ', ','])
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<&str>>()
                    .chunks(2)
                    .filter_map(|chunk| {
                        if chunk.len() == 2 {
                            let x = chunk[0].parse::<f64>().ok()?;
                            let y = chunk[1].parse::<f64>().ok()?;
                            Some(Point::new(x * self.scale, y * self.scale))
                        } else {
                            None
                        }
                    })
                    .collect();

                    if !points.is_empty() {
                        imported_shapes
                        .push(ImportedShape::Path(PathShape::from_points(&points, true)));
                    }
                }
                search_pos = abs_tag_start + tag_end + 1;
            } else {
                break;
            }
        }

        // Extract all <path d="..."/> elements
        let mut search_pos = 0;
        while let Some(path_start) = svg_content[search_pos..].find("<path") {
            let abs_path_start = search_pos + path_start;
            if let Some(path_end) = svg_content[abs_path_start..].find('>') {
                let path_tag_end = abs_path_start + path_end;

                // Find d attribute
                if let Some(d_start) = svg_content[abs_path_start..path_tag_end].find("d=\"") {
                    let abs_d_start = abs_path_start + d_start + 3;
                    if let Some(d_end) = svg_content[abs_d_start..path_tag_end].find('"') {
                        let d_value = &svg_content[abs_d_start..abs_d_start + d_end];

                        // Parse SVG path data
                        if let Some(path) = PathShape::from_svg_path(d_value) {
                            let mut new_path = path.clone();
                            if let Some((a, b, c, d_coeff, e, f)) = group_transform {
                                let transform = lyon::math::Transform::new(a, b, c, d_coeff, e, f);
                                new_path.transform(&transform);
                            }

                            let scale_transform =
                            lyon::math::Transform::scale(self.scale as f32, self.scale as f32);
                            new_path.transform(&scale_transform);

                            imported_shapes.push(ImportedShape::Path(new_path));
                        }
                    }
                }

                search_pos = path_tag_end + 1;
            } else {
                break;
            }
        }

        // Calculate bounds and mirror
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;

        for shape in &imported_shapes {
            let (_, s_min_y, _, s_max_y) = shape.bounds();
            if s_min_y < min_y {
                min_y = s_min_y;
            }
            if s_max_y > max_y {
                max_y = s_max_y;
            }
        }

        let center_y = if min_y == f64::MAX {
            0.0
        } else {
            (min_y + max_y) / 2.0
        };

        let shapes: Vec<Shape> = imported_shapes
        .into_iter()
        .map(|s| s.convert(center_y, self.offset_x, self.offset_y))
        .collect();

        // Default to 0 layers for SVG import unless more explicit layer logic is implemented
        let layer_count = 0usize;

        Ok(ImportedDesign {
            shapes,
           dimensions: (viewbox_width * self.scale, _viewbox_height * self.scale),
           format: FileFormat::Svg,
               layer_count,
           mesh_3d: None,
        })
    }

    fn extract_attr_str<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
        let pattern = format!("{}=\"", attr);
        if let Some(start) = tag.find(&pattern) {
            let val_start = start + pattern.len();
            if let Some(end) = tag[val_start..].find('"') {
                return Some(&tag[val_start..val_start + end]);
            }
        }
        None
    }

    fn extract_attr_f64(tag: &str, attr: &str) -> Option<f64> {
        Self::extract_attr_str(tag, attr).and_then(|s| s.parse().ok())
    }

    /// Parse matrix transform from SVG matrix(a,b,c,d,e,f) format
    fn parse_matrix_transform(transform_str: &str) -> Option<(f32, f32, f32, f32, f32, f32)> {
        let trimmed = transform_str.trim();
        if !trimmed.starts_with("matrix(") || !trimmed.ends_with(")") {
            return None;
        }

        let inner = &trimmed[7..trimmed.len() - 1];
        let values: Result<Vec<f32>, _> =
        inner.split(',').map(|s| s.trim().parse::<f32>()).collect();

        if let Ok(vals) = values {
            if vals.len() == 6 {
                return Some((vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]));
            }
        }
        None
    }
}

/// DXF importer for converting DXF files to Designer shapes
pub struct DxfImporter {
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl DxfImporter {
    /// Create a new DXF importer with optional scaling
    pub fn new(scale: f64, offset_x: f64, offset_y: f64) -> Self {
        Self {
            scale,
            offset_x,
            offset_y,
        }
    }

    /// Import DXF from file path
    ///
    /// # Arguments
    /// * `path` - Path to DXF file
    ///
    /// # Returns
    /// Imported design with converted shapes
    pub fn import_file(&self, path: &str) -> Result<ImportedDesign> {
        let content =
        std::fs::read_to_string(path).map_err(|e| anyhow!("Failed to read DXF file: {}", e))?;

        self.import_string(&content)
    }

    /// Import DXF from string content
    ///
    /// # Arguments
    /// * `content` - DXF file content as string
    ///
    /// # Returns
    /// Imported design with converted shapes
    pub fn import_string(&self, content: &str) -> Result<ImportedDesign> {
        let mut dxf_file = DxfParser::parse(content)?;

        // Apply scaling
        dxf_file.scale(self.scale);

        // Convert DXF entities to Designer shapes
        let shapes = self.convert_entities_to_shapes(&dxf_file)?;

        // Calculate dimensions from bounding box
        let (min, max) = dxf_file.bounds();
        let dimensions = ((max.x - min.x).abs(), (max.y - min.y).abs());

        Ok(ImportedDesign {
            shapes,
           dimensions,
           format: FileFormat::Dxf,
               layer_count: dxf_file.layer_names().len(),
           mesh_3d: None,
        })
    }

    /// Convert DXF entities to Designer shapes
    ///
    /// Note: DXF coordinates are negated on X-axis to correct for coordinate system difference.
    /// DXF uses right-handed coordinate system, Designer uses left-handed with Y-up.
    fn convert_entities_to_shapes(&self, dxf_file: &DxfFile) -> Result<Vec<Shape>> {
        let mut shapes: Vec<Shape> = Vec::new();

        // Transform to apply: negate X and add offset
        // Note: dxf_file is already scaled by self.scale
        //        let transform = lyon::math::Transform::scale(-1.0, 1.0).then_translate(lyon::math::vector(
        let transform = lyon::math::Transform::translation(
            self.offset_x as f32,
            self.offset_y as f32,
        );

        for entity in &dxf_file.entities {
            let path_opt = match entity {
                DxfEntity::Line(line) => {
                    let mut builder = Path::builder();
                    builder.begin(point(line.start.x as f32, line.start.y as f32));
                    builder.line_to(point(line.end.x as f32, line.end.y as f32));
                    builder.end(false);
                    Some(builder.build())
                }

                DxfEntity::Circle(circle) => {
                    let mut builder = Path::builder();
                    let center = point(circle.center.x as f32, circle.center.y as f32);
                    let radius = circle.radius as f32;

                    let steps = 64; // Número de puntos para un círculo suave
                    let start_point = center + lyon::math::vector(radius, 0.0);
                    builder.begin(start_point);

                    for i in 1..=steps {
                        let angle = 2.0 * std::f32::consts::PI * (i as f32 / steps as f32);
                        let x = center.x + radius * angle.cos();
                        let y = center.y + radius * angle.sin();
                        builder.line_to(point(x, y));
                    }

                    builder.close();
                    Some(builder.build())
                }

                DxfEntity::Arc(arc) => {
                    let mut builder = Path::builder();
                    let center = point(arc.center.x as f32, arc.center.y as f32);
                    let radius = arc.radius as f32;

                    let start_angle = arc.start_angle.to_radians() as f32;
                    let end_angle = arc.end_angle.to_radians() as f32;

                    let mut sweep_angle = end_angle - start_angle;
                    if sweep_angle < 0.0 {
                        sweep_angle += 2.0 * std::f32::consts::PI;
                    }

                    let steps = 64; // Número de puntos para un arco suave
                    let steps = (steps as f32 * sweep_angle / (2.0 * std::f32::consts::PI)).ceil() as usize;
                    let steps = steps.max(4); // Mínimo 4 puntos para arcos pequeños

                    // Punto inicial
                    let start_point = center + vector(
                        radius * start_angle.cos(),
                                                      radius * start_angle.sin()
                    );
                    builder.begin(start_point);

                    // Generar puntos intermedios
                    for i in 1..=steps {
                        let t = start_angle + sweep_angle * (i as f32 / steps as f32);
                        let x = center.x + radius * t.cos();
                        let y = center.y + radius * t.sin();
                        builder.line_to(point(x, y));
                    }

                    builder.end(false);
                    Some(builder.build())
                }

                DxfEntity::Ellipse(ellipse) => {
                    let center = point(ellipse.center.x as f32, ellipse.center.y as f32);

                    let major_vec = vector(ellipse.major_axis.x as f32, ellipse.major_axis.y as f32);
                    let major_radius = (major_vec.x * major_vec.x + major_vec.y * major_vec.y).sqrt();
                    let minor_radius = major_radius * ellipse.ratio as f32;

                    let rotation = major_vec.y.atan2(major_vec.x);

                    let start_angle = ellipse.start_angle as f32;
                    let end_angle = ellipse.end_angle as f32;

                    // Calc sweep angle
                    let mut sweep_angle = end_angle - start_angle;
                    if sweep_angle < 0.0 {
                        sweep_angle += 2.0 * std::f32::consts::PI;
                    }

                    let steps = 64;
                    let mut builder = Path::builder();

                    let start_x = center.x + major_radius * rotation.cos() * start_angle.cos() - minor_radius * rotation.sin() * start_angle.sin();
                    let start_y = center.y + major_radius * rotation.sin() * start_angle.cos() + minor_radius * rotation.cos() * start_angle.sin();
                    builder.begin(point(start_x, start_y));

                    for i in 1..=steps {
                        let t = start_angle + sweep_angle * (i as f32 / steps as f32);
                        let x = center.x + major_radius * rotation.cos() * t.cos() - minor_radius * rotation.sin() * t.sin();
                        let y = center.y + major_radius * rotation.sin() * t.cos() + minor_radius * rotation.cos() * t.sin();
                        builder.line_to(point(x, y));
                    }

                    let is_complete = (sweep_angle - 2.0 * std::f32::consts::PI).abs() < 0.001;
                    if is_complete {
                        builder.line_to(point(start_x, start_y));
                    }

                    builder.end(false);
                    let path = builder.build();

                    Some(path)
                }

                DxfEntity::Polyline(polyline) => {
                    if polyline.vertices.is_empty() {
                        None
                    } else {
                        use lyon::math::{point, vector};

                        let mut builder = Path::builder();

                        // Empezar en el primer vértice
                        let start = polyline.vertices[0];
                        builder.begin(point(start.x as f32, start.y as f32));

                        // Procesar cada segmento con su posible bulge
                        for i in 0..polyline.vertices.len() - 1 {
                            let v1 = polyline.vertices[i];
                            let v2 = polyline.vertices[i + 1];

                            let p1 = point(v1.x as f32, v1.y as f32);
                            let p2 = point(v2.x as f32, v2.y as f32);

                            // Obtener el bulge para este segmento (si existe)
                            let bulge = if i < polyline.bulges.len() {
                                polyline.bulges[i]
                            } else {
                                0.0  // Sin bulge = línea recta
                            };


                            if bulge.abs() < 0.0001 {
                                // Línea recta
                                builder.line_to(p2);
                            } else {

                                let dist = ((v2.x - v1.x).powi(2) + (v2.y - v1.y).powi(2)).sqrt() as f32;

                                let included_angle = 4.0 * bulge.abs().atan() as f32;

                                let h = bulge.abs() as f32 * (dist / 2.0);

                                let radius = ( (dist / 2.0).powi(2) + h.powi(2) ) / (2.0 * h);

                                let dist_to_center = radius - h;

                                let mid = point(
                                    (v1.x + v2.x) as f32 / 2.0,
                                                (v1.y + v2.y) as f32 / 2.0
                                );

                                let perp = vector(
                                    -(v2.y - v1.y) as f32 / dist,
                                                  (v2.x - v1.x) as f32 / dist
                                );

                                let center = if bulge > 0.0 {
                                    mid + perp * dist_to_center
                                } else {
                                    mid - perp * dist_to_center
                                };

                                let start_vec = p1 - center;
                                let start_angle = start_vec.y.atan2(start_vec.x);

                                let sweep_angle = if bulge > 0.0 {
                                    included_angle
                                } else {
                                    -included_angle
                                };

                                let arc_geom = lyon::geom::Arc {
                                    center,
                                    radii: vector(radius, radius),
                                    x_rotation: lyon::math::Angle::radians(0.0),
                                    start_angle: lyon::math::Angle::radians(start_angle),
                                    sweep_angle: lyon::math::Angle::radians(sweep_angle),
                                };

                                arc_geom.for_each_cubic_bezier(&mut |ctrl| {
                                    builder.cubic_bezier_to(ctrl.ctrl1, ctrl.ctrl2, ctrl.to);
                                });
                            }

                        }

                        if polyline.closed {
                            builder.close();
                        } else {
                            builder.end(false);
                        }
                        Some(builder.build())
                    }
                }
                _ => None,
            };

            if let Some(path) = path_opt {
                let mut shape = PathShape::from_lyon_path(&path);
                shape.transform(&transform);
                shapes.push(Shape::Path(shape));
            }
        }

        Ok(shapes)
    }
}

/// STL importer for converting 3D STL files to Designer shapes via shadow projection
pub struct StlImporter {
    pub scale: f32,
    pub center_model: bool,
    pub projection_direction: nalgebra::Vector3<f32>,
}

impl StlImporter {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            center_model: true,
            projection_direction: nalgebra::Vector3::new(0.0, 0.0, -1.0), // Project along Z-axis
        }
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_centering(mut self, center: bool) -> Self {
        self.center_model = center;
        self
    }

    pub fn with_projection_direction(mut self, direction: nalgebra::Vector3<f32>) -> Self {
        self.projection_direction = direction.normalize();
        self
    }

    /// Import STL file and return both 3D mesh and 2D shadow projection
    pub fn import_file(&self, path: &str) -> Result<ImportedDesign> {
        let importer = Model3DImporter::new()
        .with_scale(self.scale)
        .with_centering(self.center_model);

        let mesh = importer.import_file(path)?;
        self.create_imported_design(mesh)
    }

    /// Import STL from binary data
    pub fn import_data(&self, data: &[u8]) -> Result<ImportedDesign> {
        let importer = Model3DImporter::new()
        .with_scale(self.scale)
        .with_centering(self.center_model);

        let mesh = importer.import_stl_data(data)?;
        self.create_imported_design(mesh)
    }

    /// Project 3D mesh to 2D shadow and create ImportedDesign
    fn create_imported_design(&self, mesh: Mesh3D) -> Result<ImportedDesign> {
        // Generate 2D shadow projection
        let shapes = mesh.project_shadow_z()?;

        // Calculate 2D dimensions from mesh bounds
        let width = (mesh.bounds_max.x - mesh.bounds_min.x) as f64;
        let height = (mesh.bounds_max.y - mesh.bounds_min.y) as f64;
        let dimensions = (width, height);

        Ok(ImportedDesign {
            shapes,
           dimensions,
           format: FileFormat::Stl,
               layer_count: 1, // STL shadow projection creates a single layer
           mesh_3d: Some(mesh),
        })
    }

    /// Import STL and slice at specific Z height
    pub fn import_with_slice(&self, path: &str, z_height: f32) -> Result<ImportedDesign> {
        let importer = Model3DImporter::new()
        .with_scale(self.scale)
        .with_centering(self.center_model);

        let mesh = importer.import_file(path)?;

        // Generate 2D slice instead of shadow projection
        let shapes = mesh.slice_at_z(z_height)?;

        // Calculate 2D dimensions from mesh bounds
        let width = (mesh.bounds_max.x - mesh.bounds_min.x) as f64;
        let height = (mesh.bounds_max.y - mesh.bounds_min.y) as f64;
        let dimensions = (width, height);

        Ok(ImportedDesign {
            shapes,
           dimensions,
           format: FileFormat::Stl,
               layer_count: 1, // Single slice creates one layer
           mesh_3d: Some(mesh),
        })
    }
}

impl Default for StlImporter {
    fn default() -> Self {
        Self::new()
    }
}
