//! Stock Material Definition
//!
//! Defines the `StockMaterial` struct used by the 3D visualizer and
//! stock removal simulation for CNC machining operations.

/// Represents the stock material dimensions and position
#[derive(Debug, Clone)]
pub struct StockMaterial {
    /// Width in X dimension (mm)
    pub width: f32,
    /// Height in Y dimension (mm)
    pub height: f32,
    /// Thickness in Z dimension (mm)
    pub thickness: f32,
    /// Origin point (bottom-left corner) in world coordinates
    pub origin: (f32, f32, f32),
    /// Safe Z height for rapid moves (mm)
    pub safe_z: f32,
}

impl StockMaterial {
    /// Default clearance above stock top (mm) for safe Z moves.
    pub const DEFAULT_SAFE_Z_ABOVE_STOCK_MM: f32 = 5.0;
    /// Minimum clearance above stock top (mm) for safe Z moves.
    pub const MIN_SAFE_Z_ABOVE_STOCK_MM: f32 = 1.0;

    /// Returns the default safe Z for a given stock thickness.
    pub fn default_safe_z_for_thickness(thickness: f32) -> f32 {
        thickness.max(0.0) + Self::DEFAULT_SAFE_Z_ABOVE_STOCK_MM
    }

    /// Create a new stock material definition
    pub fn new(width: f32, height: f32, thickness: f32, origin: (f32, f32, f32)) -> Self {
        let default_safe_z = Self::default_safe_z_for_thickness(thickness);
        let mut stock = Self {
            width,
            height,
            thickness,
            origin,
            safe_z: default_safe_z,
        };
        stock.normalize_safe_z();
        stock
    }

    /// Create a new stock material definition with custom safe Z height
    pub fn with_safe_z(
        width: f32,
        height: f32,
        thickness: f32,
        origin: (f32, f32, f32),
        safe_z: f32,
    ) -> Self {
        let mut stock = Self {
            width,
            height,
            thickness,
            origin,
            safe_z,
        };
        stock.normalize_safe_z();
        stock
    }

    /// Returns the minimum safe Z required for a given stock thickness.
    pub fn min_safe_z_for_thickness(thickness: f32) -> f32 {
        thickness.max(0.0) + Self::MIN_SAFE_Z_ABOVE_STOCK_MM
    }

    /// Clamps stock thickness/safe-Z so safe-Z is always above stock top.
    pub fn normalize_safe_z(&mut self) {
        self.thickness = self.thickness.max(0.0);
        let min_safe_z = Self::min_safe_z_for_thickness(self.thickness);
        if self.safe_z < min_safe_z {
            self.safe_z = min_safe_z;
        }
    }

    /// Get the center point of the stock
    pub fn center(&self) -> (f32, f32, f32) {
        (
            self.origin.0 + self.width / 2.0,
            self.origin.1 + self.height / 2.0,
            self.origin.2 + self.thickness / 2.0,
        )
    }

    /// Get the top surface Z coordinate
    pub fn top_z(&self) -> f32 {
        self.origin.2 + self.thickness
    }

    /// Check if a point is within stock bounds
    pub fn contains(&self, x: f32, y: f32, z: f32) -> bool {
        x >= self.origin.0
            && x <= self.origin.0 + self.width
            && y >= self.origin.1
            && y <= self.origin.1 + self.height
            && z >= self.origin.2
            && z <= self.origin.2 + self.thickness
    }
}

