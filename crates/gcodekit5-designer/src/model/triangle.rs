//! # Triangle Shape
//!
//! An isosceles triangle design shape defined by width, height, and center.
//! Supports rotation and CSG boolean operations.

use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
//use nalgebra::{Matrix4, Vector3};

use super::{DesignerShape, Point, Property, PropertyValue, LaserParams};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignTriangle {
    pub width: f64,
    pub height: f64,
    pub center: Point,
    pub rotation: f64,
    pub laser_params: LaserParams,
}

impl DesignTriangle {
    pub fn new(center: Point, width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            center,
            rotation: 0.0,
            laser_params: LaserParams::default(),
        }
    }
}

impl DesignerShape for DesignTriangle {
    fn render(&self) -> Path {
        let mut builder = Path::builder();

        let abs_w = self.width.abs() as f32;
        let abs_h = self.height.abs() as f32;
        let off_x = abs_w / 2.0;
        let off_y = abs_h / 2.0;

        let p1_x = if self.width >= 0.0 { -off_x } else { off_x };
        let p1_y = if self.height >= 0.0 { -off_y } else { off_y };
        let p1 = point(p1_x, p1_y);
        let p2 = point(-p1_x, p1_y);
        let p3 = point(p1_x, -p1_y);

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
        let abs_w = self.width.abs();
        let abs_h = self.height.abs();
        let off_x = abs_w / 2.0;
        let off_y = abs_h / 2.0;

        // Determine position the right angle according to the sign
        let p1_x = if self.width >= 0.0 { -off_x } else { off_x };
        let p1_y = if self.height >= 0.0 { -off_y } else { off_y };

        let points = vec![
            [p1_x, p1_y],  // Right angle
            [-p1_x, p1_y], // Horizontal end
            [p1_x, -p1_y], // Vertical end
        ];

        let sketch: Sketch<()> = Sketch::polygon(&points, None);

        let rotation = nalgebra::Matrix4::new_rotation(nalgebra::Vector3::new(
            0.0,
            0.0,
            self.rotation.to_radians(),
        ));
        let translation = nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(
            self.center.x,
            self.center.y,
            0.0,
        ));

        sketch.transform(&(translation * rotation))
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        // Calculate radius (half the size) always positive.
        let half_w = (self.width / 2.0).abs();
        let half_h = (self.height / 2.0).abs();
        (
            self.center.x - half_w,
            self.center.y - half_h,
            self.center.x + half_w,
            self.center.y + half_h,
        )
    }

    fn transform(&mut self, t: &Transform) {
        let p = t.transform_point(point(self.center.x as f32, self.center.y as f32));
        self.center = Point::new(p.x as f64, p.y as f64);

        // Extract individual scale for each axis
        let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
        let sy = (t.m21 * t.m21 + t.m22 * t.m22).sqrt() as f64;

        self.width *= sx;
        self.height *= sy;

        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        self.rotation += angle_deg;
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
