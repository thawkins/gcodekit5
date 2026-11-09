//! Serialization and deserialization for designer files
//!
//! Implements save/load functionality for .gck4 (GCodeKit4) design files
//! using JSON format with complete design state preservation.

use crate::model::{
    DesignCircle as Circle, DesignEllipse as Ellipse, DesignLine as Line, DesignPath as PathShape,
    DesignPolygon as Polygon, DesignRectangle as Rectangle, DesignText as TextShape,
    DesignTriangle as Triangle, RasterImage,
};
use crate::shapes::OperationType;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::canvas::DrawingObject;
use super::pocket_operations::PocketStrategy;
use crate::model::*;

/// Design file format version
const FILE_FORMAT_VERSION: &str = "1.1";

/// Document mode stored in design files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DesignMode {
    #[default]
    #[serde(rename = "2d")]
    TwoD,
    #[serde(rename = "3d")]
    ThreeD,
}

/// Complete design file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignFile {
    pub version: String,
    #[serde(default)]
    pub design_mode: DesignMode,
    pub metadata: DesignMetadata,
    pub viewport: ViewportState,
    pub shapes: Vec<ShapeData>,
    #[serde(default)]
    pub default_properties: Option<ShapeData>,
    #[serde(default)]
    pub toolpath_params: ToolpathParameters,
}

/// Design metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignMetadata {
    pub name: String,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
}

/// Viewport state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportState {
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
}

/// Serialized shape data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeData {
    pub id: i32,
    pub shape_type: String,
    #[serde(default)]
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub right_angle_corner: u8,
    #[serde(default)]
    pub points: Vec<(f64, f64)>,
    pub selected: bool,
    #[serde(default)]
    pub use_custom_values: bool,
    #[serde(default)]
    pub operation_type: String,
    #[serde(default)]
    pub pocket_depth: f64,
    #[serde(default)]
    pub start_depth: f64,
    #[serde(default)]
    pub step_down: f32,
    #[serde(default)]
    pub step_in: f32,
    #[serde(default)]
    pub text_content: String,
    #[serde(default)]
    pub font_size: f64,
    #[serde(default)]
    pub font_family: String,
    #[serde(default)]
    pub font_bold: bool,
    #[serde(default)]
    pub font_italic: bool,
    #[serde(default)]
    pub path_data: String,
    #[serde(default)]
    pub group_id: Option<u64>,
    #[serde(default)]
    pub corner_radius: f64,
    #[serde(default)]
    pub is_slot: bool,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub ramp_angle: f32,
    #[serde(default)]
    pub pocket_strategy: PocketStrategy,
    #[serde(default = "default_raster_fill_ratio")]
    pub raster_fill_ratio: f64,
    #[serde(default)]
    pub sides: u32,
    #[serde(default)]
    pub teeth: usize,
    #[serde(default)]
    pub module: f64,
    #[serde(default)]
    pub pressure_angle: f64,
    #[serde(default)]
    pub pitch: f64,
    #[serde(default)]
    pub roller_diameter: f64,
    #[serde(default)]
    pub thickness: f64,
    #[serde(default)]
    pub depth: f64,
    #[serde(default)]
    pub tab_size: f64,
    #[serde(default)]
    pub offset: f64,
    #[serde(default)]
    pub fillet: f64,
    #[serde(default)]
    pub chamfer: f64,
    #[serde(default = "default_lock_aspect_ratio")]
    pub lock_aspect_ratio: bool,
    #[serde(default)]
    pub original_path: Option<String>,
    #[serde(default = "default_true")]
    pub use_global_laser: bool,
    #[serde(default)]
    pub laser_params: LaserParams,
    // ========== PARÁMETROS LÁSER PARA IMÁGENES RÁSTER ==========
    #[serde(default = "default_feed_rate")]
    pub feed_rate: f64,
    #[serde(default = "default_travel_rate")]
    pub travel_rate: f64,
    #[serde(default = "default_min_power")]
    pub min_power: f64,
    #[serde(default = "default_max_power")]
    pub max_power: f64,
    #[serde(default = "default_ppi")]
    pub ppi: f64,
    #[serde(default = "default_bidirectional")]
    pub bidirectional: bool,
    #[serde(default = "default_invert")]
    pub invert: bool,
    #[serde(default = "default_scan_direction")]
    pub scan_direction: String,
    #[serde(default = "default_dithering")]
    pub dithering: String,
    #[serde(default = "default_halftone_threshold")]
    pub halftone_threshold: u8,
}

