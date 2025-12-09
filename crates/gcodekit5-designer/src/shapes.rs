//! Geometric shapes for the designer tool.

use lyon::path::Path;
use lyon::math::point;
use lyon::algorithms::aabb::bounding_box;
use lyon::algorithms::hit_test::hit_test_path;
use lyon::path::FillRule;
use std::any::Any;
use crate::font_manager;
use rusttype::{Scale, point as rt_point};

/// Represents a 2D point with X and Y coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Creates a new point with the given X and Y coordinates.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculates the distance to another point.
    pub fn distance_to(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

pub fn rotate_point(p: Point, center: Point, angle_deg: f64) -> Point {
    if angle_deg.abs() < 1e-6 {
        return p;
    }
    let angle_rad = angle_deg.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let dx = p.x - center.x;
    let dy = p.y - center.y;
    Point {
        x: center.x + dx * cos_a - dy * sin_a,
        y: center.y + dx * sin_a + dy * cos_a
    }
}

/// Types of shapes that can be drawn on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeType {
    Rectangle,
    Circle,
    Line,
    Ellipse,
    Path,
    Text,
}

/// Enum wrapper for all drawable shapes.
#[derive(Debug, Clone)]
pub enum Shape {
    Rectangle(Rectangle),
    Circle(Circle),
    Line(Line),
    Ellipse(Ellipse),
    Path(PathShape),
    Text(TextShape),
}

impl Shape {
    pub fn shape_type(&self) -> ShapeType {
        match self {
            Shape::Rectangle(_) => ShapeType::Rectangle,
            Shape::Circle(_) => ShapeType::Circle,
            Shape::Line(_) => ShapeType::Line,
            Shape::Ellipse(_) => ShapeType::Ellipse,
            Shape::Path(_) => ShapeType::Path,
            Shape::Text(_) => ShapeType::Text,
        }
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        match self {
            Shape::Rectangle(s) => s.bounding_box(),
            Shape::Circle(s) => s.bounding_box(),
            Shape::Line(s) => s.bounding_box(),
            Shape::Ellipse(s) => s.bounding_box(),
            Shape::Path(s) => s.bounding_box(),
            Shape::Text(s) => s.bounding_box(),
        }
    }

    pub fn local_bounding_box(&self) -> (f64, f64, f64, f64) {
        match self {
            Shape::Rectangle(s) => s.local_bounding_box(),
            Shape::Circle(s) => s.local_bounding_box(),
            Shape::Line(s) => s.local_bounding_box(),
            Shape::Ellipse(s) => s.local_bounding_box(),
            Shape::Path(s) => s.local_bounding_box(),
            Shape::Text(s) => s.local_bounding_box(),
        }
    }

    pub fn contains_point(&self, point: &Point, tolerance: f64) -> bool {
        match self {
            Shape::Rectangle(s) => s.contains_point(point, tolerance),
            Shape::Circle(s) => s.contains_point(point, tolerance),
            Shape::Line(s) => s.contains_point(point, tolerance),
            Shape::Ellipse(s) => s.contains_point(point, tolerance),
            Shape::Path(s) => s.contains_point(point, tolerance),
            Shape::Text(s) => s.contains_point(point, tolerance),
        }
    }

    pub fn rotation(&self) -> f64 {
        match self {
            Shape::Rectangle(s) => s.rotation,
            Shape::Circle(s) => s.rotation,
            Shape::Line(s) => s.rotation,
            Shape::Ellipse(s) => s.rotation,
            Shape::Path(s) => s.rotation,
            Shape::Text(s) => s.rotation,
        }
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        match self {
            Shape::Rectangle(s) => s.translate(dx, dy),
            Shape::Circle(s) => s.translate(dx, dy),
            Shape::Line(s) => s.translate(dx, dy),
            Shape::Ellipse(s) => s.translate(dx, dy),
            Shape::Path(s) => s.translate(dx, dy),
            Shape::Text(s) => s.translate(dx, dy),
        }
    }

    pub fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        match self {
            Shape::Rectangle(s) => s.resize(handle, dx, dy),
            Shape::Circle(s) => s.resize(handle, dx, dy),
            Shape::Line(s) => s.resize(handle, dx, dy),
            Shape::Ellipse(s) => s.resize(handle, dx, dy),
            Shape::Path(s) => s.resize(handle, dx, dy),
            Shape::Text(s) => s.resize(handle, dx, dy),
        }
    }

    pub fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        // For Circle -> Ellipse conversion, we might need to change the variant.
        // This is tricky with &mut self if the type changes.
        // We might need to replace `self` with a new variant.
        // Let's handle special cases.
        if let Shape::Circle(c) = self {
             if (sx - sy).abs() > 1e-6 {
                 // Convert to Ellipse
                 let new_center_x = center.x + (c.center.x - center.x) * sx;
                 let new_center_y = center.y + (c.center.y - center.y) * sy;
                 let new_rx = c.radius * sx;
                 let new_ry = c.radius * sy;
                 *self = Shape::Ellipse(Ellipse::new(Point::new(new_center_x, new_center_y), new_rx, new_ry));
                 return;
             }
        }
        
        match self {
            Shape::Rectangle(s) => s.scale(sx, sy, center),
            Shape::Circle(s) => s.scale(sx, sy, center),
            Shape::Line(s) => s.scale(sx, sy, center),
            Shape::Ellipse(s) => s.scale(sx, sy, center),
            Shape::Path(s) => s.scale(sx, sy, center),
            Shape::Text(s) => s.scale(sx, sy, center),
        }
    }

    pub fn rotate(&mut self, angle: f64, cx: f64, cy: f64) {
        match self {
            Shape::Rectangle(s) => s.rotate(angle, cx, cy),
            Shape::Circle(s) => s.rotate(angle, cx, cy),
            Shape::Line(s) => s.rotate(angle, cx, cy),
            Shape::Ellipse(s) => s.rotate(angle, cx, cy),
            Shape::Path(s) => s.rotate(angle, cx, cy),
            Shape::Text(s) => s.rotate(angle, cx, cy),
        }
    }

    pub fn as_any(&self) -> &dyn Any {
        match self {
            Shape::Rectangle(s) => s,
            Shape::Circle(s) => s,
            Shape::Line(s) => s,
            Shape::Ellipse(s) => s,
            Shape::Path(s) => s,
            Shape::Text(s) => s,
        }
    }

    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        match self {
            Shape::Rectangle(s) => s,
            Shape::Circle(s) => s,
            Shape::Line(s) => s,
            Shape::Ellipse(s) => s,
            Shape::Path(s) => s,
            Shape::Text(s) => s,
        }
    }

    pub fn to_path_shape(&self) -> PathShape {
        match self {
            Shape::Rectangle(s) => s.to_path_shape(),
            Shape::Circle(s) => s.to_path_shape(),
            Shape::Line(s) => s.to_path_shape(),
            Shape::Ellipse(s) => s.to_path_shape(),
            Shape::Path(s) => s.clone(),
            Shape::Text(s) => s.to_path_shape(),
        }
    }
}

