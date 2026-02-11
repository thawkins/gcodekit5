//! Designer Canvas - Drawing area and interaction handling for the designer
//!
//! This module contains the DesignerCanvas struct which handles:
//! - Canvas rendering and drawing
//! - Mouse and keyboard interaction
//! - Shape creation and manipulation
//! - Tool operations
//! - Toolpath preview generation

mod editing;
mod input;
mod rendering;
mod toolpath_preview;

use crate::t;
use crate::ui::gtk::designer_layers::LayersPanel;
use crate::ui::gtk::designer_properties::PropertiesPanel;
use crate::ui::gtk::designer_toolbox::{DesignerTool, DesignerToolbox};
use gcodekit5_core::constants as core_constants;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::{DesignPath as PathShape, Point, Shape};
use gcodekit5_designer::toolpath::Toolpath;
use gcodekit5_devicedb::DeviceManager;
use gcodekit5_settings::controller::SettingsController;
use gtk4::gdk::ModifierType;
use gtk4::prelude::*;
use gtk4::{
    CheckButton, Dialog, DrawingArea, DropDown, Entry, EventControllerMotion, GestureClick,
    GestureDrag,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) const MM_PER_PT: f64 = 25.4 / 72.0;

pub(crate) fn mm_to_pt(mm: f64) -> f64 {
    mm / MM_PER_PT
}

pub(crate) fn pt_to_mm(pt: f64) -> f64 {
    pt * MM_PER_PT
}

pub(crate) fn format_font_points(mm: f64) -> String {
    format!("{:.2}", mm_to_pt(mm))
}

pub(crate) fn parse_font_points_mm(s: &str) -> Option<f64> {
    let s = s.trim().trim_end_matches("pt").trim().replace(',', ".");
    let pt = s.parse::<f64>().ok()?;
    if pt <= 0.0 {
        return None;
    }
    Some(pt_to_mm(pt))
}

/// Helper to compute device bounding box from optional DeviceManager
pub(crate) fn compute_device_bbox(
    device_manager: &Option<Arc<DeviceManager>>,
) -> (f64, f64, f64, f64) {
    if let Some(dm) = device_manager {
        if let Some(profile) = dm.get_active_profile() {
            return (
                profile.x_axis.min,
                profile.y_axis.min,
                profile.x_axis.max,
                profile.y_axis.max,
            );
        }
    }
    (
        0.0,
        0.0,
        core_constants::DEFAULT_WORK_WIDTH_MM,
        core_constants::DEFAULT_WORK_HEIGHT_MM,
    )
}

