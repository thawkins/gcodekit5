//! Laser Image Engraving Tool
//!
//! Converts bitmap images to G-code for laser engraving using raster scanning.
//! Supports halftoning via pepecore, mirroring, rotation, grayscale power modulation,
//! bidirectional scanning, and various image formats.
//! Images are rendered from bottom to top to match device coordinate space where Y increases upward.

use anyhow::{Context, Result};
use gcodekit5_core::types::BoxedIterator;
use image::{DynamicImage, GrayImage};
use std::path::Path;

/// Image rotation angles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationAngle {
    /// No rotation
    Degrees0,
    /// 90 degrees clockwise
    Degrees90,
    /// 180 degrees
    Degrees180,
    /// 270 degrees clockwise
    Degrees270,
}

/// Halftoning algorithm options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HalftoneMethod {
    /// No halftoning (direct intensity mapping)
    None,
    /// Simple thresholding
    Threshold,
    /// Ordered dithering (Bayer 4x4)
    Bayer4x4,
    /// Error diffusion (Floyd-Steinberg)
    FloydSteinberg,
    /// Error diffusion (Atkinson)
    Atkinson,
}

/// Scan direction for laser engraving
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanDirection {
    /// Horizontal scanning (left to right)
    Horizontal,
    /// Vertical scanning (top to bottom)
    Vertical,
}

/// Image transformation parameters
#[derive(Debug, Clone)]
pub struct ImageTransformations {
    /// Mirror image horizontally (flip around Y axis)
    pub mirror_x: bool,
    /// Mirror image vertically (flip around X axis)
    pub mirror_y: bool,
    /// Rotation angle
    pub rotation: RotationAngle,
    /// Halftoning method
    pub halftone: HalftoneMethod,
    /// Halftone dot size (cell size in pixels, typically 2-10)
    pub halftone_dot_size: usize,
    /// Halftone threshold (0-255, deprecated - kept for compatibility)
    pub halftone_threshold: u8,
    /// Invert image (dark becomes light, light becomes dark)
    pub invert: bool,
}

impl Default for ImageTransformations {
    fn default() -> Self {
        Self {
            mirror_x: false,
            mirror_y: false,
            rotation: RotationAngle::Degrees0,
            halftone: HalftoneMethod::None,
            halftone_dot_size: 4,
            halftone_threshold: 127,
            invert: false,
        }
    }
}

/// Laser engraving parameters
#[derive(Debug, Clone)]
pub struct EngravingParameters {
    /// Output width in millimeters
    pub width_mm: f32,
    /// Output height in millimeters (auto-calculated if None based on aspect ratio)
    pub height_mm: Option<f32>,
    /// Feed rate for engraving moves (mm/min)
    pub feed_rate: f32,
    /// Travel feed rate for rapid moves (mm/min)
    pub travel_rate: f32,
    /// Minimum laser power (0-100%)
    pub min_power: f32,
    /// Maximum laser power (0-100%)
    pub max_power: f32,
    /// Resolution in pixels per millimeter
    pub pixels_per_mm: f32,
    /// Scan direction
    pub scan_direction: ScanDirection,
    /// Use bidirectional scanning (faster but may reduce quality)
    pub bidirectional: bool,
    /// Line spacing multiplier (1.0 = normal, >1.0 = faster with lines)
    pub line_spacing: f32,
    /// Laser power scale (0-1000 for GRBL S parameter)
    pub power_scale: f32,
    /// Image transformations (halftoning, mirroring, rotation)
    pub transformations: ImageTransformations,
    /// X offset from machine origin
    pub offset_x: f32,
    /// Y offset from machine origin
    pub offset_y: f32,
    /// Number of axes on the target device (default 3).
    pub num_axes: u8,
}

impl Default for EngravingParameters {
    fn default() -> Self {
        Self {
            width_mm: 100.0,
            height_mm: None,
            feed_rate: 1000.0,
            travel_rate: 3000.0,
            min_power: 0.0,
            max_power: 100.0,
            pixels_per_mm: 10.0,
            scan_direction: ScanDirection::Horizontal,
            bidirectional: true,
            line_spacing: 1.0,
            power_scale: 1000.0,
            transformations: ImageTransformations::default(),
            offset_x: 10.0,
            offset_y: 10.0,
            num_axes: 3,
        }
    }
}