/// A rectangle defined by its top-left corner and dimensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub corner_radius: f64,
    pub is_slot: bool,
    pub rotation: f64,
}

impl Rectangle {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height, corner_radius: 0.0, is_slot: false, rotation: 0.0 }
    }

    pub fn corners(&self) -> [Point; 4] {
        let center = Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0);
        let corners = [
            Point::new(self.x, self.y),
            Point::new(self.x + self.width, self.y),
            Point::new(self.x + self.width, self.y + self.height),
            Point::new(self.x, self.y + self.height),
        ];
        
        if self.rotation.abs() < 1e-6 {
            return corners;
        }
        
        [
            rotate_point(corners[0], center, self.rotation),
            rotate_point(corners[1], center, self.rotation),
            rotate_point(corners[2], center, self.rotation),
            rotate_point(corners[3], center, self.rotation),
        ]
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        if self.rotation.abs() < 1e-6 {
            return (self.x, self.y, self.x + self.width, self.y + self.height);
        }
        let center = Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0);
        let corners = [
            Point::new(self.x, self.y),
            Point::new(self.x + self.width, self.y),
            Point::new(self.x + self.width, self.y + self.height),
            Point::new(self.x, self.y + self.height),
        ];
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        
        for c in corners {
            let p = rotate_point(c, center, self.rotation);
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
        (min_x, min_y, max_x, max_y)
    }

    pub fn local_bounding_box(&self) -> (f64, f64, f64, f64) {
        (self.x, self.y, self.x + self.width, self.y + self.height)
    }

    pub fn contains_point(&self, point: &Point, tolerance: f64) -> bool {
        let center = Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0);
        let p = rotate_point(*point, center, -self.rotation);
        p.x >= self.x - tolerance
            && p.x <= self.x + self.width + tolerance
            && p.y >= self.y - tolerance
            && p.y <= self.y + self.height + tolerance
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }

    pub fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        let new_width = self.width * sx;
        let new_height = self.height * sy;
        let new_x = center.x + (self.x - center.x) * sx;
        let new_y = center.y + (self.y - center.y) * sy;
        self.x = new_x;
        self.y = new_y;
        self.width = new_width;
        self.height = new_height;
        
        // Re-constrain radius if dimensions shrink
        let max_radius = self.width.min(self.height).abs() / 2.0;
        if self.is_slot {
            self.corner_radius = max_radius;
        } else {
            self.corner_radius = self.corner_radius.min(max_radius);
        }
    }

    pub fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        let (x1, y1, x2, y2) = (self.x, self.y, self.x + self.width, self.y + self.height);
        let (new_x1, new_y1, new_x2, new_y2) = match handle {
            0 => (x1 + dx, y1 + dy, x2, y2),           // Top-left
            1 => (x1, y1 + dy, x2 + dx, y2),           // Top-right
            2 => (x1 + dx, y1, x2, y2 + dy),           // Bottom-left
            3 => (x1, y1, x2 + dx, y2 + dy),           // Bottom-right
            4 => (x1 + dx, y1 + dy, x2 + dx, y2 + dy), // Center (move)
            _ => (x1, y1, x2, y2),
        };

        self.width = (new_x2 - new_x1).abs();
        self.height = (new_y2 - new_y1).abs();
        self.x = new_x1.min(new_x2);
        self.y = new_y1.min(new_y2);
        
        // Re-constrain radius
        let max_radius = self.width.min(self.height) / 2.0;
        if self.is_slot {
            self.corner_radius = max_radius;
        } else {
            self.corner_radius = self.corner_radius.min(max_radius);
        }
    }

    pub fn rotate(&mut self, angle: f64, cx: f64, cy: f64) {
        let center = Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0);
        let new_center = rotate_point(center, Point::new(cx, cy), angle);
        self.x = new_center.x - self.width / 2.0;
        self.y = new_center.y - self.height / 2.0;
        self.rotation += angle;
    }

    pub fn to_path_shape(&self) -> PathShape {
        let mut builder = Path::builder();
        if self.corner_radius > 0.0 {
            let r = self.corner_radius as f32;
            let x = self.x as f32;
            let y = self.y as f32;
            let w = self.width as f32;
            let h = self.height as f32;
            
            builder.begin(point(x + r, y));
            builder.line_to(point(x + w - r, y));
            builder.quadratic_bezier_to(point(x + w, y), point(x + w, y + r));
            builder.line_to(point(x + w, y + h - r));
            builder.quadratic_bezier_to(point(x + w, y + h), point(x + w - r, y + h));
            builder.line_to(point(x + r, y + h));
            builder.quadratic_bezier_to(point(x, y + h), point(x, y + h - r));
            builder.line_to(point(x, y + r));
            builder.quadratic_bezier_to(point(x, y), point(x + r, y));
            builder.close();
        } else {
            builder.begin(point(self.x as f32, self.y as f32));
            builder.line_to(point((self.x + self.width) as f32, self.y as f32));
            builder.line_to(point((self.x + self.width) as f32, (self.y + self.height) as f32));
            builder.line_to(point(self.x as f32, (self.y + self.height) as f32));
            builder.close();
        }
        PathShape { path: builder.build(), rotation: self.rotation }
    }
}