/// Handle positions for resize operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct DesignerCanvas {
    pub widget: DrawingArea,
    pub state: Rc<RefCell<DesignerState>>,
    pub mouse_pos: Rc<RefCell<(f64, f64)>>, // Canvas coordinates
    pub(crate) toolbox: Option<Rc<DesignerToolbox>>,
    pub(crate) properties: Rc<RefCell<Option<Rc<PropertiesPanel>>>>,
    pub(crate) layers: Rc<RefCell<Option<Rc<LayersPanel>>>>,
    // Shape creation state
    pub(crate) creation_start: Rc<RefCell<Option<(f64, f64)>>>,
    pub(crate) creation_current: Rc<RefCell<Option<(f64, f64)>>>,
    // Track last drag offset for incremental movement
    pub(crate) last_drag_offset: Rc<RefCell<(f64, f64)>>,
    // Track if a drag operation occurred
    pub(crate) did_drag: Rc<RefCell<bool>>,
    // Resize handle state
    pub(crate) active_resize_handle: Rc<RefCell<Option<(ResizeHandle, u64)>>>, // (handle, shape_id)
    pub(crate) resize_original_bounds: Rc<RefCell<Option<(f64, f64, f64, f64)>>>, // (x, y, width, height)
    pub(crate) resize_original_shapes: Rc<RefCell<Option<Vec<(u64, Shape)>>>>,
    // Scroll adjustments
    pub(crate) hadjustment: Rc<RefCell<Option<gtk4::Adjustment>>>,
    pub(crate) vadjustment: Rc<RefCell<Option<gtk4::Adjustment>>>,
    // Keyboard state
    pub shift_pressed: Rc<RefCell<bool>>,
    pub(crate) ctrl_pressed: Rc<RefCell<bool>>,
    // Polyline state
    pub(crate) polyline_points: Rc<RefCell<Vec<Point>>>,
    // Preview shapes (e.g. for offset/fillet)
    pub preview_shapes: Rc<RefCell<Vec<Shape>>>,
    // Toolpath preview
    pub(crate) preview_toolpaths: Rc<RefCell<Vec<Toolpath>>>,
    pub preview_generating: Rc<std::cell::Cell<bool>>,
    pub(crate) preview_pending: Rc<std::cell::Cell<bool>>,
    pub preview_cancel: Arc<AtomicBool>,
    pub(crate) text_tool_dialog:
        Rc<RefCell<Option<(Dialog, Entry, DropDown, CheckButton, CheckButton, Entry)>>>,
    pub(crate) text_tool_last_font_family: Rc<RefCell<String>>,
    pub(crate) text_tool_last_bold: Rc<RefCell<bool>>,
    pub(crate) text_tool_last_italic: Rc<RefCell<bool>>,
    pub(crate) text_tool_last_size_mm: Rc<RefCell<f64>>,
    pub(crate) text_tool_pending_pos: Rc<RefCell<Option<(f64, f64)>>>,
    pub(crate) device_manager: Option<Arc<DeviceManager>>,
    pub(crate) status_bar: Option<crate::ui::gtk::status_bar::StatusBar>,
}

