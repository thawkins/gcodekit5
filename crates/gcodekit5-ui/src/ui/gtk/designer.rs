use crate::t;
use crate::ui::gtk::designer_layers::LayersPanel;
use crate::ui::gtk::designer_properties::PropertiesPanel;
use crate::ui::gtk::designer_toolbox::{DesignerTool, DesignerToolbox};
use crate::ui::gtk::osd_format::format_zoom_center_cursor;
use gcodekit5_core::constants as core_constants;
use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::commands::{DesignerCommand, PasteShapes, RemoveShape};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::font_manager;
use gcodekit5_designer::model::{
    DesignCircle as Circle, DesignEllipse as Ellipse, DesignLine as Line, DesignPath as PathShape,
    DesignPolygon as Polygon, DesignRectangle as Rectangle, DesignText as TextShape,
    DesignTriangle as Triangle, DesignerShape, Point, Shape,
};
use gcodekit5_designer::serialization::DesignFile;
use gcodekit5_designer::shapes::OperationType;
use gcodekit5_designer::toolpath::{Toolpath, ToolpathSegmentType};
use gcodekit5_devicedb::DeviceManager;
use gcodekit5_settings::controller::SettingsController;
use gtk4::gdk::{Key, ModifierType};
use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box, CheckButton, Dialog, DrawingArea, DropDown, Entry, EventControllerKey,
    EventControllerMotion, FileChooserAction, FileChooserNative, GestureClick, GestureDrag, Grid,
    Label, Orientation, Overlay, Paned, Popover, ResponseType, Scrollbar, Separator, StringList,
};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::error;
//use crate::ui::gtk::designer_file_ops; // Temporarily disabled

const MM_PER_PT: f64 = 25.4 / 72.0;

fn mm_to_pt(mm: f64) -> f64 {
    mm / MM_PER_PT
}

fn pt_to_mm(pt: f64) -> f64 {
    pt * MM_PER_PT
}

fn format_font_points(mm: f64) -> String {
    format!("{:.2}", mm_to_pt(mm))
}

