//! Property panel event handlers organized by category.
//!
//! Text-entry properties are applied on *commit* (Enter or focus-out) rather than on
//! every keystroke, so each edit produces exactly one undo step. The shared
//! [`connect_entry_commit`] helper implements that behaviour.

pub mod cam;
pub mod dimensions;
pub mod effects;
pub mod gear_sprocket;
pub mod geometry;
pub mod image;
pub mod laser_override;
pub mod text;

use gcodekit5_core::Shared;
use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::commands::{ChangeProperty, CompositeCommand, DesignerCommand};
use gcodekit5_designer::designer_state::DesignerState;
use gtk4::prelude::*;
use gtk4::{Entry, EventControllerFocus};
use std::cell::RefCell;
use std::rc::Rc;

/// Snapshot of what the user was editing when an entry gained focus.
#[derive(Default)]
struct CommitBaseline {
    /// Entry text when editing started (or after the last commit).
    text: Option<String>,
    /// Selected shape ids when editing started.
    selection: Option<Vec<u64>>,
}

/// Returns the ids of the selected shapes, or `None` if the state is busy.
fn selected_ids(state: &Shared<DesignerState>) -> Option<Vec<u64>> {
    state.try_borrow().ok().map(|s| {
        s.canvas
            .shapes()
            .filter(|o| o.selected)
            .map(|o| o.id)
            .collect()
    })
}

/// Connects `apply` so that it runs when the user commits an edit of `entry`.
///
/// An edit is committed by pressing Enter (`activate`) or by moving focus away
/// (focus-leave), and `apply` only runs if the text differs from what it was when
/// editing started (or since the last commit). This gives one model change - and one
/// undo step - per edit instead of one per keystroke.
///
/// While `apply` runs, `has_focus` is cleared so the redraw callback invoked by
/// `apply` can refresh the panel with the normalised value. If the selection changed
/// while the entry was focused (e.g. the user clicked another shape), the pending
/// edit is discarded rather than being applied to the new selection.
///
/// # Arguments
/// * `entry` - the entry to watch
/// * `state` - designer state, used to detect selection changes
/// * `updating` - panel flag set during programmatic updates; commits are ignored then
/// * `has_focus` - panel flag that suppresses refreshes while the user types
/// * `apply` - parses the entry and applies the value to the model
pub(crate) fn connect_entry_commit<F>(
    entry: &Entry,
    state: Shared<DesignerState>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
    apply: F,
) where
    F: Fn(&Entry) + 'static,
{
    let baseline = Rc::new(RefCell::new(CommitBaseline::default()));
    let apply = Rc::new(apply);

    // Runs `apply` if the entry text changed since the baseline. Returns nothing; the
    // baseline is refreshed by the callers.
    let commit = {
        let baseline = baseline.clone();
        let state = state.clone();
        let updating = updating.clone();
        let has_focus = has_focus.clone();
        let apply = apply.clone();
        Rc::new(move |entry: &Entry, keep_focus: bool| {
            if *updating.borrow() {
                return;
            }
            let text = entry.text().to_string();
            let (changed, selection_unchanged) = {
                let base = baseline.borrow();
                let changed = base.text.as_deref() != Some(text.as_str());
                let selection_unchanged = match (&base.selection, selected_ids(&state)) {
                    (Some(before), Some(now)) => *before == now,
                    _ => true,
                };
                (changed, selection_unchanged)
            };
            if !changed {
                return;
            }
            if !selection_unchanged {
                tracing::debug!("Discarding property edit: selection changed while editing");
                return;
            }

            *has_focus.borrow_mut() = false;
            apply(entry);
            *has_focus.borrow_mut() = keep_focus;
        })
    };

    entry.connect_activate({
        let commit = commit.clone();
        let baseline = baseline.clone();
        let state = state.clone();
        move |entry| {
            commit(entry, true);
            let mut base = baseline.borrow_mut();
            base.text = Some(entry.text().to_string());
            base.selection = selected_ids(&state);
        }
    });

    let focus = EventControllerFocus::new();
    focus.connect_enter({
        let baseline = baseline.clone();
        let state = state.clone();
        move |ctrl| {
            if let Some(entry) = ctrl.widget().and_then(|w| w.downcast::<Entry>().ok()) {
                let mut base = baseline.borrow_mut();
                base.text = Some(entry.text().to_string());
                base.selection = selected_ids(&state);
            }
        }
    });
    focus.connect_leave(move |ctrl| {
        if let Some(entry) = ctrl.widget().and_then(|w| w.downcast::<Entry>().ok()) {
            commit(&entry, false);
            *baseline.borrow_mut() = CommitBaseline::default();
        }
    });
    entry.add_controller(focus);
}

/// Applies `modify` to a copy of every selected shape and pushes all resulting changes
/// as one undoable composite command named `name`.
///
/// `modify` returns `true` when it changed the copy; unchanged shapes are skipped and
/// nothing is pushed if no shape changed. Returns `true` when a command was pushed.
pub(crate) fn change_selected_with_undo<F>(
    designer_state: &mut DesignerState,
    name: &str,
    mut modify: F,
) -> bool
where
    F: FnMut(&mut DrawingObject) -> bool,
{
    let mut commands = Vec::new();
    for obj in designer_state.canvas.shapes().filter(|s| s.selected) {
        let mut new_obj = obj.clone();
        if modify(&mut new_obj) {
            commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                id: obj.id,
                old_state: obj.clone(),
                new_state: new_obj,
            }));
        }
    }

    if commands.is_empty() {
        return false;
    }
    designer_state.push_command(DesignerCommand::CompositeCommand(CompositeCommand {
        commands,
        name: name.to_string(),
    }));
    true
}