impl DesignerCanvas {
    pub fn new(
        state: Rc<RefCell<DesignerState>>,
        toolbox: Option<Rc<DesignerToolbox>>,
        device_manager: Option<Arc<DeviceManager>>,
        status_bar: Option<crate::ui::gtk::status_bar::StatusBar>,
        settings_controller: Option<Rc<SettingsController>>,
    ) -> Rc<Self> {
        let widget = DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .css_classes(vec!["designer-canvas"])
            .build();

        let mouse_pos = Rc::new(RefCell::new((0.0, 0.0)));
        let creation_start = Rc::new(RefCell::new(None));
        let creation_current = Rc::new(RefCell::new(None));
        let last_drag_offset = Rc::new(RefCell::new((0.0, 0.0)));
        let did_drag = Rc::new(RefCell::new(false));
        let polyline_points = Rc::new(RefCell::new(Vec::new()));
        let preview_shapes = Rc::new(RefCell::new(Vec::new()));
        let preview_toolpaths = Rc::new(RefCell::new(Vec::new()));

        let state_clone = state.clone();
        let mouse_pos_clone = mouse_pos.clone();
        let creation_start_clone = creation_start.clone();
        let creation_current_clone = creation_current.clone();
        let polyline_points_clone = polyline_points.clone();
        let preview_shapes_clone = preview_shapes.clone();
        let preview_toolpaths_clone = preview_toolpaths.clone();
        let device_manager_draw = device_manager.clone();
        let settings_draw = settings_controller.clone();

        let state_draw = state_clone.clone();
        widget.set_draw_func(move |drawing_area, cr, width, height| {
            // Update viewport size to match widget dimensions
            if let Ok(mut state) = state_draw.try_borrow_mut() {
                state.canvas.set_canvas_size(width as f64, height as f64);
            }

            let state = state_draw.borrow();
            let mouse = *mouse_pos_clone.borrow();
            let preview_start = *creation_start_clone.borrow();
            let preview_current = *creation_current_clone.borrow();
            let poly_points = polyline_points_clone.borrow();
            let preview_shapes = preview_shapes_clone.borrow();
            let toolpaths = preview_toolpaths_clone.borrow();
            let bounds = compute_device_bbox(&device_manager_draw);

            // Get grid line widths from settings (defaults if not available)
            let (grid_major_width, grid_minor_width) = if let Some(ref settings) = settings_draw {
                let config = settings.persistence.borrow();
                (
                    config.config().ui.grid_major_line_width,
                    config.config().ui.grid_minor_line_width,
                )
            } else {
                (2.0, 1.0)
            };

            let style_context = drawing_area.style_context();

            Self::draw(
                cr,
                &state,
                width as f64,
                height as f64,
                mouse,
                preview_start,
                preview_current,
                &poly_points,
                &preview_shapes,
                &toolpaths,
                bounds,
                &style_context,
                grid_major_width,
                grid_minor_width,
            );
        });

        let canvas = Rc::new(Self {
            widget: widget.clone(),
            state: state.clone(),
            mouse_pos: mouse_pos.clone(),
            toolbox: toolbox.clone(),
            properties: Rc::new(RefCell::new(None)),
            layers: Rc::new(RefCell::new(None)),
            creation_start: creation_start.clone(),
            creation_current: creation_current.clone(),
            last_drag_offset: last_drag_offset.clone(),
            did_drag: did_drag.clone(),
            active_resize_handle: Rc::new(RefCell::new(None)),
            resize_original_bounds: Rc::new(RefCell::new(None)),
            resize_original_shapes: Rc::new(RefCell::new(None)),
            hadjustment: Rc::new(RefCell::new(None)),
            vadjustment: Rc::new(RefCell::new(None)),
            shift_pressed: Rc::new(RefCell::new(false)),
            ctrl_pressed: Rc::new(RefCell::new(false)),
            polyline_points: polyline_points.clone(),
            preview_shapes: preview_shapes.clone(),
            preview_toolpaths: preview_toolpaths.clone(),
            preview_generating: Rc::new(std::cell::Cell::new(false)),
            preview_pending: Rc::new(std::cell::Cell::new(false)),
            preview_cancel: Arc::new(AtomicBool::new(false)),
            text_tool_dialog: Rc::new(RefCell::new(None)),
            text_tool_last_font_family: Rc::new(RefCell::new("Sans".to_string())),
            text_tool_last_bold: Rc::new(RefCell::new(false)),
            text_tool_last_italic: Rc::new(RefCell::new(false)),
            text_tool_last_size_mm: Rc::new(RefCell::new(pt_to_mm(20.0))),
            text_tool_pending_pos: Rc::new(RefCell::new(None)),
            device_manager: device_manager.clone(),
            status_bar,
        });

        // Mouse motion tracking
        let motion_ctrl = EventControllerMotion::new();
        let mouse_pos_motion = mouse_pos.clone();
        let widget_motion = widget.clone();
        let state_motion = state_clone.clone();
        let canvas_motion = canvas.clone();

        motion_ctrl.connect_motion(move |_, x, y| {
            // Convert screen coords to canvas coords
            let _width = widget_motion.width() as f64;
            let height = widget_motion.height() as f64;

            let state = state_motion.borrow();
            let zoom = state.canvas.zoom();
            let pan_x = state.canvas.pan_x();
            let pan_y = state.canvas.pan_y();
            drop(state);

            // Screen (x, y) -> Canvas (cx, cy)
            let y_flipped = height - y;
            let canvas_x = (x - pan_x) / zoom;
            let canvas_y = (y_flipped - pan_y) / zoom;

            *mouse_pos_motion.borrow_mut() = (canvas_x, canvas_y);

            // Update cursor based on tool
            let tool = canvas_motion
                .toolbox
                .as_ref()
                .map(|t| t.current_tool())
                .unwrap_or(DesignerTool::Select);

            match tool {
                DesignerTool::Select => widget_motion.set_cursor(None), // default arrow
                DesignerTool::Pan => {
                    if *canvas_motion.did_drag.borrow() {
                        widget_motion.set_cursor_from_name(Some("grabbing"));
                    } else {
                        widget_motion.set_cursor_from_name(Some("grab"));
                    }
                }
                DesignerTool::Text => widget_motion.set_cursor_from_name(Some("text")),
                // Drawing tools
                DesignerTool::Rectangle
                | DesignerTool::Circle
                | DesignerTool::Line
                | DesignerTool::Ellipse
                | DesignerTool::Triangle
                | DesignerTool::Polygon
                | DesignerTool::Polyline
                | DesignerTool::Gear
                | DesignerTool::Sprocket => widget_motion.set_cursor_from_name(Some("pencil")),
            }

            widget_motion.queue_draw();
        });
        widget.add_controller(motion_ctrl);

        // Scroll to pan (Ctrl+Scroll to zoom) — matches Visualizer
        let scroll_ctrl =
            gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::BOTH_AXES);
        let canvas_scroll = canvas.clone();
        scroll_ctrl.connect_scroll(move |ctrl, dx, dy| {
            let is_ctrl = ctrl
                .current_event_state()
                .contains(ModifierType::CONTROL_MASK);
            if is_ctrl {
                if dy > 0.0 {
                    canvas_scroll.zoom_out();
                } else if dy < 0.0 {
                    canvas_scroll.zoom_in();
                }
            } else {
                let pan_step = 20.0;
                let mut state = canvas_scroll.state.borrow_mut();
                let pan_x = state.canvas.pan_x();
                let pan_y = state.canvas.pan_y();
                state
                    .canvas
                    .set_pan(pan_x - dx * pan_step, pan_y + dy * pan_step);
                let pan_x = state.canvas.pan_x();
                let pan_y = state.canvas.pan_y();
                drop(state);

                if let Some(adj) = canvas_scroll.hadjustment.borrow().as_ref() {
                    adj.set_value(-pan_x);
                }
                if let Some(adj) = canvas_scroll.vadjustment.borrow().as_ref() {
                    adj.set_value(pan_y);
                }

                canvas_scroll.widget.queue_draw();
            }
            gtk4::glib::Propagation::Stop
        });
        widget.add_controller(scroll_ctrl);

