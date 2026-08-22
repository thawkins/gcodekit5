//! # Line Shape
//!
//! A line segment design shape defined by start and end points.
//! Supports rotation and CSG sketch generation.

use lyon::math::{point, Transform};
use lyon::path::Path;
use serde::{Deserialize, Serialize};

use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Vector3};

use super::{DesignerShape, LaserParams, Point, Property, PropertyValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignLine {
    pub start: Point,
    pub end: Point,
    pub rotation: f64,
    pub laser_params: LaserParams,
}

impl DesignLine {
    pub fn new(start: Point, end: Point) -> Self {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let rotation = dy.atan2(dx).to_degrees();

        Self {
            start,
            end,
            rotation,
            laser_params: LaserParams::default(),
        }
    }

    pub fn distance_to_point(&self, point: &Point) -> f64 {
        // Vector de start a end
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;

        // If the line is a point
        if dx == 0.0 && dy == 0.0 {
            let dx = point.x - self.start.x;
            let dy = point.y - self.start.y;
            return (dx * dx + dy * dy).sqrt();
        }

        // Projection of the point onto the line
        let t =
            ((point.x - self.start.x) * dx + (point.y - self.start.y) * dy) / (dx * dx + dy * dy);

        // Find the nearest point on the segment
        let t_clamped = t.clamp(0.0, 1.0);
        let proj_x = self.start.x + t_clamped * dx;
        let proj_y = self.start.y + t_clamped * dy;

        // Distance to the projected point
        let dx = point.x - proj_x;
        let dy = point.y - proj_y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn current_angle_degrees(&self) -> f64 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        dy.atan2(dx).to_degrees()
    }

    pub fn rotate_about(&mut self, angle_deg: f64, cx: f64, cy: f64) {
        let angle_rad = angle_deg.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let rotate_point = |p: &Point| -> Point {
            let dx = p.x - cx;
            let dy = p.y - cy;
            Point::new(dx * cos_a - dy * sin_a + cx, dx * sin_a + dy * cos_a + cy)
        };

        self.start = rotate_point(&self.start);
        self.end = rotate_point(&self.end);
        self.rotation = self.current_angle_degrees();
    }

    // Obtener el rectángulo contenedor de la línea SIN aplicar rotación
    // Este es el que se muestra en los entry de dimensiones
    pub fn get_bounds_dimensions(&self) -> (f64, f64, f64, f64) {
        // Usar coordenadas originales sin rotación
        let min_x = self.start.x.min(self.end.x);
        let min_y = self.start.y.min(self.end.y);
        let max_x = self.start.x.max(self.end.x);
        let max_y = self.start.y.max(self.end.y);

        (min_x, min_y, max_x - min_x, max_y - min_y)
    }

    // Establecer las dimensiones del rectángulo contenedor manteniendo la dirección
    pub fn set_bounds_dimensions(&mut self, x: f64, y: f64, width: f64, height: f64) {
        // Guardar el ángulo actual (sin rotación)
        let angle = self.current_angle_degrees().to_radians();

        // Calcular la longitud diagonal del rectángulo
        let diagonal = (width * width + height * height).sqrt();

        // Mantener la longitud de la línea o usar la diagonal del rectángulo
        let current_length = ((self.end.x - self.start.x).powi(2) + (self.end.y - self.start.y).powi(2)).sqrt();
        let length = if current_length > 0.0 { current_length } else { diagonal.max(0.1) };

        // Calcular el centro del rectángulo
        let center_x = x + width / 2.0;
        let center_y = y + height / 2.0;

        // Calcular los nuevos puntos extremos basados en el ángulo actual
        let half_length = length / 2.0;
        let new_start_x = center_x - half_length * angle.cos();
        let new_start_y = center_y - half_length * angle.sin();
        let new_end_x = center_x + half_length * angle.cos();
        let new_end_y = center_y + half_length * angle.sin();

        self.start = Point::new(new_start_x, new_start_y);
        self.end = Point::new(new_end_x, new_end_y);

        // Actualizar la rotación
        self.rotation = self.current_angle_degrees();
    }

    // Redimensionar la línea desde los entry de dimensiones
    pub fn resize_from_bounds(&mut self, new_x: f64, new_y: f64, new_width: f64, new_height: f64) {
        // Calcular el centro actual
        let cx = (self.start.x + self.end.x) / 2.0;
        let cy = (self.start.y + self.end.y) / 2.0;

        // Calcular la longitud actual
        let current_length = ((self.end.x - self.start.x).powi(2) + (self.end.y - self.start.y).powi(2)).sqrt();

        // Calcular la nueva longitud (usando el nuevo ancho y alto)
        let new_diagonal = (new_width * new_width + new_height * new_height).sqrt();
        let new_length = if new_diagonal > 0.0 { new_diagonal } else { current_length };

        // Calcular el nuevo centro (esquina + mitad del ancho/alto)
        let new_cx = new_x + new_width / 2.0;
        let new_cy = new_y + new_height / 2.0;

        // Aplicar escala manteniendo el centro y el ángulo
        let scale = if current_length > 0.0 { new_length / current_length } else { 1.0 };

        if scale != 1.0 {
            // Escalar desde el centro actual
            let scaled_start_x = cx + (self.start.x - cx) * scale;
            let scaled_start_y = cy + (self.start.y - cy) * scale;
            let scaled_end_x = cx + (self.end.x - cx) * scale;
            let scaled_end_y = cy + (self.end.y - cy) * scale;

            self.start = Point::new(scaled_start_x, scaled_start_y);
            self.end = Point::new(scaled_end_x, scaled_end_y);
        }

        // Mover al nuevo centro
        let current_center_x = (self.start.x + self.end.x) / 2.0;
        let current_center_y = (self.start.y + self.end.y) / 2.0;
        let move_dx = new_cx - current_center_x;
        let move_dy = new_cy - current_center_y;
        self.translate(move_dx, move_dy);

        // Asegurar que la línea esté dentro del rectángulo
        let (_, _, current_width, current_height) = self.get_bounds_dimensions();
        if current_width > 0.0 && current_height > 0.0 {
            let scale_x = new_width / current_width;
            let scale_y = new_height / current_height;
            // Usar el menor factor para mantener proporción
            let final_scale = scale_x.min(scale_y);
            if final_scale != 1.0 && final_scale > 0.0 {
                let cx = (self.start.x + self.end.x) / 2.0;
                let cy = (self.start.y + self.end.y) / 2.0;
                self.start.x = cx + (self.start.x - cx) * final_scale;
                self.start.y = cy + (self.start.y - cy) * final_scale;
                self.end.x = cx + (self.end.x - cx) * final_scale;
                self.end.y = cy + (self.end.y - cy) * final_scale;
            }
        }

        // Actualizar rotación
        self.rotation = self.current_angle_degrees();
    }
}

impl DesignerShape for DesignLine {
    fn render(&self) -> Path {
        // Apply stored rotation about midpoint for rendering so lines respect rotation field.
        let cx = (self.start.x + self.end.x) / 2.0;
        let cy = (self.start.y + self.end.y) / 2.0;

        let angle_rad = self.rotation.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let rotate_point = |p: &Point| -> Point {
            let dx = p.x - cx;
            let dy = p.y - cy;
            Point::new(dx * cos_a - dy * sin_a + cx, dx * sin_a + dy * cos_a + cy)
        };

        let a = rotate_point(&self.start);
        let b = rotate_point(&self.end);

        let mut builder = Path::builder();
        builder.begin(point(a.x as f32, a.y as f32));
        builder.line_to(point(b.x as f32, b.y as f32));
        builder.end(false);
        builder.build()
    }

    fn as_csg(&self) -> Sketch<()> {
        // Lines have no area, so return empty sketch or degenerate polygon
        // For now, let's return a very thin rectangle to allow selection/boolean ops?
        // Or just a 2-point polygon (which might be invalid for boolean ops)
        // Let's try a thin rectangle (width 0.1mm)
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let len = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);

        let sketch = Sketch::rectangle(len, 0.1, None);

        // Rotate and translate
        let center_x = (self.start.x + self.end.x) / 2.0;
        let center_y = (self.start.y + self.end.y) / 2.0;

        let center_fix = Matrix4::new_translation(&Vector3::new(-len / 2.0, -0.05, 0.0));
        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, angle));
        let translation = Matrix4::new_translation(&Vector3::new(center_x, center_y, 0.0));

        sketch.transform(&(translation * rotation * center_fix))
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        // Para los bounds, usar las coordenadas originales SIN rotación
        // porque la rotación ya está almacenada como propiedad
        let (x, y, width, height) = self.get_bounds_dimensions();
        (x, y, x + width, y + height)
    }

    fn transform(&mut self, t: &Transform) {
        let p1 = t.transform_point(point(self.start.x as f32, self.start.y as f32));
        self.start = Point::new(p1.x as f64, p1.y as f64);
        let p2 = t.transform_point(point(self.end.x as f32, self.end.y as f32));
        self.end = Point::new(p2.x as f64, p2.y as f64);
        self.rotation = self.current_angle_degrees(); // Update rotation based on new positions
    }

    fn properties(&self) -> Vec<Property> {
        // Obtener las dimensiones del rectángulo contenedor SIN rotación
        let (x, y, width, height) = self.get_bounds_dimensions();

        vec![
            Property {
                name: "X".to_string(),
                value: PropertyValue::Number(x),
            },
            Property {
                name: "Y".to_string(),
                value: PropertyValue::Number(y),
            },
            Property {
                name: "Width".to_string(),
                value: PropertyValue::Number(width),
            },
            Property {
                name: "Height".to_string(),
                value: PropertyValue::Number(height),
            },
            Property {
                name: "Rotation".to_string(),
                value: PropertyValue::Number(self.rotation),
            },
            Property {
                name: "Start X".to_string(),
                value: PropertyValue::Number(self.start.x),
            },
            Property {
                name: "Start Y".to_string(),
                value: PropertyValue::Number(self.start.y),
            },
            Property {
                name: "End X".to_string(),
                value: PropertyValue::Number(self.end.x),
            },
            Property {
                name: "End Y".to_string(),
                value: PropertyValue::Number(self.end.y),
            },
        ]
    }

    fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        // Usar las coordenadas originales sin rotación para la detección de colisión
        let cx = (self.start.x + self.end.x) / 2.0;
        let cy = (self.start.y + self.end.y) / 2.0;

        let angle_rad = self.rotation.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        // Aplicar rotación inversa al punto de prueba para comparar con la línea sin rotar
        let dx = p.x - cx;
        let dy = p.y - cy;
        let rotated_x = dx * cos_a + dy * sin_a + cx;
        let rotated_y = -dx * sin_a + dy * cos_a + cy;
        let rotated_p = Point::new(rotated_x, rotated_y);

        // Calcular distancia usando la versión sin rotación
        let l2 = (self.end.x - self.start.x).powi(2) + (self.end.y - self.start.y).powi(2);
        if l2 == 0.0 {
            return (rotated_p.x - self.start.x).powi(2) + (rotated_p.y - self.start.y).powi(2)
                <= tolerance * tolerance;
        }
        let t = ((rotated_p.x - self.start.x) * (self.end.x - self.start.x)
            + (rotated_p.y - self.start.y) * (self.end.y - self.start.y))
            / l2;
        let t = t.clamp(0.0, 1.0);
        let proj_x = self.start.x + t * (self.end.x - self.start.x);
        let proj_y = self.start.y + t * (self.end.y - self.start.y);
        let dist_sq = (rotated_p.x - proj_x).powi(2) + (rotated_p.y - proj_y).powi(2);
        dist_sq <= tolerance * tolerance
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        match handle {
            0 => { // Esquina superior izquierda - mover inicio
                self.start.x += dx;
                self.start.y += dy;
                self.rotation = self.current_angle_degrees();
            }
            1 => { // Esquina inferior derecha - mover fin
                self.end.x += dx;
                self.end.y += dy;
                self.rotation = self.current_angle_degrees();
            }
            2 => { // Esquina superior derecha - mover inicio
                self.start.x += dx;
                self.start.y += dy;
                self.rotation = self.current_angle_degrees();
            }
            3 => { // Esquina inferior izquierda - mover fin
                self.end.x += dx;
                self.end.y += dy;
                self.rotation = self.current_angle_degrees();
            }
            4 => { // Centro - mover toda la línea
                self.translate(dx, dy);
            }
            _ => {}
        }
    }
}
