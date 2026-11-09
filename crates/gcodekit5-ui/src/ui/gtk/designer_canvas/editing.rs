//! Shape editing operations for the designer canvas

use super::*;
use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::commands::{DesignerCommand, PasteShapes};
use gcodekit5_designer::model::DesignerShape;

/// Give copied objects fresh group ids so that copies form their own groups instead
/// of joining (and being selected together with) the groups of the originals.
fn assign_fresh_group_ids(
    state: &mut gcodekit5_designer::designer_state::DesignerState,
    objects: &mut [DrawingObject],
) {
    let mut map: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
    for obj in objects.iter_mut() {
        if let Some(gid) = obj.group_id {
            let new_gid = *map.entry(gid).or_insert_with(|| state.canvas.generate_id());
            obj.group_id = Some(new_gid);
        }
    }
}

impl DesignerCanvas {
    /// Deletes all selected shapes as a single undo step.
    pub fn delete_selected(&self) {
        // `DesignerState::delete_selected` records one composite command, so a single
        // undo restores every deleted shape (at its original z-order position).
        self.state.borrow_mut().delete_selected();

        self.notify_model_changed();
    }

    pub fn duplicate_selected(&self) {
        let mut state = self.state.borrow_mut();
        let selected: Vec<DrawingObject> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();

        if selected.is_empty() {
            return;
        }

        // Deselect all current shapes
        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        // Create duplicates with offset
        let offset = 10.0;
        let mut new_objects = Vec::new();
        for mut obj in selected {
            obj.id = state.canvas.generate_id();
            obj.shape.translate(offset, offset);
            obj.selected = true;
            new_objects.push(obj);
        }
        assign_fresh_group_ids(&mut state, &mut new_objects);

        let cmd = DesignerCommand::PasteShapes(PasteShapes {
            ids: new_objects.iter().map(|o| o.id).collect(),
            objects: new_objects.into_iter().map(Some).collect(),
        });
        state.push_command(cmd);

        drop(state);

        self.notify_model_changed();
    }

    pub fn create_linear_array(&self, count: usize, dx: f64, dy: f64) {
        let mut state = self.state.borrow_mut();
        let selected: Vec<DrawingObject> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();

        if selected.is_empty() {
            return;
        }

        // Deselect original shapes
        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        let mut new_objects = Vec::new();

        // For each selected object, create count copies
        for i in 1..count {
            let mut copies = Vec::new();
            for obj in &selected {
                let mut new_obj = obj.clone();
                new_obj.id = state.canvas.generate_id();
                new_obj.shape.translate(dx * i as f64, dy * i as f64);
                new_obj.selected = true;
                copies.push(new_obj);
            }
            assign_fresh_group_ids(&mut state, &mut copies);
            new_objects.extend(copies);
        }

        // Re-select original items
        for obj in state.canvas.shapes_mut() {
            if selected.iter().any(|s| s.id == obj.id) {
                obj.selected = true;
            }
        }

        if !new_objects.is_empty() {
            let cmd = DesignerCommand::PasteShapes(PasteShapes {
                ids: new_objects.iter().map(|o| o.id).collect(),
                objects: new_objects.into_iter().map(Some).collect(),
            });
            state.push_command(cmd);
        }

        drop(state);

        self.notify_model_changed();
    }

    pub fn create_grid_array(&self, rows: usize, cols: usize, dx: f64, dy: f64) {
        let mut state = self.state.borrow_mut();
        let selected: Vec<DrawingObject> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();

        if selected.is_empty() {
            return;
        }

        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        let mut new_objects = Vec::new();

        for r in 0..rows {
            for c in 0..cols {
                if r == 0 && c == 0 {
                    continue;
                } // Skip original position

                let mut copies = Vec::new();
                for obj in &selected {
                    let mut new_obj = obj.clone();
                    new_obj.id = state.canvas.generate_id();
                    new_obj.shape.translate(dx * c as f64, dy * r as f64);
                    new_obj.selected = true;
                    copies.push(new_obj);
                }
                assign_fresh_group_ids(&mut state, &mut copies);
                new_objects.extend(copies);
            }
        }

        // Re-select original items
        for obj in state.canvas.shapes_mut() {
            if selected.iter().any(|s| s.id == obj.id) {
                obj.selected = true;
            }
        }

        if !new_objects.is_empty() {
            let cmd = DesignerCommand::PasteShapes(PasteShapes {
                ids: new_objects.iter().map(|o| o.id).collect(),
                objects: new_objects.into_iter().map(Some).collect(),
            });
            state.push_command(cmd);
        }

        drop(state);

        self.notify_model_changed();
    }

