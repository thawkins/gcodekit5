//! Regression tests for the UI leak and silent-failure fixes (issue #79):
//! the Tool Settings dialog is built once, refreshing an entry from the model
//! does not write the rounded value back, a newer status notice is not cleared
//! by an older notice's timer, and the shared CAM preset helpers.
//!
//! The GTK cases need a display. Without one (`gtk4::init()` fails) they
//! return early. GTK must be driven from one thread, so they run in one test.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gcodekit5_core::units::MeasurementSystem;
use gcodekit5_ui::ui::gtk::cam_tools::common::{load_preset_json, preset_length, save_preset_json};
use gcodekit5_ui::ui::gtk::designer_toolbox::{build_once, fill_entry_from_model};
use gcodekit5_ui::ui::gtk::status_bar::StatusBar;
use gtk4::prelude::*;

#[test]
fn test_dialog_is_built_once_and_reused() {
    // E1-04: the dialog was rebuilt (and leaked) on every click.
    let cell: RefCell<Option<u32>> = RefCell::new(None);
    let builds = Cell::new(0);
    let build = || {
        builds.set(builds.get() + 1);
        Some(7)
    };
    assert_eq!(build_once(&cell, build), Some((7, true)));
    assert_eq!(build_once(&cell, build), Some((7, false)));
    assert_eq!(builds.get(), 1);

    // Nothing is stored when building fails (no window yet), so it retries.
    let empty: RefCell<Option<u32>> = RefCell::new(None);
    assert_eq!(build_once(&empty, || None), None);
    assert_eq!(build_once(&empty, || Some(1)), Some((1, true)));
}

/// Runs main-loop work until `deadline`.
fn run_until(deadline: Instant) {
    let context = glib::MainContext::default();
    while Instant::now() < deadline {
        while context.iteration(false) {}
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn test_ui_leak_fixes_with_gtk() {
    if gtk4::init().is_err() {
        eprintln!("skipping test_ui_leak_fixes_with_gtk: no display available");
        return;
    }

    // E1-05: filling the entry from the model must not run the write-back.
    let entry = gtk4::Entry::new();
    let updating = Rc::new(Cell::new(false));
    let written: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let updating = updating.clone();
        let written = written.clone();
        entry.connect_changed(move |e| {
            if updating.get() {
                return;
            }
            written.borrow_mut().push(e.text().to_string());
        });
    }
    fill_entry_from_model(&entry, &updating, "0.039");
    assert!(
        written.borrow().is_empty(),
        "model refresh was written back"
    );
    entry.set_text("0.5");
    // GTK reports a replaced text as a clear and then the new value.
    assert_eq!(written.borrow().last().map(String::as_str), Some("0.5"));

    // E2-08: a bad length names the field instead of using a default.
    let length = gtk4::Entry::new();
    length.set_text("abc");
    let err = preset_length("Hole depth", &length, MeasurementSystem::Metric).expect_err("bad");
    assert!(err.contains("Hole depth"), "{err}");
    length.set_text("12.5");
    assert_eq!(
        preset_length("Hole depth", &length, MeasurementSystem::Metric),
        Ok(12.5)
    );
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/temp/ui_leaks");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("preset.json");
    let preset: BTreeMap<String, f64> = [("depth".to_string(), 12.5)].into();
    save_preset_json(&path, &preset);
    assert_eq!(
        load_preset_json::<BTreeMap<String, f64>>(&path),
        Some(preset)
    );

    // E2-09: an older notice's timer must not clear a newer notice.
    let status = StatusBar::new();
    let start = Instant::now();
    status.set_state("First notice");
    run_until(start + Duration::from_millis(2500));
    status.set_state("Second notice");
    // The first notice's 4 s timer would have fired by now.
    run_until(start + Duration::from_millis(4500));
    assert_eq!(status.alert_label.text(), "Second notice");
}
