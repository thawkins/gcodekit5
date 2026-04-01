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
}
