mod gl_loader;
mod interaction;
mod rendering;

use gcodekit5_core::constants as core_constants;
use gcodekit5_designer::stock_removal::{SimulationResult, StockMaterial};
use gcodekit5_devicedb::DeviceManager;
use gcodekit5_visualizer::visualizer::GCodeCommand;
use gcodekit5_visualizer::{Camera3D, Visualizer};
// use gcodekit5_designer::stock_removal::visualization::generate_2d_contours;
use crate::t;
use crate::ui::gtk::osd_format::format_zoom_center_cursor;
use crate::ui::gtk::shaders::StockRemovalShaderProgram;
use crate::ui::gtk::status_bar::StatusBar;
use gcodekit5_settings::controller::SettingsController;
use gcodekit5_settings::manager::SettingsManager;
use gcodekit5_visualizer::visualizer::{generate_surface_mesh, StockSimulator3D};
use glam::Vec3;

// Stock removal visualization cache
#[derive(Clone)]
pub(crate) struct ContourLayer {
    pub(crate) _z_height: f32,
    pub(crate) color: (f32, f32, f32),
    pub(crate) contours: Vec<Vec<(f32, f32)>>,
}

#[derive(Clone)]
pub(crate) struct StockRemovalVisualization {
    pub(crate) contour_layers: Vec<ContourLayer>,
}
use crate::ui::gtk::nav_cube::NavCube;
use crate::ui::gtk::renderer_3d::{
    generate_axis_data, generate_bounds_data, generate_grid_data, generate_tool_marker_data,
    generate_vertex_data, RenderBuffers,
};
use crate::ui::gtk::shaders::ShaderProgram;
use glow::HasContext;
use gtk4::gdk::Key;
use gtk4::prelude::*;
use gtk4::{EventControllerKey, GestureClick, Popover, Separator};
use tracing::debug;

use gl_loader::load_gl_func;

use gtk4::prelude::{BoxExt, ButtonExt, CheckButtonExt, WidgetExt};
use gtk4::{
    accessible::Property as AccessibleProperty, gdk::ModifierType, Adjustment, Box, Button,
    CheckButton, ComboBoxText, DrawingArea, EventControllerMotion, EventControllerScroll,
    EventControllerScrollFlags, Expander, GLArea, GestureDrag, Grid, Image, Label, ListBox,
    ListBoxRow, Orientation, Overlay, Paned, Revealer, Scrollbar, SelectionMode, Spinner, Stack,
    ToggleButton,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

// Phase 4: Render cache for expensive computations
#[derive(Clone)]
pub(crate) struct RenderCache {
    pub(crate) cache_hash: u64,
    pub(crate) intensity_buckets: Vec<Vec<(f64, f64, f64, f64)>>,
    pub(crate) cutting_bounds: Option<(f32, f32, f32, f32, f32, f32)>,
    pub(crate) total_lines: usize,
    pub(crate) _rapid_lines: usize,
    pub(crate) cut_lines: usize,
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
            cache_hash: 0,
            intensity_buckets: vec![Vec::new(); 20],
            cutting_bounds: None,
            total_lines: 0,
            _rapid_lines: 0,
            cut_lines: 0,
        }
    }
}

impl RenderCache {
    pub(crate) fn needs_rebuild(&self, new_hash: u64) -> bool {
        self.cache_hash != new_hash
    }
}

pub struct GcodeVisualizer {
    pub widget: Paned,
    pub(crate) stack: Stack,
    pub(crate) drawing_area: DrawingArea,
    pub(crate) gl_area: GLArea,
    pub(crate) visualizer: Rc<RefCell<Visualizer>>,
    pub(crate) camera: Rc<RefCell<Camera3D>>,
    pub(crate) _renderer_state: Rc<RefCell<Option<RendererState>>>,
    // Phase 4: Render cache
    pub(crate) render_cache: Rc<RefCell<RenderCache>>,
    // Visibility toggles
    pub(crate) _show_rapid: CheckButton,
    pub(crate) _show_cut: CheckButton,
    pub(crate) _show_grid: CheckButton,
    pub(crate) _show_bounds: CheckButton,
    pub(crate) _show_intensity: CheckButton,
    pub(crate) show_laser: CheckButton,
    pub(crate) show_stock_removal: CheckButton,
    // Stock removal simulation (2D)
    pub(crate) stock_material: Rc<RefCell<Option<StockMaterial>>>,
    pub(crate) simulation_result: Rc<RefCell<Option<SimulationResult>>>,
    pub(crate) _simulation_visualization: Rc<RefCell<Option<StockRemovalVisualization>>>,
    pub(crate) _simulation_resolution: Rc<RefCell<f32>>,
    pub(crate) _simulation_running: Rc<RefCell<bool>>,
    // Stock removal simulation (3D)
    pub(crate) _stock_simulator_3d: Rc<RefCell<Option<StockSimulator3D>>>,
    pub(crate) _stock_simulation_3d_pending: Rc<RefCell<bool>>,
    // Scrollbars
    pub(crate) hadjustment: Adjustment,
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
    pub(crate) device_manager: Option<Arc<DeviceManager>>,
    pub(crate) settings_controller: Rc<SettingsController>,
    #[allow(dead_code)]
    pub(crate) status_bar: Option<StatusBar>,
    pub(crate) current_pos: Rc<RefCell<(f32, f32, f32)>>,
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

