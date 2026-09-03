//! # G-code Visualizer Widget
//!
//! GTK4 GLArea-based widget for 3D visualization of G-code toolpaths.
//! Handles OpenGL context setup, camera control, and rendering pipeline
//! coordination.
//!
//! ## Thread Safety
//! The visualizer uses `Rc<RefCell<>>` for state and must be accessed
//! from the GTK main thread only.

mod gl_loader;

use gcodekit5_core::constants as core_constants;
use gcodekit5_designer::stock_removal::{StockMaterial};
use gcodekit5_devicedb::DeviceManager;
use gcodekit5_visualizer::visualizer::GCodeCommand;
use gcodekit5_visualizer::{Camera3D, Visualizer};
use crate::t;
use crate::ui::gtk::common::spacing;
use crate::ui::gtk::osd_format::format_zoom_center_cursor;
use crate::ui::gtk::shaders::StockRemovalShaderProgram;
use crate::ui::gtk::status_bar::StatusBar;
use gcodekit5_settings::controller::SettingsController;
use gcodekit5_settings::manager::SettingsManager;
use gcodekit5_visualizer::visualizer::{generate_surface_mesh, StockSimulator3D};
use gcodekit5_designer::designer_state::MachineMode;
use gcodekit5_designer::DesignerState;
use glam::Vec3;

const STOCK_REMOVAL_RESOLUTION: f32 = 0.19; // Resolución cambiada de 0.25 a 0.19

#[derive(Clone)]
pub(crate) struct StockRemovalVisualization;

pub enum RunPreviewResult {
    Started,
    EmptyInput,
    NoMotion,
    NoTrajectory,
}

use crate::ui::gtk::nav_cube::NavCube;
use crate::ui::gtk::renderer_3d::{
    generate_axis_data, generate_bounds_data, generate_grid_data, generate_tool_marker_data,
    generate_vertex_data, RenderBuffers,
};
use crate::ui::gtk::shaders::ShaderProgram;
use glow::HasContext;
use gtk4::prelude::*;
use gtk4::{GestureClick, Popover, Separator};
use tracing::debug;

use gl_loader::load_gl_func;

use gcodekit5_core::{shared, shared_none, thread_safe_none, Shared, SharedOption};
use gtk4::prelude::{BoxExt, ButtonExt, CheckButtonExt, WidgetExt};
use gtk4::{
    accessible::Property as AccessibleProperty, Adjustment, Box, Button,
    CheckButton, ComboBoxText, DrawingArea, Entry, EventControllerMotion, EventControllerScroll,
    EventControllerScrollFlags, Expander, GLArea, GestureDrag, Grid, Image, Label, ListBox,
    ListBoxRow, Orientation, Overlay, Paned, Revealer, Scrollbar, SelectionMode, Spinner, Stack,
};
use std::rc::Rc;
use std::sync::Arc;

// Render cache for expensive computations
#[derive(Clone)]
pub(crate) struct RenderCache {
    pub(crate) cutting_bounds: Option<(f32, f32, f32, f32, f32, f32)>,
    pub(crate) _rapid_lines: usize,
}

pub(crate) struct RendererState {
    shader: ShaderProgram,
    rapid_buffers: RenderBuffers,
    cut_buffers: RenderBuffers,
    grid_buffers: RenderBuffers,
    axis_buffers: RenderBuffers,
    tool_buffers: RenderBuffers,
    bounds_buffers: RenderBuffers,
    stock_removal_shader: Option<StockRemovalShaderProgram>,
    stock_removal_buffers: Option<RenderBuffers>,
}

impl Default for RenderCache {
    fn default() -> Self {
        Self {
            cutting_bounds: None,
            _rapid_lines: 0,
        }
    }
}

pub struct GcodeVisualizer {
    pub widget: Paned,
    pub(crate) stack: Stack,
    pub(crate) drawing_area: DrawingArea,
    pub(crate) gl_area: GLArea,
    pub(crate) visualizer: Shared<Visualizer>,
    pub(crate) camera: Shared<Camera3D>,
    pub(crate) _renderer_state: SharedOption<RendererState>,
    // Render cache
    pub(crate) render_cache: Shared<RenderCache>,
    // Visibility toggles
    pub(crate) _show_rapid: CheckButton,
    pub(crate) _show_cut: CheckButton,
    pub(crate) _show_grid: CheckButton,
    pub(crate) _show_bounds: CheckButton,
    pub(crate) show_laser: CheckButton,
    pub(crate) show_stock_removal: CheckButton,
    #[allow(dead_code)]
    pub(crate) stock_material: SharedOption<StockMaterial>,
    pub(crate) _simulation_visualization: SharedOption<StockRemovalVisualization>,
    pub(crate) _simulation_running: Shared<bool>,
    // Stock removal simulation (3D)
    pub(crate) _stock_simulator_3d: SharedOption<StockSimulator3D>,
    pub(crate) _stock_simulation_3d_pending: Shared<bool>,
    #[allow(dead_code)]
    pub(crate) hadjustment: Adjustment,
    #[allow(dead_code)]
    pub(crate) vadjustment: Adjustment,
    pub(crate) hadjustment_3d: Adjustment,
    pub(crate) vadjustment_3d: Adjustment,
    // Info labels
    pub(crate) bounds_x_value: Label,
    pub(crate) bounds_y_value: Label,
    pub(crate) min_s_value: Label,
    pub(crate) max_s_value: Label,
    pub(crate) avg_s_value: Label,
    pub(crate) _status_label: Label,
    pub(crate) settings_controller: Rc<SettingsController>,
    stock_tool_diameter_entry: Entry,
    stock_tool_diameter_mm: Shared<f32>,
    stock_width_entry: Entry,
    stock_height_entry: Entry,
    stock_thickness_entry: Entry,
    // Optional status bar reference for future OSD integration.
    #[allow(dead_code)]
    pub(crate) status_bar: Option<StatusBar>,
    pub(crate) current_pos: Shared<(f32, f32, f32)>,
    pub(crate) fit_btn_3d: Button,
    run_preview_points: Shared<Vec<(f32, f32, f32)>>,
    run_preview_index: Shared<usize>,
    run_preview_token: Shared<u64>,
    run_preview_running: Shared<bool>,
    run_preview_paused: Shared<bool>,
    run_preview_speed: Shared<usize>,
    pub(crate) designer_state: Option<Shared<DesignerState>>, // Nuevo
}

impl GcodeVisualizer {
    /// Queue a redraw of the visualizer
    pub fn queue_draw(&self) {
        self.drawing_area.queue_draw();
        self.gl_area.queue_render();
    }

    pub fn set_current_position(&self, x: f32, y: f32, z: f32) {
        *self.current_pos.borrow_mut() = (x, y, z);
        if self.show_laser.is_active() {
            self.drawing_area.queue_draw();
            self.gl_area.queue_render();
        }
    }

    /// Sync stock-removal tool diameter from designer tool settings.
    pub fn set_stock_tool_diameter_mm(&self, diameter_mm: f64) {
        if !diameter_mm.is_finite() || diameter_mm <= 0.0 {
            return;
        }

        let diameter = diameter_mm as f32;
        *self.stock_tool_diameter_mm.borrow_mut() = diameter;
        self.stock_tool_diameter_entry
            .set_text(&format!("{:.3}", diameter));
    }

    fn build_run_preview_points(
        commands: &[gcodekit5_visualizer::visualizer::GCodeCommand],
    ) -> Vec<(f32, f32, f32)> {
        let mut preview_points: Vec<(f32, f32, f32)> = Vec::new();

        for cmd in commands {
            match cmd {
                gcodekit5_visualizer::visualizer::GCodeCommand::Move { from, to, .. }
                | gcodekit5_visualizer::visualizer::GCodeCommand::Arc { from, to, .. } => {
                    let dx = to.x - from.x;
                    let dy = to.y - from.y;
                    let dz = to.z - from.z;
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                    let steps = ((distance / 3.0).ceil() as usize).clamp(2, 24);
                    for s in 1..=steps {
                        let t = s as f32 / steps as f32;
                        preview_points.push((from.x + dx * t, from.y + dy * t, from.z + dz * t));
                    }
                }
                gcodekit5_visualizer::visualizer::GCodeCommand::Dwell { pos, duration } => {
                    let hold_steps = ((*duration).max(0.0) * 8.0).round() as usize;
                    for _ in 0..hold_steps.clamp(1, 24) {
                        preview_points.push((pos.x, pos.y, pos.z));
                    }
                }
            }

            if preview_points.len() >= 40_000 {
                break;
            }
        }

        preview_points
    }

    pub fn run_preview_from_gcode(&self, gcode: &str) -> RunPreviewResult {
        if gcode.trim().is_empty() {
            return RunPreviewResult::EmptyInput;
        }

        self.set_gcode(gcode);
        self.show_laser.set_active(true);

        let commands = {
            let vis = self.visualizer.borrow();
            vis.commands().to_vec()
        };

        if commands.is_empty() {
            return RunPreviewResult::NoMotion;
        }

        let preview_points = Self::build_run_preview_points(&commands);
        if preview_points.is_empty() {
            return RunPreviewResult::NoTrajectory;
        }

        self.stop_run_preview();
        *self.run_preview_points.borrow_mut() = preview_points;
        *self.run_preview_index.borrow_mut() = 0;
        *self.run_preview_running.borrow_mut() = true;
        *self.run_preview_paused.borrow_mut() = false;

        let token = {
            let mut tk = self.run_preview_token.borrow_mut();
            *tk = tk.wrapping_add(1);
            *tk
        };

        let token_ref = self.run_preview_token.clone();
        let points_ref = self.run_preview_points.clone();
        let index_ref = self.run_preview_index.clone();
        let running_ref = self.run_preview_running.clone();
        let paused_ref = self.run_preview_paused.clone();
        let speed_ref = self.run_preview_speed.clone();
        let current_pos_ref = self.current_pos.clone();
        let show_laser_ref = self.show_laser.clone();
        let drawing_area_ref = self.drawing_area.clone();
        let gl_area_ref = self.gl_area.clone();

        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(33), move || {
            if *token_ref.borrow() != token {
                return gtk4::glib::ControlFlow::Break;
            }

            if !*running_ref.borrow() {
                return gtk4::glib::ControlFlow::Break;
            }

            if *paused_ref.borrow() {
                return gtk4::glib::ControlFlow::Continue;
            }

            let i = *index_ref.borrow();
            let points = points_ref.borrow();
            if i >= points.len() {
                *running_ref.borrow_mut() = false;
                return gtk4::glib::ControlFlow::Break;
            }

            let (x, y, z) = points[i];
            drop(points);
            *current_pos_ref.borrow_mut() = (x, y, z);
            if show_laser_ref.is_active() {
                drawing_area_ref.queue_draw();
                gl_area_ref.queue_render();
            }

            let step = (*speed_ref.borrow()).max(1);
            *index_ref.borrow_mut() = i.saturating_add(step);
            gtk4::glib::ControlFlow::Continue
        });

