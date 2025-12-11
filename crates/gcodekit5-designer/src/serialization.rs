//! Serialization and deserialization for designer files.
//!
//! Implements save/load functionality for .gck4 (GCodeKit4) design files
//! using JSON format with complete design state preservation.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::canvas::DrawingObject;
use super::shapes::*;
use super::pocket_operations::PocketStrategy;

/// Design file format version
const FILE_FORMAT_VERSION: &str = "1.0";

/// Complete design file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignFile {
    pub version: String,
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
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
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
    pub path_data: String,
    #[serde(default)]
    pub group_id: Option<u64>,
    #[serde(default)]
    pub corner_radius: f64,
    #[serde(default)]
    pub is_slot: bool,
    #[serde(default)]
    pub rotation: f64,
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
    #[serde(default = "default_stock_width")]
    pub stock_width: f32,
    #[serde(default = "default_stock_height")]
    pub stock_height: f32,
    #[serde(default = "default_stock_thickness")]
    pub stock_thickness: f32,
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
fn default_stock_width() -> f32 {
    200.0
}
fn default_stock_height() -> f32 {
    200.0
}
fn default_stock_thickness() -> f32 {
    10.0
}

impl Default for ToolpathParameters {
    fn default() -> Self {
        Self {
            feed_rate: default_feed_rate(),
            spindle_speed: default_spindle_speed(),
            tool_diameter: default_tool_diameter(),
            cut_depth: default_cut_depth(),
            stock_width: default_stock_width(),
            stock_height: default_stock_height(),
            stock_thickness: default_stock_thickness(),
        }
    }
}

impl DesignFile {
    /// Create a new design file with default values
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            version: FILE_FORMAT_VERSION.to_string(),
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

    /// Convert DrawingObject to ShapeData
    pub fn from_drawing_object(obj: &DrawingObject) -> ShapeData {
        let (x, y, x2, y2) = obj.shape.bounding_box();
        let width = x2 - x;
        let height = y2 - y;

        let shape_type = match obj.shape.shape_type() {
            ShapeType::Rectangle => "rectangle",
            ShapeType::Circle => "circle",
            ShapeType::Line => "line",
            ShapeType::Ellipse => "ellipse",
            ShapeType::Path => "path",
            ShapeType::Text => "text",
        };

        let (text_content, font_size) = if let Shape::Text(text_shape) = &obj.shape {
             (text_shape.text.clone(), text_shape.font_size)
        } else {
             (String::new(), 0.0)
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

        ShapeData {
            id: obj.id as i32,
            shape_type: shape_type.to_string(),
            x,
            y,
            width,
            height,
            points: Vec::new(),
            selected: false,
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
            path_data,
            group_id: obj.group_id,
            corner_radius,
            is_slot,
            rotation: obj.shape.rotation(),
        }
    }

    /// Convert ShapeData to DrawingObject
    pub fn to_drawing_object(data: &ShapeData, next_id: i32) -> Result<DrawingObject> {
        let shape: Shape = match data.shape_type.as_str() {
            "rectangle" => {
                let mut rect = Rectangle::new(data.x, data.y, data.width, data.height);
                rect.corner_radius = data.corner_radius;
                rect.is_slot = data.is_slot;
                Shape::Rectangle(rect)
            },
            "circle" => {
                let radius = data.width.min(data.height) / 2.0;
                let center = Point::new(data.x + radius, data.y + radius);
                Shape::Circle(Circle::new(center, radius))
            }
            "line" => {
                let start = Point::new(data.x, data.y);
                let end = Point::new(data.x + data.width, data.y + data.height);
                Shape::Line(Line::new(start, end))
            }
            "ellipse" => {
                let center = Point::new(data.x + data.width / 2.0, data.y + data.height / 2.0);
                Shape::Ellipse(Ellipse::new(center, data.width / 2.0, data.height / 2.0))
            }
            "polygon" | "polyline" => {
                let center = Point::new(data.x + data.width / 2.0, data.y + data.height / 2.0);
                let radius = data.width.min(data.height) / 2.0;
                let sides = 6;
                let mut vertices = Vec::with_capacity(sides);
                for i in 0..sides {
                    let angle = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
                    let x = center.x + radius * angle.cos();
                    let y = center.y + radius * angle.sin();
                    vertices.push(Point::new(x, y));
                }
                Shape::Path(PathShape::from_points(&vertices, true))
            }
            "text" => Shape::Text(TextShape::new(
                data.text_content.clone(),
                data.x,
                data.y,
                data.font_size,
            )),
            "path" => {
                if let Some(path_shape) = PathShape::from_svg_path(&data.path_data) {
                    Shape::Path(path_shape)
                } else {
                    // Fallback if path parsing fails
                    let mut rect = Rectangle::new(data.x, data.y, data.width, data.height);
                    rect.corner_radius = data.corner_radius;
                    rect.is_slot = data.is_slot;
                    Shape::Rectangle(rect)
                }
            },
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
        }

        let operation_type = match data.operation_type.as_str() {
            "pocket" => OperationType::Pocket,
            _ => OperationType::Profile,
        };

        Ok(DrawingObject {
            id: next_id as u64,
            group_id: data.group_id,
            name: match shape.shape_type() {
                crate::shapes::ShapeType::Rectangle => "Rectangle",
                crate::shapes::ShapeType::Circle => "Circle",
                crate::shapes::ShapeType::Line => "Line",
                crate::shapes::ShapeType::Ellipse => "Ellipse",
                crate::shapes::ShapeType::Path => "Path",
                crate::shapes::ShapeType::Text => "Text",
            }.to_string(),
            shape,
            selected: data.selected,
            operation_type,
            use_custom_values: data.use_custom_values,
            pocket_depth: data.pocket_depth,
            start_depth: data.start_depth,
            step_down: data.step_down,
            step_in: data.step_in,
            pocket_strategy: PocketStrategy::ContourParallel,
        })
    }
}
