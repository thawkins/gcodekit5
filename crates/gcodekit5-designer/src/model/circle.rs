use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Vector3};

use super::{DesignerShape, Point, Property, PropertyValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignCircle {
    pub radius: f64,
    pub center: Point,
    pub rotation: f64,
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
        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation.to_radians()));
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