fn parse_font_points_mm(s: &str) -> Option<f64> {
    let s = s.trim().trim_end_matches("pt").trim().replace(',', ".");
    let pt = s.parse::<f64>().ok()?;
    if pt <= 0.0 {
        return None;
    }
    Some(pt_to_mm(pt))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Helper to compute device bounding box from optional DeviceManager
fn compute_device_bbox(device_manager: &Option<Arc<DeviceManager>>) -> (f64, f64, f64, f64) {
    if let Some(dm) = device_manager {
        if let Some(profile) = dm.get_active_profile() {
            return (
                profile.x_axis.min as f64,
                profile.y_axis.min as f64,
                profile.x_axis.max as f64,
                profile.y_axis.max as f64,
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

#[cfg(test)]
mod tests_designer {
    use super::*;

    #[test]
    fn test_compute_device_bbox_default() {
        let bbox = compute_device_bbox(&None);
        assert_eq!(
            bbox,
            (
                0.0,
                0.0,
                gcodekit5_core::constants::DEFAULT_WORK_WIDTH_MM,
                gcodekit5_core::constants::DEFAULT_WORK_HEIGHT_MM
            )
        );
    }
}

#[derive(Clone)]
pub struct DesignerCanvas {
    pub widget: DrawingArea,
    state: Rc<RefCell<DesignerState>>,
    mouse_pos: Rc<RefCell<(f64, f64)>>, // Canvas coordinates
    toolbox: Option<Rc<DesignerToolbox>>,
    properties: Rc<RefCell<Option<Rc<PropertiesPanel>>>>,
    layers: Rc<RefCell<Option<Rc<LayersPanel>>>>,
    // Shape creation state
    creation_start: Rc<RefCell<Option<(f64, f64)>>>,
    creation_current: Rc<RefCell<Option<(f64, f64)>>>,
    // Track last drag offset for incremental movement
    last_drag_offset: Rc<RefCell<(f64, f64)>>,
    // Track if a drag operation occurred
    did_drag: Rc<RefCell<bool>>,
    // Resize handle state
    active_resize_handle: Rc<RefCell<Option<(ResizeHandle, u64)>>>, // (handle, shape_id)
    resize_original_bounds: Rc<RefCell<Option<(f64, f64, f64, f64)>>>, // (x, y, width, height)
    resize_original_shapes: Rc<RefCell<Option<Vec<(u64, Shape)>>>>,
    // Scroll adjustments
    hadjustment: Rc<RefCell<Option<gtk4::Adjustment>>>,
    vadjustment: Rc<RefCell<Option<gtk4::Adjustment>>>,
    // Keyboard state
    shift_pressed: Rc<RefCell<bool>>,
    ctrl_pressed: Rc<RefCell<bool>>,
    // Polyline state
    polyline_points: Rc<RefCell<Vec<Point>>>,
    // Preview shapes (e.g. for offset/fillet)
    preview_shapes: Rc<RefCell<Vec<Shape>>>,
    // Toolpath preview
    preview_toolpaths: Rc<RefCell<Vec<Toolpath>>>,
    preview_generating: Rc<std::cell::Cell<bool>>,
    preview_pending: Rc<std::cell::Cell<bool>>,
    preview_cancel: Arc<AtomicBool>,
    text_tool_dialog:
        Rc<RefCell<Option<(Dialog, Entry, DropDown, CheckButton, CheckButton, Entry)>>>,
    text_tool_last_font_family: Rc<RefCell<String>>,
    text_tool_last_bold: Rc<RefCell<bool>>,
    text_tool_last_italic: Rc<RefCell<bool>>,
    text_tool_last_size_mm: Rc<RefCell<f64>>,
    text_tool_pending_pos: Rc<RefCell<Option<(f64, f64)>>>,
    device_manager: Option<Arc<DeviceManager>>,
    status_bar: Option<crate::ui::gtk::status_bar::StatusBar>,
}

pub struct DesignerView {
    pub widget: Box,
    canvas: Rc<DesignerCanvas>,
    toolbox: Rc<DesignerToolbox>,
    _properties: Rc<PropertiesPanel>,
    layers: Rc<LayersPanel>,
    status_label: Label,
    _coord_label: Label,
    current_file: Rc<RefCell<Option<PathBuf>>>,
    on_gcode_generated: Rc<RefCell<Option<std::boxed::Box<dyn Fn(String)>>>>,
    settings_persistence: Option<Rc<RefCell<gcodekit5_settings::SettingsPersistence>>>,
}

impl DesignerCanvas {
    pub fn new(
        state: Rc<RefCell<DesignerState>>,
        toolbox: Option<Rc<DesignerToolbox>>,
        device_manager: Option<Arc<DeviceManager>>,
        status_bar: Option<crate::ui::gtk::status_bar::StatusBar>,
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
                | DesignerTool::Sprocket
                | DesignerTool::TabbedBox => widget_motion.set_cursor_from_name(Some("pencil")),
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
        right_click_gesture.connect_pressed(move |_gesture, _n_press, x, y| {
            canvas_right_click.handle_right_click(x, y);
        });
        widget.add_controller(right_click_gesture);

        let drag_gesture = GestureDrag::new();
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
                        designer_state.canvas.remove_selected();
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

                            designer_state.canvas.add_shape(shape);

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

    pub fn zoom_in(&self) {
        let mut state = self.state.borrow_mut();
        let current_zoom = state.canvas.zoom();
        state.canvas.set_zoom(current_zoom * 1.2);
        drop(state);
        self.widget.queue_draw();
    }

    pub fn zoom_out(&self) {
        let mut state = self.state.borrow_mut();
        let current_zoom = state.canvas.zoom();
        state.canvas.set_zoom(current_zoom / 1.2);
        drop(state);
        self.widget.queue_draw();
    }

    pub fn reset_view(&self) {
        let (pan_x, pan_y) = {
            let mut state = self.state.borrow_mut();
            state.canvas.set_zoom(1.0);
            state.canvas.set_pan(0.0, 0.0);
            (state.canvas.pan_x(), state.canvas.pan_y())
        };

        if let Some(adj) = self.hadjustment.borrow().as_ref() {
            adj.set_value(-pan_x);
        }
        if let Some(adj) = self.vadjustment.borrow().as_ref() {
            adj.set_value(pan_y);
        }

        self.widget.queue_draw();
    }

    pub fn zoom_fit(&self) {
        let (target_pan_x, target_pan_y) = {
            let mut state = self.state.borrow_mut();

            // Calculate bounds of all shapes
            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;

            let mut has_shapes = false;
            for obj in state.canvas.shapes() {
                has_shapes = true;
                let (sx, sy, ex, ey) = obj.get_total_bounds();
                min_x = min_x.min(sx);
                min_y = min_y.min(sy);
                max_x = max_x.max(ex);
                max_y = max_y.max(ey);
            }

            if has_shapes {
                // Fit content using the viewport fit-to-bounds (5% padding)
                state.canvas.fit_to_bounds(
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    core_constants::VIEW_PADDING,
                );
            } else {
                // No shapes: attempt device profile bounds, fallback to 250x250 mm
                let (min_x, min_y, max_x, max_y) = if let Some(dm) = &self.device_manager {
                    if let Some(profile) = dm.get_active_profile() {
                        (
                            profile.x_axis.min as f64,
                            profile.y_axis.min as f64,
                            profile.x_axis.max as f64,
                            profile.y_axis.max as f64,
                        )
                    } else {
                        (
                            0.0,
                            0.0,
                            core_constants::DEFAULT_WORK_WIDTH_MM,
                            core_constants::DEFAULT_WORK_HEIGHT_MM,
                        )
                    }
                } else {
                    (
                        0.0,
                        0.0,
                        core_constants::DEFAULT_WORK_WIDTH_MM,
                        core_constants::DEFAULT_WORK_HEIGHT_MM,
                    )
                };

                state.canvas.fit_to_bounds(
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    core_constants::VIEW_PADDING,
                );
            }

            (state.canvas.pan_x(), state.canvas.pan_y())
        };

        // Update adjustments safely
        if let Some(adj) = self.hadjustment.borrow().as_ref() {
            adj.set_value(-target_pan_x);
        }
        if let Some(adj) = self.vadjustment.borrow().as_ref() {
            adj.set_value(target_pan_y);
        }

        self.widget.queue_draw();
    }

    fn copy_cursor_coordinates(&self) {
        let (x, y) = *self.mouse_pos.borrow();
        let text = format!("X {:.2} mm  Y {:.2} mm", x, y);
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&text);
        }
    }

    fn toggle_grid(&self) {
        let mut state = self.state.borrow_mut();
        state.show_grid = !state.show_grid;
        drop(state);
        self.widget.queue_draw();
    }

    fn toggle_snap(&self) {
        let mut state = self.state.borrow_mut();
        state.snap_enabled = !state.snap_enabled;
    }

    fn toggle_toolpaths(&self) {
        let mut state = self.state.borrow_mut();
        state.show_toolpaths = !state.show_toolpaths;
        let show = state.show_toolpaths;
        drop(state);
        if show {
            self.generate_preview_toolpaths();
        } else {
            self.widget.queue_draw();
        }
    }

    /// Public method to fit to device working area from DesignerView
    // removed; wrapper belongs on DesignerView

    fn handle_right_click(&self, x: f64, y: f64) {
        // Check if we are building a polyline
        {
            let mut points = self.polyline_points.borrow_mut();
            if !points.is_empty() {
                if points.len() >= 2 {
                    // Create polyline
                    let path_shape = PathShape::from_points(&points, false); // Open polyline
                    let shape = Shape::Path(path_shape);

                    let mut state = self.state.borrow_mut();
                    state.canvas.add_shape(shape);
                    drop(state);

                    // Refresh layers panel
                    if let Some(layers_panel) = self.layers.borrow().as_ref() {
                        layers_panel.refresh(&self.state);
                    }
                }
                points.clear();
                self.widget.queue_draw();
                return;
            }
        }

        // Convert to canvas coordinates for hit testing
        let height = self.widget.height() as f64;
        let state_borrow = self.state.borrow();
        let zoom = state_borrow.canvas.zoom();
        let pan_x = state_borrow.canvas.pan_x();
        let pan_y = state_borrow.canvas.pan_y();
        drop(state_borrow);

        let y_flipped = height - y;
        let canvas_x = (x - pan_x) / zoom;
        let canvas_y = (y_flipped - pan_y) / zoom;
        let (canvas_x, canvas_y) = self.snap_canvas_point(canvas_x, canvas_y);
        let point = Point::new(canvas_x, canvas_y);

        let mut state = self.state.borrow_mut();

        // Check for hit using select_at logic (handles groups)
        let tolerance = 3.0;
        let clicked_shape_id = state.canvas.select_at(&point, tolerance, false);

        if let Some(_id) = clicked_shape_id {
            // Update UI
            drop(state); // Drop before calling update methods that might borrow
            if let Some(ref props) = *self.properties.borrow() {
                props.update_from_selection();
            }
            if let Some(ref layers) = *self.layers.borrow() {
                layers.refresh(&self.state);
            }
            self.widget.queue_draw();

            // Re-borrow for next steps
            state = self.state.borrow_mut();
        }

        let has_selection = state
            .canvas
            .selection_manager
            .selected_count(&state.canvas.shape_store)
            > 0;
        let selected_count = state.canvas.shapes().filter(|s| s.selected).count();
        let can_paste = !state.clipboard.is_empty();
        let can_group = state.can_group();
        let can_ungroup = state.can_ungroup();
        let can_align = selected_count >= 2;
        let can_boolean = selected_count >= 2;
        drop(state);

        let menu = Popover::new();
        menu.set_parent(&self.widget);
        menu.set_has_arrow(false);
        // Set position
        let rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
        menu.set_pointing_to(Some(&rect));

        let vbox = Box::new(Orientation::Vertical, 0);
        vbox.add_css_class("context-menu");

        // Helper to create menu items
        let create_item = |label: &str, action: &str, enabled: bool| {
            let btn = gtk4::Button::builder()
                .label(label)
                .has_frame(false)
                .halign(gtk4::Align::Start)
                .build();
            btn.set_sensitive(enabled);

            let canvas = self.clone();
            let menu_clone = menu.clone();
            let action_name = action.to_string();

            btn.connect_clicked(move |_| {
                menu_clone.popdown();
                match action_name.as_str() {
                    "fit_content" => canvas.zoom_fit(),
                    "fit_viewport" => canvas.reset_view(),
                    "fit_device" => {
                        canvas.fit_to_device_area();
                        canvas.widget.queue_draw();
                    }
                    "copy_cursor" => canvas.copy_cursor_coordinates(),
                    "toggle_grid" => canvas.toggle_grid(),
                    "toggle_snap" => canvas.toggle_snap(),
                    "toggle_toolpaths" => canvas.toggle_toolpaths(),
                    "cut" => canvas.cut(),
                    "copy" => canvas.copy_selected(),
                    "paste" => canvas.paste(),
                    "delete" => canvas.delete_selected(),
                    "group" => canvas.group_selected(),
                    "ungroup" => canvas.ungroup_selected(),
                    "boolean_union" => canvas.boolean_union(),
                    "boolean_difference" => canvas.boolean_difference(),
                    "boolean_intersection" => canvas.boolean_intersection(),
                    "convert_to_path" => canvas.convert_to_path(),
                    "convert_to_rectangle" => canvas.convert_to_rectangle(),
                    "mirror_x" => canvas.mirror_x(),
                    "mirror_y" => canvas.mirror_y(),
                    _ => {}
                }
            });

            btn
        };

        // Edit
        vbox.append(&create_item("Cut", "cut", has_selection));
        vbox.append(&create_item("Copy", "copy", has_selection));
        vbox.append(&create_item("Paste", "paste", can_paste));
        vbox.append(&create_item("Delete", "delete", has_selection));

        vbox.append(&Separator::new(Orientation::Horizontal));

        vbox.append(&create_item("Group", "group", can_group));
        vbox.append(&create_item("Ungroup", "ungroup", can_ungroup));

        vbox.append(&Separator::new(Orientation::Horizontal));
        vbox.append(&create_item("Union", "boolean_union", can_boolean));
        vbox.append(&create_item("Diff", "boolean_difference", can_boolean));
        vbox.append(&create_item("Inter", "boolean_intersection", can_boolean));
        vbox.append(&create_item("Mirror on X", "mirror_x", has_selection));
        vbox.append(&create_item("Mirror on Y", "mirror_y", has_selection));

        if can_align {
            let align_btn = gtk4::Button::builder()
                .label("Align ▸")
                .has_frame(false)
                .halign(gtk4::Align::Start)
                .build();

            let align_menu = Popover::new();
            align_menu.set_parent(&align_btn);
            align_menu.set_has_arrow(false);
            align_menu.set_position(gtk4::PositionType::Right);

            let align_vbox = Box::new(Orientation::Vertical, 0);
            align_vbox.add_css_class("context-menu");

            // Helper for align items
            let create_align_item = |label: &str, action: &str| {
                let btn = gtk4::Button::builder()
                    .label(label)
                    .has_frame(false)
                    .halign(gtk4::Align::Start)
                    .build();

                let canvas = self.clone();
                let menu_clone = menu.clone(); // Main menu
                let align_menu_clone = align_menu.clone();
                let action_name = action.to_string();

                btn.connect_clicked(move |_| {
                    align_menu_clone.popdown();
                    menu_clone.popdown();
                    match action_name.as_str() {
                        "align_left" => canvas.align_left(),
                        "align_right" => canvas.align_right(),
                        "align_top" => canvas.align_top(),
                        "align_bottom" => canvas.align_bottom(),
                        "align_center_x" => canvas.align_center_horizontal(),
                        "align_center_y" => canvas.align_center_vertical(),
                        _ => {}
                    }
                });
                btn
            };

            align_vbox.append(&create_align_item("Align Left", "align_left"));
            align_vbox.append(&create_align_item("Align Right", "align_right"));
            align_vbox.append(&create_align_item("Align Top", "align_top"));
            align_vbox.append(&create_align_item("Align Bottom", "align_bottom"));
            align_vbox.append(&create_align_item("Align Center X", "align_center_x"));
            align_vbox.append(&create_align_item("Align Center Y", "align_center_y"));

            align_menu.set_child(Some(&align_vbox));

            align_btn.connect_clicked(move |_| {
                align_menu.popup();
            });

            vbox.append(&align_btn);
        }

        vbox.append(&Separator::new(Orientation::Horizontal));
        vbox.append(&create_item(
            "Convert to Path",
            "convert_to_path",
            has_selection,
        ));
        vbox.append(&create_item(
            "Convert to Rectangle",
            "convert_to_rectangle",
            has_selection,
        ));

        menu.set_child(Some(&vbox));
        menu.set_autohide(true);
        menu.popup();
    }

    fn snap_canvas_point(&self, x: f64, y: f64) -> (f64, f64) {
        let state = self.state.borrow();
        if !state.snap_enabled {
            return (x, y);
        }
        let spacing = state.grid_spacing_mm;
        if spacing <= 0.0 {
            return (x, y);
        }
        let threshold = state.snap_threshold_mm.max(0.0);

        let sx = (x / spacing).round() * spacing;
        let sy = (y / spacing).round() * spacing;

        let out_x = if (sx - x).abs() <= threshold { sx } else { x };
        let out_y = if (sy - y).abs() <= threshold { sy } else { y };
        (out_x, out_y)
    }

    fn open_text_tool_dialog(&self, canvas_x: f64, canvas_y: f64) {
        *self.text_tool_pending_pos.borrow_mut() = Some((canvas_x, canvas_y));

        if self.text_tool_dialog.borrow().is_none() {
            let dialog = Dialog::builder()
                .title(t!("Add Text"))
                .modal(true)
                .resizable(false)
                .build();
            dialog.add_button(&t!("Cancel"), ResponseType::Cancel);
            dialog.add_button(&t!("Add"), ResponseType::Ok);
            dialog.set_default_response(ResponseType::Ok);

            let content = Box::new(Orientation::Vertical, 10);
            content.set_margin_top(12);
            content.set_margin_bottom(12);
            content.set_margin_start(12);
            content.set_margin_end(12);

            let header = Label::new(Some(&t!("Text")));
            header.add_css_class("title-3");
            header.set_halign(gtk4::Align::Start);
            content.append(&header);

            let entry = Entry::new();
            entry.set_placeholder_text(Some(&t!("Enter text")));
            entry.set_activates_default(true);
            content.append(&entry);

            // Font + attributes
            let grid = Grid::builder().row_spacing(8).column_spacing(8).build();

            let font_label = Label::new(Some(&t!("Font:")));
            font_label.set_halign(gtk4::Align::Start);

            let font_model = StringList::new(&[]);
            font_model.append("Sans");
            for fam in font_manager::list_font_families() {
                if fam != "Sans" {
                    font_model.append(&fam);
                }
            }
            let font_combo = DropDown::new(Some(font_model), None::<gtk4::Expression>);
            font_combo.set_hexpand(true);

            let size_label = Label::new(Some(&t!("Size:")));
            size_label.set_halign(gtk4::Align::Start);
            let size_entry = Entry::new();
            size_entry.set_hexpand(true);
            let size_unit = Label::new(Some("pt"));
            size_unit.set_width_chars(4);
            size_unit.set_halign(gtk4::Align::End);
            size_unit.set_xalign(1.0);

            let bold_check = CheckButton::with_label(&t!("Bold"));
            let italic_check = CheckButton::with_label(&t!("Italic"));
            let style_box = Box::new(Orientation::Horizontal, 8);
            style_box.append(&bold_check);
            style_box.append(&italic_check);

            let style_label = Label::new(Some(&t!("Style:")));
            style_label.set_halign(gtk4::Align::Start);

            grid.attach(&font_label, 0, 0, 1, 1);
            grid.attach(&font_combo, 1, 0, 2, 1);
            grid.attach(&size_label, 0, 1, 1, 1);
            grid.attach(&size_entry, 1, 1, 1, 1);
            grid.attach(&size_unit, 2, 1, 1, 1);
            grid.attach(&style_label, 0, 2, 1, 1);
            grid.attach(&style_box, 1, 2, 2, 1);

            content.append(&grid);

            dialog.content_area().append(&content);

            let canvas = self.clone();
            let entry_clone = entry.clone();
            let font_combo_clone = font_combo.clone();
            let bold_clone = bold_check.clone();
            let italic_clone = italic_check.clone();
            let size_entry_clone = size_entry.clone();

            dialog.connect_response(move |d, resp| {
                if resp == ResponseType::Ok {
                    let text = entry_clone.text().trim().to_string();
                    if !text.is_empty() {
                        if let Some((x, y)) = *canvas.text_tool_pending_pos.borrow() {
                            let family = font_combo_clone
                                .selected_item()
                                .and_downcast::<gtk4::StringObject>()
                                .map(|s| s.string().to_string())
                                .unwrap_or_else(|| "Sans".to_string());
                            let bold = bold_clone.is_active();
                            let italic = italic_clone.is_active();
                            let size_mm = parse_font_points_mm(&size_entry_clone.text())
                                .unwrap_or_else(|| pt_to_mm(20.0));

                            *canvas.text_tool_last_font_family.borrow_mut() = family.clone();
                            *canvas.text_tool_last_bold.borrow_mut() = bold;
                            *canvas.text_tool_last_italic.borrow_mut() = italic;
                            *canvas.text_tool_last_size_mm.borrow_mut() = size_mm;

                            let mut state = canvas.state.borrow_mut();
                            let mut shape = TextShape::new(text, x, y, size_mm);
                            shape.font_family = family;
                            shape.bold = bold;
                            shape.italic = italic;
                            let id = state.canvas.add_shape(Shape::Text(shape));
                            state.canvas.deselect_all();
                            state.canvas.select_shape(id, false);
                            drop(state);

                            canvas.widget.queue_draw();

                            if let Some(ref props) = *canvas.properties.borrow() {
                                props.update_from_selection();
                            }
                            if let Some(ref layers) = *canvas.layers.borrow() {
                                layers.refresh(&canvas.state);
                            }
                        }
                    }
                }

                entry_clone.set_text("");
                d.hide();
            });

            *self.text_tool_dialog.borrow_mut() = Some((
                dialog,
                entry,
                font_combo,
                bold_check,
                italic_check,
                size_entry,
            ));
        }

        if let Some((dialog, entry, font_combo, bold_check, italic_check, size_entry)) =
            self.text_tool_dialog.borrow().as_ref()
        {
            if let Some(root) = self.widget.root() {
                if let Ok(win) = root.downcast::<gtk4::Window>() {
                    dialog.set_transient_for(Some(&win));
                }
            }

            // Restore last-used values
            let family = self.text_tool_last_font_family.borrow().clone();
            let mut family_idx = 0;
            if let Some(model) = font_combo.model().and_downcast::<gtk4::StringList>() {
                for i in 0..model.n_items() {
                    if let Some(obj) = model.item(i) {
                        if let Ok(s) = obj.downcast::<gtk4::StringObject>() {
                            if s.string() == family {
                                family_idx = i;
                                break;
                            }
                        }
                    }
                }
            }
            font_combo.set_selected(family_idx);
            bold_check.set_active(*self.text_tool_last_bold.borrow());
            italic_check.set_active(*self.text_tool_last_italic.borrow());
            size_entry.set_text(&format_font_points(*self.text_tool_last_size_mm.borrow()));

            dialog.present();
            entry.grab_focus();
        }
    }

    fn handle_click(&self, x: f64, y: f64, ctrl_pressed_arg: bool, n_press: i32) {
        // Combine gesture modifier state with tracked keyboard state for reliability
        let ctrl_pressed = ctrl_pressed_arg || *self.ctrl_pressed.borrow();

        // Reset drag flag
        *self.did_drag.borrow_mut() = false;

        // Clear properties panel focus when user clicks on canvas
        if let Some(ref props) = *self.properties.borrow() {
            props.clear_focus();
        }

        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);

        // Convert screen coords to canvas coords
        let _width = self.widget.width() as f64;
        let height = self.widget.height() as f64;

        let state = self.state.borrow();
        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();
        drop(state);

        let y_flipped = height - y;
        let raw_canvas_x = (x - pan_x) / zoom;
        let raw_canvas_y = (y_flipped - pan_y) / zoom;
        let (snapped_x, snapped_y) = self.snap_canvas_point(raw_canvas_x, raw_canvas_y);

        // Use raw coordinates for selection to ensure we can click handles/shapes even if they are off-grid.
        // Use snapped coordinates for drawing tools.
        let (canvas_x, canvas_y) = if tool == DesignerTool::Select {
            (raw_canvas_x, raw_canvas_y)
        } else {
            (snapped_x, snapped_y)
        };

        match tool {
            DesignerTool::Select => {
                // Handle selection
                let mut state = self.state.borrow_mut();
                let point = Point::new(canvas_x, canvas_y);

                // If the click is on a resize handle for the current selection, do not
                // change selection here. Handles extend outside shapes, and a normal
                // empty-space click would deselect and prevent resizing.
                let selected_count = state.canvas.shapes().filter(|s| s.selected).count();
                if selected_count > 0 {
                    let bounds_opt = if selected_count > 1 {
                        let mut min_x = f64::INFINITY;
                        let mut min_y = f64::INFINITY;
                        let mut max_x = f64::NEG_INFINITY;
                        let mut max_y = f64::NEG_INFINITY;
                        let mut any = false;

                        for obj in state.canvas.shapes().filter(|s| s.selected) {
                            let (x1, y1, x2, y2) = Self::selection_bounds(&obj.shape);
                            min_x = min_x.min(x1);
                            min_y = min_y.min(y1);
                            max_x = max_x.max(x2);
                            max_y = max_y.max(y2);
                            any = true;
                        }

                        if any {
                            Some((min_x, min_y, max_x, max_y))
                        } else {
                            None
                        }
                    } else if let Some(selected_id) = state.canvas.selection_manager.selected_id() {
                        state
                            .canvas
                            .shapes()
                            .find(|s| s.id == selected_id)
                            .map(|obj| Self::selection_bounds(&obj.shape))
                    } else {
                        None
                    };

                    if let Some(bounds) = bounds_opt {
                        if self
                            .get_resize_handle_at(canvas_x, canvas_y, &bounds, zoom)
                            .is_some()
                        {
                            return;
                        }
                    }
                }

                // Check if we clicked on an existing shape
                let mut clicked_shape_id = None;
                let tolerance = 3.0;
                for obj in state.canvas.shapes() {
                    if obj.contains_point(&point, tolerance) {
                        clicked_shape_id = Some(obj.id);
                    }
                }

                if let Some(id) = clicked_shape_id {
                    // Check if it's already selected
                    let is_selected = state.canvas.selection_manager.selected_id() == Some(id)
                        || state.canvas.shapes().any(|s| s.id == id && s.selected);

                    if is_selected && !ctrl_pressed {
                        // Clicked on already selected item, and no Ctrl.
                        // Do NOT change selection yet. Wait for release.
                        // This allows dragging the current selection group.
                        return;
                    }
                }

                // Try to select shape at click point with multi-select if Ctrl is held
                if let Some(_selected_id) = state.canvas.select_at(&point, tolerance, ctrl_pressed)
                {
                    // Shape selected
                } else if !ctrl_pressed {
                    // Click on empty space without Ctrl - deselect all
                    state.canvas.deselect_all();
                }

                drop(state);
                self.widget.queue_draw();

                // Update properties panel
                if let Some(ref props) = *self.properties.borrow() {
                    props.update_from_selection();
                }

                // Update layers panel
                if let Some(ref layers) = *self.layers.borrow() {
                    layers.refresh(&self.state);
                }
            }
            DesignerTool::Polyline => {
                if n_press == 2 {
                    // Double click - finish
                    let mut points = self.polyline_points.borrow_mut();
                    if points.len() >= 2 {
                        // Create polyline
                        let path_shape = PathShape::from_points(&points, false);
                        let shape = Shape::Path(path_shape);

                        let mut state = self.state.borrow_mut();
                        state.canvas.add_shape(shape);
                        drop(state);

                        // Refresh layers panel
                        if let Some(layers_panel) = self.layers.borrow().as_ref() {
                            layers_panel.refresh(&self.state);
                        }
                    }
                    points.clear();
                    self.widget.queue_draw();
                } else {
                    let mut points = self.polyline_points.borrow_mut();
                    points.push(Point::new(canvas_x, canvas_y));
                    drop(points);
                    self.widget.queue_draw();
                }
            }
            DesignerTool::Text => {
                // Click-to-place text.
                self.open_text_tool_dialog(canvas_x, canvas_y);
            }
            _ => {
                // Other tools handled by drag
            }
        }
    }

    fn handle_release(&self, x: f64, y: f64, ctrl_pressed_arg: bool) {
        let ctrl_pressed = ctrl_pressed_arg || *self.ctrl_pressed.borrow();

        if *self.did_drag.borrow() {
            return;
        }

        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);

        // Convert screen coords to canvas coords
        let _width = self.widget.width() as f64;
        let height = self.widget.height() as f64;

        let state = self.state.borrow();
        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();
        drop(state);

        let y_flipped = height - y;
        let raw_canvas_x = (x - pan_x) / zoom;
        let raw_canvas_y = (y_flipped - pan_y) / zoom;
        let (snapped_x, snapped_y) = self.snap_canvas_point(raw_canvas_x, raw_canvas_y);

        // Use raw coordinates for selection to ensure we can click handles/shapes even if they are off-grid.
        // Use snapped coordinates for drawing tools.
        let (canvas_x, canvas_y) = if tool == DesignerTool::Select {
            (raw_canvas_x, raw_canvas_y)
        } else {
            (snapped_x, snapped_y)
        };

        if tool == DesignerTool::Select {
            let mut state = self.state.borrow_mut();
            let point = Point::new(canvas_x, canvas_y);

            // Check if we clicked on an existing shape
            let mut clicked_shape_id = None;
            let tolerance = 3.0;
            for obj in state.canvas.shapes() {
                if obj.contains_point(&point, tolerance) {
                    clicked_shape_id = Some(obj.id);
                }
            }

            if let Some(id) = clicked_shape_id {
                let is_selected = state.canvas.shapes().any(|s| s.id == id && s.selected);

                if is_selected && !ctrl_pressed {
                    // We clicked on a selected item and didn't drag.
                    // Now we select ONLY this item (deselect others).
                    state.canvas.deselect_all();
                    state.canvas.select_shape(id, false);

                    drop(state);
                    self.widget.queue_draw();

                    // Update properties panel
                    if let Some(ref props) = *self.properties.borrow() {
                        props.update_from_selection();
                    }

                    // Update layers panel
                    if let Some(ref layers) = *self.layers.borrow() {
                        layers.refresh(&self.state);
                    }
                }
            }
        }
    }

    fn handle_drag_begin(&self, x: f64, y: f64) {
        // Set drag flag
        *self.did_drag.borrow_mut() = true;

        // Clear properties panel focus when user drags on canvas
        if let Some(ref props) = *self.properties.borrow() {
            props.clear_focus();
        }

        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);

        // Convert screen coords to canvas coords
        let _width = self.widget.width() as f64;
        let height = self.widget.height() as f64;

        let state = self.state.borrow();
        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();
        drop(state);

        let y_flipped = height - y;
        let raw_canvas_x = (x - pan_x) / zoom;
        let raw_canvas_y = (y_flipped - pan_y) / zoom;
        let (snapped_x, snapped_y) = self.snap_canvas_point(raw_canvas_x, raw_canvas_y);

        // Use raw coordinates for selection to ensure we can click handles/shapes even if they are off-grid.
        // Use snapped coordinates for drawing tools.
        let (canvas_x, canvas_y) = if tool == DesignerTool::Select {
            (raw_canvas_x, raw_canvas_y)
        } else {
            (snapped_x, snapped_y)
        };

        match tool {
            DesignerTool::Select => {
                // Check if we're clicking on a resize handle first
                let (selected_id_opt, bounds_opt, is_group_resize) = {
                    let state = self.state.borrow();

                    let selected_count = state.canvas.shapes().filter(|s| s.selected).count();
                    if selected_count > 1 {
                        let mut min_x = f64::INFINITY;
                        let mut min_y = f64::INFINITY;
                        let mut max_x = f64::NEG_INFINITY;
                        let mut max_y = f64::NEG_INFINITY;
                        let mut any = false;

                        for obj in state.canvas.shapes().filter(|s| s.selected) {
                            let (x1, y1, x2, y2) = Self::selection_bounds(&obj.shape);
                            min_x = min_x.min(x1);
                            min_y = min_y.min(y1);
                            max_x = max_x.max(x2);
                            max_y = max_y.max(y2);
                            any = true;
                        }

                        if any {
                            (Some(0u64), Some((min_x, min_y, max_x, max_y)), true)
                        } else {
                            (None, None, false)
                        }
                    } else if let Some(selected_id) = state.canvas.selection_manager.selected_id() {
                        if let Some(obj) = state.canvas.shapes().find(|s| s.id == selected_id) {
                            let bounds = Self::selection_bounds(&obj.shape);
                            (Some(selected_id), Some(bounds), false)
                        } else {
                            (None, None, false)
                        }
                    } else {
                        (None, None, false)
                    }
                };

                if let (Some(selected_id), Some(bounds)) = (selected_id_opt, bounds_opt) {
                    if let Some(handle) =
                        self.get_resize_handle_at(canvas_x, canvas_y, &bounds, zoom)
                    {
                        // Start resizing
                        *self.active_resize_handle.borrow_mut() = Some((handle, selected_id));
                        let (min_x, min_y, max_x, max_y) = bounds;
                        *self.resize_original_bounds.borrow_mut() =
                            Some((min_x, min_y, max_x - min_x, max_y - min_y));

                        // Snapshot original shapes so resizing doesn't compound on each drag update.
                        // This matters for group resize and for path/text scaling.
                        let originals: Vec<(u64, Shape)> = {
                            let state = self.state.borrow();
                            state
                                .canvas
                                .shapes()
                                .filter(|s| s.selected)
                                .map(|s| (s.id, s.shape.clone()))
                                .collect()
                        };
                        *self.resize_original_shapes.borrow_mut() = Some(originals);

                        *self.creation_start.borrow_mut() = Some((canvas_x, canvas_y));
                        if is_group_resize {
                            // For group resize, we keep moving behavior the same but scale on drag updates.
                        }
                        return;
                    }
                }

                // Check if clicking on a selected shape for moving
                let has_selected = {
                    let state = self.state.borrow();
                    state.canvas.selection_manager.selected_id().is_some()
                };

                if has_selected {
                    // Start dragging selected shapes
                    *self.creation_start.borrow_mut() = Some((canvas_x, canvas_y));
                    *self.last_drag_offset.borrow_mut() = (0.0, 0.0); // Reset offset tracker
                } else {
                    // Start selection rectangle (future implementation)
                    *self.creation_start.borrow_mut() = Some((canvas_x, canvas_y));
                }
            }
            DesignerTool::Pan => {
                *self.creation_start.borrow_mut() = Some((x, y)); // Screen coords for pan
                *self.last_drag_offset.borrow_mut() = (0.0, 0.0); // Reset offset tracker (offsets start at 0)
                self.widget.set_cursor_from_name(Some("grabbing"));
            }
            _ => {
                // Start shape creation
                *self.creation_start.borrow_mut() = Some((canvas_x, canvas_y));
                *self.creation_current.borrow_mut() = Some((canvas_x, canvas_y));
            }
        }
    }

    fn handle_drag_update(&self, offset_x: f64, offset_y: f64) {
        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);
        let shift_pressed = *self.shift_pressed.borrow();

        // Get start point without holding the borrow
        let start_opt = *self.creation_start.borrow();

        if let Some(start) = start_opt {
            let state = self.state.borrow();
            let zoom = state.canvas.zoom();
            drop(state);

            // Convert offsets to canvas units
            let canvas_offset_x = offset_x / zoom;
            let canvas_offset_y = offset_y / zoom;

            // Update current position (offset is from drag start)
            let mut current_x = start.0 + canvas_offset_x;
            let mut current_y = start.1 - canvas_offset_y; // Flip Y offset

            // Apply Shift key constraints for creation
            if shift_pressed && tool != DesignerTool::Select {
                let dx = current_x - start.0;
                let dy = current_y - start.1;

                if tool == DesignerTool::Rectangle || tool == DesignerTool::Ellipse {
                    // Square/Circle constraint (1:1 aspect ratio)
                    let max_dim = dx.abs().max(dy.abs());
                    current_x = start.0 + max_dim * dx.signum();
                    current_y = start.1 + max_dim * dy.signum();
                } else if tool == DesignerTool::Line || tool == DesignerTool::Polyline {
                    // Snap to 45 degree increments
                    let angle = dy.atan2(dx);
                    let snap_angle = (angle / (std::f64::consts::PI / 4.0)).round()
                        * (std::f64::consts::PI / 4.0);
                    let dist = (dx * dx + dy * dy).sqrt();
                    current_x = start.0 + dist * snap_angle.cos();
                    current_y = start.1 + dist * snap_angle.sin();
                }
            }

            if tool != DesignerTool::Pan {
                (current_x, current_y) = self.snap_canvas_point(current_x, current_y);
                *self.creation_current.borrow_mut() = Some((current_x, current_y));
            }

            // If in select mode, handle resizing or moving
            if tool == DesignerTool::Select {
                // Check if we're resizing
                if let Some((handle, shape_id)) = *self.active_resize_handle.borrow() {
                    self.apply_resize(handle, shape_id, current_x, current_y, shift_pressed);
                } else {
                    let mut state = self.state.borrow_mut();
                    // Check if we have a selection - if so, move it; otherwise, marquee select
                    if state.canvas.selection_manager.selected_id().is_some() {
                        // Calculate delta from last update (incremental movement)
                        let last_offset = *self.last_drag_offset.borrow();
                        let mut delta_x = (offset_x - last_offset.0) / zoom;
                        let mut delta_y = (offset_y - last_offset.1) / zoom;

                        if shift_pressed {
                            // Constrain movement to X or Y axis based on total drag
                            let total_dx = current_x - start.0;
                            let total_dy = current_y - start.1;

                            if total_dx.abs() > total_dy.abs() {
                                delta_y = 0.0;
                            } else {
                                delta_x = 0.0;
                            }
                        }

                        // Apply incremental movement
                        state.canvas.move_selected(delta_x, -delta_y);

                        // Update last offset
                        *self.last_drag_offset.borrow_mut() = (offset_x, offset_y);
                    }
                    // Marquee selection is shown by the preview rectangle (handled in draw)
                }
            } else if tool == DesignerTool::Pan {
                // Handle panning
                // offset_x/y are total offsets from drag start.
                // We need incremental change from last frame.
                // But wait, handle_drag_update gives total offset from start.
                // In handle_drag_begin for Pan, I set last_drag_offset to (x, y) (start pos).
                // But offset_x/y here are relative to start.
                // So current screen pos = start + offset.
                // Previous screen pos = start + previous_offset.
                // Delta = current - previous.

                // Actually, let's use last_drag_offset to store the *previous offset value*.
                // In begin, offset is 0. So last_drag_offset = (0,0).

                let last_offset = *self.last_drag_offset.borrow();
                let delta_x = offset_x - last_offset.0;
                let delta_y = offset_y - last_offset.1;

                let mut state = self.state.borrow_mut();
                // Drag right (+x) -> move content right -> pan_x increases
                // Drag down (+y) -> move content down -> pan_y increases (because Y is flipped)
                // Wait, if I drag down, y increases.
                // Screen Y increases.
                // Content should move down on screen.
                // Content Y (world) is up.
                // Moving content down on screen means moving it to lower Y in world? No.
                // Screen Y = height - (world Y * zoom + pan Y).
                // If I want Screen Y to increase (move down), I can decrease pan Y?
                // height - (wy*z + (py - d)) = height - wy*z - py + d = old_sy + d.
                // So decreasing pan_y moves content down on screen.
                // So drag down (+dy) -> pan_y -= dy.

                // Let's verify with scrollbars.
                // Scrollbar down -> value increases.
                // v_adj value -> state.canvas.set_pan(px, val).
                // So pan_y increases.
                // If pan_y increases, Screen Y = height - (wy*z + py).
                // Screen Y decreases. Content moves UP.
                // So scrollbar down moves content UP. This is standard scrolling (view moves down).

                // Panning with hand: Drag UP -> content moves UP.
                // Drag UP means dy is negative.
                // If dy is negative, we want content to move UP (Screen Y decreases).
                // So pan_y should increase.
                // So pan_y -= dy (since dy is negative, -= is +=).

                // Drag DOWN means dy is positive.
                // We want content to move DOWN (Screen Y increases).
                // So pan_y should decrease.
                // So pan_y -= dy.

                // Drag RIGHT means dx is positive.
                // We want content to move RIGHT (Screen X increases).
                // Screen X = (wx * z + px).
                // So pan_x should increase.
                // So pan_x += dx.

                state.canvas.pan_by(delta_x, -delta_y);
                let new_pan_x = state.canvas.pan_x();
                let new_pan_y = state.canvas.pan_y();
                drop(state);

                // Update scrollbars
                if let Some(adj) = self.hadjustment.borrow().as_ref() {
                    adj.set_value(-new_pan_x);
                }
                if let Some(adj) = self.vadjustment.borrow().as_ref() {
                    adj.set_value(new_pan_y);
                }

                *self.last_drag_offset.borrow_mut() = (offset_x, offset_y);
            }

            self.widget.queue_draw();
        }
    }

    fn handle_drag_end(&self, offset_x: f64, offset_y: f64) {
        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);

        if tool == DesignerTool::Pan {
            *self.creation_start.borrow_mut() = None;
            *self.last_drag_offset.borrow_mut() = (0.0, 0.0);
            self.widget.set_cursor_from_name(Some("grab"));
            return;
        }

        // Get start point and release the borrow immediately
        let start_opt = *self.creation_start.borrow();

        if let Some(start) = start_opt {
            let state = self.state.borrow();
            let zoom = state.canvas.zoom();
            drop(state);

            let canvas_offset_x = offset_x / zoom;
            let canvas_offset_y = offset_y / zoom;

            let end_x = start.0 + canvas_offset_x;
            let end_y = start.1 - canvas_offset_y; // Flip Y offset

            match tool {
                DesignerTool::Select => {
                    // Clear resize state
                    *self.active_resize_handle.borrow_mut() = None;
                    *self.resize_original_bounds.borrow_mut() = None;
                    *self.resize_original_shapes.borrow_mut() = None;

                    // If we didn't have a selection and we dragged, perform marquee selection
                    let mut state = self.state.borrow_mut();
                    if state.canvas.selection_manager.selected_id().is_none() {
                        // Calculate selection rectangle
                        let min_x = start.0.min(end_x);
                        let max_x = start.0.max(end_x);
                        let min_y = start.1.min(end_y);
                        let max_y = start.1.max(end_y);

                        // Find all shapes intersecting the marquee rectangle
                        let selected_shapes: Vec<_> = state
                            .canvas
                            .shapes()
                            .filter(|obj| {
                                let (shape_min_x, shape_min_y, shape_max_x, shape_max_y) =
                                    obj.get_total_bounds();
                                // Check if bounding boxes intersect
                                !(shape_max_x < min_x
                                    || shape_min_x > max_x
                                    || shape_max_y < min_y
                                    || shape_min_y > max_y)
                            })
                            .map(|obj| obj.id)
                            .collect();

                        // Select the shapes
                        if !selected_shapes.is_empty() {
                            // Deselect all shapes first
                            for obj in state.canvas.shape_store.iter_mut() {
                                obj.selected = false;
                            }

                            // Then select the marquee-selected shapes
                            for &shape_id in &selected_shapes {
                                if let Some(shape) = state.canvas.shape_store.get_mut(shape_id) {
                                    shape.selected = true;
                                }
                            }

                            // Set primary selection to first selected shape
                            state
                                .canvas
                                .selection_manager
                                .set_selected_id(selected_shapes.first().copied());
                        }
                    }
                }
                _ => {
                    // Create the shape for drawing tools
                    self.create_shape(tool, start, (end_x, end_y));
                }
            }

            // Clear creation state (now safe - no borrows held)
            *self.creation_start.borrow_mut() = None;
            *self.creation_current.borrow_mut() = None;

            // Update properties panel after resize/move
            if let Some(ref props) = *self.properties.borrow() {
                props.update_from_selection();
            }

            // Update toolpaths if enabled
            let show_toolpaths = self.state.borrow().show_toolpaths;
            if show_toolpaths {
                self.generate_preview_toolpaths();
            }

            // Queue draw after clearing state
            self.widget.queue_draw();
        }
    }

    fn create_shape(&self, tool: DesignerTool, start: (f64, f64), end: (f64, f64)) {
        // Scope the borrow to release it before queue_draw
        {
            let mut state = self.state.borrow_mut();

            let shape = match tool {
                DesignerTool::Rectangle => {
                    let x = start.0.min(end.0);
                    let y = start.1.min(end.1);
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();

                    if width > 1.0 && height > 1.0 {
                        Some(Shape::Rectangle(Rectangle::new(x, y, width, height)))
                    } else {
                        None
                    }
                }
                DesignerTool::Circle => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let radius = width.min(height) / 2.0;

                    if radius > 1.0 {
                        Some(Shape::Circle(Circle::new(Point::new(cx, cy), radius)))
                    } else {
                        None
                    }
                }
                DesignerTool::Line => Some(Shape::Line(Line::new(
                    Point::new(start.0, start.1),
                    Point::new(end.0, end.1),
                ))),
                DesignerTool::Ellipse => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let rx = (end.0 - start.0).abs() / 2.0;
                    let ry = (end.1 - start.1).abs() / 2.0;

                    if rx > 1.0 && ry > 1.0 {
                        Some(Shape::Ellipse(Ellipse::new(Point::new(cx, cy), rx, ry)))
                    } else {
                        None
                    }
                }
                DesignerTool::Triangle => {
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;

                    if width > 1.0 && height > 1.0 {
                        Some(Shape::Triangle(Triangle::new(
                            Point::new(cx, cy),
                            width,
                            height,
                        )))
                    } else {
                        None
                    }
                }
                DesignerTool::Polygon => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let radius = width.min(height) / 2.0;

                    if radius > 1.0 {
                        Some(Shape::Polygon(Polygon::new(Point::new(cx, cy), radius, 6)))
                    } else {
                        None
                    }
                }
                DesignerTool::Gear => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let radius = width.min(height) / 2.0;

                    if radius > 1.0 {
                        // Default to module 2.0, 20 teeth
                        Some(Shape::Gear(gcodekit5_designer::model::DesignGear::new(
                            Point::new(cx, cy),
                            2.0,
                            20,
                        )))
                    } else {
                        None
                    }
                }
                DesignerTool::Sprocket => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let radius = width.min(height) / 2.0;

                    if radius > 1.0 {
                        // Default to 12.7mm pitch (ANSI 40), 15 teeth
                        Some(Shape::Sprocket(
                            gcodekit5_designer::model::DesignSprocket::new(
                                Point::new(cx, cy),
                                12.7,
                                15,
                            ),
                        ))
                    } else {
                        None
                    }
                }
                DesignerTool::TabbedBox => {
                    let x = start.0.min(end.0);
                    let y = start.1.min(end.1);
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();

                    if width > 1.0 && height > 1.0 {
                        // Default to 100x100x50, 3mm thickness, 10mm tabs
                        Some(Shape::TabbedBox(
                            gcodekit5_designer::model::DesignTabbedBox::new(
                                Point::new(x + width / 2.0, y + height / 2.0),
                                width,
                                height,
                                50.0,
                                3.0,
                                10.0,
                            ),
                        ))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(shape) = shape {
                state.canvas.add_shape(shape);
            }
        } // Drop the mutable borrow here

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }
    }

    pub fn delete_selected(&self) {
        let mut state = self.state.borrow_mut();
        let selected_ids: Vec<u64> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();

        for id in selected_ids {
            let cmd = DesignerCommand::RemoveShape(RemoveShape { id, object: None });
            state.push_command(cmd);
        }

        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        self.widget.queue_draw();
    }

    pub fn duplicate_selected(&self) {
        let mut state = self.state.borrow_mut();
        let selected: Vec<DrawingObject> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();

        if selected.is_empty() {
            return;
        }

        // Deselect all current shapes
        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        // Create duplicates with offset
        let offset = 10.0;
        let mut new_objects = Vec::new();
        for mut obj in selected {
            obj.id = state.canvas.generate_id();
            obj.shape.translate(offset, offset);
            obj.selected = true;
            new_objects.push(obj);
        }

        let cmd = DesignerCommand::PasteShapes(PasteShapes {
            ids: new_objects.iter().map(|o| o.id).collect(),
            objects: new_objects.into_iter().map(Some).collect(),
        });
        state.push_command(cmd);

        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn create_linear_array(&self, count: usize, dx: f64, dy: f64) {
        let mut state = self.state.borrow_mut();
        let selected: Vec<DrawingObject> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();

        if selected.is_empty() {
            return;
        }

        // Deselect original shapes
        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        let mut new_objects = Vec::new();

        // For each selected object, create count copies
        for i in 1..count {
            for obj in &selected {
                let mut new_obj = obj.clone();
                new_obj.id = state.canvas.generate_id();
                new_obj.shape.translate(dx * i as f64, dy * i as f64);
                new_obj.selected = true;
                new_objects.push(new_obj);
            }
        }

        // Re-select original items
        for obj in state.canvas.shapes_mut() {
            if selected.iter().any(|s| s.id == obj.id) {
                obj.selected = true;
            }
        }

        if !new_objects.is_empty() {
            let cmd = DesignerCommand::PasteShapes(PasteShapes {
                ids: new_objects.iter().map(|o| o.id).collect(),
                objects: new_objects.into_iter().map(Some).collect(),
            });
            state.push_command(cmd);
        }

        drop(state);

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn create_grid_array(&self, rows: usize, cols: usize, dx: f64, dy: f64) {
        let mut state = self.state.borrow_mut();
        let selected: Vec<DrawingObject> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();

        if selected.is_empty() {
            return;
        }

        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        let mut new_objects = Vec::new();

        for r in 0..rows {
            for c in 0..cols {
                if r == 0 && c == 0 {
                    continue;
                } // Skip original position

                for obj in &selected {
                    let mut new_obj = obj.clone();
                    new_obj.id = state.canvas.generate_id();
                    new_obj.shape.translate(dx * c as f64, dy * r as f64);
                    new_obj.selected = true;
                    new_objects.push(new_obj);
                }
            }
        }

        // Re-select original items
        for obj in state.canvas.shapes_mut() {
            if selected.iter().any(|s| s.id == obj.id) {
                obj.selected = true;
            }
        }

        if !new_objects.is_empty() {
            let cmd = DesignerCommand::PasteShapes(PasteShapes {
                ids: new_objects.iter().map(|o| o.id).collect(),
                objects: new_objects.into_iter().map(Some).collect(),
            });
            state.push_command(cmd);
        }

        drop(state);

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn create_circular_array(
        &self,
        count: usize,
        center_x: f64,
        center_y: f64,
        total_angle: f64,
    ) {
        let mut state = self.state.borrow_mut();
        let selected: Vec<DrawingObject> = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();

        if selected.is_empty() {
            return;
        }

        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        let mut new_objects = Vec::new();
        let angle_step = total_angle / count as f64;

        for i in 1..count {
            let angle = angle_step * i as f64;

            for obj in &selected {
                let mut new_obj = obj.clone();
                new_obj.id = state.canvas.generate_id();
                new_obj.shape.rotate(angle, center_x, center_y);
                new_obj.selected = true;
                new_objects.push(new_obj);
            }
        }

        // Re-select original items
        for obj in state.canvas.shapes_mut() {
            if selected.iter().any(|s| s.id == obj.id) {
                obj.selected = true;
            }
        }

        if !new_objects.is_empty() {
            let cmd = DesignerCommand::PasteShapes(PasteShapes {
                ids: new_objects.iter().map(|o| o.id).collect(),
                objects: new_objects.into_iter().map(Some).collect(),
            });
            state.push_command(cmd);
        }

        drop(state);

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn group_selected(&self) {
        let mut state = self.state.borrow_mut();
        state.group_selected();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn ungroup_selected(&self) {
        let mut state = self.state.borrow_mut();
        state.ungroup_selected();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn convert_to_path(&self) {
        let mut state = self.state.borrow_mut();
        state.convert_selected_to_path();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn convert_to_rectangle(&self) {
        let mut state = self.state.borrow_mut();
        state.convert_selected_to_rectangle();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn align_left(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_horizontal_left();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_right(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_horizontal_right();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_top(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_vertical_top();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_bottom(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_vertical_bottom();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_center_horizontal(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_horizontal_center();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn align_center_vertical(&self) {
        let mut state = self.state.borrow_mut();
        state.align_selected_vertical_center();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn boolean_union(&self) {
        let mut state = self.state.borrow_mut();
        state.perform_boolean_union();
        drop(state);

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }
        if let Some(ref props) = *self.properties.borrow() {
            props.update_from_selection();
        }

        self.widget.queue_draw();
    }

    pub fn boolean_difference(&self) {
        let mut state = self.state.borrow_mut();
        state.perform_boolean_difference();
        drop(state);

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }
        if let Some(ref props) = *self.properties.borrow() {
            props.update_from_selection();
        }

        self.widget.queue_draw();
    }

    pub fn boolean_intersection(&self) {
        let mut state = self.state.borrow_mut();
        state.perform_boolean_intersection();
        drop(state);

        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }
        if let Some(ref props) = *self.properties.borrow() {
            props.update_from_selection();
        }

        self.widget.queue_draw();
    }

    pub fn mirror_x(&self) {
        let mut state = self.state.borrow_mut();
        state.mirror_selected_x();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn mirror_y(&self) {
        let mut state = self.state.borrow_mut();
        state.mirror_selected_y();
        drop(state);
        self.widget.queue_draw();
    }

    pub fn copy_selected(&self) {
        let mut state = self.state.borrow_mut();
        state.clipboard = state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();
    }

    pub fn cut(&self) {
        self.copy_selected();
        self.delete_selected();
    }

    pub fn paste(&self) {
        let mut state = self.state.borrow_mut();
        if state.clipboard.is_empty() {
            return;
        }

        // Clone clipboard before using it
        let clipboard = state.clipboard.clone();

        // Deselect all current shapes
        for obj in state.canvas.shapes_mut() {
            obj.selected = false;
        }

        // Create copies with offset
        let offset = 10.0;
        let mut new_objects = Vec::new();
        for obj in &clipboard {
            let mut new_obj = obj.clone();
            new_obj.id = state.canvas.generate_id();
            new_obj.shape.translate(offset, offset);
            new_obj.selected = true;
            new_objects.push(new_obj);
        }

        let cmd = DesignerCommand::PasteShapes(PasteShapes {
            ids: new_objects.iter().map(|o| o.id).collect(),
            objects: new_objects.into_iter().map(Some).collect(),
        });
        state.push_command(cmd);

        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn undo(&self) {
        let mut state = self.state.borrow_mut();
        state.undo();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn redo(&self) {
        let mut state = self.state.borrow_mut();
        state.redo();
        drop(state);

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }

        // Update toolpaths if enabled
        let show_toolpaths = self.state.borrow().show_toolpaths;
        if show_toolpaths {
            self.generate_preview_toolpaths();
        }

        self.widget.queue_draw();
    }

    pub fn generate_preview_toolpaths(&self) {
        if self.preview_generating.get() {
            self.preview_pending.set(true);
            self.preview_cancel.store(true, Ordering::SeqCst);
            return;
        }

        self.preview_generating.set(true);
        self.preview_cancel.store(false, Ordering::SeqCst);

        let started_at = std::time::Instant::now();

        let (shapes, feed_rate, spindle_speed, tool_diameter, cut_depth) = {
            let state = self.state.borrow();
            (
                state.canvas.shapes().cloned().collect::<Vec<_>>(),
                state.tool_settings.feed_rate,
                state.tool_settings.spindle_speed,
                state.tool_settings.tool_diameter,
                state.tool_settings.cut_depth,
            )
        };

        let total_shapes = shapes.len().max(1);
        let done_shapes: Arc<std::sync::atomic::AtomicUsize> =
            Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // Global status bar progress + cancel (non-blocking)
        if let Some(sb) = self.status_bar.as_ref() {
            let cancel_flag = self.preview_cancel.clone();
            let generating = self.preview_generating.clone();
            sb.set_progress(0.1, "0s", "");
            sb.set_cancel_action(Some(std::boxed::Box::new(move || {
                cancel_flag.store(true, Ordering::SeqCst);
                generating.set(false);
            })));
        }

        let cancel = self.preview_cancel.clone();
        let done_shapes_thread = done_shapes.clone();
        let result_arc: Arc<std::sync::Mutex<Option<Vec<Toolpath>>>> =
            Arc::new(std::sync::Mutex::new(None));
        let result_arc_thread = result_arc.clone();

        std::thread::spawn(move || {
            use gcodekit5_designer::toolpath::ToolpathGenerator;
            let mut gen = ToolpathGenerator::new();
            gen.set_feed_rate(feed_rate);
            gen.set_spindle_speed(spindle_speed);
            gen.set_tool_diameter(tool_diameter);
            gen.set_cut_depth(cut_depth);
            gen.set_step_in(tool_diameter * 0.4);

            let mut toolpaths = Vec::new();
            for shape in shapes {
                if cancel.load(Ordering::SeqCst) {
                    return;
                }

                gen.set_pocket_strategy(shape.pocket_strategy);
                gen.set_start_depth(shape.start_depth);
                gen.set_cut_depth(shape.pocket_depth);
                gen.set_step_in(shape.step_in as f64);
                gen.set_raster_fill_ratio(shape.raster_fill_ratio);

                let effective_shape = shape.get_effective_shape();
                let shape_toolpaths = match &effective_shape {
                    Shape::Rectangle(rect) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_rectangle_pocket(
                                rect,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_rectangle_contour(rect, shape.step_down as f64)
                        }
                    }
                    Shape::Circle(circle) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_circle_pocket(
                                circle,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_circle_contour(circle, shape.step_down as f64)
                        }
                    }
                    Shape::Line(line) => gen.generate_line_contour(line, shape.step_down as f64),
                    Shape::Ellipse(ellipse) => {
                        let (x1, y1, x2, y2) = ellipse.bounds();
                        let cx = (x1 + x2) / 2.0;
                        let cy = (y1 + y2) / 2.0;
                        let radius = ((x2 - x1).abs().max((y2 - y1).abs())) / 2.0;
                        let circle = Circle::new(Point::new(cx, cy), radius);
                        gen.generate_circle_contour(&circle, shape.step_down as f64)
                    }
                    Shape::Path(path_shape) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_path_pocket(
                                path_shape,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_path_contour(path_shape, shape.step_down as f64)
                        }
                    }
                    Shape::Text(text) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_text_pocket_toolpath(text, shape.step_down as f64)
                        } else {
                            gen.generate_text_toolpath(text, shape.step_down as f64)
                        }
                    }
                    Shape::Triangle(triangle) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_triangle_pocket(
                                triangle,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_triangle_contour(triangle, shape.step_down as f64)
                        }
                    }
                    Shape::Polygon(polygon) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_polygon_pocket(
                                polygon,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_polygon_contour(polygon, shape.step_down as f64)
                        }
                    }
                    Shape::Gear(gear) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_gear_pocket(
                                gear,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_gear_contour(gear, shape.step_down as f64)
                        }
                    }
                    Shape::Sprocket(sprocket) => {
                        if shape.operation_type == OperationType::Pocket {
                            gen.generate_sprocket_pocket(
                                sprocket,
                                shape.pocket_depth,
                                shape.step_down as f64,
                                shape.step_in as f64,
                            )
                        } else {
                            gen.generate_sprocket_contour(sprocket, shape.step_down as f64)
                        }
                    }
                    Shape::TabbedBox(tbox) => {
                        gen.generate_tabbed_box_contour(tbox, shape.step_down as f64)
                    }
                };
                toolpaths.extend(shape_toolpaths);
                done_shapes_thread.fetch_add(1, Ordering::Relaxed);
            }

            *result_arc_thread.lock().unwrap() = Some(toolpaths);
        });

        // Poll for completion (non-blocking)
        let poll_count = Rc::new(RefCell::new(0u32));
        let poll_count_clone = poll_count.clone();
        let result_arc_poll = result_arc.clone();
        let canvas = self.widget.clone();
        let out = self.preview_toolpaths.clone();
        let generating = self.preview_generating.clone();
        let pending = self.preview_pending.clone();
        let cancel_poll = self.preview_cancel.clone();
        let done_shapes_poll = done_shapes.clone();
        let sb_poll = self.status_bar.clone();
        let self_ref = self.clone();

        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            *poll_count_clone.borrow_mut() += 1;

            if let Some(sb) = sb_poll.as_ref() {
                let done = done_shapes_poll.load(Ordering::Relaxed).min(total_shapes);
                let pct = (done as f64 / total_shapes as f64) * 100.0;
                let elapsed = started_at.elapsed().as_secs_f64();
                sb.set_progress(pct.max(0.1), &format!("{:.0}s", elapsed), "");
            }

            if cancel_poll.load(Ordering::SeqCst) {
                generating.set(false);
                if let Some(sb) = sb_poll.as_ref() {
                    sb.set_progress(0.0, "", "");
                    sb.set_cancel_action(None);
                }
                if pending.replace(false) {
                    self_ref.generate_preview_toolpaths();
                }
                return gtk4::glib::ControlFlow::Break;
            }

            if *poll_count_clone.borrow() > 400 {
                generating.set(false);
                if let Some(sb) = sb_poll.as_ref() {
                    sb.set_progress(0.0, "", "");
                    sb.set_cancel_action(None);
                }
                return gtk4::glib::ControlFlow::Break;
            }

            if let Ok(mut guard) = result_arc_poll.try_lock() {
                if let Some(tp) = guard.take() {
                    if !cancel_poll.load(Ordering::SeqCst) {
                        *out.borrow_mut() = tp;
                        canvas.queue_draw();
                    }

                    generating.set(false);
                    if let Some(sb) = sb_poll.as_ref() {
                        sb.set_progress(0.0, "", "");
                        sb.set_cancel_action(None);
                    }
                    if pending.replace(false) {
                        self_ref.generate_preview_toolpaths();
                    }
                    return gtk4::glib::ControlFlow::Break;
                }
            }

            gtk4::glib::ControlFlow::Continue
        });
    }

    fn draw(
        cr: &gtk4::cairo::Context,
        state: &DesignerState,
        width: f64,
        height: f64,
        mouse_pos: (f64, f64),
        preview_start: Option<(f64, f64)>,
        preview_current: Option<(f64, f64)>,
        polyline_points: &[Point],
        preview_shapes: &[Shape],
        toolpaths: &[Toolpath],
        device_bounds: (f64, f64, f64, f64),
        style_context: &gtk4::StyleContext,
    ) {
        // Background handled by CSS

        let fg_color = style_context.color();
        let accent_color = style_context
            .lookup_color("accent_color")
            .unwrap_or(gtk4::gdk::RGBA::new(0.0, 0.5, 1.0, 1.0));
        let success_color = style_context
            .lookup_color("success_color")
            .unwrap_or(gtk4::gdk::RGBA::new(0.0, 0.8, 0.0, 1.0));
        let warning_color = style_context
            .lookup_color("warning_color")
            .unwrap_or(gtk4::gdk::RGBA::new(1.0, 1.0, 0.0, 1.0));
        let error_color = style_context
            .lookup_color("error_color")
            .unwrap_or(gtk4::gdk::RGBA::new(1.0, 0.0, 0.0, 1.0));

        // Setup coordinate system
        // Designer uses Y-up (Cartesian), Cairo uses Y-down

        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();

        // Transform to bottom-left, flip Y, then apply pan and zoom
        // Origin is bottom-left of the widget
        cr.translate(0.0, height);
        cr.scale(1.0, -1.0);

        // Apply Pan (in screen pixels, but Y is flipped so +Y pan moves up)
        cr.translate(pan_x, pan_y);

        // Apply Zoom
        cr.scale(zoom, zoom);

        // Draw Grid
        if state.show_grid {
            Self::draw_grid(cr, width, height, state.grid_spacing_mm.max(0.1), &fg_color);
        }

        // Draw Device Bounds
        let (min_x, min_y, max_x, max_y) = device_bounds;
        let width = max_x - min_x;
        let height = max_y - min_y;

        cr.save().unwrap();
        cr.set_source_rgb(0.0, 0.0, 1.0); // Blue
        cr.set_line_width(2.0 / zoom); // 2px wide on screen
        cr.rectangle(min_x, min_y, width, height);
        cr.stroke().unwrap();
        cr.restore().unwrap();

        // Draw Origin Crosshair
        Self::draw_origin_crosshair(cr, zoom);

        // Draw Toolpaths (if enabled)
        if state.show_toolpaths {
            cr.save().unwrap();
            cr.set_line_width(2.0 / zoom); // Constant screen width

            for toolpath in toolpaths {
                for segment in &toolpath.segments {
                    match segment.segment_type {
                        ToolpathSegmentType::RapidMove => {
                            cr.set_source_rgba(
                                warning_color.red() as f64,
                                warning_color.green() as f64,
                                warning_color.blue() as f64,
                                0.5,
                            );
                            cr.set_dash(&[2.0 / zoom, 2.0 / zoom], 0.0);
                            cr.move_to(segment.start.x, segment.start.y);
                            cr.line_to(segment.end.x, segment.end.y);
                            cr.stroke().unwrap();
                        }
                        ToolpathSegmentType::LinearMove => {
                            cr.set_source_rgba(
                                success_color.red() as f64,
                                success_color.green() as f64,
                                success_color.blue() as f64,
                                0.7,
                            );
                            cr.set_dash(&[], 0.0);
                            cr.move_to(segment.start.x, segment.start.y);
                            cr.line_to(segment.end.x, segment.end.y);
                            cr.stroke().unwrap();
                        }
                        ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                            cr.set_source_rgba(
                                success_color.red() as f64,
                                success_color.green() as f64,
                                success_color.blue() as f64,
                                0.7,
                            );
                            cr.set_dash(&[], 0.0);

                            if let Some(center) = segment.center {
                                let radius = center.distance_to(&segment.start);
                                let angle1 =
                                    (segment.start.y - center.y).atan2(segment.start.x - center.x);
                                let angle2 =
                                    (segment.end.y - center.y).atan2(segment.end.x - center.x);

                                cr.move_to(segment.start.x, segment.start.y); // Ensure we start at correct point
                                                                              // Note: Cairo adds a line from current point to start of arc if they differ.
                                                                              // But we just moved there.

                                if segment.segment_type == ToolpathSegmentType::ArcCW {
                                    cr.arc_negative(center.x, center.y, radius, angle1, angle2);
                                } else {
                                    cr.arc(center.x, center.y, radius, angle1, angle2);
                                }
                            } else {
                                cr.move_to(segment.start.x, segment.start.y);
                                cr.line_to(segment.end.x, segment.end.y);
                            }
                            cr.stroke().unwrap();
                        }
                    }
                }
            }

            cr.restore().unwrap();
        }

        // Draw polyline in progress
        if !polyline_points.is_empty() {
            cr.save().unwrap();
            cr.set_source_rgba(
                accent_color.red() as f64,
                accent_color.green() as f64,
                accent_color.blue() as f64,
                1.0,
            );
            cr.set_line_width(2.0 / zoom);

            // Draw existing segments
            if let Some(first) = polyline_points.first() {
                cr.move_to(first.x, first.y);
                for p in polyline_points.iter().skip(1) {
                    cr.line_to(p.x, p.y);
                }

                // Draw rubber band to mouse
                cr.line_to(mouse_pos.0, mouse_pos.1);
            }

            cr.stroke().unwrap();

            // Draw points
            for p in polyline_points {
                cr.arc(p.x, p.y, 3.0 / zoom, 0.0, 2.0 * std::f64::consts::PI);
                cr.fill().unwrap();
            }

            cr.restore().unwrap();
        }

        let selected_count = state
            .canvas
            .shape_store
            .iter()
            .filter(|o| o.selected)
            .count();

        // Draw Shapes
        for obj in state.canvas.shape_store.iter() {
            // 1. Draw Base Shape
            cr.save().unwrap();

            if obj.selected {
                cr.set_source_rgba(
                    error_color.red() as f64,
                    error_color.green() as f64,
                    error_color.blue() as f64,
                    1.0,
                );
                cr.set_line_width(3.0 / zoom);
            } else if obj.group_id.is_some() {
                cr.set_source_rgba(
                    success_color.red() as f64,
                    success_color.green() as f64,
                    success_color.blue() as f64,
                    1.0,
                );
                cr.set_line_width(2.0 / zoom);
            } else {
                cr.set_source_rgba(
                    fg_color.red() as f64,
                    fg_color.green() as f64,
                    fg_color.blue() as f64,
                    fg_color.alpha() as f64,
                );
                cr.set_line_width(2.0 / zoom);
            }

            Self::draw_shape_geometry(cr, &obj.shape);

            // Draw resize handles on BASE shape
            if selected_count <= 1 && obj.selected {
                let bounds = Self::selection_bounds(&obj.shape);
                Self::draw_resize_handles(cr, &bounds, zoom, &accent_color);
            }

            cr.restore().unwrap();

            // 2. Draw Effective Shape (Yellow Overlay) if modified
            if obj.offset.abs() > 1e-6 || obj.fillet.abs() > 1e-6 || obj.chamfer.abs() > 1e-6 {
                cr.save().unwrap();
                cr.set_source_rgba(
                    warning_color.red() as f64,
                    warning_color.green() as f64,
                    warning_color.blue() as f64,
                    1.0,
                );
                cr.set_line_width(2.0 / zoom);
                Self::draw_shape_geometry(cr, &obj.get_effective_shape());
                cr.restore().unwrap();
            }
        }

        // Draw Preview Shapes (e.g. for offset/fillet) in yellow
        for shape in preview_shapes {
            cr.save().unwrap();
            cr.set_source_rgba(
                warning_color.red() as f64,
                warning_color.green() as f64,
                warning_color.blue() as f64,
                1.0,
            );
            cr.set_line_width(2.0 / zoom);
            Self::draw_shape_geometry(cr, shape);
            cr.restore().unwrap();
        }

        if selected_count > 1 {
            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;

            for obj in state.canvas.shape_store.iter().filter(|o| o.selected) {
                let (x1, y1, x2, y2) = Self::selection_bounds(&obj.shape);
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
            }

            if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
                let bounds = (min_x, min_y, max_x, max_y);
                Self::draw_resize_handles(cr, &bounds, zoom, &accent_color);
            }
        }

        // Draw preview marquee if creating a shape
        if let (Some(start), Some(current)) = (preview_start, preview_current) {
            cr.save().unwrap();

            // Draw dashed preview outline
            cr.set_source_rgba(
                accent_color.red() as f64,
                accent_color.green() as f64,
                accent_color.blue() as f64,
                0.7,
            );
            cr.set_line_width(2.0 / zoom);
            cr.set_dash(&[5.0 / zoom, 5.0 / zoom], 0.0); // Dashed line

            // Draw bounding box for the preview
            let x1 = start.0.min(current.0);
            let y1 = start.1.min(current.1);
            let x2 = start.0.max(current.0);
            let y2 = start.1.max(current.1);

            cr.rectangle(x1, y1, x2 - x1, y2 - y1);
            cr.stroke().unwrap();

            cr.restore().unwrap();
        }
    }

    fn draw_grid(
        cr: &gtk4::cairo::Context,
        width: f64,
        height: f64,
        grid_spacing: f64,
        fg_color: &gtk4::gdk::RGBA,
    ) {
        cr.save().unwrap();

        let minor_spacing = grid_spacing / 5.0;

        // Get current transform to find canvas bounds
        let matrix = cr.matrix();
        let x0 = -matrix.x0() / matrix.xx();
        let x1 = (width - matrix.x0()) / matrix.xx();
        let y0 = -matrix.y0() / matrix.yy();
        let y1 = (height - matrix.y0()) / matrix.yy();

        // Minor grid lines (lighter)
        cr.set_source_rgba(
            fg_color.red() as f64,
            fg_color.green() as f64,
            fg_color.blue() as f64,
            0.2,
        );
        cr.set_line_width(0.5);

        // Vertical minor grid lines
        let start_x = (x0 / minor_spacing).floor() * minor_spacing;
        let mut x = start_x;
        while x <= x1 {
            if ((x / grid_spacing).round() - x / grid_spacing).abs() > 0.01 {
                cr.move_to(x, y1);
                cr.line_to(x, y0);
                cr.stroke().unwrap();
            }
            x += minor_spacing;
        }

        // Horizontal minor grid lines
        let start_y = (y1 / minor_spacing).floor() * minor_spacing;
        let mut y = start_y;
        while y <= y0 {
            if ((y / grid_spacing).round() - y / grid_spacing).abs() > 0.01 {
                cr.move_to(x0, y);
                cr.line_to(x1, y);
                cr.stroke().unwrap();
            }
            y += minor_spacing;
        }

        // Major grid lines (darker)
        cr.set_source_rgba(
            fg_color.red() as f64,
            fg_color.green() as f64,
            fg_color.blue() as f64,
            0.4,
        );
        cr.set_line_width(1.0);

        // Vertical major grid lines
        x = (x0 / grid_spacing).floor() * grid_spacing;
        while x <= x1 {
            cr.move_to(x, y1);
            cr.line_to(x, y0);
            cr.stroke().unwrap();
            x += grid_spacing;
        }

        // Horizontal major grid lines
        y = (y1 / grid_spacing).floor() * grid_spacing;
        while y <= y0 {
            cr.move_to(x0, y);
            cr.line_to(x1, y);
            cr.stroke().unwrap();
            y += grid_spacing;
        }

        // Draw axes (thicker, darker) - only if they're visible
        cr.set_source_rgba(
            fg_color.red() as f64,
            fg_color.green() as f64,
            fg_color.blue() as f64,
            0.8,
        );
        cr.set_line_width(2.0);

        // X-axis (y=0)
        if y1 <= 0.0 && y0 >= 0.0 {
            cr.move_to(x0, 0.0);
            cr.line_to(x1, 0.0);
        }

        // Y-axis (x=0)
        if x0 <= 0.0 && x1 >= 0.0 {
            cr.move_to(0.0, y1);
            cr.line_to(0.0, y0);
        }
        cr.stroke().unwrap();

        cr.restore().unwrap();
    }

    fn selection_bounds(shape: &Shape) -> (f64, f64, f64, f64) {
        fn rotate_xy(x: f64, y: f64, cx: f64, cy: f64, angle: f64) -> (f64, f64) {
            let s = angle.sin();
            let c = angle.cos();
            let dx = x - cx;
            let dy = y - cy;
            (cx + dx * c - dy * s, cy + dx * s + dy * c)
        }

        match shape {
            Shape::Rectangle(rect) => {
                if rect.rotation.abs() <= 1e-9 {
                    return rect.bounds();
                }

                let cx = rect.center.x;
                let cy = rect.center.y;
                let hw = rect.width / 2.0;
                let hh = rect.height / 2.0;
                let corners = [
                    (cx - hw, cy - hh),
                    (cx + hw, cy - hh),
                    (cx + hw, cy + hh),
                    (cx - hw, cy + hh),
                ];

                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;

                for (x, y) in corners {
                    let (rx, ry) = rotate_xy(x, y, cx, cy, rect.rotation);
                    min_x = min_x.min(rx);
                    min_y = min_y.min(ry);
                    max_x = max_x.max(rx);
                    max_y = max_y.max(ry);
                }

                (min_x, min_y, max_x, max_y)
            }
            Shape::Circle(circle) => circle.bounds(),
            Shape::Line(line) => line.bounds(),
            Shape::Ellipse(ellipse) => {
                if ellipse.rotation.abs() <= 1e-9 {
                    return ellipse.bounds();
                }

                // Axis-aligned bounding box of a rotated ellipse.
                let theta = ellipse.rotation;
                let cos_t = theta.cos();
                let sin_t = theta.sin();
                let half_w = ((ellipse.rx * cos_t).powi(2) + (ellipse.ry * sin_t).powi(2)).sqrt();
                let half_h = ((ellipse.rx * sin_t).powi(2) + (ellipse.ry * cos_t).powi(2)).sqrt();

                (
                    ellipse.center.x - half_w,
                    ellipse.center.y - half_h,
                    ellipse.center.x + half_w,
                    ellipse.center.y + half_h,
                )
            }
            Shape::Path(path_shape) => {
                if path_shape.rotation.abs() <= 1e-9 {
                    return path_shape.bounds();
                }

                // Match the draw behavior: rotate about the path's AABB center.
                let (x1, y1, x2, y2) = path_shape.bounds();
                let cx = (x1 + x2) / 2.0;
                let cy = (y1 + y2) / 2.0;
                let corners = [(x1, y1), (x2, y1), (x2, y2), (x1, y2)];

                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;

                for (x, y) in corners {
                    let (rx, ry) = rotate_xy(x, y, cx, cy, path_shape.rotation);
                    min_x = min_x.min(rx);
                    min_y = min_y.min(ry);
                    max_x = max_x.max(rx);
                    max_y = max_y.max(ry);
                }

                (min_x, min_y, max_x, max_y)
            }
            Shape::Text(text) => text.bounds(),
            Shape::Triangle(triangle) => triangle.bounds(),
            Shape::Polygon(polygon) => polygon.bounds(),
            Shape::Gear(gear) => gear.bounds(),
            Shape::Sprocket(sprocket) => sprocket.bounds(),
            Shape::TabbedBox(tbox) => tbox.bounds(),
        }
    }

    fn draw_shape_geometry(cr: &gtk4::cairo::Context, shape: &Shape) {
        match shape {
            Shape::Rectangle(rect) => {
                cr.save().unwrap();
                cr.translate(rect.center.x, rect.center.y);
                if rect.rotation.abs() > 1e-9 {
                    cr.rotate(rect.rotation);
                }

                if rect.corner_radius > 0.001 {
                    let x = -rect.width / 2.0;
                    let y = -rect.height / 2.0;
                    let w = rect.width;
                    let h = rect.height;
                    let r = rect.corner_radius.min(w / 2.0).min(h / 2.0);
                    let pi = std::f64::consts::PI;

                    cr.new_sub_path();
                    // Start at right edge, bottom of TR corner
                    cr.arc(x + w - r, y + h - r, r, 0.0, 0.5 * pi); // TR
                    cr.arc(x + r, y + h - r, r, 0.5 * pi, pi); // TL
                    cr.arc(x + r, y + r, r, pi, 1.5 * pi); // BL
                    cr.arc(x + w - r, y + r, r, 1.5 * pi, 2.0 * pi); // BR
                    cr.close_path();
                    cr.stroke().unwrap();
                } else {
                    let x = -rect.width / 2.0;
                    let y = -rect.height / 2.0;
                    cr.rectangle(x, y, rect.width, rect.height);
                    cr.stroke().unwrap();
                }

                cr.restore().unwrap();
            }
            Shape::Circle(circle) => {
                cr.arc(
                    circle.center.x,
                    circle.center.y,
                    circle.radius,
                    0.0,
                    2.0 * std::f64::consts::PI,
                );
                cr.stroke().unwrap();
            }
            Shape::Line(line) => {
                cr.move_to(line.start.x, line.start.y);
                cr.line_to(line.end.x, line.end.y);
                cr.stroke().unwrap();
            }
            Shape::Ellipse(ellipse) => {
                cr.save().unwrap();
                cr.translate(ellipse.center.x, ellipse.center.y);
                if ellipse.rotation.abs() > 1e-9 {
                    cr.rotate(ellipse.rotation);
                }
                let base_width = cr.line_width();
                let scale_factor = ellipse.rx.abs().max(ellipse.ry.abs()).max(1e-6);
                cr.set_line_width(base_width / scale_factor);
                cr.scale(ellipse.rx, ellipse.ry);
                cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * std::f64::consts::PI);
                cr.stroke().unwrap();
                cr.restore().unwrap();
            }
            Shape::Path(path_shape) => {
                let (x1, y1, x2, y2) = path_shape.bounds();
                let cx = (x1 + x2) / 2.0;
                let cy = (y1 + y2) / 2.0;

                cr.save().unwrap();
                if path_shape.rotation.abs() > 1e-9 {
                    cr.translate(cx, cy);
                    cr.rotate(path_shape.rotation);
                    cr.translate(-cx, -cy);
                }

                cr.new_path();
                // Iterate lyon path
                for event in path_shape.render().iter() {
                    match event {
                        lyon::path::Event::Begin { at } => {
                            cr.move_to(at.x as f64, at.y as f64);
                        }
                        lyon::path::Event::Line { from: _, to } => {
                            cr.line_to(to.x as f64, to.y as f64);
                        }
                        lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                            // Cairo doesn't have quadratic, convert to cubic.
                            // We use current point as 'from'.
                            let (x0, y0) = cr.current_point().unwrap();
                            let x1 = x0 + (2.0 / 3.0) * (ctrl.x as f64 - x0);
                            let y1 = y0 + (2.0 / 3.0) * (ctrl.y as f64 - y0);
                            let x2 = to.x as f64 + (2.0 / 3.0) * (ctrl.x as f64 - to.x as f64);
                            let y2 = to.y as f64 + (2.0 / 3.0) * (ctrl.y as f64 - to.y as f64);
                            cr.curve_to(x1, y1, x2, y2, to.x as f64, to.y as f64);
                        }
                        lyon::path::Event::Cubic {
                            from: _,
                            ctrl1,
                            ctrl2,
                            to,
                        } => {
                            cr.curve_to(
                                ctrl1.x as f64,
                                ctrl1.y as f64,
                                ctrl2.x as f64,
                                ctrl2.y as f64,
                                to.x as f64,
                                to.y as f64,
                            );
                        }
                        lyon::path::Event::End {
                            last: _,
                            first: _,
                            close,
                        } => {
                            if close {
                                cr.close_path();
                            }
                        }
                    }
                }
                cr.stroke().unwrap();
                cr.restore().unwrap();
            }
            Shape::Text(text) => {
                // Basic text placeholder
                cr.save().unwrap();
                // Rotate around text bounds center, then flip Y for text rendering.
                let (x1, y1, x2, y2) = text.bounds();
                let cx = (x1 + x2) / 2.0;
                let cy = (y1 + y2) / 2.0;
                // Use negative angle because we flip Y after rotation.
                let angle = -text.rotation.to_radians();

                cr.translate(cx, cy);
                cr.rotate(angle);
                cr.translate(-cx, -cy);

                // Flip Y back for text so it's not upside down
                cr.translate(text.x, text.y);
                cr.scale(1.0, -1.0);
                let slant = if text.italic {
                    gtk4::cairo::FontSlant::Italic
                } else {
                    gtk4::cairo::FontSlant::Normal
                };
                let weight = if text.bold {
                    gtk4::cairo::FontWeight::Bold
                } else {
                    gtk4::cairo::FontWeight::Normal
                };
                cr.select_font_face(&text.font_family, slant, weight);
                cr.set_font_size(text.font_size);
                cr.show_text(&text.text).unwrap();
                cr.restore().unwrap();
            }
            Shape::Triangle(triangle) => {
                let path = triangle.render();
                cr.new_path();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => cr.move_to(at.x as f64, at.y as f64),
                        lyon::path::Event::Line { to, .. } => cr.line_to(to.x as f64, to.y as f64),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                cr.close_path();
                            }
                        }
                        _ => {}
                    }
                }
                cr.stroke().unwrap();
            }
            Shape::Polygon(polygon) => {
                let path = polygon.render();
                cr.new_path();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => cr.move_to(at.x as f64, at.y as f64),
                        lyon::path::Event::Line { to, .. } => cr.line_to(to.x as f64, to.y as f64),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                cr.close_path();
                            }
                        }
                        _ => {}
                    }
                }
                cr.stroke().unwrap();
            }
            Shape::Gear(gear) => {
                let path = gear.render();
                cr.new_path();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => cr.move_to(at.x as f64, at.y as f64),
                        lyon::path::Event::Line { to, .. } => cr.line_to(to.x as f64, to.y as f64),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                cr.close_path();
                            }
                        }
                        _ => {}
                    }
                }
                cr.stroke().unwrap();
            }
            Shape::Sprocket(sprocket) => {
                let path = sprocket.render();
                cr.new_path();
                for event in path.iter() {
                    match event {
                        lyon::path::Event::Begin { at } => cr.move_to(at.x as f64, at.y as f64),
                        lyon::path::Event::Line { to, .. } => cr.line_to(to.x as f64, to.y as f64),
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                cr.close_path();
                            }
                        }
                        _ => {}
                    }
                }
                cr.stroke().unwrap();
            }
            Shape::TabbedBox(tabbed_box) => {
                let paths = tabbed_box.render_all();
                for path in paths {
                    cr.new_path();
                    for event in path.iter() {
                        match event {
                            lyon::path::Event::Begin { at } => cr.move_to(at.x as f64, at.y as f64),
                            lyon::path::Event::Line { to, .. } => {
                                cr.line_to(to.x as f64, to.y as f64)
                            }
                            lyon::path::Event::End { close, .. } => {
                                if close {
                                    cr.close_path();
                                }
                            }
                            _ => {}
                        }
                    }
                    cr.stroke().unwrap();
                }
            }
        }
    }

    fn draw_origin_crosshair(cr: &gtk4::cairo::Context, zoom: f64) {
        cr.save().unwrap();

        // Draw Origin Axes (Full World Extent)
        let extent = core_constants::WORLD_EXTENT_MM as f64;
        cr.set_line_width(1.0 / zoom); // Thinner line for full axes

        // X Axis Red
        cr.set_source_rgb(1.0, 0.0, 0.0);
        cr.move_to(-extent, 0.0);
        cr.line_to(extent, 0.0);
        cr.stroke().unwrap();

        // Y Axis Green
        cr.set_source_rgb(0.0, 1.0, 0.0);
        cr.move_to(0.0, -extent);
        cr.line_to(0.0, extent);
        cr.stroke().unwrap();

        cr.restore().unwrap();
    }

    fn get_resize_handle_at(
        &self,
        x: f64,
        y: f64,
        bounds: &(f64, f64, f64, f64),
        zoom: f64,
    ) -> Option<ResizeHandle> {
        // Handles are drawn as ~8 screen pixels; in canvas units that's 8/zoom.
        let zoom = zoom.max(1e-6);
        let handle_size = 8.0 / zoom;
        let handle_tolerance = handle_size / 2.0;

        let (min_x, min_y, max_x, max_y) = *bounds;

        let corners = [
            (min_x, max_y, ResizeHandle::TopLeft), // Top-left (Y-up coords)
            (max_x, max_y, ResizeHandle::TopRight), // Top-right
            (min_x, min_y, ResizeHandle::BottomLeft), // Bottom-left
            (max_x, min_y, ResizeHandle::BottomRight), // Bottom-right
        ];

        for (cx, cy, handle) in corners {
            let dx = x - cx;
            let dy = y - cy;
            if dx.abs() <= handle_tolerance && dy.abs() <= handle_tolerance {
                return Some(handle);
            }
        }

        None
    }

    fn apply_resize(
        &self,
        handle: ResizeHandle,
        _shape_id: u64,
        current_x: f64,
        current_y: f64,
        shift_pressed: bool,
    ) {
        let orig_bounds = match *self.resize_original_bounds.borrow() {
            Some(b) => b,
            None => return,
        };

        let start = match *self.creation_start.borrow() {
            Some(s) => s,
            None => return,
        };

        let (orig_x, orig_y, orig_width, orig_height) = orig_bounds;

        // Calculate deltas
        let mut dx = current_x - start.0;
        let mut dy = current_y - start.1;

        if shift_pressed {
            // Maintain aspect ratio
            let ratio = if orig_height.abs() > 0.001 {
                orig_width / orig_height
            } else {
                1.0
            };

            // Calculate "natural" new dimensions based on mouse position
            let natural_w = match handle {
                ResizeHandle::TopLeft | ResizeHandle::BottomLeft => orig_width - dx,
                ResizeHandle::TopRight | ResizeHandle::BottomRight => orig_width + dx,
            };

            let natural_h = match handle {
                ResizeHandle::TopLeft | ResizeHandle::TopRight => orig_height + dy,
                ResizeHandle::BottomLeft | ResizeHandle::BottomRight => orig_height - dy,
            };

            // Determine which dimension to follow (use the one with larger relative change)
            let w_scale = (natural_w / orig_width).abs();
            let h_scale = (natural_h / orig_height).abs();

            let (new_w, new_h) = if w_scale > h_scale {
                // Width is dominant, adjust height
                (natural_w, natural_w / ratio)
            } else {
                // Height is dominant, adjust width
                (natural_h * ratio, natural_h)
            };

            // Back-calculate dx and dy to achieve new_w and new_h
            match handle {
                ResizeHandle::TopLeft => {
                    dx = orig_width - new_w;
                    dy = new_h - orig_height;
                }
                ResizeHandle::TopRight => {
                    dx = new_w - orig_width;
                    dy = new_h - orig_height;
                }
                ResizeHandle::BottomLeft => {
                    dx = orig_width - new_w;
                    dy = orig_height - new_h;
                }
                ResizeHandle::BottomRight => {
                    dx = new_w - orig_width;
                    dy = orig_height - new_h;
                }
            }
        }

        // Calculate new bounds based on which handle is being dragged
        let (_new_x, _new_y, new_width, new_height) = match handle {
            ResizeHandle::TopLeft => {
                // Moving top-left corner (min_x, max_y in Y-up)
                let new_min_x = orig_x + dx;
                let new_max_y = orig_y + orig_height + dy;
                let new_width = (orig_x + orig_width) - new_min_x;
                let new_height = new_max_y - orig_y;
                (new_min_x, orig_y, new_width, new_height)
            }
            ResizeHandle::TopRight => {
                // Moving top-right corner (max_x, max_y in Y-up)
                let new_max_x = orig_x + orig_width + dx;
                let new_max_y = orig_y + orig_height + dy;
                let new_width = new_max_x - orig_x;
                let new_height = new_max_y - orig_y;
                (orig_x, orig_y, new_width, new_height)
            }
            ResizeHandle::BottomLeft => {
                // Moving bottom-left corner (min_x, min_y in Y-up)
                let new_min_x = orig_x + dx;
                let new_min_y = orig_y + dy;
                let new_width = (orig_x + orig_width) - new_min_x;
                let new_height = (orig_y + orig_height) - new_min_y;
                (new_min_x, new_min_y, new_width, new_height)
            }
            ResizeHandle::BottomRight => {
                // Moving bottom-right corner (max_x, min_y in Y-up)
                let new_max_x = orig_x + orig_width + dx;
                let new_min_y = orig_y + dy;
                let new_width = new_max_x - orig_x;
                let new_height = (orig_y + orig_height) - new_min_y;
                (orig_x, new_min_y, new_width, new_height)
            }
        };

        // Apply minimum size constraints
        if new_width.abs() < 5.0 || new_height.abs() < 5.0 {
            return;
        }

        // Prevent flips (negative dimensions) which would invert shapes.
        if new_width <= 0.0 || new_height <= 0.0 {
            return;
        }

        let sx = if orig_width.abs() > 1e-6 {
            new_width / orig_width
        } else {
            1.0
        };
        let sy = if orig_height.abs() > 1e-6 {
            new_height / orig_height
        } else {
            1.0
        };

        let (anchor_x, anchor_y) = match handle {
            ResizeHandle::TopLeft => (orig_x + orig_width, orig_y),
            ResizeHandle::TopRight => (orig_x, orig_y),
            ResizeHandle::BottomLeft => (orig_x + orig_width, orig_y + orig_height),
            ResizeHandle::BottomRight => (orig_x, orig_y + orig_height),
        };

        // Update the shape
        let mut state = self.state.borrow_mut();

        // Restore original shapes first so drag updates don't compound transforms.
        // (Without this, we repeatedly multiply already-scaled dimensions and the selection
        // shrinks/grows exponentially.)
        if let Some(originals) = self.resize_original_shapes.borrow().as_ref() {
            for (id, original_shape) in originals {
                if let Some(obj) = state.canvas.shape_store.get_mut(*id) {
                    if obj.selected {
                        obj.shape = original_shape.clone();
                    }
                }
            }
        }

        // Apply scaling to all selected shapes (single or multiple)
        // This ensures consistent behavior for rotated shapes where AABB resizing
        // should be treated as a scaling operation relative to the anchor point.
        let anchor = Point::new(anchor_x, anchor_y);
        for obj in state.canvas.shape_store.iter_mut() {
            if !obj.selected {
                continue;
            }
            match &mut obj.shape {
                Shape::Rectangle(rect) => {
                    rect.center.x = anchor.x + (rect.center.x - anchor.x) * sx;
                    rect.center.y = anchor.y + (rect.center.y - anchor.y) * sy;
                    rect.width *= sx.abs();
                    rect.height *= sy.abs();

                    if rect.is_slot {
                        rect.corner_radius = rect.width.min(rect.height) / 2.0;
                    } else {
                        rect.corner_radius *= sx.abs().min(sy.abs());
                    }
                }
                Shape::Circle(circle) => {
                    circle.center.x = anchor.x + (circle.center.x - anchor.x) * sx;
                    circle.center.y = anchor.y + (circle.center.y - anchor.y) * sy;
                    let s = sx.abs().min(sy.abs());
                    circle.radius *= s;
                }
                Shape::Ellipse(ellipse) => {
                    ellipse.center.x = anchor.x + (ellipse.center.x - anchor.x) * sx;
                    ellipse.center.y = anchor.y + (ellipse.center.y - anchor.y) * sy;
                    ellipse.rx *= sx.abs();
                    ellipse.ry *= sy.abs();
                }
                Shape::Line(line) => {
                    line.start.x = anchor.x + (line.start.x - anchor.x) * sx;
                    line.start.y = anchor.y + (line.start.y - anchor.y) * sy;
                    line.end.x = anchor.x + (line.end.x - anchor.x) * sx;
                    line.end.y = anchor.y + (line.end.y - anchor.y) * sy;
                }
                Shape::Path(path_shape) => {
                    path_shape.scale(sx, sy, anchor);
                }
                Shape::Text(text) => {
                    text.scale(sx, sy, anchor);
                }
                Shape::Triangle(triangle) => {
                    triangle.center.x = anchor.x + (triangle.center.x - anchor.x) * sx;
                    triangle.center.y = anchor.y + (triangle.center.y - anchor.y) * sy;
                    triangle.width *= sx.abs();
                    triangle.height *= sy.abs();
                }
                Shape::Polygon(polygon) => {
                    polygon.center.x = anchor.x + (polygon.center.x - anchor.x) * sx;
                    polygon.center.y = anchor.y + (polygon.center.y - anchor.y) * sy;
                    let s = sx.abs().min(sy.abs());
                    polygon.radius *= s;
                }
                Shape::Gear(gear) => {
                    gear.center.x = anchor.x + (gear.center.x - anchor.x) * sx;
                    gear.center.y = anchor.y + (gear.center.y - anchor.y) * sy;
                    let s = sx.abs().min(sy.abs());
                    gear.module *= s;
                }
                Shape::Sprocket(sprocket) => {
                    sprocket.center.x = anchor.x + (sprocket.center.x - anchor.x) * sx;
                    sprocket.center.y = anchor.y + (sprocket.center.y - anchor.y) * sy;
                    let s = sx.abs().min(sy.abs());
                    sprocket.pitch *= s;
                    sprocket.roller_diameter *= s;
                }
                Shape::TabbedBox(tbox) => {
                    tbox.center.x = anchor.x + (tbox.center.x - anchor.x) * sx;
                    tbox.center.y = anchor.y + (tbox.center.y - anchor.y) * sy;
                    tbox.width *= sx.abs();
                    tbox.height *= sy.abs();
                }
            }
        }
    }

    fn draw_resize_handles(
        cr: &gtk4::cairo::Context,
        bounds: &(f64, f64, f64, f64),
        zoom: f64,
        accent_color: &gtk4::gdk::RGBA,
    ) {
        let handle_size = 8.0 / zoom;
        let half_size = handle_size / 2.0;

        let (min_x, min_y, max_x, max_y) = *bounds;

        cr.save().unwrap();

        // Draw handles at corners
        let corners = [
            (min_x, max_y), // Top-left (Y-up)
            (max_x, max_y), // Top-right
            (min_x, min_y), // Bottom-left
            (max_x, min_y), // Bottom-right
        ];

        for (cx, cy) in corners {
            // Draw white fill
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.rectangle(cx - half_size, cy - half_size, handle_size, handle_size);
            cr.fill().unwrap();

            // Draw accent border
            cr.set_source_rgba(
                accent_color.red() as f64,
                accent_color.green() as f64,
                accent_color.blue() as f64,
                accent_color.alpha() as f64,
            );
            cr.set_line_width(2.0 / zoom);
            cr.rectangle(cx - half_size, cy - half_size, handle_size, handle_size);
            cr.stroke().unwrap();
        }

        cr.restore().unwrap();
    }
}

