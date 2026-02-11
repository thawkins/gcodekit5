use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Vector3};
use rusttype::{point as rt_point, Scale};

use super::{DesignerShape, Point, Property, PropertyValue};
use crate::font_manager;

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
