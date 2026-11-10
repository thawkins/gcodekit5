//! The Designer's "Array..." dialog: copies the selection in a line, a grid
//! or around a centre point, using the canvas array operations.

use super::DesignerCanvas;
use crate::t;
use crate::ui::gtk::common::dialog_window::DialogWindow;
use gcodekit5_core::units::{self, MeasurementSystem};
use gtk4::prelude::*;
use gtk4::{Align, Box, DropDown, Entry, Grid, Label, Orientation, ResponseType, StringList};

/// The array the dialog's values describe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArraySpec {
    /// `count` items, each `dx`, `dy` (mm) from the previous one.
    Linear { count: usize, dx: f64, dy: f64 },
    /// `rows` x `cols` items spaced `dx`, `dy` (mm).
    Grid {
        rows: usize,
        cols: usize,
        dx: f64,
        dy: f64,
    },
    /// `count` items around (`cx`, `cy`) (mm) over `angle` degrees.
    Circular {
        count: usize,
        cx: f64,
        cy: f64,
        angle: f64,
    },
}

/// Text of the dialog's fields, before validation.
#[derive(Debug, Clone, Default)]
pub struct ArrayInput {
    /// 0 = linear, 1 = grid, 2 = circular.
    pub kind: u32,
    pub count: String,
    pub rows: String,
    pub cols: String,
    pub dx: String,
    pub dy: String,
    pub cx: String,
    pub cy: String,
    pub angle: String,
}

/// Parses the dialog's fields; lengths are in `system` units.
///
/// # Errors
/// A message naming the field that is not valid.
pub fn parse_array_input(
    input: &ArrayInput,
    system: MeasurementSystem,
) -> Result<ArraySpec, String> {
    let count = |name: &str, text: &str, min: usize| -> Result<usize, String> {
        text.trim()
            .parse::<usize>()
            .ok()
            .filter(|n| *n >= min)
            .ok_or_else(|| format!("{name} must be a whole number of at least {min}"))
    };
    let length = |name: &str, text: &str| -> Result<f64, String> {
        units::parse_length(text, system)
            .map(f64::from)
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("{name}: \"{text}\" is not a valid length"))
    };
    match input.kind {
        0 => Ok(ArraySpec::Linear {
            count: count("Count", &input.count, 2)?,
            dx: length("Spacing X", &input.dx)?,
            dy: length("Spacing Y", &input.dy)?,
        }),
        1 => Ok(ArraySpec::Grid {
            rows: count("Rows", &input.rows, 1)?,
            cols: count("Columns", &input.cols, 1)?,
            dx: length("Spacing X", &input.dx)?,
            dy: length("Spacing Y", &input.dy)?,
        }),
        _ => {
            let angle = input
                .angle
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|a| a.is_finite() && *a != 0.0)
                .ok_or_else(|| "Angle must be a non-zero number of degrees".to_string())?;
            Ok(ArraySpec::Circular {
                count: count("Count", &input.count, 2)?,
                cx: length("Centre X", &input.cx)?,
                cy: length("Centre Y", &input.cy)?,
                angle,
            })
        }
    }
}

/// Most objects one Array operation may create; larger arrays would freeze
/// the UI thread and can exhaust memory.
pub const MAX_ARRAY_OBJECTS: usize = 10_000;

/// Checks that `spec` applied to `selected` objects stays within
/// [`MAX_ARRAY_OBJECTS`].
///
/// # Errors
/// A message giving the limit when the array would be larger.
pub fn check_array_size(spec: &ArraySpec, selected: usize) -> Result<(), String> {
    let copies = match *spec {
        ArraySpec::Linear { count, .. } | ArraySpec::Circular { count, .. } => Some(count),
        ArraySpec::Grid { rows, cols, .. } => rows.checked_mul(cols),
    };
    match copies.and_then(|c| c.checked_mul(selected.max(1))) {
        Some(total) if total <= MAX_ARRAY_OBJECTS => Ok(()),
        _ => Err(format!(
            "The array would create more than {MAX_ARRAY_OBJECTS} objects"
        )),
    }
}

