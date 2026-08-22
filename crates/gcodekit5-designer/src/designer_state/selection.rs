//! Selection operations for designer state.

use super::DesignerState;
use crate::DrawingMode;

impl DesignerState {
    /// Get number of selected shapes.
    pub fn selected_count(&self) -> usize {
        self.canvas.selected_count()
    }

    /// Deselects all shapes.
    pub fn deselect_all(&mut self) {
        self.canvas.deselect_all();
    }

    /// Selects all shapes.
    pub fn select_all(&mut self) {
        self.canvas.select_all();
    }

    /// Selects shapes within the given rectangle.
    pub fn select_in_rect(&mut self, x: f64, y: f64, width: f64, height: f64, multi_select: bool) {
        if self.canvas.mode() == DrawingMode::Select {
            self.canvas
                .select_in_rect(x, y, width, height, multi_select);
        }
    }

    /// Select the next shape in draw order.
    pub fn select_next_shape(&mut self) {
        let selected_id = self.canvas.selected_id();
        let ids: Vec<u64> = self.canvas.shape_store.draw_order_iter().collect();

        if ids.is_empty() {
            return;
        }

        let new_id = if let Some(id) = selected_id {
            if let Some(pos) = ids.iter().position(|&x| x == id) {
                if pos + 1 < ids.len() {
                    ids[pos + 1]
                } else {
                    ids[ids.len() - 1]
                }
            } else {
                ids[0]
            }
        } else {
            ids[0]
        };

        self.canvas.select_shape(new_id, false);
    }

    /// Select the previous shape in draw order.
    pub fn select_previous_shape(&mut self) {
        let selected_id = self.canvas.selected_id();
        let ids: Vec<u64> = self.canvas.shape_store.draw_order_iter().collect();

        if ids.is_empty() {
            return;
        }

        let new_id = if let Some(id) = selected_id {
            if let Some(pos) = ids.iter().position(|&x| x == id) {
                if pos > 0 {
                    ids[pos - 1]
                } else {
                    ids[0]
                }
            } else {
                ids[0]
            }
        } else {
            ids[0]
        };

        self.canvas.select_shape(new_id, false);
    }

    /// Sets the lock aspect ratio flag for the currently selected shape.
    pub fn set_selected_lock_aspect_ratio(&mut self, locked: bool) {
        // We obtain the ID of the selected object (in singular)
        if let Some(id) = self.canvas.selected_id() {
            // We look for that object in the shape_store
            // If get a 'borrow' error, try: self.canvas.shape_store.get_mut(id)
            if let Some(shape) = self.canvas.shape_store.get_mut(id) {
                // We changed the real value
                shape.lock_aspect_ratio = locked;
            }
        }
    }
}
