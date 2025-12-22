use crate::t;
use gcodekit5_core::units;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::font_manager;
use gcodekit5_designer::model::{DesignerShape, Point, Shape};
use gcodekit5_designer::pocket_operations::PocketStrategy;
use gcodekit5_designer::shapes::OperationType;
use gcodekit5_settings::SettingsPersistence;
use gtk4::prelude::*;
use gtk4::{
    Box, CheckButton, DropDown, Entry, EventControllerFocus, Expression, Frame, Label, Orientation,
    ScrolledWindow, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

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

pub struct PropertiesPanel {
    pub widget: ScrolledWindow,
    state: Rc<RefCell<DesignerState>>,
    settings: Rc<RefCell<SettingsPersistence>>,
    _content: Box,
    header: Label,

    // Sections (visibility controlled)
    pos_frame: Frame,
    size_frame: Frame,
    rot_frame: Frame,
    corner_frame: Frame,
    text_frame: Frame,
    cam_frame: Frame,
    empty_label: Label,

    // Property widgets
    pos_x_entry: Entry,
    pos_y_entry: Entry,
    width_entry: Entry,
    height_entry: Entry,
    rotation_entry: Entry,
    // Rectangle widgets
    corner_radius_entry: Entry,
    is_slot_check: CheckButton,
    // Text widgets
    text_entry: Entry,
    font_family_combo: DropDown,
    font_bold_check: CheckButton,
    font_italic_check: CheckButton,
    font_size_entry: Entry,
    // Polygon widgets
    polygon_frame: Frame,
    sides_entry: Entry,
    // CAM widgets
    op_type_combo: DropDown,
    depth_entry: Entry,
    step_down_entry: Entry,
    step_in_entry: Entry,
    ramp_angle_entry: Entry,
    strategy_combo: DropDown,
    raster_fill_entry: Entry,
    // Unit Labels
    x_unit_label: Label,
    y_unit_label: Label,
    width_unit_label: Label,
    height_unit_label: Label,
    radius_unit_label: Label,
    font_size_unit_label: Label,
    depth_unit_label: Label,
    step_down_unit_label: Label,
    step_in_unit_label: Label,
    // Redraw callback
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    // Flag to prevent feedback loops during updates
    updating: Rc<RefCell<bool>>,
    // Flag to track if any widget has focus (being edited)
    has_focus: Rc<RefCell<bool>>,
}

impl PropertiesPanel {
    pub fn new(
        state: Rc<RefCell<DesignerState>>,
        settings: Rc<RefCell<SettingsPersistence>>,
    ) -> Rc<Self> {
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

        // Header (kept for internal state, not shown in UI)
        let header = Label::new(Some(&t!("Properties")));
        header.add_css_class("title-3");
        header.add_css_class("heading");
        header.set_halign(gtk4::Align::Start);
        header.set_visible(false);

        // Position Section
        let pos_frame = Self::create_section(&t!("Position"));
        let pos_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let x_label = Label::new(Some(&t!("X:")));
        x_label.set_halign(gtk4::Align::Start);
        let pos_x_entry = Entry::new();
        pos_x_entry.set_hexpand(true);
        let x_unit_label = Label::new(Some("mm"));
        x_unit_label.set_width_chars(4);
        x_unit_label.set_halign(gtk4::Align::End);
        x_unit_label.set_xalign(1.0);

        let y_label = Label::new(Some(&t!("Y:")));
        y_label.set_halign(gtk4::Align::Start);
        let pos_y_entry = Entry::new();
        pos_y_entry.set_hexpand(true);
        let y_unit_label = Label::new(Some("mm"));
        y_unit_label.set_width_chars(4);
        y_unit_label.set_halign(gtk4::Align::End);
        y_unit_label.set_xalign(1.0);

        pos_grid.attach(&x_label, 0, 0, 1, 1);
        pos_grid.attach(&pos_x_entry, 1, 0, 1, 1);
        pos_grid.attach(&x_unit_label, 2, 0, 1, 1);
        pos_grid.attach(&y_label, 0, 1, 1, 1);
        pos_grid.attach(&pos_y_entry, 1, 1, 1, 1);
        pos_grid.attach(&y_unit_label, 2, 1, 1, 1);

        pos_frame.set_child(Some(&pos_grid));
        content.append(&pos_frame);

        // Size Section
        let size_frame = Self::create_section(&t!("Size"));
        let size_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let width_label = Label::new(Some(&t!("Width:")));
        width_label.set_halign(gtk4::Align::Start);
        let width_entry = Entry::new();
        width_entry.set_hexpand(true);
        let width_unit_label = Label::new(Some("mm"));
        width_unit_label.set_width_chars(4);
        width_unit_label.set_halign(gtk4::Align::End);
        width_unit_label.set_xalign(1.0);

        let height_label = Label::new(Some(&t!("Height:")));
        height_label.set_halign(gtk4::Align::Start);
        let height_entry = Entry::new();
        height_entry.set_hexpand(true);
        let height_unit_label = Label::new(Some("mm"));
        height_unit_label.set_width_chars(4);
        height_unit_label.set_halign(gtk4::Align::End);
        height_unit_label.set_xalign(1.0);

        size_grid.attach(&width_label, 0, 0, 1, 1);
        size_grid.attach(&width_entry, 1, 0, 1, 1);
        size_grid.attach(&width_unit_label, 2, 0, 1, 1);
        size_grid.attach(&height_label, 0, 1, 1, 1);
        size_grid.attach(&height_entry, 1, 1, 1, 1);
        size_grid.attach(&height_unit_label, 2, 1, 1, 1);

        size_frame.set_child(Some(&size_grid));
        content.append(&size_frame);

        // Rotation Section
        let rot_frame = Self::create_section(&t!("Rotation"));
        let rot_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let rot_label = Label::new(Some(&t!("Angle:")));
        rot_label.set_halign(gtk4::Align::Start);
        let rotation_entry = Entry::new();
        rotation_entry.set_hexpand(true);
        let rot_unit = Label::new(Some("deg"));

        rot_grid.attach(&rot_label, 0, 0, 1, 1);
        rot_grid.attach(&rotation_entry, 1, 0, 1, 1);
        rot_grid.attach(&rot_unit, 2, 0, 1, 1);

        rot_frame.set_child(Some(&rot_grid));
        content.append(&rot_frame);

        // Corner Section (Rectangle only)
        let corner_frame = Self::create_section(&t!("Corner"));
        let corner_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let radius_label = Label::new(Some(&t!("Radius:")));
        radius_label.set_halign(gtk4::Align::Start);
        let corner_radius_entry = Entry::new();
        corner_radius_entry.set_hexpand(true);
        let radius_unit_label = Label::new(Some("mm"));
        radius_unit_label.set_width_chars(4);
        radius_unit_label.set_halign(gtk4::Align::End);
        radius_unit_label.set_xalign(1.0);

        let slot_label = Label::new(Some(&t!("Slot Mode:")));
        slot_label.set_halign(gtk4::Align::Start);
        let is_slot_check = CheckButton::new();

        corner_grid.attach(&radius_label, 0, 0, 1, 1);
        corner_grid.attach(&corner_radius_entry, 1, 0, 1, 1);
        corner_grid.attach(&radius_unit_label, 2, 0, 1, 1);
        corner_grid.attach(&slot_label, 0, 1, 1, 1);
        corner_grid.attach(&is_slot_check, 1, 1, 1, 1);

        corner_frame.set_child(Some(&corner_grid));
        content.append(&corner_frame);

        // Text Section
        let text_frame = Self::create_section(&t!("Text"));
        let text_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let text_content_label = Label::new(Some(&t!("Content:")));
        text_content_label.set_halign(gtk4::Align::Start);
        let text_entry = Entry::new();
        text_entry.set_hexpand(true);

        let font_label = Label::new(Some(&t!("Font:")));
        font_label.set_halign(gtk4::Align::Start);
        let font_model = StringList::new(&[]);
        font_model.append("Sans");
        for fam in font_manager::list_font_families() {
            if fam != "Sans" {
                font_model.append(&fam);
            }
        }
        let font_family_combo = DropDown::new(Some(font_model), None::<Expression>);
        font_family_combo.set_hexpand(true);

        let style_label = Label::new(Some(&t!("Style:")));
        style_label.set_halign(gtk4::Align::Start);
        let font_bold_check = CheckButton::with_label(&t!("Bold"));
        let font_italic_check = CheckButton::with_label(&t!("Italic"));
        let style_box = Box::new(Orientation::Horizontal, 8);
        style_box.append(&font_bold_check);
        style_box.append(&font_italic_check);

        let font_size_label = Label::new(Some(&t!("Size:")));
        font_size_label.set_halign(gtk4::Align::Start);
        let font_size_entry = Entry::new();
        font_size_entry.set_hexpand(true);
        let font_size_unit_label = Label::new(Some("pt"));
        font_size_unit_label.set_width_chars(4);
        font_size_unit_label.set_halign(gtk4::Align::End);
        font_size_unit_label.set_xalign(1.0);

        text_grid.attach(&text_content_label, 0, 0, 1, 1);
        text_grid.attach(&text_entry, 1, 0, 2, 1);

        text_grid.attach(&font_label, 0, 1, 1, 1);
        text_grid.attach(&font_family_combo, 1, 1, 2, 1);

        text_grid.attach(&style_label, 0, 2, 1, 1);
        text_grid.attach(&style_box, 1, 2, 2, 1);

        text_grid.attach(&font_size_label, 0, 3, 1, 1);
        text_grid.attach(&font_size_entry, 1, 3, 1, 1);
        text_grid.attach(&font_size_unit_label, 2, 3, 1, 1);

        text_frame.set_child(Some(&text_grid));
        content.append(&text_frame);

        // Polygon Section
        let polygon_frame = Self::create_section(&t!("Polygon"));
        let polygon_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let sides_label = Label::new(Some(&t!("Sides:")));
        sides_label.set_halign(gtk4::Align::Start);
        let sides_entry = Entry::new();
        sides_entry.set_hexpand(true);

        polygon_grid.attach(&sides_label, 0, 0, 1, 1);
        polygon_grid.attach(&sides_entry, 1, 0, 1, 1);

        polygon_frame.set_child(Some(&polygon_grid));
        content.append(&polygon_frame);

        // CAM Properties Section
        let cam_frame = Self::create_section(&t!("CAM Properties"));
        let cam_grid = gtk4::Grid::builder()
            .row_spacing(8)
            .column_spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        // Operation Type
        let op_label = Label::new(Some(&t!("Operation:")));
        op_label.set_halign(gtk4::Align::Start);
        let op_model = StringList::new(&[]);
        op_model.append(&t!("Profile"));
        op_model.append(&t!("Pocket"));
        let op_type_combo = DropDown::new(Some(op_model), None::<Expression>);
        op_type_combo.set_hexpand(true);

        // Pocket Depth
        let depth_label = Label::new(Some(&t!("Depth:")));
        depth_label.set_halign(gtk4::Align::Start);
        let depth_entry = Entry::new();
        depth_entry.set_hexpand(true);
        let depth_unit_label = Label::new(Some("mm"));
        depth_unit_label.set_width_chars(4);
        depth_unit_label.set_halign(gtk4::Align::End);
        depth_unit_label.set_xalign(1.0);

        // Step Down
        let step_down_label = Label::new(Some(&t!("Step Down:")));
        step_down_label.set_halign(gtk4::Align::Start);
        let step_down_entry = Entry::new();
        step_down_entry.set_hexpand(true);
        let step_down_unit_label = Label::new(Some("mm"));
        step_down_unit_label.set_width_chars(4);
        step_down_unit_label.set_halign(gtk4::Align::End);
        step_down_unit_label.set_xalign(1.0);

        // Step In (for pockets)
        let step_in_label = Label::new(Some(&t!("Step In:")));
        step_in_label.set_halign(gtk4::Align::Start);
        let step_in_entry = Entry::new();
        step_in_entry.set_hexpand(true);
        let step_in_unit_label = Label::new(Some("mm"));
        step_in_unit_label.set_width_chars(4);
        step_in_unit_label.set_halign(gtk4::Align::End);
        step_in_unit_label.set_xalign(1.0);

        // Ramp Angle
        let ramp_angle_label = Label::new(Some(&t!("Ramp Angle:")));
        ramp_angle_label.set_halign(gtk4::Align::Start);
        let ramp_angle_entry = Entry::new();
        ramp_angle_entry.set_hexpand(true);
        let ramp_angle_unit_label = Label::new(Some("deg"));
        ramp_angle_unit_label.set_width_chars(4);
        ramp_angle_unit_label.set_halign(gtk4::Align::End);
        ramp_angle_unit_label.set_xalign(1.0);

        // Pocket Strategy
        let strategy_label = Label::new(Some(&t!("Strategy:")));
        strategy_label.set_halign(gtk4::Align::Start);
        let strategy_model = StringList::new(&[]);
        strategy_model.append(&t!("Raster"));
        strategy_model.append(&t!("Offset"));
        strategy_model.append(&t!("Adaptive"));
        let strategy_combo = DropDown::new(Some(strategy_model), None::<Expression>);
        strategy_combo.set_hexpand(true);

        // Raster Fill (inverse inset)
        let raster_fill_label = Label::new(Some(&t!("Raster Fill (%):")));
        raster_fill_label.set_halign(gtk4::Align::Start);
        let raster_fill_entry = Entry::new();
        raster_fill_entry.set_hexpand(true);
        let raster_fill_hint = Label::new(Some("0 = no raster, 100 = full length"));
        raster_fill_hint.add_css_class("dim-label");
        raster_fill_hint.set_halign(gtk4::Align::Start);

        cam_grid.attach(&op_label, 0, 0, 1, 1);
        cam_grid.attach(&op_type_combo, 1, 0, 1, 1);
        cam_grid.attach(&depth_label, 0, 1, 1, 1);
        cam_grid.attach(&depth_entry, 1, 1, 1, 1);
        cam_grid.attach(&depth_unit_label, 2, 1, 1, 1);
        cam_grid.attach(&step_down_label, 0, 2, 1, 1);
        cam_grid.attach(&step_down_entry, 1, 2, 1, 1);
        cam_grid.attach(&step_down_unit_label, 2, 2, 1, 1);
        cam_grid.attach(&step_in_label, 0, 3, 1, 1);
        cam_grid.attach(&step_in_entry, 1, 3, 1, 1);
        cam_grid.attach(&step_in_unit_label, 2, 3, 1, 1);
        cam_grid.attach(&ramp_angle_label, 0, 4, 1, 1);
        cam_grid.attach(&ramp_angle_entry, 1, 4, 1, 1);
        cam_grid.attach(&ramp_angle_unit_label, 2, 4, 1, 1);
        cam_grid.attach(&strategy_label, 0, 5, 1, 1);
        cam_grid.attach(&strategy_combo, 1, 5, 1, 1);
        cam_grid.attach(&raster_fill_label, 0, 6, 1, 1);
        cam_grid.attach(&raster_fill_entry, 1, 6, 1, 1);
        cam_grid.attach(&raster_fill_hint, 0, 7, 3, 1);

        cam_frame.set_child(Some(&cam_grid));
        content.append(&cam_frame);

        // Empty state message
        let empty_label = Label::new(Some(&t!("Select a shape to edit its properties")));
        empty_label.add_css_class("dim-label");
        empty_label.set_wrap(true);
        empty_label.set_margin_top(24);
        content.append(&empty_label);

        scrolled.set_child(Some(&content));

        let panel = Rc::new(Self {
            widget: scrolled,
            state: state.clone(),
            settings: settings.clone(),
            _content: content,
            pos_frame: pos_frame.clone(),
            size_frame: size_frame.clone(),
            rot_frame: rot_frame.clone(),
            corner_frame: corner_frame.clone(),
            text_frame: text_frame.clone(),
            polygon_frame: polygon_frame.clone(),
            cam_frame: cam_frame.clone(),
            empty_label: empty_label.clone(),
            pos_x_entry: pos_x_entry.clone(),
            pos_y_entry: pos_y_entry.clone(),
            width_entry: width_entry.clone(),
            height_entry: height_entry.clone(),
            rotation_entry: rotation_entry.clone(),
            corner_radius_entry: corner_radius_entry.clone(),
            is_slot_check: is_slot_check.clone(),
            text_entry: text_entry.clone(),
            font_family_combo: font_family_combo.clone(),
            font_bold_check: font_bold_check.clone(),
            font_italic_check: font_italic_check.clone(),
            font_size_entry: font_size_entry.clone(),
            sides_entry: sides_entry.clone(),
            op_type_combo: op_type_combo.clone(),
            depth_entry: depth_entry.clone(),
            step_down_entry: step_down_entry.clone(),
            step_in_entry: step_in_entry.clone(),
            ramp_angle_entry: ramp_angle_entry.clone(),
            strategy_combo: strategy_combo.clone(),
            raster_fill_entry: raster_fill_entry.clone(),
            header: header.clone(),
            x_unit_label,
            y_unit_label,
            width_unit_label,
            height_unit_label,
            radius_unit_label,
            font_size_unit_label,
            depth_unit_label,
            step_down_unit_label,
            step_in_unit_label,
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
        let settings = self.settings.clone();
        let _pos_x = self.pos_x_entry.clone();
        let pos_y = self.pos_y_entry.clone();
        let width = self.width_entry.clone();
        let height = self.height_entry.clone();
        let redraw1 = self.redraw_callback.clone();
        let updating1 = self.updating.clone();

        // Position X changed
        self.pos_x_entry.connect_changed(move |entry| {
            if *updating1.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(val) = units::parse_length(&entry.text(), system) {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                    let y = units::parse_length(&pos_y.text(), system).unwrap_or(0.0) as f64;
                    let w = units::parse_length(&width.text(), system).unwrap_or(0.0) as f64;
                    let h = units::parse_length(&height.text(), system).unwrap_or(0.0) as f64;
                    let x = val as f64;

                    // Update shape position
                    if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                        Self::update_shape_position_and_size(&mut obj.shape, x, y, w, h);
                    }
                }
                drop(designer_state);
                if let Some(ref cb) = *redraw1.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let settings = self.settings.clone();
        let pos_x = self.pos_x_entry.clone();
        let _pos_y = self.pos_y_entry.clone();
        let width = self.width_entry.clone();
        let height = self.height_entry.clone();
        let redraw2 = self.redraw_callback.clone();
        let updating2 = self.updating.clone();

        // Position Y changed
        self.pos_y_entry.connect_changed(move |entry| {
            if *updating2.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(val) = units::parse_length(&entry.text(), system) {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                    let x = units::parse_length(&pos_x.text(), system).unwrap_or(0.0) as f64;
                    let w = units::parse_length(&width.text(), system).unwrap_or(0.0) as f64;
                    let h = units::parse_length(&height.text(), system).unwrap_or(0.0) as f64;
                    let y = val as f64;

                    if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                        Self::update_shape_position_and_size(&mut obj.shape, x, y, w, h);
                    }
                }
                drop(designer_state);
                if let Some(ref cb) = *redraw2.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let settings = self.settings.clone();
        let pos_x = self.pos_x_entry.clone();
        let pos_y = self.pos_y_entry.clone();
        let _width = self.width_entry.clone();
        let height = self.height_entry.clone();
        let redraw3 = self.redraw_callback.clone();
        let updating3 = self.updating.clone();

        // Width changed
        self.width_entry.connect_changed(move |entry| {
            if *updating3.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(val) = units::parse_length(&entry.text(), system) {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                    let x = units::parse_length(&pos_x.text(), system).unwrap_or(0.0) as f64;
                    let y = units::parse_length(&pos_y.text(), system).unwrap_or(0.0) as f64;
                    let h = units::parse_length(&height.text(), system).unwrap_or(0.0) as f64;
                    let w = val as f64;

                    if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                        Self::update_shape_position_and_size(&mut obj.shape, x, y, w, h);
                    }
                }
                drop(designer_state);
                if let Some(ref cb) = *redraw3.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let settings = self.settings.clone();
        let pos_x = self.pos_x_entry.clone();
        let pos_y = self.pos_y_entry.clone();
        let width = self.width_entry.clone();
        let _height = self.height_entry.clone();
        let redraw4 = self.redraw_callback.clone();
        let updating4 = self.updating.clone();

        // Height changed
        self.height_entry.connect_changed(move |entry| {
            if *updating4.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(val) = units::parse_length(&entry.text(), system) {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                    let x = units::parse_length(&pos_x.text(), system).unwrap_or(0.0) as f64;
                    let y = units::parse_length(&pos_y.text(), system).unwrap_or(0.0) as f64;
                    let w = units::parse_length(&width.text(), system).unwrap_or(0.0) as f64;
                    let h = val as f64;

                    if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                        Self::update_shape_position_and_size(&mut obj.shape, x, y, w, h);
                    }
                }
                drop(designer_state);
                if let Some(ref cb) = *redraw4.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let redraw5 = self.redraw_callback.clone();
        let updating5 = self.updating.clone();

        // Rotation changed (degrees)
        self.rotation_entry.connect_changed(move |entry| {
            if *updating5.borrow() {
                return;
            }
            if let Ok(val) = entry.text().parse::<f64>() {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                designer_state.set_selected_rotation(val);
                drop(designer_state);
                if let Some(ref cb) = *redraw5.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let settings = self.settings.clone();
        let redraw_cr = self.redraw_callback.clone();
        let updating_cr = self.updating.clone();

        // Corner Radius changed
        self.corner_radius_entry.connect_changed(move |entry| {
            if *updating_cr.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(val) = units::parse_length(&entry.text(), system) {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                designer_state.set_selected_corner_radius(val as f64);
                drop(designer_state);
                if let Some(ref cb) = *redraw_cr.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let redraw_slot = self.redraw_callback.clone();
        let updating_slot = self.updating.clone();

        // Is Slot changed
        self.is_slot_check.connect_toggled(move |check| {
            if *updating_slot.borrow() {
                return;
            }
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_is_slot(check.is_active());
            drop(designer_state);
            if let Some(ref cb) = *redraw_slot.borrow() {
                cb();
            }
        });

        let state = self.state.clone();
        let redraw6 = self.redraw_callback.clone();
        let updating6 = self.updating.clone();

        // Text Content changed
        self.text_entry.connect_changed(move |entry| {
            if *updating6.borrow() {
                return;
            }
            let mut designer_state = state.borrow_mut();
            if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                let text = entry.text().to_string();

                if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                    if let Shape::Text(text_shape) = &mut obj.shape {
                        text_shape.text = text;
                    }
                }
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw6.borrow() {
                cb();
            }
        });

        let state = self.state.clone();
        let _settings = self.settings.clone();
        let redraw7 = self.redraw_callback.clone();
        let updating7 = self.updating.clone();

        // Font Size changed (entered in points; stored in mm)
        self.font_size_entry.connect_changed(move |entry| {
            if *updating7.borrow() {
                return;
            }
            if let Some(size_mm) = parse_font_points_mm(&entry.text()) {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                    if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                        if let Shape::Text(text_shape) = &mut obj.shape {
                            text_shape.font_size = size_mm;
                        }
                    }
                }
                drop(designer_state);
                if let Some(ref cb) = *redraw7.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let redraw_font = self.redraw_callback.clone();
        let updating_font = self.updating.clone();
        let bold_check = self.font_bold_check.clone();
        let italic_check = self.font_italic_check.clone();

        // Font family changed
        self.font_family_combo
            .connect_selected_notify(move |combo| {
                if *updating_font.borrow() {
                    return;
                }
                let family = combo
                    .selected_item()
                    .and_downcast::<gtk4::StringObject>()
                    .map(|s| s.string().to_string())
                    .unwrap_or_else(|| "Sans".to_string());

                let bold = bold_check.is_active();
                let italic = italic_check.is_active();

                let mut designer_state = state.borrow_mut();
                if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                    if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                        if let Shape::Text(text_shape) = &mut obj.shape {
                            text_shape.font_family = family;
                            text_shape.bold = bold;
                            text_shape.italic = italic;
                        }
                    }
                }
                drop(designer_state);
                if let Some(ref cb) = *redraw_font.borrow() {
                    cb();
                }
            });

        let state = self.state.clone();
        let redraw_style = self.redraw_callback.clone();
        let updating_style = self.updating.clone();
        let font_combo = self.font_family_combo.clone();
        let bold_check = self.font_bold_check.clone();
        let italic_check = self.font_italic_check.clone();

        // Font style changed (bold)
        bold_check.clone().connect_toggled(move |_| {
            if *updating_style.borrow() {
                return;
            }
            let family = font_combo
                .selected_item()
                .and_downcast::<gtk4::StringObject>()
                .map(|s| s.string().to_string())
                .unwrap_or_else(|| "Sans".to_string());
            let bold = bold_check.is_active();
            let italic = italic_check.is_active();

            let mut designer_state = state.borrow_mut();
            if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                    if let Shape::Text(text_shape) = &mut obj.shape {
                        text_shape.font_family = family;
                        text_shape.bold = bold;
                        text_shape.italic = italic;
                    }
                }
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_style.borrow() {
                cb();
            }
        });

        let state = self.state.clone();
        let redraw_style2 = self.redraw_callback.clone();
        let updating_style2 = self.updating.clone();
        let font_combo2 = self.font_family_combo.clone();
        let bold_check2 = self.font_bold_check.clone();
        let italic_check2 = self.font_italic_check.clone();

        // Font style changed (italic)
        italic_check2.clone().connect_toggled(move |_| {
            if *updating_style2.borrow() {
                return;
            }
            let family = font_combo2
                .selected_item()
                .and_downcast::<gtk4::StringObject>()
                .map(|s| s.string().to_string())
                .unwrap_or_else(|| "Sans".to_string());
            let bold = bold_check2.is_active();
            let italic = italic_check2.is_active();

            let mut designer_state = state.borrow_mut();
            if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                    if let Shape::Text(text_shape) = &mut obj.shape {
                        text_shape.font_family = family;
                        text_shape.bold = bold;
                        text_shape.italic = italic;
                    }
                }
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_style2.borrow() {
                cb();
            }
        });

        let state = self.state.clone();
        let redraw_sides = self.redraw_callback.clone();
        let updating_sides = self.updating.clone();

        // Sides changed
        self.sides_entry.connect_changed(move |entry| {
            if *updating_sides.borrow() {
                return;
            }
            if let Ok(val) = entry.text().parse::<u32>() {
                if val >= 3 {
                    entry.remove_css_class("entry-invalid");
                    let mut designer_state = state.borrow_mut();
                    if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
                        if let Some(obj) = designer_state.canvas.shape_store.get_mut(id) {
                            if let Shape::Polygon(polygon) = &mut obj.shape {
                                polygon.sides = val;
                            }
                        }
                    }
                    drop(designer_state);
                    if let Some(ref cb) = *redraw_sides.borrow() {
                        cb();
                    }
                } else {
                    entry.add_css_class("entry-invalid");
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let updating8 = self.updating.clone();

        // Operation Type changed
        self.op_type_combo.connect_selected_notify(move |combo| {
            if *updating8.borrow() {
                return;
            }
            let mut designer_state = state.borrow_mut();
            let is_pocket = combo.selected() == 1;
            let depth = designer_state
                .canvas
                .shapes()
                .find(|s| s.selected)
                .map(|s| s.pocket_depth)
                .unwrap_or(0.0);
            designer_state.set_selected_pocket_properties(is_pocket, depth);
        });

        let state = self.state.clone();
        let settings = self.settings.clone();
        let updating9 = self.updating.clone();
        let op_combo = self.op_type_combo.clone();

        // Pocket Depth changed
        self.depth_entry.connect_changed(move |entry| {
            if *updating9.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(val) = units::parse_length(&entry.text(), system) {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                let is_pocket = op_combo.selected() == 1;
                designer_state.set_selected_pocket_properties(is_pocket, val as f64);
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let settings = self.settings.clone();
        let updating10 = self.updating.clone();

        // Step Down changed
        self.step_down_entry.connect_changed(move |entry| {
            if *updating10.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(val) = units::parse_length(&entry.text(), system) {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                designer_state.set_selected_step_down(val as f64);
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let settings = self.settings.clone();
        let updating11 = self.updating.clone();

        // Step In changed
        self.step_in_entry.connect_changed(move |entry| {
            if *updating11.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(val) = units::parse_length(&entry.text(), system) {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                designer_state.set_selected_step_in(val as f64);
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let updating_raster = self.updating.clone();

        // Raster fill changed (percentage 0-100)
        self.raster_fill_entry.connect_changed(move |entry| {
            if *updating_raster.borrow() {
                return;
            }
            if let Ok(val) = entry.text().parse::<f64>() {
                let clamped = val.clamp(0.0, 100.0);
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                designer_state.set_selected_raster_fill_ratio(clamped / 100.0);
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let updating_ramp = self.updating.clone();

        // Ramp Angle changed
        self.ramp_angle_entry.connect_changed(move |entry| {
            if *updating_ramp.borrow() {
                return;
            }
            if let Ok(val) = entry.text().parse::<f64>() {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                designer_state.set_selected_ramp_angle(val);
            } else {
                entry.add_css_class("entry-invalid");
            }
        });

        let state = self.state.clone();
        let updating12 = self.updating.clone();

        // Strategy changed
        self.strategy_combo.connect_selected_notify(move |combo| {
            if *updating12.borrow() {
                return;
            }
            let mut designer_state = state.borrow_mut();
            let strategy = match combo.selected() {
                0 => PocketStrategy::Raster {
                    angle: 0.0,
                    bidirectional: true,
                },
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
                rect.center.x = x;
                rect.center.y = y;
                rect.width = width;
                rect.height = height;

                if rect.is_slot {
                    rect.corner_radius = rect.width.min(rect.height) / 2.0;
                }
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
            Shape::Path(path) => {
                // Calculate current center and size
                let (x1, y1, x2, y2) = path.bounds();
                let current_w = x2 - x1;
                let current_h = y2 - y1;
                let current_cx = (x1 + x2) / 2.0;
                let current_cy = (y1 + y2) / 2.0;

                // Calculate scale factors
                let sx = if current_w.abs() > 1e-6 {
                    width / current_w
                } else {
                    1.0
                };
                let sy = if current_h.abs() > 1e-6 {
                    height / current_h
                } else {
                    1.0
                };

                // Scale around current center
                path.scale(sx, sy, Point::new(current_cx, current_cy));

                // Translate to new center
                path.translate(x - current_cx, y - current_cy);
            }
            Shape::Text(text) => {
                text.x = x;
                text.y = y;
                // Width/Height are derived from font size and content, so we don't update them here
                // unless we want to implement scaling via width/height
            }
            Shape::Triangle(triangle) => {
                triangle.center.x = x;
                triangle.center.y = y;
                triangle.width = width;
                triangle.height = height;
            }
            Shape::Polygon(polygon) => {
                polygon.center.x = x;
                polygon.center.y = y;
                polygon.radius = width.min(height) / 2.0;
            }
        }
    }

    pub fn update_from_selection(&self) {
        // Don't update if any widget has focus (user is editing)
        if *self.has_focus.borrow() {
            return;
        }

        // Get current measurement system
        let system = self.settings.borrow().config().ui.measurement_system;
        let unit_label = units::get_unit_label(system);

        // Update unit labels
        self.x_unit_label.set_text(unit_label);
        self.y_unit_label.set_text(unit_label);
        self.width_unit_label.set_text(unit_label);
        self.height_unit_label.set_text(unit_label);
        self.radius_unit_label.set_text(unit_label);
        self.font_size_unit_label.set_text("pt");
        self.depth_unit_label.set_text(unit_label);
        self.step_down_unit_label.set_text(unit_label);
        self.step_in_unit_label.set_text(unit_label);

        // Extract data first to avoid holding the borrow while updating widgets
        let selection_data = {
            let designer_state = self.state.borrow();
            let selected: Vec<_> = designer_state
                .canvas
                .shapes()
                .filter(|s| s.selected)
                .collect();

            if selected.is_empty() {
                None
            } else if selected.len() == 1 {
                // Single selection - show all properties
                let obj = &selected[0];
                Some((
                    vec![obj.id],
                    Some(obj.shape.clone()),
                    obj.operation_type,
                    obj.pocket_depth,
                    obj.step_down,
                    obj.step_in,
                    obj.ramp_angle,
                    obj.pocket_strategy,
                    obj.raster_fill_ratio,
                ))
            } else {
                // Multiple selection - only show CAM properties (use first shape's values)
                let obj = &selected[0];
                Some((
                    selected.iter().map(|s| s.id).collect(),
                    None, // No shape data for multi-selection
                    obj.operation_type,
                    obj.pocket_depth,
                    obj.step_down,
                    obj.step_in,
                    obj.ramp_angle,
                    obj.pocket_strategy,
                    obj.raster_fill_ratio,
                ))
            }
        };

        if let Some((
            ids,
            shape_opt,
            op_type,
            depth,
            step_down,
            step_in,
            ramp_angle,
            strategy,
            raster_fill,
        )) = selection_data
        {
            // Set flag to prevent feedback loop during updates
            *self.updating.borrow_mut() = true;

            // Update header with shape ID(s)
            if ids.len() == 1 {
                self.header
                    .set_text(&format!("{} [{}]", t!("Properties"), ids[0]));
            } else {
                self.header.set_text(&format!(
                    "{} [{} {}]",
                    t!("Properties"),
                    ids.len(),
                    t!("shapes")
                ));
            }

            self.empty_label.set_visible(false);

            if ids.len() == 1 {
                self.pos_frame.set_visible(true);
                self.size_frame.set_visible(true);
                self.rot_frame.set_visible(true);
                self.cam_frame.set_visible(true);
            } else {
                // Multi-select: CAM only
                self.pos_frame.set_visible(false);
                self.size_frame.set_visible(false);
                self.rot_frame.set_visible(false);
                self.corner_frame.set_visible(false);
                self.text_frame.set_visible(false);
                self.polygon_frame.set_visible(false);
                self.cam_frame.set_visible(true);
            }

            // Clear validation state on programmatic updates
            for e in [
                &self.pos_x_entry,
                &self.pos_y_entry,
                &self.width_entry,
                &self.height_entry,
                &self.rotation_entry,
                &self.corner_radius_entry,
                &self.font_size_entry,
                &self.sides_entry,
                &self.depth_entry,
                &self.step_down_entry,
                &self.step_in_entry,
                &self.ramp_angle_entry,
            ] {
                e.remove_css_class("entry-invalid");
            }

            if let Some(shape) = shape_opt {
                // Single selection - show all properties
                // Enable all controls by default
                self.pos_x_entry.set_sensitive(true);
                self.pos_y_entry.set_sensitive(true);
                self.width_entry.set_sensitive(true);
                self.height_entry.set_sensitive(true);
                self.rotation_entry.set_sensitive(true);

                // Update spin buttons based on shape type
                match shape {
                    Shape::Rectangle(rect) => {
                        self.corner_frame.set_visible(true);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);

                        self.pos_x_entry
                            .set_text(&units::format_length(rect.center.x as f32, system));
                        self.pos_y_entry
                            .set_text(&units::format_length(rect.center.y as f32, system));
                        self.width_entry
                            .set_text(&units::format_length(rect.width as f32, system));
                        self.height_entry
                            .set_text(&units::format_length(rect.height as f32, system));
                        self.rotation_entry
                            .set_text(&format!("{:.1}", rect.rotation.to_degrees()));

                        self.corner_radius_entry
                            .set_text(&units::format_length(rect.corner_radius as f32, system));
                        self.is_slot_check.set_active(rect.is_slot);

                        self.corner_radius_entry.set_sensitive(!rect.is_slot);
                        self.is_slot_check.set_sensitive(true);

                        self.text_entry.set_text("");
                        self.text_entry.set_sensitive(false);
                        self.font_size_entry
                            .set_text(&units::format_length(0.0, system));
                        self.font_size_entry.set_sensitive(false);
                    }
                    Shape::Circle(circle) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);

                        self.pos_x_entry
                            .set_text(&units::format_length(circle.center.x as f32, system));
                        self.pos_y_entry
                            .set_text(&units::format_length(circle.center.y as f32, system));
                        self.width_entry
                            .set_text(&units::format_length((circle.radius * 2.0) as f32, system));
                        self.height_entry
                            .set_text(&units::format_length((circle.radius * 2.0) as f32, system));
                        self.rotation_entry
                            .set_text(&format!("{:.1}", circle.rotation.to_degrees()));

                        self.corner_radius_entry.set_sensitive(false);
                        self.is_slot_check.set_sensitive(false);

                        self.text_entry.set_text("");
                        self.text_entry.set_sensitive(false);
                        self.font_size_entry.set_text(&format_font_points(0.0));
                        self.font_size_entry.set_sensitive(false);
                    }
                    Shape::Ellipse(ellipse) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);

                        self.pos_x_entry
                            .set_text(&units::format_length(ellipse.center.x as f32, system));
                        self.pos_y_entry
                            .set_text(&units::format_length(ellipse.center.y as f32, system));
                        self.width_entry
                            .set_text(&units::format_length((ellipse.rx * 2.0) as f32, system));
                        self.height_entry
                            .set_text(&units::format_length((ellipse.ry * 2.0) as f32, system));
                        self.rotation_entry
                            .set_text(&format!("{:.1}", ellipse.rotation.to_degrees()));

                        self.corner_radius_entry.set_sensitive(false);
                        self.is_slot_check.set_sensitive(false);

                        self.text_entry.set_text("");
                        self.text_entry.set_sensitive(false);
                        self.font_size_entry.set_text(&format_font_points(0.0));
                        self.font_size_entry.set_sensitive(false);
                    }
                    Shape::Line(line) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);

                        self.pos_x_entry
                            .set_text(&units::format_length(line.start.x as f32, system));
                        self.pos_y_entry
                            .set_text(&units::format_length(line.start.y as f32, system));
                        self.width_entry.set_text(&units::format_length(
                            (line.end.x - line.start.x) as f32,
                            system,
                        ));
                        self.height_entry.set_text(&units::format_length(
                            (line.end.y - line.start.y) as f32,
                            system,
                        ));
                        self.rotation_entry
                            .set_text(&format!("{:.1}", line.rotation));

                        self.corner_radius_entry.set_sensitive(false);
                        self.is_slot_check.set_sensitive(false);

                        self.text_entry.set_text("");
                        self.text_entry.set_sensitive(false);
                        self.font_size_entry.set_text(&format_font_points(0.0));
                        self.font_size_entry.set_sensitive(false);
                    }
                    Shape::Path(path) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);

                        let (x1, y1, x2, y2) = path.bounds();
                        let w = x2 - x1;
                        let h = y2 - y1;
                        let cx = (x1 + x2) / 2.0;
                        let cy = (y1 + y2) / 2.0;

                        self.pos_x_entry
                            .set_text(&units::format_length(cx as f32, system));
                        self.pos_y_entry
                            .set_text(&units::format_length(cy as f32, system));
                        self.width_entry
                            .set_text(&units::format_length(w as f32, system));
                        self.height_entry
                            .set_text(&units::format_length(h as f32, system));
                        self.rotation_entry
                            .set_text(&format!("{:.1}", path.rotation.to_degrees()));

                        self.corner_radius_entry.set_sensitive(false);
                        self.is_slot_check.set_sensitive(false);

                        self.text_entry.set_text("");
                        self.text_entry.set_sensitive(false);
                        self.font_size_entry.set_text(&format_font_points(0.0));
                        self.font_size_entry.set_sensitive(false);
                    }
                    Shape::Text(text) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(true);
                        self.polygon_frame.set_visible(false);

                        self.pos_x_entry
                            .set_text(&units::format_length(text.x as f32, system));
                        self.pos_y_entry
                            .set_text(&units::format_length(text.y as f32, system));
                        // Width/Height are derived, maybe show bounding box size?
                        let (x1, y1, x2, y2) = text.bounds();
                        self.width_entry
                            .set_text(&units::format_length((x2 - x1) as f32, system));
                        self.height_entry
                            .set_text(&units::format_length((y2 - y1) as f32, system));
                        self.rotation_entry
                            .set_text(&format!("{:.1}", text.rotation.to_degrees()));

                        self.width_entry.set_sensitive(false);
                        self.height_entry.set_sensitive(false);

                        self.corner_radius_entry.set_sensitive(false);
                        self.is_slot_check.set_sensitive(false);

                        self.text_entry.set_text(&text.text);
                        self.text_entry.set_sensitive(true);
                        self.font_size_entry
                            .set_text(&format_font_points(text.font_size));
                        self.font_size_entry.set_sensitive(true);

                        self.font_family_combo.set_sensitive(true);
                        self.font_bold_check.set_sensitive(true);
                        self.font_italic_check.set_sensitive(true);

                        let family = if text.font_family.is_empty() {
                            "Sans"
                        } else {
                            &text.font_family
                        };
                        let mut family_idx = 0;
                        if let Some(model) =
                            self.font_family_combo.model().and_downcast::<StringList>()
                        {
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
                        self.font_family_combo.set_selected(family_idx);
                        self.font_bold_check.set_active(text.bold);
                        self.font_italic_check.set_active(text.italic);
                    }
                    Shape::Triangle(triangle) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(false);

                        self.pos_x_entry
                            .set_text(&units::format_length(triangle.center.x as f32, system));
                        self.pos_y_entry
                            .set_text(&units::format_length(triangle.center.y as f32, system));
                        self.width_entry
                            .set_text(&units::format_length(triangle.width as f32, system));
                        self.height_entry
                            .set_text(&units::format_length(triangle.height as f32, system));
                        self.rotation_entry
                            .set_text(&format!("{:.1}", triangle.rotation.to_degrees()));

                        self.corner_radius_entry.set_sensitive(false);
                        self.is_slot_check.set_sensitive(false);

                        self.text_entry.set_text("");
                        self.text_entry.set_sensitive(false);
                        self.font_size_entry.set_text(&format_font_points(0.0));
                        self.font_size_entry.set_sensitive(false);
                    }
                    Shape::Polygon(polygon) => {
                        self.corner_frame.set_visible(false);
                        self.text_frame.set_visible(false);
                        self.polygon_frame.set_visible(true);

                        self.pos_x_entry
                            .set_text(&units::format_length(polygon.center.x as f32, system));
                        self.pos_y_entry
                            .set_text(&units::format_length(polygon.center.y as f32, system));
                        self.width_entry
                            .set_text(&units::format_length((polygon.radius * 2.0) as f32, system));
                        self.height_entry
                            .set_text(&units::format_length((polygon.radius * 2.0) as f32, system));
                        self.rotation_entry
                            .set_text(&format!("{:.1}", polygon.rotation.to_degrees()));
                        self.sides_entry.set_text(&polygon.sides.to_string());

                        self.corner_radius_entry.set_sensitive(false);
                        self.is_slot_check.set_sensitive(false);

                        self.text_entry.set_text("");
                        self.text_entry.set_sensitive(false);
                        self.font_size_entry.set_text(&format_font_points(0.0));
                        self.font_size_entry.set_sensitive(false);
                    }
                }
            } else {
                // Multiple selection - disable geometry properties
                self.pos_x_entry.set_sensitive(false);
                self.pos_y_entry.set_sensitive(false);
                self.width_entry.set_sensitive(false);
                self.height_entry.set_sensitive(false);
                self.rotation_entry.set_sensitive(false);
                self.corner_radius_entry.set_sensitive(false);
                self.is_slot_check.set_sensitive(false);
                self.text_entry.set_sensitive(false);
                self.font_size_entry.set_sensitive(false);

                // Clear values
                self.pos_x_entry.set_text("");
                self.pos_y_entry.set_text("");
                self.width_entry.set_text("");
                self.height_entry.set_text("");
                self.rotation_entry.set_text("");
                self.corner_radius_entry.set_text("");
                self.text_entry.set_text("");
                self.font_size_entry.set_text("");
            }

            // Update CAM properties (always enabled for single or multi-selection)
            self.op_type_combo.set_selected(match op_type {
                OperationType::Profile => 0,
                OperationType::Pocket => 1,
            });
            self.depth_entry
                .set_text(&units::format_length(depth as f32, system));
            self.step_down_entry
                .set_text(&units::format_length(step_down as f32, system));
            self.step_in_entry
                .set_text(&units::format_length(step_in as f32, system));
            self.ramp_angle_entry
                .set_text(&format!("{:.1}", ramp_angle));
            self.strategy_combo.set_selected(match strategy {
                PocketStrategy::Raster { .. } => 0,
                PocketStrategy::ContourParallel => 1,
                PocketStrategy::Adaptive => 2,
            });
            self.raster_fill_entry
                .set_text(&format!("{:.0}", raster_fill * 100.0));

            // Enable CAM controls
            self.op_type_combo.set_sensitive(true);
            self.depth_entry.set_sensitive(true);
            self.step_down_entry.set_sensitive(true);
            self.step_in_entry.set_sensitive(true);
            self.ramp_angle_entry.set_sensitive(true);
            self.strategy_combo.set_sensitive(true);
            self.raster_fill_entry.set_sensitive(true);

            // Clear flag
            *self.updating.borrow_mut() = false;
        } else {
            // No selection - reset header and disable controls
            self.header.set_text(&t!("Properties"));

            self.empty_label.set_visible(true);
            self.pos_frame.set_visible(false);
            self.size_frame.set_visible(false);
            self.rot_frame.set_visible(false);
            self.corner_frame.set_visible(false);
            self.text_frame.set_visible(false);
            self.polygon_frame.set_visible(false);
            self.cam_frame.set_visible(false);

            self.pos_x_entry.set_sensitive(false);
            self.pos_y_entry.set_sensitive(false);
            self.width_entry.set_sensitive(false);
            self.height_entry.set_sensitive(false);
            self.rotation_entry.set_sensitive(false);

            self.corner_radius_entry.set_sensitive(false);
            self.is_slot_check.set_sensitive(false);

            self.text_entry.set_sensitive(false);
            self.font_size_entry.set_sensitive(false);

            self.op_type_combo.set_sensitive(false);
            self.depth_entry.set_sensitive(false);
            self.step_down_entry.set_sensitive(false);
            self.step_in_entry.set_sensitive(false);
            self.ramp_angle_entry.set_sensitive(false);
            self.strategy_combo.set_sensitive(false);
            self.raster_fill_entry.set_sensitive(false);

            self.raster_fill_entry.set_text("");
        }
    }

    fn setup_focus_tracking(&self) {
        // Track focus for all entries to prevent updates while user is editing
        let entries = vec![
            &self.pos_x_entry,
            &self.pos_y_entry,
            &self.width_entry,
            &self.height_entry,
            &self.rotation_entry,
            &self.corner_radius_entry,
            &self.font_size_entry,
            &self.depth_entry,
            &self.step_down_entry,
            &self.step_in_entry,
            &self.ramp_angle_entry,
            &self.raster_fill_entry,
            &self.sides_entry,
        ];

        for entry in entries {
            let focus_controller = EventControllerFocus::new();
            let has_focus_enter = self.has_focus.clone();
            focus_controller.connect_enter(move |_| {
                *has_focus_enter.borrow_mut() = true;
            });

            let has_focus_leave = self.has_focus.clone();
            focus_controller.connect_leave(move |_| {
                *has_focus_leave.borrow_mut() = false;
            });

            entry.add_controller(focus_controller);
        }

        // Track focus for text entry (content)
        let focus_controller = EventControllerFocus::new();
        let has_focus_enter = self.has_focus.clone();
        focus_controller.connect_enter(move |_| {
            *has_focus_enter.borrow_mut() = true;
        });

        let has_focus_leave = self.has_focus.clone();
        focus_controller.connect_leave(move |_| {
            *has_focus_leave.borrow_mut() = false;
        });
        self.text_entry.add_controller(focus_controller);
    }

    /// Clear the focus flag - call this when user interacts with the canvas
    pub fn clear_focus(&self) {
        *self.has_focus.borrow_mut() = false;
    }
}
