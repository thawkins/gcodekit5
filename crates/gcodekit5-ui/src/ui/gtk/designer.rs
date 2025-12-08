use gtk4::prelude::*;
use gtk4::{DrawingArea, GestureClick, GestureDrag, EventControllerMotion, EventControllerKey, Box, Label, Orientation, FileChooserAction, FileChooserNative, ResponseType, Grid, Scrollbar, Adjustment, Overlay, Popover, Separator};
use gtk4::gdk::{Key, ModifierType};
use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::shapes::{Shape, Point, Rectangle, Circle, Line, Ellipse, PathShape};
use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::commands::{DesignerCommand, RemoveShape, PasteShapes};
use gcodekit5_designer::serialization::DesignFile;
use crate::ui::gtk::designer_toolbox::{DesignerToolbox, DesignerTool};
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
}

impl DesignerCanvas {
    pub fn new(state: Rc<RefCell<DesignerState>>, toolbox: Option<Rc<DesignerToolbox>>) -> Rc<Self> {
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

        let state_clone = state.clone();
        let mouse_pos_clone = mouse_pos.clone();
        let creation_start_clone = creation_start.clone();
        let creation_current_clone = creation_current.clone();
        let polyline_points_clone = polyline_points.clone();
        
        let state_draw = state_clone.clone();
        widget.set_draw_func(move |_, cr, width, height| {
            let state = state_draw.borrow();
            let mouse = *mouse_pos_clone.borrow();
            let preview_start = *creation_start_clone.borrow();
            let preview_current = *creation_current_clone.borrow();
            let poly_points = polyline_points_clone.borrow();
            Self::draw(cr, &state, width as f64, height as f64, mouse, preview_start, preview_current, &poly_points);
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
        });

        // Mouse motion tracking
        let motion_ctrl = EventControllerMotion::new();
        let mouse_pos_motion = mouse_pos.clone();
        let widget_motion = widget.clone();
        let state_motion = state_clone.clone();
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
            // Screen Y is top-down.
            // We translated to (0, height) and scaled (1, -1).
            // Then translated (pan_x, pan_y).
            // Then scaled (zoom, zoom).
            
            // Reverse transformation:
            // 1. Screen Y to Bottom-Up Y: y_flipped = height - y
            // 2. Remove Pan: x_panned = x - pan_x, y_panned = y_flipped - pan_y
            // 3. Remove Zoom: cx = x_panned / zoom, cy = y_panned / zoom
            
            let y_flipped = height - y;
            let canvas_x = (x - pan_x) / zoom;
            let canvas_y = (y_flipped - pan_y) / zoom;
            
            *mouse_pos_motion.borrow_mut() = (canvas_x, canvas_y);
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
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;
            
            let mut has_shapes = false;
            for obj in state.canvas.shapes() {
                has_shapes = true;
                let (sx, sy, ex, ey) = obj.shape.bounding_box();
                min_x = min_x.min(sx);
                min_y = min_y.min(sy);
                max_x = max_x.max(ex);
                max_y = max_y.max(ey);
            }
            
            if !has_shapes {
                // Reset to default
                state.canvas.set_zoom(1.0);
                state.canvas.set_pan(0.0, 0.0);
                (0.0, 0.0)
            } else {
                // Add margin
                let margin = 20.0;
                min_x -= margin;
                min_y -= margin;
                max_x += margin;
                max_y += margin;
                
                let content_width = max_x - min_x;
                let content_height = max_y - min_y;
                
                let view_width = self.widget.width() as f64;
                let view_height = self.widget.height() as f64;
                
                if content_width > 0.0 && content_height > 0.0 {
                    let zoom_x = view_width / content_width;
                    let zoom_y = view_height / content_height;
                    let new_zoom = zoom_x.min(zoom_y);
                    
                    state.canvas.set_zoom(new_zoom);
                    
                    let cx = (min_x + max_x) / 2.0;
                    let cy = (min_y + max_y) / 2.0;
                    
                    let pan_x = (view_width / 2.0) - (cx * new_zoom);
                    let pan_y = (view_height / 2.0) - (cy * new_zoom);
                    
                    state.canvas.set_pan(pan_x, pan_y);
                    (pan_x, pan_y)
                } else {
                    (state.canvas.pan_x(), state.canvas.pan_y())
                }
            }
        }; // state borrow dropped here
        
