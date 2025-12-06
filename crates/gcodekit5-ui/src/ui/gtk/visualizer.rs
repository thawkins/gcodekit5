use gtk4::prelude::*;
use gtk4::{DrawingArea, ScrolledWindow, PolicyType, Box, Orientation, Button, CheckButton, Label, GestureClick, GestureDrag, EventControllerScroll, EventControllerScrollFlags, Paned};
use std::cell::RefCell;
use std::rc::Rc;
use gcodekit5_visualizer::{Visualizer2D};
use gcodekit5_visualizer::visualizer::GCodeCommand;

pub struct GcodeVisualizer {
    pub widget: Paned,
    drawing_area: DrawingArea,
    visualizer: Rc<RefCell<Visualizer2D>>,
    // Visibility toggles
    show_rapid: CheckButton,
    show_cut: CheckButton,
    show_grid: CheckButton,
    // Info labels
    bounds_label: Label,
}

impl GcodeVisualizer {
    pub fn new() -> Self {
        let container = Paned::new(Orientation::Horizontal);
        container.add_css_class("visualizer-container");
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Sidebar for controls
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.set_width_request(200);
        sidebar.add_css_class("visualizer-sidebar");
        sidebar.set_margin_start(12);
        sidebar.set_margin_end(12);
        sidebar.set_margin_top(12);
        sidebar.set_margin_bottom(12);

        // View Controls
        let view_label = Label::builder()
            .label("View Controls")
            .css_classes(vec!["heading"])
            .halign(gtk4::Align::Start)
            .build();
        sidebar.append(&view_label);

        let view_controls = Box::new(Orientation::Horizontal, 6);
        let fit_btn = Button::builder().icon_name("zoom-fit-best-symbolic").tooltip_text("Fit to View").build();
        let reset_btn = Button::builder().icon_name("view-restore-symbolic").tooltip_text("Reset View").build();
        
        view_controls.append(&fit_btn);
        view_controls.append(&reset_btn);
        sidebar.append(&view_controls);

        // Visibility
        let vis_label = Label::builder()
            .label("Visibility")
            .css_classes(vec!["heading"])
            .halign(gtk4::Align::Start)
            .margin_top(12)
            .build();
        sidebar.append(&vis_label);

        let show_rapid = CheckButton::builder().label("Show Rapid Moves").active(true).build();
        let show_cut = CheckButton::builder().label("Show Cutting Moves").active(true).build();
        let show_grid = CheckButton::builder().label("Show Grid").active(true).build();

        sidebar.append(&show_rapid);
        sidebar.append(&show_cut);
        sidebar.append(&show_grid);

        // Bounds Info
        let bounds_label = Label::builder()
            .label("Bounds\nX: 0.0 to 0.0\nY: 0.0 to 0.0")
            .css_classes(vec!["caption"])
            .halign(gtk4::Align::Start)
            .margin_top(24)
            .build();
        sidebar.append(&bounds_label);

        container.set_start_child(Some(&sidebar));

        // Drawing Area
        let drawing_area = DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .css_classes(vec!["visualizer-canvas"])
            .build();
        
        container.set_end_child(Some(&drawing_area));

        container.add_tick_callback(|paned, _clock| {
            let width = paned.width();
            let target = (width as f64 * 0.2) as i32;
            if (paned.position() - target).abs() > 2 {
                paned.set_position(target);
            }
            gtk4::glib::ControlFlow::Continue
        });

        let visualizer = Rc::new(RefCell::new(Visualizer2D::new()));
        
        // Connect Draw Signal
        let vis_draw = visualizer.clone();
        let show_rapid_draw = show_rapid.clone();
        let show_cut_draw = show_cut.clone();
        let show_grid_draw = show_grid.clone();

        drawing_area.set_draw_func(move |_, cr, width, height| {
            let vis = vis_draw.borrow();
            Self::draw(
                cr, 
                &vis, 
                width as f64, 
                height as f64,
                show_rapid_draw.is_active(),
                show_cut_draw.is_active(),
                show_grid_draw.is_active()
            );
        });

        // Connect Controls
        let vis_fit = visualizer.clone();
        let da_fit = drawing_area.clone();
        fit_btn.connect_clicked(move |_| {
            let width = da_fit.width() as f32;
            let height = da_fit.height() as f32;
            vis_fit.borrow_mut().fit_to_view(width, height);
            da_fit.queue_draw();
        });

        let vis_reset = visualizer.clone();
        let da_reset = drawing_area.clone();
        reset_btn.connect_clicked(move |_| {
            let mut vis = vis_reset.borrow_mut();
            vis.reset_zoom();
            vis.reset_pan();
            da_reset.queue_draw();
        });

        let da_update = drawing_area.clone();
        show_rapid.connect_toggled(move |_| da_update.queue_draw());
        let da_update = drawing_area.clone();
        show_cut.connect_toggled(move |_| da_update.queue_draw());
        let da_update = drawing_area.clone();
        show_grid.connect_toggled(move |_| da_update.queue_draw());

        // Mouse Interaction
        Self::setup_interaction(&drawing_area, &visualizer);

        Self {
            widget: container,
            drawing_area,
            visualizer,
            show_rapid,
            show_cut,
            show_grid,
            bounds_label,
        }
    }

