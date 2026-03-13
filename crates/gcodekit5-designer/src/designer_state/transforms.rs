//! Transform operations (move, resize, align, mirror, snap) for designer state.

use super::DesignerState;
use crate::commands::*;
use crate::model::DesignerShape;
use crate::Point;

#[derive(Copy, Clone)]
enum MirrorAxis {
    X,
    Y,
}

impl DesignerState {
    /// Moves the selected shape by (dx, dy).
    pub fn move_selected(&mut self, dx: f64, dy: f64) {
        let ids: Vec<u64> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();
        if ids.is_empty() {
            return;
        }

        let cmd = DesignerCommand::MoveShapes(MoveShapes { ids, dx, dy });
        self.push_command(cmd);
    }

    /// Resizes the selected shape via handle drag.
    /// handle: 0=TL, 1=TR, 2=BL, 3=BR, 4=Center (move)
    pub fn resize_selected(&mut self, handle: usize, dx: f64, dy: f64) {
        let ids: Vec<u64> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();
        if ids.is_empty() {
            return;
        }

        // Calculate bounding box of ALL selected shapes
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for id in &ids {
            if let Some(obj) = self.canvas.get_shape(*id) {
                let (x1, y1, x2, y2) = obj.shape.bounds();
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
            }
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

        let t_dx = new_center_x - center_x;
        let t_dy = new_center_y - center_y;

        let mut commands = Vec::new();
        for id in ids {
            if let Some(obj) = self.canvas.get_shape(id) {
                let old_shape = obj.shape.clone();
                let mut new_shape = old_shape.clone();

                // Scale relative to the center of the SELECTION bounding box
                new_shape.scale(sx, sy, Point::new(center_x, center_y));

                // Translate to new center
                new_shape.translate(t_dx, t_dy);

                commands.push(DesignerCommand::ResizeShape(ResizeShape {
                    id,
                    handle,
                    dx,
                    dy,
                    old_shape: Some(old_shape),
                    new_shape: Some(new_shape),
                }));
            }
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Resize Shapes".to_string(),
        });
        self.push_command(cmd);
    }

