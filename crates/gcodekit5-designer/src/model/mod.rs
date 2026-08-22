//! # Designer Shape Model
//!
//! Defines the core shape types and traits for the visual designer.
//! All shapes implement the [`DesignerShape`] trait, which provides:
//! - Lyon path generation for rendering
//! - CSG sketch generation for boolean operations
//! - Bounding box calculation
//! - Property inspection and editing
//! - Serialization/deserialization
//!
//! ## Shape Types
//! Rectangle, Circle, Ellipse, Line, Triangle, Polygon, Text, Path, Gear, Sprocket

use lyon::math::Transform;
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;

mod circle;
mod ellipse;
mod gear;
mod line;
mod path;
mod polygon;
mod rectangle;
mod sprocket;
mod text;
mod triangle;

pub use circle::DesignCircle;
pub use ellipse::DesignEllipse;
pub use gear::DesignGear;
pub use line::DesignLine;
pub use path::DesignPath;
pub use path::ParametricPathSource;
pub use polygon::DesignPolygon;
pub use rectangle::DesignRectangle;
pub use sprocket::DesignSprocket;
pub use text::DesignText;
pub use triangle::DesignTriangle;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
    /// Returns the midpoint between this point and another point
    pub fn midpoint(&self, other: &Point) -> Point {
        Point::new((self.x + other.x) / 2.0, (self.y + other.y) / 2.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: PropertyValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyValue {
    Number(f64),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterImage {
    pub id: u64,
    pub center: Point,
    pub width_mm: f64,
    pub height_mm: f64,
    pub rotation: f64,
    pub original_path: Option<std::path::PathBuf>,
    #[serde(skip)]
    pub image_data: Vec<u8>,
    pub feed_rate: f64,   // mm/s
    pub travel_rate: f64, // mm/s
    pub min_power: f64,   // %
    pub max_power: f64,   // %
    pub ppi: f64,         // dots per inch
    pub bidirectional: bool,
    pub invert: bool,
    pub scan_direction: String, // "horizontal" or "vertical"
    pub dithering: String,      // "none", "threshold", "floyd", "atkinson", "bayer"
    pub halftone_threshold: u8,
}

impl Default for RasterImage {
    fn default() -> Self {
        Self {
            id: 0,
            center: Point::new(0.0, 0.0),
            width_mm: 100.0,
            height_mm: 100.0,
            rotation: 0.0,
            image_data: Vec::new(),
            original_path: None,
            feed_rate: 2000.0,
            travel_rate: 3000.0,
            min_power: 0.0,
            max_power: 20.0, // Only 20%
            ppi: 254.0,
            bidirectional: true,
            invert: true,
            scan_direction: "horizontal".to_string(),
            dithering: "none".to_string(),
            halftone_threshold: 127,
        }
    }
}

impl RasterImage {
    pub fn new(
        id: u64,
        center: Point,
        width_mm: f64,
        height_mm: f64,
        image_data: Vec<u8>,
        original_path: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            id,
            center,
            width_mm,
            height_mm,
            image_data,
            original_path,
            ..Default::default()
        }
    }

    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        let half_w = self.width_mm / 2.0;
        let half_h = self.height_mm / 2.0;
        (
            self.center.x - half_w, // x1 (izquierda)
            self.center.y - half_h, // y1 (abajo)
            self.center.x + half_w, // x2 (derecha)
            self.center.y + half_h, // y2 (arriba)
        )
    }

    pub fn render(&self) -> Path {
        // Returns an empty path or the bounding box
        let (x1, y1, x2, y2) = self.bounds();
        let mut builder = Path::builder();
        builder.begin(lyon::math::Point::new(x1 as f32, y1 as f32));
        builder.line_to(lyon::math::Point::new(x2 as f32, y1 as f32));
        builder.line_to(lyon::math::Point::new(x2 as f32, y2 as f32));
        builder.line_to(lyon::math::Point::new(x1 as f32, y2 as f32));
        builder.close();
        builder.build()
    }

    pub fn as_csg(&self) -> Sketch<()> {
        // For raster images, return an empty sketch.
        unimplemented!("as_csg for RasterImage")
    }

    pub fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        let (x1, y1, x2, y2) = self.bounds();
        p.x >= x1 - tolerance
            && p.x <= x2 + tolerance
            && p.y >= y1 - tolerance
            && p.y <= y2 + tolerance
    }

    pub fn resize(&mut self, _handle: usize, dx: f64, dy: f64) {
        let delta = (dx.abs() + dy.abs()) / 2.0;
        let factor = 1.0 + delta / self.width_mm;
        self.width_mm *= factor;
        self.height_mm *= factor;
    }

    pub fn transform(&mut self, t: &Transform) {
        // Apply translation
        self.center.x += t.m31 as f64; // Updated X coordinate
        self.center.y += t.m32 as f64; // Updated Y coordinate

        // Scale (optional, if needed)
        let scale_x = (t.m11 as f64).hypot(t.m12 as f64);
        let scale_y = (t.m21 as f64).hypot(t.m22 as f64);
        if scale_x > 0.0 && scale_y > 0.0 {
            self.width_mm *= scale_x;
            self.height_mm *= scale_y;
        }
    }

    pub fn rotate(&mut self, angle_deg: f64, center: Point) {
        // Accumulate rotation
        let new_rotation = (self.rotation + angle_deg) % 360.0;
        let snapped = (new_rotation / 90.0).round() * 90.0;
        let delta = snapped - self.rotation;

        if delta.abs() < 0.1 {
            return;
        }

        // Apply rotation
        self.rotation = snapped;
        if self.rotation < 0.0 {
            self.rotation += 360.0;
        }

        // Swap dimensions if necessary
        if ((snapped / 90.0) as i32).rem_euclid(2) != 0 {
            std::mem::swap(&mut self.width_mm, &mut self.height_mm);
        }

        // Rotate the center around the given point
        let dx = self.center.x - center.x;
        let dy = self.center.y - center.y;
        let rad = delta.to_radians();
        let cos = rad.cos();
        let sin = rad.sin();
        self.center.x = center.x + dx * cos - dy * sin;
        self.center.y = center.y + dx * sin + dy * cos;
    }

    pub fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Type".to_string(),
                value: PropertyValue::String("Raster Image".to_string()),
            },
            Property {
                name: "Width".to_string(),
                value: PropertyValue::Number(self.width_mm),
            },
            Property {
                name: "Height".to_string(),
                value: PropertyValue::Number(self.height_mm),
            },
            Property {
                name: "X".to_string(),
                value: PropertyValue::Number(self.center.x),
            },
            Property {
                name: "Y".to_string(),
                value: PropertyValue::Number(self.center.y),
            },
            Property {
                name: "Rotation".to_string(),
                value: PropertyValue::Number(self.rotation),
            },
        ]
    }
} // impl RasterImage

