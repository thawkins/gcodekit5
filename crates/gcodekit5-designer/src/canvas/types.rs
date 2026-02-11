//! Canvas type definitions: CanvasSnapshot, CanvasPoint, DrawingMode, DrawingObject, Alignment.

use crate::model::{DesignerShape, Point, Shape, ShapeType};
use crate::pocket_operations::PocketStrategy;
use crate::shape_store::ShapeStore;
use crate::shapes::OperationType;
use crate::spatial_manager::SpatialManager;

/// Snapshot of canvas state for undo/redo
#[derive(Clone)]
#[allow(dead_code)]
pub struct CanvasSnapshot {
    pub(crate) shape_store: ShapeStore,
    pub(crate) spatial_manager: SpatialManager,
}

/// Canvas coordinates for drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasPoint {
    pub x: f64,
    pub y: f64,
}

impl CanvasPoint {
    /// Creates a new canvas point.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Converts to a design point.
    pub fn to_point(&self) -> Point {
        Point::new(self.x, self.y)
    }
}

impl From<Point> for CanvasPoint {
    fn from(p: Point) -> Self {
        Self::new(p.x, p.y)
    }
}

/// Drawing modes for the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingMode {
    Select,
    Rectangle,
    Circle,
    Line,
    Ellipse,
    Polyline,
    Text,
    Triangle,
    Polygon,
    Gear,
    Sprocket,
    Pan,
}

/// Drawing object on the canvas that can be selected and manipulated.
#[derive(Debug, Clone)]
pub struct DrawingObject {
    pub id: u64,
    pub group_id: Option<u64>,
    pub name: String,
    pub shape: Shape,
    pub selected: bool,
    pub operation_type: OperationType,
    pub use_custom_values: bool,
    pub pocket_depth: f64,
    pub start_depth: f64,
    pub step_down: f32,
    pub step_in: f32,
    pub ramp_angle: f32,
    pub pocket_strategy: PocketStrategy,
    pub raster_fill_ratio: f64,
    pub offset: f64,
    pub fillet: f64,
    pub chamfer: f64,
    pub lock_aspect_ratio: bool,
}

impl DrawingObject {
    pub fn get_effective_shape(&self) -> Shape {
        let mut shape = self.shape.clone();
        if self.offset != 0.0 {
            shape = crate::ops::perform_offset(&shape, self.offset);
        }
        if self.fillet != 0.0 {
            shape = crate::ops::perform_fillet(&shape, self.fillet);
        }
        if self.chamfer != 0.0 {
            shape = crate::ops::perform_chamfer(&shape, self.chamfer);
        }
        shape
    }

    pub fn get_total_bounds(&self) -> (f64, f64, f64, f64) {
        let (x1, y1, x2, y2) = self.shape.bounds();
        if self.offset.abs() < 1e-6 && self.fillet.abs() < 1e-6 && self.chamfer.abs() < 1e-6 {
            return (x1, y1, x2, y2);
        }
        let (ex1, ey1, ex2, ey2) = self.get_effective_shape().bounds();
        (x1.min(ex1), y1.min(ey1), x2.max(ex2), y2.max(ey2))
    }

    pub fn contains_point(&self, point: &Point, tolerance: f64) -> bool {
        self.shape.contains_point(*point, tolerance)
            || self.get_effective_shape().contains_point(*point, tolerance)
    }

    /// Creates a new drawing object.
    pub fn new(id: u64, shape: Shape) -> Self {
        let name = match shape.shape_type() {
            ShapeType::Rectangle => "Rectangle",
            ShapeType::Circle => "Circle",
            ShapeType::Line => "Line",
            ShapeType::Ellipse => "Ellipse",
            ShapeType::Path => "Path",
            ShapeType::Text => "Text",
            ShapeType::Triangle => "Triangle",
            ShapeType::Polygon => "Polygon",
            ShapeType::Gear => "Gear",
            ShapeType::Sprocket => "Sprocket",
        }
        .to_string();

        Self {
            id,
            group_id: None,
            name,
            shape,
            selected: false,
            operation_type: OperationType::default(),
            use_custom_values: false,
            pocket_depth: 0.0,
            start_depth: 0.0,
            step_down: 0.0,
            step_in: 0.0,
            ramp_angle: 0.0,
            pocket_strategy: PocketStrategy::ContourParallel,
            raster_fill_ratio: 0.5,
            offset: 0.0,
            fillet: 0.0,
            chamfer: 0.0,
            lock_aspect_ratio: true,
        }
    }
}

pub enum Alignment {
    Left,
    CenterHorizontal,
    Right,
    Top,
    CenterVertical,
    Bottom,
}
