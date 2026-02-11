use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Vector3};

use super::{DesignerShape, Point, Property, PropertyValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignRectangle {
    pub width: f64,
    pub height: f64,
    pub center: Point,
    pub corner_radius: f64,
    /// Rotation angle in degrees (converted to radians for rendering)
    pub rotation: f64,
    pub is_slot: bool,
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

    /// Calculate the effective corner radius based on slot mode
    pub fn effective_corner_radius(&self) -> f64 {
        if self.is_slot {
            // In slot mode, use half of the smaller dimension
            self.width.min(self.height) / 2.0
        } else {
            // Normal mode, use the stored corner_radius
            self.corner_radius
        }
    }
}

impl DesignerShape for DesignRectangle {
    fn render(&self) -> Path {
        let mut builder = Path::builder();
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;
        let x = -half_w;
        let y = -half_h;

        let effective_radius = self.effective_corner_radius();
        if effective_radius > 0.0 {
            builder.add_rounded_rectangle(
                &lyon::math::Box2D::new(
                    point(x as f32, y as f32),
                    point((x + self.width) as f32, (y + self.height) as f32),
                ),
                &lyon::path::builder::BorderRadii::new(effective_radius as f32),
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
        let effective_radius = self.effective_corner_radius();
        let sketch = if effective_radius > 0.0 {
            Sketch::rounded_rectangle(self.width, self.height, effective_radius, 8, None)
        } else {
            Sketch::rectangle(self.width, self.height, None)
        };

        // Sketch::rectangle creates shape from (0,0) to (w,h).
        // We center it at (0,0) first so rotation works around the center.
        let center_fix =
            Matrix4::new_translation(&Vector3::new(-self.width / 2.0, -self.height / 2.0, 0.0));

        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation.to_radians()));
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
        // For now, assume t is translation/rotation/scale (possibly non-uniform).

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

        // Extract scale factors from basis vectors (supports non-uniform scaling)
        // X basis vector: (m11, m12) - determines width scale
        // Y basis vector: (m21, m22) - determines height scale
        let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
        let sy = (t.m21 * t.m21 + t.m22 * t.m22).sqrt() as f64;
        self.width *= sx;
        self.height *= sy;
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