    fn apply_fit_to_device(
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

    pub fn new(
        device_manager: Option<Arc<DeviceManager>>,
        settings_controller: Rc<SettingsController>,
        status_bar: Option<StatusBar>,
    ) -> Self {
        let container = Paned::new(Orientation::Horizontal);
        container.add_css_class("visualizer-container");
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Sidebar for controls (compact list + toolbar)
        let sidebar = Box::new(Orientation::Vertical, 8);
        sidebar.set_width_request(200);
        sidebar.add_css_class("visualizer-sidebar");
        sidebar.set_margin_start(12);
        sidebar.set_margin_end(12);
        sidebar.set_margin_top(12);
        sidebar.set_margin_bottom(12);

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

        // Compact 2D/3D segmented control (in a list row)
        let mode_box = Box::new(Orientation::Horizontal, 0);
        mode_box.add_css_class("linked");

        let mode_2d_btn = ToggleButton::new();
        mode_2d_btn.set_tooltip_text(Some(&t!("2D View")));
        mode_2d_btn.update_property(&[AccessibleProperty::Label(&t!("2D View"))]);
        let mode_2d_child = Box::new(Orientation::Horizontal, 4);
        mode_2d_child.append(&Image::from_icon_name("view-grid-symbolic"));
        mode_2d_child.append(&Label::new(Some(&t!("2D"))));
        mode_2d_btn.set_child(Some(&mode_2d_child));

        let mode_3d_btn = ToggleButton::new();
        mode_3d_btn.set_tooltip_text(Some(&t!("3D View")));
        mode_3d_btn.update_property(&[AccessibleProperty::Label(&t!("3D View"))]);
        let mode_3d_child = Box::new(Orientation::Horizontal, 4);
        mode_3d_child.append(&Image::from_icon_name("video-display-symbolic"));
        mode_3d_child.append(&Label::new(Some(&t!("3D"))));
        mode_3d_btn.set_child(Some(&mode_3d_child));

        mode_2d_btn.set_active(true);

        mode_box.append(&mode_2d_btn);
        mode_box.append(&mode_3d_btn);

        let sidebar_list = ListBox::new();
        sidebar_list.set_selection_mode(SelectionMode::None);
        sidebar_list.add_css_class("visualizer-sidebar-list");

        {
            let row = ListBoxRow::new();
            let mode_row = Box::new(Orientation::Horizontal, 8);
            let mode_label = Label::new(Some(&t!("Mode")));
            mode_label.add_css_class("caption");
            mode_label.set_halign(gtk4::Align::Start);
            mode_label.set_hexpand(true);
            mode_row.append(&mode_label);
            mode_row.append(&mode_box);
            row.set_child(Some(&mode_row));
            sidebar_list.append(&row);
        }

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

        let grid_spacing_mm = Rc::new(std::cell::Cell::new(10.0_f64));
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
        grid_spacing_combo.set_active_id(Some("10"));

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
        let show_intensity = CheckButton::builder()
            .label(t!("Show Intensity"))
            .active(false)
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
            .build();
        show_stock_removal.set_visible(enable_stock_removal_3d);

        // Stock configuration

        let stock_width_entry = gtk4::Entry::builder()
            .placeholder_text(t!("Width"))
            .text("200.0")
            .build();
        let stock_height_entry = gtk4::Entry::builder()
            .placeholder_text(t!("Height"))
            .text("200.0")
            .build();
        let stock_thickness_entry = gtk4::Entry::builder()
            .placeholder_text(t!("Thickness"))
            .text("10.0")
            .build();
        let stock_tool_diameter_entry = gtk4::Entry::builder()
            .placeholder_text(t!("Tool Diameter"))
            .text("3.175")
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
        simulation_box.append(&show_intensity);
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

        // Queue redraw on grid spacing change
        {
            let drawing_area = drawing_area.clone();
            grid_spacing_combo.connect_changed(move |_| drawing_area.queue_draw());
        }

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

        // Stack for 2D/3D
        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);

        // 2D Page (Grid with DrawingArea + Scrollbars)
        let grid = Grid::builder().hexpand(true).vexpand(true).build();

        grid.attach(&drawing_area, 0, 0, 1, 1);
        grid.attach(&vscrollbar, 1, 0, 1, 1);
        grid.attach(&hscrollbar, 0, 1, 1, 1);

        stack.add_titled(&grid, Some("2d"), &t!("2D View"));

        // 3D Page
        let gl_area = GLArea::builder().hexpand(true).vexpand(true).build();
        gl_area.set_required_version(3, 3);

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
        hscrollbar_3d.set_visible(false);
        vscrollbar_3d.set_visible(false);

        let grid_3d = Grid::builder().hexpand(true).vexpand(true).build();

        grid_3d.attach(&gl_area, 0, 0, 1, 1);
        grid_3d.attach(&vscrollbar_3d, 1, 0, 1, 1);
        grid_3d.attach(&hscrollbar_3d, 0, 1, 1, 1);

        stack.add_titled(&grid_3d, Some("3d"), &t!("3D View"));

        // Initialize Visualizer logic
        let visualizer = Rc::new(RefCell::new(Visualizer::new()));
        let current_pos = Rc::new(RefCell::new((0.0f32, 0.0f32, 0.0f32)));
        let camera = Rc::new(RefCell::new(Camera3D::default()));
        let renderer_state = Rc::new(RefCell::new(None));
        let is_updating_3d = Rc::new(RefCell::new(false));

        // Stock removal simulation - use default sensible values
        let initial_stock = Some(StockMaterial {
            width: 200.0,
            height: 200.0,
            thickness: 10.0,
            origin: (0.0, 0.0, 0.0),
            safe_z: 10.0,
        });
        let stock_material = Rc::new(RefCell::new(initial_stock));
        let tool_diameter = Rc::new(RefCell::new(3.175f32)); // Default 1/8" end mill
        let simulation_result = Rc::new(RefCell::new(None));
        let simulation_visualization = Rc::new(RefCell::new(None));
        let simulation_resolution = Rc::new(RefCell::new(0.1));
        let simulation_running = Rc::new(RefCell::new(false));
        let stock_simulator_3d = Rc::new(RefCell::new(None));
        let stock_simulation_3d_pending = Rc::new(RefCell::new(false));

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

        // Floating Controls (Bottom Right)
        let floating_box = Box::new(Orientation::Horizontal, 4);
        floating_box.add_css_class("visualizer-osd");
        floating_box.set_halign(gtk4::Align::End);
        floating_box.set_valign(gtk4::Align::End);
        floating_box.set_margin_bottom(20);
        floating_box.set_margin_end(20);

        let float_zoom_out = Button::builder()
            .icon_name("zoom-out-symbolic")
            .tooltip_text(t!("Zoom Out"))
            .build();
        float_zoom_out.update_property(&[AccessibleProperty::Label(&t!("Zoom Out"))]);

        let float_fit = Button::builder()
            .icon_name("zoom-fit-best-symbolic")
            .tooltip_text(t!("Fit to Content"))
            .build();
        float_fit.update_property(&[AccessibleProperty::Label(&t!("Fit to Content"))]);

        let float_reset = Button::builder()
            .icon_name("view-restore-symbolic")
            .tooltip_text(t!("Fit to Viewport"))
            .build();
        float_reset.update_property(&[AccessibleProperty::Label(&t!("Fit to Viewport"))]);

        let float_fit_device = Button::builder()
            .icon_name("preferences-desktop-display-symbolic")
            .tooltip_text(t!("Fit to Device Working Area"))
            .build();
        float_fit_device
            .update_property(&[AccessibleProperty::Label(&t!("Fit to Device Working Area"))]);

        let scrollbars_btn = Button::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text(t!("Toggle Scrollbars"))
            .build();
        scrollbars_btn.update_property(&[AccessibleProperty::Label(&t!("Toggle Scrollbars"))]);

        let help_btn = Button::builder()
            .label("?")
            .tooltip_text(t!("Keyboard Shortcuts"))
            .build();
        help_btn.update_property(&[AccessibleProperty::Label(&t!("Keyboard Shortcuts"))]);

        let help_popover = Popover::new();
        help_popover.set_parent(&help_btn);
        help_popover.set_has_arrow(true);
        {
            let help_box = Box::new(Orientation::Vertical, 6);
            help_box.set_margin_start(12);
            help_box.set_margin_end(12);
            help_box.set_margin_top(12);
            help_box.set_margin_bottom(12);
            help_box.append(&Label::new(Some(&t!("Visualizer shortcuts"))));
            help_box.append(&Label::new(Some("+ / -  —  Zoom")));
            help_box.append(&Label::new(Some("F  —  Fit to Content")));
            help_box.append(&Label::new(Some("R  —  Fit to Viewport")));
            help_box.append(&Label::new(Some("D  —  Fit to Device Working Area")));
            help_box.append(&Label::new(Some(&t!("Right click for context menu"))));
            help_popover.set_child(Some(&help_box));
        }
        {
            let pop = help_popover.clone();
            help_btn.connect_clicked(move |_| pop.popup());
        }

        let float_zoom_in = Button::builder()
            .icon_name("zoom-in-symbolic")
            .tooltip_text(t!("Zoom In"))
            .build();
        float_zoom_in.update_property(&[AccessibleProperty::Label(&t!("Zoom In"))]);

        for b in [
            &float_zoom_out,
            &float_fit,
            &float_reset,
            &float_fit_device,
            &scrollbars_btn,
            &help_btn,
            &float_zoom_in,
        ] {
            b.set_size_request(32, 32);
        }

        floating_box.append(&float_zoom_out);
        floating_box.append(&float_fit);
        floating_box.append(&float_reset);
        if device_manager.is_some() {
            floating_box.append(&float_fit_device);
        }
        floating_box.append(&scrollbars_btn);
        floating_box.append(&help_btn);
        floating_box.append(&float_zoom_in);

        // Status Panel (Bottom Left)
        let status_box = Box::new(Orientation::Horizontal, 4);
        status_box.add_css_class("visualizer-osd");
        status_box.set_halign(gtk4::Align::Start);
        status_box.set_valign(gtk4::Align::End);
        status_box.set_margin_bottom(20);
        status_box.set_margin_start(20);

        let status_label = Label::builder().label(" ").build();
        status_label.set_hexpand(true);

        let units_badge = Label::new(Some(""));
        units_badge.add_css_class("osd-units-badge");

        status_box.append(&status_label);
        status_box.append(&units_badge);

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
        overlay.add_overlay(&sidebar_show_panel);
        overlay.add_overlay(&sim_panel);

        container.set_end_child(Some(&overlay));

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
                    let (min_x_2d, max_x_2d, min_y_2d, max_y_2d) = vis.get_bounds();
                    (min_x_2d, max_x_2d, min_y_2d, max_y_2d, vis.min_z, vis.max_z)
                };
            drop(vis);

