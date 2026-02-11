use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Vector3};

use super::{DesignerShape, Point, Property, PropertyValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignEllipse {
    pub center: Point,
    pub rx: f64,
    pub ry: f64,
    pub rotation: f64,
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
        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation.to_radians()));
        let translation =
            Matrix4::new_translation(&Vector3::new(self.center.x, self.center.y, 0.0));
        sketch.transform(&(translation * rotation))
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        (
            self.center.x - self.rx,
            self.center.y - self.ry,
            self.center.x + self.rx,
            self.center.y + self.ry,
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
                value: PropertyValue::Number(self.center.x),
            },
            Property {
                name: "Center Y".to_string(),
                value: PropertyValue::Number(self.center.y),
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