/// Laser engraving tool for bitmap images
pub struct BitmapImageEngraver {
    image: GrayImage,
    params: EngravingParameters,
    output_width: u32,
    output_height: u32,
}

impl BitmapImageEngraver {
    /// Create a new laser engraver from an image file
    pub fn from_file<P: AsRef<Path>>(path: P, params: EngravingParameters) -> Result<Self> {
        let img = image::open(path.as_ref()).context("Failed to load image file")?;
        Self::from_image(img, params)
    }

    /// Create a new laser engraver from a DynamicImage
    pub fn from_image(img: DynamicImage, params: EngravingParameters) -> Result<Self> {
        let mut gray = img.to_luma8();

        // Apply transformations: mirroring -> rotation -> inversion -> halftoning
        if params.transformations.mirror_x {
            image::imageops::flip_horizontal_in_place(&mut gray);
        }
        if params.transformations.mirror_y {
            image::imageops::flip_vertical_in_place(&mut gray);
        }

        if params.transformations.rotation != RotationAngle::Degrees0 {
            gray = Self::apply_rotation_image(gray, params.transformations.rotation);
        }

        // Calculate output dimensions
        let width = gray.width();
        let height = gray.height();
        let output_width = (params.width_mm * params.pixels_per_mm) as u32;
        let aspect_ratio = height as f32 / width as f32;
        let output_height = if let Some(h) = params.height_mm {
            (h * params.pixels_per_mm) as u32
        } else {
            (output_width as f32 * aspect_ratio) as u32
        };

        // Resize image to output dimensions BEFORE halftoning
        // Use Lanczos3 for high quality downscaling/upscaling of continuous tone images
        gray = image::imageops::resize(
            &gray,
            output_width,
            output_height,
            image::imageops::FilterType::Lanczos3,
        );

        if params.transformations.invert {
            image::imageops::invert(&mut gray);
        }

        if params.transformations.halftone != HalftoneMethod::None {
            Self::apply_halftoning_image(
                &mut gray,
                params.transformations.halftone,
                params.transformations.halftone_threshold,
                params.transformations.halftone_dot_size,
            )?;
        }

        Ok(Self {
            image: gray,
            params,
            output_width,
            output_height,
        })
    }

    /// Apply rotation to image
    pub fn apply_rotation_image(image: GrayImage, rotation: RotationAngle) -> GrayImage {
        match rotation {
            RotationAngle::Degrees0 => image,
            RotationAngle::Degrees90 => image::imageops::rotate90(&image),
            RotationAngle::Degrees180 => image::imageops::rotate180(&image),
            RotationAngle::Degrees270 => image::imageops::rotate270(&image),
        }
    }

    /// Apply halftoning
    fn apply_halftoning_image(
        image: &mut GrayImage,
        method: HalftoneMethod,
        threshold: u8,
        dot_size: usize,
    ) -> Result<()> {
        if dot_size > 1 {
            let width = image.width();
            let height = image.height();
            let new_width = std::cmp::max(1, width / dot_size as u32);
            let new_height = std::cmp::max(1, height / dot_size as u32);

            // Downscale
            let mut small = image::imageops::resize(
                image,
                new_width,
                new_height,
                image::imageops::FilterType::Lanczos3,
            );

            // Apply halftone to small image
            match method {
                HalftoneMethod::Threshold => Self::apply_threshold_image(&mut small, threshold)?,
                HalftoneMethod::Bayer4x4 => Self::apply_bayer_image(&mut small)?,
                HalftoneMethod::FloydSteinberg => Self::apply_floyd_steinberg_image(&mut small)?,
                HalftoneMethod::Atkinson => Self::apply_atkinson_image(&mut small)?,
                HalftoneMethod::None => {}
            }

            // Upscale back
            *image = image::imageops::resize(
                &small,
                width,
                height,
                image::imageops::FilterType::Nearest,
            );
            return Ok(());
        }

        match method {
            HalftoneMethod::Threshold => Self::apply_threshold_image(image, threshold),
            HalftoneMethod::Bayer4x4 => Self::apply_bayer_image(image),
            HalftoneMethod::FloydSteinberg => Self::apply_floyd_steinberg_image(image),
            HalftoneMethod::Atkinson => Self::apply_atkinson_image(image),
            HalftoneMethod::None => Ok(()),
        }
    }

