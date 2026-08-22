//! Shared viewport helpers for visualizer rendering.

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
