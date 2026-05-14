//! Module to import raster images as shapes in the designer

use anyhow::{Context, Result};
use image::GenericImageView;
use std::path::Path;

use crate::model::{Point, RasterImage};
use crate::Shape;

pub struct ImageImporter;

impl ImageImporter {
    pub fn import(path: &Path) -> Result<Shape> {
        let img = image::open(path)
            .with_context(|| format!("Failed to load image: {}", path.display()))?;
        let (width_px, height_px) = img.dimensions();
        let target_width_mm = 100.0;
        let aspect = height_px as f64 / width_px as f64;
        let height_mm = target_width_mm * aspect;

        let mut image_data = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut image_data),
            image::ImageFormat::Png,
        )
        .context("Failed to encode image as PNG")?;

        let raster = RasterImage::new(
            0,
            Point::new(target_width_mm / 2.0, height_mm / 2.0),
            target_width_mm,
            height_mm,
            image_data,
            Some(path.to_path_buf()),
        );

        Ok(Shape::RasterImage(raster))
    }

    /// Load image data from a path without creating a Shape
    /// Returns (image_data_png, width_mm, height_mm)
    pub fn load_image_data(path: &Path) -> Result<(Vec<u8>, f64, f64)> {
        let img = image::open(path)
            .with_context(|| format!("Failed to load image: {}", path.display()))?;
        let (width_px, height_px) = img.dimensions();

        //Keep the original dimensions in mm (1px = 1mm by default)
        let width_mm = width_px as f64;
        let height_mm = height_px as f64;

        let mut image_data = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut image_data),
            image::ImageFormat::Png,
        )
        .context("Failed to encode image as PNG")?;

        Ok((image_data, width_mm, height_mm))
    }

    /// Load image preserving original dimensions from the saved data
    /// This respects the width/height stored in the design file
    pub fn load_image_data_with_size(
        path: &Path,
        target_width_mm: f64,
        target_height_mm: f64,
    ) -> Result<(Vec<u8>, f64, f64)> {
        let img = image::open(path)
            .with_context(|| format!("Failed to load image: {}", path.display()))?;

        let mut image_data = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut image_data),
            image::ImageFormat::Png,
        )
        .context("Failed to encode image as PNG")?;

        Ok((image_data, target_width_mm, target_height_mm))
    }
}