    /// Apply simple thresholding
    fn apply_threshold_image(image: &mut GrayImage, threshold: u8) -> Result<()> {
        for pixel in image.pixels_mut() {
            *pixel = if pixel.0[0] >= threshold {
                image::Luma([255])
            } else {
                image::Luma([0])
            };
        }
        Ok(())
    }

    /// Apply Bayer 4x4 ordered dithering
    fn apply_bayer_image(image: &mut GrayImage) -> Result<()> {
        let width = image.width();
        let height = image.height();

        // Bayer 4x4 matrix
        let bayer_matrix = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel_mut(x, y);
                let val = pixel.0[0];

                // Scale matrix value to 0-255 range
                let matrix_val = bayer_matrix[(y % 4) as usize][(x % 4) as usize];
                let threshold = (matrix_val as f32 * 16.0 + 8.0) as u8;

                *pixel = if val >= threshold {
                    image::Luma([255])
                } else {
                    image::Luma([0])
                };
            }
        }
        Ok(())
    }

    /// Apply Floyd-Steinberg error diffusion
    fn apply_floyd_steinberg_image(image: &mut GrayImage) -> Result<()> {
        let width = image.width();
        let height = image.height();

        // We need to work with i16 to handle error propagation without overflow
        // Copy to buffer
        let mut buffer: Vec<i16> = image.as_raw().iter().map(|&p| p as i16).collect();

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let old_pixel = buffer[idx];
                let new_pixel = if old_pixel > 127 { 255 } else { 0 };

                buffer[idx] = new_pixel;
                let error = old_pixel - new_pixel;

                // Distribute error
                if x + 1 < width {
                    let neighbor_idx = (y * width + (x + 1)) as usize;
                    buffer[neighbor_idx] = buffer[neighbor_idx].saturating_add(error * 7 / 16);
                }
                if x > 0 && y + 1 < height {
                    let neighbor_idx = ((y + 1) * width + (x - 1)) as usize;
                    buffer[neighbor_idx] = buffer[neighbor_idx].saturating_add(error * 3 / 16);
                }
                if y + 1 < height {
                    let neighbor_idx = ((y + 1) * width + x) as usize;
                    buffer[neighbor_idx] = buffer[neighbor_idx].saturating_add(error * 5 / 16);
                }
                if x + 1 < width && y + 1 < height {
                    let neighbor_idx = ((y + 1) * width + (x + 1)) as usize;
                    buffer[neighbor_idx] = buffer[neighbor_idx].saturating_add(error / 16);
                }
            }
        }

        // Copy back
        for (i, &val) in buffer.iter().enumerate() {
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            image.put_pixel(x, y, image::Luma([val.clamp(0, 255) as u8]));
        }
        Ok(())
    }

    /// Apply Atkinson error diffusion
    fn apply_atkinson_image(image: &mut GrayImage) -> Result<()> {
        let width = image.width();
        let height = image.height();

        let mut buffer: Vec<i16> = image.as_raw().iter().map(|&p| p as i16).collect();

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let old_pixel = buffer[idx];
                let new_pixel = if old_pixel > 127 { 255 } else { 0 };

                buffer[idx] = new_pixel;
                let error = old_pixel - new_pixel;

                // Atkinson distributes 1/8 of error to 6 neighbors
                let neighbors = [(1, 0), (2, 0), (-1, 1), (0, 1), (1, 1), (0, 2)];

                for (dx, dy) in neighbors {
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;

                    if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                        let n_idx = ny as usize * width as usize + nx as usize;
                        buffer[n_idx] = buffer[n_idx].saturating_add(error / 8);
                    }
                }
            }
        }

        for (i, &val) in buffer.iter().enumerate() {
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            image.put_pixel(x, y, image::Luma([val.clamp(0, 255) as u8]));
        }
        Ok(())
    }

    /// Get the output dimensions in millimeters
    pub fn output_size_mm(&self) -> (f32, f32) {
        (
            self.output_width as f32 / self.params.pixels_per_mm,
            self.output_height as f32 / self.params.pixels_per_mm,
        )
    }

    /// Estimate engraving time in seconds
    pub fn estimate_time(&self) -> f32 {
        let (width_mm, height_mm) = self.output_size_mm();
        let line_spacing = 1.0 / self.params.pixels_per_mm * self.params.line_spacing;
        let num_lines = (height_mm / line_spacing) as u32;

        let engrave_time = (width_mm * num_lines as f32) / self.params.feed_rate * 60.0;
        let travel_time = if self.params.bidirectional {
            (height_mm / self.params.travel_rate) * 60.0
        } else {
            (width_mm * num_lines as f32) / self.params.travel_rate * 60.0
        };

        engrave_time + travel_time
    }

    /// Generate G-code for laser engraving
    pub fn generate_gcode(&self) -> Result<String> {
        self.generate_gcode_with_progress(|_| {})
    }

    /// Generate G-code for laser engraving with progress callback
    pub fn generate_gcode_with_progress<F>(&self, mut progress_callback: F) -> Result<String>
    where
        F: FnMut(f32),
    {
        let mut gcode = String::new();

        gcode.push_str("; Laser Image Engraving G-code\n");
        gcode.push_str(&format!(
            "; Generated: {}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));
        let (width_mm, height_mm) = self.output_size_mm();
        gcode.push_str(&format!(
            "; Image size: {:.2}mm x {:.2}mm\n",
            width_mm, height_mm
        ));
        gcode.push_str(&format!(
            "; Resolution: {:.1} pixels/mm\n",
            self.params.pixels_per_mm
        ));
        gcode.push_str(&format!(
            "; Feed rate: {:.0} mm/min\n",
            self.params.feed_rate
        ));
        gcode.push_str(&format!(
            "; Power range: {:.0}%-{:.0}%\n",
            self.params.min_power, self.params.max_power
        ));
        gcode.push_str(&format!(
            "; Estimated time: {:.1} minutes\n",
            self.estimate_time() / 60.0
        ));
        gcode.push_str(";\n");

        gcode.push_str("G21 ; Set units to millimeters\n");
        gcode.push_str("G90 ; Absolute positioning\n");
        gcode.push_str("G17 ; XY plane selection\n");
        gcode.push('\n');

        gcode.push_str("; Home and set work coordinate system\n");
        gcode.push_str("$H ; Home all axes (bottom-left corner)\n");
        gcode.push_str("G10 L2 P1 X0 Y0 Z0 ; Clear G54 offset\n");
        gcode.push_str("G54 ; Select work coordinate system 1\n");
        gcode.push_str(&format!(
            "G0 X{:.1} Y{:.1} ; Move to work origin\n",
            self.params.offset_x, self.params.offset_y
        ));
        gcode.push_str("G10 L20 P1 X0 Y0 Z0 ; Set current position as work zero\n");
        if self.params.num_axes >= 3 {
            gcode.push_str(&format!(
                "G0 Z{:.2} F{:.0} ; Move to safe height\n",
                5.0, self.params.travel_rate
            ));
        }
        gcode.push('\n');

        gcode.push_str("M5 ; Laser off\n");
        gcode.push('\n');

        progress_callback(0.0);

        // Image is already resized in from_image
        progress_callback(0.1);

        let line_spacing = 1.0 / self.params.pixels_per_mm * self.params.line_spacing;
        let pixel_width = 1.0 / self.params.pixels_per_mm;

        match self.params.scan_direction {
            ScanDirection::Horizontal => {
                self.generate_horizontal_scan_with_progress(
                    &mut gcode,
                    &self.image,
                    pixel_width,
                    line_spacing,
                    &mut progress_callback,
                )?;
            }
            ScanDirection::Vertical => {
                self.generate_vertical_scan_with_progress(
                    &mut gcode,
                    &self.image,
                    pixel_width,
                    line_spacing,
                    &mut progress_callback,
                )?;
            }
        }

        progress_callback(0.9);

        gcode.push_str("\n; End of engraving\n");
        gcode.push_str("M5 ; Laser off\n");
        gcode.push_str("G0 X0 Y0 ; Return to origin\n");

        progress_callback(1.0);

        Ok(gcode)
    }

    fn generate_horizontal_scan_with_progress<F>(
        &self,
        gcode: &mut String,
        image: &GrayImage,
        pixel_width: f32,
        line_spacing: f32,
        progress_callback: &mut F,
    ) -> Result<()>
    where
        F: FnMut(f32),
    {
        let height = image.height();
        let width = image.width();
        let mut left_to_right = true;

        // Render from bottom to top to match device coordinate space
        for y_reversed in 0..height {
            if y_reversed % 10 == 0 || y_reversed == height - 1 {
                let progress = 0.1 + (y_reversed as f32 / height as f32) * 0.8;
                progress_callback(progress);
            }

            let y = height - 1 - y_reversed;
            let y_pos = y_reversed as f32 * line_spacing;

            if left_to_right || !self.params.bidirectional {
                gcode.push_str(&format!("G0 X0 Y{:.3}\n", y_pos));
            } else {
                gcode.push_str(&format!(
                    "G0 X{:.3} Y{:.3}\n",
                    (width - 1) as f32 * pixel_width,
                    y_pos
                ));
            }

            let mut in_burn = false;
            let mut last_power = 0;

            let x_range: BoxedIterator<u32> = if left_to_right || !self.params.bidirectional {
                Box::new(0..width)
            } else {
                Box::new((0..width).rev())
            };

            for x in x_range {
                let intensity = image.get_pixel(x, y).0[0];
                let power = self.intensity_to_power(intensity);
                let power_value = (power * self.params.power_scale / 100.0) as u32;
                let x_pos = x as f32 * pixel_width;

                if power_value > 0 {
                    if !in_burn || power_value != last_power {
                        gcode.push_str(&format!(
                            "G1 X{:.3} Y{:.3} F{:.0} M3 S{}\n",
                            x_pos, y_pos, self.params.feed_rate, power_value
                        ));
                        in_burn = true;
                        last_power = power_value;
                    } else {
                        gcode.push_str(&format!("G1 X{:.3} Y{:.3}\n", x_pos, y_pos));
                    }
                } else if in_burn {
                    gcode.push_str("M5\n");
                    in_burn = false;
                }
            }

            if in_burn {
                gcode.push_str("M5\n");
            }

            if self.params.bidirectional {
                left_to_right = !left_to_right;
            }
        }

        Ok(())
    }

    fn generate_vertical_scan_with_progress<F>(
        &self,
        gcode: &mut String,
        image: &GrayImage,
        pixel_width: f32,
        line_spacing: f32,
        progress_callback: &mut F,
    ) -> Result<()>
    where
        F: FnMut(f32),
    {
        let height = image.height();
        let width = image.width();
        let mut top_to_bottom = true;

        for x in 0..width {
            if x % 10 == 0 || x == width - 1 {
                let progress = 0.1 + (x as f32 / width as f32) * 0.8;
                progress_callback(progress);
            }
            let x_pos = x as f32 * line_spacing;

            if top_to_bottom || !self.params.bidirectional {
                gcode.push_str(&format!("G0 X{:.3} Y0\n", x_pos));
            } else {
                gcode.push_str(&format!(
                    "G0 X{:.3} Y{:.3}\n",
                    x_pos,
                    (height - 1) as f32 * pixel_width
                ));
            }

            let mut in_burn = false;
            let mut last_power = 0;

            let y_range: BoxedIterator<u32> = if top_to_bottom || !self.params.bidirectional {
                Box::new(0..height)
            } else {
                Box::new((0..height).rev())
            };

            for y_reversed in y_range {
                let y = height - 1 - y_reversed;
                let intensity = image.get_pixel(x, y).0[0];
                let power = self.intensity_to_power(intensity);
                let power_value = (power * self.params.power_scale / 100.0) as u32;
                let y_pos = y_reversed as f32 * pixel_width;

                if power_value > 0 {
                    if !in_burn || power_value != last_power {
                        gcode.push_str(&format!(
                            "G1 X{:.3} Y{:.3} F{:.0} M3 S{}\n",
                            x_pos, y_pos, self.params.feed_rate, power_value
                        ));
                        in_burn = true;
                        last_power = power_value;
                    } else {
                        gcode.push_str(&format!("G1 X{:.3} Y{:.3}\n", x_pos, y_pos));
                    }
                } else if in_burn {
                    gcode.push_str("M5\n");
                    in_burn = false;
                }
            }

            if in_burn {
                gcode.push_str("M5\n");
            }

            if self.params.bidirectional {
                top_to_bottom = !top_to_bottom;
            }
        }

        Ok(())
    }

    /// Convert pixel intensity to laser power
    fn intensity_to_power(&self, intensity: u8) -> f32 {
        let normalized = intensity as f32 / 255.0;
        self.params.min_power + (normalized * (self.params.max_power - self.params.min_power))
    }
}