    /// Creates `count - 1` rotated copies of the selection about (`center_x`, `center_y`)
    /// spread over `total_angle` degrees (see [`gcodekit5_designer::arrays::circular_array_step`] for the spacing).
    pub fn create_circular_array(
        &self,
        count: usize,
        center_x: f64,
        center_y: f64,
        total_angle: f64,
    ) {
        let mut state = self.state.borrow_mut();
        let selected: Vec<DrawingObject> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();

        if selected.is_empty() {
            return;
        }

        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        let mut new_objects = Vec::new();
        let angle_step = gcodekit5_designer::arrays::circular_array_step(total_angle, count);

        for i in 1..count {
            let angle = angle_step * i as f64;

            let mut copies = Vec::new();
            for obj in &selected {
                let mut new_obj = obj.clone();
                new_obj.id = state.canvas.generate_id();
                new_obj.shape.rotate(angle, center_x, center_y);
                new_obj.selected = true;
                copies.push(new_obj);
            }
            assign_fresh_group_ids(&mut state, &mut copies);
            new_objects.extend(copies);
        }

        // Re-select original items
        for obj in state.canvas.shapes_mut() {
            if selected.iter().any(|s| s.id == obj.id) {
                obj.selected = true;
            }
        }

        if !new_objects.is_empty() {
            let cmd = DesignerCommand::PasteShapes(PasteShapes {
                ids: new_objects.iter().map(|o| o.id).collect(),
                objects: new_objects.into_iter().map(Some).collect(),
            });
            state.push_command(cmd);
        }

        drop(state);

        self.notify_model_changed();
    }

    pub fn group_selected(&self) {
        let mut state = self.state.borrow_mut();
        state.group_selected();
        drop(state);

        self.notify_model_changed();
    }

    pub fn ungroup_selected(&self) {
        let mut state = self.state.borrow_mut();
        state.ungroup_selected();
        drop(state);

        self.notify_model_changed();
    }

    pub fn convert_to_path(&self) {
        let mut state = self.state.borrow_mut();
        state.convert_selected_to_path();
        drop(state);

        self.notify_model_changed();
    }

    pub fn convert_to_rectangle(&self) {
        let mut state = self.state.borrow_mut();
        state.convert_selected_to_rectangle();
        drop(state);

        self.notify_model_changed();
    }

    pub fn align_left(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_horizontal_left();
        drop(state);
        self.notify_model_changed();
    }

    pub fn align_right(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_horizontal_right();
        drop(state);
        self.notify_model_changed();
    }

    pub fn align_top(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_vertical_top();
        drop(state);
        self.notify_model_changed();
    }

    pub fn align_bottom(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_vertical_bottom();
        drop(state);
        self.notify_model_changed();
    }

    pub fn align_center_horizontal(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_horizontal_center();
        drop(state);
        self.notify_model_changed();
    }

    pub fn align_center_vertical(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_vertical_center();
        drop(state);
        self.notify_model_changed();
    }

    pub fn boolean_union(&self) {
        let mut state = self.state.borrow_mut();
        state.perform_boolean_union();
        drop(state);

        self.notify_model_changed();
    }

    pub fn boolean_difference(&self) {
        let mut state = self.state.borrow_mut();
        state.perform_boolean_difference();
        drop(state);

        self.notify_model_changed();
    }

    pub fn boolean_intersection(&self) {
        let mut state = self.state.borrow_mut();
        state.perform_boolean_intersection();
        drop(state);

        self.notify_model_changed();
    }

    pub fn mirror_x(&self) {
        let mut state = self.state.borrow_mut();
        state.mirror_selected_x();
        drop(state);
        self.notify_model_changed();
    }

    pub fn mirror_y(&self) {
        let mut state = self.state.borrow_mut();
        state.mirror_selected_y();
        drop(state);
        self.notify_model_changed();
    }

    pub fn set_selected_rotation(&self, rotation: f64) {
        let mut state = self.state.borrow_mut();
        state.set_selected_rotation(rotation);
        drop(state);
        self.notify_model_changed();
    }

    /// Rotate the selection by `delta_degrees` relative to its current rotation.
    ///
    /// `DesignerState::set_selected_rotation` takes an absolute angle (degrees) for a
    /// single shape but a delta for multi-selection, so translate accordingly.
    pub fn rotate_selected_by(&self, delta_degrees: f64) {
        let mut state = self.state.borrow_mut();
        let current: Option<f64> = state
            .canvas
            .shapes()
            .find(|s| s.selected)
            .map(|s| s.shape.rotation());
        if state.selected_count() > 1 {
            state.set_selected_rotation(delta_degrees);
        } else if let Some(current) = current {
            let mut target = (current + delta_degrees).rem_euclid(360.0);
            if target > 180.0 {
                target -= 360.0;
            }
            state.set_selected_rotation(target);
        }
        drop(state);
        self.notify_model_changed();
    }

    /// Copies the selected shapes to the designer clipboard.
    pub fn copy_selected(&self) {
        let mut state = self.state.borrow_mut();
        state.clipboard = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();
    }

    pub fn cut(&self) {
        self.copy_selected();
        self.delete_selected();
    }

    /// Pega el portapapeles centrado en las coordenadas world `(target_x, target_y)`.
    pub fn paste_at(&self, target_x: f64, target_y: f64) {
        let mut state = self.state.borrow_mut();
        if state.clipboard.is_empty() {
            return;
        }
        state.paste_at_location(target_x, target_y);
        drop(state);
        self.notify_model_changed();
    }

    /// Pega el portapapeles en la posición actual del cursor del canvas.
    pub fn paste(&self) {
        let (x, y) = *self.mouse_pos.borrow();
        self.paste_at(x, y);
    }

    pub fn undo(&self) {
        let mut state = self.state.borrow_mut();
        state.undo();
        drop(state);

        self.notify_model_changed();
    }

    pub fn redo(&self) {
        let mut state = self.state.borrow_mut();
        state.redo();
        drop(state);

        self.notify_model_changed();
    }
}
