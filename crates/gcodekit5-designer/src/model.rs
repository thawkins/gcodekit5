use lyon::math::{point, Transform};
use lyon::path::iterator::*;
use lyon::path::Path;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

use csgrs::io::svg::{FromSVG, ToSVG};
use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Vector3};

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

use crate::font_manager;
use rusttype::{point as rt_point, Scale};

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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignRectangle {
    pub width: f64,
    pub height: f64,
    pub center: Point,
    pub corner_radius: f64,
    pub rotation: f64,
    pub is_slot: bool,
}

impl DesignerShape for DesignRectangle {
    fn render(&self) -> Path {
        let mut builder = Path::builder();
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;
        let x = -half_w;
        let y = -half_h;

        if self.corner_radius > 0.0 {
            builder.add_rounded_rectangle(
                &lyon::math::Box2D::new(
                    point(x as f32, y as f32),
                    point((x + self.width) as f32, (y + self.height) as f32),
                ),
                &lyon::path::builder::BorderRadii::new(self.corner_radius as f32),
                lyon::path::Winding::Positive,
            );
        } else {
            builder.add_rectangle(
                &lyon::math::Box2D::new(
                    point(x as f32, y as f32),
                    point((x + self.width) as f32, (y + self.height) as f32),
                ),
                lyon::path::Winding::Positive,
            );
        }
        let path = builder.build();

        // Rotate around the shape origin, then translate to its center.
        let mut transform = Transform::identity();
        if self.rotation.abs() > 1e-6 {
            transform = transform
                .then_rotate(lyon::math::Angle::radians(self.rotation.to_radians() as f32));
        }
        transform = transform.then_translate(lyon::math::vector(
            self.center.x as f32,
            self.center.y as f32,
        ));

        path.transformed(&transform)
    }

    fn as_csg(&self) -> Sketch<()> {
        let sketch = if self.corner_radius > 0.0 {
            Sketch::rounded_rectangle(self.width, self.height, self.corner_radius, 8, None)
        } else {
            Sketch::rectangle(self.width, self.height, None)
        };

        // Sketch::rectangle creates shape from (0,0) to (w,h).
        // We center it at (0,0) first so rotation works around the center.
        let center_fix =
            Matrix4::new_translation(&Vector3::new(-self.width / 2.0, -self.height / 2.0, 0.0));

        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation));
        let translation =
            Matrix4::new_translation(&Vector3::new(self.center.x, self.center.y, 0.0));

        sketch.transform(&(translation * rotation * center_fix))
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        // Bounding box of rotated rectangle
        // ... implementation ...
        // For now, return axis aligned of unrotated? No, must be correct.
        // Use render().bounds()?
        let path = self.render();
        let bb = lyon::algorithms::aabb::bounding_box(path.iter());
        (
            bb.min.x as f64,
            bb.min.y as f64,
            bb.max.x as f64,
            bb.max.y as f64,
        )
    }

    fn transform(&mut self, t: &Transform) {
        // This is tricky. If t contains rotation, we update self.rotation.
        // If t contains shear, we can't represent it.
        // For now, assume t is translation/rotation/uniform scale.

        // Transform center
        let p = t.transform_point(point(self.center.x as f32, self.center.y as f32));
        self.center = Point::new(p.x as f64, p.y as f64);

        // Extract rotation and account for reflections (negative determinant flips orientation).
        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        let det = t.m11 * t.m22 - t.m12 * t.m21;
        let mut new_rotation = self.rotation + angle_deg;
        if det < 0.0 {
            new_rotation = -new_rotation;
        }
        self.rotation = new_rotation;

        // Extract uniform scale magnitude from the X basis vector.
        let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
        self.width *= sx;
        self.height *= sx; // Assume uniform scale
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Width".to_string(),
                value: PropertyValue::Number(self.width),
            },
            Property {
                name: "Height".to_string(),
                value: PropertyValue::Number(self.height),
            },
            Property {
                name: "Corner Radius".to_string(),
                value: PropertyValue::Number(self.corner_radius),
            },
            Property {
                name: "Center X".to_string(),
                value: PropertyValue::Number(self.center.x),
            },
            Property {
                name: "Center Y".to_string(),
                value: PropertyValue::Number(self.center.y),
            },
            Property {
                name: "Rotation".to_string(),
                value: PropertyValue::Number(self.rotation),
            },
        ]
    }

    fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        // Check if point is inside or near boundary
        // Rotate point to local coords
        let dx = p.x - self.center.x;
        let dy = p.y - self.center.y;
        let angle = -self.rotation;
        let rx = dx * angle.cos() - dy * angle.sin();
        let ry = dx * angle.sin() + dy * angle.cos();

        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        // Check if inside (including tolerance)
        rx.abs() <= half_w + tolerance && ry.abs() <= half_h + tolerance
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        // Handle 4 is move (center)
        if handle == 4 {
            self.translate(dx, dy);
            return;
        }

        // For resizing, we need to handle rotation.
        // But usually resize handles are axis aligned in local space?
        // Or world space?
        // shapes.rs assumed axis aligned for resize logic (except it rotated corners for rendering).
        // If we resize a rotated rect, we usually resize in local axes.
        // Let's assume local axes for now or simplify.

        // Simplified: ignore rotation for resize calculation, just update width/height/center
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;
        let x1 = self.center.x - half_w;
        let y1 = self.center.y - half_h;
        let x2 = self.center.x + half_w;
        let y2 = self.center.y + half_h;

        let (new_x1, new_y1, new_x2, new_y2) = match handle {
            0 => (x1 + dx, y1 + dy, x2, y2), // Top-left
            1 => (x1, y1 + dy, x2 + dx, y2), // Top-right
            2 => (x1 + dx, y1, x2, y2 + dy), // Bottom-left
            3 => (x1, y1, x2 + dx, y2 + dy), // Bottom-right
            _ => (x1, y1, x2, y2),
        };

        self.width = (new_x2 - new_x1).abs();
        self.height = (new_y2 - new_y1).abs();
        self.center.x = (new_x1 + new_x2) / 2.0;
        self.center.y = (new_y1 + new_y2) / 2.0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignCircle {
    pub radius: f64,
    pub center: Point,
    pub rotation: f64,
}

impl DesignerShape for DesignCircle {
    fn render(&self) -> Path {
        let mut builder = Path::builder();
        builder.add_circle(
            point(self.center.x as f32, self.center.y as f32),
            self.radius as f32,
            lyon::path::Winding::Positive,
        );
        let path = builder.build();
        // Rotation doesn't affect circle geometry but might affect texture/fill if we had it.
        // But for consistency, we should apply it if we want to support "rotating a circle" (e.g. for CAM start point?)
        // shapes.rs Circle::render didn't apply rotation to path?
        // shapes.rs Circle::to_path_shape applied rotation.
        // So we should apply it.
        if self.rotation.abs() > 1e-6 {
            let transform = Transform::translation(self.center.x as f32, self.center.y as f32)
                .then_rotate(lyon::math::Angle::radians(self.rotation as f32))
                .then_translate(lyon::math::vector(
                    -self.center.x as f32,
                    -self.center.y as f32,
                ));
            return path.transformed(&transform);
        }
        path
    }

