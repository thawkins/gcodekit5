//! Shape operations (add, delete, group, copy, paste, booleans) for designer state.

use super::DesignerState;
use crate::canvas::DrawingObject;
use crate::commands::*;
use crate::model::{DesignPath as PathShape, DesignText as TextShape, DesignerShape, Shape};
use crate::ops::{perform_boolean, BooleanOp};
use crate::{Circle, DrawingMode, Ellipse, Line, Point, Rectangle};

impl DesignerState {
    /// Check if grouping is possible (at least 2 items selected, and at least one is not already in a group).
    pub fn can_group(&self) -> bool {
        let selected: Vec<_> = self.canvas.shapes().filter(|s| s.selected).collect();
        if selected.len() < 2 {
            return false;
        }
        selected.iter().any(|s| s.group_id.is_none())
    }

    /// Check if boolean operations are possible (at least 2 items selected).
    pub fn can_perform_boolean_op(&self) -> bool {
        let selected_count = self.canvas.shapes().filter(|s| s.selected).count();
        selected_count >= 2
    }

    /// Check if ungrouping is possible (any selected item has a group_id).
    pub fn can_ungroup(&self) -> bool {
        self.canvas
            .shapes()
            .filter(|s| s.selected)
            .any(|s| s.group_id.is_some())
    }

