//! Designer View - Main designer UI container
//!
//! This module provides the DesignerView which contains the main designer interface
//! including toolbox, canvas, properties panel, and layers panel.

use crate::t;
use crate::ui::gtk::designer_canvas::DesignerCanvas;
use crate::ui::gtk::designer_layers::LayersPanel;
use crate::ui::gtk::designer_properties::PropertiesPanel;
use crate::ui::gtk::designer_toolbox::{DesignerTool, DesignerToolbox};
use crate::ui::gtk::osd_format::format_zoom_center_cursor;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::{DesignerShape, Shape};
use gcodekit5_designer::serialization::DesignFile;
use gcodekit5_designer::stock_removal::StockMaterial;
use gcodekit5_devicedb::DeviceManager;
use gcodekit5_settings::controller::SettingsController;
use gtk4::gdk::{Key, ModifierType};
use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box, EventControllerKey, FileChooserAction, FileChooserNative, GestureClick, Grid,
    Label, Orientation, Overlay, Paned, Popover, ResponseType, Scrollbar,
};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tracing::error;

#[allow(clippy::type_complexity)]
pub struct DesignerView {
    pub widget: Box,
    pub(crate) canvas: Rc<DesignerCanvas>,
    pub(crate) toolbox: Rc<DesignerToolbox>,
    pub(crate) _properties: Rc<PropertiesPanel>,
    pub(crate) layers: Rc<LayersPanel>,
    pub(crate) status_label: Label,
    pub(crate) _coord_label: Label,
    pub(crate) current_file: Rc<RefCell<Option<PathBuf>>>,
    pub(crate) on_gcode_generated: Rc<RefCell<Option<std::boxed::Box<dyn Fn(String)>>>>,
    pub(crate) settings_persistence: Option<Rc<RefCell<gcodekit5_settings::SettingsPersistence>>>,
}

