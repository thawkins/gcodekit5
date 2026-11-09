//! # Designer Commands (Undo/Redo)
//!
//! Implements the Command pattern for undoable canvas operations.
//! Each command encapsulates a discrete change (add shape, move shape,
//! delete shape, etc.) and can be executed and undone.

use crate::canvas::{Canvas, DrawingObject};
use crate::model::DesignerShape;
use crate::model::Shape;
use crate::spatial_index::Bounds;

/// A command that can be executed and undone on the canvas.
pub trait Command {
    /// Executes the command.
    fn execute(&mut self, canvas: &mut Canvas);

    /// Undoes the command.
    fn undo(&mut self, canvas: &mut Canvas);

    /// Returns the name of the command for display.
    fn name(&self) -> &str;

    /// Returns the size of the command in bytes (for memory management).
    fn size(&self) -> usize {
        std::mem::size_of_val(self)
    }
}

/// Command to add a shape to the canvas.
pub struct AddShapeCommand {
    id: u64,
    shape: Option<DrawingObject>, // Stored here when undone
}

impl AddShapeCommand {
    pub fn new(id: u64) -> Self {
        Self { id, shape: None }
    }
}

impl Command for AddShapeCommand {
    fn execute(&mut self, canvas: &mut Canvas) {
        if let Some(shape) = self.shape.take() {
            canvas.restore_shape(shape);
        }
        // If shape is None, it means we are executing for the first time?
        // But usually commands are created AFTER the action or BEFORE?
        // If we use the pattern where Command performs the action, then we need the shape data initially.
        // If we use the pattern where Command captures the state change, then:
        // 1. Action happens.
        // 2. Command is created capturing the change.
        // 3. Command is pushed to stack.
        // Undo: Revert change.
        // Redo: Apply change.

        // Let's assume the Command encapsulates the action.
        // But `AddShape` usually happens via UI interaction (drag, click).
        // So the shape is created by `Canvas` methods.
        // We might need `Canvas` to return the `Command` or `DesignerState` to construct it.
    }

    fn undo(&mut self, canvas: &mut Canvas) {
        if let Some(shape) = canvas.remove_shape_return(self.id) {
            self.shape = Some(shape);
        }
    }

    fn name(&self) -> &str {
        "Add Shape"
    }
}

// We need a way to wrap these commands.
// Since we have different structs, we can use an enum or Box<dyn Command>.
// Enum is usually better for performance and serialization.

#[derive(Debug, Clone)]
// Variant size disparity is acceptable — commands are short-lived and not stored in bulk.
#[allow(clippy::large_enum_variant)]
pub enum DesignerCommand {
    AddShape(AddShape),
    RemoveShape(RemoveShape),
    MoveShapes(MoveShapes),
    ResizeShape(ResizeShape),
    ChangeProperty(ChangeProperty),
    GroupShapes(GroupShapes),
    UngroupShapes(UngroupShapes),
    PasteShapes(PasteShapes),
    CompositeCommand(CompositeCommand),
    /// A shape removal that remembers the draw-order index the shape occupied.
    ///
    /// `RemoveShape` commands are converted to this variant the first time they are
    /// applied, so undo can put the shape back at its original z-order position.
    RemoveShapeAt(RemoveShapeAt),
    /// A grouping that remembers each shape's previous group membership.
    ///
    /// `GroupShapes` commands are converted to this variant the first time they are
    /// applied, so undo restores earlier groups instead of clearing them.
    GroupShapesTracked(GroupShapesTracked),
}

/// Removal of a shape together with the draw-order index it was removed from.
#[derive(Debug, Clone)]
pub struct RemoveShapeAt {
    /// ID of the removed shape.
    pub id: u64,
    /// Draw-order index (0 = bottom) the shape had before removal.
    pub index: Option<usize>,
    /// The removed object while the command is applied; `None` while undone.
    pub object: Option<DrawingObject>,
}

