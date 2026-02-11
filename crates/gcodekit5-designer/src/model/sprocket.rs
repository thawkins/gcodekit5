use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use super::{DesignPath, DesignerShape, Point, Property, PropertyValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSprocket {
    pub center: Point,
    pub pitch: f64,
    pub teeth: usize,
    pub roller_diameter: f64,
    pub rotation: f64,
    pub hole_radius: f64,
}

impl DesignSprocket {
    pub fn new(center: Point, pitch: f64, teeth: usize) -> Self {
        Self {
            center,
            pitch,
            teeth,
            roller_diameter: pitch * 0.6,
            rotation: 0.0,
            hole_radius: 0.0,
        }
    }
}

impl DesignerShape for DesignSprocket {
    fn render(&self) -> Path {
        let path = crate::parametric_shapes::generate_sprocket(
            self.center,
            self.pitch,
            self.teeth,
            self.roller_diameter,
            self.hole_radius,
        );

        if self.rotation.abs() > 1e-6 {
            let transform = Transform::translation(self.center.x as f32, self.center.y as f32)
                .then_rotate(lyon::math::Angle::radians(self.rotation.to_radians() as f32))
                .then_translate(lyon::math::vector(
                    -self.center.x as f32,
                    -self.center.y as f32,
                ));
            return path.transformed(&transform);
        }
        path
    }

    fn as_csg(&self) -> csgrs::sketch::Sketch<()> {
        let path = self.render();
        DesignPath::from_lyon_path(&path).as_csg()
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
        self.pitch *= sx;
        self.roller_diameter *= sx;
        self.hole_radius *= sx;
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Pitch".to_string(),
                value: PropertyValue::Number(self.pitch),
            },
            Property {
                name: "Teeth".to_string(),
                value: PropertyValue::Number(self.teeth as f64),
            },
            Property {
                name: "Roller Diameter".to_string(),
                value: PropertyValue::Number(self.roller_diameter),
            },
            Property {
                name: "Hole Radius".to_string(),
                value: PropertyValue::Number(self.hole_radius),
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