    /// Snaps the selected shape to whole millimeters.
    pub fn snap_selected_to_mm(&mut self) {
        let updates = self.canvas.calculate_snapped_shapes();
        if updates.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (id, new_obj) in updates {
            if let Some(old_obj) = self.canvas.get_shape(id) {
                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id,
                    old_state: old_obj.clone(),
                    new_state: new_obj,
                }));
            }
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Snap to Grid".to_string(),
        });
        self.push_command(cmd);
    }

    /// Sets the position and size of the selected shape.
    pub fn set_selected_position_and_size(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.set_selected_position_and_size_with_flags(x, y, w, h, true, true);
    }

    /// Sets the position and size of the selected shape with flags for which properties to update.
    pub fn set_selected_position_and_size_with_flags(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        update_position: bool,
        update_size: bool,
    ) {
        let updates = self.canvas.calculate_position_and_size_updates(
            x,
            y,
            w,
            h,
            update_position,
            update_size,
        );
        if updates.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (id, new_obj) in updates {
            if let Some(old_obj) = self.canvas.get_shape(id) {
                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id,
                    old_state: old_obj.clone(),
                    new_state: new_obj,
                }));
            }
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Resize/Move Shape".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their left edges.
    pub fn align_selected_horizontal_left(&mut self) {
        let deltas = self
            .canvas
            .calculate_alignment_deltas(crate::canvas::Alignment::Left);
        if deltas.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes {
                ids: vec![id],
                dx,
                dy,
            }));
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Left".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their horizontal centers.
    pub fn align_selected_horizontal_center(&mut self) {
        let deltas = self
            .canvas
            .calculate_alignment_deltas(crate::canvas::Alignment::CenterHorizontal);
        if deltas.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes {
                ids: vec![id],
                dx,
                dy,
            }));
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Horizontal Center".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their right edges.
    pub fn align_selected_horizontal_right(&mut self) {
        let deltas = self
            .canvas
            .calculate_alignment_deltas(crate::canvas::Alignment::Right);
        if deltas.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes {
                ids: vec![id],
                dx,
                dy,
            }));
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Right".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their top edges.
    pub fn align_selected_vertical_top(&mut self) {
        let deltas = self
            .canvas
            .calculate_alignment_deltas(crate::canvas::Alignment::Top);
        if deltas.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes {
                ids: vec![id],
                dx,
                dy,
            }));
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Top".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their vertical centers.
    pub fn align_selected_vertical_center(&mut self) {
        let deltas = self
            .canvas
            .calculate_alignment_deltas(crate::canvas::Alignment::CenterVertical);
        if deltas.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes {
                ids: vec![id],
                dx,
                dy,
            }));
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Vertical Center".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their bottom edges.
    pub fn align_selected_vertical_bottom(&mut self) {
        let deltas = self
            .canvas
            .calculate_alignment_deltas(crate::canvas::Alignment::Bottom);
        if deltas.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes {
                ids: vec![id],
                dx,
                dy,
            }));
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Bottom".to_string(),
        });
        self.push_command(cmd);
    }

    /// Mirrors selected shapes across the global X axis (horizontal flip).
    pub fn mirror_selected_x(&mut self) {
        self.mirror_selected(MirrorAxis::X);
    }

    /// Mirrors selected shapes across the global Y axis (vertical flip).
    pub fn mirror_selected_y(&mut self) {
        self.mirror_selected(MirrorAxis::Y);
    }

    fn mirror_selected(&mut self, axis: MirrorAxis) {
        let mut selected = Vec::new();
        for obj in self.canvas.shapes().filter(|s| s.selected) {
            selected.push(obj.clone());
        }

        if selected.is_empty() {
            return;
        }

        let (center_x, center_y) = match self.canvas.selection_bounds() {
            Some((min_x, min_y, max_x, max_y)) => ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0),
            None => return,
        };

        let (sx, sy, name) = match axis {
            MirrorAxis::X => (-1.0, 1.0, "Mirror X"),
            MirrorAxis::Y => (1.0, -1.0, "Mirror Y"),
        };

        let mut commands = Vec::new();
        for obj in selected {
            let mut new_obj = obj.clone();
            new_obj.shape.scale(sx, sy, Point::new(center_x, center_y));

            commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                id: obj.id,
                old_state: obj,
                new_state: new_obj,
            }));
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: name.to_string(),
        });
        self.push_command(cmd);
    }

    /// Sets the offset for the selected shapes.
    pub fn set_offset_selected(&mut self, distance: f64) {
        let selected_ids: Vec<u64> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();

        if selected_ids.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for id in selected_ids {
            if let Some(obj) = self.canvas.get_shape(id) {
                let mut new_obj = obj.clone();
                new_obj.offset = distance;

                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id,
                    old_state: obj.clone(),
                    new_state: new_obj,
                }));
            }
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Set Offset".to_string(),
        });
        self.push_command(cmd);
    }

    /// Sets the fillet for the selected shapes.
    pub fn set_fillet_selected(&mut self, radius: f64) {
        let selected_ids: Vec<u64> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();

        if selected_ids.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for id in selected_ids {
            if let Some(obj) = self.canvas.get_shape(id) {
                let mut new_obj = obj.clone();
                new_obj.fillet = radius;

                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id,
                    old_state: obj.clone(),
                    new_state: new_obj,
                }));
            }
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Set Fillet".to_string(),
        });
        self.push_command(cmd);
    }

    /// Sets the chamfer for the selected shapes.
    pub fn set_chamfer_selected(&mut self, distance: f64) {
        let selected_ids: Vec<u64> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();

        if selected_ids.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for id in selected_ids {
            if let Some(obj) = self.canvas.get_shape(id) {
                let mut new_obj = obj.clone();
                new_obj.chamfer = distance;

                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id,
                    old_state: obj.clone(),
                    new_state: new_obj,
                }));
            }
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Set Chamfer".to_string(),
        });
        self.push_command(cmd);
    }
}
