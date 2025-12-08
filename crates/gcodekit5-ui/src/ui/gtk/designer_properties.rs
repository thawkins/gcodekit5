use gtk4::prelude::*;
use gtk4::{Box, Label, SpinButton, Orientation, Frame, ScrolledWindow, EventControllerFocus, DropDown, StringList, Expression};
use std::cell::RefCell;
use std::rc::Rc;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::shapes::{Shape, Point, OperationType};
use gcodekit5_designer::pocket_operations::PocketStrategy;

pub struct PropertiesPanel {
    pub widget: ScrolledWindow,
    state: Rc<RefCell<DesignerState>>,
    _content: Box,
    // Property widgets
    pos_x_spin: SpinButton,
    pos_y_spin: SpinButton,
    width_spin: SpinButton,
    height_spin: SpinButton,
    rotation_spin: SpinButton,
    // CAM widgets
    op_type_combo: DropDown,
    depth_spin: SpinButton,
    step_down_spin: SpinButton,
    step_in_spin: SpinButton,
    strategy_combo: DropDown,
    // Redraw callback
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    // Flag to prevent feedback loops during updates
    updating: Rc<RefCell<bool>>,
    // Flag to track if any widget has focus (being edited)
    has_focus: Rc<RefCell<bool>>,
}

impl PropertiesPanel {
    pub fn new(state: Rc<RefCell<DesignerState>>) -> Rc<Self> {
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .width_request(280)
            .hexpand(false)
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

        // CAM Properties Section
        let cam_frame = Self::create_section("CAM Properties");
        let cam_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        // Operation Type
        let op_label = Label::new(Some("Operation:"));
        op_label.set_halign(gtk4::Align::Start);
        let op_model = StringList::new(&["Profile", "Pocket"]);
        let op_type_combo = DropDown::new(Some(op_model), None::<Expression>);
        op_type_combo.set_hexpand(true);

        // Pocket Depth
        let depth_label = Label::new(Some("Depth:"));
        depth_label.set_halign(gtk4::Align::Start);
        let depth_spin = SpinButton::with_range(0.0, 100.0, 0.1);
        depth_spin.set_digits(2);
        depth_spin.set_hexpand(true);

        // Step Down
        let step_down_label = Label::new(Some("Step Down:"));
        step_down_label.set_halign(gtk4::Align::Start);
        let step_down_spin = SpinButton::with_range(0.1, 20.0, 0.1);
        step_down_spin.set_digits(2);
        step_down_spin.set_hexpand(true);

        // Step In (for pockets)
        let step_in_label = Label::new(Some("Step In:"));
        step_in_label.set_halign(gtk4::Align::Start);
        let step_in_spin = SpinButton::with_range(0.1, 20.0, 0.1);
        step_in_spin.set_digits(2);
        step_in_spin.set_hexpand(true);

        // Pocket Strategy
        let strategy_label = Label::new(Some("Strategy:"));
        strategy_label.set_halign(gtk4::Align::Start);
        let strategy_model = StringList::new(&["Raster", "Offset", "Adaptive"]);
        let strategy_combo = DropDown::new(Some(strategy_model), None::<Expression>);
        strategy_combo.set_hexpand(true);

        cam_grid.attach(&op_label, 0, 0, 1, 1);
        cam_grid.attach(&op_type_combo, 1, 0, 1, 1);
        cam_grid.attach(&depth_label, 0, 1, 1, 1);
        cam_grid.attach(&depth_spin, 1, 1, 1, 1);
        cam_grid.attach(&step_down_label, 0, 2, 1, 1);
        cam_grid.attach(&step_down_spin, 1, 2, 1, 1);
        cam_grid.attach(&step_in_label, 0, 3, 1, 1);
        cam_grid.attach(&step_in_spin, 1, 3, 1, 1);
        cam_grid.attach(&strategy_label, 0, 4, 1, 1);
        cam_grid.attach(&strategy_combo, 1, 4, 1, 1);

        cam_frame.set_child(Some(&cam_grid));
        content.append(&cam_frame);

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
            _content: content,
            pos_x_spin: pos_x_spin.clone(),
            pos_y_spin: pos_y_spin.clone(),
            width_spin: width_spin.clone(),
            height_spin: height_spin.clone(),
            rotation_spin: rotation_spin.clone(),
            op_type_combo: op_type_combo.clone(),
            depth_spin: depth_spin.clone(),
            step_down_spin: step_down_spin.clone(),
            step_in_spin: step_in_spin.clone(),
            strategy_combo: strategy_combo.clone(),
            redraw_callback: Rc::new(RefCell::new(None)),
            updating: Rc::new(RefCell::new(false)),
            has_focus: Rc::new(RefCell::new(false)),
        });

        // Connect value change handlers
        panel.setup_handlers();
        
        // Setup focus tracking for all spin buttons
        panel.setup_focus_tracking();

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
        let _pos_x = self.pos_x_spin.clone();
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
        let _pos_y = self.pos_y_spin.clone();
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
        let _width = self.width_spin.clone();
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
        let _height = self.height_spin.clone();
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

        let state = self.state.clone();
        let updating6 = self.updating.clone();
        
        // Operation Type changed
        self.op_type_combo.connect_selected_notify(move |combo| {
            if *updating6.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            let is_pocket = combo.selected() == 1;
            let depth = designer_state.canvas.shapes().find(|s| s.selected).map(|s| s.pocket_depth).unwrap_or(0.0);
            designer_state.set_selected_pocket_properties(is_pocket, depth);
        });

        let state = self.state.clone();
        let updating7 = self.updating.clone();
        let op_combo = self.op_type_combo.clone();

        // Pocket Depth changed
        self.depth_spin.connect_value_changed(move |spin| {
            if *updating7.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            let is_pocket = op_combo.selected() == 1;
            designer_state.set_selected_pocket_properties(is_pocket, spin.value());
        });

        let state = self.state.clone();
        let updating8 = self.updating.clone();

        // Step Down changed
        self.step_down_spin.connect_value_changed(move |spin| {
            if *updating8.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_step_down(spin.value());
        });

        let state = self.state.clone();
        let updating9 = self.updating.clone();

        // Step In changed
        self.step_in_spin.connect_value_changed(move |spin| {
            if *updating9.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_step_in(spin.value());
        });

        let state = self.state.clone();
        let updating10 = self.updating.clone();

        // Strategy changed
        self.strategy_combo.connect_selected_notify(move |combo| {
            if *updating10.borrow() { return; }
            let mut designer_state = state.borrow_mut();
            let strategy = match combo.selected() {
                0 => PocketStrategy::Raster { angle: 0.0, bidirectional: true },
                1 => PocketStrategy::ContourParallel,
                2 => PocketStrategy::Adaptive,
                _ => PocketStrategy::ContourParallel,
            };
            designer_state.set_selected_pocket_strategy(strategy);
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
        // Don't update if any widget has focus (user is editing)
        if *self.has_focus.borrow() {
            return;
        }
        
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
                
                // Update CAM properties
                self.op_type_combo.set_selected(match obj.operation_type {
                    OperationType::Profile => 0,
                    OperationType::Pocket => 1,
                });
                self.depth_spin.set_value(obj.pocket_depth);
                self.step_down_spin.set_value(obj.step_down as f64);
                self.step_in_spin.set_value(obj.step_in as f64);
                self.strategy_combo.set_selected(match obj.pocket_strategy {
                    PocketStrategy::Raster { .. } => 0,
                    PocketStrategy::ContourParallel => 1,
                    PocketStrategy::Adaptive => 2,
                });

                // Enable CAM controls
                self.op_type_combo.set_sensitive(true);
                self.depth_spin.set_sensitive(true);
                self.step_down_spin.set_sensitive(true);
                self.step_in_spin.set_sensitive(true);
                self.strategy_combo.set_sensitive(true);
            }
        } else {
            // No selection - disable controls
            self.pos_x_spin.set_sensitive(false);
            self.pos_y_spin.set_sensitive(false);
            self.width_spin.set_sensitive(false);
            self.height_spin.set_sensitive(false);
            self.rotation_spin.set_sensitive(false);
            
            self.op_type_combo.set_sensitive(false);
            self.depth_spin.set_sensitive(false);
            self.step_down_spin.set_sensitive(false);
            self.step_in_spin.set_sensitive(false);
            self.strategy_combo.set_sensitive(false);
        }
    }
    
    fn setup_focus_tracking(&self) {
        // Track focus for all spin buttons to prevent updates while user is editing
        let spinners = vec![
            &self.pos_x_spin,
            &self.pos_y_spin,
            &self.width_spin,
            &self.height_spin,
            &self.rotation_spin,
            &self.depth_spin,
            &self.step_down_spin,
            &self.step_in_spin,
        ];
        
        for spinner in spinners {
            let focus_controller = EventControllerFocus::new();
            let has_focus_enter = self.has_focus.clone();
            focus_controller.connect_enter(move |_| {
                *has_focus_enter.borrow_mut() = true;
            });
            
            let has_focus_leave = self.has_focus.clone();
            focus_controller.connect_leave(move |_| {
                *has_focus_leave.borrow_mut() = false;
            });
            
            spinner.add_controller(focus_controller);
        }
    }
    
    /// Clear the focus flag - call this when user interacts with the canvas
    pub fn clear_focus(&self) {
        *self.has_focus.borrow_mut() = false;
    }
}