    fn as_csg(&self) -> Sketch<()> {
        let sketch = Sketch::circle(self.radius, 32, None);
        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation));
        let translation =
            Matrix4::new_translation(&Vector3::new(self.center.x, self.center.y, 0.0));
        sketch.transform(&(translation * rotation))
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        (
            self.center.x - self.radius,
            self.center.y - self.radius,
            self.center.x + self.radius,
            self.center.y + self.radius,
        )
    }

    fn transform(&mut self, t: &Transform) {
        let p = t.transform_point(point(self.center.x as f32, self.center.y as f32));
        self.center = Point::new(p.x as f64, p.y as f64);

        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        let det = t.m11 * t.m22 - t.m12 * t.m21;
        let mut new_rotation = self.rotation + angle_deg;
        if det < 0.0 {
            new_rotation = -new_rotation;
        }
        self.rotation = new_rotation;

        let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
        self.radius *= sx;
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Radius".to_string(),
                value: PropertyValue::Number(self.radius),
            },
            Property {
                name: "Center X".to_string(),
                value: PropertyValue::Number(self.center.x),
            },
            Property {
                name: "Center Y".to_string(),
                value: PropertyValue::Number(self.center.y),
            },
            Property {
                name: "Rotation".to_string(),
                value: PropertyValue::Number(self.rotation),
            },
        ]
    }

    fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        let dx = p.x - self.center.x;
        let dy = p.y - self.center.y;
        let dist = (dx * dx + dy * dy).sqrt();
        dist <= self.radius + tolerance
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        if handle == 4 {
            self.translate(dx, dy);
            return;
        }
        let delta = match handle {
            0 => ((-dx) + (-dy)) / 2.0,
            1 => (dx + (-dy)) / 2.0,
            2 => ((-dx) + dy) / 2.0,
            3 => (dx + dy) / 2.0,
            _ => 0.0,
        };
        self.radius = (self.radius + delta).max(1.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignPath {
    #[serde(
        serialize_with = "serialize_sketch",
        deserialize_with = "deserialize_sketch"
    )]
    pub sketch: Sketch<()>,
    pub rotation: f64,
}

impl DesignPath {
    pub fn from_csg(sketch: Sketch<()>) -> Self {
        Self {
            sketch,
            rotation: 0.0,
        }
    }
}

fn serialize_sketch<S>(sketch: &Sketch<()>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let svg = sketch.to_svg();
    serializer.serialize_str(&svg)
}

fn deserialize_sketch<'de, D>(deserializer: D) -> Result<Sketch<()>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Sketch::from_svg(&s).map_err(serde::de::Error::custom)
}

impl DesignerShape for DesignPath {
    fn render(&self) -> Path {
        let mut builder = Path::builder();

        let mp = self.sketch.to_multipolygon();
        for poly in mp.0 {
            let exterior = poly.exterior();
            let mut first = true;
            for coord in exterior.0.iter() {
                let p = point(coord.x as f32, coord.y as f32);
                if first {
                    builder.begin(p);
                    first = false;
                } else {
                    builder.line_to(p);
                }
            }
            builder.close();

            for interior in poly.interiors() {
                let mut first = true;
                for coord in interior.0.iter() {
                    let p = point(coord.x as f32, coord.y as f32);
                    if first {
                        builder.begin(p);
                        first = false;
                    } else {
                        builder.line_to(p);
                    }
                }
                builder.close();
            }
        }

        builder.build()
    }

    fn as_csg(&self) -> Sketch<()> {
        self.sketch.clone()
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        let bb = CSG::bounding_box(&self.sketch);
        (bb.mins.x, bb.mins.y, bb.maxs.x, bb.maxs.y)
    }

    fn transform(&mut self, t: &Transform) {
        let m = Matrix4::new(
            t.m11 as f64,
            t.m21 as f64,
            0.0,
            t.m31 as f64,
            t.m12 as f64,
            t.m22 as f64,
            0.0,
            t.m32 as f64,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        );

        self.sketch = self.sketch.transform(&m);

        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        self.rotation += angle_deg;
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Type".to_string(),
                value: PropertyValue::String("Path".to_string()),
            },
            Property {
                name: "Rotation".to_string(),
                value: PropertyValue::Number(self.rotation),
            },
        ]
    }

    fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        let (x1, y1, x2, y2) = self.bounds();
        p.x >= x1 - tolerance
            && p.x <= x2 + tolerance
            && p.y >= y1 - tolerance
            && p.y <= y2 + tolerance
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        if handle == 4 {
            self.translate(dx, dy);
            return;
        }
        let (x1, y1, x2, y2) = self.bounds();
        let w = x2 - x1;
        let h = y2 - y1;

        let (new_x1, new_y1, new_x2, new_y2) = match handle {
            0 => (x1 + dx, y1 + dy, x2, y2),
            1 => (x1, y1 + dy, x2 + dx, y2),
            2 => (x1 + dx, y1, x2, y2 + dy),
            3 => (x1, y1, x2 + dx, y2 + dy),
            _ => (x1, y1, x2, y2),
        };

        let new_w = (new_x2 - new_x1).abs();
        let new_h = (new_y2 - new_y1).abs();

        let sx = if w.abs() > 1e-6 { new_w / w } else { 1.0 };
        let sy = if h.abs() > 1e-6 { new_h / h } else { 1.0 };

        let cx = (x1 + x2) / 2.0;
        let cy = (y1 + y2) / 2.0;

        self.scale(sx, sy, Point::new(cx, cy));

        let new_cx = (new_x1 + new_x2) / 2.0;
        let new_cy = (new_y1 + new_y2) / 2.0;
        self.translate(new_cx - cx, new_cy - cy);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignLine {
    pub start: Point,
    pub end: Point,
    pub rotation: f64,
}

impl DesignerShape for DesignLine {
    fn render(&self) -> Path {
        // Apply stored rotation about midpoint for rendering so lines respect rotation field.
        let cx = (self.start.x + self.end.x) / 2.0;
        let cy = (self.start.y + self.end.y) / 2.0;

        let angle_rad = self.rotation.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let rotate_point = |p: &Point| -> Point {
            let dx = p.x - cx;
            let dy = p.y - cy;
            Point::new(dx * cos_a - dy * sin_a + cx, dx * sin_a + dy * cos_a + cy)
        };

        let a = rotate_point(&self.start);
        let b = rotate_point(&self.end);

        let mut builder = Path::builder();
        builder.begin(point(a.x as f32, a.y as f32));
        builder.line_to(point(b.x as f32, b.y as f32));
        builder.end(false);
        builder.build()
    }