            let mut cam = cam_fit_3d.borrow_mut();
            cam.fit_to_bounds(
                Vec3::new(min_x, min_y, min_z),
                Vec3::new(max_x, max_y, max_z),
            );

            // Update scrollbars
            *is_updating_fit_3d.borrow_mut() = true;
            hadj_fit_3d.set_value(cam.target.x as f64);
            vadj_fit_3d.set_value(cam.target.y as f64);
            *is_updating_fit_3d.borrow_mut() = false;

            gl_area_fit_3d.queue_render();
        });

        // View mode switching
        {
            let stack = stack.clone();
            let other = mode_3d_btn.clone();
            mode_2d_btn.connect_toggled(move |btn| {
                if btn.is_active() {
                    stack.set_visible_child_name("2d");
                    if other.is_active() {
                        other.set_active(false);
                    }
                }
            });
        }
        {
            let stack = stack.clone();
            let other = mode_2d_btn.clone();
            mode_3d_btn.connect_toggled(move |btn| {
                if btn.is_active() {
                    stack.set_visible_child_name("3d");
                    if other.is_active() {
                        other.set_active(false);
                    }
                }
            });
        }

        // Visibility Logic
        let nav_widget = nav_cube.widget.clone();
        let float_box = floating_box.clone();
        let show_intensity_vis = show_intensity.clone();
        let mode_2d_btn_vis = mode_2d_btn.clone();
        let mode_3d_btn_vis = mode_3d_btn.clone();

        // Initial state
        nav_widget.set_visible(false); // Start in 2D mode

        stack.connect_visible_child_name_notify(move |stack| {
            let is_3d = stack.visible_child_name().as_deref() == Some("3d");
            if is_3d {
                nav_widget.set_visible(true);
                float_box.set_visible(false);
                show_intensity_vis.set_active(false);
                show_intensity_vis.set_sensitive(false);
            } else {
                nav_widget.set_visible(false);
                float_box.set_visible(true);
                show_intensity_vis.set_sensitive(true);
            }

            if mode_3d_btn_vis.is_active() != is_3d {
                mode_3d_btn_vis.set_active(is_3d);
            }
            if mode_2d_btn_vis.is_active() == is_3d {
                mode_2d_btn_vis.set_active(!is_3d);
            }
        });

        // Helper to update status
        let cursor_pos = Rc::new(RefCell::new((0.0_f32, 0.0_f32)));
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

        // Keyboard shortcuts (when the canvas has focus)
        drawing_area.set_focusable(true);
        drawing_area.set_can_focus(true);
        {
            let click_for_focus = GestureClick::new();
            let da_focus = drawing_area.clone();
            click_for_focus.connect_pressed(move |_, _, _, _| {
                da_focus.grab_focus();
            });
            drawing_area.add_controller(click_for_focus);
        }
        {
            let key_controller = EventControllerKey::new();
            let vis_keys = visualizer.clone();
            let da_keys = drawing_area.clone();
            let update_keys = update_status_fn.clone();
            let device_mgr_keys = device_manager.clone();
            key_controller.connect_key_pressed(move |_, key, _code, modifiers: ModifierType| {
                if modifiers.contains(ModifierType::CONTROL_MASK)
                    || modifiers.contains(ModifierType::ALT_MASK)
                {
                    return gtk4::glib::Propagation::Proceed;
                }

                match key {
                    Key::plus | Key::KP_Add | Key::equal => {
                        vis_keys.borrow_mut().zoom_in();
                        update_keys();
                        da_keys.queue_draw();
                        gtk4::glib::Propagation::Stop
                    }
                    Key::minus | Key::KP_Subtract | Key::underscore => {
                        vis_keys.borrow_mut().zoom_out();
                        update_keys();
                        da_keys.queue_draw();
                        gtk4::glib::Propagation::Stop
                    }
                    Key::f | Key::F => {
                        let width = da_keys.width() as f32;
                        let height = da_keys.height() as f32;
                        if width > 0.0 && height > 0.0 {
                            vis_keys.borrow_mut().fit_to_view(width, height);
                            update_keys();
                            da_keys.queue_draw();
                        }
                        gtk4::glib::Propagation::Stop
                    }
                    Key::r | Key::R => {
                        let mut v = vis_keys.borrow_mut();
                        v.reset_zoom();
                        v.reset_pan();
                        drop(v);
                        update_keys();
                        da_keys.queue_draw();
                        gtk4::glib::Propagation::Stop
                    }
                    Key::d | Key::D => {
                        let width = da_keys.width() as f32;
                        let height = da_keys.height() as f32;
                        if width > 0.0 && height > 0.0 {
                            let mut v = vis_keys.borrow_mut();
                            Self::apply_fit_to_device(&mut v, &device_mgr_keys, width, height);
                            drop(v);
                            update_keys();
                            da_keys.queue_draw();
                        }
                        gtk4::glib::Propagation::Stop
                    }
                    _ => gtk4::glib::Propagation::Proceed,
                }
            });
            drawing_area.add_controller(key_controller);
        }

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
        let is_updating = Rc::new(RefCell::new(false));
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

        // Connect Floating Controls
        let vis_float_out = visualizer.clone();
        let da_float_out = drawing_area.clone();
        let update_status = update_ui.clone();
        float_zoom_out.connect_clicked(move |_| {
            vis_float_out.borrow_mut().zoom_out();
            update_status();
            da_float_out.queue_draw();
        });

        let vis_float_in = visualizer.clone();
        let da_float_in = drawing_area.clone();
        let update_status = update_ui.clone();
        float_zoom_in.connect_clicked(move |_| {
            vis_float_in.borrow_mut().zoom_in();
            update_status();
            da_float_in.queue_draw();
        });

        let vis_float_fit = visualizer.clone();
        let da_float_fit = drawing_area.clone();
        let update_status = update_ui.clone();
        float_fit.connect_clicked(move |_| {
            let width = da_float_fit.width() as f32;
            let height = da_float_fit.height() as f32;
            if width > 0.0 && height > 0.0 {
                vis_float_fit.borrow_mut().fit_to_view(width, height);
                update_status();
                da_float_fit.queue_draw();
            }
        });

        let vis_float_reset = visualizer.clone();
        let da_float_reset = drawing_area.clone();
        let update_status = update_ui.clone();
        float_reset.connect_clicked(move |_| {
            let mut vis = vis_float_reset.borrow_mut();
            vis.reset_zoom();
            vis.reset_pan();
            drop(vis);
            update_status();
            da_float_reset.queue_draw();
        });

        // Fit to device in OSD (matches Designer)
        let vis_float_fit_dev = visualizer.clone();
        let da_float_fit_dev = drawing_area.clone();
        let update_status = update_ui.clone();
        let device_manager_fit_dev = device_manager.clone();
        float_fit_device.connect_clicked(move |_| {
            let width = da_float_fit_dev.width() as f32;
            let height = da_float_fit_dev.height() as f32;
            if width > 0.0 && height > 0.0 {
                let mut v = vis_float_fit_dev.borrow_mut();
                Self::apply_fit_to_device(&mut v, &device_manager_fit_dev, width, height);
                drop(v);
                update_status();
                da_float_fit_dev.queue_draw();
            }
        });

        // Scrollbars toggle (2D + 3D)
        let show_scrollbars = Rc::new(std::cell::Cell::new(false));
        let hsb_2d = hscrollbar.clone();
        let vsb_2d = vscrollbar.clone();
        let hsb_3d = hscrollbar_3d.clone();
        let vsb_3d = vscrollbar_3d.clone();
        let show_scrollbars_btn = show_scrollbars.clone();
        scrollbars_btn.connect_clicked(move |_| {
            let next = !show_scrollbars_btn.get();
            show_scrollbars_btn.set(next);
            hsb_2d.set_visible(next);
            vsb_2d.set_visible(next);
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

        // Connect Draw Signal
        let vis_draw = visualizer.clone();
        let render_cache_draw = Rc::new(RefCell::new(RenderCache::default()));
        let show_rapid_draw = show_rapid.clone();
        let show_cut_draw = show_cut.clone();
        let show_grid_draw = show_grid.clone();
        let show_bounds_draw = show_bounds.clone();
        let show_intensity_draw = show_intensity.clone();
        let show_laser_draw = show_laser.clone();
        let show_stock_removal_draw = show_stock_removal.clone();
        let simulation_result_draw = simulation_result.clone();
        let simulation_visualization_draw = simulation_visualization.clone();
        let stock_material_draw = stock_material.clone();
        let device_manager_draw = device_manager.clone();
        let current_pos_draw = current_pos.clone();
        let grid_spacing_draw = grid_spacing_mm.clone();
        let settings_draw = settings_controller.clone();

        drawing_area.set_draw_func(move |da, cr, width, height| {
            let vis = vis_draw.borrow();
            let mut cache = render_cache_draw.borrow_mut();
            let pos = *current_pos_draw.borrow();
            let style = da.style_context();
            let config = settings_draw.persistence.borrow();
            let grid_major_width = config.config().ui.grid_major_line_width;
            let grid_minor_width = config.config().ui.grid_minor_line_width;
            drop(config);
            Self::draw(
                cr,
                &vis,
                &mut cache,
                width as f64,
                height as f64,
                show_rapid_draw.is_active(),
                show_cut_draw.is_active(),
                show_grid_draw.is_active(),
                show_bounds_draw.is_active(),
                show_intensity_draw.is_active(),
                show_laser_draw.is_active(),
                show_stock_removal_draw.is_active(),
                &simulation_result_draw.borrow(),
                &simulation_visualization_draw.borrow(),
                &stock_material_draw.borrow(),
                pos,
                &device_manager_draw,
                grid_spacing_draw.get(),
                grid_major_width,
                grid_minor_width,
                &style,
            );
        });

        // Connect Controls
        let vis_fit = visualizer.clone();
        let da_fit = drawing_area.clone();
        let gl_area_fit = gl_area.clone();
        let stack_fit = stack.clone();
        let camera_fit = camera.clone();
        let update_status = update_ui.clone();
        let hadj_fit_main_3d = hadjustment_3d.clone();
        let vadj_fit_main_3d = vadjustment_3d.clone();
        let is_updating_fit_main_3d = is_updating_3d.clone();

        fit_btn.connect_clicked(move |_| {
            if stack_fit.visible_child_name().as_deref() == Some("3d") {
                let vis = vis_fit.borrow();
                let (min_x, max_x, min_y, max_y, min_z, max_z) =
                    if let Some(bounds) = vis.get_cutting_bounds() {
                        bounds
                    } else {
                        let (min_x_2d, max_x_2d, min_y_2d, max_y_2d) = vis.get_bounds();
                        (min_x_2d, max_x_2d, min_y_2d, max_y_2d, vis.min_z, vis.max_z)
                    };
                drop(vis);

                let mut cam = camera_fit.borrow_mut();
                cam.fit_to_bounds(
                    Vec3::new(min_x, min_y, min_z),
                    Vec3::new(max_x, max_y, max_z),
                );

                // Update scrollbars
                *is_updating_fit_main_3d.borrow_mut() = true;
                hadj_fit_main_3d.set_value(cam.target.x as f64);
                vadj_fit_main_3d.set_value(cam.target.y as f64);
                *is_updating_fit_main_3d.borrow_mut() = false;

                gl_area_fit.queue_render();
            } else {
                let width = da_fit.width() as f32;
                let height = da_fit.height() as f32;
                vis_fit.borrow_mut().fit_to_view(width, height);
                update_status();
                da_fit.queue_draw();
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
        show_intensity.connect_toggled(move |_| {
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
        let _simulation_result_stock = simulation_result.clone();
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
                    use std::sync::{Arc, Mutex};

                    // Run simulation in background thread
                    let stock_clone = stock.clone();
                    let tool_radius_value = *tool_diameter_stock.borrow() / 2.0;
                    let result_3d_ref = stock_simulator_3d_stock.clone();
                    let gl_ref = gl_update.clone();

                    // Convert GCode commands to toolpath segments for 3D
                    use gcodekit5_visualizer::{ToolpathSegment, ToolpathSegmentType};
                    let mut toolpath_segments_3d = Vec::new();

                    // G-code Z is typically negative when cutting (Z=-5 means 5mm below surface)
                    // Voxel grid expects Z from 0 (bottom) to thickness (top)
                    // So we convert: voxel_z = stock_thickness + gcode_z
                    let stock_thickness = stock_clone.thickness;

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
                                // Convert G-code Z (negative) to voxel Z (positive from bottom)
                                let start_z = stock_thickness + from.z;
                                let end_z = stock_thickness + to.z;
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
                                // Convert G-code Z (negative) to voxel Z (positive from bottom)
                                let start_z = stock_thickness + from.z;
                                let end_z = stock_thickness + to.z;
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
                    let result_arc = Arc::new(Mutex::new(None));
                    let result_arc_clone = result_arc.clone();

                    let cancel_thread = sim_cancel_flag.clone();
                    let progress_thread = sim_progress_flag.clone();

                    std::thread::spawn(move || {
                        use gcodekit5_visualizer::{StockSimulator3D, VoxelGrid};

                        let resolution = 0.25; // 0.25mm voxel resolution (doubled from 0.5mm)
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
                        let _ =
                            simulator.simulate_toolpath_with_progress(&toolpath_segments_3d, |p| {
                                if p > 0.0 {
                                    progress.store(
                                        (p * 100.0).round() as usize,
                                        std::sync::atomic::Ordering::Relaxed,
                                    );
                                }
                                !cancel.load(std::sync::atomic::Ordering::SeqCst)
                            });
                        progress.store(100, std::sync::atomic::Ordering::Relaxed);

                        let result_sim = simulator;

                        // Store in Arc
                        *result_arc_clone.lock().unwrap_or_else(|p| p.into_inner()) =
                            Some(result_sim);
                    });

                    // Poll for completion on main thread with timeout limit
                    let result_arc_poll = result_arc.clone();
                    let poll_count = Rc::new(RefCell::new(0u32));
                    let poll_count_clone = poll_count.clone();
                    let sim_running_poll = simulation_running_flag.clone();

                    let pending_flag = stock_simulation_3d_pending_toggle.clone();
                    let sim_cancel_flag_poll = sim_cancel_flag.clone();
                    let sim_panel_toggle_poll = sim_panel_toggle.clone();
                    let sim_progress_poll = sim_progress_flag.clone();
                    let sim_progress_label_poll = sim_progress_label_toggle.clone();
                    let sb_poll = status_bar_sim.clone();
                    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                        *poll_count_clone.borrow_mut() += 1;

                        let pct = sim_progress_poll
                            .load(std::sync::atomic::Ordering::Relaxed)
                            .min(100);
                        sim_progress_label_poll.set_text(&format!("{}%", pct));
                        if let Some(sb) = sb_poll.as_ref() {
                            let elapsed = started_at.elapsed().as_secs_f64();
                            sb.set_progress((pct as f64).max(0.1), &format!("{:.0}s", elapsed), "");
                        }

                        if sim_cancel_flag_poll.load(std::sync::atomic::Ordering::SeqCst) {
                            *sim_running_poll.borrow_mut() = false;
                            sim_panel_toggle_poll.set_visible(false);
                            if let Some(sb) = sb_poll.as_ref() {
                                sb.set_progress(0.0, "", "");
                                sb.set_cancel_action(None);
                            }
                            return glib::ControlFlow::Break;
                        }

                        // Stop after 300 iterations (30 seconds)
                        if *poll_count_clone.borrow() > 300 {
                            *sim_running_poll.borrow_mut() = false;
                            sim_panel_toggle_poll.set_visible(false);
                            if let Some(sb) = sb_poll.as_ref() {
                                sb.set_progress(0.0, "", "");
                                sb.set_cancel_action(None);
                            }
                            return glib::ControlFlow::Break;
                        }

                        if let Ok(mut guard) = result_arc_poll.try_lock() {
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

                                *result_3d_ref.borrow_mut() = Some(result_simulator);
                                *pending_flag.borrow_mut() = true;

                                *sim_running_poll.borrow_mut() = false;
                                sim_panel_toggle_poll.set_visible(false);
                                if let Some(sb) = sb_poll.as_ref() {
                                    sb.set_progress(0.0, "", "");
                                    sb.set_cancel_action(None);
                                }
                                gl_ref.queue_render();

                                return glib::ControlFlow::Break;
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
        stock_tool_diameter_entry.connect_changed(move |entry| {
            if let Ok(diameter) = entry.text().parse::<f32>() {
                *tool_diameter.borrow_mut() = diameter;
            }
        });

        // Mouse Interaction
        Self::setup_interaction(&drawing_area, &visualizer, update_ui.clone());

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

        gl_area.connect_render(move |area, _context| {
            if let Some(err) = area.error() {
                tracing::error!(error = %err, "GLArea error");
                return gtk4::glib::Propagation::Stop;
            }

            let mut state_ref = renderer_state_clone.borrow_mut();

            if state_ref.is_none() {
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
                    unsafe {
                        gl.uniform_matrix_4_f32_slice(Some(&loc), false, &mvp.to_cols_array());
                    }
                }

                // Draw Grid
                if show_grid_3d.is_active() {
                    state.grid_buffers.draw();
                }

                // Draw Axes
                state.axis_buffers.draw();

                // Draw Bounds
                if show_bounds_3d.is_active() {
                    state.bounds_buffers.draw();
                }

                // Draw Toolpath
                if show_rapid_3d.is_active() {
                    state.rapid_buffers.draw();
                }
                if show_cut_3d.is_active() {
                    state.cut_buffers.draw();
                }

                // Draw Tool Marker
                if show_laser_3d.is_active() {
                    let pos = *current_pos_3d.borrow();
                    let model = glam::Mat4::from_translation(glam::Vec3::new(pos.0, pos.1, pos.2));
                    let mvp_tool = proj * view * model;

                    if let Some(loc) = state.shader.get_uniform_location("uModelViewProjection") {
                        unsafe {
                            gl.uniform_matrix_4_f32_slice(
                                Some(&loc),
                                false,
                                &mvp_tool.to_cols_array(),
                            );
                        }
                    }

                    unsafe {
                        gl.enable(glow::BLEND);
                        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                    }
                    state.tool_buffers.draw();
                    unsafe {
                        gl.disable(glow::BLEND);
                    }
                }

                state.shader.unbind();

                // Draw 3D Stock Removal
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
                            shader.bind();

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
                                let normal_matrix =
                                    glam::Mat3::from_mat4(view).inverse().transpose();
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

                            unsafe {
                                gl.enable(glow::CULL_FACE);
                                gl.cull_face(glow::BACK);
                            }
                            buffers.draw();
                            unsafe {
                                gl.disable(glow::CULL_FACE);
                            }

                            shader.unbind();
                        }
                    } else {
                        debug!("No stock simulator available for rendering");
                    }
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

        let last_drag_pos = Rc::new(RefCell::new((0.0f64, 0.0f64)));
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
                // Pass raw screen deltas, camera handles scaling by distance
                // X is positive when dragging right.
                // To move object right (pan right), we need to move camera left.
                // Camera::pan(delta_x) moves target left by delta_x.
                // So passing positive delta_x moves target left -> object right.
                // Wait, if target moves left, camera moves left.
                // If camera moves left, object appears to move right.
                // So positive delta_x -> object moves right.
                // User said "reversed", so currently it must be moving object left.
                // Currently: cam.pan(-delta_x, ...)
                // -delta_x is negative.
                // Camera::pan(neg) -> target moves right -> camera moves right -> object moves left.
                // So yes, remove the negation to make object move right.
                cam.pan(delta_x as f32, delta_y as f32);
            } else {
                // Orbit
                // Scale orbit speed by distance for finer control when zoomed in
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
            cam.target.y = val as f32;
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

        Self {
            widget: container,
            stack,
            drawing_area,
            gl_area,
            visualizer,
            camera,
            _renderer_state: renderer_state,
            render_cache: Rc::new(RefCell::new(RenderCache::default())),
            _show_rapid: show_rapid,
            _show_cut: show_cut,
            _show_grid: show_grid,
            _show_bounds: show_bounds,
            _show_intensity: show_intensity,
            show_laser,
            show_stock_removal,
            stock_material,
            simulation_result,
            _simulation_visualization: Rc::new(RefCell::new(None)),
            _simulation_resolution: simulation_resolution,
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
            device_manager,
            settings_controller,
            status_bar,
            current_pos,
        }
    }

    pub fn set_gcode(&self, gcode: &str) {
        let mut vis = self.visualizer.borrow_mut();
        vis.parse_gcode(gcode);

        // Phase 4: Invalidate render cache when G-code changes
        let mut cache = self.render_cache.borrow_mut();
        cache.cache_hash = 0; // Force rebuild
        cache.cutting_bounds = None;
        drop(cache);

        // Update bounds
        // Note: Visualizer::get_bounds() includes viewport padding and an origin clamp (min <= 0) for nicer navigation.
        // For the Inspector we want the true cutting extents.
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

        // Auto fit
        let width = self.drawing_area.width() as f32;
        let height = self.drawing_area.height() as f32;
        if width > 0.0 && height > 0.0 {
            if vis.get_command_count() > 0 {
                vis.fit_to_view(width, height);
            } else {
                Self::apply_fit_to_device(&mut vis, &self.device_manager, width, height);
            }
        }

        // Auto-detect 3D content
        let has_z_travel = if let Some((_, _, _, _, min_z, max_z)) = vis.get_cutting_bounds() {
            (max_z - min_z).abs() > 0.001
        } else {
            false
        };

        if has_z_travel {
            self.stack.set_visible_child_name("3d");
            // Explicitly disable intensity for 3D view
            self._show_intensity.set_active(false);
            self._show_intensity.set_sensitive(false);

            // Fit 3D view
            let (min_x, max_x, min_y, max_y, min_z, max_z) =
                if let Some(bounds) = vis.get_cutting_bounds() {
                    bounds
                } else {
                    let (min_x_2d, max_x_2d, min_y_2d, max_y_2d) = vis.get_bounds();
                    (min_x_2d, max_x_2d, min_y_2d, max_y_2d, vis.min_z, vis.max_z)
                };

            let (target_x, target_y) = {
                let mut cam = self.camera.borrow_mut();
                cam.fit_to_bounds(
                    Vec3::new(min_x, min_y, min_z),
                    Vec3::new(max_x, max_y, max_z),
                );
                (cam.target.x, cam.target.y)
            };

            // Update 3D scrollbars
            self.hadjustment_3d.set_value(target_x as f64);
            self.vadjustment_3d.set_value(target_y as f64);

            self.gl_area.queue_render();
        } else {
            self.stack.set_visible_child_name("2d");
        }

        // Run stock removal simulation if enabled
        if self.show_stock_removal.is_active() {
            if let Some(stock) = self.stock_material.borrow().as_ref() {
                use gcodekit5_designer::stock_removal::StockSimulator2D;
                use gcodekit5_designer::{create_arc_segment, create_linear_segment};

                // Convert GCode commands to toolpath segments
                let mut toolpath_segments = Vec::new();
                for cmd in vis.commands() {
                    match cmd {
                        GCodeCommand::Move {
                            from, to, rapid, ..
                        } => {
                            let segment = create_linear_segment(
                                from.x, from.y, from.z, to.x, to.y, to.z, *rapid,
                                100.0, // Default feed rate
                                3000,  // Default spindle speed
                            );
                            toolpath_segments.push(segment);
                        }
                        GCodeCommand::Arc {
                            from,
                            to,
                            center,
                            clockwise,
                            ..
                        } => {
                            let segment = create_arc_segment(
                                from.x, from.y, from.z, to.x, to.y, to.z, center.x, center.y,
                                *clockwise, 100.0, // Default feed rate
                                3000,  // Default spindle speed
                            );
                            toolpath_segments.push(segment);
                        }
                        GCodeCommand::Dwell { .. } => {
                            // Dwell commands don't remove material, skip
                        }
                    }
                }

                // Create simulator with default tool radius
                let tool_radius = 1.585; // 3.17mm diameter / 2
                let resolution = 0.1; // 0.1mm resolution
                let mut simulator = StockSimulator2D::new(stock.clone(), tool_radius, resolution);

                // Run simulation
                simulator.simulate_toolpath(&toolpath_segments);
                let result = simulator.get_simulation_result();
                *self.simulation_result.borrow_mut() = Some(result);
            }
        } else {
            *self.simulation_result.borrow_mut() = None;
        }

        drop(vis);
        self.update_scrollbars();
        self.drawing_area.queue_draw();
    }
}

#[cfg(test)]
mod tests_visualizer {
    use super::*;
    // Visualizer is already imported via super::*

    #[test]
    fn test_apply_fit_to_device_with_no_profile_uses_default_bbox() {
        let mut vis = Visualizer::new();
        let width = 1200.0f32;
        let height = 800.0f32;

        // Call apply_fit_to_device with no device manager
        GcodeVisualizer::apply_fit_to_device(&mut vis, &None, width, height);

        // Compute expected scale
        let margin_percent = 0.05f32;
        let available_width = width * (1.0 - margin_percent * 2.0);
        let available_height = height * (1.0 - margin_percent * 2.0);
        let expected_scale = (available_width / core_constants::DEFAULT_WORK_WIDTH_MM as f32)
            .min(available_height / core_constants::DEFAULT_WORK_HEIGHT_MM as f32);

        assert!(
            (vis.zoom_scale - expected_scale).abs() < 1e-4,
            "zoom {} expected {}",
            vis.zoom_scale,
            expected_scale
        );
    }
}
