use crate::shape_store::ShapeStore;
use crate::shapes::Point;
use crate::spatial_index::{Bounds, SpatialIndex};
use std::collections::{HashMap, HashSet};

/// Manages shape selection state and selection operations.
///
/// `SelectionManager` is responsible for:
/// - Tracking which shape is the "primary" selected shape
/// - Handling point-based selection (clicking on shapes)
/// - Handling rectangle-based selection (drag-select)
/// - Managing group selection (selecting all shapes in a group)
/// - Multi-select operations (Shift+click)
///
/// # Selection Model
///
/// - **Primary Selection**: One shape is designated as the "primary" selection (stored in `selected_id`)
/// - **Multiple Selection**: Multiple shapes can have their `selected` flag set to `true`
/// - **Group Selection**: Clicking on any shape in a group selects the entire group
/// - **Multi-select**: Holding Shift allows toggling selection without deselecting others
///
/// # Design
///
/// The manager coordinates with `ShapeStore` to modify selection flags and uses
/// `SpatialIndex` for efficient hit-testing during selection operations.
#[derive(Debug, Clone, Default)]
pub struct SelectionManager {
    /// The ID of the primary selected shape, if any
    selected_id: Option<u64>,
}

impl SelectionManager {
    /// Creates a new `SelectionManager` with no selection.
    ///
    /// # Examples
    ///
    /// ```
    /// use gcodekit5_designer::selection_manager::SelectionManager;
    ///
    /// let manager = SelectionManager::new();
    /// assert_eq!(manager.selected_id(), None);
    /// ```
    pub fn new() -> Self {
        Self { selected_id: None }
    }

    /// Returns the ID of the primary selected shape.
    ///
    /// # Returns
    ///
    /// `Some(id)` if a shape is selected, `None` otherwise.
    pub fn selected_id(&self) -> Option<u64> {
        self.selected_id
    }

    /// Sets the primary selected shape ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the shape to set as primary, or `None` to clear
    ///
    /// # Note
    ///
    /// This does NOT modify the `selected` flag on shapes. Use `deselect_all()`
    /// or modify shapes directly if you need to update their selection state.
    pub fn set_selected_id(&mut self, id: Option<u64>) {
        self.selected_id = id;
    }

    /// Deselects all shapes and clears the primary selection.
    ///
    /// Sets the `selected` flag to `false` on all shapes in the store
    /// and clears the primary selection ID.
    ///
    /// # Arguments
    ///
    /// * `store` - The shape store containing shapes to deselect
    pub fn deselect_all(&mut self, store: &mut ShapeStore) {
        for obj in store.iter_mut() {
            obj.selected = false;
        }
        self.selected_id = None;
    }

    /// Selects all shapes in the store.
    ///
    /// Sets the `selected` flag to `true` on all shapes and sets the
    /// primary selection to the topmost (last in draw order) shape.
    ///
    /// # Arguments
    ///
    /// * `store` - The shape store containing shapes to select
    pub fn select_all(&mut self, store: &mut ShapeStore) {
        for obj in store.iter_mut() {
            obj.selected = true;
        }
        // Set selected_id to the last one if any exist
        if let Some(last_id) = store.draw_order_iter().last() {
            self.selected_id = Some(last_id);
        }
    }