    fn as_csg(&self) -> Sketch<()> {
        // Lines have no area, so return empty sketch or degenerate polygon
        // For now, let's return a very thin rectangle to allow selection/boolean ops?
        // Or just a 2-point polygon (which might be invalid for boolean ops)
        // Let's try a thin rectangle (width 0.1mm)
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let len = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);

        let sketch = Sketch::rectangle(len, 0.1, None);

        // Rotate and translate
        let center_x = (self.start.x + self.end.x) / 2.0;
        let center_y = (self.start.y + self.end.y) / 2.0;

        let center_fix = Matrix4::new_translation(&Vector3::new(-len / 2.0, -0.05, 0.0));
        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, angle as f64));
        let translation =
            Matrix4::new_translation(&Vector3::new(center_x as f64, center_y as f64, 0.0));

        sketch.transform(&(translation * rotation * center_fix))
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        let path = self.render();
        let bb = lyon::algorithms::aabb::bounding_box(path.iter());
        (
            bb.min.x as f64,
            bb.min.y as f64,
            bb.max.x as f64,
            bb.max.y as f64,
        )
    }

    fn transform(&mut self, t: &Transform) {
        let p1 = t.transform_point(point(self.start.x as f32, self.start.y as f32));
        self.start = Point::new(p1.x as f64, p1.y as f64);
        let p2 = t.transform_point(point(self.end.x as f32, self.end.y as f32));
        self.end = Point::new(p2.x as f64, p2.y as f64);
        self.rotation = self.current_angle_degrees(); // Update rotation based on new positions
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Start X".to_string(),
                value: PropertyValue::Number(self.start.x as f64),
            },
            Property {
                name: "Start Y".to_string(),
                value: PropertyValue::Number(self.start.y as f64),
            },
            Property {
                name: "End X".to_string(),
                value: PropertyValue::Number(self.end.x as f64),
            },
            Property {
                name: "End Y".to_string(),
                value: PropertyValue::Number(self.end.y as f64),
            },
        ]
    }

    fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        let l2 = (self.end.x - self.start.x).powi(2) + (self.end.y - self.start.y).powi(2);
        if l2 == 0.0 {
            return (p.x - self.start.x).powi(2) + (p.y - self.start.y).powi(2)
                <= tolerance * tolerance;
        }
        let t = ((p.x - self.start.x) * (self.end.x - self.start.x)
            + (p.y - self.start.y) * (self.end.y - self.start.y))
            / l2;
        let t = t.max(0.0).min(1.0);
        let proj_x = self.start.x + t * (self.end.x - self.start.x);
        let proj_y = self.start.y + t * (self.end.y - self.start.y);
        let dist_sq = (p.x - proj_x).powi(2) + (p.y - proj_y).powi(2);
        dist_sq <= tolerance * tolerance
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        match handle {
            0 => {
                self.start.x += dx;
                self.start.y += dy;
            }
            1 => {
                self.end.x += dx;
                self.end.y += dy;
            }
            4 => {
                self.translate(dx, dy);
            }
            _ => {}
        }
    }
}

