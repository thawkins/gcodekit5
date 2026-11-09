//! Common utilities shared across CAM tool modules

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box, Entry, Label, Orientation, Paned};
use libadwaita::prelude::*;
use libadwaita::ActionRow;
use std::cell::Cell;
use std::rc::Rc;

use gcodekit5_core::units;
use gcodekit5_settings::SettingsController;

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
