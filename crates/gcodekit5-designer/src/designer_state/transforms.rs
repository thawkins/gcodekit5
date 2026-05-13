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
