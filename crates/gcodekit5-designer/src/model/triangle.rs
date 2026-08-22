//! # Triangle Shape
//!
//! A right isosceles triangle design shape defined by width, height, and center.
//! Supports rotation and CSG boolean operations.

use lyon::math::{point, Point as LyonPoint, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;

use super::{DesignerShape, LaserParams, Point, Property, PropertyValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignTriangle {
    pub width: f64,
    pub height: f64,
    pub center: Point,
    pub rotation: f64,
    pub laser_params: LaserParams,
    #[serde(default)]
    // Especifica la posición del vértice del ángulo recto
    // 0: Inferior-Izquierda, 1: Inferior-Derecha, 2: Superior-Izquierda, 3: Superior-Derecha
    pub right_angle_corner: u8,
}

impl DesignTriangle {
    pub fn new(center: Point, width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            center,
            rotation: 0.0,
            laser_params: LaserParams::default(),
            right_angle_corner: 0,
        }
    }

    pub fn new_with_corner(center: Point, width: f64, height: f64, corner: u8) -> Self {
        Self {
            width,
            height,
            center,
            rotation: 0.0,
            laser_params: LaserParams::default(),
            right_angle_corner: corner.min(3),
        }
    }

    // Obtener los vértices del triángulo en espacio local (sin rotación)
    fn get_local_vertices(&self) -> [LyonPoint; 3] {
        let abs_w = self.width.abs();
        let abs_h = self.height.abs();
        let off_x = abs_w / 2.0;
        let off_y = abs_h / 2.0;

        // Los 4 puntos de las esquinas (sin rotación)
        let corners = [
            point(-off_x as f32, -off_y as f32), // 0: Inferior-Izquierda
            point(off_x as f32, -off_y as f32),  // 1: Inferior-Derecha
            point(-off_x as f32, off_y as f32),  // 2: Superior-Izquierda
            point(off_x as f32, off_y as f32),   // 3: Superior-Derecha
        ];

        let right_angle = corners[self.right_angle_corner as usize];

        let (v1, v2) = match self.right_angle_corner {
            0 => (corners[1], corners[2]),
            1 => (corners[0], corners[3]),
            2 => (corners[0], corners[3]),
            3 => (corners[1], corners[2]),
            _ => (corners[1], corners[2]),
        };

        [right_angle, v1, v2]
    }

    // Obtener los vértices como f64
    pub fn get_local_vertices_f64(&self) -> [Point; 3] {
        let lyon_vertices = self.get_local_vertices();
        [
            Point::new(lyon_vertices[0].x as f64, lyon_vertices[0].y as f64),
            Point::new(lyon_vertices[1].x as f64, lyon_vertices[1].y as f64),
            Point::new(lyon_vertices[2].x as f64, lyon_vertices[2].y as f64),
        ]
    }

    // Obtener los vértices rotados y trasladados
    fn get_rotated_vertices(&self) -> [LyonPoint; 3] {
        let [p1, p2, p3] = self.get_local_vertices();

        // Crear transformación: primero rotación, luego traslación
        let mut transform = Transform::identity();
        if self.rotation.abs() > 1e-6 {
            transform = transform
                .then_rotate(lyon::math::Angle::radians(self.rotation.to_radians() as f32));
        }
        transform = transform.then_translate(lyon::math::vector(
            self.center.x as f32,
            self.center.y as f32,
        ));

        [
            transform.transform_point(p1),
            transform.transform_point(p2),
            transform.transform_point(p3),
        ]
    }

    /// Scale the triangle by sx, sy around a center point
    pub fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        // Escalar las dimensiones (siempre positivas)
        self.width *= sx.abs();
        self.height *= sy.abs();

        // Escalar la posición del centro
        self.center.x = center.x + (self.center.x - center.x) * sx;
        self.center.y = center.y + (self.center.y - center.y) * sy;

        // Actualizar la esquina del ángulo recto si hay reflexión
        // Reflejo en Y (sx < 0): intercambia izquierda ↔ derecha
        if sx < 0.0 {
            self.right_angle_corner = match self.right_angle_corner {
                0 => 1, // Inferior-Izquierda → Inferior-Derecha
                1 => 0, // Inferior-Derecha → Inferior-Izquierda
                2 => 3, // Superior-Izquierda → Superior-Derecha
                3 => 2, // Superior-Derecha → Superior-Izquierda
                _ => self.right_angle_corner,
            };
        }

        // Reflejo en X (sy < 0): intercambia inferior ↔ superior
        if sy < 0.0 {
            self.right_angle_corner = match self.right_angle_corner {
                0 => 2, // Inferior-Izquierda → Superior-Izquierda
                1 => 3, // Inferior-Derecha → Superior-Derecha
                2 => 0, // Superior-Izquierda → Inferior-Izquierda
                3 => 1, // Superior-Derecha → Inferior-Derecha
                _ => self.right_angle_corner,
            };
        }

        // Invertir la rotación si hay reflejo en algún eje
        if sx < 0.0 || sy < 0.0 {
            self.rotation = -self.rotation;
        }
    }
}