impl DesignLine {
    pub fn current_angle_degrees(&self) -> f64 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        dy.atan2(dx).to_degrees()
    }

    pub fn rotate_about(&mut self, angle_deg: f64, cx: f64, cy: f64) {
        let angle_rad = angle_deg.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let rotate_point = |p: &Point| -> Point {
            let dx = p.x - cx;
            let dy = p.y - cy;
            Point::new(dx * cos_a - dy * sin_a + cx, dx * sin_a + dy * cos_a + cy)
        };

        self.start = rotate_point(&self.start);
        self.end = rotate_point(&self.end);
        self.rotation = self.current_angle_degrees();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignEllipse {
    pub center: Point,
    pub rx: f64,
    pub ry: f64,
    pub rotation: f64,
}

impl DesignerShape for DesignEllipse {
    fn render(&self) -> Path {
        let mut builder = Path::builder();
        builder.add_ellipse(
            point(self.center.x as f32, self.center.y as f32),
            lyon::math::vector(self.rx as f32, self.ry as f32),
            lyon::math::Angle::radians(self.rotation.to_radians() as f32),
            lyon::path::Winding::Positive,
        );
        builder.build()
    }

    fn as_csg(&self) -> Sketch<()> {
        // Approximate ellipse with polygon
        let steps = 32;
        let mut points = Vec::with_capacity(steps);
        for i in 0..steps {
            let theta = 2.0 * std::f64::consts::PI * (i as f64) / (steps as f64);
            let x = self.rx * theta.cos();
            let y = self.ry * theta.sin();
            points.push([x, y]);
        }

        let sketch = Sketch::polygon(&points, None);
        let translation = Matrix4::new_translation(&Vector3::new(
            self.center.x as f64,
            self.center.y as f64,
            0.0,
        ));
        sketch.transform(&translation)
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        (
            self.center.x as f64 - self.rx,
            self.center.y as f64 - self.ry,
            self.center.x as f64 + self.rx,
            self.center.y as f64 + self.ry,
        )
    }

    fn transform(&mut self, t: &Transform) {
        let p = t.transform_point(point(self.center.x as f32, self.center.y as f32));
        self.center = Point::new(p.x as f64, p.y as f64);

        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        let det = t.m11 * t.m22 - t.m12 * t.m21;
        let mut new_rotation = self.rotation + angle_deg;
        if det < 0.0 {
            new_rotation = -new_rotation;
        }
        self.rotation = new_rotation;

        // Use the X basis vector length as uniform scale factor.
        let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
        self.rx *= sx;
        self.ry *= sx;
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Center X".to_string(),
                value: PropertyValue::Number(self.center.x as f64),
            },
            Property {
                name: "Center Y".to_string(),
                value: PropertyValue::Number(self.center.y as f64),
            },
            Property {
                name: "Radius X".to_string(),
                value: PropertyValue::Number(self.rx),
            },
            Property {
                name: "Radius Y".to_string(),
                value: PropertyValue::Number(self.ry),
            },
        ]
    }

    fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        // Rotate point to local coords
        let dx = p.x - self.center.x;
        let dy = p.y - self.center.y;
        let angle = -self.rotation;
        let rx = dx * angle.cos() - dy * angle.sin();
        let ry = dx * angle.sin() + dy * angle.cos();

        // Check if inside ellipse: (x/a)^2 + (y/b)^2 <= 1
        // We add tolerance to the radii effectively
        let normalized_x = rx / (self.rx + tolerance);
        let normalized_y = ry / (self.ry + tolerance);

        (normalized_x * normalized_x + normalized_y * normalized_y) <= 1.0
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        if handle == 4 {
            self.translate(dx, dy);
            return;
        }
        let (x1, y1, x2, y2) = self.bounds();
        match handle {
            0 => {
                // Top-left
                self.rx = ((self.center.x - (x1 + dx)).abs()).max(1.0);
                self.ry = ((self.center.y - (y1 + dy)).abs()).max(1.0);
            }
            1 => {
                // Top-right
                self.rx = ((self.center.x - (x2 + dx)).abs()).max(1.0);
                self.ry = ((self.center.y - (y1 + dy)).abs()).max(1.0);
            }
            2 => {
                // Bottom-left
                self.rx = ((self.center.x - (x1 + dx)).abs()).max(1.0);
                self.ry = ((self.center.y - (y2 + dy)).abs()).max(1.0);
            }
            3 => {
                // Bottom-right
                self.rx = ((self.center.x - (x2 + dx)).abs()).max(1.0);
                self.ry = ((self.center.y - (y2 + dy)).abs()).max(1.0);
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignText {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub font_size: f64,
    pub font_family: String,
    pub bold: bool,
    pub italic: bool,
    pub rotation: f64,
}

impl DesignerShape for DesignText {
    fn render(&self) -> Path {
        // For rendering, we return a placeholder box for now,
        // or we could implement text rendering to path if we had the logic.
        // shapes.rs uses TextShape::to_path_shape which returns a box.
        let (x1, y1, x2, y2) = self.bounds();
        let mut builder = Path::builder();
        builder.add_rectangle(
            &lyon::math::Box2D::new(point(x1 as f32, y1 as f32), point(x2 as f32, y2 as f32)),
            lyon::path::Winding::Positive,
        );
        let path = builder.build();

        if self.rotation.abs() > 1e-6 {
            let cx = (x1 + x2) / 2.0;
            let cy = (y1 + y2) / 2.0;
            let transform = Transform::translation(cx as f32, cy as f32)
                .then_rotate(lyon::math::Angle::radians(self.rotation.to_radians() as f32))
                .then_translate(lyon::math::vector(-cx as f32, -cy as f32));
            return path.transformed(&transform);
        }
        path
    }

    fn as_csg(&self) -> Sketch<()> {
        // Return bounding box as sketch
        let (x1, y1, x2, y2) = self.bounds();
        let w = x2 - x1;
        let h = y2 - y1;
        let sketch = Sketch::rectangle(w, h, None);
        let cx = x1 + w / 2.0;
        let cy = y1 + h / 2.0;

        let center_fix = Matrix4::new_translation(&Vector3::new(-w / 2.0, -h / 2.0, 0.0));
        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation.to_radians()));
        let translation = Matrix4::new_translation(&Vector3::new(cx, cy, 0.0));

        sketch.transform(&(translation * rotation * center_fix))
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        let font = font_manager::get_font_for(&self.font_family, self.bold, self.italic);
        let scale = Scale::uniform(self.font_size as f32);
        let v_metrics = font.v_metrics(scale);
        let start = rt_point(self.x as f32, self.y as f32 + v_metrics.ascent);
        let glyphs: Vec<_> = font.layout(&self.text, scale, start).collect();

        if glyphs.is_empty() {
            return (self.x, self.y, self.x, self.y + self.font_size);
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for glyph in &glyphs {
            if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                let pos = glyph.position();
                min_x = min_x.min(pos.x + bb.min.x);
                min_y = min_y.min(pos.y + bb.min.y);
                max_x = max_x.max(pos.x + bb.max.x);
                max_y = max_y.max(pos.y + bb.max.y);
            }
        }

        (min_x as f64, min_y as f64, max_x as f64, max_y as f64)
    }

    fn transform(&mut self, t: &Transform) {
        let p = t.transform_point(point(self.x as f32, self.y as f32));
        self.x = p.x as f64;
        self.y = p.y as f64;
        // Scale font size, keeping magnitude when mirroring (negative scale).
        let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt();
        let sy = (t.m21 * t.m21 + t.m22 * t.m22).sqrt();
        let s = ((sx + sy) / 2.0).max(1e-6);
        self.font_size *= s as f64;

        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        let det = t.m11 * t.m22 - t.m12 * t.m21;
        let mut new_rotation = self.rotation + angle_deg;
        if det < 0.0 {
            new_rotation = -new_rotation;
        }
        self.rotation = new_rotation;
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Text".to_string(),
                value: PropertyValue::String(self.text.clone()),
            },
            Property {
                name: "X".to_string(),
                value: PropertyValue::Number(self.x),
            },
            Property {
                name: "Y".to_string(),
                value: PropertyValue::Number(self.y),
            },
            Property {
                name: "Font Size".to_string(),
                value: PropertyValue::Number(self.font_size),
            },
            Property {
                name: "Rotation".to_string(),
                value: PropertyValue::Number(self.rotation),
            },
        ]
    }

    fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        let (x1, y1, x2, y2) = self.bounds();
        p.x >= x1 - tolerance
            && p.x <= x2 + tolerance
            && p.y >= y1 - tolerance
            && p.y <= y2 + tolerance
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        if handle == 4 {
            self.translate(dx, dy);
        }
    }
}

impl DesignRectangle {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            center: Point::new(x + width / 2.0, y + height / 2.0),
            corner_radius: 0.0,
            rotation: 0.0,
            is_slot: false,
        }
    }
}

impl DesignPath {
    pub fn from_svg_path(d: &str) -> Option<Self> {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000" viewBox="0 0 1000 1000"><path d="{}"/></svg>"#,
            d
        );
        if let Ok(sketch) = Sketch::from_svg(&svg) {
            return Some(Self {
                sketch,
                rotation: 0.0,
            });
        }