fn default_true() -> bool {
    true
}

fn default_lock_aspect_ratio() -> bool {
    false
}

fn default_raster_fill_ratio() -> f64 {
    0.5
}

// ========== FUNCIONES DEFAULT PARA PARÁMETROS LÁSER ==========
fn default_travel_rate() -> f64 {
    3000.0
}

fn default_min_power() -> f64 {
    0.0
}

fn default_max_power() -> f64 {
    20.0
}

fn default_ppi() -> f64 {
    254.0
}

fn default_bidirectional() -> bool {
    true
}

fn default_invert() -> bool {
    true
}

fn default_scan_direction() -> String {
    "horizontal".to_string()
}

fn default_dithering() -> String {
    "none".to_string()
}

fn default_halftone_threshold() -> u8 {
    127
}

/// Toolpath generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolpathParameters {
    #[serde(default = "default_feed_rate")]
    pub feed_rate: f64,
    #[serde(default = "default_spindle_speed")]
    pub spindle_speed: f64,
    #[serde(default = "default_tool_diameter")]
    pub tool_diameter: f64,
    #[serde(default = "default_cut_depth")]
    pub cut_depth: f64,
    #[serde(default = "default_step_down")]
    pub step_down: f64,
    #[serde(default = "default_continuous_z_between_passes")]
    pub continuous_z_between_passes: bool,
    #[serde(default = "default_stock_width")]
    pub stock_width: f32,
    #[serde(default = "default_stock_height")]
    pub stock_height: f32,
    #[serde(default = "default_stock_thickness")]
    pub stock_thickness: f32,
    #[serde(default = "default_safe_z_height")]
    pub safe_z_height: f32,
}

fn default_feed_rate() -> f64 {
    1000.0
}
fn default_spindle_speed() -> f64 {
    3000.0
}
fn default_tool_diameter() -> f64 {
    3.175
}
fn default_cut_depth() -> f64 {
    -5.0
}
fn default_step_down() -> f64 {
    1.0
}
fn default_continuous_z_between_passes() -> bool {
    false
}
fn default_stock_width() -> f32 {
    200.0
}
fn default_stock_height() -> f32 {
    200.0
}
fn default_stock_thickness() -> f32 {
    10.0
}
fn default_safe_z_height() -> f32 {
    default_stock_thickness() + 5.0
}

impl Default for ToolpathParameters {
    fn default() -> Self {
        Self {
            feed_rate: default_feed_rate(),
            spindle_speed: default_spindle_speed(),
            tool_diameter: default_tool_diameter(),
            cut_depth: default_cut_depth(),
            step_down: default_step_down(),
            continuous_z_between_passes: default_continuous_z_between_passes(),
            stock_width: default_stock_width(),
            stock_height: default_stock_height(),
            stock_thickness: default_stock_thickness(),
            safe_z_height: default_safe_z_height(),
        }
    }
}

impl ToolpathParameters {
    /// Builder method to set feed rate in mm/min.
    pub fn with_feed_rate(mut self, rate: f64) -> Self {
        self.feed_rate = rate;
        self
    }

    /// Builder method to set spindle speed in RPM.
    pub fn with_spindle_speed(mut self, speed: f64) -> Self {
        self.spindle_speed = speed;
        self
    }

    /// Builder method to set tool diameter in mm.
    pub fn with_tool_diameter(mut self, diameter: f64) -> Self {
        self.tool_diameter = diameter;
        self
    }

    /// Builder method to set cut depth in mm (negative value).
    pub fn with_cut_depth(mut self, depth: f64) -> Self {
        self.cut_depth = depth;
        self
    }

