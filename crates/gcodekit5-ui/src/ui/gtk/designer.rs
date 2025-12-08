use gtk4::prelude::*;
use gtk4::{DrawingArea, GestureClick, GestureDrag, EventControllerMotion, Box, Label, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::shapes::{Shape, Point, Rectangle, Circle, Line, Ellipse};
use gcodekit5_designer::canvas::DrawingObject;
use crate::ui::gtk::designer_toolbox::{DesignerToolbox, DesignerTool};

pub struct DesignerCanvas {
    pub widget: DrawingArea,
    state: Rc<RefCell<DesignerState>>,
    mouse_pos: Rc<RefCell<(f64, f64)>>, // Canvas coordinates
    toolbox: Option<Rc<DesignerToolbox>>,
    // Shape creation state
    creation_start: Rc<RefCell<Option<(f64, f64)>>>,
    creation_current: Rc<RefCell<Option<(f64, f64)>>>,
    // Track last drag offset for incremental movement
    last_drag_offset: Rc<RefCell<(f64, f64)>>,
}

pub struct DesignerView {
    pub widget: Box,
    canvas: Rc<DesignerCanvas>,
    toolbox: Rc<DesignerToolbox>,
    status_label: Label,
    coord_label: Label,
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

        let state_clone = state.clone();
        let mouse_pos_clone = mouse_pos.clone();
        widget.set_draw_func(move |_, cr, width, height| {
            let state = state_clone.borrow();
            let mouse = *mouse_pos_clone.borrow();
            Self::draw(cr, &state, width as f64, height as f64, mouse);
        });

        let canvas = Rc::new(Self {
            widget: widget.clone(),
            state: state.clone(),
            mouse_pos: mouse_pos.clone(),
            toolbox: toolbox.clone(),
            creation_start: creation_start.clone(),
            creation_current: creation_current.clone(),
            last_drag_offset: last_drag_offset.clone(),
        });

        // Mouse motion tracking
        let motion_ctrl = EventControllerMotion::new();
        let mouse_pos_motion = mouse_pos.clone();
        let widget_motion = widget.clone();
        motion_ctrl.connect_motion(move |_, x, y| {
            // Convert screen coords to canvas coords
            let width = widget_motion.width() as f64;
            let height = widget_motion.height() as f64;
            
            // Transform to canvas coordinates (centered origin, Y-up)
            let canvas_x = x - width / 2.0;
            let canvas_y = -(y - height / 2.0); // Flip Y
            
            *mouse_pos_motion.borrow_mut() = (canvas_x, canvas_y);
            widget_motion.queue_draw();
        });
        widget.add_controller(motion_ctrl);

        // Interaction controllers
        let click_gesture = GestureClick::new();
        let canvas_click = canvas.clone();
        click_gesture.connect_pressed(move |gesture, n_press, x, y| {
            canvas_click.handle_click(x, y);
        });
        widget.add_controller(click_gesture);

        let drag_gesture = GestureDrag::new();
        let canvas_drag = canvas.clone();
        drag_gesture.connect_drag_begin(move |gesture, x, y| {
            canvas_drag.handle_drag_begin(x, y);
        });
        
        let canvas_drag_update = canvas.clone();
        drag_gesture.connect_drag_update(move |gesture, offset_x, offset_y| {
            canvas_drag_update.handle_drag_update(offset_x, offset_y);
        });
        
        let canvas_drag_end = canvas.clone();
        drag_gesture.connect_drag_end(move |gesture, offset_x, offset_y| {
            canvas_drag_end.handle_drag_end(offset_x, offset_y);
        });
        widget.add_controller(drag_gesture);
        
        // Keyboard controller for Delete, Escape, etc.
        let key_controller = gtk4::EventControllerKey::new();
        let state_key = state.clone();
        let widget_key = widget.clone();
        
        key_controller.connect_key_pressed(move |_controller, keyval, _keycode, _modifier| {
            let mut designer_state = state_key.borrow_mut();
            
            match keyval {
                gtk4::gdk::Key::Delete | gtk4::gdk::Key::BackSpace => {
                    // Delete selected shapes
                    if designer_state.canvas.selection_manager.selected_id().is_some() {
                        designer_state.canvas.remove_selected();
                        drop(designer_state);
                        widget_key.queue_draw();
                        return glib::Propagation::Stop;
                    }
                }
                gtk4::gdk::Key::Escape => {
                    // Deselect all
                    designer_state.canvas.deselect_all();
                    drop(designer_state);
                    widget_key.queue_draw();
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
            
            glib::Propagation::Proceed
        });
        widget.add_controller(key_controller);

        canvas
    }
    
    fn handle_click(&self, x: f64, y: f64) {
        let tool = self.toolbox.as_ref().map(|t| t.current_tool()).unwrap_or(DesignerTool::Select);
        
        // Convert screen coords to canvas coords
        let width = self.widget.width() as f64;
        let height = self.widget.height() as f64;
        let canvas_x = x - width / 2.0;
        let canvas_y = -(y - height / 2.0);
        
        match tool {
            DesignerTool::Select => {
                // Handle selection
                let mut state = self.state.borrow_mut();
                let point = Point::new(canvas_x, canvas_y);
                
                // Try to select shape at click point
                if let Some(_selected_id) = state.canvas.select_at(&point, false) {
                    // Shape selected
                } else {
                    // Click on empty space - deselect all
                    state.canvas.deselect_all();
                }
                
                drop(state);
                self.widget.queue_draw();
            }
            _ => {
                // Other tools handled by drag
            }
        }
    }
    
    fn handle_drag_begin(&self, x: f64, y: f64) {
        let tool = self.toolbox.as_ref().map(|t| t.current_tool()).unwrap_or(DesignerTool::Select);
        
        // Convert screen coords to canvas coords
        let width = self.widget.width() as f64;
        let height = self.widget.height() as f64;
        let canvas_x = x - width / 2.0;
        let canvas_y = -(y - height / 2.0);
        
        match tool {
            DesignerTool::Select => {
                // Check if we're starting to drag a selected shape
                let state = self.state.borrow();
                let point = Point::new(canvas_x, canvas_y);
                
                // Check if clicking on a selected shape
                let has_selected = state.canvas.selection_manager.selected_id().is_some();
                drop(state);
                
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
        
        // Get start point without holding the borrow
        let start_opt = *self.creation_start.borrow();
        
        if let Some(start) = start_opt {
            // Update current position (offset is from drag start)
            let current_x = start.0 + offset_x;
            let current_y = start.1 - offset_y; // Flip Y offset
            
            *self.creation_current.borrow_mut() = Some((current_x, current_y));
            
            // If in select mode and dragging, move selected shapes
            if tool == DesignerTool::Select {
                let mut state = self.state.borrow_mut();
                if state.canvas.selection_manager.selected_id().is_some() {
                    // Calculate delta from last update (incremental movement)
                    let last_offset = *self.last_drag_offset.borrow();
                    let delta_x = offset_x - last_offset.0;
                    let delta_y = offset_y - last_offset.1;
                    
                    // Apply incremental movement
                    state.canvas.move_selected(delta_x, -delta_y);
                    
                    // Update last offset
                    *self.last_drag_offset.borrow_mut() = (offset_x, offset_y);
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
            let end_x = start.0 + offset_x;
            let end_y = start.1 - offset_y; // Flip Y offset
            
            match tool {
                DesignerTool::Select => {
                    // Drag ended in select mode - movement already applied incrementally
                    // Nothing more to do
                }
                _ => {
                    // Create the shape for drawing tools
                    self.create_shape(tool, start, (end_x, end_y));
                }
            }
            
            // Clear creation state (now safe - no borrows held)
            *self.creation_start.borrow_mut() = None;
            *self.creation_current.borrow_mut() = None;
            
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
    }

    fn draw(cr: &gtk4::cairo::Context, state: &DesignerState, width: f64, height: f64, mouse_pos: (f64, f64)) {
        // Clear background
        cr.set_source_rgb(0.95, 0.95, 0.95); // Light grey background
        cr.paint().expect("Invalid cairo surface state");

        // Setup coordinate system
        // Designer uses Y-up (Cartesian), Cairo uses Y-down
        // We need to flip Y and translate origin
        // TODO: Use Viewport from state
        
        // For now, center (0,0) in the middle and flip Y
        cr.translate(width / 2.0, height / 2.0);
        cr.scale(1.0, -1.0);

        // Draw Grid
        if state.show_grid {
            Self::draw_grid(cr, width, height);
        }
        
        // Draw Origin Crosshair
        Self::draw_origin_crosshair(cr);
        
        // Draw creation preview (if creating)
        // Note: We can't access self here, so preview will be drawn separately
        // This is a limitation of the current draw_func approach

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
            
            cr.restore().unwrap();
        }
    }

    fn draw_grid(cr: &gtk4::cairo::Context, width: f64, height: f64) {
        cr.save().unwrap();
        
        // Grid spacing in mm (10mm major grid)
        let grid_spacing = 10.0;
        let minor_spacing = grid_spacing / 5.0; // 2mm minor grid
        
        // Calculate visible range
        let half_w = width / 2.0;
        let half_h = height / 2.0;
        
        // Minor grid lines (lighter)
        cr.set_source_rgba(0.85, 0.85, 0.85, 0.5);
        cr.set_line_width(0.5);
        
        let mut x = 0.0;
        while x < half_w {
            if (x / grid_spacing).round() != x / grid_spacing {
                cr.move_to(x, -half_h);
                cr.line_to(x, half_h);
                cr.move_to(-x, -half_h);
                cr.line_to(-x, half_h);
            }
            x += minor_spacing;
        }
        
        let mut y = 0.0;
        while y < half_h {
            if (y / grid_spacing).round() != y / grid_spacing {
                cr.move_to(-half_w, y);
                cr.line_to(half_w, y);
                cr.move_to(-half_w, -y);
                cr.line_to(half_w, -y);
            }
            y += minor_spacing;
        }
        cr.stroke().unwrap();
        
        // Major grid lines (darker)
        cr.set_source_rgba(0.7, 0.7, 0.7, 0.6);
        cr.set_line_width(1.0);
        
        x = 0.0;
        while x < half_w {
            cr.move_to(x, -half_h);
            cr.line_to(x, half_h);
            if x != 0.0 {
                cr.move_to(-x, -half_h);
                cr.line_to(-x, half_h);
            }
            x += grid_spacing;
        }
        
        y = 0.0;
        while y < half_h {
            cr.move_to(-half_w, y);
            cr.line_to(half_w, y);
            if y != 0.0 {
                cr.move_to(-half_w, -y);
                cr.line_to(half_w, -y);
            }
            y += grid_spacing;
        }
        cr.stroke().unwrap();
        
        // Draw axes (thicker, darker)
        cr.set_source_rgba(0.3, 0.3, 0.3, 0.8);
        cr.set_line_width(2.0);
        cr.move_to(-half_w, 0.0);
        cr.line_to(half_w, 0.0);
        cr.move_to(0.0, -half_h);
        cr.line_to(0.0, half_h);
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
}

impl DesignerView {
    pub fn new() -> Rc<Self> {
        let container = Box::new(Orientation::Vertical, 0);
        container.set_hexpand(true);
        container.set_vexpand(true);
        
        // Create designer state
        let state = Rc::new(RefCell::new(DesignerState::new()));
        
        // Create main horizontal layout (toolbox + canvas)
        let main_box = Box::new(Orientation::Horizontal, 0);
        main_box.set_hexpand(true);
        main_box.set_vexpand(true);
        
        // Create toolbox
        let toolbox = DesignerToolbox::new();
        main_box.append(&toolbox.widget);
        
        // Create canvas
        let canvas = DesignerCanvas::new(state.clone(), Some(toolbox.clone()));
        main_box.append(&canvas.widget);
        
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
        
        let view = Rc::new(Self {
            widget: container,
            canvas,
            toolbox,
            status_label,
            coord_label,
        });
        
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
}