impl DesignerCanvas {
    /// Opens the Array dialog for the current selection.
    pub(super) fn open_array_dialog(&self) {
        let system = self
            .settings_controller
            .as_ref()
            .map(|c| c.persistence.borrow().config().ui.measurement_system)
            .unwrap_or(MeasurementSystem::Metric);
        let unit = units::get_unit_label(system);

        let dialog = DialogWindow::new(&t!("Array"), true);
        dialog.window().set_resizable(false);
        dialog.add_button(&t!("Cancel"), ResponseType::Cancel);
        dialog.add_button(&t!("Create"), ResponseType::Ok);
        dialog.set_default_response(ResponseType::Ok);

        let content = Box::new(Orientation::Vertical, 10);
        for m in [12, 12] {
            content.set_margin_top(m);
            content.set_margin_bottom(m);
        }
        content.set_margin_start(12);
        content.set_margin_end(12);

        let kind = DropDown::new(
            Some(StringList::new(&[
                &t!("Linear"),
                &t!("Grid"),
                &t!("Circular"),
            ])),
            None::<gtk4::Expression>,
        );
        content.append(&kind);

        let grid = Grid::builder().row_spacing(8).column_spacing(8).build();
        let mut row = 0;
        let mut field =
            |label: String, default: &str, suffix: &str| -> (Entry, Vec<gtk4::Widget>) {
                let l = Label::new(Some(&label));
                l.set_halign(Align::Start);
                let e = Entry::new();
                e.set_text(default);
                e.set_activates_default(true);
                let s = Label::new(Some(suffix));
                s.set_halign(Align::Start);
                grid.attach(&l, 0, row, 1, 1);
                grid.attach(&e, 1, row, 1, 1);
                grid.attach(&s, 2, row, 1, 1);
                row += 1;
                let widgets = vec![l.upcast(), e.clone().upcast(), s.upcast()];
                (e, widgets)
            };
        let (count, count_w) = field(t!("Count"), "3", "");
        let (rows, rows_w) = field(t!("Rows"), "2", "");
        let (cols, cols_w) = field(t!("Columns"), "2", "");
        let (dx, dx_w) = field(t!("Spacing X"), "10", unit);
        let (dy, dy_w) = field(t!("Spacing Y"), "0", unit);
        let (cx, cx_w) = field(t!("Centre X"), "0", unit);
        let (cy, cy_w) = field(t!("Centre Y"), "0", unit);
        let (angle, angle_w) = field(t!("Angle"), "360", "deg");
        content.append(&grid);

        // Show only the fields the chosen array uses.
        let show = move |k: u32| {
            let on = |ws: &Vec<gtk4::Widget>, v: bool| ws.iter().for_each(|w| w.set_visible(v));
            on(&count_w, k != 1);
            on(&rows_w, k == 1);
            on(&cols_w, k == 1);
            on(&dx_w, k != 2);
            on(&dy_w, k != 2);
            on(&cx_w, k == 2);
            on(&cy_w, k == 2);
            on(&angle_w, k == 2);
        };
        show(0);
        kind.connect_selected_notify(move |d| show(d.selected()));

        let error = Label::new(None);
        error.add_css_class("error");
        error.set_halign(Align::Start);
        content.append(&error);
        dialog.content_area().append(&content);

        let canvas = self.clone();
        dialog.connect_response(move |d, resp| {
            if resp != ResponseType::Ok {
                d.close();
                return;
            }
            let input = ArrayInput {
                kind: kind.selected(),
                count: count.text().to_string(),
                rows: rows.text().to_string(),
                cols: cols.text().to_string(),
                dx: dx.text().to_string(),
                dy: dy.text().to_string(),
                cx: cx.text().to_string(),
                cy: cy.text().to_string(),
                angle: angle.text().to_string(),
            };
            let selected = canvas.state.borrow().selected_count();
            let spec = match parse_array_input(&input, system)
                .and_then(|spec| check_array_size(&spec, selected).map(|()| spec))
            {
                Ok(spec) => spec,
                Err(message) => {
                    error.set_text(&message);
                    return;
                }
            };
            match spec {
                ArraySpec::Linear { count, dx, dy } => canvas.create_linear_array(count, dx, dy),
                ArraySpec::Grid { rows, cols, dx, dy } => {
                    canvas.create_grid_array(rows, cols, dx, dy)
                }
                ArraySpec::Circular {
                    count,
                    cx,
                    cy,
                    angle,
                } => canvas.create_circular_array(count, cx, cy, angle),
            }
            d.close();
        });

        if let Some(root) = self.widget.root()
            && let Ok(win) = root.downcast::<gtk4::Window>()
        {
            dialog.set_transient_for(Some(&win));
        }
        dialog.present();
    }
}
