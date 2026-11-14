//! Transform operations (move, resize, align, mirror, snap) for designer state.

use super::DesignerState;
use crate::canvas::DrawingObject;
use crate::commands::*;
use crate::model::DesignerShape;
use crate::{Point, Shape};

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

    /// Moves each selected shape by the delta that aligns it, as one undo step.
    fn align_selected(&mut self, alignment: crate::canvas::Alignment, name: &str) {
        let deltas = self.canvas.calculate_alignment_deltas(alignment);
        if deltas.is_empty() {
            return;
        }

        let commands = deltas
            .into_iter()
            .map(|(id, dx, dy)| {
                DesignerCommand::MoveShapes(MoveShapes {
                    ids: vec![id],
                    dx,
                    dy,
                })
            })
            .collect();

        self.push_command(DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: name.to_string(),
        }));
    }

    /// Align selected shapes by their left edges.
    pub fn align_selected_horizontal_left(&mut self) {
        self.align_selected(crate::canvas::Alignment::Left, "Align Left");
    }

    /// Align selected shapes by their horizontal centers.
    pub fn align_selected_horizontal_center(&mut self) {
        self.align_selected(
            crate::canvas::Alignment::CenterHorizontal,
            "Align Horizontal Center",
        );
    }

    /// Align selected shapes by their right edges.
    pub fn align_selected_horizontal_right(&mut self) {
        self.align_selected(crate::canvas::Alignment::Right, "Align Right");
    }

    /// Align selected shapes by their top edges.
    pub fn align_selected_vertical_top(&mut self) {
        self.align_selected(crate::canvas::Alignment::Top, "Align Top");
    }

    /// Align selected shapes by their vertical centers.
    pub fn align_selected_vertical_center(&mut self) {
        self.align_selected(
            crate::canvas::Alignment::CenterVertical,
            "Align Vertical Center",
        );
    }

    /// Align selected shapes by their bottom edges.
    pub fn align_selected_vertical_bottom(&mut self) {
        self.align_selected(crate::canvas::Alignment::Bottom, "Align Bottom");
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
            let old_state = obj.clone();
            let mut obj = obj;
            // Verificar si es un triángulo
            if let Shape::Triangle(triangle) = &mut obj.shape {
                // Aplicar mirror específico para triángulo
                match axis {
                    MirrorAxis::X => {
                        // Horizontal flip (sx = -1, invierte X): swap left/right corners
                        triangle.right_angle_corner = match triangle.right_angle_corner {
                            0 => 1, // Inferior-Izquierda → Inferior-Derecha
                            1 => 0, // Inferior-Derecha → Inferior-Izquierda
                            2 => 3, // Superior-Izquierda → Superior-Derecha
                            3 => 2, // Superior-Derecha → Superior-Izquierda
                            _ => triangle.right_angle_corner,
                        };
                        // Invertir rotación
                        triangle.rotation = -triangle.rotation;
                    }
                    MirrorAxis::Y => {
                        // Vertical flip (sy = -1, invierte Y): swap bottom/top corners
                        triangle.right_angle_corner = match triangle.right_angle_corner {
                            0 => 2, // Inferior-Izquierda → Superior-Izquierda
                            1 => 3, // Inferior-Derecha → Superior-Derecha
                            2 => 0, // Superior-Izquierda → Inferior-Izquierda
                            3 => 1, // Superior-Derecha → Inferior-Derecha
                            _ => triangle.right_angle_corner,
                        };
                        // Invertir rotación
                        triangle.rotation = -triangle.rotation;
                    }
                }

                // Ajustar la posición del centro para que el reflejo sea alrededor del centro de selección
                triangle.center.x = center_x + (triangle.center.x - center_x) * sx;
                triangle.center.y = center_y + (triangle.center.y - center_y) * sy;

                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id: obj.id,
                    old_state,
                    new_state: obj,
                }));
            } else {
                // Para otros tipos de formas, usar el scale genérico
                let mut new_obj = obj;
                new_obj.shape.scale(sx, sy, Point::new(center_x, center_y));

                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id: new_obj.id,
                    old_state,
                    new_state: new_obj,
                }));
            }
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
        self.set_corner_selected("Set Fillet", |o| o.fillet = radius, |o| o.chamfer = 0.0);
    }

    /// Sets the chamfer for the selected shapes.
    pub fn set_chamfer_selected(&mut self, distance: f64) {
        self.set_corner_selected("Set Chamfer", |o| o.chamfer = distance, |o| o.fillet = 0.0);
    }

    /// Applies a corner treatment (`apply`) to every selected shape as one
    /// undo step. A path takes only one treatment, so `clear_other` removes
    /// the other one from paths.
    fn set_corner_selected(
        &mut self,
        name: &str,
        apply: impl Fn(&mut DrawingObject),
        clear_other: impl Fn(&mut DrawingObject),
    ) {
        let commands: Vec<DesignerCommand> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|obj| {
                let mut new_obj = obj.clone();
                apply(&mut new_obj);
                if matches!(new_obj.shape, crate::model::Shape::Path(_)) {
                    clear_other(&mut new_obj);
                }
                DesignerCommand::ChangeProperty(ChangeProperty {
                    id: obj.id,
                    old_state: obj.clone(),
                    new_state: new_obj,
                })
            })
            .collect();

        if commands.is_empty() {
            return;
        }
        self.push_command(DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: name.to_string(),
        }));
    }

    /// Mueve los shapes seleccionados al principio del orden de mecanizado.
    /// Devuelve `true` si se aplicó, `false` si se rechazó porque partiría un grupo.
    pub fn move_selected_to_top(&mut self) -> bool {
        let Some(after) = self.canvas.move_selected_to_top() else {
            return false;
        };
        let before = self.canvas.shape_store.draw_order().clone();
        self.push_command(DesignerCommand::Reorder(Reorder {
            before,
            after,
            name: "Move to Top".to_string(),
        }));
        true
    }

    pub fn move_selected_to_bottom(&mut self) -> bool {
        let Some(after) = self.canvas.move_selected_to_bottom() else {
            return false;
        };
        let before = self.canvas.shape_store.draw_order().clone();
        self.push_command(DesignerCommand::Reorder(Reorder {
            before,
            after,
            name: "Move to Bottom".to_string(),
        }));
        true
    }

    pub fn move_selected_up(&mut self) -> bool {
        let Some(after) = self.canvas.move_selected_up() else {
            return false;
        };
        let before = self.canvas.shape_store.draw_order().clone();
        self.push_command(DesignerCommand::Reorder(Reorder {
            before,
            after,
            name: "Move Up".to_string(),
        }));
        true
    }

    pub fn move_selected_down(&mut self) -> bool {
        let Some(after) = self.canvas.move_selected_down() else {
            return false;
        };
        let before = self.canvas.shape_store.draw_order().clone();
        self.push_command(DesignerCommand::Reorder(Reorder {
            before,
            after,
            name: "Move Down".to_string(),
        }));
        true
    }
}
