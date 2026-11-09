//! # Roller Chain Sprocket Shape
//!
//! A parametric roller chain sprocket defined by pitch, tooth count,
//! and roller diameter. Generates accurate tooth profiles following
//! standard sprocket geometry.

use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use super::{DesignPath, DesignerShape, LaserParams, Point, Property, PropertyValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSprocket {
    pub center: Point,
    pub pitch: f64,
    pub teeth: usize,
    pub roller_diameter: f64,
    pub rotation: f64,
    pub hole_radius: f64,
    pub laser_params: LaserParams,
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
            laser_params: LaserParams::default(),
        }
    }

    /// Devuelve el diámetro de rodillo estándar para un paso dado (en mm)
    /// Basado en norma ANSI/ISO para cadenas de rodillos
    pub fn standard_roller_diameter(pitch_mm: f64) -> f64 {
        // Tolerancia pequeña para comparar con floats
        const EPS: f64 = 0.01;

        match pitch_mm {
            p if (p - 6.35).abs() < EPS => 4.0,     // 25-1 (04C-1)
            p if (p - 8.0).abs() < EPS => 5.0,      // 35-1 (06C-1)
            p if (p - 9.525).abs() < EPS => 6.35,   // 40-1 (08A-1)
            p if (p - 12.7).abs() < EPS => 8.51,    // 50-1 (10A-1)
            p if (p - 15.875).abs() < EPS => 10.16, // 60-1 (12A-1)
            p if (p - 19.05).abs() < EPS => 12.07,  // 80-1 (16A-1)
            p if (p - 25.4).abs() < EPS => 15.88,   // 100-1 (20A-1)
            p if (p - 31.75).abs() < EPS => 19.05,  // 120-1 (24A-1)
            p if (p - 38.1).abs() < EPS => 25.4,    // 140-1 (28A-1)
            p if (p - 44.45).abs() < EPS => 27.94,  // 160-1 (32A-1)
            p if (p - 50.8).abs() < EPS => 31.75,   // 200-1 (40A-1)
            _ => pitch_mm * 0.625,                  // Valor por defecto para pasos personalizados
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
