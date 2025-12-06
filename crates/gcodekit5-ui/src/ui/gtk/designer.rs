use gtk4::prelude::*;
use gtk4::{DrawingArea, GestureClick, GestureDrag, EventControllerMotion};
use std::cell::RefCell;
use std::rc::Rc;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::shapes::{Shape, Point};

pub struct DesignerCanvas {
    pub widget: DrawingArea,
    state: Rc<RefCell<DesignerState>>,
}

impl DesignerCanvas {
    pub fn new(state: Rc<RefCell<DesignerState>>) -> Self {
        let widget = DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        let state_clone = state.clone();
        widget.set_draw_func(move |_, cr, width, height| {
            let state = state_clone.borrow();
            Self::draw(cr, &state, width as f64, height as f64);
        });

        // Interaction controllers
        let click_gesture = GestureClick::new();
        let state_click = state.clone();
        let widget_click = widget.clone();
        click_gesture.connect_pressed(move |gesture, n_press, x, y| {
            // Handle click (selection)
            // let mut state = state_click.borrow_mut();
            // state.handle_click(x, y);
            // widget_click.queue_draw();
            println!("Click at {}, {}", x, y);
        });
        widget.add_controller(click_gesture);

        let drag_gesture = GestureDrag::new();
        let state_drag = state.clone();
        let widget_drag = widget.clone();
        drag_gesture.connect_drag_begin(move |gesture, x, y| {
            println!("Drag begin at {}, {}", x, y);
        });
        drag_gesture.connect_drag_update(move |gesture, offset_x, offset_y| {
             println!("Drag update {}, {}", offset_x, offset_y);
        });
        drag_gesture.connect_drag_end(move |gesture, offset_x, offset_y| {
             println!("Drag end {}, {}", offset_x, offset_y);
        });
        widget.add_controller(drag_gesture);

        Self {
            widget,
            state,
        }
    }

    fn draw(cr: &gtk4::cairo::Context, state: &DesignerState, width: f64, height: f64) {
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
                    cr.scale(ellipse.radius_x, ellipse.radius_y);
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
        cr.set_source_rgba(0.8, 0.8, 0.8, 0.5);
        cr.set_line_width(1.0);

        // Draw axes
        cr.move_to(-1000.0, 0.0);
        cr.line_to(1000.0, 0.0);
        cr.move_to(0.0, -1000.0);
        cr.line_to(0.0, 1000.0);
        cr.stroke().unwrap();

        cr.restore().unwrap();
    }
}
