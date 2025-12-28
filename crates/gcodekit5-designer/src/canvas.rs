//! Canvas for drawing and manipulating shapes.

use super::pocket_operations::PocketStrategy;
use super::shapes::OperationType;
use super::spatial_index::Bounds;
use super::viewport::Viewport;
use crate::model::{
    DesignCircle as Circle, DesignEllipse as Ellipse, DesignLine as Line, DesignPath as PathShape,
    DesignPolygon as Polygon, DesignRectangle as Rectangle, DesignText as TextShape,
    DesignTriangle as Triangle, DesignerShape, Point, Shape, ShapeType,
};
use crate::selection_manager::SelectionManager;
use crate::shape_store::ShapeStore;
use crate::spatial_manager::SpatialManager;

/// Snapshot of canvas state for undo/redo
#[derive(Clone)]
#[allow(dead_code)]
pub struct CanvasSnapshot {
    shape_store: ShapeStore,
    spatial_manager: SpatialManager,
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

/// Canvas state managing shapes and drawing operations.
#[derive(Debug, Clone)]
pub struct Canvas {
    pub shape_store: ShapeStore,
    pub selection_manager: SelectionManager,
    pub spatial_manager: SpatialManager,
    mode: DrawingMode,
    viewport: Viewport,
    // Deprecated fields kept for compatibility if needed, but we should remove them
    // shapes: HashMap<u64, DrawingObject>,
    // draw_order: Vec<u64>,
    // next_id: u64,
    // selected_id: Option<u64>,
    // spatial_index: SpatialIndex,
}

impl Canvas {
    /// Creates a new canvas.
    pub fn new() -> Self {
        Self {
            shape_store: ShapeStore::new(),
            selection_manager: SelectionManager::new(),
            spatial_manager: SpatialManager::new(),
            mode: DrawingMode::Select,
            viewport: Viewport::new(1200.0, 600.0),
        }
    }

    /// Creates a canvas with specified dimensions.
    pub fn with_size(width: f64, height: f64) -> Self {
        Self {
            shape_store: ShapeStore::new(),
            selection_manager: SelectionManager::new(),
            spatial_manager: SpatialManager::new(),
            mode: DrawingMode::Select,
            viewport: Viewport::new(width, height),
        }
    }

    /// Sets the drawing mode.
    pub fn set_mode(&mut self, mode: DrawingMode) {
        self.mode = mode;
    }

    /// Gets the current drawing mode.
    pub fn mode(&self) -> DrawingMode {
        self.mode
    }

    /// Returns the number of shapes on the canvas.
    /// Returns the number of shapes on the canvas.
    pub fn shape_count(&self) -> usize {
        self.shape_store.len()
    }

    /// Generates a new unique ID.
    pub fn generate_id(&mut self) -> u64 {
        self.shape_store.generate_id()
    }

    /// Sets the next ID to be generated.
    pub fn set_next_id(&mut self, id: u64) {
        self.shape_store.set_next_id(id);
    }

    /// Gets a reference to a shape by ID.
    pub fn get_shape(&self, id: u64) -> Option<&DrawingObject> {
        self.shape_store.get(id)
    }

    /// Returns the axis-aligned bounding box of all selected shapes.
    /// Returns `None` when no shapes are selected.
    pub fn selection_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_selected = false;

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
            has_selected = true;
        }