    /// Builder method to set step_down
    pub fn with_step_down(mut self, step_down: f64) -> Self {
        self.step_down = step_down;
        self
    }

    /// Builder method to enable/disable continuous Z between passes.
    pub fn with_continuous_z_between_passes(mut self, enabled: bool) -> Self {
        self.continuous_z_between_passes = enabled;
        self
    }

    /// Builder method to set stock width in mm.
    pub fn with_stock_width(mut self, width: f32) -> Self {
        self.stock_width = width;
        self
    }

    /// Builder method to set stock height in mm.
    pub fn with_stock_height(mut self, height: f32) -> Self {
        self.stock_height = height;
        self
    }

    /// Builder method to set stock thickness in mm.
    pub fn with_stock_thickness(mut self, thickness: f32) -> Self {
        self.stock_thickness = thickness;
        self
    }

    /// Builder method to set safe Z height in mm.
    pub fn with_safe_z_height(mut self, height: f32) -> Self {
        self.safe_z_height = height;
        self
    }
}

impl DesignFile {
    /// Create a new design file with default values
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            version: FILE_FORMAT_VERSION.to_string(),
            design_mode: DesignMode::default(),
            metadata: DesignMetadata {
                name: name.into(),
                created: now,
                modified: now,
                author: String::new(),
                description: String::new(),
            },
            viewport: ViewportState {
                zoom: 1.0,
                pan_x: 0.0,
                pan_y: 0.0,
            },
            shapes: Vec::new(),
            default_properties: None,
            toolpath_params: ToolpathParameters::default(),
        }
    }

    /// Save design to file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("Failed to serialize design")?;
        std::fs::write(path.as_ref(), json).context("Failed to write design file")?;
        Ok(())
    }

    /// Load design from file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content =
            std::fs::read_to_string(path.as_ref()).context("Failed to read design file")?;
        let mut design: DesignFile =
            serde_json::from_str(&content).context("Failed to parse design file")?;
        // Update modified timestamp
        design.metadata.modified = Utc::now();
        Ok(design)
    }

    pub fn from_drawing_object(obj: &DrawingObject) -> ShapeData {
        // 1. We only use the bounds to calculate the center (cx, cy)
        let (x1, y1, x2, y2) = obj.shape.bounds();
        let cx = (x1 + x2) / 2.0;
        let cy = (y1 + y2) / 2.0;

        // 2. We obtain the actual width and height of the object using shape-specific parameters
        let (real_width, real_height) = match &obj.shape {
            Shape::Rectangle(r) => (r.width, r.height),
            Shape::Circle(c) => (c.radius * 2.0, c.radius * 2.0),
            Shape::Ellipse(e) => (e.rx * 2.0, e.ry * 2.0),
            Shape::Triangle(t) => (t.width, t.height),
            Shape::Polygon(p) => (p.radius * 2.0, p.radius * 2.0),
            Shape::RasterImage(r) => (r.width_mm, r.height_mm),
            _ => (x2 - x1, y2 - y1),
        };

        let shape_points = match &obj.shape {
            Shape::Line(line) => vec![(line.start.x, line.start.y), (line.end.x, line.end.y)],
            _ => Vec::new(),
        };

        let shape_type = match obj.shape.shape_type() {
            ShapeType::Rectangle => "rectangle",
            ShapeType::Circle => "circle",
            ShapeType::Line => "line",
            ShapeType::Ellipse => "ellipse",
            ShapeType::Path => "path",
            ShapeType::Text => "text",
            ShapeType::Triangle => "triangle",
            ShapeType::Polygon => "polygon",
            ShapeType::Gear => "gear",
            ShapeType::Sprocket => "sprocket",
            ShapeType::RasterImage => "raster_image",
        };

        let rotation = obj.shape.rotation();

        let (text_content, font_size, font_family, font_bold, font_italic) =
            if let Shape::Text(text_shape) = &obj.shape {
                (
                    text_shape.text.clone(),
                    text_shape.font_size,
                    text_shape.font_family.clone(),
                    text_shape.bold,
                    text_shape.italic,
                )
            } else {
                (String::new(), 0.0, String::new(), false, false)
            };

        let path_data = if let Shape::Path(path_shape) = &obj.shape {
            path_shape.to_svg_path()
        } else {
            String::new()
        };

        let (corner_radius, is_slot) = if let Shape::Rectangle(r) = &obj.shape {
            (r.corner_radius, r.is_slot)
        } else {
            (0.0, false)
        };

        let sides = if let Shape::Polygon(p) = &obj.shape {
            p.sides
        } else {
            0
        };

        let mut teeth = 0;
        let mut module = 0.0;
        let mut pressure_angle = 0.0;
        let mut pitch = 0.0;
        let mut roller_diameter = 0.0;

        match &obj.shape {
            Shape::Gear(g) => {
                teeth = g.teeth;
                module = g.module;
                pressure_angle = g.pressure_angle_deg;
            }
            Shape::Sprocket(s) => {
                teeth = s.teeth;
                pitch = s.pitch;
                roller_diameter = s.roller_diameter;
            }
            _ => {}
        }

        let original_path = if let Shape::RasterImage(raster) = &obj.shape {
            raster
                .original_path
                .clone()
                .map(|p| p.display().to_string())
        } else {
            None
        };

        // ========== EXTRAER PARÁMETROS LÁSER ==========
        let (feed_rate, travel_rate, min_power, max_power, ppi, bidirectional, invert, scan_direction, dithering, halftone_threshold) =
            if let Shape::RasterImage(raster) = &obj.shape {
                (
                    raster.feed_rate,
                    raster.travel_rate,
                    raster.min_power,
                    raster.max_power,
                    raster.ppi,
                    raster.bidirectional,
                    raster.invert,
                    raster.scan_direction.clone(),
                    raster.dithering.clone(),
                    raster.halftone_threshold,
                )
            } else {
                (
                    default_feed_rate(),
                    default_travel_rate(),
                    default_min_power(),
                    default_max_power(),
                    default_ppi(),
                    default_bidirectional(),
                    default_invert(),
                    default_scan_direction(),
                    default_dithering(),
                    default_halftone_threshold(),
                )
            };

        let right_angle_corner = if let Shape::Triangle(t) = &obj.shape {
            t.right_angle_corner
        } else {
                0
        };

        let laser_params = obj.laser_params;

        ShapeData {
            id: obj.id as i32,
            shape_type: shape_type.to_string(),
            name: obj.name.clone(),
            x: cx,
            y: cy,
            width: real_width,
            height: real_height,
            right_angle_corner,
            points: shape_points,
            selected: obj.selected,
            use_custom_values: obj.use_custom_values,
            operation_type: match obj.operation_type {
                OperationType::Profile => "profile".to_string(),
                OperationType::Pocket => "pocket".to_string(),
            },
            pocket_depth: obj.pocket_depth,
            start_depth: obj.start_depth,
            step_down: obj.step_down,
            step_in: obj.step_in,
            text_content,
            font_size,
            font_family,
            font_bold,
            font_italic,
            path_data,
            group_id: obj.group_id,
            corner_radius,
            is_slot,
            rotation,
            ramp_angle: obj.ramp_angle,
            pocket_strategy: obj.pocket_strategy,
            raster_fill_ratio: obj.raster_fill_ratio,
            sides,
            teeth,
            module,
            pressure_angle,
            pitch,
            roller_diameter,
            thickness: 0.0,
            depth: obj.pocket_depth,
            tab_size: 0.0,
            offset: obj.offset,
            fillet: obj.fillet,
            chamfer: obj.chamfer,
            lock_aspect_ratio: obj.lock_aspect_ratio,
            original_path,
            use_global_laser: obj.use_global_laser,
            // ========== PARÁMETROS LÁSER ==========
            feed_rate,
            travel_rate,
            min_power,
            max_power,
            ppi,
            bidirectional,
            invert,
            scan_direction,
            dithering,
            halftone_threshold,
            laser_params,
        }
    }

    pub fn to_drawing_object(data: &ShapeData, next_id: i32) -> Result<DrawingObject> {
        // Obtener laser_params una sola vez
        let laser_params = data.laser_params;

        let shape: Shape = match data.shape_type.as_str() {
            "rectangle" => {
                let x = data.x - data.width / 2.0;
                let y = data.y - data.height / 2.0;
                let mut rect = Rectangle::new(x, y, data.width, data.height);
                rect.corner_radius = data.corner_radius;
                rect.is_slot = data.is_slot;
                rect.laser_params = laser_params;
                Shape::Rectangle(rect)
            }
            "circle" => {
                let radius = data.width.min(data.height) / 2.0;
                let center = Point::new(data.x, data.y);
                let mut circle = Circle::new(center, radius);
                circle.laser_params = laser_params;
                Shape::Circle(circle)
            }
            "line" => {
                let mut line = if data.points.len() >= 2 {
                    let start = Point::new(data.points[0].0, data.points[0].1);
                    let end = Point::new(data.points[1].0, data.points[1].1);
                    Line::new(start, end)
                } else {
                    let half_w = data.width / 2.0;
                    let half_h = data.height / 2.0;
                    let start = Point::new(data.x - half_w, data.y - half_h);
                    let end = Point::new(data.x + half_w, data.y + half_h);
                    Line::new(start, end)
                };
                line.rotation = data.rotation;
                line.laser_params = laser_params;
                Shape::Line(line)
            }
            "ellipse" => {
                let center = Point::new(data.x, data.y);
                let mut ellipse = Ellipse::new(center, data.width / 2.0, data.height / 2.0);
                ellipse.laser_params = laser_params;
                Shape::Ellipse(ellipse)
            }
            "triangle" => {
                let center = Point::new(data.x, data.y);
                let mut triangle = Triangle::new_with_corner(
                    center,
                    data.width,
                    data.height,
                    data.right_angle_corner
                );
                triangle.rotation = data.rotation;
                triangle.laser_params = laser_params;
                Shape::Triangle(triangle)
            }
            "polygon" => {
                let center = Point::new(data.x, data.y);
                let radius = data.width.max(data.height) / 2.0;
                let mut polygon = Polygon::new(center, radius, data.sides);
                polygon.laser_params = laser_params;
                Shape::Polygon(polygon)
            }
            "polyline" => {
                let center = Point::new(data.x, data.y);
                let radius = data.width.min(data.height) / 2.0;
                let sides = 6;
                let mut vertices = Vec::with_capacity(sides);
                for i in 0..sides {
                    let angle = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
                    let x = center.x + radius * angle.cos();
                    let y = center.y + radius * angle.sin();
                    vertices.push(Point::new(x, y));
                }
                let mut path = PathShape::from_points(&vertices, true);
                path.laser_params = laser_params;
                Shape::Path(path)
            }
            "text" => {
                let mut s = TextShape::new(data.text_content.clone(), data.x, data.y, data.font_size);
                if !data.font_family.is_empty() {
                    s.font_family = data.font_family.clone();
                }
                s.bold = data.font_bold;
                s.italic = data.font_italic;
                s.laser_params = laser_params;
                Shape::Text(s)
            }
            "path" => {
                if let Some(mut path_shape) = PathShape::from_svg_path(&data.path_data) {
                    path_shape.laser_params = laser_params;
                    Shape::Path(path_shape)
                } else {
                    // Fallback...
                    let mut rect = Rectangle::new(data.x, data.y, data.width, data.height);
                    rect.corner_radius = data.corner_radius;
                    rect.is_slot = data.is_slot;
                    rect.laser_params = laser_params;
                    Shape::Rectangle(rect)
                }
            }
            "gear" => {
                let center = Point::new(data.x, data.y);
                let mut gear = DesignGear::new(center, data.module, data.teeth);
                gear.pressure_angle_deg = data.pressure_angle;
                gear.laser_params = laser_params;
                Shape::Gear(gear)
            }
            "sprocket" => {
                let center = Point::new(data.x, data.y);
                let mut sprocket = DesignSprocket::new(center, data.pitch, data.teeth);
                sprocket.roller_diameter = data.roller_diameter;
                sprocket.laser_params = laser_params;
                Shape::Sprocket(sprocket)
            }
            "raster_image" => {
                let center = Point::new(data.x, data.y);
                let original_path = data.original_path.as_ref().map(std::path::PathBuf::from);

                let (image_data, width_mm, height_mm) = if let Some(ref path) = original_path {
                    if path.exists() {
                        match crate::image_importer::ImageImporter::load_image_data_with_size(
                            path,
                            data.width,
                            data.height,
                        ) {
                            Ok((data, w, h)) => (data, w, h),
                            Err(e) => {
                                eprintln!("Warning: Could not load image {:?}: {}", path, e);
                                (Vec::new(), data.width, data.height)
                            }
                        }
                    } else {
                        eprintln!("Warning: Image file not found: {:?}", path);
                        (Vec::new(), data.width, data.height)
                    }
                } else {
                    (Vec::new(), data.width, data.height)
                };

                let mut raster = RasterImage::new(
                    data.id as u64,
                    center,
                    width_mm,
                    height_mm,
                    image_data,
                    original_path,
                );

                // Parámetros láser para raster
                raster.feed_rate = data.feed_rate;
                raster.travel_rate = data.travel_rate;
                raster.min_power = data.min_power;
                raster.max_power = data.max_power;
                raster.ppi = data.ppi;
                raster.bidirectional = data.bidirectional;
                raster.invert = data.invert;
                raster.scan_direction = data.scan_direction.clone();
                raster.dithering = data.dithering.clone();
                raster.halftone_threshold = data.halftone_threshold;
                raster.rotation = data.rotation;

                Shape::RasterImage(raster)
            }
            _ => anyhow::bail!("Unknown shape type: {}", data.shape_type),
        };

        // Apply rotation
        let mut shape = shape;
        match &mut shape {
            Shape::Rectangle(s) => s.rotation = data.rotation,
            Shape::Circle(s) => s.rotation = data.rotation,
            Shape::Line(s) => s.rotation = data.rotation,
            Shape::Ellipse(s) => s.rotation = data.rotation,
            Shape::Path(s) => s.rotation = data.rotation,
            Shape::Text(s) => s.rotation = data.rotation,
            Shape::Triangle(s) => s.rotation = data.rotation,
            Shape::Polygon(s) => s.rotation = data.rotation,
            Shape::Gear(s) => s.rotation = data.rotation,
            Shape::Sprocket(s) => s.rotation = data.rotation,
            Shape::RasterImage(s) => s.rotation = data.rotation,
        }

        let operation_type = match data.operation_type.as_str() {
            "pocket" => OperationType::Pocket,
            _ => OperationType::Profile,
        };

        let default_name = match shape.shape_type() {
            crate::model::ShapeType::Rectangle => "Rectangle",
            crate::model::ShapeType::Circle => "Circle",
            crate::model::ShapeType::Line => "Line",
            crate::model::ShapeType::Ellipse => "Ellipse",
            crate::model::ShapeType::Path => "Path",
            crate::model::ShapeType::Text => "Text",
            crate::model::ShapeType::Triangle => "Triangle",
            crate::model::ShapeType::Polygon => "Polygon",
            crate::model::ShapeType::Gear => "Gear",
            crate::model::ShapeType::Sprocket => "Sprocket",
            crate::model::ShapeType::RasterImage => "Raster Image",
        };

        Ok(DrawingObject {
            id: next_id as u64,
            group_id: data.group_id,
            name: if data.name.is_empty() {
                default_name.to_string()
            } else {
                data.name.clone()
            },
            shape,
            selected: data.selected,
            operation_type,
            use_custom_values: data.use_custom_values,
            pocket_depth: data.pocket_depth,
            start_depth: data.start_depth,
            step_down: data.step_down,
            step_in: data.step_in,
            ramp_angle: data.ramp_angle,
            pocket_strategy: data.pocket_strategy,
            raster_fill_ratio: data.raster_fill_ratio,
            offset: data.offset,
            fillet: data.fillet,
            chamfer: data.chamfer,
            lock_aspect_ratio: data.lock_aspect_ratio,
            use_global_laser: data.use_global_laser,
            laser_params,
        })
    }
}
