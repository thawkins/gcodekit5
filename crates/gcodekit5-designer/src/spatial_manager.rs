//! # Spatial Manager
//!
//! Wraps the spatial index for efficient shape queries by point or bounding box.
//! Enables O(log n) selection and intersection queries on the canvas.

use crate::model::DesignerShape;
use crate::model::Shape;
use crate::spatial_index::{Bounds, SpatialIndex};

/// Manages the spatial index for efficient shape queries.
///
/// `SpatialManager` wraps the `SpatialIndex` and provides a clean interface for:
/// - Inserting shapes into the spatial index
/// - Removing shapes from the spatial index
/// - Updating shape positions in the index
/// - Querying shapes by point or bounding box
///
/// # Purpose
///
/// The spatial index enables O(log n) queries for:
/// - "What shapes are at this point?" (for selection)
/// - "What shapes intersect this rectangle?" (for drag-select)
///
/// Without spatial indexing, these queries would be O(n), checking every shape.
///
/// # Design
///
/// This manager encapsulates the `SpatialIndex` to provide a simpler API
/// and hide implementation details from the rest of the codebase.
#[derive(Debug, Clone, Default)]
pub struct SpatialManager {
    /// The underlying spatial index structure
    index: SpatialIndex,
}

impl SpatialManager {
    /// Creates a new empty `SpatialManager`.
    ///
    /// # Examples
    ///
    /// ```
    /// use gcodekit5_designer::spatial_manager::SpatialManager;
    ///
    /// let manager = SpatialManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            index: SpatialIndex::default(),
        }
    }

    /// Inserts a shape into the spatial index.
    ///
    /// Calculates the bounding box from the shape and inserts it into the index.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the shape
    /// * `shape` - The shape to index
    ///
    /// # Note
    ///
    /// This method is currently unused. Use `insert_bounds()` instead for better performance
    /// when you already have the bounds calculated.
    #[allow(dead_code)]
    pub fn insert(&mut self, id: u64, shape: &Shape) {
        let (min_x, min_y, max_x, max_y) = shape.bounds();
        self.index
            .insert(id, &Bounds::new(min_x, min_y, max_x, max_y));
    }

    /// Inserts a shape into the spatial index using pre-calculated bounds.
    ///
    /// This is more efficient than `insert()` when you already have the bounds.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the shape
    /// * `bounds` - Pre-calculated bounding box for the shape
    pub fn insert_bounds(&mut self, id: u64, bounds: &Bounds) {
        self.index.insert(id, bounds);
    }

    /// Removes a shape from the spatial index.
    ///
    /// Calculates the bounding box from the shape and removes it from the index.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the shape
    /// * `shape` - The shape to remove (used to calculate bounds)
    ///
    /// # Note
    ///
    /// This method is currently unused. Use `remove_bounds()` instead for better performance
    /// when you already have the bounds calculated.
    #[allow(dead_code)]
    pub fn remove(&mut self, id: u64, shape: &Shape) {
        let (min_x, min_y, max_x, max_y) = shape.bounds();
        self.index
            .remove(id, &Bounds::new(min_x, min_y, max_x, max_y));
    }

    /// Removes a shape from the spatial index using pre-calculated bounds.
    ///
    /// This is more efficient than `remove()` when you already have the bounds.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the shape
    /// * `bounds` - Pre-calculated bounding box for the shape
    pub fn remove_bounds(&mut self, id: u64, bounds: &Bounds) {
        self.index.remove(id, bounds);
    }

    /// Updates a shape's position in the spatial index.
    ///
    /// Removes the old bounds and inserts the new bounds in one operation.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the shape
    /// * `old_shape` - The shape before transformation
    /// * `new_shape` - The shape after transformation
    ///
    /// # Note
    ///
    /// This method is currently unused. Most code manually calls `remove_bounds()`
    /// and `insert_bounds()` for better control.
    #[allow(dead_code)]
    pub fn update(&mut self, id: u64, old_shape: &Shape, new_shape: &Shape) {
        self.remove(id, old_shape);
        self.insert(id, new_shape);
    }

    /// Queries the spatial index for shapes intersecting the given bounds.
    ///
    /// Returns IDs of all shapes whose bounding boxes intersect with the query bounds.
    ///
    /// # Arguments
    ///
    /// * `bounds` - The bounding box to query
    ///
    /// # Returns
    ///
    /// A vector of shape IDs that intersect the query bounds.
    ///
    /// # Note
    ///
    /// This method is currently unused. Use `query_point()` for point queries or
    /// access the index directly via `inner()` for custom queries.
    #[allow(dead_code)]
    pub fn query(&self, bounds: &Bounds) -> Vec<u64> {
        self.index.query(bounds)
    }

    /// Queries the spatial index for shapes containing the given point.
    ///
    /// Returns IDs of all shapes whose bounding boxes contain the point.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate of the query point
    /// * `y` - Y coordinate of the query point
    ///
    /// # Returns
    ///
    /// A vector of shape IDs whose bounding boxes contain the point.
    ///
    /// # Note
    ///
    /// This returns candidates only. You must still perform precise point-in-shape
    /// tests on the returned shapes.
    pub fn query_point(&self, x: f64, y: f64) -> Vec<u64> {
        self.index.query_point(x, y)
    }

    /// Clears all shapes from the spatial index.
    ///
    /// After calling this, the index will be empty.
    pub fn clear(&mut self) {
        self.index.clear();
    }

    /// Returns an immutable reference to the underlying spatial index.
    ///
    /// Use this when you need direct access to the index for advanced queries.
    ///
    /// # Returns
    ///
    /// A reference to the `SpatialIndex`.
    pub fn inner(&self) -> &SpatialIndex {
        &self.index
    }

    /// Returns a mutable reference to the underlying spatial index.
    ///
    /// Use this when you need direct mutable access to the index.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `SpatialIndex`.
    ///
    /// # Note
    ///
    /// This method is currently unused but kept for potential future use.
    #[allow(dead_code)]
    pub fn inner_mut(&mut self) -> &mut SpatialIndex {
        &mut self.index
    }
}
