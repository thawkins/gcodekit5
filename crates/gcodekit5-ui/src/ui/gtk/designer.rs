use gtk4::prelude::*;
use gtk4::{DrawingArea, GestureClick, GestureDrag, EventControllerMotion, EventControllerKey, Box, Label, Orientation, FileChooserAction, FileChooserNative, ResponseType, Grid, Scrollbar, Adjustment, Overlay, Popover, Separator};
use gtk4::gdk::{Key, ModifierType};
use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::shapes::{Shape, Point, Rectangle, Circle, Line, Ellipse, PathShape, OperationType};
use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::commands::{DesignerCommand, RemoveShape, PasteShapes};
use gcodekit5_designer::serialization::DesignFile;
use gcodekit5_designer::toolpath::{Toolpath, ToolpathSegmentType};
use crate::ui::gtk::designer_toolbox::{DesignerToolbox, DesignerTool};
use gcodekit5_devicedb::DeviceManager;
use std::sync::Arc;
use gcodekit5_core::constants as core_constants;
use crate::ui::gtk::designer_properties::PropertiesPanel;
use crate::ui::gtk::designer_layers::LayersPanel;
//use crate::ui::gtk::designer_file_ops; // Temporarily disabled

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
        assert_eq!(bbox, (0.0, 0.0, gcodekit5_core::constants::DEFAULT_WORK_WIDTH_MM, gcodekit5_core::constants::DEFAULT_WORK_HEIGHT_MM));
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
    // Scroll adjustments
    hadjustment: Rc<RefCell<Option<gtk4::Adjustment>>>,
    vadjustment: Rc<RefCell<Option<gtk4::Adjustment>>>,
    // Keyboard state
    shift_pressed: Rc<RefCell<bool>>,
    // Polyline state
    polyline_points: Rc<RefCell<Vec<Point>>>,
    // Toolpath preview
    preview_toolpaths: Rc<RefCell<Vec<Toolpath>>>,
    device_manager: Option<Arc<DeviceManager>>,
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
}