        // Interaction controllers
        let click_gesture = GestureClick::new();
        click_gesture.set_button(1); // Left click only
        let canvas_click = canvas.clone();
        click_gesture.connect_pressed(move |gesture, n_press, x, y| {
            let modifiers = gesture.current_event_state();
            let ctrl_pressed = modifiers.contains(ModifierType::CONTROL_MASK);
            canvas_click.handle_click(x, y, ctrl_pressed, n_press);
        });

        let canvas_release = canvas.clone();
        click_gesture.connect_released(move |gesture, _n_press, x, y| {
            let modifiers = gesture.current_event_state();
            let ctrl_pressed = modifiers.contains(ModifierType::CONTROL_MASK);
            canvas_release.handle_release(x, y, ctrl_pressed);
        });

        widget.add_controller(click_gesture);

        // Right click gesture
        let right_click_gesture = GestureClick::new();
        right_click_gesture.set_button(3); // Right click
        let canvas_right_click = canvas.clone();
        right_click_gesture.connect_released(move |_gesture, _n_press, x, y| {
            canvas_right_click.handle_right_click(x, y);
        });
        widget.add_controller(right_click_gesture);

        let drag_gesture = GestureDrag::new();
        drag_gesture.set_button(1); // Left click only
        let canvas_drag = canvas.clone();
        drag_gesture.connect_drag_begin(move |_gesture, x, y| {
            canvas_drag.handle_drag_begin(x, y);
        });

        let canvas_drag_update = canvas.clone();
        drag_gesture.connect_drag_update(move |_gesture, offset_x, offset_y| {
            canvas_drag_update.handle_drag_update(offset_x, offset_y);
        });

        let canvas_drag_end = canvas.clone();
        drag_gesture.connect_drag_end(move |_gesture, offset_x, offset_y| {
            canvas_drag_end.handle_drag_end(offset_x, offset_y);
        });
        widget.add_controller(drag_gesture);

        // Keyboard controller for Delete, Escape, etc.
        let key_controller = gtk4::EventControllerKey::new();
        let state_key = state.clone();
        let widget_key = widget.clone();
        let shift_pressed_key = canvas.shift_pressed.clone();
        let ctrl_pressed_key = canvas.ctrl_pressed.clone();
        let polyline_points_key = canvas.polyline_points.clone();
        let layers_key = canvas.layers.clone();

