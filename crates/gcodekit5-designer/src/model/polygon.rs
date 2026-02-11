use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Vector3};

use super::{DesignerShape, Point, Property, PropertyValue};

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
        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation.to_radians()));
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

        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        self.rotation += angle_deg;

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