/// A circle defined by its center and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: Point,
    pub radius: f64,
    pub rotation: f64,
}

impl Circle {
    pub fn new(center: Point, radius: f64) -> Self {
        Self { center, radius, rotation: 0.0 }
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        (
            self.center.x - self.radius,
            self.center.y - self.radius,
            self.center.x + self.radius,
            self.center.y + self.radius,
        )
    }

    pub fn local_bounding_box(&self) -> (f64, f64, f64, f64) {
        self.bounding_box()
    }

    pub fn contains_point(&self, point: &Point, tolerance: f64) -> bool {
        self.center.distance_to(point) <= self.radius + tolerance
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.center.x += dx;
        self.center.y += dy;
    }

    pub fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        // Note: Uniform scaling only. Non-uniform scaling should convert to Ellipse in Shape::scale
        let new_center_x = center.x + (self.center.x - center.x) * sx;
        let new_center_y = center.y + (self.center.y - center.y) * sy;
        self.center.x = new_center_x;
        self.center.y = new_center_y;
        self.radius *= sx; // Assume uniform for now
    }

    pub fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        match handle {
            0 | 1 | 2 | 3 => {
                // Adjust radius
                // Simplified logic: just take average delta
                let delta = match handle {
                    0 => ((-dx) + (-dy)) / 2.0,
                    1 => (dx + (-dy)) / 2.0,
                    2 => ((-dx) + dy) / 2.0,
                    3 => (dx + dy) / 2.0,
                    _ => 0.0,
                };
                self.radius = (self.radius + delta).max(5.0);
            }
            4 => {
                self.center.x += dx;
                self.center.y += dy;
            }
            _ => {}
        }
    }

    pub fn rotate(&mut self, angle: f64, cx: f64, cy: f64) {
        self.center = rotate_point(self.center, Point::new(cx, cy), angle);
        self.rotation += angle;
    }

    pub fn to_path_shape(&self) -> PathShape {
        let mut builder = Path::builder();
        builder.add_circle(
            point(self.center.x as f32, self.center.y as f32),
            self.radius as f32,
            lyon::path::Winding::Positive,
        );
        PathShape { path: builder.build(), rotation: self.rotation }
    }
}

/// A line defined by two endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub start: Point,
    pub end: Point,
    pub rotation: f64,
}

impl Line {
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end, rotation: 0.0 }
    }

    pub fn length(&self) -> f64 {
        self.start.distance_to(&self.end)
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        if self.rotation.abs() < 1e-6 {
            return (
                self.start.x.min(self.end.x),
                self.start.y.min(self.end.y),
                self.start.x.max(self.end.x),
                self.start.y.max(self.end.y),
            );
        }
        
        let center = Point::new(
            (self.start.x + self.end.x) / 2.0,
            (self.start.y + self.end.y) / 2.0,
        );
        
        let p1 = rotate_point(self.start, center, self.rotation);
        let p2 = rotate_point(self.end, center, self.rotation);
        
        (
            p1.x.min(p2.x),
            p1.y.min(p2.y),
            p1.x.max(p2.x),
            p1.y.max(p2.y),
        )
    }

    pub fn local_bounding_box(&self) -> (f64, f64, f64, f64) {
        (
            self.start.x.min(self.end.x),
            self.start.y.min(self.end.y),
            self.start.x.max(self.end.x),
            self.start.y.max(self.end.y),
        )
    }

    pub fn contains_point(&self, point: &Point, tolerance: f64) -> bool {
        let dist_to_start = self.start.distance_to(point);
        let dist_to_end = self.end.distance_to(point);
        let line_length = self.length();

        (dist_to_start + dist_to_end - line_length).abs() <= tolerance
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.start.x += dx;
        self.start.y += dy;
        self.end.x += dx;
        self.end.y += dy;
    }

    pub fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        self.start.x = center.x + (self.start.x - center.x) * sx;
        self.start.y = center.y + (self.start.y - center.y) * sy;
        self.end.x = center.x + (self.end.x - center.x) * sx;
        self.end.y = center.y + (self.end.y - center.y) * sy;
    }

    pub fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        match handle {
            0 => { // Start
                self.start.x += dx;
                self.start.y += dy;
            }
            1 => { // End
                self.end.x += dx;
                self.end.y += dy;
            }
            4 => { // Move
                self.translate(dx, dy);
            }
            _ => {}
        }
    }

    pub fn rotate(&mut self, angle: f64, cx: f64, cy: f64) {
        self.start = rotate_point(self.start, Point::new(cx, cy), angle);
        self.end = rotate_point(self.end, Point::new(cx, cy), angle);
        self.rotation += angle;
    }

    pub fn to_path_shape(&self) -> PathShape {
        let mut builder = Path::builder();
        builder.begin(point(self.start.x as f32, self.start.y as f32));
        builder.line_to(point(self.end.x as f32, self.end.y as f32));
        builder.end(false);
        PathShape { path: builder.build(), rotation: self.rotation }
    }
}