        // Update adjustments safely
        if let Some(adj) = self.hadjustment.borrow().as_ref() {
            adj.set_value(-target_pan_x);
        }
        if let Some(adj) = self.vadjustment.borrow().as_ref() {
            adj.set_value(-target_pan_y);
        }
        
        self.widget.queue_draw();
    }

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
        let can_paste = !state.clipboard.is_empty();
        let can_group = state.can_group();
        let can_ungroup = state.can_ungroup();
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
                for obj in state.canvas.shapes() {
                    if obj.shape.contains_point(&point) {
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
                if let Some(_selected_id) = state.canvas.select_at(&point, ctrl_pressed) {
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
             for obj in state.canvas.shapes() {
                 if obj.shape.contains_point(&point) {
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

            *self.creation_current.borrow_mut() = Some((current_x, current_y));
            
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
                                // Constrain to X axis: cancel Y movement
                                // We need to cancel the accumulated Y movement.
                                // But we only control the incremental delta here.
                                // This is tricky because we've already applied previous deltas.
                                // Ideally we would reset position to start + constrained total delta.
                                // But `move_selected` is relative.
                                
                                // Workaround: If shift is pressed, we only allow movement in the dominant axis.
                                // But this only works if shift was pressed from the start.
                                // If shift is pressed mid-drag, we might want to snap back.
                                
                                // For now, let's just zero out the minor axis delta.
                                // This gives "Manhattan" movement but doesn't snap back if you drift.
                                // To do it properly, we'd need to store original positions of all selected items.
                                
                                // Let's stick to simple axis locking for now.
                                if total_dx.abs() > total_dy.abs() {
                                    delta_y = 0.0;
                                } else {
                                    delta_x = 0.0;
                                }
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
            }
            
            self.widget.queue_draw();
        }
    }
    
    fn handle_drag_end(&self, offset_x: f64, offset_y: f64) {
        let tool = self.toolbox.as_ref().map(|t| t.current_tool()).unwrap_or(DesignerTool::Select);
        
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
        
        self.widget.queue_draw();
    }

    fn draw(cr: &gtk4::cairo::Context, state: &DesignerState, width: f64, height: f64, mouse_pos: (f64, f64), preview_start: Option<(f64, f64)>, preview_current: Option<(f64, f64)>, polyline_points: &[Point]) {
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
        
        // Draw Origin Crosshair
        Self::draw_origin_crosshair(cr);
        
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
    
    fn draw_origin_crosshair(cr: &gtk4::cairo::Context) {
        cr.save().unwrap();
        
        // Draw crosshair at origin
        cr.set_source_rgb(1.0, 0.0, 0.0); // Red
        cr.set_line_width(2.0);
        
        let size = 15.0;
        
        // Horizontal line
        cr.move_to(-size, 0.0);
        cr.line_to(size, 0.0);
        
        // Vertical line
        cr.move_to(0.0, -size);
        cr.line_to(0.0, size);
        
        cr.stroke().unwrap();
        
        // Draw circle
        cr.arc(0.0, 0.0, size * 0.7, 0.0, 2.0 * std::f64::consts::PI);
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
    pub fn new() -> Rc<Self> {
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
        let canvas = DesignerCanvas::new(state.clone(), Some(toolbox.clone()));
        
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
        let float_zoom_in = gtk4::Button::builder()
            .label("+")
            .tooltip_text("Zoom In")
            .build();

        floating_box.append(&float_zoom_out);
        floating_box.append(&float_fit);
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
        // Range: -5000 to 5000 seems reasonable for a start
        let h_adjustment = Adjustment::new(0.0, -5000.0, 5000.0, 10.0, 100.0, 100.0);
        let v_adjustment = Adjustment::new(0.0, -5000.0, 5000.0, 10.0, 100.0, 100.0);
        
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

        let view = Rc::new(Self {
            widget: container,
            canvas: canvas.clone(),
            toolbox,
            _properties: properties.clone(),
            layers: layers.clone(),
            status_label,
            _coord_label: coord_label,
            current_file,
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

    pub fn export_file(&self) {
        // TODO: Export
        println!("Designer: Export File");
    }
    
    // File operations - TODO: Implement once shape structures are aligned
    // Phase 8 infrastructure is in place but needs shape struct updates
}