        RunPreviewResult::Started
    }

    pub fn pause_run_preview(&self) {
        if *self.run_preview_running.borrow() {
            *self.run_preview_paused.borrow_mut() = true;
        }
    }

    pub fn resume_run_preview(&self) {
        if *self.run_preview_running.borrow() {
            *self.run_preview_paused.borrow_mut() = false;
        }
    }

    pub fn stop_run_preview(&self) {
        *self.run_preview_running.borrow_mut() = false;
        *self.run_preview_paused.borrow_mut() = false;
        *self.run_preview_index.borrow_mut() = 0;
        let mut tk = self.run_preview_token.borrow_mut();
        *tk = tk.wrapping_add(1);
    }

    pub fn is_run_preview_running(&self) -> bool {
        *self.run_preview_running.borrow()
    }

    pub fn is_run_preview_paused(&self) -> bool {
        *self.run_preview_paused.borrow()
    }

    pub fn apply_fit_to_device(
        vis: &mut Visualizer,
        device_manager: &Option<Arc<DeviceManager>>,
        width: f32,
        height: f32,
    ) {
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        // Default device working area fallback from shared constants
        const DEFAULT_WORK_WIDTH: f32 = core_constants::DEFAULT_WORK_WIDTH_MM as f32;
        const DEFAULT_WORK_HEIGHT: f32 = core_constants::DEFAULT_WORK_HEIGHT_MM as f32;

        let (work_width, work_height, center_x, center_y) = if let Some(manager) = device_manager {
            if let Some(profile) = manager.get_active_profile() {
                let w = (profile.x_axis.max - profile.x_axis.min) as f32;
                let h = (profile.y_axis.max - profile.y_axis.min) as f32;
                (
                    w,
                    h,
                    (profile.x_axis.min as f32) + w / 2.0,
                    (profile.y_axis.min as f32) + h / 2.0,
                )
            } else {
                (
                    DEFAULT_WORK_WIDTH,
                    DEFAULT_WORK_HEIGHT,
                    DEFAULT_WORK_WIDTH / 2.0,
                    DEFAULT_WORK_HEIGHT / 2.0,
                )
            }
        } else {
            (
                DEFAULT_WORK_WIDTH,
                DEFAULT_WORK_HEIGHT,
                DEFAULT_WORK_WIDTH / 2.0,
                DEFAULT_WORK_HEIGHT / 2.0,
            )
        };

        if work_width > 0.0 && work_height > 0.0 {
            // Calculate zoom to fit device area with padding
            let padding_percent = core_constants::VIEW_PADDING as f32;
            let available_width = width * (1.0 - padding_percent * 2.0);
            let available_height = height * (1.0 - padding_percent * 2.0);

            let zoom_x = available_width / work_width;
            let zoom_y = available_height / work_height;
            let new_zoom = zoom_x.min(zoom_y).clamp(0.1, 50.0);

            vis.zoom_scale = new_zoom;

            // Center the view on the device center
            // The draw function applies: translate(screen_center) -> scale -> translate(offset)
            // So offset needs to be the negative center of the target to bring it to (0,0) before scaling/centering on screen
            vis.x_offset = -center_x;
            vis.y_offset = -center_y;
        }
    }

    fn apply_top_3d_view(&self, min: Vec3, max: Vec3) {
        let (target_x, target_y) = {
            let mut cam = self.camera.borrow_mut();
            cam.set_view(0.0, 90.0);
            cam.fit_to_bounds(min, max);
            (cam.target.x, cam.target.y)
        };

        self.hadjustment_3d.set_value(target_x as f64);
        self.vadjustment_3d.set_value(target_y as f64);
        self.gl_area.queue_render();
    }

    pub fn new(
        device_manager: Option<Arc<DeviceManager>>,
        settings_controller: Rc<SettingsController>,
        status_bar: Option<StatusBar>,
        designer_state: Option<Shared<DesignerState>>,
    ) -> Self {
        let container = Paned::new(Orientation::Horizontal);
        container.add_css_class("visualizer-container");
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Sidebar for controls (compact list + toolbar)
        let sidebar = Box::new(Orientation::Vertical, spacing::MEDIUM);
        sidebar.set_width_request(200);
        sidebar.add_css_class("visualizer-sidebar");
        sidebar.set_margin_start(spacing::PANEL);
        sidebar.set_margin_end(spacing::PANEL);
        sidebar.set_margin_top(spacing::PANEL);
        sidebar.set_margin_bottom(spacing::PANEL);

        // Top toolbar row
        let view_controls = Box::new(Orientation::Horizontal, 6);

        let fit_btn = Button::builder()
            .icon_name("zoom-fit-best-symbolic")
            .tooltip_text(t!("Fit to Content"))
            .build();
        fit_btn.update_property(&[AccessibleProperty::Label(&t!("Fit to Content"))]);

        let reset_btn = Button::builder()
            .icon_name("view-restore-symbolic")
            .tooltip_text(t!("Fit to Viewport"))
            .build();
        reset_btn.update_property(&[AccessibleProperty::Label(&t!("Fit to Viewport"))]);

        let fit_device_btn = Button::builder()
            .icon_name("preferences-desktop-display-symbolic")
            .tooltip_text(t!("Fit to Device Working Area"))
            .build();
        fit_device_btn
            .update_property(&[AccessibleProperty::Label(&t!("Fit to Device Working Area"))]);

        let sidebar_hide_btn = Button::builder().tooltip_text(t!("Hide Sidebar")).build();
        sidebar_hide_btn.update_property(&[AccessibleProperty::Label(&t!("Hide Sidebar"))]);
        {
            let child = Box::new(Orientation::Horizontal, 6);
            child.append(&Image::from_icon_name("view-conceal-symbolic"));
            child.append(&Label::new(Some(&t!("Hide"))));
            sidebar_hide_btn.set_child(Some(&child));
        }

        for b in [&fit_btn, &reset_btn, &fit_device_btn, &sidebar_hide_btn] {
            b.set_size_request(32, 32);
        }

        view_controls.append(&fit_btn);
        view_controls.append(&reset_btn);

        // Only show fit to device button if device manager is available
        if device_manager.is_some() {
            view_controls.append(&fit_device_btn);
        }
        view_controls.append(&sidebar_hide_btn);

        sidebar.append(&view_controls);

        let sidebar_list = ListBox::new();
        sidebar_list.set_selection_mode(SelectionMode::None);
        sidebar_list.add_css_class("visualizer-sidebar-list");

        let show_rapid = CheckButton::builder()
            .label(t!("Show Rapid Moves"))
            .active(true)
            .build();
        let show_cut = CheckButton::builder()
            .label(t!("Show Cutting Moves"))
            .active(true)
            .build();
        let show_grid = CheckButton::builder()
            .label(t!("Show Grid"))
            .active(true)
            .build();

        let grid_spacing_mm = Rc::new(std::cell::Cell::new(50.0_f64));
        let grid_spacing_row = Box::new(Orientation::Horizontal, 6);
        let grid_spacing_label = Label::new(Some(&t!("Grid spacing")));
        grid_spacing_label.add_css_class("caption");

        let grid_spacing_combo = ComboBoxText::new();
        grid_spacing_combo.set_tooltip_text(Some(&t!("Grid spacing")));

        let system = settings_controller
            .persistence
            .borrow()
            .config()
            .ui
            .measurement_system;
        let unit_label = gcodekit5_core::units::get_unit_label(system);
        let grid_options_mm = [1.0_f64, 5.0, 10.0, 25.0, 50.0];
        for mm in grid_options_mm {
            let label = format!(
                "{} {}",
                gcodekit5_core::units::format_length(mm as f32, system),
                unit_label
            );
            grid_spacing_combo.append(Some(&mm.to_string()), &label);
        }
        grid_spacing_combo.set_active_id(Some("50"));

        {
            let grid_spacing_mm = grid_spacing_mm.clone();
            grid_spacing_combo.connect_changed(move |cb| {
                let Some(id) = cb.active_id() else {
                    return;
                };
                if let Ok(mm) = id.parse::<f64>() {
                    grid_spacing_mm.set(mm);
                }
            });
        }

        grid_spacing_row.append(&grid_spacing_label);
        grid_spacing_row.append(&grid_spacing_combo);

        let show_bounds = CheckButton::builder()
            .label(t!("Show Machine Bounds"))
            .active(true)
            .build();

        let show_laser = CheckButton::builder()
            .label(t!("Show Laser/Spindle"))
            .active(true)
            .build();

        let enable_stock_removal_3d = settings_controller
            .persistence
            .borrow()
            .config()
            .ui
            .enable_stock_removal_3d;

        let show_stock_removal = CheckButton::builder()
            .label(t!("Show Stock Removal"))
            .active(false)
            .tooltip_text(format!(
                "{}{:.2}{}{:.2}{}",
                t!("The simulation has a resolution of "),
                STOCK_REMOVAL_RESOLUTION,
                t!("mm. Cuts smaller than "),
                STOCK_REMOVAL_RESOLUTION + 0.01,
                t!("mm will not be visually displayed, but the G-code will execute correctly.")
            ))

        .build();
        show_stock_removal.set_visible(enable_stock_removal_3d);

        // Stock configuration
        let stock_width_entry = gtk4::Entry::builder()
            .placeholder_text(t!("Width"))
            .editable(false)
            .build();
        let stock_height_entry = gtk4::Entry::builder()
            .placeholder_text(t!("Height"))
            .editable(false)
            .build();
        let stock_thickness_entry = gtk4::Entry::builder()
            .placeholder_text(t!("Thickness"))
            .editable(false)
            .build();
        let stock_tool_diameter_entry = gtk4::Entry::builder()
            .placeholder_text(t!("Tool Diameter"))
            .editable(false)
            .build();

        // Group toggles into sections
        let toolpath_box = Box::new(Orientation::Vertical, 6);
        toolpath_box.set_margin_start(6);
        toolpath_box.set_margin_end(6);
        toolpath_box.set_margin_top(6);
        toolpath_box.set_margin_bottom(6);
        toolpath_box.append(&show_rapid);
        toolpath_box.append(&show_cut);
        toolpath_box.append(&show_laser);

        let toolpath_expander = Expander::builder()
            .label(t!("Toolpath"))
            .expanded(true)
            .child(&toolpath_box)
            .build();
        {
            let row = ListBoxRow::new();
            row.set_child(Some(&toolpath_expander));
            sidebar_list.append(&row);
        }

        let guides_box = Box::new(Orientation::Vertical, 4);
        guides_box.set_margin_start(6);
        guides_box.set_margin_end(6);
        guides_box.set_margin_top(6);
        guides_box.set_margin_bottom(6);
        guides_box.append(&show_grid);
        guides_box.append(&grid_spacing_row);
        guides_box.append(&show_bounds);

        let guides_expander = Expander::builder()
            .label(t!("Guides"))
            .expanded(true)
            .child(&guides_box)
            .build();
        {
            let row = ListBoxRow::new();
            row.set_child(Some(&guides_expander));
            sidebar_list.append(&row);
        }

        let stock_box = Box::new(Orientation::Vertical, 4);
        {
            let stock_label = Label::new(Some(&t!("Stock")));
            stock_label.add_css_class("caption");
            stock_label.set_halign(gtk4::Align::Start);
            stock_box.append(&stock_label);
        }
        stock_box.append(&stock_width_entry);
        stock_box.append(&stock_height_entry);
        stock_box.append(&stock_thickness_entry);
        stock_box.append(&stock_tool_diameter_entry);

        let stock_revealer = Revealer::new();
        stock_revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
        stock_revealer.set_child(Some(&stock_box));
        stock_revealer.set_reveal_child(show_stock_removal.is_active());
        stock_revealer.set_visible(enable_stock_removal_3d);

        {
            let stock_revealer = stock_revealer.clone();
            show_stock_removal.connect_toggled(move |b| {
                stock_revealer.set_reveal_child(b.is_active());
            });
        }

        // Gate stock removal to experimental-only.
        if !enable_stock_removal_3d {
            show_stock_removal.set_active(false);
            stock_revealer.set_reveal_child(false);
        }

        {
            let show_stock_removal = show_stock_removal.clone();
            let stock_revealer = stock_revealer.clone();
            settings_controller.on_setting_changed(move |key, value| {
                if key != "enable_stock_removal_3d" {
                    return;
                }
                let enabled = value == "true";
                show_stock_removal.set_visible(enabled);
                stock_revealer.set_visible(enabled);
                if !enabled {
                    show_stock_removal.set_active(false);
                    stock_revealer.set_reveal_child(false);
                }
            });
        }

        let simulation_box = Box::new(Orientation::Vertical, 4);
        simulation_box.set_margin_start(6);
        simulation_box.set_margin_end(6);
        simulation_box.set_margin_top(6);
        simulation_box.set_margin_bottom(6);
        simulation_box.append(&show_stock_removal);
        simulation_box.append(&stock_revealer);

        let simulation_expander = Expander::builder()
            .label(t!("Simulation"))
            .expanded(false)
            .child(&simulation_box)
            .build();
        {
            let row = ListBoxRow::new();
            row.set_child(Some(&simulation_expander));
            sidebar_list.append(&row);
        }

        // Inspector
        let bounds_x_value = Label::builder()
            .label("0.0")
            .halign(gtk4::Align::End)
            .css_classes(vec!["monospace"])
            .build();
        let bounds_y_value = Label::builder()
            .label("0.0")
            .halign(gtk4::Align::End)
            .css_classes(vec!["monospace"])
            .build();

        let min_s_value = Label::builder()
            .label(t!("N/A"))
            .halign(gtk4::Align::End)
            .css_classes(vec!["monospace"])
            .build();
        let max_s_value = Label::builder()
            .label(t!("N/A"))
            .halign(gtk4::Align::End)
            .css_classes(vec!["monospace"])
            .build();
        let avg_s_value = Label::builder()
            .label(t!("N/A"))
            .halign(gtk4::Align::End)
            .css_classes(vec!["monospace"])
            .build();

        let inspector_list = ListBox::new();
        inspector_list.set_selection_mode(gtk4::SelectionMode::None);
        inspector_list.add_css_class("boxed-list");

        let make_row = |key: String, value: &Label| {
            let row_box = Box::new(Orientation::Horizontal, 12);
            row_box.set_margin_start(10);
            row_box.set_margin_end(10);
            row_box.set_margin_top(6);
            row_box.set_margin_bottom(6);

            let key_label = Label::builder()
                .label(&key)
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .css_classes(vec!["caption"])
                .build();

            row_box.append(&key_label);
            row_box.append(value);

            let row = ListBoxRow::new();
            row.set_activatable(false);
            row.set_selectable(false);
            row.set_child(Some(&row_box));
            row
        };

        inspector_list.append(&make_row(format!("{} X", t!("Bounds")), &bounds_x_value));
        inspector_list.append(&make_row(format!("{} Y", t!("Bounds")), &bounds_y_value));
        inspector_list.append(&make_row(t!("Min S:").to_string(), &min_s_value));
        inspector_list.append(&make_row(t!("Max S:").to_string(), &max_s_value));
        inspector_list.append(&make_row(t!("Avg S:").to_string(), &avg_s_value));

        let inspector_box = Box::new(Orientation::Vertical, 6);
        inspector_box.set_margin_start(6);
        inspector_box.set_margin_end(6);
        inspector_box.set_margin_top(6);
        inspector_box.set_margin_bottom(6);
        inspector_box.append(&inspector_list);

        let inspector_expander = Expander::builder()
            .label(t!("Inspector"))
            .expanded(false)
            .child(&inspector_box)
            .build();
        {
            let row = ListBoxRow::new();
            row.set_child(Some(&inspector_expander));
            sidebar_list.append(&row);
        }

        // Scroll the list content (keep toolbar pinned)
        let list_scroller = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&sidebar_list)
            .build();
        list_scroller.set_vexpand(true);
        sidebar.append(&list_scroller);

        // Widget that gets inserted/hidden in the Paned
        let scrolled_sidebar = sidebar.clone();

        let sidebar_visible_init = settings_controller
            .persistence
            .borrow()
            .config()
            .ui
            .panel_visibility
            .get("visualizer_sidebar")
            .copied()
            .unwrap_or(true);

        if sidebar_visible_init {
            container.set_start_child(Some(&scrolled_sidebar));
        } else {
            container.set_start_child(None::<&gtk4::Widget>);
        }

        // Drawing Area
        let drawing_area = DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .css_classes(vec!["visualizer-canvas"])
            .build();

        // Scrollbars
        let hadjustment = Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 10.0);
        let vadjustment = Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 10.0);

        let hscrollbar = Scrollbar::builder()
            .orientation(Orientation::Horizontal)
            .adjustment(&hadjustment)
            .build();

        let vscrollbar = Scrollbar::builder()
            .orientation(Orientation::Vertical)
            .adjustment(&vadjustment)
            .build();

        // Default hidden (toggleable) to maximize canvas space
        hscrollbar.set_visible(false);
        vscrollbar.set_visible(false);

        // Stack 3D
        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);

        // 3D Page - CONFIGURACIÓN CORRECTA
        let gl_area = GLArea::builder()
            .hexpand(true)
            .vexpand(true)
            .has_depth_buffer(true)      // Habilitar depth buffer
            .has_stencil_buffer(false)
            .auto_render(true)           // Renderizar automáticamente
            .build();
        gl_area.set_required_version(3, 3);

        // CONFIGURAR EL CONTEXTO ANTES DE USARLO
        gl_area.set_has_depth_buffer(true);
        gl_area.set_auto_render(true);

        // CONECTAR REALIZE - NECESARIO PARA INICIALIZAR
        gl_area.connect_realize(|area| {
            // El contexto GL se activa automáticamente en realize
            // Hacer que el área sea focuseable para recibir eventos
            area.set_can_focus(true);
            area.grab_focus();
        });

        // CONECTAR UNMAP - Limpiar recursos cuando se oculta
        gl_area.connect_unmap(|_area| {
            // El contexto se libera automáticamente
        });

        // 3D Scrollbars
        let extent = core_constants::WORLD_EXTENT_MM;
        let hadjustment_3d = Adjustment::new(0.0, -extent, extent, 10.0, 100.0, 100.0);
        let vadjustment_3d = Adjustment::new(0.0, -extent, extent, 10.0, 100.0, 100.0);

        let hscrollbar_3d = Scrollbar::builder()
            .orientation(Orientation::Horizontal)
            .adjustment(&hadjustment_3d)
            .build();

        let vscrollbar_3d = Scrollbar::builder()
            .orientation(Orientation::Vertical)
            .adjustment(&vadjustment_3d)
            .build();

        // Default hidden (toggleable) to maximize canvas space
        hscrollbar_3d.set_visible(true);
        vscrollbar_3d.set_visible(true);

        let grid_3d = Grid::builder().hexpand(true).vexpand(true).build();

        grid_3d.attach(&gl_area, 0, 0, 1, 1);
        grid_3d.attach(&vscrollbar_3d, 1, 0, 1, 1);
        grid_3d.attach(&hscrollbar_3d, 0, 1, 1, 1);

        stack.add_titled(&grid_3d, Some("3d"), &t!("3D View"));

        // Initialize Visualizer logic
        let visualizer = shared(Visualizer::new());
        let current_pos = shared((0.0f32, 0.0f32, 0.0f32));
        let run_preview_points = shared(Vec::<(f32, f32, f32)>::new());
        let run_preview_index = shared(0usize);
        let run_preview_token = shared(0u64);
        let run_preview_running = shared(false);
        let run_preview_paused = shared(false);
        let run_preview_speed = shared(1usize);
        let camera = shared(Camera3D::default());
        let renderer_state = shared_none();
        let is_updating_3d = shared(false);

        // Stock removal simulation - use default sensible values
        let initial_stock = Some(StockMaterial {
            width: 200.0,
            height: 200.0,
            thickness: 10.0,
            origin: (0.0, 0.0, 0.0),
            safe_z: 10.0,
        });
        let stock_material = shared(initial_stock);
        let tool_diameter = shared(3.175f32); // Default 1/8" end mill
        let simulation_visualization = shared_none::<StockRemovalVisualization>();
        let simulation_running = shared(false);
        let stock_simulator_3d = shared_none();
        let stock_simulation_3d_pending = shared(false);

        // Overlay for floating controls
        let overlay = Overlay::new();
        overlay.set_child(Some(&stack));

        // Nav Cube (Top Right)
        let nav_cube = NavCube::new(camera.clone(), gl_area.clone());
        overlay.add_overlay(&nav_cube.widget);

        // Empty state (shown when no G-code is loaded)
        let empty_box = Box::new(Orientation::Vertical, 8);
        empty_box.add_css_class("visualizer-osd");
        empty_box.set_halign(gtk4::Align::Center);
        empty_box.set_valign(gtk4::Align::Center);
        empty_box.set_margin_start(20);
        empty_box.set_margin_end(20);
        empty_box.set_margin_top(20);
        empty_box.set_margin_bottom(20);
        empty_box.append(&Label::new(Some(&t!("No G-code loaded"))));
        empty_box.append(&Label::new(Some(&t!("Open a file to preview toolpaths."))));
        empty_box.set_visible(true);
        overlay.add_overlay(&empty_box);

        // Floating Controls
        let floating_box = Box::new(Orientation::Vertical, 4);
        floating_box.add_css_class("visualizer-osd");
        floating_box.set_halign(gtk4::Align::End);
        floating_box.set_valign(gtk4::Align::Start);
        floating_box.set_margin_top(150);
        floating_box.set_margin_end(10);

        // Scroll bars
        let scrollbars_btn = Button::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text(t!("Toggle Scrollbars"))
            .build();
        scrollbars_btn.update_property(&[AccessibleProperty::Label(&t!("Toggle Scrollbars"))]);

        // Top View
        let top_view_btn = Button::builder()
            .icon_name("view-top-symbolic")
            .tooltip_text(t!("Top View"))
            .build();
        scrollbars_btn.update_property(&[AccessibleProperty::Label(&t!("Top View"))]);

        // Fit to device area button
        let nav_fit_device_btn = Button::builder()
            .icon_name("preferences-desktop-display-symbolic")
            .tooltip_text(t!("Fit to Device Working Area"))
            .build();
        nav_fit_device_btn.update_property(&[AccessibleProperty::Label(&t!("Fit to Device Working Area"))]);

        let help_btn = Button::builder()
            .label("?")
            .tooltip_text(t!("Mouse Controls"))
            .build();
        help_btn.update_property(&[AccessibleProperty::Label(&t!("Mouse Controls"))]);

        let help_popover = Popover::new();
        help_popover.set_parent(&help_btn);
        help_popover.set_has_arrow(true);
        {
            let help_box = Box::new(Orientation::Vertical, 6);
            help_box.set_margin_start(12);
            help_box.set_margin_end(12);
            help_box.set_margin_top(12);
            help_box.set_margin_bottom(12);
            help_box.append(&Label::new(Some(&t!("* Use mouse to orbit, pan and zoom *"))));
            help_box.append(&Label::new(Some(&t!("Shift+Left button+Drag: Pan"))));
            help_box.append(&Label::new(Some(&t!("Mouse wheel: Zoom+/-"))));
            help_box.append(&Label::new(Some(&t!("Left Button+Drag: Orbit"))));
            help_popover.set_child(Some(&help_box));
        }
        {
            let pop = help_popover.clone();
            help_btn.connect_clicked(move |_| pop.popup());
        }

        for b in [&scrollbars_btn, &help_btn] {
            b.set_size_request(32, 32);
        }

        floating_box.append(&scrollbars_btn);
        floating_box.append(&top_view_btn);
        floating_box.append(&nav_fit_device_btn);
        floating_box.append(&help_btn);

        // Status Panel (Bottom Left)
        let status_box = Box::new(Orientation::Horizontal, 4);
        status_box.add_css_class("visualizer-osd");
        status_box.set_halign(gtk4::Align::Start);
        status_box.set_valign(gtk4::Align::End);
        status_box.set_margin_bottom(-10);
        status_box.set_margin_start(20);

        let status_label = Label::builder().label(" ").build();
        status_label.set_hexpand(true);

        let units_badge = Label::new(Some(""));
        units_badge.add_css_class("osd-units-badge");

        status_box.append(&status_label);
        status_box.append(&units_badge);

        // Run preview controls (Bottom Left)
        let run_controls_box = Box::new(Orientation::Horizontal, 6);
        run_controls_box.add_css_class("visualizer-osd");
        run_controls_box.set_halign(gtk4::Align::Start);
        run_controls_box.set_valign(gtk4::Align::End);
        run_controls_box.set_margin_start(20);
        run_controls_box.set_margin_bottom(24);

        let run_speed_label = Label::new(Some(&t!("Speed")));
        let run_speed_combo = ComboBoxText::new();
        run_speed_combo.append(Some("slow"), &t!("Slow"));
        run_speed_combo.append(Some("normal"), &t!("Normal"));
        run_speed_combo.append(Some("fast"), &t!("Fast"));
        run_speed_combo.append(Some("turbo"), &t!("Turbo"));
        run_speed_combo.set_active_id(Some("normal"));
        run_speed_combo.set_tooltip_text(Some(&t!("Preview playback speed")));

        let run_play_btn = Button::builder()
            .icon_name("media-playback-start-symbolic")
            .tooltip_text(t!("Play preview"))
            .build();
        run_play_btn.update_property(&[AccessibleProperty::Label(&t!("Play preview"))]);

        let run_pause_btn = Button::builder()
            .icon_name("media-playback-pause-symbolic")
            .tooltip_text(t!("Pause preview"))
            .build();
        run_pause_btn.update_property(&[AccessibleProperty::Label(&t!("Pause preview"))]);

        let run_stop_btn = Button::builder()
            .icon_name("media-playback-stop-symbolic")
            .tooltip_text(t!("Stop preview"))
            .build();
        run_stop_btn.update_property(&[AccessibleProperty::Label(&t!("Stop preview"))]);

        for b in [&run_play_btn, &run_pause_btn, &run_stop_btn] {
            b.set_size_request(32, 32);
            run_controls_box.append(b);
        }
        run_controls_box.append(&run_speed_label);
        run_controls_box.append(&run_speed_combo);

        // Sidebar show panel (floating) — matches Device Console UX
        let sidebar_show_btn = Button::builder().tooltip_text(t!("Show Sidebar")).build();
        sidebar_show_btn.update_property(&[AccessibleProperty::Label(&t!("Show Sidebar"))]);
        {
            let child = Box::new(Orientation::Horizontal, 6);
            child.append(&Image::from_icon_name("view-reveal-symbolic"));
            child.append(&Label::new(Some(&t!("Show Sidebar"))));
            sidebar_show_btn.set_child(Some(&child));
        }

        let sidebar_show_panel = Box::new(Orientation::Horizontal, 0);
        sidebar_show_panel.add_css_class("visualizer-osd");
        sidebar_show_panel.set_halign(gtk4::Align::Start);
        sidebar_show_panel.set_valign(gtk4::Align::Start);
        sidebar_show_panel.set_margin_start(12);
        sidebar_show_panel.set_margin_top(12);
        sidebar_show_panel.append(&sidebar_show_btn);
        sidebar_show_panel.set_visible(!sidebar_visible_init);

        // Stock removal progress (non-blocking) + cancel
        let sim_cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let sim_progress = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let sim_spinner = Spinner::new();
        sim_spinner.start();

        let sim_progress_label = Label::new(Some(""));

        let sim_cancel_btn = Button::builder().tooltip_text(t!("Cancel")).build();
        sim_cancel_btn.update_property(&[AccessibleProperty::Label(&t!("Cancel"))]);
        {
            let child = Box::new(Orientation::Horizontal, 6);
            child.append(&Image::from_icon_name("process-stop-symbolic"));
            child.append(&Label::new(Some(&t!("Cancel"))));
            sim_cancel_btn.set_child(Some(&child));
        }

        let sim_panel = Box::new(Orientation::Horizontal, 8);
        sim_panel.add_css_class("visualizer-osd");
        sim_panel.set_halign(gtk4::Align::Center);
        sim_panel.set_valign(gtk4::Align::Start);
        sim_panel.set_margin_top(12);
        sim_panel.append(&Label::new(Some(&t!("Simulating stock removal…"))));
        sim_panel.append(&sim_progress_label);
        sim_panel.append(&sim_spinner);
        sim_panel.append(&sim_cancel_btn);
        sim_panel.set_visible(false);

        {
            let cancel_flag = sim_cancel.clone();
            let show_stock = show_stock_removal.clone();
            let panel = sim_panel.clone();
            let sb = status_bar.clone();
            sim_cancel_btn.connect_clicked(move |_| {
                cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                panel.set_visible(false);
                show_stock.set_active(false);
                if let Some(sb) = sb.as_ref() {
                    sb.set_progress(0.0, "", "");
                    sb.set_cancel_action(None);
                }
            });
        }

        overlay.add_overlay(&floating_box);
        overlay.add_overlay(&status_box);
        overlay.add_overlay(&run_controls_box);
        overlay.add_overlay(&sidebar_show_panel);
        overlay.add_overlay(&sim_panel);

        container.set_end_child(Some(&overlay));

        // Connect NavCube fit device button
        if device_manager.is_some() {
            let cam_fit_nav = camera.clone();
            let gl_area_fit_nav = gl_area.clone();
            let device_manager_fit_nav = device_manager.clone();
            nav_fit_device_btn.connect_clicked(move |_| {
                if let Some(manager) = device_manager_fit_nav.as_ref() {
                    if let Some(profile) = manager.get_active_profile() {
                        let min_x = profile.x_axis.min as f32;
                        let max_x = profile.x_axis.max as f32;
                        let min_y = profile.y_axis.min as f32;
                        let max_y = profile.y_axis.max as f32;
                        let min_z = profile.z_axis.min as f32;
                        let max_z = profile.z_axis.max as f32;

                        let mut cam = cam_fit_nav.borrow_mut();
                        cam.fit_to_bounds(
                            Vec3::new(min_x, min_y, min_z),
                            Vec3::new(max_x, max_y, max_z),
                        );

                        gl_area_fit_nav.queue_render();
                    }
                }
            });
        }

        // Connect NavCube Fit Button
        let fit_btn_3d = nav_cube.fit_btn.clone();
        let vis_fit_3d = visualizer.clone();
        let cam_fit_3d = camera.clone();
        let gl_area_fit_3d = gl_area.clone();
        let hadj_fit_3d = hadjustment_3d.clone();
        let vadj_fit_3d = vadjustment_3d.clone();
        let is_updating_fit_3d = is_updating_3d.clone();

        fit_btn_3d.connect_clicked(move |_| {
            let vis = vis_fit_3d.borrow();
            let (min_x, max_x, min_y, max_y, min_z, max_z) =
                if let Some(bounds) = vis.get_cutting_bounds() {
                    bounds
                } else {
                    let (min_x, max_x, min_y, max_y) = vis.get_bounds();
                    (min_x, max_x, min_y, max_y, vis.min_z, vis.max_z)
                };
            drop(vis);

            // Comprobar contenido
            let has_content = (max_x - min_x).abs() > 0.001 || (max_y - min_y).abs() > 0.001;

            if has_content {
                let mut cam = cam_fit_3d.borrow_mut();
                cam.fit_to_bounds(
                    Vec3::new(min_x, min_y, min_z),
                    Vec3::new(max_x, max_y, max_z),
                );

                *is_updating_fit_3d.borrow_mut() = true;
                hadj_fit_3d.set_value(cam.target.x as f64);
                vadj_fit_3d.set_value(cam.target.y as f64);
                *is_updating_fit_3d.borrow_mut() = false;

                gl_area_fit_3d.queue_render();
            }
        });

        // Visibility Logic
        let nav_widget = nav_cube.widget.clone();
        let float_box = floating_box.clone();

        nav_widget.set_visible(true);
        float_box.set_visible(true);

        stack.set_visible_child_name("3d");

        // Helper to update status
        let cursor_pos = shared((0.0_f32, 0.0_f32));
        let update_status_fn: Rc<dyn Fn()> = Rc::new({
            let label = status_label.clone();
            let units_badge = units_badge.clone();
            let empty_box = empty_box.clone();
            let vis = visualizer.clone();
            let cursor_pos = cursor_pos.clone();
            let settings = settings_controller.clone();
            move || {
                let v = vis.borrow();
                let (cursor_x, cursor_y) = *cursor_pos.borrow();
                let system = settings.persistence.borrow().config().ui.measurement_system;

                // Visualizer offsets are negative of center, so we negate them to show center
                let center_x = -v.x_offset;
                let center_y = -v.y_offset;

                label.set_text(&format_zoom_center_cursor(
                    v.zoom_scale as f64,
                    center_x,
                    center_y,
                    cursor_x,
                    cursor_y,
                    system,
                ));

                units_badge.set_text(gcodekit5_core::units::get_unit_label(system));
                empty_box.set_visible(v.commands().is_empty());
            }
        });

        // Track cursor position in world coordinates
        let motion = EventControllerMotion::new();
        let vis_motion = visualizer.clone();
        let da_motion = drawing_area.clone();
        let cursor_pos_motion = cursor_pos.clone();
        motion.connect_motion(move |_, x, y| {
            let v = vis_motion.borrow();
            let width = da_motion.width() as f64;
            let height = da_motion.height() as f64;
            if width <= 0.0 || height <= 0.0 {
                return;
            }

            let center_x = width / 2.0;
            let center_y = height / 2.0;
            let s = v.zoom_scale as f64;
            if s == 0.0 {
                return;
            }

            let world_x = (x - center_x) / s - v.x_offset as f64;
            let world_y = -((y - center_y) / s) - v.y_offset as f64;
            *cursor_pos_motion.borrow_mut() = (world_x as f32, world_y as f32);
        });
        drawing_area.add_controller(motion);

        // connect_cliked for top_view_btn
        let cam_top = camera.clone();
        let gl_top = gl_area.clone();
        top_view_btn.connect_clicked(move |_| {
            let mut cam = cam_top.borrow_mut();
            cam.set_view(0.0, 90.0);
            gl_top.queue_render();
        });

        // Right-click context menu (matches Designer structure)
        {
            let right_click = GestureClick::new();
            right_click.set_button(3);
            let da_menu = drawing_area.clone();
            let vis_menu = visualizer.clone();
            let cursor_pos_menu = cursor_pos.clone();
            let settings_menu = settings_controller.clone();
            let update_menu = update_status_fn.clone();
            let device_mgr_menu = device_manager.clone();
            let show_grid_menu = show_grid.clone();
            let show_bounds_menu = show_bounds.clone();
            let show_rapid_menu = show_rapid.clone();
            let show_cut_menu = show_cut.clone();
            right_click.connect_pressed(move |_g, _n, x, y| {
                let menu = Popover::new();
                menu.set_parent(&da_menu);
                menu.set_has_arrow(false);
                let rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                menu.set_pointing_to(Some(&rect));

                let vbox = Box::new(Orientation::Vertical, 0);
                vbox.add_css_class("context-menu");

                let add_item = |label: &str, cb: std::boxed::Box<dyn Fn()>| {
                    let btn = Button::builder()
                        .label(label)
                        .has_frame(false)
                        .halign(gtk4::Align::Start)
                        .build();
                    let menu = menu.clone();
                    btn.connect_clicked(move |_| {
                        menu.popdown();
                        cb();
                    });
                    vbox.append(&btn);
                };

                // View
                {
                    let vis = vis_menu.clone();
                    let da = da_menu.clone();
                    let update = update_menu.clone();
                    add_item(
                        "Fit to Content",
                        std::boxed::Box::new(move || {
                            let width = da.width() as f32;
                            let height = da.height() as f32;
                            if width > 0.0 && height > 0.0 {
                                vis.borrow_mut().fit_to_view(width, height);
                                update();
                                da.queue_draw();
                            }
                        }),
                    );
                }
                {
                    let vis = vis_menu.clone();
                    let da = da_menu.clone();
                    let update = update_menu.clone();
                    add_item(
                        "Fit to Viewport",
                        std::boxed::Box::new(move || {
                            let mut v = vis.borrow_mut();
                            v.reset_zoom();
                            v.reset_pan();
                            drop(v);
                            update();
                            da.queue_draw();
                        }),
                    );
                }
                {
                    let vis = vis_menu.clone();
                    let da = da_menu.clone();
                    let update = update_menu.clone();
                    let dm = device_mgr_menu.clone();
                    add_item(
                        "Fit to Device Working Area",
                        std::boxed::Box::new(move || {
                            let width = da.width() as f32;
                            let height = da.height() as f32;
                            if width > 0.0 && height > 0.0 {
                                let mut v = vis.borrow_mut();
                                Self::apply_fit_to_device(&mut v, &dm, width, height);
                                drop(v);
                                update();
                                da.queue_draw();
                            }
                        }),
                    );
                }

                vbox.append(&Separator::new(Orientation::Horizontal));

                // Copy
                {
                    let cursor_pos = cursor_pos_menu.clone();
                    let settings = settings_menu.clone();
                    add_item(
                        "Copy cursor coordinates",
                        std::boxed::Box::new(move || {
                            let (x, y) = *cursor_pos.borrow();
                            let system =
                                settings.persistence.borrow().config().ui.measurement_system;
                            let text = format!(
                                "X {}  Y {}",
                                gcodekit5_core::units::format_length(x, system),
                                gcodekit5_core::units::format_length(y, system)
                            );
                            if let Some(display) = gtk4::gdk::Display::default() {
                                display.clipboard().set_text(&text);
                            }
                        }),
                    );
                }

                vbox.append(&Separator::new(Orientation::Horizontal));

                // Toggles
                {
                    let btn = Button::builder()
                        .label("Toggle Grid")
                        .has_frame(false)
                        .halign(gtk4::Align::Start)
                        .build();
                    let menu = menu.clone();
                    let cb = show_grid_menu.clone();
                    btn.connect_clicked(move |_| {
                        menu.popdown();
                        cb.set_active(!cb.is_active());
                    });
                    vbox.append(&btn);
                }
                {
                    let btn = Button::builder()
                        .label("Toggle Machine Bounds")
                        .has_frame(false)
                        .halign(gtk4::Align::Start)
                        .build();
                    let menu = menu.clone();
                    let cb = show_bounds_menu.clone();
                    btn.connect_clicked(move |_| {
                        menu.popdown();
                        cb.set_active(!cb.is_active());
                    });
                    vbox.append(&btn);
                }
                {
                    let btn = Button::builder()
                        .label("Toggle Rapid Moves")
                        .has_frame(false)
                        .halign(gtk4::Align::Start)
                        .build();
                    let menu = menu.clone();
                    let cb = show_rapid_menu.clone();
                    btn.connect_clicked(move |_| {
                        menu.popdown();
                        cb.set_active(!cb.is_active());
                    });
                    vbox.append(&btn);
                }
                {
                    let btn = Button::builder()
                        .label("Toggle Cutting Moves")
                        .has_frame(false)
                        .halign(gtk4::Align::Start)
                        .build();
                    let menu = menu.clone();
                    let cb = show_cut_menu.clone();
                    btn.connect_clicked(move |_| {
                        menu.popdown();
                        cb.set_active(!cb.is_active());
                    });
                    vbox.append(&btn);
                }

                menu.set_child(Some(&vbox));
                menu.popup();
            });
            drawing_area.add_controller(right_click);
        }

        // Keep status text fresh while moving the mouse
        {
            let u = update_status_fn.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                u();
                gtk4::glib::ControlFlow::Continue
            });
        }

        // Helper to update scrollbars
        let is_updating = shared(false);
        let update_scrollbars_fn = {
            let vis = visualizer.clone();
            let hadj = hadjustment.clone();
            let vadj = vadjustment.clone();
            let da = drawing_area.clone();
            let is_updating = is_updating.clone();
            move || {
                let v = vis.borrow();
                let width = da.width() as f64;
                let height = da.height() as f64;

                if width <= 0.0 || height <= 0.0 {
                    return;
                }

                let zoom = v.zoom_scale as f64;
                let page_size_x = width / zoom;
                let page_size_y = height / zoom;

                let center_x = -v.x_offset as f64;
                let center_y = -v.y_offset as f64;

                let val_x = center_x - page_size_x / 2.0;
                let val_y = center_y - page_size_y / 2.0;

                let (min_x, max_x, min_y, max_y) = v.get_bounds();
                let margin = 10.0;

                // Use World Extents for scrollbar range
                let extent = core_constants::WORLD_EXTENT_MM;

                // Ensure the range includes the current view and content
                let lower_x = (-extent).min(min_x as f64 - margin).min(val_x);
                let upper_x = (extent).max(max_x as f64 + margin).max(val_x + page_size_x);
                let lower_y = (-extent).min(min_y as f64 - margin).min(val_y);
                let upper_y = (extent).max(max_y as f64 + margin).max(val_y + page_size_y);

                drop(v);

                *is_updating.borrow_mut() = true;
                hadj.configure(
                    val_x,
                    lower_x,
                    upper_x,
                    page_size_x * 0.1,
                    page_size_x * 0.9,
                    page_size_x,
                );
                vadj.configure(
                    val_y,
                    lower_y,
                    upper_y,
                    page_size_y * 0.1,
                    page_size_y * 0.9,
                    page_size_y,
                );
                *is_updating.borrow_mut() = false;
            }
        };

        // Connect local Run preview controls (visualization only)
        let run_points_play = run_preview_points.clone();
        let run_index_play = run_preview_index.clone();
        let run_running_play = run_preview_running.clone();
        let run_paused_play = run_preview_paused.clone();
        let run_token_play = run_preview_token.clone();
        let run_speed_play = run_preview_speed.clone();
        let current_pos_play = current_pos.clone();
        let show_laser_play = show_laser.clone();
        let drawing_area_play = drawing_area.clone();
        let gl_area_play = gl_area.clone();
        let visualizer_play = visualizer.clone();

        run_play_btn.connect_clicked(move |_| {
            if *run_running_play.borrow() {
                *run_paused_play.borrow_mut() = false;
                return;
            }

            if run_points_play.borrow().is_empty() {
                let commands = {
                    let vis = visualizer_play.borrow();
                    vis.commands().to_vec()
                };

                if commands.is_empty() {
                    return;
                }

                let preview_points = Self::build_run_preview_points(&commands);
                if preview_points.is_empty() {
                    return;
                }

                *run_points_play.borrow_mut() = preview_points;
                *run_index_play.borrow_mut() = 0;
                show_laser_play.set_active(true);
            }

            *run_index_play.borrow_mut() = 0;
            *run_running_play.borrow_mut() = true;
            *run_paused_play.borrow_mut() = false;

            let token = {
                let mut tk = run_token_play.borrow_mut();
                *tk = tk.wrapping_add(1);
                *tk
            };

            let token_ref = run_token_play.clone();
            let points_ref = run_points_play.clone();
            let index_ref = run_index_play.clone();
            let running_ref = run_running_play.clone();
            let paused_ref = run_paused_play.clone();
            let speed_ref = run_speed_play.clone();
            let current_pos_ref = current_pos_play.clone();
            let show_laser_ref = show_laser_play.clone();
            let drawing_area_ref = drawing_area_play.clone();
            let gl_area_ref = gl_area_play.clone();

            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(33), move || {
                if *token_ref.borrow() != token {
                    return gtk4::glib::ControlFlow::Break;
                }

                if !*running_ref.borrow() {
                    return gtk4::glib::ControlFlow::Break;
                }

                if *paused_ref.borrow() {
                    return gtk4::glib::ControlFlow::Continue;
                }

                let i = *index_ref.borrow();
                let points = points_ref.borrow();
                if i >= points.len() {
                    *running_ref.borrow_mut() = false;
                    return gtk4::glib::ControlFlow::Break;
                }

                let (x, y, z) = points[i];
                drop(points);

                *current_pos_ref.borrow_mut() = (x, y, z);
                if show_laser_ref.is_active() {
                    drawing_area_ref.queue_draw();
                    gl_area_ref.queue_render();
                }

                let step = (*speed_ref.borrow()).max(1);
                *index_ref.borrow_mut() = i.saturating_add(step);
                gtk4::glib::ControlFlow::Continue
            });
        });

        let run_speed_state = run_preview_speed.clone();
        run_speed_combo.connect_changed(move |cb| {
            let speed = match cb.active_id().as_deref() {
                Some("slow") => 1,
                Some("normal") => 2,
                Some("fast") => 5,
                Some("turbo") => 20,
                _ => 2,
            };
            *run_speed_state.borrow_mut() = speed;
        });

        // Ensure default speed stays in Normal even before first user interaction.
        *run_preview_speed.borrow_mut() = 2;

        let run_paused_pause = run_preview_paused.clone();
        let run_running_pause = run_preview_running.clone();
        run_pause_btn.connect_clicked(move |_| {
            if *run_running_pause.borrow() {
                *run_paused_pause.borrow_mut() = true;
            }
        });

        let run_index_stop = run_preview_index.clone();
        let run_running_stop = run_preview_running.clone();
        let run_paused_stop = run_preview_paused.clone();
        let run_token_stop = run_preview_token.clone();
        run_stop_btn.connect_clicked(move |_| {
            *run_running_stop.borrow_mut() = false;
            *run_paused_stop.borrow_mut() = false;
            *run_index_stop.borrow_mut() = 0;
            let mut tk = run_token_stop.borrow_mut();
            *tk = tk.wrapping_add(1);
        });

        let update_ui = {
            let u1 = update_status_fn.clone();
            let u2 = update_scrollbars_fn.clone();
            move || {
                u1();
                u2();
            }
        };

        // Connect Scrollbars
        let vis_h = visualizer.clone();
        let da_h = drawing_area.clone();
        let is_updating_h = is_updating.clone();
        let update_status_h = update_status_fn.clone();
        hadjustment.connect_value_changed(move |adj| {
            if *is_updating_h.borrow() {
                return;
            }
            let val = adj.value();
            let page_size = adj.page_size();
            let center_x = val + page_size / 2.0;

            let mut v = vis_h.borrow_mut();
            v.x_offset = -center_x as f32;
            drop(v);

            update_status_h();
            da_h.queue_draw();
        });

        let vis_v = visualizer.clone();
        let da_v = drawing_area.clone();
        let is_updating_v = is_updating.clone();
        let update_status_v = update_status_fn.clone();
        vadjustment.connect_value_changed(move |adj| {
            if *is_updating_v.borrow() {
                return;
            }
            let val = adj.value();
            let page_size = adj.page_size();
            let center_y = val + page_size / 2.0;

            let mut v = vis_v.borrow_mut();
            v.y_offset = -center_y as f32;
            drop(v);

            update_status_v();
            da_v.queue_draw();
        });

        // 3D-only floating controls - legacy 2D handlers removed

        // Scrollbars toggle for 3D only
        let show_scrollbars = Rc::new(std::cell::Cell::new(false));
        let hsb_3d = hscrollbar_3d.clone();
        let vsb_3d = vscrollbar_3d.clone();
        let show_scrollbars_btn = show_scrollbars.clone();
        scrollbars_btn.connect_clicked(move |_| {
            let next = !show_scrollbars_btn.get();
            show_scrollbars_btn.set(next);
            hsb_3d.set_visible(next);
            vsb_3d.set_visible(next);
        });

        // Set initial sidebar width once (and then respect user changes)
        let did_set_paned = Rc::new(std::cell::Cell::new(false));
        let did_set_paned_map = did_set_paned.clone();
        let settings_map = settings_controller.clone();
        container.connect_map(move |paned| {
            if did_set_paned_map.get() {
                return;
            }
            did_set_paned_map.set(true);

            // If the sidebar starts hidden, don't restore a position.
            if !sidebar_visible_init {
                return;
            }

            let paned = paned.clone();
            let settings_map = settings_map.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                let stored = settings_map
                    .persistence
                    .borrow()
                    .config()
                    .ui
                    .visualizer_sidebar_position;

                let width = paned.width();
                if width <= 0 {
                    return gtk4::glib::ControlFlow::Continue;
                }

                let min_pos = 280;
                let max_25 = ((width as f64) * 0.25) as i32;
                let max_canvas = (width - 420).max(min_pos);
                let max_pos = max_25.min(max_canvas).clamp(min_pos, i32::MAX);

                let mut pos = stored.unwrap_or(max_25);
                if pos < min_pos {
                    pos = min_pos;
                }
                if pos > max_pos {
                    pos = max_pos;
                }

                paned.set_position(pos);
                gtk4::glib::ControlFlow::Break
            });
        });

        // Sidebar hide/show (same UX as Device Console)
        let sidebar_collapsed = Rc::new(std::cell::Cell::new(!sidebar_visible_init));
        let sidebar_last_pos = Rc::new(std::cell::Cell::new(0));

        {
            let paned = container.clone();
            let hide_btn = sidebar_hide_btn.clone();
            let collapsed = sidebar_collapsed.clone();
            let last_pos = sidebar_last_pos.clone();
            let show_panel = sidebar_show_panel.clone();
            let settings = settings_controller.clone();

            sidebar_hide_btn.connect_clicked(move |_| {
                if collapsed.get() {
                    return;
                }

                last_pos.set(paned.position());
                paned.set_start_child(None::<&gtk4::Widget>);
                hide_btn.set_sensitive(false);
                collapsed.set(true);
                show_panel.set_visible(true);

                // Persist collapsed state
                {
                    let mut p = settings.persistence.borrow_mut();
                    p.config_mut()
                        .ui
                        .panel_visibility
                        .insert("visualizer_sidebar".to_string(), false);
                    if let Ok(path) = SettingsManager::config_file_path() {
                        let _ = SettingsManager::ensure_config_dir();
                        let _ = p.save_to_file(&path);
                    }
                }
            });
        }

        {
            let paned = container.clone();
            let sidebar_scroller = scrolled_sidebar.clone();
            let hide_btn = sidebar_hide_btn.clone();
            let collapsed = sidebar_collapsed.clone();
            let last_pos = sidebar_last_pos.clone();
            let show_panel = sidebar_show_panel.clone();
            let settings = settings_controller.clone();

            sidebar_show_btn.connect_clicked(move |_| {
                if !collapsed.get() {
                    return;
                }

                paned.set_start_child(Some(&sidebar_scroller));

                let width = paned.width();
                if width > 0 {
                    let min_pos = 280;
                    let max_25 = ((width as f64) * 0.25) as i32;
                    let max_canvas = (width - 420).max(min_pos);
                    let max_pos = max_25.min(max_canvas).clamp(min_pos, i32::MAX);

                    let mut pos = last_pos.get();
                    if pos <= 0 {
                        pos = max_25;
                    }
                    if pos < min_pos {
                        pos = min_pos;
                    }
                    if pos > max_pos {
                        pos = max_pos;
                    }
                    paned.set_position(pos);

                    // Persist position and visible state
                    {
                        let mut p = settings.persistence.borrow_mut();
                        p.config_mut().ui.visualizer_sidebar_position = Some(pos);
                        p.config_mut()
                            .ui
                            .panel_visibility
                            .insert("visualizer_sidebar".to_string(), true);
                        if let Ok(path) = SettingsManager::config_file_path() {
                            let _ = SettingsManager::ensure_config_dir();
                            let _ = p.save_to_file(&path);
                        }
                    }
                }

                hide_btn.set_sensitive(true);
                collapsed.set(false);
                show_panel.set_visible(false);
            });
        }

        // Persist user choice (ignore bogus early values)
        let settings_persist = settings_controller.clone();
        container.connect_notify_local(Some("position"), move |paned, _| {
            // If sidebar is hidden, ignore position changes.
            if paned.start_child().is_none() {
                return;
            }

            let width = paned.width();
            if width <= 0 {
                return;
            }

            let min_pos = 280;
            let max_25 = ((width as f64) * 0.25) as i32;
            let max_canvas = (width - 420).max(min_pos);
            let max_pos = max_25.min(max_canvas).clamp(min_pos, i32::MAX);

            let mut pos = paned.position();
            if pos < min_pos {
                return;
            }
            if pos > max_pos {
                pos = max_pos;
            }

            {
                let mut p = settings_persist.persistence.borrow_mut();
                p.config_mut().ui.visualizer_sidebar_position = Some(pos);
                if let Ok(path) = SettingsManager::config_file_path() {
                    let _ = SettingsManager::ensure_config_dir();
                    let _ = p.save_to_file(&path);
                }
            }
        });

        // Fit to Device button
        if let Some(device_mgr) = device_manager.clone() {
            let vis_fit_dev = visualizer.clone();
            let da_fit_dev = drawing_area.clone();
            let update_status = update_ui.clone();
            let device_mgr_clone = device_mgr.clone();
            fit_device_btn.connect_clicked(move |_| {
                let width = da_fit_dev.width() as f32;
                let height = da_fit_dev.height() as f32;

                let mut vis = vis_fit_dev.borrow_mut();
                let mgr_opt = Some(device_mgr_clone.clone());
                Self::apply_fit_to_device(&mut vis, &mgr_opt, width, height);
                drop(vis);

                update_status();
                da_fit_dev.queue_draw();
            });
        }

        let vis_reset = visualizer.clone();
        let da_reset = drawing_area.clone();
        let update_status = update_ui.clone();
        reset_btn.connect_clicked(move |_| {
            {
                let mut vis = vis_reset.borrow_mut();
                vis.reset_zoom();
                vis.reset_pan();
            }
            update_status();
            da_reset.queue_draw();
        });

        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        let _da_update = drawing_area.clone();
        let _gl_update = gl_area.clone();
        show_rapid.connect_toggled(move |_| {
            da_update.queue_draw();
            gl_update.queue_render();
        });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        show_cut.connect_toggled(move |_| {
            da_update.queue_draw();
            gl_update.queue_render();
        });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        show_grid.connect_toggled(move |_| {
            da_update.queue_draw();
            gl_update.queue_render();
        });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        show_bounds.connect_toggled(move |_| {
            da_update.queue_draw();
            gl_update.queue_render();
        });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        show_laser.connect_toggled(move |_| {
            da_update.queue_draw();
            gl_update.queue_render();
        });
        let _da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        let visualizer_stock = visualizer.clone();
        let _simulation_visualization_stock = simulation_visualization.clone();
        let stock_material_stock = stock_material.clone();
        let tool_diameter_stock = tool_diameter.clone();
        let simulation_running_flag = simulation_running.clone();
        let stock_simulator_3d_stock = stock_simulator_3d.clone();
        let stock_simulation_3d_pending_toggle = stock_simulation_3d_pending.clone();
        let sim_panel_toggle = sim_panel.clone();
        let sim_cancel_flag = sim_cancel.clone();
        let sim_progress_flag = sim_progress.clone();
        let sim_progress_label_toggle = sim_progress_label.clone();
        let status_bar_sim = status_bar.clone();