impl DesignerShape for DesignTriangle {

    fn render(&self) -> Path {
        let mut builder = Path::builder();

        // Obtener los vértices del triángulo en espacio local
        let [p1, p2, p3] = self.get_local_vertices(); // Usar get_local_vertices

        builder.begin(p1);
        builder.line_to(p2);
        builder.line_to(p3);
        builder.close();

        let path = builder.build();

        // Aplicar rotación y traslación
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
        // Obtener los vértices del triángulo en espacio local (sin rotación ni traslación)
        let [p1, p2, p3] = self.get_local_vertices_f64(); // Usar get_local_vertices_f64

        let points = vec![
            [p1.x, p1.y],
            [p2.x, p2.y],
            [p3.x, p3.y],
        ];

        // Crear el sketch con los vértices locales
        let mut sketch: Sketch<()> = Sketch::polygon(&points, None);

        // Aplicar rotación y traslación
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

        sketch = sketch.transform(&(translation * rotation));
        sketch
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        // Obtener los vértices rotados
        let [p1, p2, p3] = self.get_rotated_vertices();

        // Calcular los bounds a partir de los vértices rotados
        let x_coords = [p1.x, p2.x, p3.x];
        let y_coords = [p1.y, p2.y, p3.y];

        let min_x = x_coords.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_x = x_coords.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min_y = y_coords.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_y = y_coords.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

        (
            min_x as f64,
            min_y as f64,
            max_x as f64,
            max_y as f64,
        )
    }

    fn transform(&mut self, t: &Transform) {
        let p = t.transform_point(point(self.center.x as f32, self.center.y as f32));
        self.center = Point::new(p.x as f64, p.y as f64);

        // Extraer escala para cada eje
        let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
        let sy = (t.m21 * t.m21 + t.m22 * t.m22).sqrt() as f64;

        self.width *= sx;
        self.height *= sy;

        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        self.rotation += angle_deg;
    }

    fn properties(&self) -> Vec<Property> {
        let corner_names = ["Inferior-Izquierda", "Inferior-Derecha", "Superior-Izquierda", "Superior-Derecha"];
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
            Property {
                name: "Right Angle Corner".to_string(),
                value: PropertyValue::String(corner_names[self.right_angle_corner as usize].to_string()),
            },
        ]
    }

    fn contains_point(&self, p: Point, _tolerance: f64) -> bool {
        // Transformar punto al espacio local
        let dx = p.x - self.center.x;
        let dy = p.y - self.center.y;
        let angle = -self.rotation;
        let rx = dx * angle.cos() - dy * angle.sin();
        let ry = dx * angle.sin() + dy * angle.cos();

        let [p1, p2, p3] = self.get_local_vertices_f64();
        let pt = Point::new(rx, ry);

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

    fn scale(&mut self, sx: f64, sy: f64, center: Point) {
        self.scale(sx, sy, center);
    }
}
