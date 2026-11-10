//! Common utilities shared across CAM tool modules

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box, Entry, Label, Orientation, Paned};
use libadwaita::prelude::*;
use libadwaita::ActionRow;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::device_status;
use gcodekit5_core::units;
use gcodekit5_settings::SettingsController;

/// Poll the device connection state every 500 ms for as long as `anchor` exists.
///
/// The timer only holds a weak reference to `anchor` (pass weak references to any other
/// widgets `update` touches, too), so it does not keep the page alive. It is removed when
/// `anchor` is destroyed, and stops by itself if the anchor can no longer be upgraded.
///
/// # Arguments
/// * `anchor` - Widget whose lifetime bounds the timer.
/// * `update` - Called on every tick with the anchor and `true` when a device is connected.
pub fn poll_connection_status<W>(anchor: &W, update: impl Fn(&W, bool) + 'static)
where
    W: IsA<gtk4::Widget>,
{
    let source: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let weak_anchor = anchor.downgrade();
    let timer_source = source.clone();
    let id = glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        let Some(anchor) = weak_anchor.upgrade() else {
            // Returning Break removes the source; forget the id so it is not removed twice.
            timer_source.borrow_mut().take();
            return glib::ControlFlow::Break;
        };
        update(&anchor, device_status::get_status().is_connected);
        glib::ControlFlow::Continue
    });
    *source.borrow_mut() = Some(id);
    anchor.connect_destroy(move |_| {
        if let Some(id) = source.borrow_mut().take() {
            id.remove();
        }
    });
}

/// Show the problems found while loading a preset, if any.
///
/// # Arguments
/// * `warnings` - Messages from the preset loader.
pub fn show_load_warnings(warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }
    super::CamToolsView::show_error_dialog(
        &crate::t!("Preset Loaded With Warnings"),
        &warnings.join("\n"),
    );
}

/// Write `contents` to `path`, showing an error dialog on failure.
///
/// # Arguments
/// * `path` - Destination file.
/// * `contents` - File contents.
pub fn write_preset_file(path: &std::path::Path, contents: &str) {
    if let Err(e) = std::fs::write(path, contents) {
        tracing::error!(path = %path.display(), error = %e, "Failed to save CAM preset");
        super::CamToolsView::show_error_dialog(
            &crate::t!("Save Error"),
            &format!("{}: {}", path.display(), e),
        );
    }
}

/// Parses a length entry for a preset file.
///
/// # Errors
/// A message naming the field when the text is not a length, so a preset is
/// never saved with a silently substituted default.
pub fn preset_length(
    name: &str,
    entry: &Entry,
    system: units::MeasurementSystem,
) -> Result<f32, String> {
    units::parse_length(&entry.text(), system)
        .map_err(|_| format!("{name}: \"{}\" is not a valid length", entry.text()))
}

/// Serialises `value` and writes it to `path`, showing an error dialog on
/// failure.
pub fn save_preset_json<T: serde::Serialize>(path: &std::path::Path, value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => write_preset_file(path, &json),
        Err(e) => super::CamToolsView::show_error_dialog(&crate::t!("Save Error"), &e.to_string()),
    }
}

/// Reads a preset JSON file, showing an error dialog when it cannot be read
/// or parsed.
pub fn load_preset_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    let loaded = std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|content| serde_json::from_str::<T>(&content).map_err(|e| e.to_string()));
    match loaded {
        Ok(value) => Some(value),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "Failed to load CAM preset");
            super::CamToolsView::show_error_dialog(
                &crate::t!("Load Error"),
                &format!("{}: {}", path.display(), e),
            );
            None
        }
    }
}

/// Set the initial fraction (position) for a Paned widget after it's mapped
pub fn set_paned_initial_fraction(paned: &Paned, fraction: f64) {
    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    paned.connect_map(move |paned| {
        if done2.replace(true) {
            return;
        }
        let paned = paned.clone();
        glib::idle_add_local_once(move || {
            let width = paned.width();
            if width > 0 {
                paned.set_position((width as f64 * fraction) as i32);
            }
        });
    });
}

/// Create a dimension entry row with unit label that respects measurement system settings
pub fn create_dimension_row(
    title: &str,
    initial_mm: f64,
    settings: &Rc<SettingsController>,
) -> (ActionRow, Entry, Label) {
    let row = ActionRow::builder().title(title).build();
    let box_ = Box::new(Orientation::Horizontal, 6);

    let system = settings.persistence.borrow().config().ui.measurement_system;
    let initial_text = units::format_length(initial_mm as f32, system);

    let entry = Entry::builder()
        .text(&initial_text)
        .valign(Align::Center)
        .width_chars(8)
        .build();

    let label = Label::new(Some(units::get_unit_label(system)));
    label.add_css_class("dim-label");

    box_.append(&entry);
    box_.append(&label);
    row.add_suffix(&box_);

    (row, entry, label)
}

/// Keep dimension entries created by [`create_dimension_row`] in sync with the
/// measurement system: when the setting changes, each entry's value is converted
/// from the previous system to the new one and its unit label is updated.
///
/// # Arguments
/// * `settings` - Settings controller whose changes are observed.
/// * `rows` - `(entry, unit label)` pairs to keep in sync.
pub fn track_measurement_system(settings: &Rc<SettingsController>, rows: Vec<(Entry, Label)>) {
    let last_system = Rc::new(Cell::new(
        settings.persistence.borrow().config().ui.measurement_system,
    ));
    settings.on_setting_changed(move |key, value| {
        if key != "measurement_system" {
            return;
        }
        // Listeners fire before the change is saved to persistence, so the new
        // system must be taken from `value`.
        let Ok(new_system) = value.parse::<units::MeasurementSystem>() else {
            return;
        };
        let old_system = last_system.get();
        if new_system == old_system {
            return;
        }
        let unit_label = units::get_unit_label(new_system);
        for (entry, label) in &rows {
            if let Ok(val_mm) = units::parse_length(&entry.text(), old_system) {
                entry.set_text(&units::format_length(val_mm, new_system));
            }
            label.set_text(unit_label);
        }
        last_system.set(new_system);
    });
}