/// An ellipse defined by its center, horizontal radius, and vertical radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipse {
    pub center: Point,
    pub rx: f64,
    pub ry: f64,
    pub rotation: f64,
}

impl Ellipse {
    pub fn new(center: Point, rx: f64, ry: f64) -> Self {
        Self { center, rx, ry, rotation: 0.0 }
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        if self.rotation.abs() < 1e-6 {
            return (
                self.center.x - self.rx,
                self.center.y - self.ry,
                self.center.x + self.rx,
                self.center.y + self.ry,
            );
        }
        
        // For rotated ellipse, we calculate the bounding box of the rotated shape
        // Parametric equation:
        // x = cx + rx*cos(t)*cos(rot) - ry*sin(t)*sin(rot)
        // y = cy + rx*cos(t)*sin(rot) + ry*sin(t)*cos(rot)
        // To find min/max x/y, we differentiate and solve for t.
        // Or we can just rotate the 4 extreme points? No, extreme points change.
        // But we can approximate by rotating the unrotated bounding box corners? No, that's loose.
        
        // Exact solution:
        // Max x occurs when dx/dt = 0.
        // tan(t) = -(ry*sin(rot)) / (rx*cos(rot)) = - (ry/rx) * tan(rot)
        // This is getting complicated.
        // Let's use the 4 corners of the unrotated bounding box and rotate them.
        // This gives the bounding box of the rotated bounding box, which is a loose bound but safe.
        // Actually, for Group Center calculation, we want the center to be stable.
        // The center of the rotated bounding box of an ellipse IS the center of the ellipse.
        // So (cx, cy) is invariant!
        // But the EXTENTS change.
        // If we use loose bounds, the center is still (cx, cy).
        // So for Ellipse, the center is stable regardless of rotation.
        // But for consistency with other shapes (like Rectangle), we should return the rotated bounds.
        
        let rot_rad = self.rotation.to_radians();
        let cos_r = rot_rad.cos();
        let sin_r = rot_rad.sin();
        
        let ux = (self.rx * cos_r).powi(2) + (self.ry * sin_r).powi(2);
        let uy = (self.rx * sin_r).powi(2) + (self.ry * cos_r).powi(2);
        
        let w_half = ux.sqrt();
        let h_half = uy.sqrt();
        
        (
            self.center.x - w_half,
            self.center.y - h_half,
            self.center.x + w_half,
            self.center.y + h_half,
        )
    }

    pub fn local_bounding_box(&self) -> (f64, f64, f64, f64) {
        (
            self.center.x - self.rx,
            self.center.y - self.ry,
            self.center.x + self.rx,
            self.center.y + self.ry,
        )
    }

    pub fn contains_point(&self, point: &Point, tolerance: f64) -> bool {
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        let rx = self.rx + tolerance;
        let ry = self.ry + tolerance;
        (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry) <= 1.0
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.center.x += dx;
        self.center.y += dy;
    }

    pub fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        self.center.x = center.x + (self.center.x - center.x) * sx;
        self.center.y = center.y + (self.center.y - center.y) * sy;
        self.rx *= sx;
        self.ry *= sy;
    }

    pub fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        let (x1, y1, x2, y2) = self.bounding_box();
        match handle {
            0 => { // Top-left
                self.rx = ((self.center.x - (x1 + dx)) / 1.0).abs().max(5.0);
                self.ry = ((self.center.y - (y1 + dy)) / 1.0).abs().max(5.0);
            }
            1 => { // Top-right
                self.rx = ((self.center.x - (x2 + dx)) / 1.0).abs().max(5.0);
                self.ry = ((self.center.y - (y1 + dy)) / 1.0).abs().max(5.0);
            }
            2 => { // Bottom-left
                self.rx = ((self.center.x - (x1 + dx)) / 1.0).abs().max(5.0);
                self.ry = ((self.center.y - (y2 + dy)) / 1.0).abs().max(5.0);
            }
            3 => { // Bottom-right
                self.rx = ((self.center.x - (x2 + dx)) / 1.0).abs().max(5.0);
                self.ry = ((self.center.y - (y2 + dy)) / 1.0).abs().max(5.0);
            }
            4 => { // Center
                self.translate(dx, dy);
            }
            _ => {}
        }
    }

    pub fn rotate(&mut self, angle: f64, cx: f64, cy: f64) {
        self.center = rotate_point(self.center, Point::new(cx, cy), angle);
        self.rotation += angle;
    }

    pub fn to_path_shape(&self) -> PathShape {
        let mut builder = Path::builder();
        builder.add_ellipse(
            point(self.center.x as f32, self.center.y as f32),
            lyon::math::vector(self.rx as f32, self.ry as f32),
            lyon::math::Angle::radians(0.0),
            lyon::path::Winding::Positive,
        );
        PathShape { path: builder.build(), rotation: self.rotation }
    }
}

