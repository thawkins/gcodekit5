//! Properties panel for the designer.
//!
//! This module provides the property editing panel shown on the right side of the designer.
//! It's organized into:
//! - Main panel orchestration (this file)
//! - Handlers for different property categories (handlers/)

mod builders;
mod handlers;
mod update;

use crate::t;
use gcodekit5_core::units;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::font_manager;
use gcodekit5_designer::model::{DesignerShape, Shape};
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

fn format_font_points(mm: f64) -> String {
    format!("{:.2}", mm_to_pt(mm))
}

/// Properties panel showing editable properties for selected shapes.
#[allow(clippy::type_complexity)]
pub struct PropertiesPanel {
    pub widget: ScrolledWindow,
    pub(crate) state: Rc<RefCell<DesignerState>>,
    pub(crate) settings: Rc<RefCell<SettingsPersistence>>,
    pub(crate) preview_shapes: Rc<RefCell<Vec<Shape>>>,
    pub(crate) _content: Box,
    pub(crate) header: Label,

    // Sections (visibility controlled)
    pub(crate) pos_frame: Frame,
    pub(crate) size_frame: Frame,
    pub(crate) rot_frame: Frame,
    pub(crate) corner_frame: Frame,
    pub(crate) text_frame: Frame,
    pub(crate) cam_frame: Frame,
    pub(crate) ops_frame: Frame,
    pub(crate) empty_label: Label,

    // Property widgets
    pub(crate) pos_x_entry: Entry,
    pub(crate) pos_y_entry: Entry,
    pub(crate) width_entry: Entry,
    pub(crate) height_entry: Entry,
    pub(crate) lock_aspect_ratio: CheckButton,
    pub(crate) rotation_entry: Entry,
    // Rectangle widgets
    pub(crate) corner_radius_entry: Entry,
    pub(crate) is_slot_check: CheckButton,
    // Text widgets
    pub(crate) text_entry: Entry,
    pub(crate) font_family_combo: DropDown,
    pub(crate) font_bold_check: CheckButton,
    pub(crate) font_italic_check: CheckButton,
    pub(crate) font_size_entry: Entry,
    // Polygon widgets
    pub(crate) polygon_frame: Frame,
    pub(crate) sides_entry: Entry,

    // Gear widgets
    pub(crate) gear_frame: Frame,
    pub(crate) gear_module_entry: Entry,
    pub(crate) gear_teeth_entry: Entry,
    pub(crate) gear_pressure_angle_entry: Entry,

    // Sprocket widgets
    pub(crate) sprocket_frame: Frame,
    pub(crate) sprocket_pitch_entry: Entry,
    pub(crate) sprocket_teeth_entry: Entry,
    pub(crate) sprocket_roller_diameter_entry: Entry,

    // CAM widgets
    pub(crate) op_type_combo: DropDown,
    pub(crate) depth_entry: Entry,
    pub(crate) step_down_entry: Entry,
    pub(crate) step_in_entry: Entry,
    pub(crate) ramp_angle_entry: Entry,
    pub(crate) strategy_combo: DropDown,
    pub(crate) raster_fill_entry: Entry,

    // Geometry Ops widgets
    pub(crate) offset_entry: Entry,
    pub(crate) fillet_entry: Entry,
    pub(crate) chamfer_entry: Entry,

    // Unit Labels
    pub(crate) x_unit_label: Label,
    pub(crate) y_unit_label: Label,
    pub(crate) width_unit_label: Label,
    pub(crate) height_unit_label: Label,
    pub(crate) radius_unit_label: Label,
    pub(crate) font_size_unit_label: Label,
    pub(crate) depth_unit_label: Label,
    pub(crate) step_down_unit_label: Label,
    pub(crate) step_in_unit_label: Label,
    pub(crate) offset_unit_label: Label,
    pub(crate) fillet_unit_label: Label,
    pub(crate) chamfer_unit_label: Label,
    // Redraw callback
    pub(crate) redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    // Flag to prevent feedback loops during updates
    pub(crate) updating: Rc<RefCell<bool>>,
    // Flag to track if any widget has focus (being edited)
    pub(crate) has_focus: Rc<RefCell<bool>>,
    // Aspect ratio (width/height) for locked resizing
    pub(crate) aspect_ratio: Rc<RefCell<f64>>,
}