    /// Selects the topmost shape at the given point.
    ///
    /// Uses the spatial index for efficient hit-testing, then performs precise
    /// point-in-shape tests on candidates. If the clicked shape is part of a group,
    /// the entire group is selected.
    ///
    /// # Arguments
    ///
    /// * `store` - The shape store to select from
    /// * `spatial_index` - Spatial index for efficient hit-testing
    /// * `point` - The point to test for selection
    /// * `multi` - If `true`, enables multi-select mode (Shift+click behavior)
    ///
    /// # Multi-select Behavior
    ///
    /// - If `multi` is `false`: Deselects all other shapes before selecting
    /// - If `multi` is `true`: Toggles selection without affecting other shapes
    ///
    /// # Returns
    ///
    /// The ID of the newly selected shape, or `None` if no shape was clicked.
    pub fn select_at(
        &mut self,
        store: &mut ShapeStore,
        spatial_index: &SpatialIndex,
        point: &Point,
        tolerance: f64,
        multi: bool,
    ) -> Option<u64> {
        let mut found_id = None;
        let mut found_group_id = None;

        // Query spatial index for candidates (used for single shapes)
        let candidates = spatial_index.query_point(point.x, point.y);

        // Pre-calculate group bounding boxes
        let mut group_bounds: HashMap<u64, (f64, f64, f64, f64)> = HashMap::new();
        for obj in store.iter() {
            if let Some(gid) = obj.group_id {
                let (sx1, sy1, sx2, sy2) = obj.shape.bounding_box();
                group_bounds
                    .entry(gid)
                    .and_modify(|(min_x, min_y, max_x, max_y)| {
                        *min_x = min_x.min(sx1);
                        *min_y = min_y.min(sy1);
                        *max_x = max_x.max(sx2);
                        *max_y = max_y.max(sy2);
                    })
                    .or_insert((sx1, sy1, sx2, sy2));
            }
        }

        let mut processed_groups = HashSet::new();

        // Find the shape at the point (topmost first)
        // We iterate in reverse draw order
        // candidates_at_point logged temporarily during debugging
        for id in store.draw_order_iter().rev() {
            if let Some(obj) = store.get(id) {
                if let Some(gid) = obj.group_id {
                    // Handle group selection: check composite bounding box
                    if processed_groups.contains(&gid) {
                        continue;
                    }
                    processed_groups.insert(gid);

                    if let Some(&(min_x, min_y, max_x, max_y)) = group_bounds.get(&gid) {
                        if point.x >= min_x - tolerance
                            && point.x <= max_x + tolerance
                            && point.y >= min_y - tolerance
                            && point.y <= max_y + tolerance
                        {
                            found_id = Some(obj.id);
                            found_group_id = Some(gid);
                            break;
                        }
                    }
                } else {
                    // Handle single shape selection: use precise hit test
                    if candidates.contains(&obj.id) {
                        if obj.shape.contains_point(point, tolerance) {
                            found_id = Some(obj.id);
                            found_group_id = None;
                            break;
                        }
                    }
                }
            }
        }

        if !multi {
            self.deselect_all(store);
        }

        if let Some(id) = found_id {
            // Determine which IDs to select (single shape or whole group)
            let ids_to_select: Vec<u64> = if let Some(gid) = found_group_id {
                store
                    .iter()
                    .filter(|o| o.group_id == Some(gid))
                    .map(|o| o.id)
                    .collect()
            } else {
                vec![id]
            };

            // If multi-select, check if we should toggle off (only if all are already selected)
            let all_selected = ids_to_select
                .iter()
                .all(|&sid| store.get(sid).map(|o| o.selected).unwrap_or(false));

            let should_select = if multi { !all_selected } else { true };

            for sid in ids_to_select {
                if let Some(obj) = store.get_mut(sid) {
                    obj.selected = should_select;
                }
            }

            if should_select {
                self.selected_id = Some(id); // Set primary to the clicked one
            } else if self.selected_id == Some(id) {
                self.selected_id = None;
                // Try to find another selected shape
                if let Some(other) = store.iter().find(|o| o.selected) {
                    self.selected_id = Some(other.id);
                }
            }
        } else if !multi {
            // Clicked on empty space without shift -> deselect all
            self.selected_id = None;
        }

        self.selected_id
    }