// ---
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LaserParams {
    pub feed_rate: f64,     // mm/min
    pub power_percent: f64, // 0-100
    pub passes: u32,
    pub use_global: bool,
}

impl Default for LaserParams {
    fn default() -> Self {
        Self {
            feed_rate: 1010.0,
            power_percent: 100.0,
            passes: 1,
            use_global: true,
        }
    }
}
// ---

pub trait DesignerShape {
    fn render(&self) -> Path;
    fn as_csg(&self) -> Sketch<()>;
    fn bounds(&self) -> (f64, f64, f64, f64);
    fn transform(&mut self, t: &Transform);
    fn properties(&self) -> Vec<Property>;

    fn contains_point(&self, p: Point, tolerance: f64) -> bool;
    fn resize(&mut self, handle: usize, dx: f64, dy: f64);

    fn translate(&mut self, dx: f64, dy: f64) {
        let t = Transform::translation(dx as f32, dy as f32);
        self.transform(&t);
    }

    fn rotate(&mut self, angle: f64, cx: f64, cy: f64) {
        let t = Transform::translation(cx as f32, cy as f32)
            .then_rotate(lyon::math::Angle::radians(angle as f32))
            .then_translate(lyon::math::vector(-cx as f32, -cy as f32));
        self.transform(&t);
    }

    fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        // Translate to origin, scale, then translate back to keep the pivot fixed.
        let t = Transform::translation(-center.x as f32, -center.y as f32)
            .then_scale(sx as f32, sy as f32)
            .then_translate(lyon::math::vector(center.x as f32, center.y as f32));
        self.transform(&t);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeType {
    Rectangle,
    Circle,
    Path,
    Line,
    Ellipse,
    Text,
    Triangle,
    Polygon,
    Gear,
    Sprocket,
    RasterImage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Shape {
    Rectangle(DesignRectangle),
    Circle(DesignCircle),
    Path(DesignPath),
    Line(DesignLine),
    Ellipse(DesignEllipse),
    Text(DesignText),
    Triangle(DesignTriangle),
    Polygon(DesignPolygon),
    Gear(DesignGear),
    Sprocket(DesignSprocket),
    RasterImage(RasterImage),
}

impl DesignerShape for Shape {
    fn render(&self) -> Path {
        match self {
            Shape::Rectangle(s) => s.render(),
            Shape::Circle(s) => s.render(),
            Shape::Path(s) => s.render(),
            Shape::Line(s) => s.render(),
            Shape::Ellipse(s) => s.render(),
            Shape::Text(s) => s.render(),
            Shape::Triangle(s) => s.render(),
            Shape::Polygon(s) => s.render(),
            Shape::Gear(s) => s.render(),
            Shape::Sprocket(s) => s.render(),
            Shape::RasterImage(s) => s.render(),
        }
    }

    fn as_csg(&self) -> Sketch<()> {
        match self {
            Shape::Rectangle(s) => s.as_csg(),
            Shape::Circle(s) => s.as_csg(),
            Shape::Path(s) => s.as_csg(),
            Shape::Line(s) => s.as_csg(),
            Shape::Ellipse(s) => s.as_csg(),
            Shape::Text(s) => s.as_csg(),
            Shape::Triangle(s) => s.as_csg(),
            Shape::Polygon(s) => s.as_csg(),
            Shape::Gear(s) => s.as_csg(),
            Shape::Sprocket(s) => s.as_csg(),
            Shape::RasterImage(s) => s.as_csg(),
        }
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        match self {
            Shape::Rectangle(s) => s.bounds(),
            Shape::Circle(s) => s.bounds(),
            Shape::Path(s) => s.bounds(),
            Shape::Line(s) => s.bounds(),
            Shape::Ellipse(s) => s.bounds(),
            Shape::Text(s) => s.bounds(),
            Shape::Triangle(s) => s.bounds(),
            Shape::Polygon(s) => s.bounds(),
            Shape::Gear(s) => s.bounds(),
            Shape::Sprocket(s) => s.bounds(),
            Shape::RasterImage(s) => s.bounds(),
        }
    }

    fn transform(&mut self, t: &Transform) {
        match self {
            Shape::Rectangle(s) => s.transform(t),
            Shape::Circle(s) => s.transform(t),
            Shape::Path(s) => s.transform(t),
            Shape::Line(s) => s.transform(t),
            Shape::Ellipse(s) => s.transform(t),
            Shape::Text(s) => s.transform(t),
            Shape::Triangle(s) => s.transform(t),
            Shape::Polygon(s) => s.transform(t),
            Shape::Gear(s) => s.transform(t),
            Shape::Sprocket(s) => s.transform(t),
            Shape::RasterImage(s) => s.transform(t),
        }
    }

    fn properties(&self) -> Vec<Property> {
        match self {
            Shape::Rectangle(s) => s.properties(),
            Shape::Circle(s) => s.properties(),
            Shape::Path(s) => s.properties(),
            Shape::Line(s) => s.properties(),
            Shape::Ellipse(s) => s.properties(),
            Shape::Text(s) => s.properties(),
            Shape::Triangle(s) => s.properties(),
            Shape::Polygon(s) => s.properties(),
            Shape::Gear(s) => s.properties(),
            Shape::Sprocket(s) => s.properties(),
            Shape::RasterImage(s) => s.properties(),
        }
    }

    fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        match self {
            Shape::Rectangle(s) => s.contains_point(p, tolerance),
            Shape::Circle(s) => s.contains_point(p, tolerance),
            Shape::Path(s) => s.contains_point(p, tolerance),
            Shape::Line(s) => s.contains_point(p, tolerance),
            Shape::Ellipse(s) => s.contains_point(p, tolerance),
            Shape::Text(s) => s.contains_point(p, tolerance),
            Shape::Triangle(s) => s.contains_point(p, tolerance),
            Shape::Polygon(s) => s.contains_point(p, tolerance),
            Shape::Gear(s) => s.contains_point(p, tolerance),
            Shape::Sprocket(s) => s.contains_point(p, tolerance),
            Shape::RasterImage(s) => s.contains_point(p, tolerance),
        }
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        match self {
            Shape::Rectangle(s) => s.resize(handle, dx, dy),
            Shape::Circle(s) => s.resize(handle, dx, dy),
            Shape::Path(s) => s.resize(handle, dx, dy),
            Shape::Line(s) => s.resize(handle, dx, dy),
            Shape::Ellipse(s) => s.resize(handle, dx, dy),
            Shape::Text(s) => s.resize(handle, dx, dy),
            Shape::Triangle(s) => s.resize(handle, dx, dy),
            Shape::Polygon(s) => s.resize(handle, dx, dy),
            Shape::Gear(s) => s.resize(handle, dx, dy),
            Shape::Sprocket(s) => s.resize(handle, dx, dy),
            Shape::RasterImage(s) => s.resize(handle, dx, dy),
        }
    }
}

impl Shape {
    pub fn shape_type(&self) -> ShapeType {
        match self {
            Shape::Rectangle(_) => ShapeType::Rectangle,
            Shape::Circle(_) => ShapeType::Circle,
            Shape::Path(_) => ShapeType::Path,
            Shape::Line(_) => ShapeType::Line,
            Shape::Ellipse(_) => ShapeType::Ellipse,
            Shape::Text(_) => ShapeType::Text,
            Shape::Triangle(_) => ShapeType::Triangle,
            Shape::Polygon(_) => ShapeType::Polygon,
            Shape::Gear(_) => ShapeType::Gear,
            Shape::Sprocket(_) => ShapeType::Sprocket,
            Shape::RasterImage(_) => ShapeType::RasterImage,
        }
    }

