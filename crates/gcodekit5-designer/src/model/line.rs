use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Vector3};

use super::{DesignerShape, Point, Property, PropertyValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignLine {
    pub start: Point,
    pub end: Point,
    pub rotation: f64,
}

impl DesignLine {
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
            rotation: 0.0,
        }
    }

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
        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, angle));
        let translation = Matrix4::new_translation(&Vector3::new(center_x, center_y, 0.0));

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
                value: PropertyValue::Number(self.start.x),
            },
            Property {
                name: "Start Y".to_string(),
                value: PropertyValue::Number(self.start.y),
            },
            Property {
                name: "End X".to_string(),
                value: PropertyValue::Number(self.end.x),
            },
            Property {
                name: "End Y".to_string(),
                value: PropertyValue::Number(self.end.y),
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
        let t = t.clamp(0.0, 1.0);
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