    /// Selects all shapes that intersect with the given rectangle.
    ///
    /// Uses the spatial index to find candidate shapes, then performs precise
    /// bounding box intersection tests. If any shape in a group is selected,
    /// the entire group is selected.
    ///
    /// # Arguments
    ///
    /// * `store` - The shape store to select from
    /// * `spatial_index` - Spatial index for efficient querying
    /// * `x` - X coordinate of rectangle corner
    /// * `y` - Y coordinate of rectangle corner
    /// * `width` - Width of selection rectangle (can be negative)
    /// * `height` - Height of selection rectangle (can be negative)
    /// * `multi` - If `true`, adds to existing selection; if `false`, replaces it
    ///
    /// # Note
    ///
    /// The rectangle is automatically normalized (negative width/height are handled).
    pub fn select_in_rect(
        &mut self,
        store: &mut ShapeStore,
        spatial_index: &SpatialIndex,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        multi: bool,
    ) {
        if !multi {
            self.deselect_all(store);
        }

        // Normalize rectangle
        let (rx, rw) = if width < 0.0 {
            (x + width, -width)
        } else {
            (x, width)
        };
        let (ry, rh) = if height < 0.0 {
            (y + height, -height)
        } else {
            (y, height)
        };
        let rect_bounds = Bounds::new(rx, ry, rx + rw, ry + rh);

        // Query spatial index for candidates
        let candidates = spatial_index.query(&rect_bounds);

        let mut groups_to_select = Vec::new();

        for obj in store.iter_mut() {
            if candidates.contains(&obj.id) {
                let (sx1, sy1, sx2, sy2) = obj.shape.bounding_box();
                // Check for intersection
                if sx1 < rx + rw && sx2 > rx && sy1 < ry + rh && sy2 > ry {
                    obj.selected = true;
                    if let Some(gid) = obj.group_id {
                        if !groups_to_select.contains(&gid) {
                            groups_to_select.push(gid);
                        }
                    }
                    // If we just selected something and there was no primary selection, set it
                    if self.selected_id.is_none() {
                        self.selected_id = Some(obj.id);
                    }
                }
            }
        }

        // Select all members of intersected groups
        if !groups_to_select.is_empty() {
            for obj in store.iter_mut() {
                if let Some(gid) = obj.group_id {
                    if groups_to_select.contains(&gid) {
                        obj.selected = true;
                    }
                }
            }
        }
    }

    /// Selects a shape by ID.
    ///
    /// # Arguments
    ///
    /// * `store` - The shape store to select from
    /// * `id` - The ID of the shape to select
    /// * `multi` - If `true`, adds to existing selection; if `false`, replaces it
    pub fn select_id(&mut self, store: &mut ShapeStore, id: u64, multi: bool) {
        if !multi {
            self.deselect_all(store);
        }

        let mut group_id_to_select = None;

        if let Some(obj) = store.get(id) {
            group_id_to_select = obj.group_id;
        }

        if let Some(gid) = group_id_to_select {
            for obj in store.iter_mut() {
                if obj.group_id == Some(gid) {
                    obj.selected = true;
                }
            }
            self.selected_id = Some(id);
        } else if let Some(obj) = store.get_mut(id) {
            obj.selected = true;
            self.selected_id = Some(id);
        }
    }

    /// Returns the number of currently selected shapes.
    ///
    /// Counts all shapes in the store that have their `selected` flag set to `true`.
    ///
    /// # Arguments
    ///
    /// * `store` - The shape store to count selections from
    ///
    /// # Returns
    ///
    /// The count of selected shapes.
    pub fn selected_count(&self, store: &ShapeStore) -> usize {
        store.iter().filter(|o| o.selected).count()
    }

    /// Removes all selected shapes from the store and spatial index.
    ///
    /// Deletes shapes that have their `selected` flag set to `true`, removes them
    /// from the spatial index, and clears the primary selection.
    ///
    /// # Arguments
    ///
    /// * `store` - The shape store to remove shapes from
    /// * `spatial_index` - The spatial index to update
    pub fn remove_selected(&mut self, store: &mut ShapeStore, spatial_index: &mut SpatialIndex) {
        let selected_ids: Vec<u64> = store.iter().filter(|o| o.selected).map(|o| o.id).collect();

        for id in selected_ids {
            if let Some(obj) = store.remove(id) {
                let (min_x, min_y, max_x, max_y) = obj.shape.bounding_box();
                spatial_index.remove(id, &Bounds::new(min_x, min_y, max_x, max_y));
            }
        }
        self.selected_id = None;
    }
}
