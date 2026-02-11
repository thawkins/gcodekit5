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
            },
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