impl PropertiesPanel {
    /// Create a new properties panel.
    pub fn new(
        state: Rc<RefCell<DesignerState>>,
        settings: Rc<RefCell<SettingsPersistence>>,
        preview_shapes: Rc<RefCell<Vec<Shape>>>,
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

        // Build all UI sections
        let (pos_frame, pos_x_entry, pos_y_entry, x_unit_label, y_unit_label) =
            Self::build_position_section();
        content.append(&pos_frame);

        let (
            size_frame,
            width_entry,
            height_entry,
            lock_aspect_ratio,
            width_unit_label,
            height_unit_label,
        ) = Self::build_size_section();
        content.append(&size_frame);

        let (rot_frame, rotation_entry) = Self::build_rotation_section();
        content.append(&rot_frame);

        let (corner_frame, corner_radius_entry, is_slot_check, radius_unit_label) =
            Self::build_corner_section();
        content.append(&corner_frame);

        let (
            text_frame,
            text_entry,
            font_family_combo,
            font_bold_check,
            font_italic_check,
            font_size_entry,
            font_size_unit_label,
        ) = Self::build_text_section();
        content.append(&text_frame);

        let (polygon_frame, sides_entry) = Self::build_polygon_section();
        content.append(&polygon_frame);

        let (gear_frame, gear_module_entry, gear_teeth_entry, gear_pressure_angle_entry) =
            Self::build_gear_section();
        content.append(&gear_frame);

        let (
            sprocket_frame,
            sprocket_pitch_entry,
            sprocket_teeth_entry,
            sprocket_roller_diameter_entry,
        ) = Self::build_sprocket_section();
        content.append(&sprocket_frame);

        let (
            ops_frame,
            offset_entry,
            fillet_entry,
            chamfer_entry,
            offset_unit_label,
            fillet_unit_label,
            chamfer_unit_label,
        ) = Self::build_geometry_ops_section();
        content.append(&ops_frame);

        let (
            cam_frame,
            op_type_combo,
            depth_entry,
            step_down_entry,
            step_in_entry,
            ramp_angle_entry,
            strategy_combo,
            raster_fill_entry,
            depth_unit_label,
            step_down_unit_label,
            step_in_unit_label,
        ) = Self::build_cam_section();
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
            preview_shapes: preview_shapes.clone(),
            _content: content,
            pos_frame,
            size_frame,
            rot_frame,
            corner_frame,
            text_frame,
            polygon_frame,
            gear_frame,
            sprocket_frame,
            cam_frame,
            ops_frame,
            empty_label,
            pos_x_entry,
            pos_y_entry,
            width_entry,
            height_entry,
            rotation_entry,
            corner_radius_entry,
            is_slot_check,
            text_entry,
            font_family_combo,
            font_bold_check,
            font_italic_check,
            font_size_entry,
            sides_entry,
            gear_module_entry,
            gear_teeth_entry,
            gear_pressure_angle_entry,
            sprocket_pitch_entry,
            sprocket_teeth_entry,
            sprocket_roller_diameter_entry,
            op_type_combo,
            depth_entry,
            step_down_entry,
            step_in_entry,
            ramp_angle_entry,
            strategy_combo,
            raster_fill_entry,
            offset_entry,
            fillet_entry,
            chamfer_entry,
            header,
            x_unit_label,
            y_unit_label,
            width_unit_label,
            height_unit_label,
            radius_unit_label,
            font_size_unit_label,
            depth_unit_label,
            step_down_unit_label,
            step_in_unit_label,
            offset_unit_label,
            fillet_unit_label,
            chamfer_unit_label,
            lock_aspect_ratio,
            redraw_callback: Rc::new(RefCell::new(None)),
            updating: Rc::new(RefCell::new(false)),
            has_focus: Rc::new(RefCell::new(bool::default())),
            aspect_ratio: Rc::new(RefCell::new(1.0)),
        });

        // Connect value change handlers
        panel.setup_handlers();

        // Setup focus tracking for all spin buttons
        panel.setup_focus_tracking();

        panel
    }

    /// Set the callback to redraw the canvas.
    pub fn set_redraw_callback<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.redraw_callback.borrow_mut() = Some(Rc::new(callback));
    }

    /// Setup all property change handlers using modular handler functions.
    fn setup_handlers(&self) {
        // Dimension handlers
        handlers::dimensions::setup_position_x_handler(
            &self.pos_x_entry,
            &self.pos_y_entry,
            &self.width_entry,
            &self.height_entry,
            self.state.clone(),
            self.settings.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::dimensions::setup_position_y_handler(
            &self.pos_y_entry,
            &self.pos_x_entry,
            &self.width_entry,
            &self.height_entry,
            self.state.clone(),
            self.settings.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::dimensions::setup_width_activate_handler(
            &self.width_entry,
            &self.height_entry,
            &self.pos_x_entry,
            &self.pos_y_entry,
            &self.lock_aspect_ratio,
            self.aspect_ratio.clone(),
            self.state.clone(),
            self.settings.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::dimensions::setup_height_activate_handler(
            &self.height_entry,
            &self.width_entry,
            &self.pos_x_entry,
            &self.pos_y_entry,
            &self.lock_aspect_ratio,
            self.aspect_ratio.clone(),
            self.state.clone(),
            self.settings.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::dimensions::setup_lock_aspect_handler(
            &self.lock_aspect_ratio,
            &self.width_entry,
            &self.height_entry,
            self.aspect_ratio.clone(),
            self.state.clone(),
            self.settings.clone(),
        );

        handlers::dimensions::setup_width_focus_out_handler(
            &self.width_entry,
            &self.height_entry,
            &self.pos_x_entry,
            &self.pos_y_entry,
            &self.lock_aspect_ratio,
            self.aspect_ratio.clone(),
            self.state.clone(),
            self.settings.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::dimensions::setup_height_focus_out_handler(
            &self.height_entry,
            &self.width_entry,
            &self.pos_x_entry,
            &self.pos_y_entry,
            &self.lock_aspect_ratio,
            self.aspect_ratio.clone(),
            self.state.clone(),
            self.settings.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        // Geometry handlers
        handlers::geometry::setup_rotation_handler(
            &self.rotation_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::geometry::setup_corner_radius_handler(
            &self.corner_radius_entry,
            self.state.clone(),
            self.settings.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::geometry::setup_is_slot_handler(
            &self.is_slot_check,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::geometry::setup_sides_handler(
            &self.sides_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        // Text handlers
        handlers::text::setup_text_content_handler(
            &self.text_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::text::setup_font_size_handler(
            &self.font_size_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::text::setup_font_family_handler(
            &self.font_family_combo,
            &self.font_bold_check,
            &self.font_italic_check,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::text::setup_font_bold_handler(
            &self.font_bold_check,
            &self.font_family_combo,
            &self.font_italic_check,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::text::setup_font_italic_handler(
            &self.font_italic_check,
            &self.font_family_combo,
            &self.font_bold_check,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        // CAM handlers
        handlers::cam::setup_operation_type_handler(
            &self.op_type_combo,
            self.state.clone(),
            self.updating.clone(),
        );

        handlers::cam::setup_depth_handler(
            &self.depth_entry,
            &self.op_type_combo,
            self.state.clone(),
            self.settings.clone(),
            self.updating.clone(),
        );

        handlers::cam::setup_step_down_handler(
            &self.step_down_entry,
            self.state.clone(),
            self.settings.clone(),
            self.updating.clone(),
        );

        handlers::cam::setup_step_in_handler(
            &self.step_in_entry,
            self.state.clone(),
            self.settings.clone(),
            self.updating.clone(),
        );

        handlers::cam::setup_raster_fill_handler(
            &self.raster_fill_entry,
            self.state.clone(),
            self.updating.clone(),
        );

        handlers::cam::setup_ramp_angle_handler(
            &self.ramp_angle_entry,
            self.state.clone(),
            self.updating.clone(),
        );

        handlers::cam::setup_strategy_handler(
            &self.strategy_combo,
            self.state.clone(),
            self.updating.clone(),
        );

        // Gear/Sprocket handlers
        handlers::gear_sprocket::setup_gear_module_handler(
            &self.gear_module_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::gear_sprocket::setup_gear_teeth_handler(
            &self.gear_teeth_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::gear_sprocket::setup_gear_pressure_angle_handler(
            &self.gear_pressure_angle_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::gear_sprocket::setup_sprocket_pitch_handler(
            &self.sprocket_pitch_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::gear_sprocket::setup_sprocket_teeth_handler(
            &self.sprocket_teeth_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        handlers::gear_sprocket::setup_sprocket_roller_diameter_handler(
            &self.sprocket_roller_diameter_entry,
            self.state.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
        );

        // Effects handlers
        handlers::effects::setup_offset_handler(
            &self.offset_entry,
            self.state.clone(),
            self.preview_shapes.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
            self.has_focus.clone(),
        );

        handlers::effects::setup_fillet_handler(
            &self.fillet_entry,
            self.state.clone(),
            self.preview_shapes.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
            self.has_focus.clone(),
        );

        handlers::effects::setup_chamfer_handler(
            &self.chamfer_entry,
            self.state.clone(),
            self.preview_shapes.clone(),
            self.redraw_callback.clone(),
            self.updating.clone(),
            self.has_focus.clone(),
        );
    }
}
