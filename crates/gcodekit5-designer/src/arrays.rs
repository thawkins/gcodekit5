//! # Array Operations Module
//!
//! Provides functionality for creating multiple copies of shapes in linear, circular, and grid patterns.
//!
//! Supports:
//! - Linear arrays (X/Y direction copies with uniform spacing)
//! - Circular arrays (rotational copies around a center point)
//! - Grid arrays (2D rectangular arrays with row/column spacing)
//! - Configurable spacing, count, and orientation
//! - Integration with existing shapes and toolpath generation

use crate::model::Point;
use anyhow::Result;

/// Represents different types of array operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayType {
    /// Linear array in X and Y directions
    Linear,
    /// Circular array around a center point
    Circular,
    /// Grid/rectangular array in 2D
    Grid,
}

/// Parameters for linear array operations
#[derive(Debug, Clone)]
pub struct LinearArrayParams {
    /// Number of copies in X direction
    pub count_x: u32,
    /// Number of copies in Y direction
    pub count_y: u32,
    /// Spacing between copies in X direction (mm)
    pub spacing_x: f64,
    /// Spacing between copies in Y direction (mm)
    pub spacing_y: f64,
}

impl LinearArrayParams {
    /// Create new linear array parameters
    pub fn new(count_x: u32, count_y: u32, spacing_x: f64, spacing_y: f64) -> Self {
        debug_assert!(
            spacing_x.is_finite(),
            "spacing_x must be finite, got {spacing_x}"
        );
        debug_assert!(
            spacing_y.is_finite(),
            "spacing_y must be finite, got {spacing_y}"
        );
        Self {
            count_x,
            count_y,
            spacing_x,
            spacing_y,
        }
    }

    /// Validate parameters
    pub fn is_valid(&self) -> bool {
        self.count_x > 0 && self.count_y > 0 && self.spacing_x >= 0.0 && self.spacing_y >= 0.0
    }

    /// Get total number of copies
    pub fn total_copies(&self) -> u32 {
        self.count_x * self.count_y
    }

    /// Calculate bounding box of the array
    pub fn calculate_bounds(&self, original_bounds: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
        let (min_x, min_y, max_x, max_y) = original_bounds;
        let width = max_x - min_x;
        let height = max_y - min_y;

        let array_width = width + (self.count_x - 1) as f64 * self.spacing_x;
        let array_height = height + (self.count_y - 1) as f64 * self.spacing_y;

        (min_x, min_y, min_x + array_width, min_y + array_height)
    }
}

/// Parameters for circular array operations
#[derive(Debug, Clone)]
pub struct CircularArrayParams {
    /// Number of copies to create
    pub count: u32,
    /// Center point of the array
    pub center: Point,
    /// Radius from center to original shape
    pub radius: f64,
    /// Starting angle in degrees (0-360)
    pub start_angle: f64,
    /// Rotation direction: true = clockwise, false = counter-clockwise
    pub clockwise: bool,
}

impl CircularArrayParams {
    /// Create new circular array parameters
    pub fn new(count: u32, center: Point, radius: f64, start_angle: f64, clockwise: bool) -> Self {
        debug_assert!(radius.is_finite(), "radius must be finite, got {radius}");
        debug_assert!(
            start_angle.is_finite(),
            "start_angle must be finite, got {start_angle}"
        );
        Self {
            count,
            center,
            radius,
            start_angle,
            clockwise,
        }
    }

    /// Validate parameters
    pub fn is_valid(&self) -> bool {
        self.count > 0 && self.radius >= 0.0 && self.start_angle >= 0.0 && self.start_angle <= 360.0
    }

    /// Calculate angle step between copies
    pub fn angle_step(&self) -> f64 {
        360.0 / self.count as f64
    }

    /// Calculate the position of the Nth copy relative to original
    pub fn get_offset(&self, copy_index: u32) -> (f64, f64) {
        if copy_index == 0 {
            return (0.0, 0.0);
        }

        let angle_step = self.angle_step();
        let angle = if self.clockwise {
            self.start_angle - (copy_index as f64) * angle_step
        } else {
            self.start_angle + (copy_index as f64) * angle_step
        };

        let angle_rad = angle.to_radians();
        (self.radius * angle_rad.cos(), self.radius * angle_rad.sin())
    }
}

/// Parameters for grid array operations
#[derive(Debug, Clone)]
pub struct GridArrayParams {
    /// Number of columns
    pub columns: u32,
    /// Number of rows
    pub rows: u32,
    /// Horizontal spacing between columns (mm)
    pub column_spacing: f64,
    /// Vertical spacing between rows (mm)
    pub row_spacing: f64,
}

impl GridArrayParams {
    /// Create new grid array parameters
    pub fn new(columns: u32, rows: u32, column_spacing: f64, row_spacing: f64) -> Self {
        debug_assert!(
            column_spacing.is_finite(),
            "column_spacing must be finite, got {column_spacing}"
        );
        debug_assert!(
            row_spacing.is_finite(),
            "row_spacing must be finite, got {row_spacing}"
        );
        Self {
            columns,
            rows,
            column_spacing,
            row_spacing,
        }
    }