    /// Deletes the selected shape(s).
    pub fn delete_selected(&mut self) {
        let ids: Vec<u64> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();
        if ids.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for id in ids {
            commands.push(DesignerCommand::RemoveShape(RemoveShape {
                id,
                object: None,
            }));
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Delete Shapes".to_string(),
        });
        self.push_command(cmd);
    }

    /// Copies selected shapes to clipboard.
    pub fn copy_selected(&mut self) {
        self.clipboard = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();
    }

    /// Pastes shapes from clipboard at specified location.
    pub fn paste_at_location(&mut self, x: f64, y: f64) {
        if self.clipboard.is_empty() {
            return;
        }

        // Calculate bounding box of clipboard items
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for obj in &self.clipboard {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
        }

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        // Calculate offset to move center to (x, y)
        let dx = x - center_x;
        let dy = y - center_y;

        self.canvas.deselect_all();

        let mut new_ids = Vec::new();
        let mut pasted_objects = Vec::new();
        let mut group_map = std::collections::HashMap::new();

        for obj in &self.clipboard {
            let mut new_obj = obj.clone();
            let id = self.canvas.generate_id();
            new_obj.id = id;

            // Handle group ID mapping
            if let Some(gid) = obj.group_id {
                let new_gid = *group_map
                    .entry(gid)
                    .or_insert_with(|| self.canvas.generate_id());
                new_obj.group_id = Some(new_gid);
            }

            // Apply offset
            new_obj.shape.translate(dx, dy);
            new_obj.selected = true;

            new_ids.push(id);
            pasted_objects.push(Some(new_obj));
        }

        // Update selected_id to the last pasted object
        if let Some(last_id) = new_ids.last() {
            self.canvas.set_selected_id(Some(*last_id));
        }

        let cmd = DesignerCommand::PasteShapes(PasteShapes {
            ids: new_ids,
            objects: pasted_objects,
        });
        self.push_command(cmd);
    }

    /// Groups the selected shapes.
    pub fn group_selected(&mut self) {
        let ids: Vec<u64> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();
        if ids.len() < 2 {
            return;
        }

        let group_id = self.canvas.generate_id();
        let cmd = DesignerCommand::GroupShapes(GroupShapes { ids, group_id });
        self.push_command(cmd);
    }

    /// Ungroups the selected shapes.
    pub fn ungroup_selected(&mut self) {
        let mut group_map: std::collections::HashMap<u64, Vec<u64>> =
            std::collections::HashMap::new();
        for obj in self.canvas.shapes() {
            if obj.selected {
                if let Some(gid) = obj.group_id {
                    group_map.entry(gid).or_default().push(obj.id);
                }
            }
        }

        if group_map.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (gid, ids) in group_map {
            commands.push(DesignerCommand::UngroupShapes(UngroupShapes {
                ids,
                group_id: gid,
            }));
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Ungroup Shapes".to_string(),
        });
        self.push_command(cmd);
    }

    /// Clears all shapes from the canvas.
    pub fn clear_canvas(&mut self) {
        if self.canvas.shape_count() > 0 {
            let ids: Vec<u64> = self.canvas.shapes().map(|s| s.id).collect();
            let mut commands = Vec::new();
            for id in ids {
                commands.push(DesignerCommand::RemoveShape(RemoveShape {
                    id,
                    object: None,
                }));
            }
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Clear Canvas".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Adds a shape to the canvas with undo/redo support.
    pub fn add_shape_with_undo(&mut self, shape: Shape) -> u64 {
        let id = self.canvas.generate_id();
        let obj = DrawingObject::new(id, shape);
        let cmd = DesignerCommand::AddShape(AddShape {
            id,
            object: Some(obj),
        });
        self.push_command(cmd);
        id
    }

    /// Adds a shape to the canvas at the specified position based on current mode.
    pub fn add_shape_at(&mut self, x: f64, y: f64, multi_select: bool) {
        match self.canvas.mode() {
            DrawingMode::Select => {
                let tolerance = 3.0 / self.canvas.zoom();
                self.canvas
                    .select_at(&Point::new(x, y), tolerance, multi_select);
            }
            DrawingMode::Rectangle => {
                let id = self.canvas.generate_id();
                let rect = Rectangle::new(x, y, 60.0, 40.0);
                let obj = DrawingObject::new(id, Shape::Rectangle(rect));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Circle => {
                let id = self.canvas.generate_id();
                let circle = Circle::new(Point::new(x, y), 25.0);
                let obj = DrawingObject::new(id, Shape::Circle(circle));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Line => {
                let id = self.canvas.generate_id();
                let line = Line::new(Point::new(x, y), Point::new(x + 50.0, y));
                let obj = DrawingObject::new(id, Shape::Line(line));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Ellipse => {
                let id = self.canvas.generate_id();
                let ellipse = Ellipse::new(Point::new(x, y), 40.0, 25.0);
                let obj = DrawingObject::new(id, Shape::Ellipse(ellipse));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Polyline => {
                let id = self.canvas.generate_id();
                let center = Point::new(x, y);
                let radius = 30.0;
                let sides = 6;
                let mut vertices = Vec::with_capacity(sides);
                for i in 0..sides {
                    let angle = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
                    let vx = center.x + radius * angle.cos();
                    let vy = center.y + radius * angle.sin();
                    vertices.push(Point::new(vx, vy));
                }
                let path_shape = PathShape::from_points(&vertices, false); // antes true y cierra
                let obj = DrawingObject::new(id, Shape::Path(path_shape));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Text => {
                let id = self.canvas.generate_id();
                let text = TextShape::new("Text".to_string(), x, y, 20.0);
                let obj = DrawingObject::new(id, Shape::Text(text));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Triangle => {
                let id = self.canvas.generate_id();
                let triangle = crate::model::DesignTriangle::new(Point::new(x, y), 50.0, 50.0);
                let obj = DrawingObject::new(id, Shape::Triangle(triangle));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Polygon => {
                let id = self.canvas.generate_id();
                let polygon = crate::model::DesignPolygon::new(Point::new(x, y), 30.0, 6);
                let obj = DrawingObject::new(id, Shape::Polygon(polygon));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Gear => {
                let id = self.canvas.generate_id();
                let gear = crate::model::DesignGear::new(Point::new(x, y), 2.0, 20);
                let obj = DrawingObject::new(id, Shape::Gear(gear));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Sprocket => {
                let id = self.canvas.generate_id();
                let sprocket = crate::model::DesignSprocket::new(Point::new(x, y), 12.7, 15);
                let obj = DrawingObject::new(id, Shape::Sprocket(sprocket));
                let cmd = DesignerCommand::AddShape(AddShape {
                    id,
                    object: Some(obj),
                });
                self.push_command(cmd);
            }
            DrawingMode::Pan => {}
        }
    }

    /// Adds a test rectangle to the canvas.
    pub fn add_test_rectangle(&mut self) {
        let id = self.canvas.generate_id();
        let rect = Rectangle::new(10.0, 10.0, 50.0, 40.0);
        let obj = DrawingObject::new(id, Shape::Rectangle(rect));
        let cmd = DesignerCommand::AddShape(AddShape {
            id,
            object: Some(obj),
        });
        self.push_command(cmd);
    }

    /// Adds a test circle to the canvas.
    pub fn add_test_circle(&mut self) {
        let id = self.canvas.generate_id();
        let circle = Circle::new(Point::new(75.0, 75.0), 20.0);
        let obj = DrawingObject::new(id, Shape::Circle(circle));
        let cmd = DesignerCommand::AddShape(AddShape {
            id,
            object: Some(obj),
        });
        self.push_command(cmd);
    }

    /// Adds a test line to the canvas.
    pub fn add_test_line(&mut self) {
        let id = self.canvas.generate_id();
        let line = Line::new(Point::new(10.0, 10.0), Point::new(100.0, 100.0));
        let obj = DrawingObject::new(id, Shape::Line(line));
        let cmd = DesignerCommand::AddShape(AddShape {
            id,
            object: Some(obj),
        });
        self.push_command(cmd);
    }

    /// Performs a boolean union on selected shapes.
    pub fn perform_boolean_union(&mut self) {
        self.perform_boolean_op(BooleanOp::Union, "Union");
    }

    /// Performs a boolean difference on selected shapes.
    pub fn perform_boolean_difference(&mut self) {
        self.perform_boolean_op(BooleanOp::Difference, "Difference");
    }

    /// Performs a boolean intersection on selected shapes.
    pub fn perform_boolean_intersection(&mut self) {
        self.perform_boolean_op(BooleanOp::Intersection, "Intersection");
    }

    fn perform_boolean_op(&mut self, op: BooleanOp, name: &str) {
        let selected: Vec<_> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();

        if selected.len() < 2 {
            return;
        }

        let mut result_shape = selected[0].shape.clone();

        for item in &selected[1..] {
            result_shape = perform_boolean(
                &result_shape,
                &item.shape,
                match op {
                    BooleanOp::Union => BooleanOp::Union,
                    BooleanOp::Difference => BooleanOp::Difference,
                    BooleanOp::Intersection => BooleanOp::Intersection,
                },
            );
        }

        let new_id = self.canvas.generate_id();
        let mut new_obj = DrawingObject::new(new_id, result_shape);
        new_obj.selected = true;

        let mut commands = Vec::new();
        for obj in selected {
            commands.push(DesignerCommand::RemoveShape(RemoveShape {
                id: obj.id,
                object: Some(obj),
            }));
        }
        commands.push(DesignerCommand::AddShape(AddShape {
            id: new_id,
            object: Some(new_obj),
        }));

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: name.to_string(),
        });
        self.push_command(cmd);
    }
}
