//! Image Engraving Tool - Generates continuous G-code for laser engraving

use anyhow::{Context, Result};
use image::{DynamicImage, GrayImage};
use std::path::Path;

/// Rotation angles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationAngle {
    Degrees0,
    Degrees90,
    Degrees180,
    Degrees270,
}

/// Scanning direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanDirection {
    Horizontal,
    Vertical,
}

/// Halftoning method
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HalftoneMethod {
    None,
    Threshold,
    Bayer4x4,
    FloydSteinberg,
    Atkinson,
}

/// Structure containing the engraving parameters
#[derive(Debug, Clone)]
pub struct EngravingParams {
    pub width_mm: f32,
    pub height_mm: Option<f32>,
    pub feed_rate: f32,
    pub travel_rate: f32,
    pub min_power: f32,
    pub max_power: f32,
    pub ppi: f32,
    pub scan_direction: ScanDirection,
    pub bidirectional: bool,
    pub invert: bool,
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub rotation: RotationAngle,
    pub halftone: HalftoneMethod,
    pub halftone_threshold: u8,
    pub offset_x: f32,
    pub offset_y: f32,
    pub power_scale: f32,
    pub line_spacing: f32,
}

impl Default for EngravingParams {
    fn default() -> Self {
        Self {
            width_mm: 100.0,
            height_mm: None,
            feed_rate: 1000.0,
            travel_rate: 3000.0,
            min_power: 0.0,
            max_power: 100.0,
            ppi: 254.0,
            scan_direction: ScanDirection::Horizontal,
            bidirectional: true,
            invert: false,
            mirror_x: false,
            mirror_y: false,
            rotation: RotationAngle::Degrees0,
            halftone: HalftoneMethod::None,
            halftone_threshold: 127,
            offset_x: 10.0,
            offset_y: 10.0,
            power_scale: 1000.0,
            line_spacing: 1.0,
        }
    }
}

pub const OVERSCAN_MM: f32 = 2.5;

impl EngravingParams {
    /// Convert PPI to pixels per mm
    pub fn pixels_per_mm(&self) -> f32 {
        self.ppi / 25.4
    }
}

/// G-code generator for images
pub struct ImageEngraver {
    image: GrayImage,
    params: EngravingParams,
    #[allow(dead_code)]
    output_width: u32,
    #[allow(dead_code)]
    output_height: u32,
}

impl ImageEngraver {

    /// Check if a horizontal line (row) has any pixel that will produce laser power
    fn is_row_empty(&self, y: u32) -> bool {
        let result = (0..self.image.width()).all(|x| {
            let intensity = self.image.get_pixel(x, y).0[0];
            let power = self.intensity_to_power(intensity);
            power == 0
        });
        result
    }

    /// Get only rows that have content to engrave (actual laser power > 0)
    fn get_non_empty_rows(&self) -> Vec<u32> {
        let rows: Vec<u32> = (0..self.image.height())
            .rev()
            .filter(|&y| {
                let is_empty = self.is_row_empty(y);

                !is_empty
            })
            .collect();
        rows
    }

    fn get_non_empty_columns(&self) -> Vec<u32> {
        (0..self.image.width())
            .rev()  // <- Añade esto para invertir el orden (derecha a izquierda)
            .filter(|&x| !self.is_column_empty(x))
            .collect()
    }

    fn is_column_empty(&self, x: u32) -> bool {
        (0..self.image.height()).all(|y| {
            let intensity = self.image.get_pixel(x, y).0[0];
            let power = self.intensity_to_power(intensity);
            power == 0
        })
    }

    /// Create a new engraver from a file
    pub fn from_file<P: AsRef<Path>>(path: P, params: EngravingParams) -> Result<Self> {
        let img = image::open(path.as_ref()).context("Failed to load image file")?;
        Self::from_image(img, params)
    }

    /// Create a new engraver from a designer RasterImage
    pub fn from_raster_image(
        image: &crate::model::RasterImage,
        params: EngravingParams,
    ) -> Result<Self> {
        let path = image
            .original_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Image has no original path"))?;

        let img = image::open(path).context("Failed to load image file")?;
        Self::from_image(img, params)
    }

