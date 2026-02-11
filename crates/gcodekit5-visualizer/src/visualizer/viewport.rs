//! Shared viewport helpers for 2D visualizer rendering.

/// Bounding box accumulator used while parsing toolpaths.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl Default for Bounds {
    fn default() -> Self {
        Self::new()
    }
}

impl Bounds {
    pub fn new() -> Self {
        Self {
            min_x: f32::MAX,
            max_x: f32::MIN,
            min_y: f32::MAX,
            max_y: f32::MIN,
            min_z: f32::MAX,
            max_z: f32::MIN,
        }
    }

    pub fn update(&mut self, x: f32, y: f32, z: f32) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
        self.min_z = self.min_z.min(z);
        self.max_z = self.max_z.max(z);
    }

    pub fn is_valid(&self) -> bool {
        self.min_x.is_finite()
            && self.max_x.is_finite()
            && self.min_y.is_finite()
            && self.max_y.is_finite()
            && self.min_z.is_finite()
            && self.max_z.is_finite()
            && self.min_x <= self.max_x
            && self.min_y <= self.max_y
            // Z might be single plane (min_z == max_z) which is valid
            && self.min_z <= self.max_z
    }

    pub fn finalize_with_padding(self, padding_factor: f32) -> (f32, f32, f32, f32, f32, f32) {
        if !self.is_valid() {
            return (0.0, 100.0, 0.0, 100.0, 0.0, 10.0);
        }

        let padding_x = (self.max_x - self.min_x) * padding_factor;
        let padding_y = (self.max_y - self.min_y) * padding_factor;
        let padding_z = if self.max_z > self.min_z {
            (self.max_z - self.min_z) * padding_factor
        } else {
            1.0 // Default padding for flat Z
        };

        let final_min_x = (self.min_x - padding_x).min(0.0);
        let final_min_y = (self.min_y - padding_y).min(0.0);
        let final_min_z = (self.min_z - padding_z).min(0.0);

        (
            final_min_x,
            self.max_x + padding_x,
            final_min_y,
            self.max_y + padding_y,
            final_min_z,
            self.max_z + padding_z,
        )
    }
}

/// Helper responsible for translating world coordinates into SVG viewport values.
#[derive(Debug, Clone, Copy)]
pub struct ViewportTransform {
    padding: f32,
}

impl ViewportTransform {
    pub fn new(padding: f32) -> Self {
        Self { padding }
    }

    #[inline]
    pub fn padding(&self) -> f32 {
        self.padding
    }

    /// Compute the SVG viewbox tuple for the provided view configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn viewbox(
        &self,
        min_x: f32,
        min_y: f32,
        zoom_scale: f32,
        scale_factor: f32,
        x_offset: f32,
        y_offset: f32,
        width: f32,
        height: f32,
    ) -> (f32, f32, f32, f32) {
        let scale = zoom_scale * scale_factor;

        let left = (0.0 - self.padding - x_offset) / scale + min_x;
        let right = (width - self.padding - x_offset) / scale + min_x;

        let bottom = (0.0 - self.padding + y_offset) / scale + min_y;
        let top = (height - 0.0 - self.padding + y_offset) / scale + min_y;

        let svg_min_x = left;
        let svg_min_y = -top;
        let svg_width = right - left;
        let svg_height = top - bottom;

        (svg_min_x, svg_min_y, svg_width, svg_height)
    }

    /// Determine pan offsets that place a world coordinate at a specific screen target.
    #[allow(clippy::too_many_arguments)]
    pub fn offsets_to_place_world_point(
        &self,
        min_x: f32,
        min_y: f32,
        zoom_scale: f32,
        scale_factor: f32,
        canvas_height: f32,
        world_x: f32,
        world_y: f32,
        target_screen_x: f32,
        target_screen_y: f32,
    ) -> (f32, f32) {
        let scale = zoom_scale * scale_factor;
        let x_offset = target_screen_x - ((world_x - min_x) * scale + self.padding);
        let y_offset = (world_y - min_y) * scale + self.padding - (canvas_height - target_screen_y);
        (x_offset, y_offset)
    }
}
