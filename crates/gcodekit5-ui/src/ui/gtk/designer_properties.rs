use gtk4::prelude::*;
use gtk4::{Box, Label, Entry, SpinButton, Orientation, Frame, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::shapes::{Shape, Point};

pub struct PropertiesPanel {
    pub widget: ScrolledWindow,
    state: Rc<RefCell<DesignerState>>,
    content: Box,
    // Property widgets
    pos_x_spin: SpinButton,
    pos_y_spin: SpinButton,
    width_spin: SpinButton,
    height_spin: SpinButton,
    rotation_spin: SpinButton,
    // Redraw callback
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    // Flag to prevent feedback loops during updates
    updating: Rc<RefCell<bool>>,
}

impl PropertiesPanel {
    pub fn new(state: Rc<RefCell<DesignerState>>) -> Rc<Self> {
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .width_request(280)
            .build();

        let content = Box::new(Orientation::Vertical, 12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);

        // Header
        let header = Label::new(Some("Properties"));
        header.add_css_class("title-3");
        header.set_halign(gtk4::Align::Start);
        content.append(&header);

        // Position Section
        let pos_frame = Self::create_section("Position");
        let pos_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let x_label = Label::new(Some("X:"));
        x_label.set_halign(gtk4::Align::Start);
        let pos_x_spin = SpinButton::with_range(-1000.0, 1000.0, 0.1);
        pos_x_spin.set_digits(2);
        pos_x_spin.set_hexpand(true);

        let y_label = Label::new(Some("Y:"));
        y_label.set_halign(gtk4::Align::Start);
        let pos_y_spin = SpinButton::with_range(-1000.0, 1000.0, 0.1);
        pos_y_spin.set_digits(2);
        pos_y_spin.set_hexpand(true);

        pos_grid.attach(&x_label, 0, 0, 1, 1);
        pos_grid.attach(&pos_x_spin, 1, 0, 1, 1);
        pos_grid.attach(&y_label, 0, 1, 1, 1);
        pos_grid.attach(&pos_y_spin, 1, 1, 1, 1);

        pos_frame.set_child(Some(&pos_grid));
        content.append(&pos_frame);

        // Size Section
        let size_frame = Self::create_section("Size");
        let size_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let width_label = Label::new(Some("Width:"));
        width_label.set_halign(gtk4::Align::Start);
        let width_spin = SpinButton::with_range(0.1, 1000.0, 0.1);
        width_spin.set_digits(2);
        width_spin.set_hexpand(true);

        let height_label = Label::new(Some("Height:"));
        height_label.set_halign(gtk4::Align::Start);
        let height_spin = SpinButton::with_range(0.1, 1000.0, 0.1);
        height_spin.set_digits(2);
        height_spin.set_hexpand(true);

        size_grid.attach(&width_label, 0, 0, 1, 1);
        size_grid.attach(&width_spin, 1, 0, 1, 1);
        size_grid.attach(&height_label, 0, 1, 1, 1);
        size_grid.attach(&height_spin, 1, 1, 1, 1);

        size_frame.set_child(Some(&size_grid));
        content.append(&size_frame);

        // Rotation Section
        let rot_frame = Self::create_section("Rotation");
        let rot_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let rot_label = Label::new(Some("Angle:"));
        rot_label.set_halign(gtk4::Align::Start);
        let rotation_spin = SpinButton::with_range(-360.0, 360.0, 1.0);
        rotation_spin.set_digits(1);
        rotation_spin.set_hexpand(true);

        rot_grid.attach(&rot_label, 0, 0, 1, 1);
        rot_grid.attach(&rotation_spin, 1, 0, 1, 1);

        rot_frame.set_child(Some(&rot_grid));
        content.append(&rot_frame);

        // Empty state message
        let empty_label = Label::new(Some("Select a shape to edit its properties"));
        empty_label.add_css_class("dim-label");
        empty_label.set_wrap(true);
        empty_label.set_margin_top(24);
        content.append(&empty_label);

        scrolled.set_child(Some(&content));

        let panel = Rc::new(Self {
            widget: scrolled,
            state: state.clone(),
            content,
            pos_x_spin: pos_x_spin.clone(),
            pos_y_spin: pos_y_spin.clone(),
            width_spin: width_spin.clone(),
            height_spin: height_spin.clone(),
            rotation_spin: rotation_spin.clone(),
            redraw_callback: Rc::new(RefCell::new(None)),
            updating: Rc::new(RefCell::new(false)),
        });

        // Connect value change handlers
        panel.setup_handlers();

        panel
    }
    
    pub fn set_redraw_callback<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.redraw_callback.borrow_mut() = Some(Rc::new(callback));
    }

    fn create_section(title: &str) -> Frame {
        let frame = Frame::new(Some(title));
        frame
    }

    fn setup_handlers(&self) {
        let state = self.state.clone();
        let pos_x = self.pos_x_spin.clone();
        let pos_y = self.pos_y_spin.clone();
        let width = self.width_spin.clone();
        let height = self.height_spin.clone();
        let redraw1 = self.redraw_callback.clone();
        let updating1 = self.updating.clone();

        // Position X changed
        self.pos_x_spin.connect_value_changed(move |spin| {
            if *updating1.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                let y = pos_y.value();
                let w = width.value();
                let h = height.value();
                let x = spin.value();
                
                // Update shape position
                if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                    Self::update_shape_position_and_size(&mut obj.shape, x, y, w, h);
                }
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw1.borrow() {
                cb();
            }
        });

        let state = self.state.clone();
        let pos_x = self.pos_x_spin.clone();
        let pos_y = self.pos_y_spin.clone();
        let width = self.width_spin.clone();
        let height = self.height_spin.clone();
        let redraw2 = self.redraw_callback.clone();
        let updating2 = self.updating.clone();

        // Position Y changed
        self.pos_y_spin.connect_value_changed(move |spin| {
            if *updating2.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                let x = pos_x.value();
                let w = width.value();
                let h = height.value();
                let y = spin.value();
                
                if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                    Self::update_shape_position_and_size(&mut obj.shape, x, y, w, h);
                }
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw2.borrow() {
                cb();
            }
        });

        let state = self.state.clone();
        let pos_x = self.pos_x_spin.clone();
        let pos_y = self.pos_y_spin.clone();
        let width = self.width_spin.clone();
        let height = self.height_spin.clone();
        let redraw3 = self.redraw_callback.clone();
        let updating3 = self.updating.clone();

        // Width changed
        self.width_spin.connect_value_changed(move |spin| {
            if *updating3.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                let x = pos_x.value();
                let y = pos_y.value();
                let h = height.value();
                let w = spin.value();
                
                if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                    Self::update_shape_position_and_size(&mut obj.shape, x, y, w, h);
                }
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw3.borrow() {
                cb();
            }
        });

        let state = self.state.clone();
        let pos_x = self.pos_x_spin.clone();
        let pos_y = self.pos_y_spin.clone();
        let width = self.width_spin.clone();
        let height = self.height_spin.clone();
        let redraw4 = self.redraw_callback.clone();
        let updating4 = self.updating.clone();

        // Height changed
        self.height_spin.connect_value_changed(move |spin| {
            if *updating4.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                let x = pos_x.value();
                let y = pos_y.value();
                let w = width.value();
                let h = spin.value();
                
                if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                    Self::update_shape_position_and_size(&mut obj.shape, x, y, w, h);
                }
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw4.borrow() {
                cb();
            }
        });

        let state = self.state.clone();
        let redraw5 = self.redraw_callback.clone();
        let updating5 = self.updating.clone();
        
        // Rotation changed
        self.rotation_spin.connect_value_changed(move |spin| {
            if *updating5.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                let angle = spin.value();
                
                if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                    // Set rotation directly on shape variants that support it
                    match &mut obj.shape {
                        Shape::Rectangle(rect) => rect.rotation = angle.to_radians(),
                        Shape::Circle(circle) => circle.rotation = angle.to_radians(),
                        Shape::Ellipse(ellipse) => ellipse.rotation = angle.to_radians(),
                        _ => {}
                    }
                }
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw5.borrow() {
                cb();
            }
        });
    }

    fn update_shape_position_and_size(shape: &mut Shape, x: f64, y: f64, width: f64, height: f64) {
        match shape {
            Shape::Rectangle(rect) => {
                rect.x = x;
                rect.y = y;
                rect.width = width;
                rect.height = height;
            }
            Shape::Circle(circle) => {
                circle.center = Point::new(x, y);
                circle.radius = width / 2.0; // Use width as diameter
            }
            Shape::Ellipse(ellipse) => {
                ellipse.center = Point::new(x, y);
                ellipse.rx = width / 2.0;
                ellipse.ry = height / 2.0;
            }
            Shape::Line(line) => {
                // For line, x,y is start point, width/height define end point
                line.start = Point::new(x, y);
                line.end = Point::new(x + width, y + height);
            }
            _ => {
                // Other shapes not yet implemented
            }
        }
    }

    pub fn update_from_selection(&self) {
        let designer_state = self.state.borrow();
        
        if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
            if let Some(obj) = designer_state.canvas.shape_store.get(id) {
                // Set flag to prevent feedback loop during updates
                *self.updating.borrow_mut() = true;

                // Update spin buttons based on shape type
                match &obj.shape {
                    Shape::Rectangle(rect) => {
                        self.pos_x_spin.set_value(rect.x);
                        self.pos_y_spin.set_value(rect.y);
                        self.width_spin.set_value(rect.width);
                        self.height_spin.set_value(rect.height);
                        self.rotation_spin.set_value(rect.rotation.to_degrees());
                    }
                    Shape::Circle(circle) => {
                        self.pos_x_spin.set_value(circle.center.x);
                        self.pos_y_spin.set_value(circle.center.y);
                        self.width_spin.set_value(circle.radius * 2.0);
                        self.height_spin.set_value(circle.radius * 2.0);
                        self.rotation_spin.set_value(circle.rotation.to_degrees());
                    }
                    Shape::Ellipse(ellipse) => {
                        self.pos_x_spin.set_value(ellipse.center.x);
                        self.pos_y_spin.set_value(ellipse.center.y);
                        self.width_spin.set_value(ellipse.rx * 2.0);
                        self.height_spin.set_value(ellipse.ry * 2.0);
                        self.rotation_spin.set_value(ellipse.rotation.to_degrees());
                    }
                    Shape::Line(line) => {
                        self.pos_x_spin.set_value(line.start.x);
                        self.pos_y_spin.set_value(line.start.y);
                        self.width_spin.set_value(line.end.x - line.start.x);
                        self.height_spin.set_value(line.end.y - line.start.y);
                        self.rotation_spin.set_value(0.0);
                    }
                    _ => {
                        // Other shapes
                    }
                }

                // Clear flag
                *self.updating.borrow_mut() = false;

                // Enable all controls
                self.pos_x_spin.set_sensitive(true);
                self.pos_y_spin.set_sensitive(true);
                self.width_spin.set_sensitive(true);
                self.height_spin.set_sensitive(true);
                self.rotation_spin.set_sensitive(true);
            }
        } else {
            // No selection - disable controls
            self.pos_x_spin.set_sensitive(false);
            self.pos_y_spin.set_sensitive(false);
            self.width_spin.set_sensitive(false);
            self.height_spin.set_sensitive(false);
            self.rotation_spin.set_sensitive(false);
        }
    }
}