    pub fn from_image(img: DynamicImage, params: EngravingParams) -> Result<Self> {
        // Componer sobre fondo blanco si la imagen tiene canal alfa
        let img = if img.color().has_alpha() {

            // Componer la imagen original sobre fondo blanco
            let mut composed = img.to_rgba8();
            for pixel in composed.pixels_mut() {
                let alpha = pixel[3] as f32 / 255.0;
                if alpha < 1.0 {
                    // Mezclar con blanco (255,255,255)
                    pixel[0] = ((pixel[0] as f32 * alpha) + (255.0 * (1.0 - alpha))) as u8;
                    pixel[1] = ((pixel[1] as f32 * alpha) + (255.0 * (1.0 - alpha))) as u8;
                    pixel[2] = ((pixel[2] as f32 * alpha) + (255.0 * (1.0 - alpha))) as u8;
                }
                pixel[3] = 255; // Canal alfa opaco después de componer
            }
            DynamicImage::ImageRgba8(composed)
        } else {
            img
        };

        let mut gray = img.to_luma8();

        // Aplicar transformaciones
        if params.mirror_x {
            image::imageops::flip_horizontal_in_place(&mut gray);
        }
        if params.mirror_y {
            image::imageops::flip_vertical_in_place(&mut gray);
        }

        if params.rotation != RotationAngle::Degrees0 {
            gray = match params.rotation {
                RotationAngle::Degrees90 => image::imageops::rotate90(&gray),
                RotationAngle::Degrees180 => image::imageops::rotate180(&gray),
                RotationAngle::Degrees270 => image::imageops::rotate270(&gray),
                RotationAngle::Degrees0 => gray,
            };
        }

        let width = gray.width();
        let height = gray.height();
        let output_width = (params.width_mm * params.pixels_per_mm()) as u32;
        let aspect_ratio = height as f32 / width as f32;
        let output_height = params
            .height_mm
            .map(|h| (h * params.pixels_per_mm()) as u32)
            .unwrap_or((output_width as f32 * aspect_ratio) as u32);

        gray = image::imageops::resize(
            &gray,
            output_width,
            output_height,
            image::imageops::FilterType::Lanczos3,
        );

        if params.invert {
            image::imageops::invert(&mut gray);
        }

        if params.halftone != HalftoneMethod::None {
            Self::apply_halftoning(&mut gray, &params)?;
        }

        Ok(Self {
            image: gray,
            params,
            output_width,
            output_height,
        })
    }

    /// Get the output dimensions in millimeters
    pub fn output_size_mm(&self) -> (f32, f32) {
        (
            self.output_width as f32 / self.params.pixels_per_mm(),
            self.output_height as f32 / self.params.pixels_per_mm(),
        )
    }

    fn apply_halftoning(image: &mut GrayImage, params: &EngravingParams) -> Result<()> {
        let dot_size = 1;

        if dot_size > 1 {
        } else {
            match params.halftone {
                HalftoneMethod::Threshold => Self::threshold(image, params.halftone_threshold),
                HalftoneMethod::Bayer4x4 => Self::bayer_dither(image),
                HalftoneMethod::FloydSteinberg => Self::floyd_steinberg(image),
                HalftoneMethod::Atkinson => Self::atkinson(image),
                HalftoneMethod::None => {}
            }
        }
        Ok(())
    }

    fn threshold(image: &mut GrayImage, threshold: u8) {
        for pixel in image.pixels_mut() {
            pixel.0[0] = if pixel.0[0] >= threshold { 255 } else { 0 };
        }
    }

    fn bayer_dither(image: &mut GrayImage) {
        let bayer = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
        for y in 0..image.height() {
            for x in 0..image.width() {
                let val = image.get_pixel(x, y).0[0];
                let threshold = (bayer[(y % 4) as usize][(x % 4) as usize] * 16 + 8) as u8;
                image.put_pixel(x, y, image::Luma([if val >= threshold { 255 } else { 0 }]));
            }
        }
    }

