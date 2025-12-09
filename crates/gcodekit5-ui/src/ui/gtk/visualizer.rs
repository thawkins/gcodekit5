use gcodekit5_devicedb::DeviceManager;
use gcodekit5_core::constants as core_constants;
use gcodekit5_visualizer::visualizer::GCodeCommand;
use gcodekit5_visualizer::Visualizer2D;
use gtk4::prelude::*;
use gtk4::prelude::{BoxExt, ButtonExt, CheckButtonExt, WidgetExt};
use gtk4::{
    Box, Button, CheckButton, DrawingArea, EventControllerScroll, EventControllerScrollFlags,
    GestureDrag, Grid, Adjustment, Scrollbar, Label, Orientation, Overlay, Paned,
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
    cutting_bounds: Option<(f32, f32, f32, f32)>, // (min_x, max_x, min_y, max_y)
    
    // Statistics
    total_lines: usize,
    _rapid_lines: usize,
    cut_lines: usize,
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
    drawing_area: DrawingArea,
    visualizer: Rc<RefCell<Visualizer2D>>,
    // Phase 4: Render cache
    render_cache: Rc<RefCell<RenderCache>>,
    // Visibility toggles
    _show_rapid: CheckButton,
    _show_cut: CheckButton,
    _show_grid: CheckButton,
    _show_bounds: CheckButton,
    _show_intensity: CheckButton,
    show_laser: CheckButton,
    // Scrollbars
    hadjustment: Adjustment,
    vadjustment: Adjustment,
    // Info labels
    bounds_label: Label,
    min_s_label: Label,
    max_s_label: Label,
    avg_s_label: Label,
    _status_label: Label,
    device_manager: Option<Arc<DeviceManager>>,
    current_pos: Rc<RefCell<(f32, f32)>>,
}

impl GcodeVisualizer {
    pub fn set_current_position(&self, x: f32, y: f32) {
        *self.current_pos.borrow_mut() = (x, y);
        if self.show_laser.is_active() {
            self.drawing_area.queue_draw();
        }
    }

    fn apply_fit_to_device(
        vis: &mut Visualizer2D,
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

        sidebar.append(&show_rapid);
        sidebar.append(&show_cut);
        sidebar.append(&show_grid);
        sidebar.append(&show_bounds);
        sidebar.append(&show_intensity);
        sidebar.append(&show_laser);

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

        container.set_start_child(Some(&sidebar));

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

        // Grid for DrawingArea + Scrollbars
        let grid = Grid::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        grid.attach(&drawing_area, 0, 0, 1, 1);
        grid.attach(&vscrollbar, 1, 0, 1, 1);
        grid.attach(&hscrollbar, 0, 1, 1, 1);

        // Initialize Visualizer logic
        let visualizer = Rc::new(RefCell::new(Visualizer2D::new()));
        let current_pos = Rc::new(RefCell::new((0.0f32, 0.0f32)));

        // Overlay for floating controls
        let overlay = Overlay::new();
        overlay.set_child(Some(&grid));

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
                
                let lower_x = (min_x as f64 - margin).min(val_x);
                let upper_x = (max_x as f64 + margin).max(val_x + page_size_x);
                let lower_y = (min_y as f64 - margin).min(val_y);
                let upper_y = (max_y as f64 + margin).max(val_y + page_size_y);
                
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
                pos,
                &device_manager_draw,
            );
        });

        // Connect Controls
        let vis_fit = visualizer.clone();
        let da_fit = drawing_area.clone();
        let update_status = update_ui.clone();
        fit_btn.connect_clicked(move |_| {
            let width = da_fit.width() as f32;
            let height = da_fit.height() as f32;
            vis_fit.borrow_mut().fit_to_view(width, height);
            update_status();
            da_fit.queue_draw();
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
        show_rapid.connect_toggled(move |_| da_update.queue_draw());
        let da_update = drawing_area.clone();
        show_cut.connect_toggled(move |_| da_update.queue_draw());
        let da_update = drawing_area.clone();
        show_grid.connect_toggled(move |_| da_update.queue_draw());
        let da_update = drawing_area.clone();
        show_bounds.connect_toggled(move |_| da_update.queue_draw());
        let da_update = drawing_area.clone();
        show_intensity.connect_toggled(move |_| da_update.queue_draw());
        let da_update = drawing_area.clone();
        show_laser.connect_toggled(move |_| da_update.queue_draw());

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

        Self {
            widget: container,
            drawing_area,
            visualizer,
            render_cache: Rc::new(RefCell::new(RenderCache::default())),
            _show_rapid: show_rapid,
            _show_cut: show_cut,
            _show_grid: show_grid,
            _show_bounds: show_bounds,
            _show_intensity: show_intensity,
            show_laser,
            hadjustment,
            vadjustment,
            bounds_label,
            min_s_label,
            max_s_label,
            avg_s_label,
            _status_label: status_label,
            device_manager,
            current_pos,
        }
    }

    fn setup_interaction<F: Fn() + 'static>(da: &DrawingArea, vis: &Rc<RefCell<Visualizer2D>>, update_ui: F) {
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

        drop(vis);
        self.update_scrollbars();
        self.drawing_area.queue_draw();
    }

    fn draw(
        cr: &gtk4::cairo::Context,
        vis: &Visualizer2D,
        cache: &mut RenderCache,
        width: f64,
        height: f64,
        show_rapid: bool,
        show_cut: bool,
        show_grid: bool,
        show_bounds: bool,
        show_intensity: bool,
        show_laser: bool,
        current_pos: (f32, f32),
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
                let mut has_bounds = false;
                
                for cmd in vis.commands() {
                    match cmd {
                        GCodeCommand::Move { from, to, rapid: false, .. } => {
                            bounds_min_x = bounds_min_x.min(from.x).min(to.x);
                            bounds_max_x = bounds_max_x.max(from.x).max(to.x);
                            bounds_min_y = bounds_min_y.min(from.y).min(to.y);
                            bounds_max_y = bounds_max_y.max(from.y).max(to.y);
                            has_bounds = true;
                        }
                        GCodeCommand::Arc { from, to: _, center, .. } => {
                            let radius = ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();
                            bounds_min_x = bounds_min_x.min(center.x - radius);
                            bounds_max_x = bounds_max_x.max(center.x + radius);
                            bounds_min_y = bounds_min_y.min(center.y - radius);
                            bounds_max_y = bounds_max_y.max(center.y + radius);
                            has_bounds = true;
                        }
                        _ => {}
                    }
                }
                
                if has_bounds {
                    cache.cutting_bounds = Some((bounds_min_x, bounds_max_x, bounds_min_y, bounds_max_y));
                }
            }
            
            if let Some((bounds_min_x, bounds_max_x, bounds_min_y, bounds_max_y)) = cache.cutting_bounds {
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

    fn draw_grid(cr: &gtk4::cairo::Context, vis: &Visualizer2D) {
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
}

#[cfg(test)]
mod tests_visualizer {
    use super::*;
    use gcodekit5_visualizer::visualizer::Visualizer2D;

    #[test]
    fn test_apply_fit_to_device_with_no_profile_uses_default_bbox() {
        let mut vis = Visualizer2D::new();
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
