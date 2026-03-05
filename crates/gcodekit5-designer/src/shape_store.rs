//! # Shape Store
//!
//! Manages the storage and retrieval of drawing objects (shapes) on the canvas.
//! Uses a HashMap for O(1) lookup by ID and a Vec for maintaining draw order.

use crate::canvas::DrawingObject;
use std::collections::HashMap;

/// Manages the storage and retrieval of drawing objects (shapes) on the canvas.
///
/// `ShapeStore` is responsible for:
/// - Storing shapes in a HashMap for O(1) lookup by ID
/// - Maintaining draw order for proper z-ordering during rendering
/// - Generating unique IDs for new shapes
/// - Providing iterators for accessing shapes in draw order
///
/// # Design
///
/// The store uses two data structures:
/// - `HashMap<u64, DrawingObject>`: Fast lookup by ID
/// - `Vec<u64>`: Maintains the order shapes should be drawn (back to front)
///
/// This dual structure allows both fast access and correct rendering order.
#[derive(Debug, Clone)]
pub struct ShapeStore {
    /// Map of shape IDs to their corresponding DrawingObject instances
    shapes: HashMap<u64, DrawingObject>,
    /// Ordered list of shape IDs defining draw order (back to front)
    draw_order: Vec<u64>,
    /// Counter for generating unique shape IDs
    next_id: u64,
}