impl DesignerView {
    pub fn new(
        device_manager: Option<Arc<DeviceManager>>,
        settings_controller: Rc<SettingsController>,
        status_bar: Option<crate::ui::gtk::status_bar::StatusBar>,
    ) -> Rc<Self> {
        let container = Box::new(Orientation::Vertical, 0);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Create designer state
        let state = Rc::new(RefCell::new(DesignerState::new()));

        // Create main horizontal layout (toolbox + canvas + properties)
        let main_box = Box::new(Orientation::Horizontal, 0);
        main_box.set_hexpand(true);
        main_box.set_vexpand(true);

        // Create toolbox + left sidebar container (toolbox + view/legend)
        let toolbox = DesignerToolbox::new(state.clone(), settings_controller.clone());
        let left_sidebar = Box::new(Orientation::Vertical, 0);
        left_sidebar.set_vexpand(true);
        left_sidebar.set_hexpand(false);
        left_sidebar.set_halign(gtk4::Align::Fill);
        left_sidebar.append(&toolbox.widget);

        // Keep left sidebar at ~20% of the main window width (set on first map).
        let last_left_width = Rc::new(std::cell::Cell::new(-1));
        {
            let left_sidebar = left_sidebar.clone();
            let last_left_width = last_left_width.clone();
            let container_width = container.clone();
            container.connect_map(move |_| {
                let left_sidebar = left_sidebar.clone();
                let last_left_width = last_left_width.clone();
                let container_width = container_width.clone();
                gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                    let w = container_width.width();
                    if w <= 0 {
                        return gtk4::glib::ControlFlow::Continue;
                    }
                    let target = ((w as f64) * 0.20).round() as i32;
                    let target = target.max(160);
                    if last_left_width.get() != target {
                        last_left_width.set(target);
                        left_sidebar.set_width_request(target);
                    }
                    gtk4::glib::ControlFlow::Break
                });
            });
        }

        // Paned layout: left sidebar is resizable
        let left_paned = Paned::new(Orientation::Horizontal);
        left_paned.set_start_child(Some(&left_sidebar));
        left_paned.set_resize_start_child(true);
        left_paned.set_shrink_start_child(false);

        // Create canvas
        let canvas = DesignerCanvas::new(
            state.clone(),
            Some(toolbox.clone()),
            device_manager.clone(),
            status_bar.clone(),
            Some(settings_controller.clone()),
        );

        // Create Grid for Canvas + Scrollbars
        let canvas_grid = Grid::new();
        canvas_grid.set_hexpand(true);
        canvas_grid.set_vexpand(true);

        canvas.widget.set_hexpand(true);
        canvas.widget.set_vexpand(true);

        // Overlay for floating controls
        let overlay = Overlay::new();
        overlay.set_child(Some(&canvas.widget));

        // Floating Controls (Bottom Right)
        let (
            floating_box,
            float_zoom_in,
            float_zoom_out,
            float_fit,
            float_reset,
            float_fit_device,
            scrollbars_btn,
        ) = Self::create_floating_controls(device_manager.is_some());

        // Empty state (shown when no shapes)
        let (
            empty_box,
            empty_new_btn,
            empty_open_btn,
            empty_import_svg_btn,
            empty_import_dxf_btn,
            empty_import_stl_btn,
        ) = Self::create_empty_state(&settings_controller);

        overlay.add_overlay(&empty_box);
        overlay.add_overlay(&floating_box);

        // Status Panel (Bottom Left)
        let (status_box, status_label_osd, units_badge) = Self::create_status_panel();
        overlay.add_overlay(&status_box);

        // Attach Overlay to Grid (instead of direct canvas)
        canvas_grid.attach(&overlay, 0, 0, 1, 1);

        // Scrollbars
        // Range: use shared world extent (±WORLD_EXTENT_MM)
        let world_extent = gcodekit5_core::constants::WORLD_EXTENT_MM;
        let h_adjustment = Adjustment::new(0.0, -world_extent, world_extent, 10.0, 100.0, 100.0);
        let v_adjustment = Adjustment::new(0.0, -world_extent, world_extent, 10.0, 100.0, 100.0);

        let h_scrollbar = Scrollbar::new(Orientation::Horizontal, Some(&h_adjustment));
        let v_scrollbar = Scrollbar::new(Orientation::Vertical, Some(&v_adjustment));

        // Default hidden (toggleable) to maximize canvas space
        h_scrollbar.set_visible(false);
        v_scrollbar.set_visible(false);

        canvas_grid.attach(&v_scrollbar, 1, 0, 1, 1);
        canvas_grid.attach(&h_scrollbar, 0, 1, 1, 1);

        // Set center area placeholder to be replaced later
        // We'll update this after creating the center paned
        left_paned.set_resize_end_child(true);
        left_paned.set_shrink_end_child(false);
        left_paned.set_hexpand(true);
        left_paned.set_vexpand(true);
        left_paned.set_position(180); // Initial position for left sidebar

        main_box.append(&left_paned);

        // Connect scrollbars to canvas pan
        let canvas_h = canvas.clone();
        h_adjustment.connect_value_changed(move |adj| {
            let val = adj.value();
            let mut state = canvas_h.state.borrow_mut();
            // Pan is opposite to scroll value usually
            let current_pan_y = state.canvas.pan_y();
            state.canvas.set_pan(-val, current_pan_y);
            drop(state);
            canvas_h.widget.queue_draw();
        });

        let canvas_v = canvas.clone();
        v_adjustment.connect_value_changed(move |adj| {
            let val = adj.value();
            let mut state = canvas_v.state.borrow_mut();
            // Positive scroll value (down) moves content up (positive pan_y)
            let current_pan_x = state.canvas.pan_x();
            state.canvas.set_pan(current_pan_x, val);
            drop(state);
            canvas_v.widget.queue_draw();
        });

        // Pass adjustments to canvas
        canvas.set_adjustments(h_adjustment.clone(), v_adjustment.clone());

        // Connect Floating Zoom Buttons
        let canvas_zoom = canvas.clone();
        float_zoom_in.connect_clicked(move |_| {
            canvas_zoom.zoom_in();
        });

        let canvas_zoom = canvas.clone();
        float_zoom_out.connect_clicked(move |_| {
            canvas_zoom.zoom_out();
        });

        let canvas_zoom = canvas.clone();
        float_fit.connect_clicked(move |_| {
            canvas_zoom.zoom_fit();
        });

        let canvas_reset = canvas.clone();
        float_reset.connect_clicked(move |_| {
            canvas_reset.reset_view();
        });

        let canvas_fitdev = canvas.clone();
        float_fit_device.connect_clicked(move |_| {
            canvas_fitdev.fit_to_device_area();
            canvas_fitdev.widget.queue_draw();
        });

        // Scrollbars toggle
        let show_scrollbars = Rc::new(std::cell::Cell::new(false));
        let show_scrollbars_btn = show_scrollbars.clone();
        let hsb = h_scrollbar.clone();
        let vsb = v_scrollbar.clone();
        scrollbars_btn.connect_clicked(move |_| {
            let next = !show_scrollbars_btn.get();
            show_scrollbars_btn.set(next);
            hsb.set_visible(next);
            vsb.set_visible(next);
        });

        // Auto-fit when designer is mapped (visible) — schedule after layout like Visualizer
        let canvas_map = canvas.clone();
        container.connect_map(move |_| {
            let canvas_run = canvas_map.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                // Ensure viewport knows the correct size before fitting
                let width = canvas_run.widget.width() as f64;
                let height = canvas_run.widget.height() as f64;
                if width > 0.0 && height > 0.0 {
                    if let Ok(mut state) = canvas_run.state.try_borrow_mut() {
                        state.canvas.set_canvas_size(width, height);
                    }
                }

                // Always fit to device on initialization as per user request
                canvas_run.fit_to_device_area();
                canvas_run.widget.queue_draw();
                gtk4::glib::ControlFlow::Break
            });
        });

        // Create right sidebar with properties and layers
        let right_sidebar = Box::new(Orientation::Vertical, 5);
        right_sidebar.set_hexpand(false);
        right_sidebar.set_halign(gtk4::Align::End);

        // Keep right sidebar at ~20% of the main window width (set on first map).
        let last_right_width = Rc::new(std::cell::Cell::new(-1));
        {
            let right_sidebar = right_sidebar.clone();
            let last_right_width = last_right_width.clone();
            let container_width = container.clone();
            container.connect_map(move |_| {
                let right_sidebar = right_sidebar.clone();
                let last_right_width = last_right_width.clone();
                let container_width = container_width.clone();
                gtk4::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                    let w = container_width.width();
                    if w <= 0 {
                        return gtk4::glib::ControlFlow::Continue;
                    }
                    let target = ((w as f64) * 0.20).round() as i32;
                    let target = target.clamp(240, 520);
                    if last_right_width.get() != target {
                        last_right_width.set(target);
                        right_sidebar.set_width_request(target);
                    }
                    gtk4::glib::ControlFlow::Break
                });
            });
        }

        // Create properties panel
        let properties = PropertiesPanel::new(
            state.clone(),
            settings_controller.persistence.clone(),
            canvas.preview_shapes.clone(),
        );
        properties.widget.set_vexpand(true);
        properties.widget.set_valign(gtk4::Align::Fill);

        // Set up redraw callback for properties
        let canvas_redraw = canvas.clone();
        properties.set_redraw_callback(move || {
            let show_toolpaths = canvas_redraw.state.borrow().show_toolpaths;
            if show_toolpaths {
                canvas_redraw.generate_preview_toolpaths();
            }
            canvas_redraw.widget.queue_draw();
        });

        // Inspector header + hide button (matches DeviceConsole / Visualizer sidebar UX)
        let props_hidden = Rc::new(Cell::new(false));

        let inspector_header = Box::new(Orientation::Horizontal, 6);
        inspector_header.set_margin_start(6);
        inspector_header.set_margin_end(6);
        inspector_header.set_margin_top(6);

        let inspector_label = Label::builder()
            .label(t!("Inspector"))
            .css_classes(vec!["heading"])
            .halign(gtk4::Align::Start)
            .build();
        inspector_label.set_hexpand(true);
        inspector_header.append(&inspector_label);

        let props_hide_btn = gtk4::Button::builder()
            .tooltip_text(t!("Hide Properties"))
            .build();
        props_hide_btn
            .update_property(&[gtk4::accessible::Property::Label(&t!("Hide Properties"))]);
        {
            let child = Box::new(Orientation::Horizontal, 6);
            child.append(&gtk4::Image::from_icon_name("view-conceal-symbolic"));
            child.append(&Label::new(Some(&t!("Hide"))));
            props_hide_btn.set_child(Some(&child));
        }
        inspector_header.append(&props_hide_btn);

        right_sidebar.append(&inspector_header);

        // Create layers panel
        let layers = Rc::new(LayersPanel::new(state.clone(), canvas.widget.clone()));
        layers.widget.set_vexpand(true);
        layers.widget.set_valign(gtk4::Align::Fill);

        // Draggable divider between Properties and Layers
        let inspector_paned = Paned::new(Orientation::Vertical);
        inspector_paned.set_vexpand(true);
        inspector_paned.set_start_child(Some(&properties.widget));
        inspector_paned.set_end_child(Some(&layers.widget));
        inspector_paned.set_resize_start_child(true);
        inspector_paned.set_resize_end_child(true);
        inspector_paned.set_shrink_start_child(false);
        inspector_paned.set_shrink_end_child(false);
        inspector_paned.set_position(520);

        right_sidebar.append(&inspector_paned);

        // Floating unhide button (top-right of canvas)
        let props_show_btn = gtk4::Button::builder()
            .tooltip_text(t!("Unhide Properties"))
            .build();
        props_show_btn
            .update_property(&[gtk4::accessible::Property::Label(&t!("Unhide Properties"))]);
        {
            let child = Box::new(Orientation::Horizontal, 6);
            child.append(&gtk4::Image::from_icon_name("view-reveal-symbolic"));
            child.append(&Label::new(Some(&t!("Unhide"))));
            props_show_btn.set_child(Some(&child));
        }

        let props_show_panel = Box::new(Orientation::Horizontal, 0);
        props_show_panel.add_css_class("visualizer-osd");
        props_show_panel.add_css_class("osd-controls");
        props_show_panel.set_halign(gtk4::Align::End);
        props_show_panel.set_valign(gtk4::Align::Start);
        props_show_panel.set_margin_end(12);
        props_show_panel.set_margin_top(12);
        props_show_panel.append(&props_show_btn);
        props_show_panel.set_visible(false);
        overlay.add_overlay(&props_show_panel);

        {
            let right_sidebar = right_sidebar.clone();
            let props_hidden = props_hidden.clone();
            let show_panel = props_show_panel.clone();
            let hide_btn = props_hide_btn.clone();
            props_hide_btn.connect_clicked(move |_| {
                if props_hidden.get() {
                    return;
                }
                right_sidebar.set_visible(false);
                hide_btn.set_sensitive(false);
                props_hidden.set(true);
                show_panel.set_visible(true);
            });
        }

        {
            let right_sidebar = right_sidebar.clone();
            let props_hidden = props_hidden.clone();
            let show_panel = props_show_panel.clone();
            let hide_btn = props_hide_btn.clone();
            props_show_btn.connect_clicked(move |_| {
                if !props_hidden.get() {
                    return;
                }
                right_sidebar.set_visible(true);
                hide_btn.set_sensitive(true);
                props_hidden.set(false);
                show_panel.set_visible(false);
            });
        }

        // Legend moved to left sidebar

        // Give canvas references to panels
        canvas.set_properties_panel(properties.clone());
        canvas.set_layers_panel(layers.clone());

        // Paned layout: right sidebar is resizable from the center paned
        let center_paned = Paned::new(Orientation::Horizontal);
        center_paned.set_start_child(Some(&canvas_grid));
        center_paned.set_end_child(Some(&right_sidebar));
        center_paned.set_resize_start_child(true);
        center_paned.set_resize_end_child(true);
        center_paned.set_shrink_start_child(false);
        center_paned.set_shrink_end_child(false);
        center_paned.set_hexpand(true);
        center_paned.set_vexpand(true);
        center_paned.set_position(600); // Will be adjusted on map

        // Now set the center paned as the end child of the left paned
        left_paned.set_end_child(Some(&center_paned));

        // Auto-size the center paned position when window is mapped
        let center_paned_size = center_paned.clone();
        let right_sidebar_width = right_sidebar.clone();
        container.connect_map(move |_cont| {
            let center_paned = center_paned_size.clone();
            let right_sidebar = right_sidebar_width.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                let total_width = center_paned.width();
                if total_width > 0 {
                    // Get the right sidebar preferred width (should be set by now)
                    let right_w = right_sidebar.width_request();
                    let right_w = if right_w > 0 { right_w } else { 300 };
                    // Position center_paned divider to give most space to canvas
                    let canvas_width = total_width - right_w;
                    if canvas_width > 100 {
                        center_paned.set_position(canvas_width);
                    }
                }
                gtk4::glib::ControlFlow::Break
            });
        });

        container.append(&main_box);

        // Hidden labels retained for internal status messages (status bar removed)
        let status_label = Label::new(None);
        let coord_label = Label::new(None);

        // View controls (moved to helper function)
        let view_controls_expander =
            Self::create_view_controls_expander(&state, &canvas, &settings_controller);
        left_sidebar.append(&view_controls_expander);

        // Start status OSD update loop
        Self::start_status_update_loop(
            status_label_osd,
            units_badge,
            empty_box.clone(),
            canvas.clone(),
            settings_controller.clone(),
        );

        let current_file = Rc::new(RefCell::new(None));
        #[allow(clippy::type_complexity)]
        let on_gcode_generated: Rc<RefCell<Option<std::boxed::Box<dyn Fn(String)>>>> =
            Rc::new(RefCell::new(None));

        // Connect Generate G-Code button
        let canvas_gen = canvas.clone();
        let on_gen = on_gcode_generated.clone();
        let status_label_gen = status_label.clone();

        toolbox.connect_generate_clicked(move || {
            let mut state = canvas_gen.state.borrow_mut();

            // Copy settings to avoid borrow issues
            let feed_rate = state.tool_settings.feed_rate;
            let spindle_speed = state.tool_settings.spindle_speed;
            let tool_diameter = state.tool_settings.tool_diameter;
            let cut_depth = state.tool_settings.cut_depth;
            let start_depth = state.tool_settings.start_depth;

            // Update toolpath generator settings from state
            state.toolpath_generator.set_feed_rate(feed_rate);
            state.toolpath_generator.set_spindle_speed(spindle_speed);
            state.toolpath_generator.set_tool_diameter(tool_diameter);
            state.toolpath_generator.set_cut_depth(cut_depth);
            state.toolpath_generator.set_start_depth(start_depth);
            state.toolpath_generator.set_step_in(tool_diameter * 0.4); // Default stepover

            let gcode = state.generate_gcode();
            drop(state);

            status_label_gen.set_text(&t!("G-Code generated"));

            if let Some(callback) = on_gen.borrow().as_ref() {
                callback(gcode);
            }
        });

        // Connect fast shape gallery to insert shapes
        let canvas_shape = canvas.clone();
        let layers_shape = layers.clone();
        toolbox
            .fast_shape_gallery()
            .connect_shape_selected(move |shape| {
                let mut state = canvas_shape.state.borrow_mut();
                state.add_shape_with_undo(shape);
                drop(state);

                // Refresh layers panel
                layers_shape.refresh(&canvas_shape.state);
                canvas_shape.widget.queue_draw();
            });

        let view = Rc::new(Self {
            widget: container,
            canvas: canvas.clone(),
            toolbox: toolbox.clone(),
            _properties: properties.clone(),
            layers: layers.clone(),
            status_label,
            _coord_label: coord_label,
            current_file,
            on_gcode_generated,
            settings_persistence: Some(settings_controller.persistence.clone()),
        });

        // Empty state actions
        {
            let v = view.clone();
            empty_new_btn.connect_clicked(move |_| v.new_file());
        }
        {
            let v = view.clone();
            empty_open_btn.connect_clicked(move |_| v.open_file());
        }
        {
            let v = view.clone();
            empty_import_svg_btn.connect_clicked(move |_| v.import_svg_file());
        }
        {
            let v = view.clone();
            empty_import_dxf_btn.connect_clicked(move |_| v.import_dxf_file());
        }
        {
            let v = view.clone();
            empty_import_stl_btn.connect_clicked(move |_| v.import_stl_file());
        }

        // Add settings change listener for STL import feature
        {
            let empty_import_stl_btn = empty_import_stl_btn.clone();
            settings_controller.on_setting_changed(move |key, value| {
                if key != "enable_stl_import" {
                    return;
                }
                let enabled = value == "true";
                empty_import_stl_btn.set_visible(enabled);
            });
        }

        // Update properties panel and toolbox when selection changes
        let props_update = properties.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            // Check if we need to update properties (when canvas is redrawn or selection changes)
            props_update.update_from_selection();

            gtk4::glib::ControlFlow::Continue
        });

        // Setup keyboard shortcuts
        Self::setup_keyboard_shortcuts(&canvas);

        // Grab focus initially
        canvas.widget.grab_focus();

        view
    }

    pub fn current_tool(&self) -> DesignerTool {
        self.toolbox.current_tool()
    }

    pub fn set_tool(&self, tool: DesignerTool) {
        self.toolbox.set_tool(tool);
    }

    pub fn set_status(&self, message: &str) {
        self.status_label.set_text(message);
    }

    pub fn set_on_gcode_generated<F: Fn(String) + 'static>(&self, f: F) {
        *self.on_gcode_generated.borrow_mut() = Some(std::boxed::Box::new(f));
    }

    pub fn fit_to_device(&self) {
        self.canvas.fit_to_device_area();
        self.canvas.widget.queue_draw();
    }

    pub fn undo(&self) {
        self.canvas.undo();
    }

    pub fn redo(&self) {
        self.canvas.redo();
    }

    pub fn cut(&self) {
        self.canvas.copy_selected();
        self.canvas.delete_selected();
    }

    pub fn copy(&self) {
        self.canvas.copy_selected();
    }

    pub fn paste(&self) {
        self.canvas.paste();
    }

    pub fn delete(&self) {
        self.canvas.delete_selected();
    }

    /// Queue a redraw of the designer canvas
    pub fn queue_draw(&self) {
        self.canvas.widget.queue_draw();
    }
}

mod file_ops;
mod ui_builders;
