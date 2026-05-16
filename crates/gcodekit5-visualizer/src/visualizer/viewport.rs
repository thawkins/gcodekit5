//! Shared viewport helpers for 2D visualizer rendering.

/// Bounding box accumulator used while parsing toolpaths.
/// Supports full 6-axis tracking including rotary axes (A, B, C).
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
    // Rotary axis bounds (4th, 5th, 6th axes)
    pub min_a: f32,
    pub max_a: f32,
    pub min_b: f32,
    pub max_b: f32,
    pub min_c: f32,
    pub max_c: f32,
    // Track if rotary axes have been set
    pub has_rotary: bool,
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
            // Initialize rotary axis bounds
            min_a: f32::MAX,
            max_a: f32::MIN,
            min_b: f32::MAX,
            max_b: f32::MIN,
            min_c: f32::MAX,
            max_c: f32::MIN,
            has_rotary: false,
        }
    }

    /// Update bounds with X, Y, Z coordinates
    pub fn update_xyz(&mut self, x: f32, y: f32, z: f32) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
        self.min_z = self.min_z.min(z);
        self.max_z = self.max_z.max(z);
    }

    /// Legacy alias for update_xyz - maintains backward compatibility
    pub fn update(&mut self, x: f32, y: f32, z: f32) {
        self.update_xyz(x, y, z);
    }

    /// Update bounds with rotary axis coordinates (A, B, C)
    pub fn update_rotary(&mut self, a: Option<f32>, b: Option<f32>, c: Option<f32>) {
        if let Some(a_val) = a {
            self.min_a = self.min_a.min(a_val);
            self.max_a = self.max_a.max(a_val);
            self.has_rotary = true;
        }
        if let Some(b_val) = b {
            self.min_b = self.min_b.min(b_val);
            self.max_b = self.max_b.max(b_val);
            self.has_rotary = true;
        }
        if let Some(c_val) = c {
            self.min_c = self.min_c.min(c_val);
            self.max_c = self.max_c.max(c_val);
            self.has_rotary = true;
        }
    }

    /// Check if bounds include any rotary axis movement
    pub fn has_rotary_movement(&self) -> bool {
        self.has_rotary
            && (self.min_a != f32::MAX || self.min_b != f32::MAX || self.min_c != f32::MAX)
    }

    /// Get rotary axis ranges as (min, max) tuples
    pub fn rotary_ranges(&self) -> ((f32, f32), (f32, f32), (f32, f32)) {
        let a_range = if self.min_a == f32::MAX {
            (0.0, 0.0)
        } else {
            (self.min_a, self.max_a)
        };
        let b_range = if self.min_b == f32::MAX {
            (0.0, 0.0)
        } else {
            (self.min_b, self.max_b)
        };
        let c_range = if self.min_c == f32::MAX {
            (0.0, 0.0)
        } else {
            (self.min_c, self.max_c)
        };
        (a_range, b_range, c_range)
    }

    /// Get maximum rotation magnitude across all rotary axes
    pub fn max_rotary_range(&self) -> f32 {
        let mut max_range = 0.0f32;
        if self.min_a != f32::MAX {
            max_range = max_range.max(self.max_a - self.min_a);
        }
        if self.min_b != f32::MAX {
            max_range = max_range.max(self.max_b - self.min_b);
        }
        if self.min_c != f32::MAX {
            max_range = max_range.max(self.max_c - self.min_c);
        }
        max_range
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
