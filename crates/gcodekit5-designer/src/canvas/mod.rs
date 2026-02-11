//! Canvas for drawing and manipulating shapes.

mod operations;
mod types;

pub use types::{Alignment, CanvasPoint, CanvasSnapshot, DrawingMode, DrawingObject};

use super::spatial_index::Bounds;
use super::viewport::Viewport;
use crate::model::{
    DesignCircle as Circle, DesignEllipse as Ellipse, DesignLine as Line, DesignPath as PathShape,
    DesignPolygon as Polygon, DesignRectangle as Rectangle, DesignText as TextShape,
    DesignTriangle as Triangle, DesignerShape, Point, Shape,
};
use crate::selection_manager::SelectionManager;
use crate::shape_store::ShapeStore;
use crate::spatial_manager::SpatialManager;

/// Canvas state managing shapes and drawing operations.
#[derive(Debug, Clone)]
pub struct Canvas {
    pub shape_store: ShapeStore,
    pub selection_manager: SelectionManager,
    pub spatial_manager: SpatialManager,
    mode: DrawingMode,
    viewport: Viewport,
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

    pub fn set_selected_id(&mut self, id: Option<u64>) {
        self.selection_manager.set_selected_id(id);
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}