impl ShapeStore {
    /// Creates a new empty `ShapeStore`.
    ///
    /// # Examples
    ///
    /// ```
    /// use gcodekit5_designer::shape_store::ShapeStore;
    ///
    /// let store = ShapeStore::new();
    /// assert!(store.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            shapes: HashMap::new(),
            draw_order: Vec::new(),
            next_id: 1,
        }
    }

    /// Generates a new unique ID for a shape.
    ///
    /// IDs are generated sequentially starting from 1. Each call increments
    /// the internal counter, ensuring uniqueness.
    ///
    /// # Returns
    ///
    /// A unique `u64` identifier for a new shape.
    pub fn generate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Inserts a shape into the store with the given ID.
    ///
    /// The shape is added to both the HashMap and the draw order list.
    /// Shapes added later appear on top (drawn last).
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the shape
    /// * `object` - The DrawingObject to store
    ///
    /// # Note
    ///
    /// If a shape with the same ID already exists, it will be replaced,
    /// but the draw order will have a duplicate entry. Use `remove` first
    /// if replacing an existing shape.
    pub fn insert(&mut self, id: u64, object: DrawingObject) {
        self.shapes.insert(id, object);
        self.draw_order.push(id);
    }

    /// Removes a shape from the store by ID.
    ///
    /// Removes the shape from both the HashMap and the draw order list.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shape to remove
    ///
    /// # Returns
    ///
    /// The removed `DrawingObject` if it existed, or `None` if not found.
    pub fn remove(&mut self, id: u64) -> Option<DrawingObject> {
        if let Some(obj) = self.shapes.remove(&id) {
            if let Some(pos) = self.draw_order.iter().position(|&x| x == id) {
                self.draw_order.remove(pos);
            }
            Some(obj)
        } else {
            None
        }
    }

    /// Gets an immutable reference to a shape by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shape to retrieve
    ///
    /// # Returns
    ///
    /// A reference to the `DrawingObject` if found, or `None` if not found.
    pub fn get(&self, id: u64) -> Option<&DrawingObject> {
        self.shapes.get(&id)
    }

    /// Gets a mutable reference to a shape by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shape to retrieve
    ///
    /// # Returns
    ///
    /// A mutable reference to the `DrawingObject` if found, or `None` if not found.
    pub fn get_mut(&mut self, id: u64) -> Option<&mut DrawingObject> {
        self.shapes.get_mut(&id)
    }

    /// Returns an iterator over all shapes in draw order (back to front).
    ///
    /// This iterator respects the z-ordering of shapes, yielding them in the
    /// order they should be rendered.
    ///
    /// # Returns
    ///
    /// An iterator yielding immutable references to `DrawingObject`s in draw order.
    pub fn iter(&self) -> impl Iterator<Item = &DrawingObject> {
        self.draw_order.iter().filter_map(|id| self.shapes.get(id))
    }

    /// Returns a mutable iterator over all shapes.
    ///
    /// # Note
    ///
    /// This iterator does NOT respect draw order due to Rust's borrowing rules.
    /// Use this when you need to modify shapes but don't care about order.
    /// For ordered iteration, use `iter()`.
    ///
    /// # Returns
    ///
    /// An iterator yielding mutable references to `DrawingObject`s in arbitrary order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut DrawingObject> {
        // We can't return iterator based on draw_order easily because of mutable borrowing.
        // But HashMap values_mut is fine, order doesn't matter for mutable iteration usually.
        // If order matters, we have a problem.
        // For rendering (iter), order matters.
        // For updates (iter_mut), order usually doesn't matter.
        self.shapes.values_mut()
    }

    /// Returns an iterator over shape IDs in draw order.
    ///
    /// This is useful when you need to iterate by ID rather than by reference,
    /// or when you need reverse iteration (front to back).
    ///
    /// # Returns
    ///
    /// A double-ended iterator yielding shape IDs in draw order.
    pub fn draw_order_iter(&self) -> impl DoubleEndedIterator<Item = u64> + '_ {
        self.draw_order.iter().copied()
    }

    /// Returns the number of shapes in the store.
    ///
    /// # Returns
    ///
    /// The count of shapes currently stored.
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// Checks if the store contains any shapes.
    ///
    /// # Returns
    ///
    /// `true` if the store is empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Removes all shapes from the store and resets the ID counter.
    ///
    /// After calling this method, the store will be empty and the next
    /// generated ID will be 1.
    pub fn clear(&mut self) {
        self.shapes.clear();
        self.draw_order.clear();
        self.next_id = 1;
    }

    /// Checks if a shape with the given ID exists in the store.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to check for
    ///
    /// # Returns
    ///
    /// `true` if a shape with this ID exists, `false` otherwise.
    pub fn contains(&self, id: u64) -> bool {
        self.shapes.contains_key(&id)
    }

    /// Brings a shape to the front (top of draw order).
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shape to bring to front
    pub fn bring_to_front(&mut self, id: u64) {
        if let Some(pos) = self.draw_order.iter().position(|&x| x == id) {
            self.draw_order.remove(pos);
            self.draw_order.push(id);
        }
    }

    /// Sends a shape to the back (bottom of draw order).
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shape to send to back
    pub fn send_to_back(&mut self, id: u64) {
        if let Some(pos) = self.draw_order.iter().position(|&x| x == id) {
            self.draw_order.remove(pos);
            self.draw_order.insert(0, id);
        }
    }

    /// Moves a shape forward one position in draw order.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shape to move forward
    pub fn bring_forward(&mut self, id: u64) {
        if let Some(pos) = self.draw_order.iter().position(|&x| x == id) {
            if pos < self.draw_order.len() - 1 {
                self.draw_order.swap(pos, pos + 1);
            }
        }
    }

    /// Moves a shape backward one position in draw order.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shape to move backward
    pub fn send_backward(&mut self, id: u64) {
        if let Some(pos) = self.draw_order.iter().position(|&x| x == id) {
            if pos > 0 {
                self.draw_order.swap(pos, pos - 1);
            }
        }
    }
    /// Sets the next ID to be generated.
    ///
    /// This is useful when loading shapes from a file to ensure new IDs
    /// don't conflict with loaded IDs.
    pub fn set_next_id(&mut self, id: u64) {
        self.next_id = id;
    }
}

impl Default for ShapeStore {
    fn default() -> Self {
        Self::new()
    }
}