/// Grouping of shapes that records the group each shape belonged to beforehand.
#[derive(Debug, Clone)]
pub struct GroupShapesTracked {
    /// IDs of the grouped shapes.
    pub ids: Vec<u64>,
    /// The new group ID assigned to all shapes.
    pub group_id: u64,
    /// Previous group ID of each shape in `ids` (same order).
    pub previous_group_ids: Vec<Option<u64>>,
}

#[derive(Debug, Clone)]
pub struct CompositeCommand {
    pub commands: Vec<DesignerCommand>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct AddShape {
    pub id: u64,
    pub object: Option<DrawingObject>, // None when on canvas, Some when undone
}

#[derive(Debug, Clone)]
pub struct RemoveShape {
    pub id: u64,
    pub object: Option<DrawingObject>, // Some when executed (removed), None when undone (on canvas)
}

#[derive(Debug, Clone)]
pub struct MoveShapes {
    pub ids: Vec<u64>,
    pub dx: f64,
    pub dy: f64,
}

#[derive(Debug, Clone)]
pub struct ResizeShape {
    pub id: u64,
    pub handle: usize,
    pub dx: f64,
    pub dy: f64,
    // We might need old state if resize is complex/lossy?
    // But resize is usually reversible by -dx, -dy IF it's a translation of a handle.
    // However, scaling might lose precision or be non-linear.
    // Storing the old shape might be safer.
    pub old_shape: Option<Shape>,
    pub new_shape: Option<Shape>,
}

#[derive(Debug, Clone)]
pub struct ChangeProperty {
    pub id: u64,
    // We can store the old and new DrawingObject state, or specific fields.
    // Storing the whole object is safer and easier.
    pub old_state: DrawingObject,
    pub new_state: DrawingObject,
}

#[derive(Debug, Clone)]
pub struct GroupShapes {
    pub ids: Vec<u64>,
    pub group_id: u64,
}

#[derive(Debug, Clone)]
pub struct UngroupShapes {
    pub ids: Vec<u64>,
    pub group_id: u64,
}

#[derive(Debug, Clone)]
pub struct PasteShapes {
    pub ids: Vec<u64>, // IDs of pasted shapes
    // When undone, we remove these shapes.
    // When redone, we need to restore them.
    pub objects: Vec<Option<DrawingObject>>,
}

/// Removes `id` from the canvas, returning its previous draw-order index and the object.
fn remove_with_index(canvas: &mut Canvas, id: u64) -> (Option<usize>, Option<DrawingObject>) {
    let index = canvas.shape_store.get_depth(id);
    let object = canvas.remove_shape_return(id);
    (index, object)
}

/// Restores `obj` to the canvas and moves it back to draw-order `index` when known.
fn restore_at_index(canvas: &mut Canvas, obj: DrawingObject, index: Option<usize>) {
    let id = obj.id;
    canvas.restore_shape(obj);
    if let Some(index) = index {
        canvas.shape_store.move_to_position(id, index);
    }
}

impl DesignerCommand {
    /// Applies (executes or redoes) the command on `canvas`.
    ///
    /// Some variants are normalised into richer variants on first application
    /// (`RemoveShape` -> `RemoveShapeAt`, `GroupShapes` -> `GroupShapesTracked`) so
    /// that undo can restore the exact previous state.
    pub fn apply(&mut self, canvas: &mut Canvas) {
        match self {
            DesignerCommand::AddShape(cmd) => {
                if let Some(obj) = cmd.object.take() {
                    canvas.restore_shape(obj);
                }
            }
            DesignerCommand::RemoveShape(cmd) => {
                let id = cmd.id;
                let (index, object) = remove_with_index(canvas, id);
                *self = DesignerCommand::RemoveShapeAt(RemoveShapeAt { id, index, object });
            }
            DesignerCommand::RemoveShapeAt(cmd) => {
                let (index, object) = remove_with_index(canvas, cmd.id);
                cmd.index = index;
                cmd.object = object;
            }
            DesignerCommand::MoveShapes(cmd) => {
                for id in &cmd.ids {
                    if let Some(obj) = canvas.get_shape_mut(*id) {
                        let (x1, y1, x2, y2) = obj.get_total_bounds();
                        let old_bounds = Bounds::new(x1, y1, x2, y2);

                        obj.shape.translate(cmd.dx, cmd.dy);

                        let (nx1, ny1, nx2, ny2) = obj.get_total_bounds();
                        let new_bounds = Bounds::new(nx1, ny1, nx2, ny2);

                        canvas.remove_from_index(*id, &old_bounds);
                        canvas.insert_into_index(*id, &new_bounds);
                    }
                }
            }
            DesignerCommand::ResizeShape(cmd) => {
                // If we stored shapes, swap them
                if let Some(new_shape) = &cmd.new_shape {
                    if let Some(obj) = canvas.get_shape_mut(cmd.id) {
                        let (x1, y1, x2, y2) = obj.get_total_bounds();
                        let old_bounds = Bounds::new(x1, y1, x2, y2);

                        obj.shape = new_shape.clone();

                        let (nx1, ny1, nx2, ny2) = obj.get_total_bounds();
                        let new_bounds = Bounds::new(nx1, ny1, nx2, ny2);

                        canvas.remove_from_index(cmd.id, &old_bounds);
                        canvas.insert_into_index(cmd.id, &new_bounds);
                    }
                }
            }
            DesignerCommand::ChangeProperty(cmd) => {
                if let Some(obj) = canvas.get_shape_mut(cmd.id) {
                    let (x1, y1, x2, y2) = obj.get_total_bounds();
                    let old_bounds = Bounds::new(x1, y1, x2, y2);

                    *obj = cmd.new_state.clone();

                    let (nx1, ny1, nx2, ny2) = obj.get_total_bounds();
                    let new_bounds = Bounds::new(nx1, ny1, nx2, ny2);

                    canvas.remove_from_index(cmd.id, &old_bounds);
                    canvas.insert_into_index(cmd.id, &new_bounds);
                }
            }
            DesignerCommand::GroupShapes(cmd) => {
                let ids = std::mem::take(&mut cmd.ids);
                let group_id = cmd.group_id;
                let previous_group_ids = assign_group(canvas, &ids, group_id);
                *self = DesignerCommand::GroupShapesTracked(GroupShapesTracked {
                    ids,
                    group_id,
                    previous_group_ids,
                });
            }
            DesignerCommand::GroupShapesTracked(cmd) => {
                cmd.previous_group_ids = assign_group(canvas, &cmd.ids, cmd.group_id);
            }
            DesignerCommand::UngroupShapes(cmd) => {
                for id in &cmd.ids {
                    if let Some(obj) = canvas.get_shape_mut(*id) {
                        obj.group_id = None;
                    }
                }
            }
            DesignerCommand::PasteShapes(cmd) => {
                for (i, _) in cmd.ids.iter().enumerate() {
                    if let Some(obj) = cmd.objects[i].take() {
                        canvas.restore_shape(obj);
                    }
                }
            }
            DesignerCommand::CompositeCommand(cmd) => {
                for sub_cmd in &mut cmd.commands {
                    sub_cmd.apply(canvas);
                }
            }
        }
    }

