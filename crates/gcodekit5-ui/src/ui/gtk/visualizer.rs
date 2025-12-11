use gcodekit5_devicedb::DeviceManager;
use gcodekit5_core::constants as core_constants;
use gcodekit5_visualizer::visualizer::GCodeCommand;
use gcodekit5_visualizer::{Visualizer, Camera3D};
use gcodekit5_designer::stock_removal::{StockMaterial, SimulationResult};
use gcodekit5_designer::stock_removal::visualization::generate_2d_contours;
use gcodekit5_visualizer::visualizer::{StockSimulator3D, generate_surface_mesh};
use crate::ui::gtk::shaders::StockRemovalShaderProgram;
use glam::Vec3;

// Stock removal visualization cache
#[derive(Clone)]
struct ContourLayer {
    z_height: f32,
    color: (f32, f32, f32),
    contours: Vec<Vec<(f32, f32)>>,
}

#[derive(Clone)]
struct StockRemovalVisualization {
    contour_layers: Vec<ContourLayer>,
}
use crate::ui::gtk::nav_cube::NavCube;
use crate::ui::gtk::renderer_3d::{RenderBuffers, generate_vertex_data, generate_grid_data, generate_axis_data, generate_tool_marker_data, generate_bounds_data};
use crate::ui::gtk::shaders::ShaderProgram;
use glow::HasContext;
use gtk4::prelude::*;
use libloading::Library;
use std::sync::Once;

static mut EPOXY_LIB: Option<Library> = None;
static mut GL_LIB: Option<Library> = None;
static EPOXY_INIT: Once = Once::new();