    /// Validate parameters
    pub fn is_valid(&self) -> bool {
        self.columns > 0 && self.rows > 0 && self.column_spacing >= 0.0 && self.row_spacing >= 0.0
    }

    /// Get total number of copies
    pub fn total_copies(&self) -> u32 {
        self.columns * self.rows
    }

    /// Calculate position offset for a specific cell in the grid
    pub fn get_offset(&self, column: u32, row: u32) -> Option<(f64, f64)> {
        if column >= self.columns || row >= self.rows {
            return None;
        }

        Some((
            column as f64 * self.column_spacing,
            row as f64 * self.row_spacing,
        ))
    }

    /// Calculate bounding box of the grid array
    pub fn calculate_bounds(&self, original_bounds: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
        let (min_x, min_y, max_x, max_y) = original_bounds;
        let width = max_x - min_x;
        let height = max_y - min_y;

        let array_width = width + (self.columns - 1) as f64 * self.column_spacing;
        let array_height = height + (self.rows - 1) as f64 * self.row_spacing;

        (min_x, min_y, min_x + array_width, min_y + array_height)
    }
}

/// Main array operation combining type and parameters
#[derive(Debug, Clone)]
pub enum ArrayOperation {
    /// Linear array with its parameters
    Linear(LinearArrayParams),
    /// Circular array with its parameters
    Circular(CircularArrayParams),
    /// Grid array with its parameters
    Grid(GridArrayParams),
}

impl ArrayOperation {
    /// Get the array type
    pub fn array_type(&self) -> ArrayType {
        match self {
            ArrayOperation::Linear(_) => ArrayType::Linear,
            ArrayOperation::Circular(_) => ArrayType::Circular,
            ArrayOperation::Grid(_) => ArrayType::Grid,
        }
    }

    /// Validate the array operation
    pub fn is_valid(&self) -> bool {
        match self {
            ArrayOperation::Linear(params) => params.is_valid(),
            ArrayOperation::Circular(params) => params.is_valid(),
            ArrayOperation::Grid(params) => params.is_valid(),
        }
    }

    /// Get total number of copies
    pub fn total_copies(&self) -> u32 {
        match self {
            ArrayOperation::Linear(params) => params.total_copies(),
            ArrayOperation::Circular(params) => params.count,
            ArrayOperation::Grid(params) => params.total_copies(),
        }
    }
}

/// Generator for array copies
pub struct ArrayGenerator;

impl ArrayGenerator {
    /// Generate copy offsets for a linear array
    pub fn generate_linear(params: &LinearArrayParams) -> Result<Vec<(f64, f64)>> {
        if !params.is_valid() {
            return Err(anyhow::anyhow!(
                "Invalid linear array parameters: count_x={}, count_y={}, spacing_x={}, spacing_y={}",
                params.count_x,
                params.count_y,
                params.spacing_x,
                params.spacing_y
            ));
        }

        let mut offsets = Vec::new();
        for y in 0..params.count_y {
            for x in 0..params.count_x {
                let offset_x = x as f64 * params.spacing_x;
                let offset_y = y as f64 * params.spacing_y;
                offsets.push((offset_x, offset_y));
            }
        }

        Ok(offsets)
    }

    /// Generate copy offsets for a circular array
    pub fn generate_circular(params: &CircularArrayParams) -> Result<Vec<(f64, f64)>> {
        if !params.is_valid() {
            return Err(anyhow::anyhow!(
                "Invalid circular array parameters: count={}, radius={}",
                params.count,
                params.radius
            ));
        }

        let mut offsets = Vec::new();
        for i in 0..params.count {
            let offset = params.get_offset(i);
            offsets.push(offset);
        }

        Ok(offsets)
    }

    /// Generate copy offsets for a grid array
    pub fn generate_grid(params: &GridArrayParams) -> Result<Vec<(f64, f64)>> {
        if !params.is_valid() {
            return Err(anyhow::anyhow!(
                "Invalid grid array parameters: columns={}, rows={}, column_spacing={}, row_spacing={}",
                params.columns,
                params.rows,
                params.column_spacing,
                params.row_spacing
            ));
        }

        let mut offsets = Vec::new();
        for row in 0..params.rows {
            for col in 0..params.columns {
                if let Some(offset) = params.get_offset(col, row) {
                    offsets.push(offset);
                }
            }
        }

        Ok(offsets)
    }

    /// Generate copy offsets for any array operation
    pub fn generate(operation: &ArrayOperation) -> Result<Vec<(f64, f64)>> {
        match operation {
            ArrayOperation::Linear(params) => Self::generate_linear(params),
            ArrayOperation::Circular(params) => Self::generate_circular(params),
            ArrayOperation::Grid(params) => Self::generate_grid(params),
        }
    }
}