// ============================================================
// STOCK REMOVAL TOGGLED - VERSIÓN CON CANAL PARA PROGRESO
// ============================================================
show_stock_removal.connect_toggled(move |checkbox| {
    if checkbox.is_active() {
        // Check if simulation is already running
        if *simulation_running_flag.borrow() {
            return;
        }

        sim_cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);
        sim_progress_flag.store(0, std::sync::atomic::Ordering::Relaxed);
        sim_progress_label_toggle.set_text("0%");
        sim_panel_toggle.set_visible(true);

        if let Some(sb) = status_bar_sim.as_ref() {
            let cancel_flag = sim_cancel_flag.clone();
            let show_stock = checkbox.clone();
            let panel = sim_panel_toggle.clone();
            sb.set_progress(0.1, "0s", "");
            sb.set_cancel_action(Some(std::boxed::Box::new(move || {
                cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                panel.set_visible(false);
                show_stock.set_active(false);
            })));
        }

        *simulation_running_flag.borrow_mut() = true;

        let started_at = std::time::Instant::now();

        // Run simulation when enabled
        let vis = visualizer_stock.borrow();

        if let Some(stock) = stock_material_stock.borrow().as_ref() {
            // Run simulation in background thread
            let stock_clone = stock.clone();
            let tool_radius_value = *tool_diameter_stock.borrow() / 2.0;
            let result_3d_ref = stock_simulator_3d_stock.clone();
            let gl_ref = gl_update.clone();

            // Convert GCode commands to toolpath segments for 3D
            use gcodekit5_visualizer::{ToolpathSegment, ToolpathSegmentType};
            let mut toolpath_segments_3d = Vec::new();

            for cmd in vis.commands() {
                match cmd {
                    GCodeCommand::Move {
                        from, to, rapid, ..
                    } => {
                        let seg_type = if *rapid {
                            ToolpathSegmentType::RapidMove
                        } else {
                            ToolpathSegmentType::LinearMove
                        };
                        let start_z = from.z;
                        let end_z = to.z;
                        toolpath_segments_3d.push(ToolpathSegment {
                            segment_type: seg_type,
                            start: (from.x, from.y, start_z),
                            end: (to.x, to.y, end_z),
                            center: None,
                            feed_rate: 100.0,
                            spindle_speed: 3000.0,
                        });
                    }
                    GCodeCommand::Arc {
                        from,
                        to,
                        center,
                        clockwise,
                        ..
                    } => {
                        let seg_type = if *clockwise {
                            ToolpathSegmentType::ArcCW
                        } else {
                            ToolpathSegmentType::ArcCCW
                        };
                        let start_z = from.z;
                        let end_z = to.z;
                        toolpath_segments_3d.push(ToolpathSegment {
                            segment_type: seg_type,
                            start: (from.x, from.y, start_z),
                            end: (to.x, to.y, end_z),
                            center: Some((center.x, center.y)),
                            feed_rate: 100.0,
                            spindle_speed: 3000.0,
                        });
                    }
                    GCodeCommand::Dwell { .. } => {
                        // Dwell commands don't remove material, skip
                    }
                }
            }

            // Use Arc<Mutex<>> for thread-safe sharing
            let result_arc = thread_safe_none();
            let result_arc_clone = result_arc.clone();

            let cancel_thread = sim_cancel_flag.clone();
            let progress_thread = sim_progress_flag.clone();

            // ============================================================
            // CREAR CANAL PARA EL PROGRESO
            // ============================================================
            let (progress_tx, progress_rx) = std::sync::mpsc::channel();

            // ✅ USAR Arc<AtomicBool> - SIN importar Borrow
            let render_ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let render_ready_clone = render_ready.clone();

            std::thread::spawn(move || {
                use gcodekit5_visualizer::{StockSimulator3D, VoxelGrid};

                let resolution = STOCK_REMOVAL_RESOLUTION; 
                let _grid = VoxelGrid::new(
                    stock_clone.width,
                    stock_clone.height,
                    stock_clone.thickness,
                    resolution,
                );

                let mut simulator = StockSimulator3D::new(
                    stock_clone.width,
                    stock_clone.height,
                    stock_clone.thickness,
                    resolution,
                    tool_radius_value,
                );

                let cancel = cancel_thread.clone();
                let progress = progress_thread.clone();
                let tx = progress_tx.clone();

                let _ = simulator.simulate_toolpath_with_progress(&toolpath_segments_3d, |p| {
                    // Enviar progreso por el canal
                    let _ = tx.send(p);

                    // Actualizar también la variable atómica para compatibilidad
                    if p > 0.0 {
                        progress.store(
                            (p * 100.0).round() as usize,
                            std::sync::atomic::Ordering::Relaxed,
                        );
                    }
                    !cancel.load(std::sync::atomic::Ordering::SeqCst)
                });

                progress.store(100, std::sync::atomic::Ordering::Relaxed);
                // Enviar 100% al final
                let _ = progress_tx.send(1.0);

                let result_sim = simulator;

                // Store in Arc
                *result_arc_clone.lock() = Some(result_sim);
                // ✅ Usar store en lugar de borrow_mut
                render_ready_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            });

            // ============================================================
            // IMPROVED POLLER WITH CHANNEL
            // ============================================================
            let result_arc_poll = result_arc.clone();
            let sim_running_poll = simulation_running_flag.clone();
            let pending_flag = stock_simulation_3d_pending_toggle.clone();
            let sim_cancel_flag_poll = sim_cancel_flag.clone();
            let sim_panel_toggle_poll = sim_panel_toggle.clone();
            let sim_progress_label_poll = sim_progress_label_toggle.clone();
            let sb_poll = status_bar_sim.clone();
            let render_ready_poll = render_ready.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                // 1. Actualizar progreso desde el canal
                while let Ok(p) = progress_rx.try_recv() {
                    let pct = (p * 100.0).round() as usize;
                    sim_progress_label_poll.set_text(&format!("{}%", pct));
                    if let Some(sb) = sb_poll.as_ref() {
                        let elapsed = started_at.elapsed().as_secs_f64();
                        sb.set_progress((pct as f64).max(0.1), &format!("{:.0}s", elapsed), "");
                    }
                }

                // 2. Verificar cancelación
                if sim_cancel_flag_poll.load(std::sync::atomic::Ordering::SeqCst) {
                    *sim_running_poll.borrow_mut() = false;
                    sim_panel_toggle_poll.set_visible(false);
                    if let Some(sb) = sb_poll.as_ref() {
                        sb.set_progress(0.0, "", "");
                        sb.set_cancel_action(None);
                    }
                    return glib::ControlFlow::Break;
                }

                // 3. Verificar si el renderizado está listo
                // ✅ Usar load en lugar de borrow
                if render_ready_poll.load(std::sync::atomic::Ordering::SeqCst) {
                    if let Some(mut guard) = result_arc_poll.try_lock() {
                        if let Some(result_simulator) = guard.take() {
                            if sim_cancel_flag_poll.load(std::sync::atomic::Ordering::SeqCst) {
                                *sim_running_poll.borrow_mut() = false;
                                sim_panel_toggle_poll.set_visible(false);
                                if let Some(sb) = sb_poll.as_ref() {
                                    sb.set_progress(0.0, "", "");
                                    sb.set_cancel_action(None);
                                }
                                return glib::ControlFlow::Break;
                            }

                            // Asignar el simulador
                            *result_3d_ref.borrow_mut() = Some(result_simulator);
                            *pending_flag.borrow_mut() = true;
                            *sim_running_poll.borrow_mut() = false;

                            // Ocultar panel de progreso
                            sim_panel_toggle_poll.set_visible(false);
                            if let Some(sb) = sb_poll.as_ref() {
                                sb.set_progress(0.0, "", "");
                                sb.set_cancel_action(None);
                            }

                            // Forzar renderizado
                            gl_ref.queue_render();

                            // ✅ Detener el poller AHORA
                            return glib::ControlFlow::Break;
                        }
                    }
                }

                glib::ControlFlow::Continue
            });
        } else {
            if let Some(sb) = status_bar_sim.as_ref() {
                sb.set_progress(0.0, "", "");
                sb.set_cancel_action(None);
            }
            *stock_simulator_3d_stock.borrow_mut() = None;
            *simulation_running_flag.borrow_mut() = false;
            sim_panel_toggle.set_visible(false);
        }
    } else {
        // Clear simulation when disabled
        sim_cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        sim_progress_flag.store(0, std::sync::atomic::Ordering::Relaxed);
        sim_progress_label_toggle.set_text("");
        if let Some(sb) = status_bar_sim.as_ref() {
            sb.set_progress(0.0, "", "");
            sb.set_cancel_action(None);
        }

        *stock_simulator_3d_stock.borrow_mut() = None;
        *simulation_running_flag.borrow_mut() = false;
        sim_panel_toggle.set_visible(false);
        gl_update.queue_render();
    }
});

        // Stock dimension entry handlers
        let stock_material_width = stock_material.clone();
        // Stock parameter changes - update values only, don't trigger simulation
        stock_width_entry.connect_changed(move |entry| {
            if let Ok(width) = entry.text().parse::<f32>() {
                if let Some(ref mut stock) = *stock_material_width.borrow_mut() {
                    stock.width = width;
                }
            }
        });

        let stock_material_height = stock_material.clone();
        stock_height_entry.connect_changed(move |entry| {
            if let Ok(height) = entry.text().parse::<f32>() {
                if let Some(ref mut stock) = *stock_material_height.borrow_mut() {
                    stock.height = height;
                }
            }
        });

        let stock_material_thickness = stock_material.clone();
        stock_thickness_entry.connect_changed(move |entry| {
            if let Ok(thickness) = entry.text().parse::<f32>() {
                if let Some(ref mut stock) = *stock_material_thickness.borrow_mut() {
                    stock.thickness = thickness;
                }
            }
        });

        let tool_diameter = tool_diameter.clone();
        let tool_diameter_entry_state = tool_diameter.clone();
        stock_tool_diameter_entry.connect_changed(move |entry| {
            if let Ok(diameter) = entry.text().parse::<f32>() {
                *tool_diameter_entry_state.borrow_mut() = diameter;
            }
        });

        // Auto-fit when mapped (visible/focused) with a slight delay to allow layout
        let vis_map = visualizer.clone();
        let da_map = drawing_area.clone();
        let update_status = update_ui.clone();
        let device_manager_map = device_manager.clone();
        container.connect_map(move |_| {
            let vis = vis_map.clone();
            let da = da_map.clone();
            let update = update_status.clone();
            let dev_mgr = device_manager_map.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                let width = da.width() as f32;
                let height = da.height() as f32;
                if width > 0.0 && height > 0.0 {
                    let mut v = vis.borrow_mut();
                    // Always fit to device on initialization as per user request
                    Self::apply_fit_to_device(&mut v, &dev_mgr, width, height);
                    drop(v);
                    update();
                    da.queue_draw();
                }
                gtk4::glib::ControlFlow::Break
            });
        });

        // Auto-disable stock removal simulation when leaving the visualizer tab.
        // This forces an explicit re-enable and fresh recomputation on return.
        {
            let show_stock_removal = show_stock_removal.clone();
            container.connect_unmap(move |_| {
                if show_stock_removal.is_active() {
                    show_stock_removal.set_active(false);
                }
            });
        }

        // 3D Renderer Setup
        let renderer_state_clone = renderer_state.clone();
        let visualizer_3d = visualizer.clone();
        let camera_3d = camera.clone();
        let current_pos_3d = current_pos.clone();
        let device_manager_3d = device_manager.clone();
        let stock_simulator_3d_render = stock_simulator_3d.clone();
        let _stock_material_3d = stock_material.clone();
        let stock_simulation_3d_pending_render = stock_simulation_3d_pending.clone();

        // Capture checkbox states
        let show_rapid_3d = show_rapid.clone();
        let show_cut_3d = show_cut.clone();
        let show_grid_3d = show_grid.clone();
        let show_bounds_3d = show_bounds.clone();
        let show_laser_3d = show_laser.clone();
        let show_stock_removal_3d = show_stock_removal.clone();

        let grid_spacing_mm_render = grid_spacing_mm.clone();
        // Guardar el último valor procesado para detectar cambios
        let last_grid_spacing = std::cell::Cell::new(50.0_f64);

        gl_area.connect_render(move |area, _context| {
            if let Some(err) = area.error() {
                tracing::error!(error = %err, "GLArea error");
                return gtk4::glib::Propagation::Stop;
            }

            let mut state_ref = renderer_state_clone.borrow_mut();

            if state_ref.is_none() {
                // SAFETY: GL context is current within GLArea render callback.
                // load_gl_func resolves GL function pointers via epoxy/libGL.
                let gl = unsafe { glow::Context::from_loader_function(load_gl_func) };
                let gl = Rc::new(gl);

                let shader_res = ShaderProgram::new(gl.clone());
                let rapid_res = RenderBuffers::new(gl.clone(), glow::LINES);
                let cut_res = RenderBuffers::new(gl.clone(), glow::LINES);
                let grid_res = RenderBuffers::new(gl.clone(), glow::LINES);
                let axis_res = RenderBuffers::new(gl.clone(), glow::LINES);
                let tool_res = RenderBuffers::new(gl.clone(), glow::TRIANGLES);
                let bounds_res = RenderBuffers::new(gl.clone(), glow::LINES);

                match (
                    shader_res, rapid_res, cut_res, grid_res, axis_res, tool_res, bounds_res,
                ) {
                    (
                        Ok(shader),
                        Ok(rapid_buffers),
                        Ok(cut_buffers),
                        Ok(mut grid_buffers),
                        Ok(mut axis_buffers),
                        Ok(mut tool_buffers),
                        Ok(bounds_buffers),
                    ) => {
                        let grid_data = generate_grid_data(4000.0, 10.0);
                        grid_buffers.update(&grid_data);

                        let axis_data = generate_axis_data(100.0);
                        axis_buffers.update(&axis_data);

                        let tool_data = generate_tool_marker_data();
                        tool_buffers.update(&tool_data);

                        *state_ref = Some(RendererState {
                            shader,
                            rapid_buffers,
                            cut_buffers,
                            grid_buffers,
                            axis_buffers,
                            tool_buffers,
                            bounds_buffers,
                            stock_removal_shader: None,
                            stock_removal_buffers: None,
                        });
                    }
                    (shader, rapid, cut, grid, axis, tool, bounds) => {
                        if let Err(e) = shader {
                            tracing::error!(error = %e, "shader init failed");
                        }
                        if let Err(e) = rapid {
                            tracing::error!(error = %e, "rapid buffer init failed");
                        }
                        if let Err(e) = cut {
                            tracing::error!(error = %e, "cut buffer init failed");
                        }
                        if let Err(e) = grid {
                            tracing::error!(error = %e, "grid buffer init failed");
                        }
                        if let Err(e) = axis {
                            tracing::error!(error = %e, "axis buffer init failed");
                        }
                        if let Err(e) = tool {
                            tracing::error!(error = %e, "tool buffer init failed");
                        }
                        if let Err(e) = bounds {
                            tracing::error!(error = %e, "bounds buffer init failed");
                        }
                        tracing::error!("failed to initialize 3D renderer");
                        return gtk4::glib::Propagation::Stop;
                    }
                }
            }

            if let Some(state) = state_ref.as_mut() {
                let gl = &state.shader.gl;

                // SAFETY: GL context is current; clearing and enabling depth test
                // are standard GL state operations.
                unsafe {
                    gl.clear_color(0.15, 0.15, 0.15, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                    gl.enable(glow::DEPTH_TEST);
                }

                // Update buffers only when data has changed
                let mut vis = visualizer_3d.borrow_mut();
                if vis.is_dirty() {
                    let (rapid_data, cut_data) = generate_vertex_data(&vis);
                    state.rapid_buffers.update(&rapid_data);
                    state.cut_buffers.update(&cut_data);
                    vis.clear_dirty();
                }
                drop(vis);

                // --- Actualización dinámica de la rejilla ---
                let current_spacing = grid_spacing_mm_render.get();
                if current_spacing != last_grid_spacing.get() {
                    let grid_data = generate_grid_data(4000.0, current_spacing as f32);
                    state.grid_buffers.update(&grid_data);
                    last_grid_spacing.set(current_spacing); // Guardar estado
                }

                // Update bounds buffer
                if let Some(manager) = &device_manager_3d {
                    if let Some(profile) = manager.get_active_profile() {
                        let min_x = profile.x_axis.min as f32;
                        let max_x = profile.x_axis.max as f32;
                        let min_y = profile.y_axis.min as f32;
                        let max_y = profile.y_axis.max as f32;
                        let min_z = profile.z_axis.min as f32;
                        let max_z = profile.z_axis.max as f32;

                        let bounds_data =
                            generate_bounds_data(min_x, max_x, min_y, max_y, min_z, max_z);
                        state.bounds_buffers.update(&bounds_data);
                    }
                }

                // Matrices
                let cam = camera_3d.borrow();
                let view = cam.get_view_matrix();
                let proj = cam.get_projection_matrix();
                let mvp = proj * view;

                state.shader.bind();

                if let Some(loc) = state.shader.get_uniform_location("uModelViewProjection") {
                    // SAFETY: GL context is current; uploading a uniform matrix
                    // to a valid location on the bound shader program.
                    unsafe {
                        gl.uniform_matrix_4_f32_slice(Some(&loc), false, &mvp.to_cols_array());
                    }
                }

                // Draw Grid (sin depth test)
                if show_grid_3d.is_active() {
                    unsafe {
                        gl.disable(glow::DEPTH_TEST);  // ← Desactivar depth test
                    }
                    state.grid_buffers.draw();
                    unsafe {
                        gl.enable(glow::DEPTH_TEST);   // ← Reactivar
                    }
                }

                // Draw Axes
                state.axis_buffers.draw();

                // Draw Bounds
                if show_bounds_3d.is_active() {
                    state.bounds_buffers.draw();
                }

                if show_cut_3d.is_active() {
                    state.cut_buffers.draw();
                }

                state.shader.unbind();

                // Draw 3D Stock Removal - CON BACK-FACE CULLING
                if show_stock_removal_3d.is_active() {
                    if let Some(simulator) = stock_simulator_3d_render.borrow().as_ref() {
                        // Initialize stock removal shader if needed
                        if state.stock_removal_shader.is_none() {
                            match StockRemovalShaderProgram::new(gl.clone()) {
                                Ok(stock_shader) => state.stock_removal_shader = Some(stock_shader),
                                Err(e) => tracing::error!(error = %e, "failed to create stock removal shader"),
                            }
                        }

                        // Rebuild mesh when pending or buffers missing
                        if state.stock_removal_buffers.is_none()
                            || *stock_simulation_3d_pending_render.borrow()
                        {
                            let mesh_vertices = generate_surface_mesh(simulator.get_grid());
                            match RenderBuffers::new(gl.clone(), glow::TRIANGLES) {
                                Ok(mut buffers) => {
                                    buffers.update_mesh(&mesh_vertices);
                                    state.stock_removal_buffers = Some(buffers);
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "failed to create stock removal mesh buffers")
                                }
                            }
                            *stock_simulation_3d_pending_render.borrow_mut() = false;
                        }

                        if let (Some(shader), Some(buffers)) =
                            (&state.stock_removal_shader, &state.stock_removal_buffers)
                        {
                            // Configurar estado de renderizado para sólido opaco
                            unsafe {
                                // Deshabilitar blending para evitar transparencias
                                gl.disable(glow::BLEND);
                                // Habilitar depth test para correcta profundidad
                                gl.enable(glow::DEPTH_TEST);
                                // Configurar depth function para que funcione correctamente
                                gl.depth_func(glow::LESS);
                                // Habilitar back-face culling para eliminar caras internas
                                gl.enable(glow::CULL_FACE);
                                gl.cull_face(glow::BACK);
                                // Orientación de caras: CCW (counter-clockwise)
                                gl.front_face(glow::CCW);
                            }

                            shader.bind();

                            // Configurar matriz MVP
                            if let Some(loc) = shader.get_uniform_location("uModelViewProjection") {
                                unsafe {
                                    gl.uniform_matrix_4_f32_slice(
                                        Some(&loc),
                                        false,
                                        &mvp.to_cols_array(),
                                    );
                                }
                            }

                            if let Some(loc) = shader.get_uniform_location("uNormalMatrix") {
                                let normal_matrix = glam::Mat3::from_mat4(view).inverse().transpose();
                                unsafe {
                                    gl.uniform_matrix_3_f32_slice(
                                        Some(&loc),
                                        false,
                                        &normal_matrix.to_cols_array(),
                                    );
                                }
                            }

                            if let Some(loc) = shader.get_uniform_location("uLightDir") {
                                unsafe {
                                    gl.uniform_3_f32(Some(&loc), 0.35, 0.35, 1.0);
                                }
                            }

                            // Dibujar el mesh
                            buffers.draw();

                            // Restaurar estado
                            unsafe {
                                gl.disable(glow::CULL_FACE);
                                // NO deshabilitar depth test porque el toolpath lo necesita
                            }

                            shader.unbind();
                        }
                    } else {
                        debug!("No stock simulator available for rendering");
                    }
                }

                // Después de dibujar el stock removal, dibujar el toolpath
                // Draw Toolpath (asegurar que se dibuja encima del stock)
                if show_rapid_3d.is_active() || show_cut_3d.is_active() {
                    state.shader.bind();

                    if let Some(loc) = state.shader.get_uniform_location("uModelViewProjection") {
                        unsafe {
                            gl.uniform_matrix_4_f32_slice(
                                Some(&loc),
                                false,
                                &mvp.to_cols_array(),
                            );
                        }
                    }

                    // Dibujar con depth test activo pero con offset para evitar z-fighting
                    unsafe {
                        gl.enable(glow::POLYGON_OFFSET_FILL);
                        gl.polygon_offset(-1.0, -1.0);
                    }

                    if show_rapid_3d.is_active() {
                        state.rapid_buffers.draw();
                    }
                    if show_cut_3d.is_active() {
                        state.cut_buffers.draw();
                    }

                    unsafe {
                        gl.disable(glow::POLYGON_OFFSET_FILL);
                    }

                    state.shader.unbind();
                }

                // Draw Tool Marker last so it stays visible above the stock preview.
                if show_laser_3d.is_active() {
                    let pos = *current_pos_3d.borrow();
                    let model = glam::Mat4::from_translation(glam::Vec3::new(pos.0, pos.1, pos.2));
                    let mvp_tool = proj * view * model;

                    state.shader.bind();

                    if let Some(loc) = state.shader.get_uniform_location("uModelViewProjection") {
                        // SAFETY: GL context is current; uploading uniform to valid location.
                        unsafe {
                            gl.uniform_matrix_4_f32_slice(
                                Some(&loc),
                                false,
                                &mvp_tool.to_cols_array(),
                            );
                        }
                    }

                    // SAFETY: GL context is current; drawing marker over stock preview.
                    unsafe {
                        gl.disable(glow::DEPTH_TEST);
                        gl.enable(glow::BLEND);
                        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                    }
                    state.tool_buffers.draw();
                    // SAFETY: GL context is current; restoring GL state after overlay draw.
                    unsafe {
                        gl.disable(glow::BLEND);
                        gl.enable(glow::DEPTH_TEST);
                    }

                    state.shader.unbind();
                }
            }

            gtk4::glib::Propagation::Stop
        });

        // Resize Signal
        let camera_resize = camera.clone();
        gl_area.connect_resize(move |_area, width, height| {
            let mut cam = camera_resize.borrow_mut();
            cam.update_aspect_ratio(width as f32, height as f32);
        });

        // 3D Input Handling
        let gesture_drag = GestureDrag::new();
        let camera_drag = camera.clone();
        let gl_area_drag = gl_area.clone();

        let last_drag_pos = shared((0.0f64, 0.0f64));
        let last_drag_pos_begin = last_drag_pos.clone();

        gesture_drag.connect_drag_begin(move |_, _, _| {
            *last_drag_pos_begin.borrow_mut() = (0.0, 0.0);
        });

        let last_drag_pos_update = last_drag_pos.clone();
        let hadj_3d_drag = hadjustment_3d.clone();
        let vadj_3d_drag = vadjustment_3d.clone();
        let is_updating_3d_drag = is_updating_3d.clone();

        gesture_drag.connect_drag_update(move |gesture, dx, dy| {
            let mut last_pos = last_drag_pos_update.borrow_mut();
            let delta_x = dx - last_pos.0;
            let delta_y = dy - last_pos.1;
            *last_pos = (dx, dy);

            let mut cam = camera_drag.borrow_mut();

            // Check for Shift key
            let is_shift = if let Some(event) = gesture.current_event() {
                event
                    .modifier_state()
                    .contains(gtk4::gdk::ModifierType::SHIFT_MASK)
            } else {
                false
            };

            if is_shift {
                // Pan
                cam.pan(delta_x as f32, delta_y as f32);
            } else {
                // Orbit
                let orbit_scale = (cam.distance / 100.0).clamp(0.2, 5.0);
                let sensitivity = 0.005 * orbit_scale;
                cam.orbit(-delta_x as f32 * sensitivity, -delta_y as f32 * sensitivity);
            }

            // Update scrollbars
            *is_updating_3d_drag.borrow_mut() = true;
            hadj_3d_drag.set_value(cam.target.x as f64);
            vadj_3d_drag.set_value(cam.target.y as f64);
            *is_updating_3d_drag.borrow_mut() = false;

            gl_area_drag.queue_render();
        });
        gl_area.add_controller(gesture_drag);

        // Connect 3D Scrollbars
        let camera_h = camera.clone();
        let gl_area_h = gl_area.clone();
        let is_updating_h = is_updating_3d.clone();
        hadjustment_3d.connect_value_changed(move |adj| {
            if *is_updating_h.borrow() {
                return;
            }
            let val = adj.value();
            let mut cam = camera_h.borrow_mut();
            cam.target.x = val as f32;
            gl_area_h.queue_render();
        });

        let camera_v = camera.clone();
        let gl_area_v = gl_area.clone();
        let is_updating_v = is_updating_3d.clone();
        vadjustment_3d.connect_value_changed(move |adj| {
            if *is_updating_v.borrow() {
                return;
            }
            let val = adj.value();
            let mut cam = camera_v.borrow_mut();
            cam.target.y = -val as f32;
            gl_area_v.queue_render();
        });

        // Update 3D scrollbars on fit/reset
        let update_3d_scrollbars = {
            let hadj = hadjustment_3d.clone();
            let vadj = vadjustment_3d.clone();
            let cam = camera.clone();
            let is_updating = is_updating_3d.clone();
            move || {
                let c = cam.borrow();
                *is_updating.borrow_mut() = true;
                hadj.set_value(c.target.x as f64);
                vadj.set_value(c.target.y as f64);

                // Update page size based on view extent
                let fov_rad = c.fov.to_radians();
                let visible_height = 2.0 * c.distance * (fov_rad / 2.0).tan();
                let visible_width = visible_height * c.aspect_ratio;

                hadj.set_page_size(visible_width as f64);
                vadj.set_page_size(visible_height as f64);

                *is_updating.borrow_mut() = false;
            }
        };

        let scroll_3d = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        let camera_scroll = camera.clone();
        let gl_area_scroll = gl_area.clone();
        let update_scroll_3d = update_3d_scrollbars.clone();

        scroll_3d.connect_scroll(move |_controller, _dx, dy| {
            let mut cam = camera_scroll.borrow_mut();
            let sensitivity = 5.0;
            cam.zoom(dy as f32 * sensitivity);
            drop(cam);
            update_scroll_3d();
            gl_area_scroll.queue_render();
            gtk4::glib::Propagation::Stop
        });
        gl_area.add_controller(scroll_3d);

        // Click focus
        let click_focus = GestureClick::new();
        let gl_focus = gl_area.clone();
        click_focus.connect_pressed(move |_, _, _, _| {
            gl_focus.grab_focus();
        });
        gl_area.add_controller(click_focus);

        Self {
            widget: container,
            stack,
            drawing_area,
            gl_area,
            visualizer,
            camera,
            _renderer_state: renderer_state,
            render_cache: shared(RenderCache::default()),
            _show_rapid: show_rapid,
            _show_cut: show_cut,
            _show_grid: show_grid,
            _show_bounds: show_bounds,
            show_laser,
            show_stock_removal,
            stock_material,
            _simulation_visualization: shared_none(),
            _simulation_running: simulation_running,
            _stock_simulator_3d: stock_simulator_3d,
            _stock_simulation_3d_pending: stock_simulation_3d_pending,
            hadjustment,
            vadjustment,
            hadjustment_3d,
            vadjustment_3d,
            bounds_x_value,
            bounds_y_value,
            min_s_value,
            max_s_value,
            avg_s_value,
            _status_label: status_label,
            settings_controller,
            stock_tool_diameter_entry,
            stock_tool_diameter_mm: tool_diameter,
            stock_width_entry,
            stock_height_entry,
            stock_thickness_entry,
            status_bar,
            current_pos,
            fit_btn_3d,
            run_preview_points,
            run_preview_index,
            run_preview_token,
            run_preview_running,
            run_preview_paused,
            run_preview_speed,
            designer_state,
        }
    }

    pub fn set_gcode(&self, gcode: &str) {
        // Any G-code reload invalidates prior preview trajectory.
        self.stop_run_preview();
        self.run_preview_points.borrow_mut().clear();

        let mut vis = self.visualizer.borrow_mut();
        vis.parse_gcode(gcode);

        // Invalidate render cache when G-code changes
        let mut cache = self.render_cache.borrow_mut();
        cache.cutting_bounds = None;
        drop(cache);

        // ============================================================
        // ✅ DETECTAR SI ES 2D SEGÚN MACHINE_MODE
        // ============================================================
        let is_2d = self.is_machine_2d();

        // Ocultar/mostrar el checkbox de simulación
        self.show_stock_removal.set_visible(!is_2d);
        if is_2d {
            self.show_stock_removal.set_active(false);
        }

        // Update bounds
        let (min_x, max_x, min_y, max_y) = vis
            .get_cutting_bounds()
            .map(|(min_x, max_x, min_y, max_y, _min_z, _max_z)| (min_x, max_x, min_y, max_y))
            .unwrap_or_else(|| vis.get_bounds());

        let system = self
            .settings_controller
            .persistence
            .borrow()
            .config()
            .ui
            .measurement_system;
        let min_x_str = gcodekit5_core::units::format_length(min_x, system);
        let max_x_str = gcodekit5_core::units::format_length(max_x, system);
        let min_y_str = gcodekit5_core::units::format_length(min_y, system);
        let max_y_str = gcodekit5_core::units::format_length(max_y, system);

        self.bounds_x_value
            .set_text(&format!("{} {} {}", min_x_str, t!("to"), max_x_str));
        self.bounds_y_value
            .set_text(&format!("{} {} {}", min_y_str, t!("to"), max_y_str));

        // Calculate S statistics
        let mut min_s = f32::MAX;
        let mut max_s = f32::MIN;
        let mut sum_s = 0.0;
        let mut count_s = 0;

        for cmd in vis.commands() {
            let s = match cmd {
                GCodeCommand::Move {
                    intensity: Some(s), ..
                } => Some(*s),
                GCodeCommand::Arc {
                    intensity: Some(s), ..
                } => Some(*s),
                _ => None,
            };

            if let Some(val) = s {
                if val < min_s {
                    min_s = val;
                }
                if val > max_s {
                    max_s = val;
                }
                sum_s += val;
                count_s += 1;
            }
        }

        if count_s > 0 {
            self.min_s_value.set_text(&format!("{:.1}", min_s));
            self.max_s_value.set_text(&format!("{:.1}", max_s));
            self.avg_s_value
                .set_text(&format!("{:.1}", sum_s / count_s as f32));
        } else {
            self.min_s_value.set_text(&t!("N/A"));
            self.max_s_value.set_text(&t!("N/A"));
            self.avg_s_value.set_text(&t!("N/A"));
        }

        let (min_x, max_x, min_y, max_y, min_z, max_z) = vis
            .get_cutting_bounds()
            .unwrap_or_else(|| {
                let (min_x, max_x, min_y, max_y) = vis.get_bounds();
                (min_x, max_x, min_y, max_y, vis.min_z, vis.max_z)
            });
        drop(vis);

        // ★ Sincronizar stock con el diseñador ★
        self.sync_stock_from_designer();

        // Force 3D top view and fit whenever G-code is loaded
        self.stack.set_visible_child_name("3d");
        self.apply_top_3d_view(
            Vec3::new(min_x, min_y, min_z),
            Vec3::new(max_x, max_y, max_z),
        );

        // Simular Click Fit button
        let fit_btn = self.fit_btn_3d.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            fit_btn.emit_clicked();
            gtk4::glib::ControlFlow::Break
        });

        self.drawing_area.queue_draw();
    }

    /// Detecta si la máquina está en modo 2D (Láser)
    fn is_machine_2d(&self) -> bool {
        if let Some(state) = self.designer_state.as_ref() {
            let mode = state.borrow().machine_mode();
            return mode == MachineMode::Laser2D;
        }
        false  // Por defecto, asumir 3D
    }

    pub fn sync_stock_from_designer(&self) {
        if let Some(state) = self.designer_state.as_ref() {
            if let Some(stock) = &state.borrow().stock_material {
                self.stock_width_entry.set_text(&format!("{:.1}", stock.width));
                self.stock_height_entry.set_text(&format!("{:.1}", stock.height));
                self.stock_thickness_entry.set_text(&format!("{:.1}", stock.thickness));
            }
        }
    }
}