fn load_gl_func(name: &str) -> *const std::ffi::c_void {
    unsafe {
        EPOXY_INIT.call_once(|| {
            let lib = Library::new("libepoxy.so.0")
                .or_else(|_| Library::new("libepoxy.so"))
                .ok();
            EPOXY_LIB = lib;

            let gl_lib = Library::new("libGL.so.1")
                .or_else(|_| Library::new("libGL.so"))
                .ok();
            GL_LIB = gl_lib;
        });

        // Try epoxy first
        if let Some(lib) = (*(&raw const EPOXY_LIB)).as_ref() {
            // Try epoxy_get_proc_addr
            if let Ok(get_proc_addr) = lib.get::<unsafe extern "C" fn(*const i8) -> *const std::ffi::c_void>(b"epoxy_get_proc_addr") {
                let c_name = std::ffi::CString::new(name).unwrap();
                let ptr = get_proc_addr(c_name.as_ptr());
                if !ptr.is_null() {
                    return ptr;
                }
            }
            // Fallback: try to load symbol directly from epoxy
            let c_name = std::ffi::CString::new(name).unwrap();
            if let Ok(sym) = lib.get::<*const std::ffi::c_void>(c_name.as_bytes()) {
                return *sym;
            }
        }

        // Try libGL as fallback
        if let Some(lib) = (*(&raw const GL_LIB)).as_ref() {
            let c_name = std::ffi::CString::new(name).unwrap();
            if let Ok(sym) = lib.get::<*const std::ffi::c_void>(c_name.as_bytes()) {
                return *sym;
            }
        }

        std::ptr::null()
    }
}
use gtk4::prelude::{BoxExt, ButtonExt, CheckButtonExt, WidgetExt};
use gtk4::{
    Box, Button, CheckButton, DrawingArea, EventControllerScroll, EventControllerScrollFlags,
    GestureDrag, Grid, Adjustment, Scrollbar, Label, Orientation, Overlay, Paned,
    Stack, StackSwitcher, GLArea,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

// Phase 4: Render cache for expensive computations
#[derive(Clone)]
struct RenderCache {
    // Cache key: hash of visualizer state
    cache_hash: u64,
    
    // Cached intensity buckets (for intensity mode)
    intensity_buckets: Vec<Vec<(f64, f64, f64, f64)>>,
    
    // Cached cutting bounds (for LOD 3)
    cutting_bounds: Option<(f32, f32, f32, f32, f32, f32)>, // (min_x, max_x, min_y, max_y, min_z, max_z)
    
    // Statistics
    total_lines: usize,
    _rapid_lines: usize,
    cut_lines: usize,
}

struct RendererState {
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
    fn needs_rebuild(&self, new_hash: u64) -> bool {
        self.cache_hash != new_hash
    }
}

pub struct GcodeVisualizer {
    pub widget: Paned,
    stack: Stack,
    drawing_area: DrawingArea,
    gl_area: GLArea,
    visualizer: Rc<RefCell<Visualizer>>,
    camera: Rc<RefCell<Camera3D>>,
    renderer_state: Rc<RefCell<Option<RendererState>>>,
    // Phase 4: Render cache
    render_cache: Rc<RefCell<RenderCache>>,
    // Visibility toggles
    _show_rapid: CheckButton,
    _show_cut: CheckButton,
    _show_grid: CheckButton,
    _show_bounds: CheckButton,
    _show_intensity: CheckButton,
    show_laser: CheckButton,
    show_stock_removal: CheckButton,
    // Stock removal simulation (2D)
    stock_material: Rc<RefCell<Option<StockMaterial>>>,
    simulation_result: Rc<RefCell<Option<SimulationResult>>>,
    simulation_visualization: Rc<RefCell<Option<StockRemovalVisualization>>>,
    simulation_resolution: Rc<RefCell<f32>>,
    simulation_running: Rc<RefCell<bool>>,
    // Stock removal simulation (3D)
    stock_simulator_3d: Rc<RefCell<Option<StockSimulator3D>>>,
    stock_simulation_3d_pending: Rc<RefCell<bool>>,
    // Scrollbars
    hadjustment: Adjustment,
    vadjustment: Adjustment,
    hadjustment_3d: Adjustment,
    vadjustment_3d: Adjustment,
    // Info labels
    bounds_label: Label,
    min_s_label: Label,
    max_s_label: Label,
    avg_s_label: Label,
    _status_label: Label,
    device_manager: Option<Arc<DeviceManager>>,
    current_pos: Rc<RefCell<(f32, f32, f32)>>,
}

impl GcodeVisualizer {
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
            let new_zoom = zoom_x.min(zoom_y).max(0.1).min(50.0);

            vis.zoom_scale = new_zoom;

            // Center the view on the device center
            // The draw function applies: translate(screen_center) -> scale -> translate(offset)
            // So offset needs to be the negative center of the target to bring it to (0,0) before scaling/centering on screen
            vis.x_offset = -center_x;
            vis.y_offset = -center_y;
        }
    }

    pub fn new(device_manager: Option<Arc<DeviceManager>>) -> Self {
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
        let fit_btn = Button::builder()
            .icon_name("zoom-fit-best-symbolic")
            .tooltip_text("Fit to View")
            .build();
        let reset_btn = Button::builder()
            .icon_name("view-restore-symbolic")
            .tooltip_text("Reset View")
            .build();
        let fit_device_btn = Button::builder()
            .icon_name("preferences-desktop-display-symbolic")
            .tooltip_text("Fit to Device Working Area")
            .build();

        view_controls.append(&fit_btn);
        view_controls.append(&reset_btn);
        
        // Only show fit to device button if device manager is available
        if device_manager.is_some() {
            view_controls.append(&fit_device_btn);
        }
        
        sidebar.append(&view_controls);

        // View Mode Switcher
        let mode_label = Label::builder()
            .label("View Mode")
            .css_classes(vec!["heading"])
            .halign(gtk4::Align::Start)
            .margin_top(12)
            .build();
        sidebar.append(&mode_label);

        let stack_switcher = StackSwitcher::builder()
            .halign(gtk4::Align::Fill)
            .build();
        sidebar.append(&stack_switcher);

        // Visibility
        let vis_label = Label::builder()
            .label("Visibility")
            .css_classes(vec!["heading"])
            .halign(gtk4::Align::Start)
            .margin_top(12)
            .build();
        sidebar.append(&vis_label);

        let show_rapid = CheckButton::builder()
            .label("Show Rapid Moves")
            .active(true)
            .build();
        let show_cut = CheckButton::builder()
            .label("Show Cutting Moves")
            .active(true)
            .build();
        let show_grid = CheckButton::builder()
            .label("Show Grid")
            .active(true)
            .build();
        let show_bounds = CheckButton::builder()
            .label("Show Machine Bounds")
            .active(true)
            .build();
        let show_intensity = CheckButton::builder()
            .label("Show Intensity")
            .active(false)
            .build();
        let show_laser = CheckButton::builder()
            .label("Show Laser/Spindle")
            .active(true)
            .build();
        let show_stock_removal = CheckButton::builder()
            .label("Show Stock Removal")
            .active(false)
            .build();
        
        // Stock configuration
        let stock_config_label = Label::builder()
            .label("Stock Dimensions (mm)")
            .css_classes(vec!["heading"])
            .halign(gtk4::Align::Start)
            .margin_top(12)
            .build();
        
        let stock_width_entry = gtk4::Entry::builder()
            .placeholder_text("Width")
            .text("200.0")
            .build();
        let stock_height_entry = gtk4::Entry::builder()
            .placeholder_text("Height")
            .text("200.0")
            .build();
        let stock_thickness_entry = gtk4::Entry::builder()
            .placeholder_text("Thickness")
            .text("10.0")
            .build();
        let stock_tool_radius_entry = gtk4::Entry::builder()
            .placeholder_text("Tool Radius")
            .text("1.5875")
            .build();

        sidebar.append(&show_rapid);
        sidebar.append(&show_cut);
        sidebar.append(&show_grid);
        sidebar.append(&show_bounds);
        sidebar.append(&show_intensity);
        sidebar.append(&show_laser);
        sidebar.append(&show_stock_removal);
        sidebar.append(&stock_config_label);
        sidebar.append(&stock_width_entry);
        sidebar.append(&stock_height_entry);
        sidebar.append(&stock_thickness_entry);
        sidebar.append(&stock_tool_radius_entry);

        // Bounds Info
        let bounds_label = Label::builder()
            .label("Bounds\nX: 0.0 to 0.0\nY: 0.0 to 0.0")
            .css_classes(vec!["caption"])
            .halign(gtk4::Align::Start)
            .margin_top(24)
            .build();
        sidebar.append(&bounds_label);

        // Statistics
        let stats_label = Label::builder()
            .label("Statistics")
            .css_classes(vec!["heading"])
            .halign(gtk4::Align::Start)
            .margin_top(12)
            .build();
        sidebar.append(&stats_label);

        let min_s_label = Label::builder()
            .label("Min S: -")
            .halign(gtk4::Align::Start)
            .build();
        let max_s_label = Label::builder()
            .label("Max S: -")
            .halign(gtk4::Align::Start)
            .build();
        let avg_s_label = Label::builder()
            .label("Avg S: -")
            .halign(gtk4::Align::Start)
            .build();

        sidebar.append(&min_s_label);
        sidebar.append(&max_s_label);
        sidebar.append(&avg_s_label);

        // Wrap sidebar in a scrolled window
        let scrolled_sidebar = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&sidebar)
            .build();

        container.set_start_child(Some(&scrolled_sidebar));

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

        // Stack for 2D/3D
        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack_switcher.set_stack(Some(&stack));

        // 2D Page (Grid with DrawingArea + Scrollbars)
        let grid = Grid::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        grid.attach(&drawing_area, 0, 0, 1, 1);
        grid.attach(&vscrollbar, 1, 0, 1, 1);
        grid.attach(&hscrollbar, 0, 1, 1, 1);
        
        stack.add_titled(&grid, Some("2d"), "2D View");

        // 3D Page
        let gl_area = GLArea::builder()
            .hexpand(true)
            .vexpand(true)
            .build();
        gl_area.set_required_version(3, 3);
        
        // 3D Scrollbars
        let extent = core_constants::WORLD_EXTENT_MM as f64;
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

        let grid_3d = Grid::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        grid_3d.attach(&gl_area, 0, 0, 1, 1);
        grid_3d.attach(&vscrollbar_3d, 1, 0, 1, 1);
        grid_3d.attach(&hscrollbar_3d, 0, 1, 1, 1);
        
        stack.add_titled(&grid_3d, Some("3d"), "3D View");

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
        });
        let stock_material = Rc::new(RefCell::new(initial_stock));
        let tool_radius = Rc::new(RefCell::new(1.5875f32)); // Default 1/16" end mill
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

        // Floating Controls (Bottom Right)
        let floating_box = Box::new(Orientation::Horizontal, 4);
        floating_box.add_css_class("visualizer-osd");
        floating_box.set_halign(gtk4::Align::End);
        floating_box.set_valign(gtk4::Align::End);
        floating_box.set_margin_bottom(20);
        floating_box.set_margin_end(20);

        let float_zoom_out = Button::builder()
            .label("-")
            .tooltip_text("Zoom Out")
            .build();
        let float_fit = Button::builder()
            .label("Fit")
            .tooltip_text("Fit to View")
            .build();
        let float_zoom_in = Button::builder()
            .label("+")
            .tooltip_text("Zoom In")
            .build();

        floating_box.append(&float_zoom_out);
        floating_box.append(&float_fit);
        floating_box.append(&float_zoom_in);

        // Status Panel (Bottom Left)
        let status_box = Box::new(Orientation::Horizontal, 4);
        status_box.add_css_class("visualizer-osd");
        status_box.set_halign(gtk4::Align::Start);
        status_box.set_valign(gtk4::Align::End);
        status_box.set_margin_bottom(20);
        status_box.set_margin_start(20);

        let status_label = Label::builder()
            .label("100%   X: 0.0   Y: 0.0   10.0mm")
            .build();
        status_box.append(&status_label);

        overlay.add_overlay(&floating_box);
        overlay.add_overlay(&status_box);

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
            let (min_x, max_x, min_y, max_y, min_z, max_z) = if let Some(bounds) = vis.get_cutting_bounds() {
                bounds
            } else {
                let (min_x_2d, max_x_2d, min_y_2d, max_y_2d) = vis.get_bounds();
                (min_x_2d, max_x_2d, min_y_2d, max_y_2d, vis.min_z, vis.max_z)
            };
            drop(vis);
            
            let mut cam = cam_fit_3d.borrow_mut();
            cam.fit_to_bounds(
                Vec3::new(min_x, min_y, min_z),
                Vec3::new(max_x, max_y, max_z)
            );
            
            // Update scrollbars
            *is_updating_fit_3d.borrow_mut() = true;
            hadj_fit_3d.set_value(cam.target.x as f64);
            vadj_fit_3d.set_value(cam.target.y as f64);
            *is_updating_fit_3d.borrow_mut() = false;
            
            gl_area_fit_3d.queue_render();
        });

        // Visibility Logic
        let nav_widget = nav_cube.widget.clone();
        let float_box = floating_box.clone();
        let show_intensity_vis = show_intensity.clone();
        
        // Initial state
        nav_widget.set_visible(false); // Start in 2D mode
        
        stack.connect_visible_child_name_notify(move |stack| {
            if stack.visible_child_name().as_deref() == Some("3d") {
                nav_widget.set_visible(true);
                float_box.set_visible(false);
                show_intensity_vis.set_active(false);
                show_intensity_vis.set_sensitive(false);
            } else {
                nav_widget.set_visible(false);
                float_box.set_visible(true);
                show_intensity_vis.set_sensitive(true);
            }
        });

        // Helper to update status
        let update_status_fn = {
            let label = status_label.clone();
            let vis = visualizer.clone();
            move || {
                let v = vis.borrow();
                label.set_text(&format!(
                    "{:.0}%   X: {:.1}   Y: {:.1}   10.0mm",
                    v.zoom_scale * 100.0,
                    -v.x_offset, // Display inverted because visualizer offset compensates for center
                    -v.y_offset
                ));
            }
        };

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
                let extent = core_constants::WORLD_EXTENT_MM as f64;
                
                // Ensure the range includes the current view and content
                let lower_x = (-extent).min(min_x as f64 - margin).min(val_x);
                let upper_x = (extent).max(max_x as f64 + margin).max(val_x + page_size_x);
                let lower_y = (-extent).min(min_y as f64 - margin).min(val_y);
                let upper_y = (extent).max(max_y as f64 + margin).max(val_y + page_size_y);
                
                drop(v);

                *is_updating.borrow_mut() = true;
                hadj.configure(val_x, lower_x, upper_x, page_size_x * 0.1, page_size_x * 0.9, page_size_x);
                vadj.configure(val_y, lower_y, upper_y, page_size_y * 0.1, page_size_y * 0.9, page_size_y);
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
            if *is_updating_h.borrow() { return; }
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
            if *is_updating_v.borrow() { return; }
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

        container.add_tick_callback(|paned, _clock| {
            let width = paned.width();
            let target = (width as f64 * 0.2) as i32;
            if (paned.position() - target).abs() > 2 {
                paned.set_position(target);
            }
            gtk4::glib::ControlFlow::Continue
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

        drawing_area.set_draw_func(move |_, cr, width, height| {
            let vis = vis_draw.borrow();
            let mut cache = render_cache_draw.borrow_mut();
            let pos = *current_pos_draw.borrow();
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
                let (min_x, max_x, min_y, max_y, min_z, max_z) = if let Some(bounds) = vis.get_cutting_bounds() {
                    bounds
                } else {
                    let (min_x_2d, max_x_2d, min_y_2d, max_y_2d) = vis.get_bounds();
                    (min_x_2d, max_x_2d, min_y_2d, max_y_2d, vis.min_z, vis.max_z)
                };
                drop(vis);
                
                let mut cam = camera_fit.borrow_mut();
                cam.fit_to_bounds(
                    Vec3::new(min_x, min_y, min_z),
                    Vec3::new(max_x, max_y, max_z)
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
        show_rapid.connect_toggled(move |_| { da_update.queue_draw(); gl_update.queue_render(); });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        show_cut.connect_toggled(move |_| { da_update.queue_draw(); gl_update.queue_render(); });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        show_grid.connect_toggled(move |_| { da_update.queue_draw(); gl_update.queue_render(); });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        show_bounds.connect_toggled(move |_| { da_update.queue_draw(); gl_update.queue_render(); });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        show_intensity.connect_toggled(move |_| { da_update.queue_draw(); gl_update.queue_render(); });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        show_laser.connect_toggled(move |_| { da_update.queue_draw(); gl_update.queue_render(); });
        let da_update = drawing_area.clone();
        let gl_update = gl_area.clone();
        let visualizer_stock = visualizer.clone();
        let simulation_result_stock = simulation_result.clone();
        let simulation_visualization_stock = simulation_visualization.clone();
        let stock_material_stock = stock_material.clone();
        let tool_radius_stock = tool_radius.clone();
        let simulation_running_flag = simulation_running.clone();
        let stock_simulator_3d_stock = stock_simulator_3d.clone();
        let stock_simulation_3d_pending_toggle = stock_simulation_3d_pending.clone();
        show_stock_removal.connect_toggled(move |checkbox| {
            if checkbox.is_active() {
                // Check if simulation is already running
                if *simulation_running_flag.borrow() {
                    return;
                }
                
                *simulation_running_flag.borrow_mut() = true;
                
                // Run simulation when enabled
                let vis = visualizer_stock.borrow();
                
                if let Some(stock) = stock_material_stock.borrow().as_ref() {
                    use std::sync::{Arc, Mutex};
                    
                    // Run simulation in background thread
                    let stock_clone = stock.clone();
                    let tool_radius_value = *tool_radius_stock.borrow();
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
                            GCodeCommand::Move { from, to, rapid, .. } => {
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
                            GCodeCommand::Arc { from, to, center, clockwise, .. } => {
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
                    
                    std::thread::spawn(move || {
                        use gcodekit5_visualizer::{StockSimulator3D, VoxelGrid};
                        
                        let resolution = 0.25; // 0.25mm voxel resolution (doubled from 0.5mm)
                        let mut grid = VoxelGrid::new(
                            stock_clone.width,
                            stock_clone.height,
                            stock_clone.thickness,
                            resolution
                        );
                        
                        let mut simulator = StockSimulator3D::new(
                            stock_clone.width,
                            stock_clone.height,
                            stock_clone.thickness,
                            resolution,
                            tool_radius_value
                        );
                        simulator.simulate_toolpath(&toolpath_segments_3d);
                        
                        let result_sim = simulator;
                        
                        // Store in Arc
                        *result_arc_clone.lock().unwrap() = Some(result_sim);
                    });
                    
                    // Poll for completion on main thread with timeout limit
                    let result_arc_poll = result_arc.clone();
                    let poll_count = Rc::new(RefCell::new(0u32));
                    let poll_count_clone = poll_count.clone();
                    let sim_running_poll = simulation_running_flag.clone();
                    
                    let pending_flag = stock_simulation_3d_pending_toggle.clone();
                    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                        *poll_count_clone.borrow_mut() += 1;
                        
                        // Stop after 300 iterations (30 seconds)
                        if *poll_count_clone.borrow() > 300 {

                            *sim_running_poll.borrow_mut() = false;
                            return glib::ControlFlow::Break;
                        }
                        
                        if let Ok(mut guard) = result_arc_poll.try_lock() {
                            if let Some(result_simulator) = guard.take() {
                                *result_3d_ref.borrow_mut() = Some(result_simulator);
                                *pending_flag.borrow_mut() = true;
                                
                                *sim_running_poll.borrow_mut() = false;
                                gl_ref.queue_render();
                                
                                return glib::ControlFlow::Break;
                            }
                        }
                        glib::ControlFlow::Continue
                    });
                } else {

                    *stock_simulator_3d_stock.borrow_mut() = None;
                    *simulation_running_flag.borrow_mut() = false;
                }
            } else {
                // Clear simulation when disabled
                *stock_simulator_3d_stock.borrow_mut() = None;
                *simulation_running_flag.borrow_mut() = false;
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
        
        let tool_radius = tool_radius.clone();
        stock_tool_radius_entry.connect_changed(move |entry| {
            if let Ok(radius) = entry.text().parse::<f32>() {
                *tool_radius.borrow_mut() = radius;
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
        let stock_material_3d = stock_material.clone();
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
                eprintln!("GLArea error: {}", err);
                return gtk4::glib::Propagation::Stop;
            }

            let mut state_ref = renderer_state_clone.borrow_mut();
            
            if state_ref.is_none() {
                let gl = unsafe {
                    glow::Context::from_loader_function(|s| {
                        load_gl_func(s)
                    })
                };
                let gl = Rc::new(gl);
                
                let shader_res = ShaderProgram::new(gl.clone());
                let rapid_res = RenderBuffers::new(gl.clone(), glow::LINES);
                let cut_res = RenderBuffers::new(gl.clone(), glow::LINES);
                let grid_res = RenderBuffers::new(gl.clone(), glow::LINES);
                let axis_res = RenderBuffers::new(gl.clone(), glow::LINES);
                let tool_res = RenderBuffers::new(gl.clone(), glow::TRIANGLES);
                let bounds_res = RenderBuffers::new(gl.clone(), glow::LINES);

                match (shader_res, rapid_res, cut_res, grid_res, axis_res, tool_res, bounds_res) {
                    (Ok(shader), Ok(rapid_buffers), Ok(cut_buffers), Ok(mut grid_buffers), Ok(mut axis_buffers), Ok(mut tool_buffers), Ok(bounds_buffers)) => {
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
                        if let Err(e) = shader { eprintln!("Shader init failed: {}", e); }
                        if let Err(e) = rapid { eprintln!("Rapid buffer init failed: {}", e); }
                        if let Err(e) = cut { eprintln!("Cut buffer init failed: {}", e); }
                        if let Err(e) = grid { eprintln!("Grid buffer init failed: {}", e); }
                        if let Err(e) = axis { eprintln!("Axis buffer init failed: {}", e); }
                        if let Err(e) = tool { eprintln!("Tool buffer init failed: {}", e); }
                        if let Err(e) = bounds { eprintln!("Bounds buffer init failed: {}", e); }
                        eprintln!("Failed to initialize 3D renderer");
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
                
                // Update buffers
                // TODO: Only update when dirty
                let vis = visualizer_3d.borrow();
                let (rapid_data, cut_data) = generate_vertex_data(&vis);
                state.rapid_buffers.update(&rapid_data);
                state.cut_buffers.update(&cut_data);
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
                        
                        let bounds_data = generate_bounds_data(min_x, max_x, min_y, max_y, min_z, max_z);
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
                            gl.uniform_matrix_4_f32_slice(Some(&loc), false, &mvp_tool.to_cols_array());
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
                                Err(e) => eprintln!("Failed to create stock removal shader: {}", e),
                            }
                        }

                        // Rebuild mesh when pending or buffers missing
                        if state.stock_removal_buffers.is_none() || *stock_simulation_3d_pending_render.borrow() {
                            let mesh_vertices = generate_surface_mesh(simulator.get_grid());
                            match RenderBuffers::new(gl.clone(), glow::TRIANGLES) {
                                Ok(mut buffers) => {
                                    buffers.update_mesh(&mesh_vertices);
                                    state.stock_removal_buffers = Some(buffers);
                                }
                                Err(e) => eprintln!("Failed to create stock removal mesh buffers: {}", e),
                            }
                            *stock_simulation_3d_pending_render.borrow_mut() = false;
                        }

                        if let (Some(shader), Some(buffers)) = (&state.stock_removal_shader, &state.stock_removal_buffers) {
                            shader.bind();

                            if let Some(loc) = shader.get_uniform_location("uModelViewProjection") {
                                unsafe {
                                    gl.uniform_matrix_4_f32_slice(Some(&loc), false, &mvp.to_cols_array());
                                }
                            }

                            if let Some(loc) = shader.get_uniform_location("uNormalMatrix") {
                                let normal_matrix = glam::Mat3::from_mat4(view).inverse().transpose();
                                unsafe {
                                    gl.uniform_matrix_3_f32_slice(Some(&loc), false, &normal_matrix.to_cols_array());
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
                        println!("No stock simulator available for rendering");
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
                event.modifier_state().contains(gtk4::gdk::ModifierType::SHIFT_MASK)
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
            if *is_updating_h.borrow() { return; }
            let val = adj.value();
            let mut cam = camera_h.borrow_mut();
            cam.target.x = val as f32;
            gl_area_h.queue_render();
        });

        let camera_v = camera.clone();
        let gl_area_v = gl_area.clone();
        let is_updating_v = is_updating_3d.clone();
        vadjustment_3d.connect_value_changed(move |adj| {
            if *is_updating_v.borrow() { return; }
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
            renderer_state,
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
            simulation_visualization: Rc::new(RefCell::new(None)),
            simulation_resolution,
            simulation_running,
            stock_simulator_3d,
            stock_simulation_3d_pending,
            hadjustment,
            vadjustment,
            hadjustment_3d,
            vadjustment_3d,
            bounds_label,
            min_s_label,
            max_s_label,
            avg_s_label,
            _status_label: status_label,
            device_manager,
            current_pos,
        }
    }

    fn setup_interaction<F: Fn() + 'static>(da: &DrawingArea, vis: &Rc<RefCell<Visualizer>>, update_ui: F) {
        // Scroll to Zoom
        let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        let vis_scroll = vis.clone();
        let da_scroll = da.clone();
        let update_scroll = Rc::new(update_ui);

        let update_scroll_clone = update_scroll.clone();
        scroll.connect_scroll(move |_, _, dy| {
            let mut vis = vis_scroll.borrow_mut();
            if dy > 0.0 {
                vis.zoom_out();
            } else {
                vis.zoom_in();
            }
            drop(vis);

            update_scroll_clone();

            da_scroll.queue_draw();
            gtk4::glib::Propagation::Stop
        });
        da.add_controller(scroll);

        // Drag to Pan
        let drag = GestureDrag::new();
        let vis_drag = vis.clone();

        let start_pan = Rc::new(RefCell::new((0.0f32, 0.0f32)));
        let _da_drag = da.clone();
        let update_drag = update_scroll.clone();

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
            drop(vis);

            update_drag();

            da_drag_update.queue_draw();
        });
        da.add_controller(drag);
    }

    fn update_scrollbars(&self) {
        let width = self.drawing_area.width() as f64;
        let height = self.drawing_area.height() as f64;
        
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let v = self.visualizer.borrow();
        let zoom = v.zoom_scale as f64;
        let page_size_x = width / zoom;
        let page_size_y = height / zoom;
        
        let center_x = -v.x_offset as f64;
        let center_y = -v.y_offset as f64;
        
        let val_x = center_x - page_size_x / 2.0;
        let val_y = center_y - page_size_y / 2.0;
        
        let (min_x, max_x, min_y, max_y) = v.get_bounds();
        let margin = 10.0;
        
        let lower_x = (min_x as f64 - margin).min(val_x);
        let upper_x = (max_x as f64 + margin).max(val_x + page_size_x);
        let lower_y = (min_y as f64 - margin).min(val_y);
        let upper_y = (max_y as f64 + margin).max(val_y + page_size_y);
        
        drop(v);

        self.hadjustment.configure(val_x, lower_x, upper_x, page_size_x * 0.1, page_size_x * 0.9, page_size_x);
        self.vadjustment.configure(val_y, lower_y, upper_y, page_size_y * 0.1, page_size_y * 0.9, page_size_y);
    }

    pub fn set_gcode(&self, gcode: &str) {
        let mut vis = self.visualizer.borrow_mut();
        vis.parse_gcode(gcode);

        // Phase 4: Invalidate render cache when G-code changes
        let mut cache = self.render_cache.borrow_mut();
        cache.cache_hash = 0; // Force rebuild
        cache.cutting_bounds = None;
        drop(cache);

        // Update bounds label
        let (min_x, max_x, min_y, max_y) = vis.get_bounds();
        self.bounds_label.set_text(&format!(
            "Bounds\nX: {:.1} to {:.1}\nY: {:.1} to {:.1}",
            min_x, max_x, min_y, max_y
        ));

        // Calculate S statistics
        let mut min_s = f32::MAX;
        let mut max_s = f32::MIN;
        let mut sum_s = 0.0;
        let mut count_s = 0;

        for cmd in vis.commands() {
            let s = match cmd {
                GCodeCommand::Move { intensity: Some(s), .. } => Some(*s),
                GCodeCommand::Arc { intensity: Some(s), .. } => Some(*s),
                _ => None,
            };

            if let Some(val) = s {
                if val < min_s { min_s = val; }
                if val > max_s { max_s = val; }
                sum_s += val;
                count_s += 1;
            }
        }

        if count_s > 0 {
            self.min_s_label.set_text(&format!("Min S: {:.1}", min_s));
            self.max_s_label.set_text(&format!("Max S: {:.1}", max_s));
            self.avg_s_label.set_text(&format!("Avg S: {:.1}", sum_s / count_s as f32));
        } else {
            self.min_s_label.set_text("Min S: N/A");
            self.max_s_label.set_text("Max S: N/A");
            self.avg_s_label.set_text("Avg S: N/A");
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
            let (min_x, max_x, min_y, max_y, min_z, max_z) = if let Some(bounds) = vis.get_cutting_bounds() {
                bounds
            } else {
                let (min_x_2d, max_x_2d, min_y_2d, max_y_2d) = vis.get_bounds();
                (min_x_2d, max_x_2d, min_y_2d, max_y_2d, vis.min_z, vis.max_z)
            };
            
            let (target_x, target_y) = {
                let mut cam = self.camera.borrow_mut();
                cam.fit_to_bounds(
                    Vec3::new(min_x, min_y, min_z),
                    Vec3::new(max_x, max_y, max_z)
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
                use gcodekit5_designer::{create_linear_segment, create_arc_segment};
                
                // Convert GCode commands to toolpath segments
                let mut toolpath_segments = Vec::new();
                for cmd in vis.commands() {
                    match cmd {
                        GCodeCommand::Move { from, to, rapid, .. } => {
                            let segment = create_linear_segment(
                                from.x, from.y, from.z,
                                to.x, to.y, to.z,
                                *rapid,
                                100.0, // Default feed rate
                                3000,  // Default spindle speed
                            );
                            toolpath_segments.push(segment);
                        }
                        GCodeCommand::Arc { from, to, center, clockwise, .. } => {
                            let segment = create_arc_segment(
                                from.x, from.y, from.z,
                                to.x, to.y, to.z,
                                center.x, center.y,
                                *clockwise,
                                100.0, // Default feed rate
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

    fn draw(
        cr: &gtk4::cairo::Context,
        vis: &Visualizer,
        cache: &mut RenderCache,
        width: f64,
        height: f64,
        show_rapid: bool,
        show_cut: bool,
        show_grid: bool,
        show_bounds: bool,
        show_intensity: bool,
        show_laser: bool,
        show_stock_removal: bool,
        simulation_result: &Option<SimulationResult>,
        simulation_visualization: &Option<StockRemovalVisualization>,
        stock_material: &Option<StockMaterial>,
        current_pos: (f32, f32, f32),
        device_manager: &Option<Arc<DeviceManager>>,
    ) {
        // Phase 4: Calculate cache hash from visualizer state
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        vis.commands().len().hash(&mut hasher);
        show_intensity.hash(&mut hasher);
        let new_hash = hasher.finish();
        // Clear background
        if show_intensity {
            cr.set_source_rgb(1.0, 1.0, 1.0); // White background for intensity mode
        } else {
            cr.set_source_rgb(0.15, 0.15, 0.15); // Dark grey background
        }
        cr.paint().expect("Invalid cairo surface state");

        // Determine Max S Value
        let max_s_value = if let Some(manager) = device_manager {
            manager
                .get_active_profile()
                .map(|p| p.max_s_value)
                .unwrap_or(1000.0)
        } else {
            1000.0
        };

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

        // Draw Machine Bounds
        if show_bounds {
            if let Some(manager) = device_manager {
                if let Some(profile) = manager.get_active_profile() {
                    let min_x = profile.x_axis.min;
                    let max_x = profile.x_axis.max;
                    let min_y = profile.y_axis.min;
                    let max_y = profile.y_axis.max;
                    let width = max_x - min_x;
                    let height = max_y - min_y;

                    cr.set_source_rgb(0.0, 0.5, 1.0); // Bright Blue
                                                      // Calc line width in user space to result in 3px on screen
                                                      // Zoom scale is user units per screen pixel? No.
                                                      // Cairo scale(s, -s) means 1 user unit = s pixels.
                                                      // So 1 pixel = 1/s user units.
                                                      // 3 pixels = 3/s user units.
                    cr.set_line_width(3.0 / vis.zoom_scale as f64);

                    cr.rectangle(min_x, min_y, width, height);
                    cr.stroke().unwrap();
                }
            }
        }

        // Draw Origin Axes (Full World Extent)
        let extent = core_constants::WORLD_EXTENT_MM as f64;
        cr.set_line_width(1.0 / vis.zoom_scale as f64); // Thinner line for full axes
        
        // X Axis Red
        cr.set_source_rgb(1.0, 0.0, 0.0); 
        cr.move_to(-extent, 0.0);
        cr.line_to(extent, 0.0);
        cr.stroke().unwrap();

        // Y Axis Green
        cr.set_source_rgb(0.0, 1.0, 0.0); 
        cr.move_to(0.0, -extent);
        cr.line_to(0.0, extent);
        cr.stroke().unwrap();

        // Draw Stock Removal - only draw cached result, don't regenerate
        if show_stock_removal {
            if let Some(cached_viz) = simulation_visualization {
                static DRAW_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                let count = DRAW_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count % 10 == 0 {
                }
                Self::draw_stock_removal_cached(cr, vis, cached_viz);
            }
        }

        // Draw Toolpath - Phase 1, 2 & 3 Optimization: Batched Rendering + Viewport Culling + LOD
        cr.set_line_width(1.5 / vis.zoom_scale as f64);

        // Phase 3: Level of Detail - calculate pixels per mm to determine detail level
        // At low zoom (far out), lines become sub-pixel and we can skip detail
        let pixels_per_mm = vis.zoom_scale;
        
        // LOD thresholds:
        // - LOD 0 (High): zoom >= 1.0 (1:1 or closer) - Draw everything
        // - LOD 1 (Medium): 0.2 <= zoom < 1.0 - Skip every other line
        // - LOD 2 (Low): 0.05 <= zoom < 0.2 - Skip 3 of 4 lines
        // - LOD 3 (Minimal): zoom < 0.05 - Draw bounding box only
        let lod_level = if pixels_per_mm >= 1.0 {
            0 // High detail
        } else if pixels_per_mm >= 0.2 {
            1 // Medium detail
        } else if pixels_per_mm >= 0.05 {
            2 // Low detail
        } else {
            3 // Minimal (bounding box)
        };
        
        // Phase 2: Calculate visible viewport bounds in world coordinates
        // The viewport in screen space is centered at (center_x, center_y) with dimensions (width, height)
        // After transformations: translate(center) -> scale(zoom) -> translate(offset)
        // To find world coordinates visible, we reverse: screen -> unscale -> unoffset
        let half_width_world = (width as f32 / 2.0) / vis.zoom_scale;
        let half_height_world = (height as f32 / 2.0) / vis.zoom_scale;
        
        // Add 5% margin to prevent popping at edges during pan
        let margin = 0.1;
        let margin_x = half_width_world * margin;
        let margin_y = half_height_world * margin;
        
        let view_min_x = -vis.x_offset - half_width_world - margin_x;
        let view_max_x = -vis.x_offset + half_width_world + margin_x;
        let view_min_y = -vis.y_offset - half_height_world - margin_y;
        let view_max_y = -vis.y_offset + half_height_world + margin_y;

        // OPTIMIZATION: Batch rapid moves together (single stroke) + viewport culling + LOD
        if show_rapid && lod_level < 3 {
            cr.new_path();
            cr.set_source_rgba(0.0, 0.8, 1.0, 0.5); // Cyan for rapid
            
            let mut line_counter = 0u32;
            for cmd in vis.commands() {
                if let GCodeCommand::Move { from, to, rapid: true, .. } = cmd {
                    // Phase 2: Viewport culling - skip lines completely outside view
                    let line_min_x = from.x.min(to.x);
                    let line_max_x = from.x.max(to.x);
                    let line_min_y = from.y.min(to.y);
                    let line_max_y = from.y.max(to.y);
                    
                    // Skip if line is entirely outside viewport
                    if line_max_x < view_min_x || line_min_x > view_max_x ||
                       line_max_y < view_min_y || line_min_y > view_max_y {
                        continue;
                    }
                    
                    // Phase 3: LOD - skip lines based on detail level
                    line_counter += 1;
                    match lod_level {
                        1 => if line_counter % 2 != 0 { continue; }, // Skip every other line
                        2 => if line_counter % 4 != 0 { continue; }, // Skip 3 of 4 lines
                        _ => {} // LOD 0: Draw all
                    }
                    
                    cr.move_to(from.x as f64, from.y as f64);
                    cr.line_to(to.x as f64, to.y as f64);
                }
            }
            cr.stroke().unwrap(); // Single stroke for all rapid moves!
        }

        // OPTIMIZATION: Batch cutting moves by intensity + LOD
        if show_cut && lod_level < 3 {
            if show_intensity {
                // Phase 4: Check if we need to rebuild intensity buckets cache
                const INTENSITY_BUCKETS: usize = 20;
                
                if cache.needs_rebuild(new_hash) || cache.intensity_buckets.len() != INTENSITY_BUCKETS {
                    // Rebuild cache
                    cache.cache_hash = new_hash;
                    cache.intensity_buckets = vec![Vec::new(); INTENSITY_BUCKETS];
                    cache.total_lines = 0;
                    cache.cut_lines = 0;
                    
                    // Pre-compute intensity buckets (WITHOUT viewport/LOD - cache is view-independent)
                    for cmd in vis.commands() {
                        cache.total_lines += 1;
                        if let GCodeCommand::Move { from, to, rapid: false, intensity } = cmd {
                            cache.cut_lines += 1;
                            
                            let s = intensity.unwrap_or(0.0);
                            let mut gray = 1.0 - (s as f64 / max_s_value).clamp(0.0, 1.0);
                            if s > 0.0 && gray > 0.95 {
                                gray = 0.95;
                            }
                            
                            let bucket_idx = ((gray * (INTENSITY_BUCKETS as f64 - 1.0)).round() as usize)
                                .min(INTENSITY_BUCKETS - 1);
                            
                            cache.intensity_buckets[bucket_idx].push((
                                from.x as f64,
                                from.y as f64,
                                to.x as f64,
                                to.y as f64
                            ));
                        }
                    }
                }
                
                // Phase 4: Render from cached buckets (apply viewport culling + LOD during render)
                let mut line_counter = 0u32;
                
                // Draw each bucket with a single stroke (apply viewport + LOD filtering)
                for (bucket_idx, lines) in cache.intensity_buckets.iter().enumerate() {
                    if lines.is_empty() {
                        continue;
                    }
                    
                    let gray = (bucket_idx as f64) / ((INTENSITY_BUCKETS - 1) as f64);
                    cr.set_source_rgb(gray, gray, gray);
                    cr.new_path();
                    
                    // line_counter = 0;
                    for (fx, fy, tx, ty) in lines {
                        // Phase 2: Viewport culling (on cached data)
                        let line_min_x = (*fx as f32).min(*tx as f32);
                        let line_max_x = (*fx as f32).max(*tx as f32);
                        let line_min_y = (*fy as f32).min(*ty as f32);
                        let line_max_y = (*fy as f32).max(*ty as f32);
                        
                        if line_max_x < view_min_x || line_min_x > view_max_x ||
                           line_max_y < view_min_y || line_min_y > view_max_y {
                            continue;
                        }
                        
                        // Phase 3: LOD (on cached data)
                        line_counter += 1;
                        match lod_level {
                            1 => if line_counter % 2 != 0 { continue; },
                            2 => if line_counter % 4 != 0 { continue; },
                            _ => {}
                        }
                        
                        cr.move_to(*fx, *fy);
                        cr.line_to(*tx, *ty);
                    }
                    
                    cr.stroke().unwrap(); // One stroke per intensity level!
                }
                
                // Draw arcs separately (usually fewer)
                for cmd in vis.commands() {
                    if let GCodeCommand::Arc { from, to, center, clockwise, intensity } = cmd {
                        // Phase 2: Viewport culling for arcs (check bounding box)
                        let radius = ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();
                        let arc_min_x = center.x - radius;
                        let arc_max_x = center.x + radius;
                        let arc_min_y = center.y - radius;
                        let arc_max_y = center.y + radius;
                        
                        if arc_max_x < view_min_x || arc_min_x > view_max_x ||
                           arc_max_y < view_min_y || arc_min_y > view_max_y {
                            continue;
                        }
                        
                        let s = intensity.unwrap_or(0.0);
                        let mut gray = 1.0 - (s as f64 / max_s_value).clamp(0.0, 1.0);
                        if s > 0.0 && gray > 0.95 {
                            gray = 0.95;
                        }
                        cr.set_source_rgb(gray, gray, gray);

                        let radius = radius as f64;
                        let start_angle = (from.y - center.y).atan2(from.x - center.x) as f64;
                        let end_angle = (to.y - center.y).atan2(to.x - center.x) as f64;

                        if *clockwise {
                            cr.arc_negative(
                                center.x as f64,
                                center.y as f64,
                                radius,
                                start_angle,
                                end_angle,
                            );
                        } else {
                            cr.arc(
                                center.x as f64,
                                center.y as f64,
                                radius,
                                start_angle,
                                end_angle,
                            );
                        }
                        cr.stroke().unwrap();
                    }
                }
            } else {
                // Non-intensity mode: Single color, single stroke! + viewport culling + LOD
                cr.new_path();
                cr.set_source_rgb(1.0, 1.0, 0.0); // Yellow for cut
                
                let mut line_counter = 0u32;
                for cmd in vis.commands() {
                    match cmd {
                        GCodeCommand::Move { from, to, rapid: false, .. } => {
                            // Phase 2: Viewport culling
                            let line_min_x = from.x.min(to.x);
                            let line_max_x = from.x.max(to.x);
                            let line_min_y = from.y.min(to.y);
                            let line_max_y = from.y.max(to.y);
                            
                            if line_max_x < view_min_x || line_min_x > view_max_x ||
                               line_max_y < view_min_y || line_min_y > view_max_y {
                                continue;
                            }
                            
                            // Phase 3: LOD - skip lines based on detail level
                            line_counter += 1;
                            match lod_level {
                                1 => if line_counter % 2 != 0 { continue; },
                                2 => if line_counter % 4 != 0 { continue; },
                                _ => {}
                            }
                            
                            cr.move_to(from.x as f64, from.y as f64);
                            cr.line_to(to.x as f64, to.y as f64);
                        }
                        GCodeCommand::Arc { from, to, center, clockwise, .. } => {
                            // Phase 2: Viewport culling for arcs
                            let radius = ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();
                            let arc_min_x = center.x - radius;
                            let arc_max_x = center.x + radius;
                            let arc_min_y = center.y - radius;
                            let arc_max_y = center.y + radius;
                            
                            if arc_max_x < view_min_x || arc_min_x > view_max_x ||
                               arc_max_y < view_min_y || arc_min_y > view_max_y {
                                continue;
                            }
                            
                            let radius = radius as f64;
                            let start_angle = (from.y - center.y).atan2(from.x - center.x) as f64;
                            let end_angle = (to.y - center.y).atan2(to.x - center.x) as f64;

                            if *clockwise {
                                cr.arc_negative(
                                    center.x as f64,
                                    center.y as f64,
                                    radius,
                                    start_angle,
                                    end_angle,
                                );
                            } else {
                                cr.arc(
                                    center.x as f64,
                                    center.y as f64,
                                    radius,
                                    start_angle,
                                    end_angle,
                                );
                            }
                        }
                        _ => {}
                    }
                }
                cr.stroke().unwrap(); // Single stroke for all cutting moves!
            }
        }
        
        // Phase 3 + 4: LOD Level 3 (Minimal) - Draw bounding box only at extreme zoom out
        if lod_level == 3 && show_cut {
            // Phase 4: Use cached bounds if available
            if cache.cutting_bounds.is_none() && cache.needs_rebuild(new_hash) {
                // Compute and cache cutting bounds
                let mut bounds_min_x = f32::MAX;
                let mut bounds_max_x = f32::MIN;
                let mut bounds_min_y = f32::MAX;
                let mut bounds_max_y = f32::MIN;
                let mut bounds_min_z = f32::MAX;
                let mut bounds_max_z = f32::MIN;
                let mut has_bounds = false;
                
                for cmd in vis.commands() {
                    match cmd {
                        GCodeCommand::Move { from, to, rapid: false, .. } => {
                            bounds_min_x = bounds_min_x.min(from.x).min(to.x);
                            bounds_max_x = bounds_max_x.max(from.x).max(to.x);
                            bounds_min_y = bounds_min_y.min(from.y).min(to.y);
                            bounds_max_y = bounds_max_y.max(from.y).max(to.y);
                            bounds_min_z = bounds_min_z.min(from.z).min(to.z);
                            bounds_max_z = bounds_max_z.max(from.z).max(to.z);
                            has_bounds = true;
                        }
                        GCodeCommand::Arc { from, to: _, center, .. } => {
                            let radius = ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();
                            bounds_min_x = bounds_min_x.min(center.x - radius);
                            bounds_max_x = bounds_max_x.max(center.x + radius);
                            bounds_min_y = bounds_min_y.min(center.y - radius);
                            bounds_max_y = bounds_max_y.max(center.y + radius);
                            bounds_min_z = bounds_min_z.min(from.z);
                            bounds_max_z = bounds_max_z.max(from.z);
                            has_bounds = true;
                        }
                        _ => {}
                    }
                }
                
                if has_bounds {
                    cache.cutting_bounds = Some((bounds_min_x, bounds_max_x, bounds_min_y, bounds_max_y, bounds_min_z, bounds_max_z));
                }
            }
            
            if let Some((bounds_min_x, bounds_max_x, bounds_min_y, bounds_max_y, _, _)) = cache.cutting_bounds {
                // Draw filled rectangle for toolpath bounds (using cached bounds)
                cr.set_source_rgba(1.0, 1.0, 0.0, 0.5); // Semi-transparent yellow
                cr.rectangle(
                    bounds_min_x as f64,
                    bounds_min_y as f64,
                    (bounds_max_x - bounds_min_x) as f64,
                    (bounds_max_y - bounds_min_y) as f64
                );
                cr.fill().unwrap();
                
                // Draw outline
                cr.set_source_rgb(1.0, 1.0, 0.0);
                cr.set_line_width(2.0 / vis.zoom_scale as f64);
                cr.rectangle(
                    bounds_min_x as f64,
                    bounds_min_y as f64,
                    (bounds_max_x - bounds_min_x) as f64,
                    (bounds_max_y - bounds_min_y) as f64
                );
                cr.stroke().unwrap();
            }
        }

        // Draw Laser/Spindle Position
        if show_laser {
            cr.set_source_rgb(1.0, 0.0, 0.0); // Red
            // Draw a circle at current_pos
            // Radius 4.0 pixels (diameter 8)
            // We are in transformed space, so we need to scale the radius
            let radius = 4.0 / vis.zoom_scale as f64;
            cr.arc(current_pos.0 as f64, current_pos.1 as f64, radius, 0.0, 2.0 * std::f64::consts::PI);
            cr.fill().unwrap();
        }

        cr.restore().unwrap();
    }

    fn draw_grid(cr: &gtk4::cairo::Context, vis: &Visualizer) {
        let grid_size = 10.0;
        let grid_color = (0.3, 0.3, 0.3, 0.5);

        // Calculate visible area in world coordinates
        // This is a simplification, ideally we'd project the viewport corners
        let range = core_constants::WORLD_EXTENT_MM as f64; // Draw a large enough grid to match world extent

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

    fn draw_stock_removal_cached(
        cr: &gtk4::cairo::Context,
        vis: &Visualizer,
        cached_viz: &StockRemovalVisualization,
    ) {
        // Just draw the pre-computed contours - NO computation here!
        cr.set_line_width(1.5 / vis.zoom_scale as f64);
        
        for layer in &cached_viz.contour_layers {
            cr.set_source_rgba(
                layer.color.0 as f64,
                layer.color.1 as f64,
                layer.color.2 as f64,
                0.7
            );
            
            for contour in &layer.contours {
                if contour.len() < 2 { continue; }
                
                cr.move_to(contour[0].0 as f64, contour[0].1 as f64);
                for point in &contour[1..] {
                    cr.line_to(point.0 as f64, point.1 as f64);
                }
                cr.stroke().unwrap();
            }
        }
    }
    
    fn generate_stock_visualization(result: &SimulationResult, stock: &StockMaterial) -> StockRemovalVisualization {
        use gcodekit5_designer::stock_removal::visualization::generate_2d_contours;
        
        // Reduce contour count dramatically - only 3 levels
        let num_contours = 3;
        let mut contour_layers = Vec::new();
        
        for i in 0..num_contours {
            let t = i as f32 / (num_contours - 1) as f32;
            let z_height = stock.thickness * t;
            
            // Simple color gradient from yellow (shallow) to blue (deep)
            let r = 1.0 - t;
            let g = 0.7 - t * 0.5;
            let b = t;
            
            let contours = generate_2d_contours(&result.height_map, z_height);
            
            // Convert Vec<Vec<Point2D>> to Vec<Vec<(f32, f32)>>
            let contours: Vec<Vec<(f32, f32)>> = contours.into_iter()
                .map(|contour| contour.into_iter().map(|p| (p.x, p.y)).collect())
                .collect();
            
            contour_layers.push(ContourLayer {
                z_height,
                color: (r, g, b),
                contours,
            });
        }
        
        StockRemovalVisualization {
            contour_layers,
        }
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
        let expected_scale = (available_width / core_constants::DEFAULT_WORK_WIDTH_MM as f32).min(available_height / core_constants::DEFAULT_WORK_HEIGHT_MM as f32);

        assert!((vis.zoom_scale - expected_scale).abs() < 1e-4, "zoom {} expected {}", vis.zoom_scale, expected_scale);
    }
}