    /// Returns the rotation angle in degrees
    pub fn rotation(&self) -> f64 {
        match self {
            Shape::Rectangle(s) => s.rotation,
            Shape::Circle(s) => s.rotation,
            Shape::Path(s) => s.rotation,
            Shape::Line(s) => s.rotation,
            Shape::Ellipse(s) => s.rotation,
            Shape::Text(s) => s.rotation,
            Shape::Triangle(s) => s.rotation,
            Shape::Polygon(s) => s.rotation,
            Shape::Gear(s) => s.rotation,
            Shape::Sprocket(s) => s.rotation,
            Shape::RasterImage(s) => s.rotation,
        }
    }

    pub fn as_any(&self) -> &dyn std::any::Any {
        match self {
            Shape::Rectangle(s) => s,
            Shape::Circle(s) => s,
            Shape::Path(s) => s,
            Shape::Line(s) => s,
            Shape::Ellipse(s) => s,
            Shape::Text(s) => s,
            Shape::Triangle(s) => s,
            Shape::Polygon(s) => s,
            Shape::Gear(s) => s,
            Shape::Sprocket(s) => s,
            Shape::RasterImage(s) => s,
        }
    }

    pub fn to_path_shape(&self) -> DesignPath {
        DesignPath {
            sketch: self.as_csg(),
            rotation: match self {
                Shape::Rectangle(s) => s.rotation,
                Shape::Circle(s) => s.rotation,
                Shape::Path(s) => s.rotation,
                Shape::Line(s) => s.rotation,
                Shape::Ellipse(s) => s.rotation,
                Shape::Text(s) => s.rotation,
                Shape::Triangle(s) => s.rotation,
                Shape::Polygon(s) => s.rotation,
                Shape::Gear(s) => s.rotation,
                Shape::Sprocket(s) => s.rotation,
                Shape::RasterImage(s) => s.rotation,
            },
            closed: false,
            original_path: None,
            lock_aspect_ratio: true,
            parametric_source: None,
            laser_params: LaserParams::default(),
        }
    }
}

pub fn rotate_point(p: Point, center: Point, angle_deg: f64) -> Point {
    let angle_rad = angle_deg.to_radians();
    let s = angle_rad.sin();
    let c = angle_rad.cos();
    let dx = p.x - center.x;
    let dy = p.y - center.y;
    Point {
        x: center.x + dx * c - dy * s,
        y: center.y + dx * s + dy * c,
    }
}
