//! Shape editing operations for the designer canvas

use super::*;
use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::commands::{DesignerCommand, PasteShapes, RemoveShape};
use gcodekit5_designer::model::DesignerShape;

impl DesignerCanvas {
    pub fn delete_selected(&self) {
        let mut state = self.state.borrow_mut();
        let selected_ids: Vec<u64> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();

        for id in selected_ids {
            let cmd = DesignerCommand::RemoveShape(RemoveShape { id, object: None });
            state.push_command(cmd);
        }

        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        self.widget.queue_draw();
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

        let cmd = DesignerCommand::PasteShapes(PasteShapes {
            ids: new_objects.iter().map(|o| o.id).collect(),
            objects: new_objects.into_iter().map(Some).collect(),
        });
        state.push_command(cmd);

        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
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
            for obj in &selected {
                let mut new_obj = obj.clone();
                new_obj.id = state.canvas.generate_id();
                new_obj.shape.translate(dx * i as f64, dy * i as f64);
                new_obj.selected = true;
                new_objects.push(new_obj);
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

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
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

                for obj in &selected {
                    let mut new_obj = obj.clone();
                    new_obj.id = state.canvas.generate_id();
                    new_obj.shape.translate(dx * c as f64, dy * r as f64);
                    new_obj.selected = true;
                    new_objects.push(new_obj);
                }
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

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

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
        let angle_step = total_angle / count as f64;

        for i in 1..count {
            let angle = angle_step * i as f64;

            for obj in &selected {
                let mut new_obj = obj.clone();
                new_obj.id = state.canvas.generate_id();
                new_obj.shape.rotate(angle, center_x, center_y);
                new_obj.selected = true;
                new_objects.push(new_obj);
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

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn group_selected(&self) {
        let mut state = self.state.borrow_mut();
        state.group_selected();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn ungroup_selected(&self) {
        let mut state = self.state.borrow_mut();
        state.ungroup_selected();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn convert_to_path(&self) {
        let mut state = self.state.borrow_mut();
        state.convert_selected_to_path();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn convert_to_rectangle(&self) {
        let mut state = self.state.borrow_mut();
        state.convert_selected_to_rectangle();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn align_left(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_horizontal_left();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_right(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_horizontal_right();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_top(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_vertical_top();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_bottom(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_vertical_bottom();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_center_horizontal(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_horizontal_center();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_center_vertical(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_vertical_center();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn boolean_union(&self) {
        let mut state = self.state.borrow_mut();
        state.perform_boolean_union();
        drop(state);

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }
        if let Some(ref props) = *self.properties.borrow() {
            props.update_from_selection();
        }

        self.widget.queue_draw();
    }

    pub fn boolean_difference(&self) {
        let mut state = self.state.borrow_mut();
        state.perform_boolean_difference();
        drop(state);

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }
        if let Some(ref props) = *self.properties.borrow() {
            props.update_from_selection();
        }

        self.widget.queue_draw();
    }

    pub fn boolean_intersection(&self) {
        let mut state = self.state.borrow_mut();
        state.perform_boolean_intersection();
        drop(state);

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }
        if let Some(ref props) = *self.properties.borrow() {
            props.update_from_selection();
        }

        self.widget.queue_draw();
    }

    pub fn mirror_x(&self) {
        let mut state = self.state.borrow_mut();
        state.mirror_selected_x();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn mirror_y(&self) {
        let mut state = self.state.borrow_mut();
        state.mirror_selected_y();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn set_selected_rotation(&self, rotation: f64) {
        let mut state = self.state.borrow_mut();
        state.set_selected_rotation(rotation);
        drop(state);
        self.widget.queue_draw();
    }

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

    pub fn paste(&self) {
        let mut state = self.state.borrow_mut();
        if state.clipboard.is_empty() {
            return;
        }

        // Clone clipboard before using it
        let clipboard = state.clipboard.clone();

        // Deselect all current shapes
        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        // Create copies with offset
        let offset = 10.0;
        let mut new_objects = Vec::new();
        for obj in &clipboard {
            let mut new_obj = obj.clone();
            new_obj.id = state.canvas.generate_id();
            new_obj.shape.translate(offset, offset);
            new_obj.selected = true;
            new_objects.push(new_obj);
        }

        let cmd = DesignerCommand::PasteShapes(PasteShapes {
            ids: new_objects.iter().map(|o| o.id).collect(),
            objects: new_objects.into_iter().map(Some).collect(),
        });
        state.push_command(cmd);

        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn undo(&self) {
        let mut state = self.state.borrow_mut();
        state.undo();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn redo(&self) {
        let mut state = self.state.borrow_mut();
        state.redo();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }
}
