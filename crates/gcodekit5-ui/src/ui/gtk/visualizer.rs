use gtk4::prelude::*;
use gtk4::{DrawingArea, ScrolledWindow, PolicyType};
use std::cell::RefCell;
use std::rc::Rc;
use gcodekit5_visualizer::{Visualizer2D, GCodeCommand};

pub struct GcodeVisualizer {
    pub widget: DrawingArea,
    visualizer: Rc<RefCell<Visualizer2D>>,
}

impl GcodeVisualizer {
    pub fn new() -> Self {
        let widget = DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        let visualizer = Rc::new(RefCell::new(Visualizer2D::new()));
        let vis_clone = visualizer.clone();

        widget.set_draw_func(move |_, cr, width, height| {
            let vis = vis_clone.borrow();
            Self::draw(cr, &vis, width as f64, height as f64);
        });

        Self {
            widget,
            visualizer,
        }
    }

    pub fn set_gcode(&self, gcode: &str) {
        let mut vis = self.visualizer.borrow_mut();
        vis.parse_gcode(gcode);
        self.widget.queue_draw();
    }

    fn draw(cr: &gtk4::cairo::Context, vis: &Visualizer2D, width: f64, height: f64) {
        // Clear background
        cr.set_source_rgb(0.1, 0.1, 0.1); // Dark background
        cr.paint().expect("Invalid cairo surface state");

        // Setup coordinate system (flip Y because G-code is Y-up, Cairo is Y-down)
        // Also handle zoom/pan (TODO: Implement interactive zoom/pan)
        // For now, fit to view
        let (vb_x, vb_y, vb_w, vb_h) = vis.get_viewbox(width as f32, height as f32);
        
        // Calculate scale to fit
        let scale_x = width / vb_w as f64;
        let scale_y = height / vb_h as f64;
        let scale = scale_x.min(scale_y) * 0.9; // 90% fit

        // Center
        let center_x = width / 2.0;
        let center_y = height / 2.0;
        let vb_center_x = (vb_x + vb_w / 2.0) as f64;
        let vb_center_y = (vb_y + vb_h / 2.0) as f64;

        cr.translate(center_x, center_y);
        cr.scale(scale, -scale); // Flip Y
        cr.translate(-vb_center_x, -vb_center_y);

        // Draw Grid
        cr.set_source_rgba(0.3, 0.3, 0.3, 0.5);
        cr.set_line_width(1.0 / scale);
        // TODO: Draw grid lines based on vis.get_viewbox logic
        
        // Draw Origin
        cr.set_source_rgb(1.0, 0.0, 0.0);
        cr.move_to(0.0, 0.0);
        cr.line_to(10.0, 0.0);
        cr.stroke().unwrap();
        cr.set_source_rgb(0.0, 1.0, 0.0);
        cr.move_to(0.0, 0.0);
        cr.line_to(0.0, 10.0);
        cr.stroke().unwrap();

        // Draw Toolpath
        cr.set_line_width(1.5 / scale);
        
        for cmd in vis.commands() {
            match cmd {
                GCodeCommand::Move { from, to, rapid, .. } => {
                    if *rapid {
                        cr.set_source_rgba(0.5, 0.5, 0.5, 0.5); // Grey for rapid
                    } else {
                        cr.set_source_rgb(0.2, 0.6, 1.0); // Blue for cut
                    }
                    cr.move_to(from.x as f64, from.y as f64);
                    cr.line_to(to.x as f64, to.y as f64);
                    cr.stroke().unwrap();
                }
                GCodeCommand::Arc { from, to, center, clockwise, .. } => {
                    cr.set_source_rgb(0.2, 0.6, 1.0);
                    
                    let radius = ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt() as f64;
                    let start_angle = (from.y - center.y).atan2(from.x - center.x) as f64;
                    let end_angle = (to.y - center.y).atan2(to.x - center.x) as f64;
                    
                    if *clockwise {
                        cr.arc_negative(center.x as f64, center.y as f64, radius, start_angle, end_angle);
                    } else {
                        cr.arc(center.x as f64, center.y as f64, radius, start_angle, end_angle);
                    }
                    cr.stroke().unwrap();
                }
                _ => {}
            }
        }
    }
}