    fn floyd_steinberg(image: &mut GrayImage) {
        let width = image.width();
        let height = image.height();
        let mut buffer: Vec<i16> = image.as_raw().iter().map(|&p| p as i16).collect();

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let old = buffer[idx];
                let new = if old > 127 { 255 } else { 0 };
                buffer[idx] = new;
                let err = old - new;

                if x + 1 < width {
                    buffer[(y * width + x + 1) as usize] += err * 7 / 16;
                }
                if x > 0 && y + 1 < height {
                    buffer[((y + 1) * width + x - 1) as usize] += err * 3 / 16;
                }
                if y + 1 < height {
                    buffer[((y + 1) * width + x) as usize] += err * 5 / 16;
                }
                if x + 1 < width && y + 1 < height {
                    buffer[((y + 1) * width + x + 1) as usize] += err / 16;
                }
            }
        }

        for (i, &val) in buffer.iter().enumerate() {
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            image.put_pixel(x, y, image::Luma([val.clamp(0, 255) as u8]));
        }
    }

    fn atkinson(image: &mut GrayImage) {
        let width = image.width();
        let height = image.height();
        let mut buffer: Vec<i16> = image.as_raw().iter().map(|&p| p as i16).collect();

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let old = buffer[idx];
                let new = if old > 127 { 255 } else { 0 };
                buffer[idx] = new;
                let err = old - new;

                let neighbors = [(1, 0), (2, 0), (-1, 1), (0, 1), (1, 1), (0, 2)];
                for (dx, dy) in neighbors {
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                        buffer[(ny as u32 * width + nx as u32) as usize] += err / 8;
                    }
                }
            }
        }

        for (i, &val) in buffer.iter().enumerate() {
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            image.put_pixel(x, y, image::Luma([val.clamp(0, 255) as u8]));
        }
    }

    fn intensity_to_power(&self, intensity: u8) -> u32 {
        let normalized = intensity as f32 / 255.0;
        let gamma = 0.7;
        let corrected = normalized.powf(gamma);
        let power =
            self.params.min_power + (corrected * (self.params.max_power - self.params.min_power));
        let raw_power = ((power * self.params.power_scale / 100.0) as u32).clamp(0, 1000);
        self.quantize_power(raw_power)
    }

    fn quantize_power(&self, power: u32) -> u32 {
        if self.params.halftone != HalftoneMethod::None {
            return power;
        }

        // Reduce command churn by quantizing grayscale power levels.
        // This keeps tonal shading while avoiding one-pixel power changes.
        const LEVELS: u32 = 64;
        let step = (1000 + LEVELS / 2) / LEVELS; // ~16
        let quantized = ((power + step / 2) / step) * step;
        quantized.min(1000)
    }

    /// Generate G-code
    pub fn generate_gcode(&self) -> Result<String> {
        let mut gcode = String::new();

        gcode.push_str("; GcodeKit5 Image Engraving\n");
        gcode.push_str(&format!("; Feed Rate: {} \n", self.params.feed_rate,));
        gcode.push_str(&format!("; Max Power: {}% \n", self.params.max_power,));
        gcode.push_str("G90 G17 G40 G21\n"); // Absolute coordinates, XY Plane, Radius compensation off, Units mm, absolute coordinates

        let start_offset_x = self.params.offset_x.max(OVERSCAN_MM);
        let start_offset_y = self.params.offset_y.max(OVERSCAN_MM);

        gcode.push_str("M5\n");
        gcode.push_str(&format!(
            "G0 X{:.2} Y{:.2}\n",
            start_offset_x, start_offset_y
        ));
        gcode.push_str(&format!("F{:.0}\n", self.params.feed_rate));
        gcode.push('\n');

        let pixel_width = 1.0 / self.params.pixels_per_mm();
        let line_spacing = pixel_width;

        match self.params.scan_direction {
            ScanDirection::Horizontal => {
                self.generate_horizontal(&mut gcode, pixel_width, line_spacing)?;
            }
            ScanDirection::Vertical => {
                self.generate_vertical(&mut gcode, pixel_width, line_spacing)?;
            }
        }

        gcode.push_str("\nM5\n");
        gcode.push_str("G0 X0 Y0\n");

        Ok(gcode)
    }

    fn generate_horizontal(
        &self,
        gcode: &mut String,
        pixel_width: f32,
        line_spacing: f32,
    ) -> Result<()> {
        let non_empty_rows = self.get_non_empty_rows();

        if non_empty_rows.is_empty() {
            gcode.push_str("; No content to engrave\n");
            return Ok(());
        }

        let mut left_to_right = true;
        let offset_x = self.params.offset_x.max(OVERSCAN_MM);
        let offset_y = self.params.offset_y.max(OVERSCAN_MM);

        for (row_idx, &original_y) in non_empty_rows.iter().enumerate() {
            let y_rev = self.image.height() - 1 - original_y;
            let y_pos = y_rev as f32 * line_spacing;
            let y_absolute = offset_y + y_pos;

            if row_idx == 0 {
                gcode.push_str(&format!("G0 Y{:.3} F{:.0}\n", y_absolute, self.params.travel_rate));
            } else {
                gcode.push_str(&format!("G0 Y{:.3}\n", y_absolute));
            }

            // Llamar a scan_line pasando también el feed rate de grabado (engraving_rate)
            self.scan_line(gcode, original_y, y_pos, pixel_width, left_to_right, offset_x, offset_y);

            // ELIMINADA LA PAUSA G04 QUE CAUSABA PARONES ADICIONALES

            if self.params.bidirectional {
                left_to_right = !left_to_right;
            }
        }
        Ok(())
    }

    fn scan_line(
        &self,
        gcode: &mut String,
        y: u32,
        _y_pos: f32,
        pixel_width: f32,
        left_to_right: bool,
        offset_x: f32,
        _offset_y: f32,
    ) {
        let width = self.image.width();

        let mut min_x = width;
        let mut max_x = 0u32;

        for x in 0..width {
            let intensity = self.image.get_pixel(x, y).0[0];
            let power = self.intensity_to_power(intensity);
            if power > 0 {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }

        if min_x == width {
            return;
        }

        let margin = 2;

        let (start_x, end_x) = if left_to_right {
            let start = min_x.saturating_sub(margin);
            let end = (max_x + margin).min(width - 1);
            (start, end)
        } else {
            let start = (max_x + margin).min(width - 1);
            let end = min_x.saturating_sub(margin);
            (start, end)
        };

        if start_x > end_x && left_to_right {
            return;
        }
        if start_x < end_x && !left_to_right {
            return;
        }

        let mut points: Vec<(f32, u32)> = Vec::new();

        if left_to_right {
            for x in start_x..=end_x {
                let intensity = self.image.get_pixel(x, y).0[0];
                let power = self.intensity_to_power(intensity);
                let x_pos = offset_x + x as f32 * pixel_width;
                points.push((x_pos, power));
            }
        } else {
            for x in (end_x..=start_x).rev() {
                let intensity = self.image.get_pixel(x, y).0[0];
                let power = self.intensity_to_power(intensity);
                let x_pos = offset_x + x as f32 * pixel_width;
                points.push((x_pos, power));
            }
        }

        if points.is_empty() {
            return;
        }

        let mut merged: Vec<(f32, f32, u32)> = Vec::new();
        let mut seg_start_x = points[0].0;
        let mut current_power = points[0].1;

        for i in 1..points.len() {
            if points[i].1 != current_power {
                merged.push((seg_start_x, points[i-1].0, current_power));
                seg_start_x = points[i].0;
                current_power = points[i].1;
            }
        }
        merged.push((seg_start_x, points.last().unwrap().0, current_power));

        let overscan_dist = OVERSCAN_MM;
        let first_x = merged[0].0;
        let last_x = merged.last().unwrap().1;

        let (entry_x, exit_x) = if left_to_right {
            (first_x - overscan_dist, last_x + overscan_dist)
        } else {
            (first_x + overscan_dist, last_x - overscan_dist)
        };

        // CORRECCIÓN: Aseguramos el feed rate de corte/grabado en cada inicio de línea
        gcode.push_str(&format!("G0 X{:.2}\n", entry_x));
        gcode.push_str(&format!("G1 X{:.2} S0 F{:.0} M4\n", first_x, self.params.feed_rate));

        for segment in &merged {
            gcode.push_str(&format!("X{:.2} S{}\n", segment.1, segment.2));
        }

        gcode.push_str(&format!("X{:.2} S0\n", exit_x));
    }

    fn generate_vertical(
        &self,
        gcode: &mut String,
        pixel_width: f32,
        line_spacing: f32,
    ) -> Result<()> {
        let non_empty_cols = self.get_non_empty_columns();

        if non_empty_cols.is_empty() {
            gcode.push_str("; No content to engrave\n");
            return Ok(());
        }

        let mut top_to_bottom = true;
        let offset_x = self.params.offset_x.max(OVERSCAN_MM);
        let offset_y = self.params.offset_y.max(OVERSCAN_MM);

        let cols: Vec<u32> = non_empty_cols;

        for (col_idx, &original_x) in cols.iter().enumerate() {
            let x_pos = original_x as f32 * line_spacing;
            let x_absolute = offset_x + x_pos;

            if col_idx == 0 {
                gcode.push_str(&format!("G0 X{:.3} F{:.0}\n", x_absolute, self.params.travel_rate));
            } else {
                gcode.push_str(&format!("G0 X{:.3}\n", x_absolute));
            }

            self.scan_column(
                gcode,
                original_x,
                x_pos, // Pasamos el x_pos calculado con line_spacing
                pixel_width,
                top_to_bottom,
                offset_y,
                x_absolute, // Pasamos directamente la coordenada absoluta X ya calculada
            );

            if self.params.bidirectional {
                top_to_bottom = !top_to_bottom;
            }
        }
        Ok(())
    }

    fn scan_column(
        &self,
        gcode: &mut String,
        x: u32,
        _x_pos: f32,
        pixel_width: f32,
        top_to_bottom: bool,
        offset_y: f32,
        x_absolute: f32, // Recibe la coordenada X absoluta exacta
    ) {
        let height = self.image.height();

        let mut min_y = height;
        let mut max_y = 0u32;

        for y in 0..height {
            let intensity = self.image.get_pixel(x, y).0[0];
            let power = self.intensity_to_power(intensity);
            if power > 0 {
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }

        if min_y == height {
            return;
        }

        let margin = 2;

        let (start_y, end_y) = if top_to_bottom {
            let start = min_y.saturating_sub(margin);
            let end = (max_y + margin).min(height - 1);
            (start, end)
        } else {
            let start = (max_y + margin).min(height - 1);
            let end = min_y.saturating_sub(margin);
            (start, end)
        };

        let mut points: Vec<(f32, u32)> = Vec::new();

        if top_to_bottom {
            for y in start_y..=end_y {
                let intensity = self.image.get_pixel(x, y).0[0];
                let power = self.intensity_to_power(intensity);
                let flipped_y = (height - 1 - y) as f32;
                let y_pos = offset_y + flipped_y * pixel_width;
                points.push((y_pos, power));
            }
        } else {
            for y in (end_y..=start_y).rev() {
                let intensity = self.image.get_pixel(x, y).0[0];
                let power = self.intensity_to_power(intensity);
                let flipped_y = (height - 1 - y) as f32;
                let y_pos = offset_y + flipped_y * pixel_width;
                points.push((y_pos, power));
            }
        }

        if points.is_empty() {
            return;
        }

        let mut merged: Vec<(f32, f32, u32)> = Vec::new();
        let mut seg_start_y = points[0].0;
        let mut current_power = points[0].1;

        for i in 1..points.len() {
            if points[i].1 != current_power {
                merged.push((seg_start_y, points[i-1].0, current_power));
                seg_start_y = points[i].0;
                current_power = points[i].1;
            }
        }
        merged.push((seg_start_y, points.last().unwrap().0, current_power));

        let overscan_dist = OVERSCAN_MM;
        let first_y = merged[0].0;
        let last_y = merged.last().unwrap().1;

        let (entry_y, exit_y) = if top_to_bottom {
            (first_y + overscan_dist, last_y - overscan_dist)
        } else {
            (first_y - overscan_dist, last_y + overscan_dist)
        };

        // CORRECCIÓN: Se fuerza la velocidad de trabajo (feed_rate) al iniciar el movimiento G1 de la columna
        gcode.push_str(&format!("G0 X{:.2}\n", x_absolute));
        gcode.push_str(&format!("G0 Y{:.2}\n", entry_y));
        gcode.push_str(&format!("G1 Y{:.2} S0 F{:.0} M4\n", first_y, self.params.feed_rate));

        for segment in &merged {
            gcode.push_str(&format!("Y{:.2} S{}\n", segment.1, segment.2));
        }

        gcode.push_str(&format!("Y{:.2} S0\n", exit_y));
    }
} // impl ImageEngraver

