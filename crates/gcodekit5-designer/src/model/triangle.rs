use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Vector3};

use super::{DesignerShape, Point, Property, PropertyValue};

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

        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation.to_radians()));
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