    /// Reverts the command on `canvas`.
    pub fn undo(&mut self, canvas: &mut Canvas) {
        match self {
            DesignerCommand::AddShape(cmd) => {
                if let Some(obj) = canvas.remove_shape_return(cmd.id) {
                    cmd.object = Some(obj);
                }
            }
            DesignerCommand::RemoveShape(cmd) => {
                if let Some(obj) = cmd.object.take() {
                    canvas.restore_shape(obj);
                }
            }
            DesignerCommand::RemoveShapeAt(cmd) => {
                if let Some(obj) = cmd.object.take() {
                    restore_at_index(canvas, obj, cmd.index);
                }
            }
            DesignerCommand::MoveShapes(cmd) => {
                for id in &cmd.ids {
                    if let Some(obj) = canvas.get_shape_mut(*id) {
                        let (x1, y1, x2, y2) = obj.get_total_bounds();
                        let old_bounds = Bounds::new(x1, y1, x2, y2);

                        obj.shape.translate(-cmd.dx, -cmd.dy);

                        let (nx1, ny1, nx2, ny2) = obj.get_total_bounds();
                        let new_bounds = Bounds::new(nx1, ny1, nx2, ny2);

                        canvas.remove_from_index(*id, &old_bounds);
                        canvas.insert_into_index(*id, &new_bounds);
                    }
                }
            }
            DesignerCommand::ResizeShape(cmd) => {
                if let Some(old_shape) = &cmd.old_shape {
                    if let Some(obj) = canvas.get_shape_mut(cmd.id) {
                        let (x1, y1, x2, y2) = obj.get_total_bounds();
                        let old_bounds = Bounds::new(x1, y1, x2, y2);

                        obj.shape = old_shape.clone();

                        let (nx1, ny1, nx2, ny2) = obj.get_total_bounds();
                        let new_bounds = Bounds::new(nx1, ny1, nx2, ny2);

                        canvas.remove_from_index(cmd.id, &old_bounds);
                        canvas.insert_into_index(cmd.id, &new_bounds);
                    }
                }
            }
            DesignerCommand::ChangeProperty(cmd) => {
                if let Some(obj) = canvas.get_shape_mut(cmd.id) {
                    let (x1, y1, x2, y2) = obj.get_total_bounds();
                    let old_bounds = Bounds::new(x1, y1, x2, y2);

                    *obj = cmd.old_state.clone();

                    let (nx1, ny1, nx2, ny2) = obj.get_total_bounds();
                    let new_bounds = Bounds::new(nx1, ny1, nx2, ny2);

                    canvas.remove_from_index(cmd.id, &old_bounds);
                    canvas.insert_into_index(cmd.id, &new_bounds);
                }
            }
            DesignerCommand::GroupShapes(cmd) => {
                for id in &cmd.ids {
                    if let Some(obj) = canvas.get_shape_mut(*id) {
                        obj.group_id = None;
                    }
                }
            }
            DesignerCommand::GroupShapesTracked(cmd) => {
                for (id, previous) in cmd.ids.iter().zip(cmd.previous_group_ids.iter()) {
                    if let Some(obj) = canvas.get_shape_mut(*id) {
                        obj.group_id = *previous;
                    }
                }
            }
            DesignerCommand::UngroupShapes(cmd) => {
                for id in &cmd.ids {
                    if let Some(obj) = canvas.get_shape_mut(*id) {
                        obj.group_id = Some(cmd.group_id);
                    }
                }
            }
            DesignerCommand::PasteShapes(cmd) => {
                // Remove pasted shapes
                cmd.objects.clear();
                for id in &cmd.ids {
                    if let Some(obj) = canvas.remove_shape_return(*id) {
                        cmd.objects.push(Some(obj));
                    } else {
                        cmd.objects.push(None); // Should not happen
                    }
                }
            }
            DesignerCommand::CompositeCommand(cmd) => {
                for sub_cmd in cmd.commands.iter_mut().rev() {
                    sub_cmd.undo(canvas);
                }
            }
        }
    }
}

/// Sets `group_id` on every shape in `ids`, returning each shape's previous group id.
fn assign_group(canvas: &mut Canvas, ids: &[u64], group_id: u64) -> Vec<Option<u64>> {
    ids.iter()
        .map(|id| match canvas.get_shape_mut(*id) {
            Some(obj) => obj.group_id.replace(group_id),
            None => None,
        })
        .collect()
}