        key_controller.connect_key_pressed(move |_controller, keyval, _keycode, _modifier| {
            if keyval == gtk4::gdk::Key::Shift_L || keyval == gtk4::gdk::Key::Shift_R {
                *shift_pressed_key.borrow_mut() = true;
                return glib::Propagation::Proceed;
            }
            if keyval == gtk4::gdk::Key::Control_L || keyval == gtk4::gdk::Key::Control_R {
                *ctrl_pressed_key.borrow_mut() = true;
                return glib::Propagation::Proceed;
            }

            let mut designer_state = state_key.borrow_mut();

            match keyval {
                gtk4::gdk::Key::Delete | gtk4::gdk::Key::BackSpace => {
                    // Delete selected shapes
                    if designer_state
                        .canvas
                        .selection_manager
                        .selected_id()
                        .is_some()
                    {
                        designer_state.delete_selected();
                        drop(designer_state);

                        // Refresh layers
                        if let Some(layers) = layers_key.borrow().as_ref() {
                            layers.refresh(&state_key);
                        }

                        widget_key.queue_draw();
                        return glib::Propagation::Stop;
                    }
                }
                gtk4::gdk::Key::Escape => {
                    // Cancel polyline creation
                    let mut points = polyline_points_key.borrow_mut();
                    if !points.is_empty() {
                        points.clear();
                        drop(points);
                        drop(designer_state);
                        widget_key.queue_draw();
                        return glib::Propagation::Stop;
                    }
                    drop(points);

                    // Deselect all
                    designer_state.canvas.deselect_all();
                    drop(designer_state);

                    // Refresh layers
                    if let Some(layers) = layers_key.borrow().as_ref() {
                        layers.refresh(&state_key);
                    }

                    widget_key.queue_draw();
                    return glib::Propagation::Stop;
                }
                gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter => {
                    // Finish polyline creation
                    let mut points = polyline_points_key.borrow_mut();
                    if !points.is_empty() {
                        if points.len() >= 2 {
                            // Create polyline
                            let path_shape = PathShape::from_points(&points, false);
                            let shape = Shape::Path(path_shape);

                            designer_state.add_shape_with_undo(shape);

                            // Refresh layers
                            if let Some(layers) = layers_key.borrow().as_ref() {
                                layers.refresh(&state_key);
                            }
                        }
                        points.clear();
                        drop(points);
                        drop(designer_state);
                        widget_key.queue_draw();
                        return glib::Propagation::Stop;
                    }
                }
                _ => {}
            }

            glib::Propagation::Proceed
        });

        let shift_released_key = canvas.shift_pressed.clone();
        let ctrl_released_key = canvas.ctrl_pressed.clone();
        key_controller.connect_key_released(move |_controller, keyval, _keycode, _modifier| {
            if keyval == gtk4::gdk::Key::Shift_L || keyval == gtk4::gdk::Key::Shift_R {
                *shift_released_key.borrow_mut() = false;
            }
            if keyval == gtk4::gdk::Key::Control_L || keyval == gtk4::gdk::Key::Control_R {
                *ctrl_released_key.borrow_mut() = false;
            }
        });

        widget.add_controller(key_controller);

        canvas
    }

    /// Fit the canvas to the active device working area (or a 250x250 mm fallback)
    pub fn fit_to_device_area(&self) {
        let (min_x, min_y, max_x, max_y) = compute_device_bbox(&self.device_manager);

        self.state.borrow_mut().canvas.fit_to_bounds(
            min_x,
            min_y,
            max_x,
            max_y,
            core_constants::VIEW_PADDING,
        );
    }
    pub fn set_properties_panel(&self, panel: Rc<PropertiesPanel>) {
        *self.properties.borrow_mut() = Some(panel);
    }

    pub fn set_layers_panel(&self, panel: Rc<LayersPanel>) {
        *self.layers.borrow_mut() = Some(panel);
    }

    pub fn set_adjustments(&self, hadj: gtk4::Adjustment, vadj: gtk4::Adjustment) {
        *self.hadjustment.borrow_mut() = Some(hadj);
        *self.vadjustment.borrow_mut() = Some(vadj);
    }
}