/// A generic path shape wrapping lyon::path::Path
#[derive(Debug, Clone)]
pub struct PathShape {
    pub path: Path,
    pub rotation: f64,
}

impl PathShape {
    pub fn new(path: Path) -> Self {
        Self { path, rotation: 0.0 }
    }

    pub fn from_points(points: &[Point], closed: bool) -> Self {
        let mut builder = Path::builder();
        if let Some(first) = points.first() {
            builder.begin(point(first.x as f32, first.y as f32));
            for p in points.iter().skip(1) {
                builder.line_to(point(p.x as f32, p.y as f32));
            }
            if closed {
                builder.close();
            } else {
                builder.end(false);
            }
        }
        Self { path: builder.build(), rotation: 0.0 }
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        let aabb = bounding_box(self.path.iter());
        let (x1, y1, x2, y2) = (aabb.min.x as f64, aabb.min.y as f64, aabb.max.x as f64, aabb.max.y as f64);
        
        if self.rotation.abs() < 1e-6 {
            return (x1, y1, x2, y2);
        }
        
        let center_x = (x1 + x2) / 2.0;
        let center_y = (y1 + y2) / 2.0;
        let center = Point::new(center_x, center_y);
        
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        
        for event in self.path.iter() {
            match event {
                lyon::path::Event::Begin { at } => {
                    let p = rotate_point(Point::new(at.x as f64, at.y as f64), center, self.rotation);
                    min_x = min_x.min(p.x); min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x); max_y = max_y.max(p.y);
                }
                lyon::path::Event::Line { to, .. } => {
                    let p = rotate_point(Point::new(to.x as f64, to.y as f64), center, self.rotation);
                    min_x = min_x.min(p.x); min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x); max_y = max_y.max(p.y);
                }
                lyon::path::Event::Quadratic { ctrl, to, .. } => {
                    let p1 = rotate_point(Point::new(ctrl.x as f64, ctrl.y as f64), center, self.rotation);
                    let p2 = rotate_point(Point::new(to.x as f64, to.y as f64), center, self.rotation);
                    min_x = min_x.min(p1.x).min(p2.x); min_y = min_y.min(p1.y).min(p2.y);
                    max_x = max_x.max(p1.x).max(p2.x); max_y = max_y.max(p1.y).max(p2.y);
                }
                lyon::path::Event::Cubic { ctrl1, ctrl2, to, .. } => {
                    let p1 = rotate_point(Point::new(ctrl1.x as f64, ctrl1.y as f64), center, self.rotation);
                    let p2 = rotate_point(Point::new(ctrl2.x as f64, ctrl2.y as f64), center, self.rotation);
                    let p3 = rotate_point(Point::new(to.x as f64, to.y as f64), center, self.rotation);
                    min_x = min_x.min(p1.x).min(p2.x).min(p3.x); 
                    min_y = min_y.min(p1.y).min(p2.y).min(p3.y);
                    max_x = max_x.max(p1.x).max(p2.x).max(p3.x); 
                    max_y = max_y.max(p1.y).max(p2.y).max(p3.y);
                }
                _ => {}
            }
        }
        
        // If path is empty or invalid, return unrotated box
        if min_x == f64::INFINITY {
            return (x1, y1, x2, y2);
        }
        
        (min_x, min_y, max_x, max_y)
    }

    pub fn local_bounding_box(&self) -> (f64, f64, f64, f64) {
        let aabb = bounding_box(self.path.iter());
        (aabb.min.x as f64, aabb.min.y as f64, aabb.max.x as f64, aabb.max.y as f64)
    }

    pub fn contains_point(&self, p: &Point, tolerance: f64) -> bool {
        hit_test_path(
            &point(p.x as f32, p.y as f32),
            self.path.iter(),
            FillRule::NonZero,
            tolerance as f32
        )
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        let mut builder = Path::builder();
        for event in self.path.iter() {
            match event {
                lyon::path::Event::Begin { at } => {
                    builder.begin(point(at.x + dx as f32, at.y + dy as f32));
                }
                lyon::path::Event::Line { from: _, to } => {
                    builder.line_to(point(to.x + dx as f32, to.y + dy as f32));
                }
                lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                    builder.quadratic_bezier_to(
                        point(ctrl.x + dx as f32, ctrl.y + dy as f32),
                        point(to.x + dx as f32, to.y + dy as f32),
                    );
                }
                lyon::path::Event::Cubic { from: _, ctrl1, ctrl2, to } => {
                    builder.cubic_bezier_to(
                        point(ctrl1.x + dx as f32, ctrl1.y + dy as f32),
                        point(ctrl2.x + dx as f32, ctrl2.y + dy as f32),
                        point(to.x + dx as f32, to.y + dy as f32),
                    );
                }
                lyon::path::Event::End { last: _, first: _, close } => {
                    if close {
                        builder.close();
                    } else {
                        builder.end(false);
                    }
                }
            }
        }
        self.path = builder.build();
    }

    pub fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        let mut builder = Path::builder();
        let transform = |p: lyon::math::Point| -> lyon::math::Point {
            let x = center.x + (p.x as f64 - center.x) * sx;
            let y = center.y + (p.y as f64 - center.y) * sy;
            point(x as f32, y as f32)
        };

        for event in self.path.iter() {
            match event {
                lyon::path::Event::Begin { at } => {
                    builder.begin(transform(at));
                }
                lyon::path::Event::Line { from: _, to } => {
                    builder.line_to(transform(to));
                }
                lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                    builder.quadratic_bezier_to(transform(ctrl), transform(to));
                }
                lyon::path::Event::Cubic { from: _, ctrl1, ctrl2, to } => {
                    builder.cubic_bezier_to(transform(ctrl1), transform(ctrl2), transform(to));
                }
                lyon::path::Event::End { last: _, first: _, close } => {
                    if close {
                        builder.close();
                    } else {
                        builder.end(false);
                    }
                }
            }
        }
        self.path = builder.build();
    }

    pub fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        if handle == 4 {
            self.translate(dx, dy);
            return;
        }

        let (x1, y1, x2, y2) = self.bounding_box();
        let (new_x1, new_y1, new_x2, new_y2) = match handle {
            0 => (x1 + dx, y1 + dy, x2, y2), // Top-left
            1 => (x1, y1 + dy, x2 + dx, y2), // Top-right
            2 => (x1 + dx, y1, x2, y2 + dy), // Bottom-left
            3 => (x1, y1, x2 + dx, y2 + dy), // Bottom-right
            _ => (x1, y1, x2, y2),
        };
        let width = x2 - x1;
        let height = y2 - y1;
        let new_width = (new_x2 - new_x1).abs();
        let new_height = (new_y2 - new_y1).abs();

        let sx = if width.abs() > 1e-6 {
            new_width / width
        } else {
            1.0
        };
        let sy = if height.abs() > 1e-6 {
            new_height / height
        } else {
            1.0
        };

        let center_x = (x1 + x2) / 2.0;
        let center_y = (y1 + y2) / 2.0;

        self.scale(sx, sy, Point::new(center_x, center_y));

        let (final_x1, final_y1, final_x2, final_y2) = self.bounding_box();
        let final_center_x = (final_x1 + final_x2) / 2.0;
        let final_center_y = (final_y1 + final_y2) / 2.0;
        
        let target_center_x = (new_x1 + new_x2) / 2.0;
        let target_center_y = (new_y1 + new_y2) / 2.0;
        
        let t_dx = target_center_x - final_center_x;
        let t_dy = target_center_y - final_center_y;

        self.translate(t_dx, t_dy);
    }

    pub fn rotate(&mut self, angle: f64, cx: f64, cy: f64) {
        let mut builder = Path::builder();
        for event in self.path.iter() {
            match event {
                lyon::path::Event::Begin { at } => {
                    let p = rotate_point(Point::new(at.x as f64, at.y as f64), Point::new(cx, cy), angle);
                    builder.begin(point(p.x as f32, p.y as f32));
                },
                lyon::path::Event::Line { from: _, to } => {
                    let p = rotate_point(Point::new(to.x as f64, to.y as f64), Point::new(cx, cy), angle);
                    builder.line_to(point(p.x as f32, p.y as f32));
                },
                lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                    let c = rotate_point(Point::new(ctrl.x as f64, ctrl.y as f64), Point::new(cx, cy), angle);
                    let p = rotate_point(Point::new(to.x as f64, to.y as f64), Point::new(cx, cy), angle);
                    builder.quadratic_bezier_to(point(c.x as f32, c.y as f32), point(p.x as f32, p.y as f32));
                },
                lyon::path::Event::Cubic { from: _, ctrl1, ctrl2, to } => {
                    let c1 = rotate_point(Point::new(ctrl1.x as f64, ctrl1.y as f64), Point::new(cx, cy), angle);
                    let c2 = rotate_point(Point::new(ctrl2.x as f64, ctrl2.y as f64), Point::new(cx, cy), angle);
                    let p = rotate_point(Point::new(to.x as f64, to.y as f64), Point::new(cx, cy), angle);
                    builder.cubic_bezier_to(point(c1.x as f32, c1.y as f32), point(c2.x as f32, c2.y as f32), point(p.x as f32, p.y as f32));
                },
                lyon::path::Event::End { last: _, first: _, close } => {
                    if close {
                        builder.close();
                    } else {
                        builder.end(false);
                    }
                },
            }
        }
        self.path = builder.build();
        self.rotation += angle;
    }
    
    // SVG path helpers kept as is
    pub fn to_svg_path(&self) -> String {
        let mut path_str = String::new();
        for event in self.path.iter() {
            match event {
                lyon::path::Event::Begin { at } => {
                    path_str.push_str(&format!("M {} {} ", at.x, at.y));
                }
                lyon::path::Event::Line { from: _, to } => {
                    path_str.push_str(&format!("L {} {} ", to.x, to.y));
                }
                lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                    path_str.push_str(&format!("Q {} {} {} {} ", ctrl.x, ctrl.y, to.x, to.y));
                }
                lyon::path::Event::Cubic { from: _, ctrl1, ctrl2, to } => {
                    path_str.push_str(&format!("C {} {} {} {} {} {} ", ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y));
                }
                lyon::path::Event::End { last: _, first: _, close } => {
                    if close {
                        path_str.push_str("Z ");
                    }
                }
            }
        }
        path_str
    }

    pub fn from_svg_path(data_str: &str) -> Option<Self> {
        let mut builder = Path::builder();
        let mut current_x = 0.0f32;
        let mut current_y = 0.0f32;
        let mut start_x = 0.0f32;
        let mut start_y = 0.0f32;
        let mut subpath_active = false;

        let commands = Self::tokenize_svg_path(data_str);
        let mut i = 0;

        while i < commands.len() {
            let cmd = &commands[i];

            match cmd.as_str() {
                "M" | "m" => {
                    if i + 2 < commands.len() {
                        let x: f32 = commands[i + 1].parse().unwrap_or(0.0);
                        let y: f32 = commands[i + 2].parse().unwrap_or(0.0);

                        if cmd == "m" {
                            current_x += x;
                            current_y += y;
                        } else {
                            current_x = x;
                            current_y = y;
                        }
                        
                        if subpath_active {
                            builder.end(false);
                        }
                        
                        start_x = current_x;
                        start_y = current_y;
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
                "L" | "l" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    let mut j = i + 1;
                    while j + 1 < commands.len() {
                        let x: f32 = commands[j].parse().unwrap_or(0.0);
                        let y: f32 = commands[j + 1].parse().unwrap_or(0.0);

                        if cmd == "l" {
                            current_x += x;
                            current_y += y;
                        } else {
                            current_x = x;
                            current_y = y;
                        }

                        builder.line_to(point(current_x, current_y));
                        j += 2;

                        if j < commands.len() {
                            let next = &commands[j];
                            if next.len() == 1 && next.chars().all(|c| c.is_alphabetic()) {
                                break;
                            } else if next.parse::<f32>().is_err() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    i = j;
                }
                "H" | "h" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    if i + 1 < commands.len() {
                        let x: f32 = commands[i + 1].parse().unwrap_or(0.0);
                        if cmd == "h" {
                            current_x += x;
                        } else {
                            current_x = x;
                        }
                        builder.line_to(point(current_x, current_y));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "V" | "v" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    if i + 1 < commands.len() {
                        let y: f32 = commands[i + 1].parse().unwrap_or(0.0);
                        if cmd == "v" {
                            current_y += y;
                        } else {
                            current_y = y;
                        }
                        builder.line_to(point(current_x, current_y));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "C" | "c" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    let mut j = i + 1;
                    while j + 5 < commands.len() {
                        let x1: f32 = commands[j].parse().unwrap_or(0.0);
                        let y1: f32 = commands[j + 1].parse().unwrap_or(0.0);
                        let x2: f32 = commands[j + 2].parse().unwrap_or(0.0);
                        let y2: f32 = commands[j + 3].parse().unwrap_or(0.0);
                        let x: f32 = commands[j + 4].parse().unwrap_or(0.0);
                        let y: f32 = commands[j + 5].parse().unwrap_or(0.0);

                        let mut cp1_x = x1;
                        let mut cp1_y = y1;
                        let mut cp2_x = x2;
                        let mut cp2_y = y2;
                        let mut end_x = x;
                        let mut end_y = y;

                        if cmd == "c" {
                            cp1_x += current_x;
                            cp1_y += current_y;
                            cp2_x += current_x;
                            cp2_y += current_y;
                            end_x += current_x;
                            end_y += current_y;
                        }

                        builder.cubic_bezier_to(
                            point(cp1_x, cp1_y),
                            point(cp2_x, cp2_y),
                            point(end_x, end_y)
                        );

                        current_x = end_x;
                        current_y = end_y;
                        j += 6;

                        if j < commands.len() {
                            let next = &commands[j];
                            if next.len() == 1 && next.chars().all(|c| c.is_alphabetic()) {
                                break;
                            } else if next.parse::<f32>().is_err() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    i = j;
                }
                "Q" | "q" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    let mut j = i + 1;
                    while j + 3 < commands.len() {
                        let x1: f32 = commands[j].parse().unwrap_or(0.0);
                        let y1: f32 = commands[j + 1].parse().unwrap_or(0.0);
                        let x: f32 = commands[j + 2].parse().unwrap_or(0.0);
                        let y: f32 = commands[j + 3].parse().unwrap_or(0.0);

                        let mut cp_x = x1;
                        let mut cp_y = y1;
                        let mut end_x = x;
                        let mut end_y = y;

                        if cmd == "q" {
                            cp_x += current_x;
                            cp_y += current_y;
                            end_x += current_x;
                            end_y += current_y;
                        }

                        builder.quadratic_bezier_to(
                            point(cp_x, cp_y),
                            point(end_x, end_y)
                        );

                        current_x = end_x;
                        current_y = end_y;
                        j += 4;

                        if j < commands.len() {
                            let next = &commands[j];
                            if next.len() == 1 && next.chars().all(|c| c.is_alphabetic()) {
                                break;
                            } else if next.parse::<f32>().is_err() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    i = j;
                }
                "Z" | "z" => {
                    if subpath_active {
                        builder.close();
                        subpath_active = false;
                    }
                    current_x = start_x;
                    current_y = start_y;
                    i += 1;
                }
                _ => i += 1,
            }
        }
        
        if subpath_active {
            builder.end(false);
        }
        Some(Self { path: builder.build(), rotation: 0.0 })
    }

    fn tokenize_svg_path(path_data: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();

        for ch in path_data.chars() {
            match ch {
                'M' | 'm' | 'L' | 'l' | 'H' | 'h' | 'V' | 'v' | 'C' | 'c' | 'S' | 's' | 'Q'
                | 'q' | 'T' | 't' | 'A' | 'a' | 'Z' | 'z' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    tokens.push(ch.to_string());
                }
                ' ' | ',' | '\n' | '\r' | '\t' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
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

#[derive(Debug, Clone)]
pub struct TextShape {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub font_size: f64,
    pub rotation: f64,
}

impl TextShape {
    pub fn new(text: String, x: f64, y: f64, font_size: f64) -> Self {
        Self {
            text,
            x,
            y,
            font_size,
            rotation: 0.0,
        }
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        let font = font_manager::get_font();
        let scale = Scale::uniform(self.font_size as f32);
        let v_metrics = font.v_metrics(scale);
        
        let start = rt_point(self.x as f32, self.y as f32 + v_metrics.ascent);
        
        let glyphs: Vec<_> = font.layout(&self.text, scale, start).collect();
        
        let (min_x, min_y, max_x, max_y) = if glyphs.is_empty() {
             (self.x, self.y, self.x, self.y + self.font_size)
        } else {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            
            let mut has_bounds = false;

            for glyph in &glyphs {
                if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                    let pos = glyph.position();
                    min_x = min_x.min(pos.x + bb.min.x);
                    min_y = min_y.min(pos.y + bb.min.y);
                    max_x = max_x.max(pos.x + bb.max.x);
                    max_y = max_y.max(pos.y + bb.max.y);
                    has_bounds = true;
                }
            }
            
            if !has_bounds {
                 let width = self.text.len() as f64 * self.font_size * 0.6;
                 (self.x, self.y, self.x + width, self.y + self.font_size)
            } else {
                (min_x as f64, min_y as f64, max_x as f64, max_y as f64)
            }
        };
        
        if self.rotation.abs() < 1e-6 {
            return (min_x, min_y, max_x, max_y);
        }
        
        let center = Point::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        let corners = [
            Point::new(min_x, min_y),
            Point::new(max_x, min_y),
            Point::new(max_x, max_y),
            Point::new(min_x, max_y),
        ];
        
        let mut r_min_x = f64::INFINITY;
        let mut r_min_y = f64::INFINITY;
        let mut r_max_x = f64::NEG_INFINITY;
        let mut r_max_y = f64::NEG_INFINITY;
        
        for c in corners {
            let p = rotate_point(c, center, self.rotation);
            r_min_x = r_min_x.min(p.x);
            r_min_y = r_min_y.min(p.y);
            r_max_x = r_max_x.max(p.x);
            r_max_y = r_max_y.max(p.y);
        }
        (r_min_x, r_min_y, r_max_x, r_max_y)
    }

    pub fn local_bounding_box(&self) -> (f64, f64, f64, f64) {
        let font = font_manager::get_font();
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
        
        let mut has_bounds = false;

        for glyph in &glyphs {
            if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                let pos = glyph.position();
                min_x = min_x.min(pos.x + bb.min.x);
                min_y = min_y.min(pos.y + bb.min.y);
                max_x = max_x.max(pos.x + bb.max.x);
                max_y = max_y.max(pos.y + bb.max.y);
                has_bounds = true;
            }
        }
        
        if !has_bounds {
             let width = self.text.len() as f64 * self.font_size * 0.6;
             (self.x, self.y, self.x + width, self.y + self.font_size)
        } else {
            (min_x as f64, min_y as f64, max_x as f64, max_y as f64)
        }
    }

    pub fn contains_point(&self, point: &Point, tolerance: f64) -> bool {
        // For hit testing, we need unrotated bounding box
        // So we rotate point backwards around center of unrotated box
        // But we need to calculate unrotated box first.
        // This is inefficient to do every time.
        // But for now it's fine.
        
        // Duplicate logic to get unrotated bounds
        let font = font_manager::get_font();
        let scale = Scale::uniform(self.font_size as f32);
        let v_metrics = font.v_metrics(scale);
        let start = rt_point(self.x as f32, self.y as f32 + v_metrics.ascent);
        let glyphs: Vec<_> = font.layout(&self.text, scale, start).collect();
        
        let (min_x, min_y, max_x, max_y) = if glyphs.is_empty() {
             (self.x, self.y, self.x, self.y + self.font_size)
        } else {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            let mut has_bounds = false;
            for glyph in &glyphs {
                if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                    let pos = glyph.position();
                    min_x = min_x.min(pos.x + bb.min.x);
                    min_y = min_y.min(pos.y + bb.min.y);
                    max_x = max_x.max(pos.x + bb.max.x);
                    max_y = max_y.max(pos.y + bb.max.y);
                    has_bounds = true;
                }
            }
            if !has_bounds {
                 let width = self.text.len() as f64 * self.font_size * 0.6;
                 (self.x, self.y, self.x + width, self.y + self.font_size)
            } else {
                (min_x as f64, min_y as f64, max_x as f64, max_y as f64)
            }
        };
        
        let center = Point::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        let p = rotate_point(*point, center, -self.rotation);
        p.x >= min_x - tolerance && p.x <= max_x + tolerance && p.y >= min_y - tolerance && p.y <= max_y + tolerance
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }

    pub fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        let new_x = center.x + (self.x - center.x) * sx;
        let new_y = center.y + (self.y - center.y) * sy;
        let avg_scale = (sx + sy) / 2.0;
        self.font_size *= avg_scale;
        self.x = new_x;
        self.y = new_y;
    }

    pub fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        if handle == 4 {
            self.translate(dx, dy);
        }
    }

    pub fn rotate(&mut self, angle: f64, cx: f64, cy: f64) {
        let p = Point::new(self.x, self.y);
        let new_p = rotate_point(p, Point::new(cx, cy), angle);
        self.x = new_p.x;
        self.y = new_p.y;
        self.rotation += angle;
    }

    pub fn to_path_shape(&self) -> PathShape {
        let (x1, y1, x2, y2) = self.bounding_box();
        let rect = Rectangle::new(x1, y1, x2 - x1, y2 - y1);
        let mut path = rect.to_path_shape();
        path.rotation = self.rotation;
        path
    }
}

/// Type of CAM operation to perform on the shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Profile,
    Pocket,
}

impl Default for OperationType {
    fn default() -> Self {
        Self::Profile
    }
}