        if has_selected {
            Some((min_x, min_y, max_x, max_y))
        } else {
            None
        }
    }

    /// Adds a rectangle to the canvas.
    pub fn add_rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) -> u64 {
        let id = self.shape_store.generate_id();
        let rect = Rectangle::new(x, y, width, height);
        let (min_x, min_y, max_x, max_y) = rect.bounds();
        self.shape_store
            .insert(id, DrawingObject::new(id, Shape::Rectangle(rect)));
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        id
    }

    /// Adds a circle to the canvas.
    pub fn add_circle(&mut self, center: Point, radius: f64) -> u64 {
        let id = self.shape_store.generate_id();
        let circle = Circle::new(center, radius);
        let (min_x, min_y, max_x, max_y) = circle.bounds();
        self.shape_store
            .insert(id, DrawingObject::new(id, Shape::Circle(circle)));
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        id
    }

    /// Adds a generic shape to the canvas.
    pub fn add_shape(&mut self, shape: Shape) -> u64 {
        let id = self.shape_store.generate_id();
        let (min_x, min_y, max_x, max_y) = shape.bounds();
        self.shape_store.insert(id, DrawingObject::new(id, shape));
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        id
    }

    /// Adds a line to the canvas.
    pub fn add_line(&mut self, start: Point, end: Point) -> u64 {
        let id = self.shape_store.generate_id();
        let line = Line::new(start, end);
        let (min_x, min_y, max_x, max_y) = line.bounds();
        self.shape_store
            .insert(id, DrawingObject::new(id, Shape::Line(line)));
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        id
    }

    /// Adds an ellipse to the canvas.
    pub fn add_ellipse(&mut self, center: Point, rx: f64, ry: f64) -> u64 {
        let id = self.shape_store.generate_id();
        let ellipse = Ellipse::new(center, rx, ry);
        let (min_x, min_y, max_x, max_y) = ellipse.bounds();
        self.shape_store
            .insert(id, DrawingObject::new(id, Shape::Ellipse(ellipse)));
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        id
    }

    /// Adds a polyline to the canvas.
    pub fn add_polyline(&mut self, vertices: Vec<Point>) -> u64 {
        let id = self.shape_store.generate_id();
        // Create a closed PathShape from vertices
        let path_shape = PathShape::from_points(&vertices, true);
        let (min_x, min_y, max_x, max_y) = path_shape.bounds();
        self.shape_store
            .insert(id, DrawingObject::new(id, Shape::Path(path_shape)));
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        id
    }

    /// Adds a text shape to the canvas.
    pub fn add_text(&mut self, text: String, x: f64, y: f64, font_size: f64) -> u64 {
        let id = self.shape_store.generate_id();
        let shape = TextShape::new(text, x, y, font_size);
        let (min_x, min_y, max_x, max_y) = shape.bounds();
        self.shape_store
            .insert(id, DrawingObject::new(id, Shape::Text(shape)));
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        id
    }

    /// Adds a triangle to the canvas.
    pub fn add_triangle(&mut self, center: Point, width: f64, height: f64) -> u64 {
        let id = self.shape_store.generate_id();
        let triangle = Triangle::new(center, width, height);
        let (min_x, min_y, max_x, max_y) = triangle.bounds();
        self.shape_store
            .insert(id, DrawingObject::new(id, Shape::Triangle(triangle)));
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        id
    }

    /// Adds a polygon to the canvas.
    pub fn add_polygon(&mut self, center: Point, radius: f64, sides: u32) -> u64 {
        let id = self.shape_store.generate_id();
        let polygon = Polygon::new(center, radius, sides);
        let (min_x, min_y, max_x, max_y) = polygon.bounds();
        self.shape_store
            .insert(id, DrawingObject::new(id, Shape::Polygon(polygon)));
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        id
    }

    /// Groups the selected shapes.
    pub fn group_selected(&mut self) {
        let selected_count = self.selection_manager.selected_count(&self.shape_store);
        if selected_count < 2 {
            return;
        }

        let group_id = self.shape_store.generate_id();

        for obj in self.shape_store.iter_mut() {
            if obj.selected {
                obj.group_id = Some(group_id);
            }
        }
    }

    /// Ungroups the selected shapes.
    pub fn ungroup_selected(&mut self) {
        for obj in self.shape_store.iter_mut() {
            if obj.selected {
                obj.group_id = None;
            }
        }
    }

    /// Selects a shape at the given point.
    /// If multi is true, toggles selection of the shape at point while keeping others.
    /// If multi is false, clears other selections and selects the shape at point.
    pub fn select_at(&mut self, point: &Point, tolerance: f64, multi: bool) -> Option<u64> {
        self.selection_manager.select_at(
            &mut self.shape_store,
            self.spatial_manager.inner(),
            point,
            tolerance,
            multi,
        )
    }

    /// Selects shapes within or intersecting the given rectangle.
    /// If multi is true, adds to current selection.
    /// If multi is false, clears other selections first.
    pub fn select_in_rect(&mut self, x: f64, y: f64, width: f64, height: f64, multi: bool) {
        self.selection_manager.select_in_rect(
            &mut self.shape_store,
            self.spatial_manager.inner(),
            x,
            y,
            width,
            height,
            multi,
        );
    }

    /// Selects a shape by ID.
    pub fn select_shape(&mut self, id: u64, multi: bool) {
        self.selection_manager
            .select_id(&mut self.shape_store, id, multi);
    }

    /// Gets the number of selected shapes.
    pub fn selected_count(&self) -> usize {
        self.selection_manager.selected_count(&self.shape_store)
    }

    /// Removes all selected shapes.
    pub fn remove_selected(&mut self) {
        self.selection_manager
            .remove_selected(&mut self.shape_store, self.spatial_manager.inner_mut());
    }

    /// Gets all shapes on the canvas.
    pub fn shapes(&self) -> impl Iterator<Item = &DrawingObject> {
        self.shape_store.iter()
    }

    /// Returns a mutable reference to the shapes array.
    pub fn shapes_mut(&mut self) -> impl Iterator<Item = &mut DrawingObject> {
        self.shape_store.iter_mut()
    }

    /// Gets the selected shape ID.
    pub fn selected_id(&self) -> Option<u64> {
        self.selection_manager.selected_id()
    }

    /// Removes a shape by ID.
    pub fn remove_shape(&mut self, id: u64) -> bool {
        self.remove_shape_return(id).is_some()
    }

    /// Removes a shape and returns it (used for undo/redo).
    pub fn remove_shape_return(&mut self, id: u64) -> Option<DrawingObject> {
        if let Some(obj) = self.shape_store.remove(id) {
            let (min_x, min_y, max_x, max_y) = obj.get_total_bounds();
            self.spatial_manager
                .remove_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));

            if self.selection_manager.selected_id() == Some(id) {
                self.selection_manager.set_selected_id(None);
            }
            Some(obj)
        } else {
            None
        }
    }

    /// Restores a shape (used for undo/redo).
    pub fn restore_shape(&mut self, obj: DrawingObject) {
        let id = obj.id;
        let (min_x, min_y, max_x, max_y) = obj.get_total_bounds();
        self.spatial_manager
            .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
        self.shape_store.insert(id, obj);
    }

    /// Gets a mutable reference to a shape by ID.
    pub fn get_shape_mut(&mut self, id: u64) -> Option<&mut DrawingObject> {
        self.shape_store.get_mut(id)
    }

    /// Removes an item from the spatial index.
    pub fn remove_from_index(&mut self, id: u64, bounds: &Bounds) {
        self.spatial_manager.remove_bounds(id, bounds);
    }

    /// Inserts an item into the spatial index.
    pub fn insert_into_index(&mut self, id: u64, bounds: &Bounds) {
        self.spatial_manager.insert_bounds(id, bounds);
    }

    /// Deselects all shapes.
    pub fn deselect_all(&mut self) {
        self.selection_manager.deselect_all(&mut self.shape_store);
    }

    /// Selects all shapes.
    pub fn select_all(&mut self) {
        self.selection_manager.select_all(&mut self.shape_store);
    }

    /// Checks if point is inside currently selected shape (primary selection only)
    pub fn is_point_in_selected(&self, point: &Point) -> bool {
        if let Some(id) = self.selection_manager.selected_id() {
            if let Some(obj) = self.shape_store.get(id) {
                return obj.contains_point(point, 3.0);
            }
        }
        false
    }

    /// Sets the canvas size (viewport dimensions).
    pub fn set_canvas_size(&mut self, width: f64, height: f64) {
        self.viewport.set_canvas_size(width, height);
    }

    /// Sets zoom level (1.0 = 100%).
    pub fn set_zoom(&mut self, zoom: f64) {
        self.viewport.set_zoom(zoom);
    }

    /// Gets current zoom level.
    pub fn zoom(&self) -> f64 {
        self.viewport.zoom()
    }

    /// Fit the viewport to the given world bounds using specified padding.
    pub fn fit_to_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64, padding: f64) {
        self.viewport
            .fit_to_bounds(min_x, min_y, max_x, max_y, padding);
    }

    /// Zooms in.
    pub fn zoom_in(&mut self) {
        self.viewport.zoom_in();
    }

    /// Zooms out.
    pub fn zoom_out(&mut self) {
        self.viewport.zoom_out();
    }

    /// Resets zoom to 100%.
    pub fn reset_zoom(&mut self) {
        self.viewport.reset_zoom();
    }

    /// Sets pan offset.
    pub fn set_pan(&mut self, x: f64, y: f64) {
        self.viewport.set_pan(x, y);
    }

    /// Gets pan X offset.
    pub fn pan_x(&self) -> f64 {
        self.viewport.pan_x()
    }

    /// Gets pan Y offset.
    pub fn pan_y(&self) -> f64 {
        self.viewport.pan_y()
    }

    /// Pans by a delta amount.
    pub fn pan_by(&mut self, dx: f64, dy: f64) {
        self.viewport.pan_by(dx, dy);
    }

    /// Resets pan to origin.
    pub fn reset_pan(&mut self) {
        self.viewport.reset_pan();
    }

    /// Gets a reference to the viewport for coordinate transformations.
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Gets a mutable reference to the viewport.
    pub fn viewport_mut(&mut self) -> &mut Viewport {
        &mut self.viewport
    }

    /// Converts pixel coordinates to world coordinates.
    pub fn pixel_to_world(&self, pixel_x: f64, pixel_y: f64) -> Point {
        self.viewport.pixel_to_world(pixel_x, pixel_y)
    }

    /// Converts world coordinates to pixel coordinates.
    pub fn world_to_pixel(&self, world_x: f64, world_y: f64) -> (f64, f64) {
        self.viewport.world_to_pixel(world_x, world_y)
    }

    /// Fits the canvas to show all shapes with padding.
    pub fn fit_all_shapes(&mut self) {
        if self.shape_store.is_empty() {
            // If canvas is empty, fit_to_view should set zoom to 100% and offset origin by 30px X and -30px Y
            self.viewport.set_zoom(1.0);
            self.viewport.set_pan(30.0, -30.0);
            return;
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for obj in self.shape_store.iter() {
            let (x1, y1, x2, y2) = obj.get_total_bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
        }

        // Fit content to view with a VIEW_PADDING per-edge padding so that content has small margins
        self.viewport.fit_to_bounds(
            min_x,
            min_y,
            max_x,
            max_y,
            gcodekit5_core::constants::VIEW_PADDING,
        );
    }

    /// Zooms to a point with optional zoom level.
    pub fn zoom_to_point(&mut self, world_point: &Point, zoom: f64) {
        self.viewport.zoom_to_point(world_point, zoom);
    }

    /// Centers the canvas on a point.
    pub fn center_on(&mut self, point: &Point) {
        self.viewport.center_on_point(point);
    }

    /// Resets viewport to default state.
    pub fn reset_view(&mut self) {
        self.viewport.reset();
    }

    /// Gets the pan offset (compatibility method).
    pub fn pan_offset(&self) -> (f64, f64) {
        (self.viewport.pan_x(), self.viewport.pan_y())
    }

    /// Pans the canvas (compatibility method).
    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.viewport.pan_by(dx, dy);
    }

    /// Clears all shapes from the canvas.
    pub fn clear(&mut self) {
        self.shape_store.clear();
        self.selection_manager.set_selected_id(None);
        self.spatial_manager.clear();
    }

    /// Moves the selected shape by (dx, dy).
    pub fn move_selected(&mut self, dx: f64, dy: f64) {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if obj.selected {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();

                obj.shape.translate(dx, dy);

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }
    }

    /// Updates the geometry modifiers for a shape.
    pub fn update_shape_geometry(&mut self, id: u64, offset: f64, fillet: f64, chamfer: f64) {
        if let Some(obj) = self.shape_store.get_mut(id) {
            let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();
            let old_bounds = Bounds::new(old_x1, old_y1, old_x2, old_y2);

            obj.offset = offset;
            obj.fillet = fillet;
            obj.chamfer = chamfer;

            let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
            let new_bounds = Bounds::new(new_x1, new_y1, new_x2, new_y2);

            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }
    }

    /// Calculates the deltas (dx, dy) required to align each selected shape according to the specified alignment.
    /// Returns a vector of (shape_id, dx, dy) for each selected shape that needs to move.
    pub fn calculate_alignment_deltas(&self, alignment: Alignment) -> Vec<(u64, f64, f64)> {
        let selected: Vec<_> = self.shape_store.iter().filter(|o| o.selected).collect();
        if selected.is_empty() {
            return Vec::new();
        }

        // 1. Calculate target value
        let target = match alignment {
            Alignment::Left => selected
                .iter()
                .map(|o| o.shape.bounds().0)
                .fold(f64::INFINITY, f64::min),
            Alignment::Right => selected
                .iter()
                .map(|o| o.shape.bounds().2)
                .fold(f64::NEG_INFINITY, f64::max),
            Alignment::CenterHorizontal => {
                let (min_x, max_x) =
                    selected
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), o| {
                            let (x1, _, x2, _) = o.shape.bounds();
                            (min.min(x1), max.max(x2))
                        });
                if min_x.is_infinite() {
                    f64::INFINITY
                } else {
                    (min_x + max_x) / 2.0
                }
            }
            Alignment::Top => selected
                .iter()
                .map(|o| o.shape.bounds().3)
                .fold(f64::NEG_INFINITY, f64::max),
            Alignment::Bottom => selected
                .iter()
                .map(|o| o.shape.bounds().1)
                .fold(f64::INFINITY, f64::min),
            Alignment::CenterVertical => {
                let (min_y, max_y) =
                    selected
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), o| {
                            let (_, y1, _, y2) = o.shape.bounds();
                            (min.min(y1), max.max(y2))
                        });
                if min_y.is_infinite() {
                    f64::INFINITY
                } else {
                    (min_y + max_y) / 2.0
                }
            }
        };

        if target.is_infinite() {
            return Vec::new();
        }

        let mut deltas = Vec::new();

        for obj in selected {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            let (dx, dy) = match alignment {
                Alignment::Left => (target - x1, 0.0),
                Alignment::Right => (target - x2, 0.0),
                Alignment::CenterHorizontal => (target - (x1 + x2) / 2.0, 0.0),
                Alignment::Top => (0.0, target - y2),
                Alignment::Bottom => (0.0, target - y1),
                Alignment::CenterVertical => (0.0, target - (y1 + y2) / 2.0),
            };

            if dx.abs() > f64::EPSILON || dy.abs() > f64::EPSILON {
                deltas.push((obj.id, dx, dy));
            }
        }

        deltas
    }

    /// Pastes objects onto the canvas with an offset.
    /// Returns the IDs of the new objects.
    pub fn paste_objects(
        &mut self,
        objects: &[DrawingObject],
        offset_x: f64,
        offset_y: f64,
    ) -> Vec<u64> {
        let mut new_ids = Vec::new();
        let mut group_map = std::collections::HashMap::new();

        self.selection_manager.deselect_all(&mut self.shape_store);

        for obj in objects {
            let id = self.shape_store.generate_id();

            let mut new_shape = obj.shape.clone();
            new_shape.translate(offset_x, offset_y);
            let (min_x, min_y, max_x, max_y) = new_shape.bounds();

            // Handle group ID mapping
            let new_group_id = if let Some(old_gid) = obj.group_id {
                if !group_map.contains_key(&old_gid) {
                    let new_gid = self.shape_store.generate_id();
                    group_map.insert(old_gid, new_gid);
                    Some(new_gid)
                } else {
                    Some(group_map[&old_gid])
                }
            } else {
                None
            };

            let new_obj = DrawingObject {
                id,
                group_id: new_group_id,
                name: obj.name.clone(),
                shape: new_shape,
                selected: true, // Select the new object
                operation_type: obj.operation_type,
                use_custom_values: obj.use_custom_values,
                pocket_depth: obj.pocket_depth,
                start_depth: obj.start_depth,
                step_down: obj.step_down,
                step_in: obj.step_in,
                ramp_angle: obj.ramp_angle,
                pocket_strategy: obj.pocket_strategy,
                raster_fill_ratio: obj.raster_fill_ratio,
                offset: obj.offset,
                fillet: obj.fillet,
                chamfer: obj.chamfer,
            };

            self.shape_store.insert(id, new_obj);
            self.spatial_manager
                .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
            new_ids.push(id);
        }

        // Update selected_id to the last pasted object if any
        if let Some(last_id) = new_ids.last() {
            self.selection_manager.set_selected_id(Some(*last_id));
        }

        new_ids
    }

    /// Resizes the selected shape. Handles: 0=TL, 1=TR, 2=BL, 3=BR, 4=Center (moves)
    pub fn resize_selected(&mut self, handle: usize, dx: f64, dy: f64) {
        // Calculate bounding box of ALL selected shapes
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_selected = false;

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
            has_selected = true;
        }

        if !has_selected {
            return;
        }

        // If handle is 4 (move), we just translate all selected shapes
        if handle == 4 {
            self.move_selected(dx, dy);
            return;
        }

        // Calculate new bounding box based on handle movement
        let (new_min_x, new_min_y, new_max_x, new_max_y) = match handle {
            0 => (min_x + dx, min_y + dy, max_x, max_y), // Top-left
            1 => (min_x, min_y + dy, max_x + dx, max_y), // Top-right
            2 => (min_x + dx, min_y, max_x, max_y + dy), // Bottom-left
            3 => (min_x, min_y, max_x + dx, max_y + dy), // Bottom-right
            _ => (min_x, min_y, max_x, max_y),
        };

        let old_width = max_x - min_x;
        let old_height = max_y - min_y;
        let new_width = (new_max_x - new_min_x).abs();
        let new_height = (new_max_y - new_min_y).abs();

        // Calculate scale factors
        let sx = if old_width.abs() > 1e-6 {
            new_width / old_width
        } else {
            1.0
        };
        let sy = if old_height.abs() > 1e-6 {
            new_height / old_height
        } else {
            1.0
        };

        // Center of scaling
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        let new_center_x = (new_min_x + new_max_x) / 2.0;
        let new_center_y = (new_min_y + new_max_y) / 2.0;

        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if obj.selected {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();

                // Scale relative to the center of the SELECTION bounding box
                obj.shape.scale(sx, sy, Point::new(center_x, center_y));

                // Translate to new center
                let t_dx = new_center_x - center_x;
                let t_dy = new_center_y - center_y;
                obj.shape.translate(t_dx, t_dy);

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }
    }

    /// Calculates the snapped shapes without modifying the canvas.
    pub fn calculate_snapped_shapes(&self) -> Vec<(u64, DrawingObject)> {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            let width = x2 - x1;
            let height = y2 - y1;

            // Snap the top-left corner and dimensions to whole mm
            let snapped_x1 = (x1 + 0.5).floor();
            let snapped_y1 = (y1 + 0.5).floor();
            let snapped_width = (width + 0.5).floor();
            let snapped_height = (height + 0.5).floor();

            // Replace the shape with snapped position and dimensions
            let shape = &obj.shape;
            let new_shape: Shape = match shape {
                Shape::Rectangle(_) => Shape::Rectangle(Rectangle::new(
                    snapped_x1,
                    snapped_y1,
                    snapped_width,
                    snapped_height,
                )),
                Shape::Circle(_) => {
                    let radius = snapped_width / 2.0;
                    Shape::Circle(Circle::new(
                        Point::new(snapped_x1 + radius, snapped_y1 + radius),
                        radius,
                    ))
                }
                Shape::Line(_) => Shape::Line(Line::new(
                    Point::new(snapped_x1, snapped_y1),
                    Point::new(snapped_x1 + snapped_width, snapped_y1 + snapped_height),
                )),
                Shape::Ellipse(_) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    Shape::Ellipse(Ellipse::new(
                        center,
                        snapped_width / 2.0,
                        snapped_height / 2.0,
                    ))
                }
                Shape::Path(path_shape) => {
                    let (path_x1, path_y1, path_x2, path_y2) = path_shape.bounds();
                    let path_w = path_x2 - path_x1;
                    let path_h = path_y2 - path_y1;

                    let scale_x = if path_w.abs() > 1e-6 {
                        snapped_width / path_w
                    } else {
                        1.0
                    };
                    let scale_y = if path_h.abs() > 1e-6 {
                        snapped_height / path_h
                    } else {
                        1.0
                    };

                    let center_x = (path_x1 + path_x2) / 2.0;
                    let center_y = (path_y1 + path_y2) / 2.0;

                    let mut scaled = path_shape.clone();
                    scaled.scale(scale_x, scale_y, Point::new(center_x, center_y));

                    let new_center_x = snapped_x1 + snapped_width / 2.0;
                    let new_center_y = snapped_y1 + snapped_height / 2.0;

                    let dx = new_center_x - center_x;
                    let dy = new_center_y - center_y;

                    scaled.translate(dx, dy);
                    Shape::Path(scaled)
                }
                Shape::Text(text) => Shape::Text(TextShape::new(
                    text.text.clone(),
                    snapped_x1,
                    snapped_y1,
                    text.font_size,
                )),
                Shape::Triangle(_triangle) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    Shape::Triangle(Triangle::new(center, snapped_width, snapped_height))
                }
                Shape::Polygon(polygon) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    let radius = snapped_width.min(snapped_height) / 2.0;
                    Shape::Polygon(Polygon::new(center, radius, polygon.sides))
                }
                Shape::Gear(gear) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    let module = snapped_width / gear.teeth as f64;
                    Shape::Gear(crate::model::DesignGear::new(center, module, gear.teeth))
                }
                Shape::Sprocket(sprocket) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    let pitch =
                        snapped_width * (std::f64::consts::PI / sprocket.teeth as f64).sin();
                    Shape::Sprocket(crate::model::DesignSprocket::new(
                        center,
                        pitch,
                        sprocket.teeth,
                    ))
                }
            };

            let mut new_obj = obj.clone();
            new_obj.shape = new_shape;
            updates.push((obj.id, new_obj));
        }

        updates
    }

    /// Snaps the selected shape's position to whole millimeters
    pub fn snap_selected_to_mm(&mut self) {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if obj.selected {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();
                let (x1, y1, x2, y2) = obj.shape.bounds();
                let width = x2 - x1;
                let height = y2 - y1;

                // Snap the top-left corner and dimensions to whole mm
                let snapped_x1 = (x1 + 0.5).floor();
                let snapped_y1 = (y1 + 0.5).floor();
                let snapped_width = (width + 0.5).floor();
                let snapped_height = (height + 0.5).floor();

                // Replace the shape with snapped position and dimensions
                let shape = &obj.shape;
                let new_shape: Shape = match shape.shape_type() {
                    ShapeType::Rectangle => Shape::Rectangle(Rectangle::new(
                        snapped_x1,
                        snapped_y1,
                        snapped_width,
                        snapped_height,
                    )),
                    ShapeType::Circle => {
                        let radius = snapped_width / 2.0;
                        Shape::Circle(Circle::new(
                            Point::new(snapped_x1 + radius, snapped_y1 + radius),
                            radius,
                        ))
                    }
                    ShapeType::Line => Shape::Line(Line::new(
                        Point::new(snapped_x1, snapped_y1),
                        Point::new(snapped_x1 + snapped_width, snapped_y1 + snapped_height),
                    )),
                    ShapeType::Ellipse => {
                        let rx = snapped_width / 2.0;
                        let ry = snapped_height / 2.0;
                        Shape::Ellipse(Ellipse::new(
                            Point::new(snapped_x1 + rx, snapped_y1 + ry),
                            rx,
                            ry,
                        ))
                    }
                    ShapeType::Path => {
                        if let Some(path_shape) = shape.as_any().downcast_ref::<PathShape>() {
                            let sx = if width.abs() > 1e-6 {
                                snapped_width / width
                            } else {
                                1.0
                            };
                            let sy = if height.abs() > 1e-6 {
                                snapped_height / height
                            } else {
                                1.0
                            };

                            let center_x = (x1 + x2) / 2.0;
                            let center_y = (y1 + y2) / 2.0;

                            let mut new_path_shape = path_shape.clone();
                            new_path_shape.scale(sx, sy, Point::new(center_x, center_y));

                            let new_center_x = snapped_x1 + snapped_width / 2.0;
                            let new_center_y = snapped_y1 + snapped_height / 2.0;
                            let t_dx = new_center_x - center_x;
                            let t_dy = new_center_y - center_y;

                            new_path_shape.translate(t_dx, t_dy);
                            Shape::Path(new_path_shape)
                        } else {
                            shape.clone()
                        }
                    }
                    ShapeType::Text => {
                        if let Some(text) = shape.as_any().downcast_ref::<TextShape>() {
                            Shape::Text(TextShape::new(
                                text.text.clone(),
                                snapped_x1,
                                snapped_y1,
                                text.font_size,
                            ))
                        } else {
                            shape.clone()
                        }
                    }
                    ShapeType::Triangle => {
                        let center = Point::new(
                            snapped_x1 + snapped_width / 2.0,
                            snapped_y1 + snapped_height / 2.0,
                        );
                        Shape::Triangle(Triangle::new(center, snapped_width, snapped_height))
                    }
                    ShapeType::Polygon => {
                        if let Some(poly) = shape.as_any().downcast_ref::<Polygon>() {
                            let radius = snapped_width.min(snapped_height) / 2.0;
                            let center = Point::new(
                                snapped_x1 + snapped_width / 2.0,
                                snapped_y1 + snapped_height / 2.0,
                            );
                            Shape::Polygon(Polygon::new(center, radius, poly.sides))
                        } else {
                            shape.clone()
                        }
                    }
                    ShapeType::Gear => {
                        if let Some(gear) =
                            shape.as_any().downcast_ref::<crate::model::DesignGear>()
                        {
                            let center = Point::new(
                                snapped_x1 + snapped_width / 2.0,
                                snapped_y1 + snapped_height / 2.0,
                            );
                            let module = snapped_width / gear.teeth as f64;
                            Shape::Gear(crate::model::DesignGear::new(center, module, gear.teeth))
                        } else {
                            shape.clone()
                        }
                    }
                    ShapeType::Sprocket => {
                        if let Some(sprocket) = shape
                            .as_any()
                            .downcast_ref::<crate::model::DesignSprocket>()
                        {
                            let center = Point::new(
                                snapped_x1 + snapped_width / 2.0,
                                snapped_y1 + snapped_height / 2.0,
                            );
                            let pitch = snapped_width
                                * (std::f64::consts::PI / sprocket.teeth as f64).sin();
                            Shape::Sprocket(crate::model::DesignSprocket::new(
                                center,
                                pitch,
                                sprocket.teeth,
                            ))
                        } else {
                            shape.clone()
                        }
                    }
                };
                obj.shape = new_shape;

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }
    }
    /// Calculates position and size updates without modifying the canvas.
    pub fn calculate_position_and_size_updates(
        &self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        update_position: bool,
        update_size: bool,
    ) -> Vec<(u64, DrawingObject)> {
        let mut updates = Vec::new();

        // 1. Calculate union bounding box of all selected items
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_selected = false;

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
            has_selected = true;
        }

        if !has_selected {
            return updates;
        }

        let old_w = max_x - min_x;
        let old_h = max_y - min_y;

        // 2. Determine target values
        let target_x = if update_position { x } else { x };
        let target_y = if update_position { y } else { y };
        let target_w = if update_size { w } else { old_w };
        let target_h = if update_size { h } else { old_h };

        // 3. Calculate scale factors
        let sx = if update_size && old_w.abs() > 1e-6 {
            target_w / old_w
        } else {
            1.0
        };
        let sy = if update_size && old_h.abs() > 1e-6 {
            target_h / old_h
        } else {
            1.0
        };

        // Center of the original group
        let group_center_x = min_x + old_w / 2.0;
        let group_center_y = min_y + old_h / 2.0;

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let mut new_obj = obj.clone();

            // Apply scaling relative to group center
            new_obj
                .shape
                .scale(sx, sy, Point::new(group_center_x, group_center_y));

            // Calculate translation to move to target position
            // The group's new center after scaling is still (group_center_x, group_center_y)
            // But its size is (target_w, target_h), so its new top-left is:
            let current_new_x = group_center_x - target_w / 2.0;
            let current_new_y = group_center_y - target_h / 2.0;

            let dx = target_x - current_new_x;
            let dy = target_y - current_new_y;

            new_obj.shape.translate(dx, dy);

            updates.push((obj.id, new_obj));
        }

        updates
    }

    pub fn set_selected_position_and_size_with_flags(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        update_position: bool,
        update_size: bool,
    ) -> bool {
        // 1. Calculate union bounding box
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_selected = false;

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
            has_selected = true;
        }

        if !has_selected {
            return false;
        }

        let old_w = max_x - min_x;
        let old_h = max_y - min_y;

        // 2. Determine target values
        let target_x = if update_position { x } else { x };
        let target_y = if update_position { y } else { y };
        let target_w = if update_size { w } else { old_w };
        let target_h = if update_size { h } else { old_h };

        // 3. Calculate scale factors
        let sx = if update_size && old_w.abs() > 1e-6 {
            target_w / old_w
        } else {
            1.0
        };
        let sy = if update_size && old_h.abs() > 1e-6 {
            target_h / old_h
        } else {
            1.0
        };

        // Center of the original group
        let group_center_x = min_x + old_w / 2.0;
        let group_center_y = min_y + old_h / 2.0;

        let mut changed_any = false;
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if !obj.selected {
                continue;
            }

            let (old_x, old_y, old_x2, old_y2) = obj.get_total_bounds();

            // Apply scaling relative to group center
            obj.shape
                .scale(sx, sy, Point::new(group_center_x, group_center_y));

            // Calculate translation to move to target position
            let current_new_x = group_center_x - target_w / 2.0;
            let current_new_y = group_center_y - target_h / 2.0;

            let dx = target_x - current_new_x;
            let dy = target_y - current_new_y;

            obj.shape.translate(dx, dy);

            changed_any = true;

            let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
            updates.push((
                obj.id,
                Bounds::new(old_x, old_y, old_x2, old_y2),
                Bounds::new(new_x1, new_y1, new_x2, new_y2),
            ));
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }

        changed_any
    }
    /// Calculates text property updates without modifying the canvas.
    pub fn calculate_text_property_updates(
        &self,
        content: &str,
        font_size: f64,
    ) -> Vec<(u64, DrawingObject)> {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            if let Some(text) = obj.shape.as_any().downcast_ref::<TextShape>() {
                let (x, y) = (text.x, text.y);

                let mut new_obj = obj.clone();
                new_obj.shape = Shape::Text(TextShape::new(content.to_string(), x, y, font_size));
                updates.push((obj.id, new_obj));
            }
        }

        updates
    }

    pub fn set_selected_text_properties(&mut self, content: &str, font_size: f64) -> bool {
        let mut changed = false;
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if !obj.selected {
                continue;
            }
            if let Some(text) = obj.shape.as_any().downcast_ref::<TextShape>() {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();
                let (x, y) = (text.x, text.y);

                obj.shape = Shape::Text(TextShape::new(content.to_string(), x, y, font_size));
                changed = true;

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }

        changed
    }

    /// Calculates rectangle property updates without modifying the canvas.
    pub fn calculate_rectangle_property_updates(
        &self,
        corner_radius: f64,
        is_slot: bool,
    ) -> Vec<(u64, DrawingObject)> {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            if let Some(rect) = obj.shape.as_any().downcast_ref::<Rectangle>() {
                let mut new_rect = rect.clone();
                new_rect.corner_radius = corner_radius;
                new_rect.is_slot = is_slot;

                // Re-constrain radius
                let max_radius = new_rect.width.min(new_rect.height).abs() / 2.0;
                if new_rect.is_slot {
                    new_rect.corner_radius = max_radius;
                } else {
                    new_rect.corner_radius = new_rect.corner_radius.min(max_radius);
                }

                let mut new_obj = obj.clone();
                new_obj.shape = Shape::Rectangle(new_rect);
                updates.push((obj.id, new_obj));
            }
        }

        updates
    }

    pub fn set_selected_rectangle_properties(&mut self, corner_radius: f64, is_slot: bool) -> bool {
        let mut changed = false;
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if !obj.selected {
                continue;
            }
            if let Some(rect) = obj.shape.as_any().downcast_ref::<Rectangle>() {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();

                let mut new_rect = rect.clone();
                new_rect.corner_radius = corner_radius;
                new_rect.is_slot = is_slot;

                // Re-constrain radius
                let max_radius = new_rect.width.min(new_rect.height).abs() / 2.0;
                if new_rect.is_slot {
                    new_rect.corner_radius = max_radius;
                } else {
                    new_rect.corner_radius = new_rect.corner_radius.min(max_radius);
                }

                obj.shape = Shape::Rectangle(new_rect);
                changed = true;

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }

        changed
    }

    pub fn set_selected_id(&mut self, id: Option<u64>) {
        self.selection_manager.set_selected_id(id);
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}