impl DesignerCanvas {
    pub fn new(state: Rc<RefCell<DesignerState>>, toolbox: Option<Rc<DesignerToolbox>>, device_manager: Option<Arc<DeviceManager>>) -> Rc<Self> {
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
        let preview_toolpaths = Rc::new(RefCell::new(Vec::new()));

        let state_clone = state.clone();
        let mouse_pos_clone = mouse_pos.clone();
        let creation_start_clone = creation_start.clone();
        let creation_current_clone = creation_current.clone();
        let polyline_points_clone = polyline_points.clone();
        let preview_toolpaths_clone = preview_toolpaths.clone();
        let device_manager_draw = device_manager.clone();
        
        let state_draw = state_clone.clone();
        widget.set_draw_func(move |_, cr, width, height| {
            // Update viewport size to match widget dimensions
            if let Ok(mut state) = state_draw.try_borrow_mut() {
                state.canvas.set_canvas_size(width as f64, height as f64);
            }

            let state = state_draw.borrow();
            let mouse = *mouse_pos_clone.borrow();
            let preview_start = *creation_start_clone.borrow();
            let preview_current = *creation_current_clone.borrow();
            let poly_points = polyline_points_clone.borrow();
            let toolpaths = preview_toolpaths_clone.borrow();
            let bounds = compute_device_bbox(&device_manager_draw);
            Self::draw(cr, &state, width as f64, height as f64, mouse, preview_start, preview_current, &poly_points, &toolpaths, bounds);
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
            hadjustment: Rc::new(RefCell::new(None)),
            vadjustment: Rc::new(RefCell::new(None)),
            shift_pressed: Rc::new(RefCell::new(false)),
            polyline_points: polyline_points.clone(),
            preview_toolpaths: preview_toolpaths.clone(),
            device_manager: device_manager.clone(),
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
            let tool = canvas_motion.toolbox.as_ref().map(|t| t.current_tool()).unwrap_or(DesignerTool::Select);
            if tool == DesignerTool::Pan {
                if *canvas_motion.did_drag.borrow() {
                     widget_motion.set_cursor_from_name(Some("grabbing"));
                } else {
                     widget_motion.set_cursor_from_name(Some("grab"));
                }
            } else {
                widget_motion.set_cursor(None);
            }

            widget_motion.queue_draw();
        });
        widget.add_controller(motion_ctrl);

        // Mouse wheel zoom
        let scroll_ctrl = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
        let canvas_scroll = canvas.clone();
        scroll_ctrl.connect_scroll(move |_ctrl, _dx, dy| {
            if dy > 0.0 {
                canvas_scroll.zoom_out();
            } else if dy < 0.0 {
                canvas_scroll.zoom_in();
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
        let polyline_points_key = canvas.polyline_points.clone();
        let layers_key = canvas.layers.clone();
        
        key_controller.connect_key_pressed(move |_controller, keyval, _keycode, _modifier| {
            if keyval == gtk4::gdk::Key::Shift_L || keyval == gtk4::gdk::Key::Shift_R {
                *shift_pressed_key.borrow_mut() = true;
                return glib::Propagation::Proceed;
            }

            let mut designer_state = state_key.borrow_mut();
            
            match keyval {
                gtk4::gdk::Key::Delete | gtk4::gdk::Key::BackSpace => {
                    // Delete selected shapes
                    if designer_state.canvas.selection_manager.selected_id().is_some() {
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
        key_controller.connect_key_released(move |_controller, keyval, _keycode, _modifier| {
            if keyval == gtk4::gdk::Key::Shift_L || keyval == gtk4::gdk::Key::Shift_R {
                *shift_released_key.borrow_mut() = false;
            }
        });

        widget.add_controller(key_controller);

        canvas
    }

    /// Fit the canvas to the active device working area (or a 250x250 mm fallback)
    pub fn fit_to_device_area(&self) {
        let (min_x, min_y, max_x, max_y) = compute_device_bbox(&self.device_manager);

        self.state.borrow_mut().canvas.fit_to_bounds(min_x, min_y, max_x, max_y, core_constants::VIEW_PADDING);
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
                let (sx, sy, ex, ey) = obj.shape.bounding_box();
                min_x = min_x.min(sx);
                min_y = min_y.min(sy);
                max_x = max_x.max(ex);
                max_y = max_y.max(ey);
            }

            if has_shapes {
                // Fit content using the viewport fit-to-bounds (5% padding)
                state.canvas.fit_to_bounds(min_x, min_y, max_x, max_y, core_constants::VIEW_PADDING);
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
                        (0.0, 0.0, core_constants::DEFAULT_WORK_WIDTH_MM, core_constants::DEFAULT_WORK_HEIGHT_MM)
                    }
                } else {
                    (0.0, 0.0, core_constants::DEFAULT_WORK_WIDTH_MM, core_constants::DEFAULT_WORK_HEIGHT_MM)
                };

                state.canvas.fit_to_bounds(min_x, min_y, max_x, max_y, core_constants::VIEW_PADDING);
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

        let state = self.state.borrow();
        let has_selection = state.canvas.selection_manager.selected_id().is_some();
        let selected_count = state.canvas.shapes().filter(|s| s.selected).count();
        let can_paste = !state.clipboard.is_empty();
        let can_group = state.can_group();
        let can_ungroup = state.can_ungroup();
        let can_align = selected_count >= 2;
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
                    "cut" => canvas.cut(),
                    "copy" => canvas.copy_selected(),
                    "paste" => canvas.paste(),
                    "delete" => canvas.delete_selected(),
                    "group" => canvas.group_selected(),
                    "ungroup" => canvas.ungroup_selected(),
                    "convert_to_path" => canvas.convert_to_path(),
                    "convert_to_rectangle" => canvas.convert_to_rectangle(),
                    _ => {}
                }
            });
            
            btn
        };

        vbox.append(&create_item("Cut", "cut", has_selection));
        vbox.append(&create_item("Copy", "copy", has_selection));
        vbox.append(&create_item("Paste", "paste", can_paste));
        vbox.append(&create_item("Delete", "delete", has_selection));
        
        vbox.append(&Separator::new(Orientation::Horizontal));
        
        vbox.append(&create_item("Group", "group", can_group));
        vbox.append(&create_item("Ungroup", "ungroup", can_ungroup));

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
        vbox.append(&create_item("Convert to Path", "convert_to_path", has_selection));
        vbox.append(&create_item("Convert to Rectangle", "convert_to_rectangle", has_selection));

        menu.set_child(Some(&vbox));
        menu.popup();
    }

    fn handle_click(&self, x: f64, y: f64, ctrl_pressed: bool, n_press: i32) {
        // Reset drag flag
        *self.did_drag.borrow_mut() = false;

        // Clear properties panel focus when user clicks on canvas
        if let Some(ref props) = *self.properties.borrow() {
            props.clear_focus();
        }
        
        let tool = self.toolbox.as_ref().map(|t| t.current_tool()).unwrap_or(DesignerTool::Select);
        
        // Convert screen coords to canvas coords
        let _width = self.widget.width() as f64;
        let height = self.widget.height() as f64;
        
        let state = self.state.borrow();
        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();
        drop(state);
        
        let y_flipped = height - y;
        let canvas_x = (x - pan_x) / zoom;
        let canvas_y = (y_flipped - pan_y) / zoom;
        
        match tool {
            DesignerTool::Select => {
                // Handle selection
                let mut state = self.state.borrow_mut();
                let point = Point::new(canvas_x, canvas_y);
                
                // Check if we clicked on an existing shape
                let mut clicked_shape_id = None;
                let tolerance = 3.0;
                for obj in state.canvas.shapes() {
                    if obj.shape.contains_point(&point, tolerance) {
                        clicked_shape_id = Some(obj.id);
                    }
                }
                
                if let Some(id) = clicked_shape_id {
                    // Check if it's already selected
                    let is_selected = state.canvas.selection_manager.selected_id() == Some(id) || 
                                      state.canvas.shapes().any(|s| s.id == id && s.selected);
                    
                    if is_selected && !ctrl_pressed {
                        // Clicked on already selected item, and no Ctrl.
                        // Do NOT change selection yet. Wait for release.
                        // This allows dragging the current selection group.
                        return; 
                    }
                }
                
                // Try to select shape at click point with multi-select if Ctrl is held
                if let Some(_selected_id) = state.canvas.select_at(&point, tolerance, ctrl_pressed) {
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
            _ => {
                // Other tools handled by drag
            }
        }
    }
    
    fn handle_release(&self, x: f64, y: f64, ctrl_pressed: bool) {
        if *self.did_drag.borrow() {
            return;
        }
        
        let tool = self.toolbox.as_ref().map(|t| t.current_tool()).unwrap_or(DesignerTool::Select);
        
        // Convert screen coords to canvas coords
        let _width = self.widget.width() as f64;
        let height = self.widget.height() as f64;
        
        let state = self.state.borrow();
        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();
        drop(state);
        
        let y_flipped = height - y;
        let canvas_x = (x - pan_x) / zoom;
        let canvas_y = (y_flipped - pan_y) / zoom;
        
        if tool == DesignerTool::Select {
             let mut state = self.state.borrow_mut();
             let point = Point::new(canvas_x, canvas_y);
             
             // Check if we clicked on an existing shape
             let mut clicked_shape_id = None;
             let tolerance = 3.0;
             for obj in state.canvas.shapes() {
                 if obj.shape.contains_point(&point, tolerance) {
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
        
        let tool = self.toolbox.as_ref().map(|t| t.current_tool()).unwrap_or(DesignerTool::Select);
        
        // Convert screen coords to canvas coords
        let _width = self.widget.width() as f64;
        let height = self.widget.height() as f64;
        
        let state = self.state.borrow();
        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();
        drop(state);
        
        let y_flipped = height - y;
        let canvas_x = (x - pan_x) / zoom;
        let canvas_y = (y_flipped - pan_y) / zoom;
        
        match tool {
            DesignerTool::Select => {
                // Check if we're clicking on a resize handle first
                let (selected_id_opt, bounds_opt) = {
                    let state = self.state.borrow();
                    if let Some(selected_id) = state.canvas.selection_manager.selected_id() {
                        if let Some(obj) = state.canvas.shapes().find(|s| s.id == selected_id) {
                            let bounds = obj.shape.bounding_box();
                            (Some(selected_id), Some(bounds))
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    }
                };
                
                if let (Some(selected_id), Some(bounds)) = (selected_id_opt, bounds_opt) {
                    if let Some(handle) = self.get_resize_handle_at(canvas_x, canvas_y, &bounds) {
                        // Start resizing
                        *self.active_resize_handle.borrow_mut() = Some((handle, selected_id));
                        let (min_x, min_y, max_x, max_y) = bounds;
                        *self.resize_original_bounds.borrow_mut() = Some((
                            min_x, min_y,
                            max_x - min_x,
                            max_y - min_y
                        ));
                        *self.creation_start.borrow_mut() = Some((canvas_x, canvas_y));
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
        let tool = self.toolbox.as_ref().map(|t| t.current_tool()).unwrap_or(DesignerTool::Select);
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
                    let snap_angle = (angle / (std::f64::consts::PI / 4.0)).round() * (std::f64::consts::PI / 4.0);
                    let dist = (dx*dx + dy*dy).sqrt();
                    current_x = start.0 + dist * snap_angle.cos();
                    current_y = start.1 + dist * snap_angle.sin();
                }
            }

            if tool != DesignerTool::Pan {
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
        let tool = self.toolbox.as_ref().map(|t| t.current_tool()).unwrap_or(DesignerTool::Select);
        
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
                    
                    // If we didn't have a selection and we dragged, perform marquee selection
                    let mut state = self.state.borrow_mut();
                    if state.canvas.selection_manager.selected_id().is_none() {
                        // Calculate selection rectangle
                        let min_x = start.0.min(end_x);
                        let max_x = start.0.max(end_x);
                        let min_y = start.1.min(end_y);
                        let max_y = start.1.max(end_y);
                        
                        // Find all shapes intersecting the marquee rectangle
                        let selected_shapes: Vec<_> = state.canvas.shapes()
                            .filter(|obj| {
                                let (shape_min_x, shape_min_y, shape_max_x, shape_max_y) = obj.shape.bounding_box();
                                // Check if bounding boxes intersect
                                !(shape_max_x < min_x || shape_min_x > max_x || 
                                  shape_max_y < min_y || shape_min_y > max_y)
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
                            state.canvas.selection_manager.set_selected_id(selected_shapes.first().copied());
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
                let cx = start.0;
                let cy = start.1;
                let dx = end.0 - start.0;
                let dy = end.1 - start.1;
                let radius = (dx * dx + dy * dy).sqrt();
                
                if radius > 1.0 {
                    Some(Shape::Circle(Circle::new(Point::new(cx, cy), radius)))
                } else {
                    None
                }
            }
            DesignerTool::Line => {
                Some(Shape::Line(Line::new(
                    Point::new(start.0, start.1),
                    Point::new(end.0, end.1),
                )))
            }
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
        let selected_ids: Vec<u64> = state.canvas.shapes()
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
        let selected: Vec<DrawingObject> = state.canvas.shapes()
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
        let selected: Vec<DrawingObject> = state.canvas.shapes()
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
        let selected: Vec<DrawingObject> = state.canvas.shapes()
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
                if r == 0 && c == 0 { continue; } // Skip original position
                
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

    pub fn create_circular_array(&self, count: usize, center_x: f64, center_y: f64, total_angle: f64) {
        let mut state = self.state.borrow_mut();
        let selected: Vec<DrawingObject> = state.canvas.shapes()
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
    
    pub fn copy_selected(&self) {
        let mut state = self.state.borrow_mut();
        state.clipboard = state.canvas.shapes()
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
        let mut state = self.state.borrow_mut();
        let mut toolpaths = Vec::new();
        
        // Copy settings to avoid borrow issues
        let feed_rate = state.tool_settings.feed_rate;
        let spindle_speed = state.tool_settings.spindle_speed;
        let tool_diameter = state.tool_settings.tool_diameter;
        let cut_depth = state.tool_settings.cut_depth;
        
        // Update toolpath generator settings from state
        state.toolpath_generator.set_feed_rate(feed_rate);
        state.toolpath_generator.set_spindle_speed(spindle_speed);
        state.toolpath_generator.set_tool_diameter(tool_diameter);
        state.toolpath_generator.set_cut_depth(cut_depth);
        state.toolpath_generator.set_step_in(tool_diameter * 0.4); // Default stepover
        
        let shapes: Vec<_> = state.canvas.shapes().cloned().collect();
        
        for shape in shapes {
            // Set strategy for this shape
            state.toolpath_generator.set_pocket_strategy(shape.pocket_strategy);

            let shape_toolpaths = match &shape.shape {
                Shape::Rectangle(rect) => {
                    if shape.operation_type == OperationType::Pocket {
                        state.toolpath_generator.generate_rectangle_pocket(
                            rect,
                            shape.pocket_depth,
                            shape.step_down as f64,
                            shape.step_in as f64,
                        )
                    } else {
                        vec![state.toolpath_generator.generate_rectangle_contour(rect)]
                    }
                }
                Shape::Circle(circle) => {
                    if shape.operation_type == OperationType::Pocket {
                        state.toolpath_generator.generate_circle_pocket(
                            circle,
                            shape.pocket_depth,
                            shape.step_down as f64,
                            shape.step_in as f64,
                        )
                    } else {
                        vec![state.toolpath_generator.generate_circle_contour(circle)]
                    }
                }
                Shape::Line(line) => {
                    vec![state.toolpath_generator.generate_line_contour(line)]
                }
                Shape::Ellipse(ellipse) => {
                    let (x1, y1, x2, y2) = ellipse.bounding_box();
                    let cx = (x1 + x2) / 2.0;
                    let cy = (y1 + y2) / 2.0;
                    let radius = ((x2 - x1).abs().max((y2 - y1).abs())) / 2.0;
                    let circle = Circle::new(Point::new(cx, cy), radius);
                    vec![state.toolpath_generator.generate_circle_contour(&circle)]
                }
                Shape::Path(path_shape) => {
                    if shape.operation_type == OperationType::Pocket {
                        state.toolpath_generator.generate_path_pocket(
                            path_shape,
                            shape.pocket_depth,
                            shape.step_down as f64,
                            shape.step_in as f64,
                        )
                    } else {
                        vec![state.toolpath_generator.generate_path_contour(path_shape)]
                    }
                }
                Shape::Text(text) => {
                    vec![state.toolpath_generator.generate_text_toolpath(text)]
                }
            };
            toolpaths.extend(shape_toolpaths);
        }
        
        drop(state);
        *self.preview_toolpaths.borrow_mut() = toolpaths;
        self.widget.queue_draw();
    }

    fn draw(cr: &gtk4::cairo::Context, state: &DesignerState, width: f64, height: f64, mouse_pos: (f64, f64), preview_start: Option<(f64, f64)>, preview_current: Option<(f64, f64)>, polyline_points: &[Point], toolpaths: &[Toolpath], device_bounds: (f64, f64, f64, f64)) {
        // Clear background
        cr.set_source_rgb(0.95, 0.95, 0.95); // Light grey background
        cr.paint().expect("Invalid cairo surface state");

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
            Self::draw_grid(cr, width, height);
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
            cr.set_line_width(1.0 / zoom); // Constant screen width
            
            for toolpath in toolpaths {
                for segment in &toolpath.segments {
                    match segment.segment_type {
                        ToolpathSegmentType::RapidMove => {
                            cr.set_source_rgba(1.0, 0.0, 0.0, 0.5); // Red for rapid
                            cr.set_dash(&[2.0 / zoom, 2.0 / zoom], 0.0);
                        }
                        ToolpathSegmentType::LinearMove | ToolpathSegmentType::ArcMove => {
                            cr.set_source_rgba(0.0, 0.8, 0.0, 0.7); // Green for cut
                            cr.set_dash(&[], 0.0);
                        }
                    }
                    
                    cr.move_to(segment.start.x, segment.start.y);
                    cr.line_to(segment.end.x, segment.end.y);
                    cr.stroke().unwrap();
                }
            }
            
            cr.restore().unwrap();
        }
        
        // Draw polyline in progress
        if !polyline_points.is_empty() {
            cr.save().unwrap();
            cr.set_source_rgb(0.0, 0.0, 1.0); // Blue for creation
            cr.set_line_width(1.5);
            
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
                cr.arc(p.x, p.y, 3.0, 0.0, 2.0 * std::f64::consts::PI);
                cr.fill().unwrap();
            }
            
            cr.restore().unwrap();
        }

        // Draw Shapes
        for obj in state.canvas.shape_store.iter() {
            cr.save().unwrap();
            
            if obj.selected {
                cr.set_source_rgb(1.0, 0.0, 0.0); // Red for selected
                cr.set_line_width(2.0);
            } else if obj.group_id.is_some() {
                cr.set_source_rgb(0.0, 0.5, 0.0); // Green for grouped
                cr.set_line_width(1.0);
            } else {
                cr.set_source_rgb(0.0, 0.0, 0.0); // Black for normal
                cr.set_line_width(1.0);
            }

            match &obj.shape {
                Shape::Rectangle(rect) => {
                    // Rectangle center is x,y
                    // Cairo rectangle is x,y,w,h (top-left)
                    // But we are in Y-up, so top-left is x, y+h? No, Cairo rect is x,y,w,h.
                    // If we scaled Y by -1, then +Y is up.
                    // Rectangle struct has x,y as center? No, usually bottom-left or top-left.
                    // Let's assume x,y is bottom-left for Cartesian.
                    
                    // Checking Rectangle definition:
                    // pub struct Rectangle { pub x: f64, pub y: f64, pub width: f64, pub height: f64, ... }
                    // Usually x,y is bottom-left in G-code context.
                    
                    cr.rectangle(rect.x, rect.y, rect.width, rect.height);
                    cr.stroke().unwrap();
                }
                Shape::Circle(circle) => {
                    cr.arc(circle.center.x, circle.center.y, circle.radius, 0.0, 2.0 * std::f64::consts::PI);
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
                    // TODO: Rotation
                    cr.scale(ellipse.rx, ellipse.ry);
                    cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.restore().unwrap();
                    cr.stroke().unwrap();
                }
                Shape::Path(path_shape) => {
                    // Iterate lyon path
                    for event in path_shape.path.iter() {
                        match event {
                            lyon::path::Event::Begin { at } => {
                                cr.move_to(at.x as f64, at.y as f64);
                            }
                            lyon::path::Event::Line { from: _, to } => {
                                cr.line_to(to.x as f64, to.y as f64);
                            }
                            lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                                // Cairo doesn't have quadratic, convert to cubic
                                // CP1 = from + 2/3 * (ctrl - from)
                                // CP2 = to + 2/3 * (ctrl - to)
                                // But we don't have 'from' easily available in all events? 
                                // Lyon Event::Quadratic gives 'from'.
                                // Wait, cairo has curve_to (cubic).
                                // Q to C conversion:
                                // q0 = from, q1 = ctrl, q2 = to
                                // c0 = q0
                                // c1 = q0 + (2/3)(q1 - q0)
                                // c2 = q2 + (2/3)(q1 - q2)
                                // c3 = q2
                                // cr.curve_to(c1.x, c1.y, c2.x, c2.y, c3.x, c3.y)
                                // Actually we can just use current point as from.
                                let (x0, y0) = cr.current_point().unwrap();
                                let x1 = x0 + (2.0/3.0) * (ctrl.x as f64 - x0);
                                let y1 = y0 + (2.0/3.0) * (ctrl.y as f64 - y0);
                                let x2 = to.x as f64 + (2.0/3.0) * (ctrl.x as f64 - to.x as f64);
                                let y2 = to.y as f64 + (2.0/3.0) * (ctrl.y as f64 - to.y as f64);
                                cr.curve_to(x1, y1, x2, y2, to.x as f64, to.y as f64);
                            }
                            lyon::path::Event::Cubic { from: _, ctrl1, ctrl2, to } => {
                                cr.curve_to(ctrl1.x as f64, ctrl1.y as f64, ctrl2.x as f64, ctrl2.y as f64, to.x as f64, to.y as f64);
                            }
                            lyon::path::Event::End { last: _, first: _, close } => {
                                if close {
                                    cr.close_path();
                                }
                            }
                        }
                    }
                    cr.stroke().unwrap();
                }
                Shape::Text(text) => {
                    // Basic text placeholder
                    cr.save().unwrap();
                    // Flip Y back for text so it's not upside down
                    cr.translate(text.x, text.y);
                    cr.scale(1.0, -1.0); 
                    cr.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Normal);
                    cr.set_font_size(text.font_size);
                    cr.show_text(&text.text).unwrap();
                    cr.restore().unwrap();
                }
            }
            
            // Draw resize handles for selected shapes
            if obj.selected {
                let bounds = obj.shape.bounding_box();
                Self::draw_resize_handles(cr, &bounds);
            }
            
            cr.restore().unwrap();
        }
        
        // Draw preview marquee if creating a shape
        if let (Some(start), Some(current)) = (preview_start, preview_current) {
            cr.save().unwrap();
            
            // Draw dashed preview outline
            cr.set_source_rgba(0.5, 0.7, 1.0, 0.7); // Light blue semi-transparent
            cr.set_line_width(1.5);
            cr.set_dash(&[5.0, 5.0], 0.0); // Dashed line
            
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

    fn draw_grid(cr: &gtk4::cairo::Context, width: f64, height: f64) {
        cr.save().unwrap();
        
        // Grid spacing in mm (10mm major grid)
        let grid_spacing = 10.0;
        let minor_spacing = grid_spacing / 5.0; // 2mm minor grid
        
        // Get current transform to find canvas bounds
        let matrix = cr.matrix();
        let x0 = -matrix.x0() / matrix.xx();
        let x1 = (width - matrix.x0()) / matrix.xx();
        let y0 = -matrix.y0() / matrix.yy();
        let y1 = (height - matrix.y0()) / matrix.yy();
        
        // Minor grid lines (lighter)
        cr.set_source_rgba(0.85, 0.85, 0.85, 0.5);
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
        cr.set_source_rgba(0.7, 0.7, 0.7, 0.6);
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
        cr.set_source_rgba(0.3, 0.3, 0.3, 0.8);
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
    
    fn get_resize_handle_at(&self, x: f64, y: f64, bounds: &(f64, f64, f64, f64)) -> Option<ResizeHandle> {
        const HANDLE_SIZE: f64 = 8.0;
        const HANDLE_TOLERANCE: f64 = HANDLE_SIZE / 2.0;
        
        let (min_x, min_y, max_x, max_y) = *bounds;
        
        let corners = [
            (min_x, max_y, ResizeHandle::TopLeft),      // Top-left (Y-up coords)
            (max_x, max_y, ResizeHandle::TopRight),     // Top-right
            (min_x, min_y, ResizeHandle::BottomLeft),   // Bottom-left
            (max_x, min_y, ResizeHandle::BottomRight),  // Bottom-right
        ];
        
        for (cx, cy, handle) in corners {
            let dx = x - cx;
            let dy = y - cy;
            if dx.abs() <= HANDLE_TOLERANCE && dy.abs() <= HANDLE_TOLERANCE {
                return Some(handle);
            }
        }
        
        None
    }
    
    fn apply_resize(&self, handle: ResizeHandle, shape_id: u64, current_x: f64, current_y: f64, shift_pressed: bool) {
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
            let ratio = if orig_height.abs() > 0.001 { orig_width / orig_height } else { 1.0 };
            
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
        let (new_x, new_y, new_width, new_height) = match handle {
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
        
        // Update the shape
        let mut state = self.state.borrow_mut();
        
        if let Some(obj) = state.canvas.shape_store.get_mut(shape_id) {
            match &mut obj.shape {
                Shape::Rectangle(rect) => {
                    rect.x = new_x;
                    rect.y = new_y;
                    rect.width = new_width;
                    rect.height = new_height;
                }
                Shape::Circle(circle) => {
                    // For circles, resize by adjusting radius
                    let center_x = new_x + new_width / 2.0;
                    let center_y = new_y + new_height / 2.0;
                    let radius = (new_width.min(new_height) / 2.0).abs();
                    circle.center = Point::new(center_x, center_y);
                    circle.radius = radius;
                }
                Shape::Ellipse(ellipse) => {
                    let center_x = new_x + new_width / 2.0;
                    let center_y = new_y + new_height / 2.0;
                    ellipse.center = Point::new(center_x, center_y);
                    ellipse.rx = (new_width / 2.0).abs();
                    ellipse.ry = (new_height / 2.0).abs();
                }
                Shape::Line(line) => {
                    // Resize line by moving end points
                    match handle {
                        ResizeHandle::TopLeft | ResizeHandle::BottomLeft => {
                            line.start.x = new_x;
                            line.start.y = if handle == ResizeHandle::TopLeft {
                                new_y + new_height
                            } else {
                                new_y
                            };
                        }
                        ResizeHandle::TopRight | ResizeHandle::BottomRight => {
                            line.end.x = new_x + new_width;
                            line.end.y = if handle == ResizeHandle::TopRight {
                                new_y + new_height
                            } else {
                                new_y
                            };
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    fn draw_resize_handles(cr: &gtk4::cairo::Context, bounds: &(f64, f64, f64, f64)) {
        const HANDLE_SIZE: f64 = 8.0;
        const HALF_SIZE: f64 = HANDLE_SIZE / 2.0;
        
        let (min_x, min_y, max_x, max_y) = *bounds;
        
        cr.save().unwrap();
        
        // Draw handles at corners
        let corners = [
            (min_x, max_y),  // Top-left (Y-up)
            (max_x, max_y),  // Top-right
            (min_x, min_y),  // Bottom-left
            (max_x, min_y),  // Bottom-right
        ];
        
        for (cx, cy) in corners {
            // Draw white fill
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.rectangle(cx - HALF_SIZE, cy - HALF_SIZE, HANDLE_SIZE, HANDLE_SIZE);
            cr.fill().unwrap();
            
            // Draw blue border
            cr.set_source_rgb(0.0, 0.5, 1.0);
            cr.set_line_width(1.5);
            cr.rectangle(cx - HALF_SIZE, cy - HALF_SIZE, HANDLE_SIZE, HANDLE_SIZE);
            cr.stroke().unwrap();
        }
        
        cr.restore().unwrap();
    }
}

impl DesignerView {
    pub fn new(device_manager: Option<Arc<DeviceManager>>) -> Rc<Self> {
        let container = Box::new(Orientation::Vertical, 0);
        container.set_hexpand(true);
        container.set_vexpand(true);
        
        // Create designer state
        let state = Rc::new(RefCell::new(DesignerState::new()));
        
        // Create main horizontal layout (toolbox + canvas + properties)
        let main_box = Box::new(Orientation::Horizontal, 0);
        main_box.set_hexpand(true);
        main_box.set_vexpand(true);
        
        // Create toolbox
        let toolbox = DesignerToolbox::new(state.clone());
        main_box.append(&toolbox.widget);
        
        // Create canvas
        let canvas = DesignerCanvas::new(state.clone(), Some(toolbox.clone()), device_manager.clone());
        
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
        floating_box.set_halign(gtk4::Align::End);
        floating_box.set_valign(gtk4::Align::End);
        floating_box.set_margin_bottom(20);
        floating_box.set_margin_end(20);

        let float_zoom_out = gtk4::Button::builder()
            .label("-")
            .tooltip_text("Zoom Out")
            .build();
        let float_fit = gtk4::Button::builder()
            .label("Fit")
            .tooltip_text("Fit to View")
            .build();
        let float_fit_device = gtk4::Button::builder()
            .icon_name("preferences-desktop-display-symbolic")
            .tooltip_text("Fit to Device Working Area")
            .build();
        let float_zoom_in = gtk4::Button::builder()
            .label("+")
            .tooltip_text("Zoom In")
            .build();

        floating_box.append(&float_zoom_out);
        floating_box.append(&float_fit);
        if device_manager.is_some() {
            floating_box.append(&float_fit_device);
        }
        floating_box.append(&float_zoom_in);

        overlay.add_overlay(&floating_box);

        // Status Panel (Bottom Left)
        let status_box = Box::new(Orientation::Horizontal, 4);
        status_box.add_css_class("visualizer-osd");
        status_box.set_halign(gtk4::Align::Start);
        status_box.set_valign(gtk4::Align::End);
        status_box.set_margin_bottom(20);
        status_box.set_margin_start(20);

        let status_label_osd = Label::builder()
            .label("100%   X: 0.0   Y: 0.0   10.0mm")
            .build();
        status_box.append(&status_label_osd);

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

        let canvas_fitdev = canvas.clone();
        float_fit_device.connect_clicked(move |_| {
            canvas_fitdev.fit_to_device_area();
            canvas_fitdev.widget.queue_draw();
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
        right_sidebar.set_width_request(250);
        
        // Create properties panel
        let properties = PropertiesPanel::new(state.clone());
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
        
        right_sidebar.append(&properties.widget);
        
        // Create layers panel below properties
        let layers = Rc::new(LayersPanel::new(state.clone()));
        layers.widget.set_vexpand(false);
        layers.widget.set_valign(gtk4::Align::Fill);
        right_sidebar.append(&layers.widget);
        
        // Give canvas references to panels
        canvas.set_properties_panel(properties.clone());
        canvas.set_layers_panel(layers.clone());
        
        main_box.append(&right_sidebar);
        
        container.append(&main_box);
        
        // Status bar at bottom
        let status_bar = Box::new(Orientation::Horizontal, 10);
        status_bar.set_margin_start(10);
        status_bar.set_margin_end(10);
        status_bar.set_margin_top(5);
        status_bar.set_margin_bottom(5);
        status_bar.add_css_class("statusbar");
        
        let status_label = Label::new(Some("Ready"));
        status_label.set_halign(gtk4::Align::Start);
        status_label.set_hexpand(true);
        status_bar.append(&status_label);
        
        // Grid toggle
        let grid_toggle = gtk4::CheckButton::with_label("Show Grid");
        grid_toggle.set_active(true);
        let state_grid = state.clone();
        let canvas_grid = canvas.clone();
        grid_toggle.connect_toggled(move |btn| {
            state_grid.borrow_mut().show_grid = btn.is_active();
            canvas_grid.widget.queue_draw();
        });
        status_bar.append(&grid_toggle);

        // Toolpath toggle
        let toolpath_toggle = gtk4::CheckButton::with_label("Show Toolpaths");
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
        status_bar.append(&toolpath_toggle);
        
        // Coordinate display
        let coord_label = Label::new(Some("X: 0.00  Y: 0.00"));
        coord_label.set_halign(gtk4::Align::End);
        coord_label.add_css_class("monospace");
        status_bar.append(&coord_label);
        
        container.append(&status_bar);
        
        // Update coordinate label on mouse move
        let coord_label_clone = coord_label.clone();
        let canvas_coord = canvas.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            let (x, y) = *canvas_coord.mouse_pos.borrow();
            coord_label_clone.set_text(&format!("X: {:.2}  Y: {:.2}", x, y));
            gtk4::glib::ControlFlow::Continue
        });

        // Update status OSD
        let status_osd_clone = status_label_osd.clone();
        let canvas_osd = canvas.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let state = canvas_osd.state.borrow();
            let zoom = state.canvas.zoom();
            let pan_x = state.canvas.pan_x();
            let pan_y = state.canvas.pan_y();
            
            status_osd_clone.set_text(&format!(
                "{:.0}%   X: {:.1}   Y: {:.1}   10.0mm",
                zoom * 100.0,
                pan_x,
                pan_y
            ));
            gtk4::glib::ControlFlow::Continue
        });
        
        let current_file = Rc::new(RefCell::new(None));
        let on_gcode_generated: Rc<RefCell<Option<std::boxed::Box<dyn Fn(String)>>>> = Rc::new(RefCell::new(None));

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
            
            // Update toolpath generator settings from state
            state.toolpath_generator.set_feed_rate(feed_rate);
            state.toolpath_generator.set_spindle_speed(spindle_speed);
            state.toolpath_generator.set_tool_diameter(tool_diameter);
            state.toolpath_generator.set_cut_depth(cut_depth);
            state.toolpath_generator.set_step_in(tool_diameter * 0.4); // Default stepover
            
            let gcode = state.generate_gcode();
            drop(state);
            
            status_label_gen.set_text("G-Code generated");
            
            if let Some(callback) = on_gen.borrow().as_ref() {
                callback(gcode);
            }
        });

        let view = Rc::new(Self {
            widget: container,
            canvas: canvas.clone(),
            toolbox,
            _properties: properties.clone(),
            layers: layers.clone(),
            status_label,
            _coord_label: coord_label,
            current_file,
            on_gcode_generated,
        });
        
        // Update properties panel when selection changes
        let props_update = properties.clone();
        let _canvas_props = canvas.clone();
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
                _ => gtk4::glib::Propagation::Proceed
            }
        });
        
        // Add keyboard controller to canvas so it receives focus
        canvas.widget.set_focusable(true);
        canvas.widget.set_can_focus(true);
        canvas.widget.add_controller(key_controller);
        
        // Grab focus on canvas when clicked
        let canvas_focus = canvas.clone();
        let click_for_focus = GestureClick::new();
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
        self.set_status("New design created");
    }

    pub fn open_file(&self) {
        let dialog = FileChooserNative::builder()
            .title("Open Design File")
            .action(FileChooserAction::Open)
            .modal(true)
            .build();
            
        if let Some(root) = self.widget.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.set_transient_for(Some(window));
            }
        }
        
        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("GCodeKit Design Files"));
        filter.add_pattern("*.gckd");
        filter.add_pattern("*.gck5");
        dialog.add_filter(&filter);
        
        let all_filter = gtk4::FileFilter::new();
        all_filter.set_name(Some("All Files"));
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
                                for shape_data in design.shapes {
                                    let id = shape_data.id as u64;
                                    if id > max_id { max_id = id; }
                                    
                                    if let Ok(obj) = DesignFile::to_drawing_object(&shape_data, id as i32) {
                                        state.canvas.restore_shape(obj);
                                    }
                                }
                                
                                state.canvas.set_next_id(max_id + 1);
                                
                                // Update viewport
                                state.canvas.set_zoom(design.viewport.zoom);
                                state.canvas.set_pan(design.viewport.pan_x, design.viewport.pan_y);
                                
                                *current_file.borrow_mut() = Some(path.clone());
                                drop(state);
                                
                                layers.refresh(&canvas.state);
                                canvas.widget.queue_draw();
                                status_label.set_text(&format!("Loaded: {}", path.display()));
                            }
                            Err(e) => {
                                eprintln!("Error loading file: {}", e);
                                status_label.set_text(&format!("Error loading file: {}", e));
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
        let dialog = FileChooserNative::builder()
            .title("Import Design File")
            .action(FileChooserAction::Open)
            .modal(true)
            .build();
            
        if let Some(root) = self.widget.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.set_transient_for(Some(window));
            }
        }
        
        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("Supported Files"));
        filter.add_pattern("*.svg");
        filter.add_pattern("*.dxf");
        dialog.add_filter(&filter);
        
        let svg_filter = gtk4::FileFilter::new();
        svg_filter.set_name(Some("SVG Files"));
        svg_filter.add_pattern("*.svg");
        dialog.add_filter(&svg_filter);

        let dxf_filter = gtk4::FileFilter::new();
        dxf_filter.set_name(Some("DXF Files"));
        dxf_filter.add_pattern("*.dxf");
        dialog.add_filter(&dxf_filter);
        
        let canvas = self.canvas.clone();
        let layers = self.layers.clone();
        let status_label = self.status_label.clone();
        
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let result = if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            match ext.to_lowercase().as_str() {
                                "svg" => {
                                    match std::fs::read_to_string(&path) {
                                        Ok(content) => {
                                            let importer = gcodekit5_designer::import::SvgImporter::new(1.0, 0.0, 0.0);
                                            importer.import_string(&content)
                                        },
                                        Err(e) => Err(anyhow::anyhow!("Failed to read file: {}", e)),
                                    }
                                },
                                "dxf" => {
                                    let importer = gcodekit5_designer::import::DxfImporter::new(1.0, 0.0, 0.0);
                                    importer.import_file(path.to_str().unwrap_or(""))
                                },
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
                                
                                layers.refresh(&canvas.state);
                                canvas.widget.queue_draw();
                                status_label.set_text(&format!("Imported: {}", path.display()));
                            }
                            Err(e) => {
                                eprintln!("Error importing file: {}", e);
                                status_label.set_text(&format!("Error importing file: {}", e));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });
        
        dialog.show();
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
        
        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("GCodeKit Design Files"));
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
                                *current_file.borrow_mut() = Some(path.clone());
                                status_label.set_text(&format!("Saved: {}", path.display()));
                            }
                            Err(e) => {
                                eprintln!("Error saving file: {}", e);
                                status_label.set_text(&format!("Error saving file: {}", e));
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
                self.set_status(&format!("Saved: {}", path.display()));
            }
            Err(e) => {
                eprintln!("Error saving file: {}", e);
                self.set_status(&format!("Error saving file: {}", e));
            }
        }
    }

    pub fn export_gcode(&self) {
        let window = self.widget.root().and_then(|w| w.downcast::<gtk4::Window>().ok());
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
                        
                        // Update toolpath generator settings from state
                        state.toolpath_generator.set_feed_rate(feed_rate);
                        state.toolpath_generator.set_spindle_speed(spindle_speed);
                        state.toolpath_generator.set_tool_diameter(tool_diameter);
                        state.toolpath_generator.set_cut_depth(cut_depth);
                        state.toolpath_generator.set_step_in(tool_diameter * 0.4); // Default stepover
                        
                        let gcode = state.generate_gcode();
                        
                        match std::fs::write(&path, gcode) {
                            Ok(_) => {
                                status_label.set_text(&format!("Exported G-Code: {}", path.display()));
                            }
                            Err(e) => {
                                eprintln!("Error exporting G-Code: {}", e);
                                status_label.set_text(&format!("Error exporting G-Code: {}", e));
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
        let window = self.widget.root().and_then(|w| w.downcast::<gtk4::Window>().ok());
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
                            status_label.set_text("Nothing to export");
                            dialog.destroy();
                            return;
                        }

                        for obj in &shapes {
                            let (x1, y1, x2, y2) = obj.shape.bounding_box();
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
                            match &obj.shape {
                                Shape::Rectangle(r) => {
                                    svg.push_str(&format!(r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        r.x, r.y, r.width, r.height, r.corner_radius, style,
                                        r.rotation, r.x + r.width/2.0, r.y + r.height/2.0
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
                                Shape::Ellipse(e) => {
                                    svg.push_str(&format!(r#"<ellipse cx="{:.2}" cy="{:.2}" rx="{:.2}" ry="{:.2}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        e.center.x, e.center.y, e.rx, e.ry, style,
                                        e.rotation, e.center.x, e.center.y
                                    ));
                                }
                                Shape::Path(p) => {
                                    let mut d = String::new();
                                    for event in p.path.iter() {
                                        match event {
                                            lyon::path::Event::Begin { at } => d.push_str(&format!("M {:.2} {:.2} ", at.x, at.y)),
                                            lyon::path::Event::Line { from: _, to } => d.push_str(&format!("L {:.2} {:.2} ", to.x, to.y)),
                                            lyon::path::Event::Quadratic { from: _, ctrl, to } => d.push_str(&format!("Q {:.2} {:.2} {:.2} {:.2} ", ctrl.x, ctrl.y, to.x, to.y)),
                                            lyon::path::Event::Cubic { from: _, ctrl1, ctrl2, to } => d.push_str(&format!("C {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} ", ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y)),
                                            lyon::path::Event::End { last: _, first: _, close } => if close { d.push_str("Z "); },
                                        }
                                    }
                                    let rect = lyon::algorithms::aabb::bounding_box(&p.path);
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
                            }
                            svg.push('\n');
                        }
                        
                        svg.push_str("</svg>");
                        
                        match std::fs::write(&path, svg) {
                            Ok(_) => {
                                status_label.set_text(&format!("Exported SVG: {}", path.display()));
                            }
                            Err(e) => {
                                eprintln!("Error exporting SVG: {}", e);
                                status_label.set_text(&format!("Error exporting SVG: {}", e));
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