        let lyon_path = Self::build_lyon_path_from_svg_data(d)?;
        Some(Self::from_lyon_path(&lyon_path))
    }

    /// Build a `lyon::path::Path` from SVG path data.
    ///
    /// This is a fallback for cases where `csgrs::Sketch::from_svg` cannot parse
    /// certain SVG path features. It supports a practical subset of SVG commands:
    /// `m/l/h/v/c/s/q/t/a/z` (and their uppercase forms).
    fn build_lyon_path_from_svg_data(data_str: &str) -> Option<Path> {
        let mut builder = Path::builder();
        let mut current_x = 0.0f32;
        let mut current_y = 0.0f32;
        let mut start_x = 0.0f32;
        let mut start_y = 0.0f32;
        let mut subpath_active = false;

        // Previous control points for smooth commands.
        let mut prev_cubic_ctrl: Option<(f32, f32)> = None;
        let mut prev_quad_ctrl: Option<(f32, f32)> = None;
        let mut prev_cmd: Option<char> = None;

        let tokens = Self::tokenize_svg_path(data_str);
        let mut i = 0usize;

        fn is_cmd_token(s: &str) -> bool {
            s.len() == 1 && s.chars().next().unwrap().is_ascii_alphabetic()
        }

        fn parse_f32(s: &str) -> Option<f32> {
            s.parse::<f32>().ok()
        }

        fn reflect(p: (f32, f32), around: (f32, f32)) -> (f32, f32) {
            (2.0 * around.0 - p.0, 2.0 * around.1 - p.1)
        }

        fn angle_between(u: (f32, f32), v: (f32, f32)) -> f32 {
            let dot = u.0 * v.0 + u.1 * v.1;
            let det = u.0 * v.1 - u.1 * v.0;
            det.atan2(dot)
        }

        fn unit_vector_angle(v: (f32, f32)) -> f32 {
            angle_between((1.0, 0.0), v)
        }

        fn ellipse_transform_point(
            cx: f32,
            cy: f32,
            rx: f32,
            ry: f32,
            cos_phi: f32,
            sin_phi: f32,
            u: f32,
            v: f32,
        ) -> (f32, f32) {
            // [x;y] = [cx;cy] + R(phi) * [rx*u; ry*v]
            let x = cx + cos_phi * (rx * u) - sin_phi * (ry * v);
            let y = cy + sin_phi * (rx * u) + cos_phi * (ry * v);
            (x, y)
        }

        fn arc_to_cubics(
            x1: f32,
            y1: f32,
            x2: f32,
            y2: f32,
            mut rx: f32,
            mut ry: f32,
            phi_deg: f32,
            large_arc: bool,
            sweep: bool,
        ) -> Option<Vec<((f32, f32), (f32, f32), (f32, f32))>> {
            if rx.abs() < f32::EPSILON || ry.abs() < f32::EPSILON {
                return Some(vec![((x1, y1), (x2, y2), (x2, y2))]);
            }

            rx = rx.abs();
            ry = ry.abs();

            let phi = phi_deg.to_radians();
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();

            // Step 1: Compute (x1', y1')
            let dx2 = (x1 - x2) / 2.0;
            let dy2 = (y1 - y2) / 2.0;
            let x1p = cos_phi * dx2 + sin_phi * dy2;
            let y1p = -sin_phi * dx2 + cos_phi * dy2;

            // Step 2: Ensure radii are large enough
            let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
            if lambda > 1.0 {
                let scale = lambda.sqrt();
                rx *= scale;
                ry *= scale;
            }

            // Step 3: Compute (cx', cy')
            let rx2 = rx * rx;
            let ry2 = ry * ry;
            let x1p2 = x1p * x1p;
            let y1p2 = y1p * y1p;
            let denom = rx2 * y1p2 + ry2 * x1p2;
            if denom.abs() < f32::EPSILON {
                return None;
            }

            let mut numer = rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2;
            if numer < 0.0 {
                // Numeric precision; clamp.
                numer = 0.0;
            }

            let sign = if large_arc == sweep { -1.0 } else { 1.0 };
            let coef = sign * (numer / denom).sqrt();
            let cxp = coef * (rx * y1p / ry);
            let cyp = coef * (-ry * x1p / rx);

            // Step 4: Compute (cx, cy)
            let cx = cos_phi * cxp - sin_phi * cyp + (x1 + x2) / 2.0;
            let cy = sin_phi * cxp + cos_phi * cyp + (y1 + y2) / 2.0;

            // Step 5: Angles
            let ux = (x1p - cxp) / rx;
            let uy = (y1p - cyp) / ry;
            let vx = (-x1p - cxp) / rx;
            let vy = (-y1p - cyp) / ry;

            let mut theta1 = unit_vector_angle((ux, uy));
            let mut delta = angle_between((ux, uy), (vx, vy));

            if !sweep && delta > 0.0 {
                delta -= std::f32::consts::TAU;
            } else if sweep && delta < 0.0 {
                delta += std::f32::consts::TAU;
            }

            // Step 6: Split into <= 90deg segments
            let segment_count = (delta.abs() / (std::f32::consts::FRAC_PI_2)).ceil() as i32;
            let segment_count = segment_count.max(1);
            let delta_seg = delta / (segment_count as f32);

            let mut cubics = Vec::with_capacity(segment_count as usize);
            for _ in 0..segment_count {
                let t0 = theta1;
                let t1 = theta1 + delta_seg;
                let dt = t1 - t0;

                let k = 4.0 / 3.0 * (dt / 4.0).tan();

                // Unit circle points
                let (c0, s0) = (t0.cos(), t0.sin());
                let (c1, s1) = (t1.cos(), t1.sin());
                let p0 = (c0, s0);
                let p3 = (c1, s1);
                let p1 = (c0 - k * s0, s0 + k * c0);
                let p2 = (c1 + k * s1, s1 - k * c1);

                // Transform to ellipse
                let cp1 = ellipse_transform_point(cx, cy, rx, ry, cos_phi, sin_phi, p1.0, p1.1);
                let cp2 = ellipse_transform_point(cx, cy, rx, ry, cos_phi, sin_phi, p2.0, p2.1);
                let end = ellipse_transform_point(cx, cy, rx, ry, cos_phi, sin_phi, p3.0, p3.1);
                let _start = ellipse_transform_point(cx, cy, rx, ry, cos_phi, sin_phi, p0.0, p0.1);

                cubics.push((cp1, cp2, end));
                theta1 = t1;
            }

            Some(cubics)
        }

        while i < tokens.len() {
            let cmd_token = &tokens[i];
            if !is_cmd_token(cmd_token) {
                i += 1;
                continue;
            }

            let cmd = cmd_token.chars().next()?;
            let is_relative = cmd.is_ascii_lowercase();
            let cmd_upper = cmd.to_ascii_uppercase();
            i += 1;

            match cmd_upper {
                'M' => {
                    // One or more pairs; first is moveto, rest are implicit lineto.
                    let mut first = true;
                    while i + 1 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x = parse_f32(&tokens[i])?;
                        let y = parse_f32(&tokens[i + 1])?;
                        i += 2;

                        let nx = if is_relative { current_x + x } else { x };
                        let ny = if is_relative { current_y + y } else { y };

                        if first {
                            if subpath_active {
                                builder.end(false);
                            }
                            builder.begin(point(nx, ny));
                            subpath_active = true;
                            start_x = nx;
                            start_y = ny;
                            first = false;
                        } else {
                            if !subpath_active {
                                builder.begin(point(current_x, current_y));
                                subpath_active = true;
                                start_x = current_x;
                                start_y = current_y;
                            }
                            builder.line_to(point(nx, ny));
                        }

                        current_x = nx;
                        current_y = ny;
                    }
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                'L' => {
                    while i + 1 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x = parse_f32(&tokens[i])?;
                        let y = parse_f32(&tokens[i + 1])?;
                        i += 2;

                        let nx = if is_relative { current_x + x } else { x };
                        let ny = if is_relative { current_y + y } else { y };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.line_to(point(nx, ny));
                        current_x = nx;
                        current_y = ny;
                    }
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                'H' => {
                    while i < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x = parse_f32(&tokens[i])?;
                        i += 1;
                        let nx = if is_relative { current_x + x } else { x };
                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.line_to(point(nx, current_y));
                        current_x = nx;
                    }
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                'V' => {
                    while i < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let y = parse_f32(&tokens[i])?;
                        i += 1;
                        let ny = if is_relative { current_y + y } else { y };
                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.line_to(point(current_x, ny));
                        current_y = ny;
                    }
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                'C' => {
                    while i + 5 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x1 = parse_f32(&tokens[i])?;
                        let y1 = parse_f32(&tokens[i + 1])?;
                        let x2 = parse_f32(&tokens[i + 2])?;
                        let y2 = parse_f32(&tokens[i + 3])?;
                        let x = parse_f32(&tokens[i + 4])?;
                        let y = parse_f32(&tokens[i + 5])?;
                        i += 6;

                        let (cp1_x, cp1_y, cp2_x, cp2_y, end_x, end_y) = if is_relative {
                            (
                                current_x + x1,
                                current_y + y1,
                                current_x + x2,
                                current_y + y2,
                                current_x + x,
                                current_y + y,
                            )
                        } else {
                            (x1, y1, x2, y2, x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.cubic_bezier_to(
                            point(cp1_x, cp1_y),
                            point(cp2_x, cp2_y),
                            point(end_x, end_y),
                        );
                        current_x = end_x;
                        current_y = end_y;
                        prev_cubic_ctrl = Some((cp2_x, cp2_y));
                        prev_quad_ctrl = None;
                    }
                }
                'S' => {
                    while i + 3 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x2 = parse_f32(&tokens[i])?;
                        let y2 = parse_f32(&tokens[i + 1])?;
                        let x = parse_f32(&tokens[i + 2])?;
                        let y = parse_f32(&tokens[i + 3])?;
                        i += 4;

                        let cp1 = if matches!(prev_cmd, Some('C' | 'c' | 'S' | 's')) {
                            if let Some(prev) = prev_cubic_ctrl {
                                reflect(prev, (current_x, current_y))
                            } else {
                                (current_x, current_y)
                            }
                        } else {
                            (current_x, current_y)
                        };

                        let (cp2_x, cp2_y, end_x, end_y) = if is_relative {
                            (current_x + x2, current_y + y2, current_x + x, current_y + y)
                        } else {
                            (x2, y2, x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.cubic_bezier_to(
                            point(cp1.0, cp1.1),
                            point(cp2_x, cp2_y),
                            point(end_x, end_y),
                        );
                        current_x = end_x;
                        current_y = end_y;
                        prev_cubic_ctrl = Some((cp2_x, cp2_y));
                        prev_quad_ctrl = None;
                    }
                }
                'Q' => {
                    while i + 3 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x1 = parse_f32(&tokens[i])?;
                        let y1 = parse_f32(&tokens[i + 1])?;
                        let x = parse_f32(&tokens[i + 2])?;
                        let y = parse_f32(&tokens[i + 3])?;
                        i += 4;

                        let (cp_x, cp_y, end_x, end_y) = if is_relative {
                            (current_x + x1, current_y + y1, current_x + x, current_y + y)
                        } else {
                            (x1, y1, x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.quadratic_bezier_to(point(cp_x, cp_y), point(end_x, end_y));
                        current_x = end_x;
                        current_y = end_y;
                        prev_quad_ctrl = Some((cp_x, cp_y));
                        prev_cubic_ctrl = None;
                    }
                }
                'T' => {
                    while i + 1 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x = parse_f32(&tokens[i])?;
                        let y = parse_f32(&tokens[i + 1])?;
                        i += 2;

                        let cp = if matches!(prev_cmd, Some('Q' | 'q' | 'T' | 't')) {
                            if let Some(prev) = prev_quad_ctrl {
                                reflect(prev, (current_x, current_y))
                            } else {
                                (current_x, current_y)
                            }
                        } else {
                            (current_x, current_y)
                        };

                        let (end_x, end_y) = if is_relative {
                            (current_x + x, current_y + y)
                        } else {
                            (x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.quadratic_bezier_to(point(cp.0, cp.1), point(end_x, end_y));
                        current_x = end_x;
                        current_y = end_y;
                        prev_quad_ctrl = Some(cp);
                        prev_cubic_ctrl = None;
                    }
                }
                'A' => {
                    while i + 6 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let rx = parse_f32(&tokens[i])?;
                        let ry = parse_f32(&tokens[i + 1])?;
                        let x_axis_rotation = parse_f32(&tokens[i + 2])?;
                        let large_arc_flag = tokens[i + 3].parse::<i32>().ok()? != 0;
                        let sweep_flag = tokens[i + 4].parse::<i32>().ok()? != 0;
                        let x = parse_f32(&tokens[i + 5])?;
                        let y = parse_f32(&tokens[i + 6])?;
                        i += 7;

                        let (end_x, end_y) = if is_relative {
                            (current_x + x, current_y + y)
                        } else {
                            (x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }

                        if let Some(cubics) = arc_to_cubics(
                            current_x,
                            current_y,
                            end_x,
                            end_y,
                            rx,
                            ry,
                            x_axis_rotation,
                            large_arc_flag,
                            sweep_flag,
                        ) {
                            for (cp1, cp2, end) in cubics {
                                builder.cubic_bezier_to(
                                    point(cp1.0, cp1.1),
                                    point(cp2.0, cp2.1),
                                    point(end.0, end.1),
                                );
                            }
                        } else {
                            builder.line_to(point(end_x, end_y));
                        }

                        current_x = end_x;
                        current_y = end_y;
                        prev_cubic_ctrl = None;
                        prev_quad_ctrl = None;
                    }
                }
                'Z' => {
                    if subpath_active {
                        builder.close();
                        subpath_active = false;
                    }
                    current_x = start_x;
                    current_y = start_y;
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                _ => {
                    // Unsupported command - bail out so caller can treat as unparseable.
                    return None;
                }
            }

            prev_cmd = Some(cmd);
        }

        if subpath_active {
            builder.end(false);
        }

        Some(builder.build())
    }

    /// Tokenize SVG path data into commands and numeric strings.
    ///
    /// This handles commas/whitespace and also splits on `+`/`-` when they begin a
    /// new number (e.g. `10-5` -> `10`, `-5`), while preserving scientific notation.
    fn tokenize_svg_path(path_data: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();

        for ch in path_data.chars() {
            match ch {
                'M' | 'm' | 'L' | 'l' | 'H' | 'h' | 'V' | 'v' | 'C' | 'c' | 'S' | 's' | 'Q'
                | 'q' | 'T' | 't' | 'A' | 'a' | 'Z' | 'z' => {
                    if !current_token.is_empty() {
                        tokens.push(std::mem::take(&mut current_token));
                    }
                    tokens.push(ch.to_string());
                }
                ' ' | ',' | '\n' | '\r' | '\t' => {
                    if !current_token.is_empty() {
                        tokens.push(std::mem::take(&mut current_token));
                    }
                }
                '-' | '+' => {
                    if current_token.is_empty() {
                        current_token.push(ch);
                        continue;
                    }

                    // If the previous char indicates scientific notation, keep the sign.
                    if matches!(current_token.chars().last(), Some('e' | 'E')) {
                        current_token.push(ch);
                    } else {
                        tokens.push(std::mem::take(&mut current_token));
                        current_token.push(ch);
                    }
                }
                _ => current_token.push(ch),
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        tokens
    }
}

impl DesignCircle {
    pub fn new(center: Point, radius: f64) -> Self {
        Self {
            center,
            radius,
            rotation: 0.0,
        }
    }
}

impl DesignLine {
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
            rotation: 0.0,
        }
    }
}

impl DesignEllipse {
    pub fn new(center: Point, rx: f64, ry: f64) -> Self {
        Self {
            center,
            rx,
            ry,
            rotation: 0.0,
        }
    }
}

impl DesignText {
    pub fn new(text: String, x: f64, y: f64, font_size: f64) -> Self {
        Self {
            text,
            x,
            y,
            font_size,
            font_family: "Sans".to_string(),
            bold: false,
            italic: false,
            rotation: 0.0,
        }
    }
}

impl DesignPath {
    pub fn from_points(points: &[Point], _closed: bool) -> Self {
        let pts: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
        let sketch = Sketch::polygon(&pts, None);
        Self {
            sketch,
            rotation: 0.0,
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
        }
    }

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
            },
        }
    }
}

impl DesignPath {
    pub fn from_lyon_path(path: &Path) -> Self {
        let tolerance = 0.1;
        let flattened = path.iter().flattened(tolerance);
        let mut polygons: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut current_poly: Vec<[f64; 2]> = Vec::new();

        for event in flattened {
            match event {
                lyon::path::Event::Begin { at } => {
                    current_poly.clear();
                    current_poly.push([at.x as f64, at.y as f64]);
                }
                lyon::path::Event::Line { to, .. } => {
                    current_poly.push([to.x as f64, to.y as f64]);
                }
                lyon::path::Event::End { .. } => {
                    if !current_poly.is_empty() {
                        polygons.push(current_poly.clone());
                    }
                }
                _ => {}
            }
        }

        if polygons.is_empty() && !current_poly.is_empty() {
            polygons.push(current_poly);
        }

        let mut sketch = Sketch::new();
        for poly in polygons {
            let s = Sketch::polygon(&poly, None);
            sketch = sketch.union(&s);
        }

        Self {
            sketch,
            rotation: 0.0,
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
}

impl DesignPath {
    pub fn to_svg_path(&self) -> String {
        let path = self.render();
        let mut svg = String::new();
        for event in path.iter() {
            match event {
                lyon::path::Event::Begin { at } => svg.push_str(&format!("M {} {} ", at.x, at.y)),
                lyon::path::Event::Line { to, .. } => {
                    svg.push_str(&format!("L {} {} ", to.x, to.y))
                }
                lyon::path::Event::Quadratic { ctrl, to, .. } => {
                    svg.push_str(&format!("Q {} {} {} {} ", ctrl.x, ctrl.y, to.x, to.y))
                }
                lyon::path::Event::Cubic {
                    ctrl1, ctrl2, to, ..
                } => svg.push_str(&format!(
                    "C {} {} {} {} {} {} ",
                    ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y
                )),
                lyon::path::Event::End { close, .. } => {
                    if close {
                        svg.push_str("Z ");
                    }
                }
            }
        }
        svg
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignTriangle {
    pub width: f64,
    pub height: f64,
    pub center: Point,
    pub rotation: f64,
}

impl DesignTriangle {
    pub fn new(center: Point, width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            center,
            rotation: 0.0,
        }
    }
}

impl DesignerShape for DesignTriangle {
    fn render(&self) -> Path {
        let mut builder = Path::builder();
        // Right angle triangle
        // Points relative to center:
        // (-w/2, -h/2) -> (w/2, -h/2) -> (-w/2, h/2)
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        let p1 = point(-half_w as f32, -half_h as f32);
        let p2 = point(half_w as f32, -half_h as f32);
        let p3 = point(-half_w as f32, half_h as f32);

        builder.begin(p1);
        builder.line_to(p2);
        builder.line_to(p3);
        builder.close();

        let path = builder.build();

        // Apply rotation first (around origin), then translate to center
        let mut transform = Transform::identity();
        if self.rotation.abs() > 1e-6 {
            transform = transform
                .then_rotate(lyon::math::Angle::radians(self.rotation.to_radians() as f32));
        }
        transform = transform.then_translate(lyon::math::vector(
            self.center.x as f32,
            self.center.y as f32,
        ));

        path.transformed(&transform)
    }

    fn as_csg(&self) -> Sketch<()> {
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        let points = vec![[-half_w, -half_h], [half_w, -half_h], [-half_w, half_h]];

        let sketch = Sketch::polygon(&points, None);

        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation));
        let translation =
            Matrix4::new_translation(&Vector3::new(self.center.x, self.center.y, 0.0));

        sketch.transform(&(translation * rotation))
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        let path = self.render();
        let bb = lyon::algorithms::aabb::bounding_box(path.iter());
        (
            bb.min.x as f64,
            bb.min.y as f64,
            bb.max.x as f64,
            bb.max.y as f64,
        )
    }

    fn transform(&mut self, t: &Transform) {
        let p = t.transform_point(point(self.center.x as f32, self.center.y as f32));
        self.center = Point::new(p.x as f64, p.y as f64);

        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        let det = t.m11 * t.m22 - t.m12 * t.m21;
        let mut new_rotation = self.rotation + angle_deg;
        if det < 0.0 {
            new_rotation = -new_rotation;
        }
        self.rotation = new_rotation;

        let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
        self.width *= sx;
        self.height *= sx;
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Width".to_string(),
                value: PropertyValue::Number(self.width),
            },
            Property {
                name: "Height".to_string(),
                value: PropertyValue::Number(self.height),
            },
            Property {
                name: "Center X".to_string(),
                value: PropertyValue::Number(self.center.x),
            },
            Property {
                name: "Center Y".to_string(),
                value: PropertyValue::Number(self.center.y),
            },
            Property {
                name: "Rotation".to_string(),
                value: PropertyValue::Number(self.rotation),
            },
        ]
    }

    fn contains_point(&self, p: Point, _tolerance: f64) -> bool {
        // Check if point is inside triangle
        // Transform point to local space
        let dx = p.x - self.center.x;
        let dy = p.y - self.center.y;
        let angle = -self.rotation;
        let rx = dx * angle.cos() - dy * angle.sin();
        let ry = dx * angle.sin() + dy * angle.cos();

        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        // Local points
        let p1 = Point::new(-half_w, -half_h);
        let p2 = Point::new(half_w, -half_h);
        let p3 = Point::new(-half_w, half_h);
        let pt = Point::new(rx, ry);

        // Barycentric coordinates or edge checks
        fn sign(p1: Point, p2: Point, p3: Point) -> f64 {
            (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
        }

        let d1 = sign(pt, p1, p2);
        let d2 = sign(pt, p2, p3);
        let d3 = sign(pt, p3, p1);

        let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
        let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);

        !(has_neg && has_pos)
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        if handle == 4 {
            self.translate(dx, dy);
            return;
        }
        // Simplified resize
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;
        let x1 = self.center.x - half_w;
        let y1 = self.center.y - half_h;
        let x2 = self.center.x + half_w;
        let y2 = self.center.y + half_h;

        let (new_x1, new_y1, new_x2, new_y2) = match handle {
            0 => (x1 + dx, y1 + dy, x2, y2),
            1 => (x1, y1 + dy, x2 + dx, y2),
            2 => (x1 + dx, y1, x2, y2 + dy),
            3 => (x1, y1, x2 + dx, y2 + dy),
            _ => (x1, y1, x2, y2),
        };

        self.width = (new_x2 - new_x1).abs();
        self.height = (new_y2 - new_y1).abs();
        self.center.x = (new_x1 + new_x2) / 2.0;
        self.center.y = (new_y1 + new_y2) / 2.0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignPolygon {
    pub radius: f64,
    pub sides: u32,
    pub center: Point,
    pub rotation: f64,
}

impl DesignPolygon {
    pub fn new(center: Point, radius: f64, sides: u32) -> Self {
        Self {
            radius,
            sides,
            center,
            rotation: 0.0,
        }
    }
}

impl DesignerShape for DesignPolygon {
    fn render(&self) -> Path {
        let mut builder = Path::builder();
        let sides = self.sides.max(3);

        for i in 0..sides {
            let theta = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
            let x = self.radius * theta.cos();
            let y = self.radius * theta.sin();
            let p = point(x as f32, y as f32);

            if i == 0 {
                builder.begin(p);
            } else {
                builder.line_to(p);
            }
        }
        builder.close();

        let path = builder.build();

        // Apply rotation first (around origin), then translate to center
        let mut transform = Transform::identity();
        if self.rotation.abs() > 1e-6 {
            transform = transform
                .then_rotate(lyon::math::Angle::radians(self.rotation.to_radians() as f32));
        }
        transform = transform.then_translate(lyon::math::vector(
            self.center.x as f32,
            self.center.y as f32,
        ));

        path.transformed(&transform)
    }

    fn as_csg(&self) -> Sketch<()> {
        let sides = self.sides.max(3);
        let mut points = Vec::with_capacity(sides as usize);

        for i in 0..sides {
            let theta = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
            let x = self.center.x + self.radius * theta.cos();
            let y = self.center.y + self.radius * theta.sin();
            points.push([x, y]);
        }

        let sketch = Sketch::polygon(&points, None);

        // Apply rotation around center after translation
        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation));
        let center_translate =
            Matrix4::new_translation(&Vector3::new(self.center.x, self.center.y, 0.0));
        let inverse_center =
            Matrix4::new_translation(&Vector3::new(-self.center.x, -self.center.y, 0.0));

        sketch.transform(&(center_translate * rotation * inverse_center))
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        let path = self.render();
        let bb = lyon::algorithms::aabb::bounding_box(path.iter());
        (
            bb.min.x as f64,
            bb.min.y as f64,
            bb.max.x as f64,
            bb.max.y as f64,
        )
    }

    fn transform(&mut self, t: &Transform) {
        let p = t.transform_point(point(self.center.x as f32, self.center.y as f32));
        self.center = Point::new(p.x as f64, p.y as f64);

        let angle = t.m12.atan2(t.m11) as f64;
        self.rotation += angle;

        let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
        self.radius *= sx;
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Radius".to_string(),
                value: PropertyValue::Number(self.radius),
            },
            Property {
                name: "Sides".to_string(),
                value: PropertyValue::Number(self.sides as f64),
            },
            Property {
                name: "Center X".to_string(),
                value: PropertyValue::Number(self.center.x),
            },
            Property {
                name: "Center Y".to_string(),
                value: PropertyValue::Number(self.center.y),
            },
            Property {
                name: "Rotation".to_string(),
                value: PropertyValue::Number(self.rotation),
            },
        ]
    }

    fn contains_point(&self, p: Point, _tolerance: f64) -> bool {
        // Check if point is inside polygon
        // Transform point to local space
        let dx = p.x - self.center.x;
        let dy = p.y - self.center.y;
        let angle = -self.rotation;
        let rx = dx * angle.cos() - dy * angle.sin();
        let ry = dx * angle.sin() + dy * angle.cos();

        // Ray casting algorithm for local polygon
        let sides = self.sides.max(3);
        let mut inside = false;
        let mut j = (sides - 1) as usize;

        for i in 0..sides as usize {
            let theta_i = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
            let xi = self.radius * theta_i.cos();
            let yi = self.radius * theta_i.sin();

            let theta_j = 2.0 * std::f64::consts::PI * (j as f64) / (sides as f64);
            let xj = self.radius * theta_j.cos();
            let yj = self.radius * theta_j.sin();

            if ((yi > ry) != (yj > ry)) && (rx < (xj - xi) * (ry - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }

        inside
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        if handle == 4 {
            self.translate(dx, dy);
            return;
        }
        // Resize radius based on distance from center to handle?
        // Or just use bounding box resize logic?
        // Let's use bounding box logic to scale radius
        let (x1, y1, x2, y2) = self.bounds();
        let w = x2 - x1;
        let h = y2 - y1;

        let (new_x1, new_y1, new_x2, new_y2) = match handle {
            0 => (x1 + dx, y1 + dy, x2, y2),
            1 => (x1, y1 + dy, x2 + dx, y2),
            2 => (x1 + dx, y1, x2, y2 + dy),
            3 => (x1, y1, x2 + dx, y2 + dy),
            _ => (x1, y1, x2, y2),
        };

        let new_w = (new_x2 - new_x1).abs();
        let new_h = (new_y2 - new_y1).abs();

        let sx = if w.abs() > 1e-6 { new_w / w } else { 1.0 };
        let sy = if h.abs() > 1e-6 { new_h / h } else { 1.0 };

        // Uniform scale for radius
        let s = (sx + sy) / 2.0;
        self.radius *= s;

        self.center.x = (new_x1 + new_x2) / 2.0;
        self.center.y = (new_y1 + new_y2) / 2.0;
    }
}