impl DesignerView {
    pub fn new(
        device_manager: Option<Arc<DeviceManager>>,
        settings_controller: Rc<SettingsController>,
        status_bar: Option<crate::ui::gtk::status_bar::StatusBar>,
    ) -> Rc<Self> {
        let container = Box::new(Orientation::Vertical, 0);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Create designer state
        let state = Rc::new(RefCell::new(DesignerState::new()));

        // Create main horizontal layout (toolbox + canvas + properties)
        let main_box = Box::new(Orientation::Horizontal, 0);
        main_box.set_hexpand(true);
        main_box.set_vexpand(true);

        // Create toolbox + left sidebar container (toolbox + view/legend)
        let toolbox = DesignerToolbox::new(state.clone(), settings_controller.clone());
        let left_sidebar = Box::new(Orientation::Vertical, 0);
        left_sidebar.set_vexpand(true);
        left_sidebar.set_hexpand(false);
        left_sidebar.set_halign(gtk4::Align::Fill);
        left_sidebar.append(&toolbox.widget);

        // Keep left sidebar at ~20% of the main window width (set on first map).
        let last_left_width = Rc::new(std::cell::Cell::new(-1));
        {
            let left_sidebar = left_sidebar.clone();
            let last_left_width = last_left_width.clone();
            let container_width = container.clone();
            container.connect_map(move |_| {
                let left_sidebar = left_sidebar.clone();
                let last_left_width = last_left_width.clone();
                let container_width = container_width.clone();
                gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                    let w = container_width.width();
                    if w <= 0 {
                        return gtk4::glib::ControlFlow::Continue;
                    }
                    let target = ((w as f64) * 0.20).round() as i32;
                    let target = target.max(160);
                    if last_left_width.get() != target {
                        last_left_width.set(target);
                        left_sidebar.set_width_request(target);
                    }
                    gtk4::glib::ControlFlow::Break
                });
            });
        }

        main_box.append(&left_sidebar);

        // Create canvas
        let canvas = DesignerCanvas::new(
            state.clone(),
            Some(toolbox.clone()),
            device_manager.clone(),
            status_bar.clone(),
        );

        // Create Grid for Canvas + Scrollbars
        let canvas_grid = Grid::new();
        canvas_grid.set_hexpand(true);
        canvas_grid.set_vexpand(true);

        canvas.widget.set_hexpand(true);
        canvas.widget.set_vexpand(true);

        // Overlay for floating controls
        let overlay = Overlay::new();
        overlay.set_child(Some(&canvas.widget));

        // Floating Controls (Bottom Right)
        let floating_box = Box::new(Orientation::Horizontal, 4);
        floating_box.add_css_class("visualizer-osd"); // Reuse visualizer OSD style
        floating_box.add_css_class("osd-controls");
        floating_box.set_halign(gtk4::Align::End);
        floating_box.set_valign(gtk4::Align::End);
        floating_box.set_margin_bottom(20);
        floating_box.set_margin_end(20);

        let float_zoom_out = gtk4::Button::builder()
            .icon_name("zoom-out-symbolic")
            .tooltip_text(t!("Zoom Out"))
            .build();
        float_zoom_out.update_property(&[gtk4::accessible::Property::Label(&t!("Zoom Out"))]);

        let float_fit = gtk4::Button::builder()
            .icon_name("zoom-fit-best-symbolic")
            .tooltip_text(t!("Fit to Content"))
            .build();
        float_fit.update_property(&[gtk4::accessible::Property::Label(&t!("Fit to Content"))]);

        let float_reset = gtk4::Button::builder()
            .icon_name("view-restore-symbolic")
            .tooltip_text(t!("Fit to Viewport"))
            .build();
        float_reset.update_property(&[gtk4::accessible::Property::Label(&t!("Fit to Viewport"))]);

        let float_fit_device = gtk4::Button::builder()
            .icon_name("preferences-desktop-display-symbolic")
            .tooltip_text(t!("Fit to Device Working Area"))
            .build();
        float_fit_device.update_property(&[gtk4::accessible::Property::Label(&t!(
            "Fit to Device Working Area"
        ))]);

        let scrollbars_btn = gtk4::Button::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text(t!("Toggle Scrollbars"))
            .build();
        scrollbars_btn
            .update_property(&[gtk4::accessible::Property::Label(&t!("Toggle Scrollbars"))]);

        let help_btn = gtk4::Button::builder()
            .label("?")
            .tooltip_text(t!("Keyboard Shortcuts"))
            .build();
        help_btn.update_property(&[gtk4::accessible::Property::Label(&t!("Keyboard Shortcuts"))]);

        let help_popover = Popover::new();
        help_popover.set_parent(&help_btn);
        help_popover.set_has_arrow(true);
        {
            let help_box = Box::new(Orientation::Vertical, 6);
            help_box.set_margin_start(12);
            help_box.set_margin_end(12);
            help_box.set_margin_top(12);
            help_box.set_margin_bottom(12);
            help_box.append(&Label::new(Some(&t!("Designer shortcuts"))));
            help_box.append(&Label::new(Some("Ctrl+Z / Ctrl+Y  —  Undo / Redo")));
            help_box.append(&Label::new(Some("Ctrl+C / Ctrl+V  —  Copy / Paste")));
            help_box.append(&Label::new(Some("Delete  —  Delete selection")));
            help_box.append(&Label::new(Some("+ / -  —  Zoom")));
            help_box.append(&Label::new(Some("F  —  Fit to Content")));
            help_box.append(&Label::new(Some("R  —  Fit to Viewport")));
            help_box.append(&Label::new(Some("D  —  Fit to Device Working Area")));
            help_box.append(&Label::new(Some(&t!("Right click for context menu"))));
            help_popover.set_child(Some(&help_box));
        }
        {
            let pop = help_popover.clone();
            help_btn.connect_clicked(move |_| pop.popup());
        }

        let float_zoom_in = gtk4::Button::builder()
            .icon_name("zoom-in-symbolic")
            .tooltip_text(t!("Zoom In"))
            .build();
        float_zoom_in.update_property(&[gtk4::accessible::Property::Label(&t!("Zoom In"))]);

        for b in [
            &float_zoom_out,
            &float_fit,
            &float_reset,
            &float_fit_device,
            &scrollbars_btn,
            &help_btn,
            &float_zoom_in,
        ] {
            b.set_size_request(28, 28);
        }

        floating_box.append(&float_zoom_out);
        floating_box.append(&float_fit);
        floating_box.append(&float_reset);
        if device_manager.is_some() {
            floating_box.append(&float_fit_device);
        }
        floating_box.append(&scrollbars_btn);
        floating_box.append(&help_btn);
        floating_box.append(&float_zoom_in);

        // Empty state (shown when no shapes)
        let empty_box = Box::new(Orientation::Vertical, 8);
        empty_box.add_css_class("visualizer-osd");
        empty_box.set_halign(gtk4::Align::Center);
        empty_box.set_valign(gtk4::Align::Center);
        empty_box.set_margin_start(20);
        empty_box.set_margin_end(20);
        empty_box.set_margin_top(20);
        empty_box.set_margin_bottom(20);
        empty_box.append(&Label::new(Some(&t!("No shapes yet"))));
        empty_box.append(&Label::new(Some(&t!(
            "Use the toolbox on the left to start drawing."
        ))));

        let empty_actions = Box::new(Orientation::Horizontal, 8);
        empty_actions.set_halign(gtk4::Align::Center);

        let empty_new_btn = gtk4::Button::with_label(&t!("New"));
        empty_new_btn.add_css_class("suggested-action");
        let empty_open_btn = gtk4::Button::with_label(&t!("Load Design"));
        let empty_import_svg_btn = gtk4::Button::with_label(&t!("Import SVG"));
        let empty_import_dxf_btn = gtk4::Button::with_label(&t!("Import DXF"));

        empty_actions.append(&empty_new_btn);
        empty_actions.append(&empty_open_btn);
        empty_actions.append(&empty_import_svg_btn);
        empty_actions.append(&empty_import_dxf_btn);
        empty_box.append(&empty_actions);

        empty_box.set_visible(true);

        overlay.add_overlay(&empty_box);
        overlay.add_overlay(&floating_box);

        // Status Panel (Bottom Left)
        let status_box = Box::new(Orientation::Horizontal, 4);
        status_box.add_css_class("visualizer-osd");
        status_box.set_halign(gtk4::Align::Start);
        status_box.set_valign(gtk4::Align::End);
        status_box.set_margin_bottom(20);
        status_box.set_margin_start(20);

        let status_label_osd = Label::builder().label(" ").build();
        status_label_osd.set_hexpand(true);

        let units_badge = Label::new(Some(""));
        units_badge.add_css_class("osd-units-badge");

        status_box.append(&status_label_osd);
        status_box.append(&units_badge);

        overlay.add_overlay(&status_box);

        // Attach Overlay to Grid (instead of direct canvas)
        canvas_grid.attach(&overlay, 0, 0, 1, 1);

        // Scrollbars
        // Range: use shared world extent (±WORLD_EXTENT_MM)
        let world_extent = gcodekit5_core::constants::WORLD_EXTENT_MM as f64;
        let h_adjustment = Adjustment::new(0.0, -world_extent, world_extent, 10.0, 100.0, 100.0);
        let v_adjustment = Adjustment::new(0.0, -world_extent, world_extent, 10.0, 100.0, 100.0);

        let h_scrollbar = Scrollbar::new(Orientation::Horizontal, Some(&h_adjustment));
        let v_scrollbar = Scrollbar::new(Orientation::Vertical, Some(&v_adjustment));

        // Default hidden (toggleable) to maximize canvas space
        h_scrollbar.set_visible(false);
        v_scrollbar.set_visible(false);

        canvas_grid.attach(&v_scrollbar, 1, 0, 1, 1);
        canvas_grid.attach(&h_scrollbar, 0, 1, 1, 1);

        main_box.append(&canvas_grid);

        // Connect scrollbars to canvas pan
        let canvas_h = canvas.clone();
        h_adjustment.connect_value_changed(move |adj| {
            let val = adj.value();
            let mut state = canvas_h.state.borrow_mut();
            // Pan is opposite to scroll value usually
            let current_pan_y = state.canvas.pan_y();
            state.canvas.set_pan(-val, current_pan_y);
            drop(state);
            canvas_h.widget.queue_draw();
        });

        let canvas_v = canvas.clone();
        v_adjustment.connect_value_changed(move |adj| {
            let val = adj.value();
            let mut state = canvas_v.state.borrow_mut();
            // Positive scroll value (down) moves content up (positive pan_y)
            let current_pan_x = state.canvas.pan_x();
            state.canvas.set_pan(current_pan_x, val);
            drop(state);
            canvas_v.widget.queue_draw();
        });

        // Pass adjustments to canvas
        canvas.set_adjustments(h_adjustment.clone(), v_adjustment.clone());

        // Connect Floating Zoom Buttons
        let canvas_zoom = canvas.clone();
        float_zoom_in.connect_clicked(move |_| {
            canvas_zoom.zoom_in();
        });

        let canvas_zoom = canvas.clone();
        float_zoom_out.connect_clicked(move |_| {
            canvas_zoom.zoom_out();
        });

        let canvas_zoom = canvas.clone();
        float_fit.connect_clicked(move |_| {
            canvas_zoom.zoom_fit();
        });

        let canvas_reset = canvas.clone();
        float_reset.connect_clicked(move |_| {
            canvas_reset.reset_view();
        });

        let canvas_fitdev = canvas.clone();
        float_fit_device.connect_clicked(move |_| {
            canvas_fitdev.fit_to_device_area();
            canvas_fitdev.widget.queue_draw();
        });

        // Scrollbars toggle
        let show_scrollbars = Rc::new(std::cell::Cell::new(false));
        let show_scrollbars_btn = show_scrollbars.clone();
        let hsb = h_scrollbar.clone();
        let vsb = v_scrollbar.clone();
        scrollbars_btn.connect_clicked(move |_| {
            let next = !show_scrollbars_btn.get();
            show_scrollbars_btn.set(next);
            hsb.set_visible(next);
            vsb.set_visible(next);
        });

        // Auto-fit when designer is mapped (visible) — schedule after layout like Visualizer
        let canvas_map = canvas.clone();
        container.connect_map(move |_| {
            let canvas_run = canvas_map.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                // Ensure viewport knows the correct size before fitting
                let width = canvas_run.widget.width() as f64;
                let height = canvas_run.widget.height() as f64;
                if width > 0.0 && height > 0.0 {
                    if let Ok(mut state) = canvas_run.state.try_borrow_mut() {
                        state.canvas.set_canvas_size(width, height);
                    }
                }

                // Always fit to device on initialization as per user request
                canvas_run.fit_to_device_area();
                canvas_run.widget.queue_draw();
                gtk4::glib::ControlFlow::Break
            });
        });

        // Create right sidebar with properties and layers
        let right_sidebar = Box::new(Orientation::Vertical, 5);
        right_sidebar.set_hexpand(false);
        right_sidebar.set_halign(gtk4::Align::End);

        // Keep right sidebar at ~20% of the main window width (set on first map).
        let last_right_width = Rc::new(std::cell::Cell::new(-1));
        {
            let right_sidebar = right_sidebar.clone();
            let last_right_width = last_right_width.clone();
            let container_width = container.clone();
            container.connect_map(move |_| {
                let right_sidebar = right_sidebar.clone();
                let last_right_width = last_right_width.clone();
                let container_width = container_width.clone();
                gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                    let w = container_width.width();
                    if w <= 0 {
                        return gtk4::glib::ControlFlow::Continue;
                    }
                    let target = ((w as f64) * 0.20).round() as i32;
                    let target = target.clamp(240, 520);
                    if last_right_width.get() != target {
                        last_right_width.set(target);
                        right_sidebar.set_width_request(target);
                    }
                    gtk4::glib::ControlFlow::Break
                });
            });
        }

        // Create properties panel
        let properties = PropertiesPanel::new(
            state.clone(),
            settings_controller.persistence.clone(),
            canvas.preview_shapes.clone(),
        );
        properties.widget.set_vexpand(true);
        properties.widget.set_valign(gtk4::Align::Fill);

        // Set up redraw callback for properties
        let canvas_redraw = canvas.clone();
        properties.set_redraw_callback(move || {
            let show_toolpaths = canvas_redraw.state.borrow().show_toolpaths;
            if show_toolpaths {
                canvas_redraw.generate_preview_toolpaths();
            }
            canvas_redraw.widget.queue_draw();
        });

        // Inspector header + hide button (matches DeviceConsole / Visualizer sidebar UX)
        let props_hidden = Rc::new(Cell::new(false));

        let inspector_header = Box::new(Orientation::Horizontal, 6);
        inspector_header.set_margin_start(6);
        inspector_header.set_margin_end(6);
        inspector_header.set_margin_top(6);

        let inspector_label = Label::builder()
            .label(t!("Inspector"))
            .css_classes(vec!["heading"])
            .halign(gtk4::Align::Start)
            .build();
        inspector_label.set_hexpand(true);
        inspector_header.append(&inspector_label);

        let props_hide_btn = gtk4::Button::builder()
            .tooltip_text(t!("Hide Properties"))
            .build();
        props_hide_btn
            .update_property(&[gtk4::accessible::Property::Label(&t!("Hide Properties"))]);
        {
            let child = Box::new(Orientation::Horizontal, 6);
            child.append(&gtk4::Image::from_icon_name("view-conceal-symbolic"));
            child.append(&Label::new(Some(&t!("Hide"))));
            props_hide_btn.set_child(Some(&child));
        }
        inspector_header.append(&props_hide_btn);

        right_sidebar.append(&inspector_header);

        // Create layers panel
        let layers = Rc::new(LayersPanel::new(state.clone(), canvas.widget.clone()));
        layers.widget.set_vexpand(true);
        layers.widget.set_valign(gtk4::Align::Fill);

        // Draggable divider between Properties and Layers
        let inspector_paned = Paned::new(Orientation::Vertical);
        inspector_paned.set_vexpand(true);
        inspector_paned.set_start_child(Some(&properties.widget));
        inspector_paned.set_end_child(Some(&layers.widget));
        inspector_paned.set_resize_start_child(true);
        inspector_paned.set_resize_end_child(true);
        inspector_paned.set_shrink_start_child(false);
        inspector_paned.set_shrink_end_child(false);
        inspector_paned.set_position(520);

        right_sidebar.append(&inspector_paned);

        // Floating unhide button (top-right of canvas)
        let props_show_btn = gtk4::Button::builder()
            .tooltip_text(t!("Unhide Properties"))
            .build();
        props_show_btn
            .update_property(&[gtk4::accessible::Property::Label(&t!("Unhide Properties"))]);
        {
            let child = Box::new(Orientation::Horizontal, 6);
            child.append(&gtk4::Image::from_icon_name("view-reveal-symbolic"));
            child.append(&Label::new(Some(&t!("Unhide"))));
            props_show_btn.set_child(Some(&child));
        }

        let props_show_panel = Box::new(Orientation::Horizontal, 0);
        props_show_panel.add_css_class("visualizer-osd");
        props_show_panel.add_css_class("osd-controls");
        props_show_panel.set_halign(gtk4::Align::End);
        props_show_panel.set_valign(gtk4::Align::Start);
        props_show_panel.set_margin_end(12);
        props_show_panel.set_margin_top(12);
        props_show_panel.append(&props_show_btn);
        props_show_panel.set_visible(false);
        overlay.add_overlay(&props_show_panel);

        {
            let right_sidebar = right_sidebar.clone();
            let props_hidden = props_hidden.clone();
            let show_panel = props_show_panel.clone();
            let hide_btn = props_hide_btn.clone();
            props_hide_btn.connect_clicked(move |_| {
                if props_hidden.get() {
                    return;
                }
                right_sidebar.set_visible(false);
                hide_btn.set_sensitive(false);
                props_hidden.set(true);
                show_panel.set_visible(true);
            });
        }

        {
            let right_sidebar = right_sidebar.clone();
            let props_hidden = props_hidden.clone();
            let show_panel = props_show_panel.clone();
            let hide_btn = props_hide_btn.clone();
            props_show_btn.connect_clicked(move |_| {
                if !props_hidden.get() {
                    return;
                }
                right_sidebar.set_visible(true);
                hide_btn.set_sensitive(true);
                props_hidden.set(false);
                show_panel.set_visible(false);
            });
        }

        // Legend moved to left sidebar

        // Give canvas references to panels
        canvas.set_properties_panel(properties.clone());
        canvas.set_layers_panel(layers.clone());

        main_box.append(&right_sidebar);

        container.append(&main_box);

        // Hidden labels retained for internal status messages (status bar removed)
        let status_label = Label::new(None);
        let coord_label = Label::new(None);

        // View controls (moved from the removed bottom status bar)
        let view_controls_box = Box::new(Orientation::Vertical, 6);
        view_controls_box.set_margin_start(6);
        view_controls_box.set_margin_end(6);
        view_controls_box.set_margin_top(6);
        view_controls_box.set_margin_bottom(6);

        // Grid toggle
        let grid_toggle = gtk4::CheckButton::with_label(&t!("Show Grid"));
        grid_toggle.set_active(true);
        let state_grid = state.clone();
        let canvas_grid = canvas.clone();
        grid_toggle.connect_toggled(move |btn| {
            state_grid.borrow_mut().show_grid = btn.is_active();
            canvas_grid.widget.queue_draw();
        });
        view_controls_box.append(&grid_toggle);

        // Grid spacing
        let system = settings_controller
            .persistence
            .borrow()
            .config()
            .ui
            .measurement_system;
        let unit_label = gcodekit5_core::units::get_unit_label(system);

        let grid_spacing_combo = gtk4::ComboBoxText::new();
        grid_spacing_combo.set_hexpand(true);
        grid_spacing_combo.set_tooltip_text(Some(&t!("Grid spacing")));

        let grid_options_mm = [1.0_f64, 5.0, 10.0, 25.0, 50.0];
        for mm in grid_options_mm {
            let label = format!(
                "{} {}",
                gcodekit5_core::units::format_length(mm as f32, system),
                unit_label
            );
            grid_spacing_combo.append(Some(&mm.to_string()), &label);
        }

        // Default 10mm
        grid_spacing_combo.set_active_id(Some("10"));
        {
            let state_grid_spacing = state.clone();
            let canvas_grid_spacing = canvas.clone();
            grid_spacing_combo.connect_changed(move |cb| {
                let Some(id) = cb.active_id() else {
                    return;
                };
                if let Ok(mm) = id.parse::<f64>() {
                    state_grid_spacing.borrow_mut().grid_spacing_mm = mm;
                    canvas_grid_spacing.widget.queue_draw();
                }
            });
        }

        let grid_spacing_row = Box::new(Orientation::Horizontal, 6);
        grid_spacing_row.append(&Label::new(Some(&t!("Grid spacing"))));
        grid_spacing_row.append(&grid_spacing_combo);
        view_controls_box.append(&grid_spacing_row);

        // Snap controls
        let snap_toggle = gtk4::CheckButton::with_label(&t!("Snap"));
        snap_toggle.set_tooltip_text(Some(&t!("Snap to grid")));
        snap_toggle.set_active(state.borrow().snap_enabled);
        {
            let state_snap = state.clone();
            snap_toggle.connect_toggled(move |btn| {
                state_snap.borrow_mut().snap_enabled = btn.is_active();
            });
        }
        view_controls_box.append(&snap_toggle);

        let snap_threshold = gtk4::SpinButton::with_range(0.0, 5.0, 0.1);
        snap_threshold.set_tooltip_text(Some(&t!("Snap threshold")));
        let snap_display = match system {
            gcodekit5_core::units::MeasurementSystem::Metric => state.borrow().snap_threshold_mm,
            gcodekit5_core::units::MeasurementSystem::Imperial => {
                state.borrow().snap_threshold_mm / 25.4
            }
        };
        snap_threshold.set_value(snap_display);
        {
            let state_snap = state.clone();
            snap_threshold.connect_value_changed(move |sp| {
                let val = sp.value();
                let mm = match system {
                    gcodekit5_core::units::MeasurementSystem::Metric => val,
                    gcodekit5_core::units::MeasurementSystem::Imperial => val * 25.4,
                };
                state_snap.borrow_mut().snap_threshold_mm = mm.max(0.0);
            });
        }

        let snap_threshold_row = Box::new(Orientation::Horizontal, 6);
        snap_threshold_row.append(&Label::new(Some(&t!("Snap threshold"))));
        snap_threshold_row.append(&snap_threshold);
        view_controls_box.append(&snap_threshold_row);

        // Toolpath toggle
        let toolpath_toggle = gtk4::CheckButton::with_label(&t!("Show Toolpaths"));
        toolpath_toggle.set_active(false);
        let state_toolpath = state.clone();
        let canvas_toolpath = canvas.clone();
        toolpath_toggle.connect_toggled(move |btn| {
            let active = btn.is_active();
            state_toolpath.borrow_mut().show_toolpaths = active;
            if active {
                canvas_toolpath.generate_preview_toolpaths();
            } else {
                canvas_toolpath.widget.queue_draw();
            }
        });
        view_controls_box.append(&toolpath_toggle);

        // Preview generation progress + cancel
        let preview_spinner = gtk4::Spinner::new();
        preview_spinner.set_visible(false);

        let preview_cancel_btn = gtk4::Button::builder()
            .icon_name("process-stop-symbolic")
            .tooltip_text(t!("Cancel"))
            .build();
        preview_cancel_btn.set_visible(false);
        preview_cancel_btn.update_property(&[gtk4::accessible::Property::Label(&t!("Cancel"))]);

        {
            let cancel_flag = canvas.preview_cancel.clone();
            let generating = canvas.preview_generating.clone();
            preview_cancel_btn.connect_clicked(move |_| {
                cancel_flag.store(true, Ordering::SeqCst);
                generating.set(false);
            });
        }

        let preview_row = Box::new(Orientation::Horizontal, 6);
        preview_row.append(&preview_spinner);
        preview_row.append(&preview_cancel_btn);
        view_controls_box.append(&preview_row);

        // Keep widgets in sync
        {
            let generating = canvas.preview_generating.clone();
            let spinner = preview_spinner.clone();
            let cancel_btn = preview_cancel_btn.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                let on = generating.get();
                spinner.set_visible(on);
                cancel_btn.set_visible(on);
                if on {
                    spinner.start();
                } else {
                    spinner.stop();
                }
                gtk4::glib::ControlFlow::Continue
            });
        }

        let view_controls_expander = gtk4::Expander::builder()
            .label(t!("View Controls"))
            .expanded(true)
            .child(&view_controls_box)
            .build();
        left_sidebar.append(&view_controls_expander);

        // Update status OSD + empty state
        let status_osd_clone = status_label_osd.clone();
        let units_badge_osd = units_badge.clone();
        let empty_osd = empty_box.clone();
        let canvas_osd = canvas.clone();
        let settings_osd = settings_controller.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let state = canvas_osd.state.borrow();
            let zoom = state.canvas.zoom();
            let pan_x = state.canvas.pan_x();
            let pan_y = state.canvas.pan_y();
            let has_shapes = !state.canvas.shape_store.is_empty();
            let snap_on = state.snap_enabled;
            drop(state);

            let constraint_on = *canvas_osd.shift_pressed.borrow();

            let width = canvas_osd.widget.width() as f64;
            let height = canvas_osd.widget.height() as f64;

            // Center-of-viewport in canvas coordinates
            let center_x = ((width / 2.0) - pan_x) / zoom;
            let center_y = ((height / 2.0) - pan_y) / zoom;

            let (cursor_x, cursor_y) = *canvas_osd.mouse_pos.borrow();

            let system = settings_osd
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system;
            let mut status = format_zoom_center_cursor(
                zoom,
                center_x as f32,
                center_y as f32,
                cursor_x as f32,
                cursor_y as f32,
                system,
            );

            if snap_on || constraint_on {
                let mut parts: Vec<String> = Vec::new();
                if snap_on {
                    parts.push(t!("Snap"));
                }
                if constraint_on {
                    parts.push(t!("Constraint"));
                }
                status.push_str(&format!("  {}", parts.join(" / ")));
            }

            status_osd_clone.set_text(&status);
            units_badge_osd.set_text(gcodekit5_core::units::get_unit_label(system));
            empty_osd.set_visible(!has_shapes);

            gtk4::glib::ControlFlow::Continue
        });

        let current_file = Rc::new(RefCell::new(None));
        let on_gcode_generated: Rc<RefCell<Option<std::boxed::Box<dyn Fn(String)>>>> =
            Rc::new(RefCell::new(None));

        // Connect Generate G-Code button
        let canvas_gen = canvas.clone();
        let on_gen = on_gcode_generated.clone();
        let status_label_gen = status_label.clone();

        toolbox.connect_generate_clicked(move || {
            let mut state = canvas_gen.state.borrow_mut();

            // Copy settings to avoid borrow issues
            let feed_rate = state.tool_settings.feed_rate;
            let spindle_speed = state.tool_settings.spindle_speed;
            let tool_diameter = state.tool_settings.tool_diameter;
            let cut_depth = state.tool_settings.cut_depth;
            let start_depth = state.tool_settings.start_depth;

            // Update toolpath generator settings from state
            state.toolpath_generator.set_feed_rate(feed_rate);
            state.toolpath_generator.set_spindle_speed(spindle_speed);
            state.toolpath_generator.set_tool_diameter(tool_diameter);
            state.toolpath_generator.set_cut_depth(cut_depth);
            state.toolpath_generator.set_start_depth(start_depth);
            state.toolpath_generator.set_step_in(tool_diameter * 0.4); // Default stepover

            let gcode = state.generate_gcode();
            drop(state);

            status_label_gen.set_text(&t!("G-Code generated"));

            if let Some(callback) = on_gen.borrow().as_ref() {
                callback(gcode);
            }
        });

        // Connect toolbox operation buttons
        let canvas_mirror_x = canvas.clone();
        toolbox.connect_mirror_x_clicked(move || {
            canvas_mirror_x.mirror_x();
        });

        let canvas_mirror_y = canvas.clone();
        toolbox.connect_mirror_y_clicked(move || {
            canvas_mirror_y.mirror_y();
        });

        let view = Rc::new(Self {
            widget: container,
            canvas: canvas.clone(),
            toolbox: toolbox.clone(),
            _properties: properties.clone(),
            layers: layers.clone(),
            status_label,
            _coord_label: coord_label,
            current_file,
            on_gcode_generated,
            settings_persistence: Some(settings_controller.persistence.clone()),
        });

        // Empty state actions
        {
            let v = view.clone();
            empty_new_btn.connect_clicked(move |_| v.new_file());
        }
        {
            let v = view.clone();
            empty_open_btn.connect_clicked(move |_| v.open_file());
        }
        {
            let v = view.clone();
            empty_import_svg_btn.connect_clicked(move |_| v.import_svg_file());
        }
        {
            let v = view.clone();
            empty_import_dxf_btn.connect_clicked(move |_| v.import_dxf_file());
        }

        // Update properties panel and toolbox when selection changes
        let props_update = properties.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            // Check if we need to update properties (when canvas is redrawn or selection changes)
            props_update.update_from_selection();

            gtk4::glib::ControlFlow::Continue
        });

        // Setup keyboard shortcuts
        let key_controller = EventControllerKey::new();
        let canvas_keys = canvas.clone();
        key_controller.connect_key_pressed(move |_, key, _code, modifiers| {
            let ctrl = modifiers.contains(ModifierType::CONTROL_MASK);

            match (key, ctrl) {
                // Ctrl+Z - Undo
                (Key::z, true) | (Key::Z, true) => {
                    canvas_keys.undo();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+Y or Ctrl+Shift+Z - Redo
                (Key::y, true) | (Key::Y, true) => {
                    canvas_keys.redo();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+C - Copy
                (Key::c, true) | (Key::C, true) => {
                    canvas_keys.copy_selected();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+V - Paste
                (Key::v, true) | (Key::V, true) => {
                    canvas_keys.paste();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+D - Duplicate
                (Key::d, true) | (Key::D, true) => {
                    canvas_keys.duplicate_selected();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+G - Group (Shift+G for Ungroup)
                (Key::g, true) | (Key::G, true) => {
                    if modifiers.contains(ModifierType::SHIFT_MASK) {
                        canvas_keys.ungroup_selected();
                    } else {
                        canvas_keys.group_selected();
                    }
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+U - Ungroup
                (Key::u, true) | (Key::U, true) => {
                    canvas_keys.ungroup_selected();
                    gtk4::glib::Propagation::Stop
                }
                // Delete or Backspace - Delete selected
                (Key::Delete, _) | (Key::BackSpace, _) => {
                    canvas_keys.delete_selected();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+L - Align Left
                (Key::l, false) | (Key::L, false) if modifiers.contains(ModifierType::ALT_MASK) => {
                    canvas_keys.align_left();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+R - Align Right
                (Key::r, false) | (Key::R, false) if modifiers.contains(ModifierType::ALT_MASK) => {
                    canvas_keys.align_right();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+T - Align Top
                (Key::t, false) | (Key::T, false) if modifiers.contains(ModifierType::ALT_MASK) => {
                    canvas_keys.align_top();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+B - Align Bottom
                (Key::b, false) | (Key::B, false) if modifiers.contains(ModifierType::ALT_MASK) => {
                    canvas_keys.align_bottom();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+H - Align Center Horizontal
                (Key::h, false) | (Key::H, false) if modifiers.contains(ModifierType::ALT_MASK) => {
                    canvas_keys.align_center_horizontal();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+V - Align Center Vertical
                (Key::v, false) | (Key::V, false) if modifiers.contains(ModifierType::ALT_MASK) => {
                    canvas_keys.align_center_vertical();
                    gtk4::glib::Propagation::Stop
                }
                // View shortcuts (no Ctrl/Alt)
                (Key::plus, false) | (Key::KP_Add, false) | (Key::equal, false)
                    if !modifiers.contains(ModifierType::ALT_MASK) =>
                {
                    canvas_keys.zoom_in();
                    gtk4::glib::Propagation::Stop
                }
                (Key::minus, false) | (Key::KP_Subtract, false) | (Key::underscore, false)
                    if !modifiers.contains(ModifierType::ALT_MASK) =>
                {
                    canvas_keys.zoom_out();
                    gtk4::glib::Propagation::Stop
                }
                (Key::f, false) | (Key::F, false)
                    if !modifiers.contains(ModifierType::ALT_MASK) =>
                {
                    canvas_keys.zoom_fit();
                    gtk4::glib::Propagation::Stop
                }
                (Key::r, false) | (Key::R, false)
                    if !modifiers.contains(ModifierType::ALT_MASK) =>
                {
                    canvas_keys.reset_view();
                    gtk4::glib::Propagation::Stop
                }
                (Key::d, false) | (Key::D, false)
                    if !modifiers.contains(ModifierType::ALT_MASK) =>
                {
                    canvas_keys.fit_to_device_area();
                    canvas_keys.widget.queue_draw();
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });

        // Add keyboard controller to canvas so it receives focus
        canvas.widget.set_focusable(true);
        canvas.widget.set_can_focus(true);
        canvas.widget.add_controller(key_controller);

        // Grab focus on canvas when clicked
        let canvas_focus = canvas.clone();
        let click_for_focus = GestureClick::new();
        click_for_focus.set_button(1); // Only left click for focus to avoid interfering with right-click menu
        click_for_focus.connect_pressed(move |_, _, _, _| {
            canvas_focus.widget.grab_focus();
        });
        canvas.widget.add_controller(click_for_focus);

        // Grab focus initially
        canvas.widget.grab_focus();

        view
    }

    pub fn current_tool(&self) -> DesignerTool {
        self.toolbox.current_tool()
    }

    pub fn set_tool(&self, tool: DesignerTool) {
        self.toolbox.set_tool(tool);
    }

    pub fn set_status(&self, message: &str) {
        self.status_label.set_text(message);
    }

    pub fn set_on_gcode_generated<F: Fn(String) + 'static>(&self, f: F) {
        *self.on_gcode_generated.borrow_mut() = Some(std::boxed::Box::new(f));
    }

    pub fn fit_to_device(&self) {
        self.canvas.fit_to_device_area();
        self.canvas.widget.queue_draw();
    }

    pub fn undo(&self) {
        self.canvas.undo();
    }

    pub fn redo(&self) {
        self.canvas.redo();
    }

    pub fn cut(&self) {
        self.canvas.copy_selected();
        self.canvas.delete_selected();
    }

    pub fn copy(&self) {
        self.canvas.copy_selected();
    }

    pub fn paste(&self) {
        self.canvas.paste();
    }

    pub fn delete(&self) {
        self.canvas.delete_selected();
    }

    pub fn new_file(&self) {
        let mut state = self.canvas.state.borrow_mut();
        state.canvas.clear();
        *self.current_file.borrow_mut() = None;
        drop(state);

        // Refresh layers
        self.layers.refresh(&self.canvas.state);
        self.canvas.widget.queue_draw();
        self.set_status(&t!("New design created"));
    }

    pub fn open_file(&self) {
        let dialog = FileChooserNative::builder()
            .title(t!("Open Design File"))
            .action(FileChooserAction::Open)
            .modal(true)
            .build();

        if let Some(root) = self.widget.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.set_transient_for(Some(window));
            }
        }

        // Set initial directory from settings
        if let Some(ref settings) = self.settings_persistence {
            if let Ok(settings_ref) = settings.try_borrow() {
                let default_dir = &settings_ref.config().file_processing.output_directory;
                if default_dir.exists() {
                    let file = gtk4::gio::File::for_path(default_dir);
                    let _ = dialog.set_current_folder(Some(&file));
                }
            }
        }

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some(&t!("GCodeKit Design Files")));
        filter.add_pattern("*.gckd");
        filter.add_pattern("*.gck5");
        dialog.add_filter(&filter);

        let all_filter = gtk4::FileFilter::new();
        all_filter.set_name(Some(&t!("All Files")));
        all_filter.add_pattern("*");
        dialog.add_filter(&all_filter);

        let canvas = self.canvas.clone();
        let current_file = self.current_file.clone();
        let layers = self.layers.clone();
        let status_label = self.status_label.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        match DesignFile::load_from_file(&path) {
                            Ok(design) => {
                                let mut state = canvas.state.borrow_mut();
                                state.canvas.clear();

                                let mut max_id = 0;
                                let mut restored_shapes: usize = 0;
                                for shape_data in design.shapes {
                                    let id = shape_data.id as u64;
                                    if id > max_id {
                                        max_id = id;
                                    }

                                    if let Ok(obj) =
                                        DesignFile::to_drawing_object(&shape_data, id as i32)
                                    {
                                        state.canvas.restore_shape(obj);
                                        restored_shapes += 1;
                                    }
                                }

                                state.canvas.set_next_id(max_id + 1);

                                // Update viewport (fallback to fit if invalid)
                                let zoom = design.viewport.zoom;
                                let pan_x = design.viewport.pan_x;
                                let pan_y = design.viewport.pan_y;
                                let viewport_ok = zoom.is_finite()
                                    && zoom > 0.0001
                                    && pan_x.is_finite()
                                    && pan_y.is_finite();
                                if viewport_ok {
                                    state.canvas.set_zoom(zoom);
                                    state.canvas.set_pan(pan_x, pan_y);
                                }

                                *current_file.borrow_mut() = Some(path.clone());
                                drop(state);

                                // If the saved viewport is missing/degenerate, frame the loaded geometry.
                                if restored_shapes > 0 && !viewport_ok {
                                    canvas.zoom_fit();
                                }

                                layers.refresh(&canvas.state);
                                canvas.widget.queue_draw();
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Loaded:"),
                                    path.display()
                                ));
                            }
                            Err(e) => {
                                error!("Error loading file: {}", e);
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Error loading file:"),
                                    e
                                ));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    fn import_file_internal(&self, kind: Option<&'static str>) {
        let title = match kind {
            Some("svg") => t!("Import SVG File"),
            Some("dxf") => t!("Import DXF File"),
            _ => t!("Import Design File"),
        };

        let dialog = FileChooserNative::builder()
            .title(title)
            .action(FileChooserAction::Open)
            .modal(true)
            .build();

        if let Some(root) = self.widget.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.set_transient_for(Some(window));
            }
        }

        // Set initial directory from settings
        if let Some(ref settings) = self.settings_persistence {
            if let Ok(settings_ref) = settings.try_borrow() {
                let default_dir = &settings_ref.config().file_processing.output_directory;
                if default_dir.exists() {
                    let file = gtk4::gio::File::for_path(default_dir);
                    let _ = dialog.set_current_folder(Some(&file));
                }
            }
        }

        match kind {
            Some("svg") => {
                let svg_filter = gtk4::FileFilter::new();
                svg_filter.set_name(Some(&t!("SVG Files")));
                svg_filter.add_pattern("*.svg");
                dialog.add_filter(&svg_filter);
            }
            Some("dxf") => {
                let dxf_filter = gtk4::FileFilter::new();
                dxf_filter.set_name(Some(&t!("DXF Files")));
                dxf_filter.add_pattern("*.dxf");
                dialog.add_filter(&dxf_filter);
            }
            _ => {
                let filter = gtk4::FileFilter::new();
                filter.set_name(Some(&t!("Supported Files")));
                filter.add_pattern("*.svg");
                filter.add_pattern("*.dxf");
                dialog.add_filter(&filter);

                let svg_filter = gtk4::FileFilter::new();
                svg_filter.set_name(Some(&t!("SVG Files")));
                svg_filter.add_pattern("*.svg");
                dialog.add_filter(&svg_filter);

                let dxf_filter = gtk4::FileFilter::new();
                dxf_filter.set_name(Some(&t!("DXF Files")));
                dxf_filter.add_pattern("*.dxf");
                dialog.add_filter(&dxf_filter);
            }
        }

        let all_filter = gtk4::FileFilter::new();
        all_filter.set_name(Some(&t!("All Files")));
        all_filter.add_pattern("*");
        dialog.add_filter(&all_filter);

        let canvas = self.canvas.clone();
        let layers = self.layers.clone();
        let status_label = self.status_label.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let result = if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            match ext.to_lowercase().as_str() {
                                "svg" => match std::fs::read_to_string(&path) {
                                    Ok(content) => {
                                        let importer = gcodekit5_designer::import::SvgImporter::new(
                                            1.0, 0.0, 0.0,
                                        );
                                        importer.import_string(&content)
                                    }
                                    Err(e) => Err(anyhow::anyhow!("Failed to read file: {}", e)),
                                },
                                "dxf" => {
                                    let importer =
                                        gcodekit5_designer::import::DxfImporter::new(1.0, 0.0, 0.0);
                                    importer.import_file(path.to_str().unwrap_or(""))
                                }
                                _ => Err(anyhow::anyhow!("Unsupported file format")),
                            }
                        } else {
                            Err(anyhow::anyhow!("Unknown file extension"))
                        };

                        match result {
                            Ok(design) => {
                                let mut state = canvas.state.borrow_mut();

                                // Add imported shapes to canvas
                                for shape in design.shapes {
                                    state.canvas.add_shape(shape);
                                }

                                drop(state);

                                // Make imported geometry visible immediately
                                canvas.zoom_fit();

                                layers.refresh(&canvas.state);
                                canvas.widget.queue_draw();
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Imported:"),
                                    path.display()
                                ));
                            }
                            Err(e) => {
                                error!("Error importing file: {}", e);
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Error importing file:"),
                                    e
                                ));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    pub fn import_file(&self) {
        self.import_file_internal(None);
    }

    pub fn import_svg_file(&self) {
        self.import_file_internal(Some("svg"));
    }

    pub fn import_dxf_file(&self) {
        self.import_file_internal(Some("dxf"));
    }

    pub fn save_file(&self) {
        let current_path = self.current_file.borrow().clone();

        if let Some(path) = current_path {
            self.save_to_path(path);
        } else {
            self.save_as_file();
        }
    }

    pub fn save_as_file(&self) {
        let dialog = FileChooserNative::builder()
            .title("Save Design File")
            .action(FileChooserAction::Save)
            .modal(true)
            .build();

        if let Some(root) = self.widget.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.set_transient_for(Some(window));
            }
        }

        // Set initial directory from settings
        if let Some(ref settings) = self.settings_persistence {
            if let Ok(settings_ref) = settings.try_borrow() {
                let default_dir = &settings_ref.config().file_processing.output_directory;
                if default_dir.exists() {
                    let file = gtk4::gio::File::for_path(default_dir);
                    let _ = dialog.set_current_folder(Some(&file));
                }
            }
        }

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some(&t!("GCodeKit Design Files")));
        filter.add_pattern("*.gckd");
        dialog.add_filter(&filter);

        let canvas = self.canvas.clone();
        let current_file = self.current_file.clone();
        let status_label = self.status_label.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(mut path) = file.path() {
                        if path.extension().is_none() {
                            path.set_extension("gckd");
                        }

                        // Save logic
                        let state = canvas.state.borrow();
                        let mut design =
                            DesignFile::new(path.file_stem().unwrap_or_default().to_string_lossy());

                        // Viewport
                        design.viewport.zoom = state.canvas.zoom();
                        design.viewport.pan_x = state.canvas.pan_x();
                        design.viewport.pan_y = state.canvas.pan_y();

                        // Shapes
                        for obj in state.canvas.shapes() {
                            let shape_data = DesignFile::from_drawing_object(obj);
                            design.shapes.push(shape_data);
                        }

                        match design.save_to_file(&path) {
                            Ok(_) => {
                                *current_file.borrow_mut() = Some(path.clone());
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Saved:"),
                                    path.display()
                                ));
                            }
                            Err(e) => {
                                error!("Error saving file: {}", e);
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Error saving file:"),
                                    e
                                ));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    fn save_to_path(&self, path: PathBuf) {
        let state = self.canvas.state.borrow();
        let mut design = DesignFile::new(path.file_stem().unwrap_or_default().to_string_lossy());

        // Viewport
        design.viewport.zoom = state.canvas.zoom();
        design.viewport.pan_x = state.canvas.pan_x();
        design.viewport.pan_y = state.canvas.pan_y();

        // Shapes
        for obj in state.canvas.shapes() {
            let shape_data = DesignFile::from_drawing_object(obj);
            design.shapes.push(shape_data);
        }

        match design.save_to_file(&path) {
            Ok(_) => {
                self.set_status(&format!("{} {}", t!("Saved:"), path.display()));
            }
            Err(e) => {
                error!("Error saving file: {}", e);
                self.set_status(&format!("{} {}", t!("Error saving file:"), e));
            }
        }
    }

    pub fn export_gcode(&self) {
        let window = self
            .widget
            .root()
            .and_then(|w| w.downcast::<gtk4::Window>().ok());
        let dialog = FileChooserNative::new(
            Some("Export G-Code"),
            window.as_ref(),
            FileChooserAction::Save,
            Some("Export"),
            Some("Cancel"),
        );

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("G-Code Files"));
        filter.add_pattern("*.nc");
        filter.add_pattern("*.gcode");
        filter.add_pattern("*.gc");
        dialog.add_filter(&filter);

        let canvas = self.canvas.clone();
        let status_label = self.status_label.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(mut path) = file.path() {
                        if path.extension().is_none() {
                            path.set_extension("nc");
                        }

                        // Generate G-code
                        let mut state = canvas.state.borrow_mut();

                        // Copy settings to avoid borrow issues
                        let feed_rate = state.tool_settings.feed_rate;
                        let spindle_speed = state.tool_settings.spindle_speed;
                        let tool_diameter = state.tool_settings.tool_diameter;
                        let cut_depth = state.tool_settings.cut_depth;
                        let start_depth = state.tool_settings.start_depth;

                        // Update toolpath generator settings from state
                        state.toolpath_generator.set_feed_rate(feed_rate);
                        state.toolpath_generator.set_spindle_speed(spindle_speed);
                        state.toolpath_generator.set_tool_diameter(tool_diameter);
                        state.toolpath_generator.set_cut_depth(cut_depth);
                        state.toolpath_generator.set_start_depth(start_depth);
                        state.toolpath_generator.set_step_in(tool_diameter * 0.4); // Default stepover

                        let gcode = state.generate_gcode();

                        match std::fs::write(&path, gcode) {
                            Ok(_) => {
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Exported G-Code:"),
                                    path.display()
                                ));
                            }
                            Err(e) => {
                                error!("Error exporting G-Code: {}", e);
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Error exporting G-Code:"),
                                    e
                                ));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    pub fn export_svg(&self) {
        let window = self
            .widget
            .root()
            .and_then(|w| w.downcast::<gtk4::Window>().ok());
        let dialog = FileChooserNative::new(
            Some("Export SVG"),
            window.as_ref(),
            FileChooserAction::Save,
            Some("Export"),
            Some("Cancel"),
        );

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("SVG Files"));
        filter.add_pattern("*.svg");
        dialog.add_filter(&filter);

        let canvas = self.canvas.clone();
        let status_label = self.status_label.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(mut path) = file.path() {
                        if path.extension().is_none() {
                            path.set_extension("svg");
                        }

                        let state = canvas.state.borrow();

                        // Calculate bounds
                        let mut min_x = f64::INFINITY;
                        let mut min_y = f64::INFINITY;
                        let mut max_x = f64::NEG_INFINITY;
                        let mut max_y = f64::NEG_INFINITY;

                        let shapes: Vec<_> = state.canvas.shapes().collect();
                        if shapes.is_empty() {
                            status_label.set_text(&t!("Nothing to export"));
                            dialog.destroy();
                            return;
                        }

                        for obj in &shapes {
                            let (x1, y1, x2, y2) = obj.get_effective_shape().bounds();
                            min_x = min_x.min(x1);
                            min_y = min_y.min(y1);
                            max_x = max_x.max(x2);
                            max_y = max_y.max(y2);
                        }

                        // Add some padding
                        let padding = 10.0;
                        min_x -= padding;
                        min_y -= padding;
                        max_x += padding;
                        max_y += padding;

                        let width = max_x - min_x;
                        let height = max_y - min_y;

                        let mut svg = String::new();
                        svg.push_str(&format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<svg width="{:.2}mm" height="{:.2}mm" viewBox="{:.2} {:.2} {:.2} {:.2}" xmlns="http://www.w3.org/2000/svg">
"#, width, height, min_x, min_y, width, height));

                        for obj in &shapes {
                            let style = "fill:none;stroke:black;stroke-width:0.5";
                            let effective_shape = obj.get_effective_shape();
                            match &effective_shape {
                                Shape::Rectangle(r) => {
                                    let x = r.center.x - r.width / 2.0;
                                    let y = r.center.y - r.height / 2.0;
                                    svg.push_str(&format!(r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        x, y, r.width, r.height, r.corner_radius, style,
                                        r.rotation, r.center.x, r.center.y
                                    ));
                                }
                                Shape::Circle(c) => {
                                    svg.push_str(&format!(r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" style="{}" />"#,
                                        c.center.x, c.center.y, c.radius, style
                                    ));
                                }
                                Shape::Line(l) => {
                                    svg.push_str(&format!(r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        l.start.x, l.start.y, l.end.x, l.end.y, style,
                                        l.rotation, (l.start.x+l.end.x)/2.0, (l.start.y+l.end.y)/2.0
                                    ));
                                }
                                Shape::Triangle(t) => {
                                    let path = t.render();
                                    let d = gcodekit5_designer::model::DesignPath::from_lyon_path(&path).to_svg_path();
                                    svg.push_str(&format!(r#"<path d="{}" style="{}" />"#, d, style));
                                }
                                Shape::Polygon(p) => {
                                    let path = p.render();
                                    let d = gcodekit5_designer::model::DesignPath::from_lyon_path(&path).to_svg_path();
                                    svg.push_str(&format!(r#"<path d="{}" style="{}" />"#, d, style));
                                }
                                Shape::Ellipse(e) => {
                                    svg.push_str(&format!(r#"<ellipse cx="{:.2}" cy="{:.2}" rx="{:.2}" ry="{:.2}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        e.center.x, e.center.y, e.rx, e.ry, style,
                                        e.rotation, e.center.x, e.center.y
                                    ));
                                }
                                Shape::Path(p) => {
                                    let mut d = String::new();
                                    let path = p.render();
                                    for event in path.iter() {
                                        match event {
                                            lyon::path::Event::Begin { at } => d.push_str(&format!("M {:.2} {:.2} ", at.x, at.y)),
                                            lyon::path::Event::Line { from: _, to } => d.push_str(&format!("L {:.2} {:.2} ", to.x, to.y)),
                                            lyon::path::Event::Quadratic { from: _, ctrl, to } => d.push_str(&format!("Q {:.2} {:.2} {:.2} {:.2} ", ctrl.x, ctrl.y, to.x, to.y)),
                                            lyon::path::Event::Cubic { from: _, ctrl1, ctrl2, to } => d.push_str(&format!("C {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} ", ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y)),
                                            lyon::path::Event::End { last: _, first: _, close } => if close { d.push_str("Z "); },
                                        }
                                    }
                                    let rect = lyon::algorithms::aabb::bounding_box(&path);
                                    let cx = (rect.min.x + rect.max.x) / 2.0;
                                    let cy = (rect.min.y + rect.max.y) / 2.0;

                                    svg.push_str(&format!(r#"<path d="{}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        d, style, p.rotation, cx, cy
                                    ));
                                }
                                Shape::Text(t) => {
                                    svg.push_str(&format!(r#"<text x="{:.2}" y="{:.2}" font-size="{:.2}" style="fill:black;stroke:none" transform="rotate({:.2} {:.2} {:.2})">{}</text>"#,
                                        t.x, t.y, t.font_size,
                                        t.rotation, t.x, t.y,
                                        t.text
                                    ));
                                }
                                Shape::Gear(g) => {
                                    let path = g.render();
                                    let d = gcodekit5_designer::model::DesignPath::from_lyon_path(&path).to_svg_path();
                                    svg.push_str(&format!(r#"<path d="{}" style="{}" />"#, d, style));
                                }
                                Shape::Sprocket(s) => {
                                    let path = s.render();
                                    let d = gcodekit5_designer::model::DesignPath::from_lyon_path(&path).to_svg_path();
                                    svg.push_str(&format!(r#"<path d="{}" style="{}" />"#, d, style));
                                }
                                Shape::TabbedBox(b) => {
                                    let paths = b.render_all();
                                    for path in paths {
                                        let d = gcodekit5_designer::model::DesignPath::from_lyon_path(&path).to_svg_path();
                                        svg.push_str(&format!(r#"<path d="{}" style="{}" />"#, d, style));
                                    }
                                }
                            }
                            svg.push('\n');
                        }

                        svg.push_str("</svg>");

                        match std::fs::write(&path, svg) {
                            Ok(_) => {
                                status_label.set_text(&format!("{} {}", t!("Exported SVG:"), path.display()));
                            }
                            Err(e) => {
                                error!("Error exporting SVG: {}", e);
                                status_label.set_text(&format!("{} {}", t!("Error exporting SVG:"), e));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    // File operations - TODO: Implement once shape structures are aligned
    // Phase 8 infrastructure is in place but needs shape struct updates
}