    fn setup_interaction(da: &DrawingArea, vis: &Rc<RefCell<Visualizer2D>>) {
        // Scroll to Zoom
        let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        let vis_scroll = vis.clone();
        let da_scroll = da.clone();
        
        scroll.connect_scroll(move |_, _, dy| {
            let mut vis = vis_scroll.borrow_mut();
            if dy > 0.0 {
                vis.zoom_out();
            } else {
                vis.zoom_in();
            }
            da_scroll.queue_draw();
            gtk4::glib::Propagation::Stop
        });
        da.add_controller(scroll);

        // Drag to Pan
        let drag = GestureDrag::new();
        let vis_drag = vis.clone();
        let da_drag = da.clone();
        let start_pan = Rc::new(RefCell::new((0.0f32, 0.0f32)));

        let start_pan_begin = start_pan.clone();
        drag.connect_drag_begin(move |_, _, _| {
            let vis = vis_drag.borrow();
            *start_pan_begin.borrow_mut() = (vis.x_offset, vis.y_offset);
        });

        let vis_drag_update = vis.clone();
        let da_drag_update = da.clone();
        let start_pan_update = start_pan.clone();
        
        drag.connect_drag_update(move |_, dx, dy| {
            let mut vis = vis_drag_update.borrow_mut();
            let (sx, sy) = *start_pan_update.borrow();
            // Invert Y for pan because canvas Y is flipped
            vis.x_offset = sx + dx as f32;
            vis.y_offset = sy - dy as f32; 
            da_drag_update.queue_draw();
        });
        da.add_controller(drag);
    }

    pub fn set_gcode(&self, gcode: &str) {
        let mut vis = self.visualizer.borrow_mut();
        vis.parse_gcode(gcode);
        
        // Update bounds label
        let (min_x, max_x, min_y, max_y) = vis.get_bounds();
        self.bounds_label.set_text(&format!(
            "Bounds\nX: {:.1} to {:.1}\nY: {:.1} to {:.1}",
            min_x, max_x, min_y, max_y
        ));

        // Auto fit
        let width = self.drawing_area.width() as f32;
        let height = self.drawing_area.height() as f32;
        if width > 0.0 && height > 0.0 {
            vis.fit_to_view(width, height);
        }
        
        self.drawing_area.queue_draw();
    }

    fn draw(
        cr: &gtk4::cairo::Context, 
        vis: &Visualizer2D, 
        width: f64, 
        height: f64,
        show_rapid: bool,
        show_cut: bool,
        show_grid: bool
    ) {
        // Clear background
        cr.set_source_rgb(0.15, 0.15, 0.15); // Dark grey background
        cr.paint().expect("Invalid cairo surface state");

        // Apply transformations
        let center_x = width / 2.0;
        let center_y = height / 2.0;
        
        cr.save().unwrap();
        cr.translate(center_x, center_y);
        cr.scale(vis.zoom_scale as f64, -vis.zoom_scale as f64); // Flip Y
        cr.translate(vis.x_offset as f64, vis.y_offset as f64);

        // Draw Grid
        if show_grid {
            Self::draw_grid(cr, vis);
        }

        // Draw Origin
        cr.set_line_width(2.0 / vis.zoom_scale as f64);
        cr.set_source_rgb(1.0, 0.0, 0.0); // X Axis Red
        cr.move_to(0.0, 0.0);
        cr.line_to(20.0, 0.0);
        cr.stroke().unwrap();
        
        cr.set_source_rgb(0.0, 1.0, 0.0); // Y Axis Green
        cr.move_to(0.0, 0.0);
        cr.line_to(0.0, 20.0);
        cr.stroke().unwrap();

        // Draw Toolpath
        cr.set_line_width(1.5 / vis.zoom_scale as f64);
        
        for cmd in vis.commands() {
            match cmd {
                GCodeCommand::Move { from, to, rapid, .. } => {
                    if *rapid {
                        if show_rapid {
                            cr.set_source_rgba(0.0, 0.8, 1.0, 0.5); // Cyan for rapid
                            cr.move_to(from.x as f64, from.y as f64);
                            cr.line_to(to.x as f64, to.y as f64);
                            cr.stroke().unwrap();
                        }
                    } else {
                        if show_cut {
                            cr.set_source_rgb(1.0, 1.0, 0.0); // Yellow for cut
                            cr.move_to(from.x as f64, from.y as f64);
                            cr.line_to(to.x as f64, to.y as f64);
                            cr.stroke().unwrap();
                        }
                    }
                }
                GCodeCommand::Arc { from, to, center, clockwise, .. } => {
                    if show_cut {
                        cr.set_source_rgb(1.0, 1.0, 0.0); // Yellow for cut
                        
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
                }
                _ => {}
            }
        }
        
        cr.restore().unwrap();
    }

    fn draw_grid(cr: &gtk4::cairo::Context, vis: &Visualizer2D) {
        let grid_size = 10.0;
        let grid_color = (0.3, 0.3, 0.3, 0.5);
        
        // Calculate visible area in world coordinates
        // This is a simplification, ideally we'd project the viewport corners
        let range = 1000.0; // Draw a large enough grid for now
        
        cr.set_source_rgba(grid_color.0, grid_color.1, grid_color.2, grid_color.3);
        cr.set_line_width(1.0 / vis.zoom_scale as f64);

        let mut x = -range;
        while x <= range {
            cr.move_to(x, -range);
            cr.line_to(x, range);
            x += grid_size;
        }

        let mut y = -range;
        while y <= range {
            cr.move_to(-range, y);
            cr.line_to(range, y);
            y += grid_size;
        }
        
        cr.stroke().unwrap();
    }
}