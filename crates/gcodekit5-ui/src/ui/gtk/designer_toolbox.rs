use crate::t;
use crate::ui::gtk::fast_shape_gallery::FastShapeGallery;
use gcodekit5_core::units::MeasurementSystem;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_settings::controller::SettingsController;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, Dialog, Entry, Frame, Grid, Image, Label, Orientation, PolicyType,
    ResponseType, ScrolledWindow,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// Dialog widgets should not be dropped; they are owned by closures attached to buttons.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignerTool {
    Select = 0,
    Rectangle = 1,
    Circle = 2,
    Line = 3,
    Ellipse = 4,
    Polyline = 5,
    Text = 6,
    Pan = 7,
    Triangle = 8,
    Polygon = 9,
    Gear = 10,
    Sprocket = 11,
}

impl DesignerTool {
    pub fn name(&self) -> String {
        match self {
            DesignerTool::Select => t!("Select"),
            DesignerTool::Rectangle => t!("Rectangle"),
            DesignerTool::Circle => t!("Circle"),
            DesignerTool::Line => t!("Line"),
            DesignerTool::Ellipse => t!("Ellipse"),
            DesignerTool::Polyline => t!("Polyline"),
            DesignerTool::Text => t!("Text"),
            DesignerTool::Pan => t!("Pan"),
            DesignerTool::Triangle => t!("Triangle"),
            DesignerTool::Polygon => t!("Polygon"),
            DesignerTool::Gear => t!("Gear"),
            DesignerTool::Sprocket => t!("Sprocket"),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DesignerTool::Select => "select.svg",
            DesignerTool::Rectangle => "rectangle.svg",
            DesignerTool::Circle => "circle.svg",
            DesignerTool::Line => "line.svg",
            DesignerTool::Ellipse => "ellipse.svg",
            DesignerTool::Polyline => "polyline.svg",
            DesignerTool::Text => "text.svg",
            DesignerTool::Pan => "grab.svg",
            DesignerTool::Triangle => "media-playback-start-symbolic",
            DesignerTool::Polygon => "emblem-shared-symbolic",
            DesignerTool::Gear => "system-run-symbolic",
            DesignerTool::Sprocket => "emblem-system-symbolic",
        }
    }

    pub fn tooltip(&self) -> String {
        match self {
            DesignerTool::Select => t!("Select (S)"),
            DesignerTool::Rectangle => t!("Rectangle (R)"),
            DesignerTool::Circle => t!("Circle (C)"),
            DesignerTool::Line => t!("Line (L)"),
            DesignerTool::Ellipse => t!("Ellipse (E)"),
            DesignerTool::Polyline => t!("Polyline (P)"),
            DesignerTool::Text => t!("Text (T)"),
            DesignerTool::Pan => t!("Pan (Space)"),
            DesignerTool::Triangle => t!("Triangle"),
            DesignerTool::Polygon => t!("Polygon"),
            DesignerTool::Gear => t!("Gear"),
            DesignerTool::Sprocket => t!("Sprocket"),
        }
    }
}

pub struct DesignerToolbox {
    pub widget: Box,
    current_tool: Rc<RefCell<DesignerTool>>,
    active_tool_label: Label,
    buttons: Vec<Button>,
    tools: Vec<DesignerTool>,
    generate_btn: Button,
    fast_shape_gallery: Rc<FastShapeGallery>,
    _state: Rc<RefCell<DesignerState>>,
    _settings_controller: Rc<SettingsController>,
    _current_units: Arc<Mutex<MeasurementSystem>>,
    /// Callbacks to refresh tool/stock setting UI widgets from state
    refresh_callbacks: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl DesignerToolbox {
    pub fn new(
        state: Rc<RefCell<DesignerState>>,
        settings_controller: Rc<SettingsController>,
    ) -> Rc<Self> {
        #[derive(Clone, Copy)]
        enum UnitsKind {
            Length,
            FeedRate,
            Rpm,
        }

        let main_container = Box::new(Orientation::Vertical, 0);
        main_container.set_width_request(160); // Increased width for 3 columns
        main_container.set_hexpand(true);
        main_container.add_css_class("designer-toolbox");
        main_container.set_margin_top(5);
        main_container.set_margin_bottom(5);
        main_container.set_margin_start(5);
        main_container.set_margin_end(5);

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(false)
            .vexpand(true)
            .build();

        let content_box = Box::new(Orientation::Vertical, 2);

        let current_tool = Rc::new(RefCell::new(DesignerTool::Select));
        let init_tool = DesignerTool::Select;
        let active_tool_label = Label::new(Some(&format!(
            "{} {}",
            t!("Active tool:"),
            init_tool.tooltip()
        )));
        active_tool_label.add_css_class("active-tool-chip");
        active_tool_label.set_halign(Align::Center);
        active_tool_label.set_margin_bottom(0);

        let mut buttons: Vec<Button> = Vec::new();

        let tools = vec![
            DesignerTool::Select,
            DesignerTool::Pan,
            DesignerTool::Rectangle,
            DesignerTool::Circle,
            DesignerTool::Line,
            DesignerTool::Ellipse,
            DesignerTool::Triangle,
            DesignerTool::Polygon,
            DesignerTool::Polyline,
            DesignerTool::Text,
            DesignerTool::Gear,
            DesignerTool::Sprocket,
        ];

        let grid = gtk4::Grid::builder()
            .column_spacing(2)
            .row_spacing(2)
            .halign(Align::Center)
            .build();

        for (i, tool) in tools.iter().enumerate() {
            let btn = Button::new();
            btn.set_size_request(40, 40); // Slightly smaller for 3 columns
            btn.set_halign(Align::Center);
            let tooltip = tool.tooltip();
            btn.set_tooltip_text(Some(&tooltip));

            // Use icon from compiled resources or standard icon name
            let icon_name = tool.icon();
            let icon = if icon_name.ends_with(".svg") {
                let resource_path = format!("/com/gcodekit5/icons/{}", icon_name);
                Image::from_resource(&resource_path)
            } else {
                Image::from_icon_name(icon_name)
            };
            icon.set_pixel_size(20);
            btn.set_child(Some(&icon));

            buttons.push(btn.clone());

            // Select tool is initially selected
            if *tool == DesignerTool::Select {
                btn.add_css_class("selected-tool");
            }

            grid.attach(&btn, (i % 3) as i32, (i / 3) as i32, 1, 1);
        }

        content_box.append(&grid);

        // Now wire up click handlers after all buttons are collected
        for (i, btn) in buttons.iter().enumerate() {
            let current_tool_clone = current_tool.clone();
            let buttons_clone = buttons.clone();
            let tools_clone = tools.clone();
            let tool = tools[i];

            let active_tool_label = active_tool_label.clone();
            btn.connect_clicked(move |_| {
                *current_tool_clone.borrow_mut() = tool;
                active_tool_label.set_text(&format!("{} {}", t!("Active tool:"), tool.tooltip()));

                // Update button styles
                for (j, b) in buttons_clone.iter().enumerate() {
                    if tools_clone[j] == tool {
                        b.add_css_class("selected-tool");
                    } else {
                        b.remove_css_class("selected-tool");
                    }
                }
            });
        }

        // Fast Shapes Section
        let fast_shapes_label = Label::new(Some(&t!("Fast Shapes")));
        fast_shapes_label.add_css_class("caption");
        fast_shapes_label.set_halign(Align::Start);
        fast_shapes_label.set_margin_top(10);
        fast_shapes_label.set_margin_bottom(4);
        content_box.append(&fast_shapes_label);

        let fast_shapes_btn = Button::with_label(&t!("Shape Gallery…"));
        fast_shapes_btn.set_margin_start(5);
        fast_shapes_btn.set_margin_end(5);
        fast_shapes_btn.set_margin_bottom(5);

        let fast_shape_gallery = FastShapeGallery::new();
        let gallery_clone = fast_shape_gallery.clone();
        let fast_shapes_btn_clone = fast_shapes_btn.clone();
        fast_shapes_btn.connect_clicked(move |_| {
            if let Some(root) = fast_shapes_btn_clone.root() {
                if let Ok(win) = root.downcast::<gtk4::Window>() {
                    gallery_clone.show(&win);
                }
            }
        });

        content_box.append(&fast_shapes_btn);

        // Add separator
        let separator = gtk4::Separator::new(Orientation::Horizontal);
        separator.set_margin_top(10);
        separator.set_margin_bottom(10);
        content_box.append(&separator);

        // Tool Settings
        let settings_box = Box::new(Orientation::Vertical, 8);
        settings_box.set_margin_start(8);
        settings_box.set_margin_end(8);
        settings_box.set_margin_top(8);
        settings_box.set_margin_bottom(8);

        let settings_grid = Grid::builder().row_spacing(8).column_spacing(8).build();
        settings_box.append(&settings_grid);

        let current_units = Arc::new(Mutex::new(
            settings_controller
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system,
        ));

        // Collection of callbacks to refresh all settings UI widgets from state
        let refresh_callbacks: Rc<RefCell<Vec<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));

        let tool_row = Rc::new(Cell::new(0));

        // Helper to create a properties-style row: label | value | units
        let create_setting = {
            let settings_controller = settings_controller.clone();
            let current_units = current_units.clone();
            let settings_grid = settings_grid.clone();
            let tool_row = tool_row.clone();
            let refresh_callbacks = refresh_callbacks.clone();

            move |label_text: String,
                  getter: Rc<dyn Fn() -> f64>,
                  setter: Rc<dyn Fn(f64)>,
                  tooltip: String,
                  units_kind: UnitsKind|
                  -> Entry {
                let label = Label::new(Some(&format!("{}:", label_text)));
                label.set_halign(Align::Start);

                let entry = Entry::builder().tooltip_text(&tooltip).build();
                entry.set_hexpand(true);

                let units_label = Label::new(Some(""));
                units_label.set_halign(Align::End);
                units_label.set_xalign(1.0);
                units_label.set_width_chars(6);

                let row = tool_row.get();
                tool_row.set(row + 1);

                settings_grid.attach(&label, 0, row, 1, 1);
                settings_grid.attach(&entry, 1, row, 1, 1);
                settings_grid.attach(&units_label, 2, row, 1, 1);

                let update_display = {
                    let entry = entry.clone();
                    let units_label = units_label.clone();
                    let getter = getter.clone();
                    let current_units = current_units.clone();
                    let units_kind = units_kind;

                    Rc::new(move || {
                        let val_mm = getter();
                        let units = *current_units.lock().unwrap();

                        let (val_display, unit_str) = match units_kind {
                            UnitsKind::Length => match units {
                                MeasurementSystem::Metric => (val_mm, "mm"),
                                MeasurementSystem::Imperial => (val_mm / 25.4, "in"),
                            },
                            UnitsKind::FeedRate => match units {
                                MeasurementSystem::Metric => (val_mm, "mm/min"),
                                MeasurementSystem::Imperial => (val_mm / 25.4, "in/min"),
                            },
                            UnitsKind::Rpm => (val_mm, "RPM"),
                        };

                        units_label.set_text(unit_str);
                        entry.set_text(&format!("{:.3}", val_display));
                    })
                };

                // Register for external refresh (e.g., after file load)
                refresh_callbacks.borrow_mut().push(update_display.clone());

                update_display();

                // Connect entry changed
                {
                    let current_units = current_units.clone();
                    let setter = setter.clone();
                    let units_kind = units_kind;
                    entry.connect_changed(move |e| {
                        if let Ok(val) = e.text().parse::<f64>() {
                            e.remove_css_class("entry-invalid");
                            let units = *current_units.lock().unwrap();
                            let val_mm = match units_kind {
                                UnitsKind::Length | UnitsKind::FeedRate => match units {
                                    MeasurementSystem::Metric => val,
                                    MeasurementSystem::Imperial => val * 25.4,
                                },
                                UnitsKind::Rpm => val,
                            };
                            setter(val_mm);
                        } else {
                            e.add_css_class("entry-invalid");
                        }
                    });
                }

                // Connect settings changed
                {
                    let update_display = update_display.clone();
                    let current_units = current_units.clone();
                    settings_controller.on_setting_changed(move |key, value| {
                        if key == "units.measurement_system" {
                            if let Ok(system) =
                                serde_json::from_str::<MeasurementSystem>(&format!("\"{}\"", value))
                            {
                                *current_units.lock().unwrap() = system;
                            } else if value == "Metric" {
                                *current_units.lock().unwrap() = MeasurementSystem::Metric;
                            } else if value == "Imperial" {
                                *current_units.lock().unwrap() = MeasurementSystem::Imperial;
                            }
                            update_display();
                        }
                    });
                }

                entry
            }
        };

        // Feed Rate
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().tool_settings.feed_rate);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f64| state_setter.borrow_mut().set_feed_rate(val));
            create_setting(
                t!("Feed"),
                getter,
                setter,
                t!("Feed Rate"),
                UnitsKind::FeedRate,
            );
        }

        // Spindle Speed
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().tool_settings.spindle_speed as f64);
            let state_setter = state.clone();
            let setter =
                Rc::new(move |val: f64| state_setter.borrow_mut().set_spindle_speed(val as u32));
            create_setting(
                t!("Speed"),
                getter,
                setter,
                t!("Spindle Speed"),
                UnitsKind::Rpm,
            );
        }

        // Tool Diameter
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().tool_settings.tool_diameter);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f64| state_setter.borrow_mut().set_tool_diameter(val));
            create_setting(
                t!("Tool Dia"),
                getter,
                setter,
                t!("Tool Diameter"),
                UnitsKind::Length,
            );
        }

        // Cut Depth
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().tool_settings.cut_depth);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f64| state_setter.borrow_mut().set_cut_depth(val));
            create_setting(
                t!("Cut Depth"),
                getter,
                setter,
                t!("Target Cut Depth (positive)"),
                UnitsKind::Length,
            );
        }

        // Step Down
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().tool_settings.step_down);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f64| state_setter.borrow_mut().set_step_down(val));
            create_setting(
                t!("Step Down"),
                getter,
                setter,
                t!("Depth per pass"),
                UnitsKind::Length,
            );
        }

        // Tool Settings popup
        let tool_settings_btn = Button::with_label(&t!("Tool Settings…"));
        tool_settings_btn.set_margin_top(6);
        tool_settings_btn.set_margin_start(5);
        tool_settings_btn.set_margin_end(5);

        let tool_settings_dialog = Dialog::builder()
            .title(t!("Tool Settings"))
            .modal(true)
            .resizable(true)
            .build();
        tool_settings_dialog.set_default_size(520, 520);
        tool_settings_dialog.add_button(&t!("Close"), ResponseType::Close);
        tool_settings_dialog.connect_response(|d, _| d.hide());

        let tool_dialog_content = Box::new(Orientation::Vertical, 12);
        tool_dialog_content.set_margin_start(12);
        tool_dialog_content.set_margin_end(12);
        tool_dialog_content.set_margin_top(12);
        tool_dialog_content.set_margin_bottom(12);

        let tool_header = Label::new(Some(&t!("Tool Settings")));
        tool_header.add_css_class("title-3");
        tool_header.set_halign(Align::Start);
        tool_dialog_content.append(&tool_header);

        let tool_frame = Frame::new(Some(&t!("Tool Parameters")));
        tool_frame.set_child(Some(&settings_box));
        tool_dialog_content.append(&tool_frame);

        let tool_scroller = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .min_content_width(520)
            .min_content_height(360)
            .child(&tool_dialog_content)
            .build();
        tool_settings_dialog.content_area().append(&tool_scroller);

        {
            let dlg = tool_settings_dialog.clone();
            let btn = tool_settings_btn.clone();
            tool_settings_btn.connect_clicked(move |_| {
                if let Some(root) = btn.root() {
                    if let Ok(win) = root.downcast::<gtk4::Window>() {
                        dlg.set_transient_for(Some(&win));
                    }
                }
                dlg.present();
            });
        }

        content_box.append(&tool_settings_btn);

        // Stock Settings
        let stock_box = Box::new(Orientation::Vertical, 8);
        stock_box.set_margin_start(8);
        stock_box.set_margin_end(8);
        stock_box.set_margin_top(8);
        stock_box.set_margin_bottom(8);

        let stock_grid = Grid::builder().row_spacing(8).column_spacing(8).build();
        stock_box.append(&stock_grid);
        let stock_row = Rc::new(Cell::new(0));

        // Helper for stock settings (f32): label | value | units
        let create_stock_setting = {
            let settings_controller = settings_controller.clone();
            let current_units = current_units.clone();
            let stock_grid = stock_grid.clone();
            let stock_row = stock_row.clone();
            let refresh_callbacks = refresh_callbacks.clone();

            move |label_text: String,
                  getter: Rc<dyn Fn() -> f32>,
                  setter: Rc<dyn Fn(f32)>,
                  tooltip: String|
                  -> Entry {
                let label = Label::new(Some(&format!("{}:", label_text)));
                label.set_halign(Align::Start);

                let entry = Entry::builder().tooltip_text(&tooltip).build();
                entry.set_hexpand(true);

                let units_label = Label::new(Some(""));
                units_label.set_halign(Align::End);
                units_label.set_xalign(1.0);
                units_label.set_width_chars(6);

                let row = stock_row.get();
                stock_row.set(row + 1);

                stock_grid.attach(&label, 0, row, 1, 1);
                stock_grid.attach(&entry, 1, row, 1, 1);
                stock_grid.attach(&units_label, 2, row, 1, 1);

                let update_display = {
                    let entry = entry.clone();
                    let units_label = units_label.clone();
                    let getter = getter.clone();
                    let current_units = current_units.clone();

                    Rc::new(move || {
                        let val_mm = getter();
                        let units = *current_units.lock().unwrap();

                        let (val_display, unit_str) = match units {
                            MeasurementSystem::Metric => (val_mm, "mm"),
                            MeasurementSystem::Imperial => (val_mm / 25.4, "in"),
                        };

                        units_label.set_text(unit_str);
                        entry.set_text(&format!("{:.3}", val_display));
                    })
                };

                // Register for external refresh (e.g., after file load)
                refresh_callbacks.borrow_mut().push(update_display.clone());

                update_display();

                // Connect entry changed
                {
                    let current_units = current_units.clone();
                    let setter = setter.clone();
                    entry.connect_changed(move |e| {
                        if let Ok(val) = e.text().parse::<f32>() {
                            e.remove_css_class("entry-invalid");
                            let units = *current_units.lock().unwrap();
                            let val_mm = match units {
                                MeasurementSystem::Metric => val,
                                MeasurementSystem::Imperial => val * 25.4,
                            };
                            setter(val_mm);
                        } else {
                            e.add_css_class("entry-invalid");
                        }
                    });
                }

                // Connect settings changed
                {
                    let update_display = update_display.clone();
                    let current_units = current_units.clone();
                    settings_controller.on_setting_changed(move |key, value| {
                        if key == "units.measurement_system" {
                            if let Ok(system) =
                                serde_json::from_str::<MeasurementSystem>(&format!("\"{}\"", value))
                            {
                                *current_units.lock().unwrap() = system;
                            } else if value == "Metric" {
                                *current_units.lock().unwrap() = MeasurementSystem::Metric;
                            } else if value == "Imperial" {
                                *current_units.lock().unwrap() = MeasurementSystem::Imperial;
                            }
                            update_display();
                        }
                    });
                }

                entry
            }
        };

        // Stock dimensions
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || {
                state_getter
                    .borrow()
                    .stock_material
                    .as_ref()
                    .map(|s| s.width)
                    .unwrap_or(200.0)
            });
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f32| {
                let mut s = state_setter.borrow_mut();
                if let Some(ref mut stock) = s.stock_material {
                    stock.width = val;
                }
            });
            create_stock_setting(
                t!("Stock Width"),
                getter,
                setter,
                t!("Stock material width"),
            );
        }

        {
            let state_getter = state.clone();
            let getter = Rc::new(move || {
                state_getter
                    .borrow()
                    .stock_material
                    .as_ref()
                    .map(|s| s.height)
                    .unwrap_or(200.0)
            });
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f32| {
                let mut s = state_setter.borrow_mut();
                if let Some(ref mut stock) = s.stock_material {
                    stock.height = val;
                }
            });
            create_stock_setting(
                t!("Stock Height"),
                getter,
                setter,
                t!("Stock material height"),
            );
        }

        {
            let state_getter = state.clone();
            let getter = Rc::new(move || {
                state_getter
                    .borrow()
                    .stock_material
                    .as_ref()
                    .map(|s| s.thickness)
                    .unwrap_or(10.0)
            });
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f32| {
                let mut s = state_setter.borrow_mut();
                if let Some(ref mut stock) = s.stock_material {
                    stock.thickness = val;
                }
            });
            create_stock_setting(
                t!("Stock Thickness"),
                getter,
                setter,
                t!("Stock material thickness"),
            );
        }

        // Safe Z Height
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || {
                state_getter
                    .borrow()
                    .stock_material
                    .as_ref()
                    .map(|s| s.safe_z)
                    .unwrap_or(10.0)
            });
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f32| {
                let mut s = state_setter.borrow_mut();
                if let Some(ref mut stock) = s.stock_material {
                    stock.safe_z = val;
                }
            });
            create_stock_setting(
                t!("Safe Z Height"),
                getter,
                setter,
                t!("Safe height for rapid moves"),
            );
        }

        // Resolution
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().simulation_resolution);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f32| {
                state_setter.borrow_mut().simulation_resolution = val.max(0.01).min(2.0);
            });
            create_stock_setting(
                t!("Resolution"),
                getter,
                setter,
                t!("Simulation resolution (lower = more detail)"),
            );
        }

        // Show Stock Removal checkbox
        let show_stock_check = gtk4::CheckButton::with_label(&t!("Show Stock Removal"));
        show_stock_check.set_tooltip_text(Some(&t!("Enable stock removal visualization")));
        show_stock_check.set_margin_top(5);
        let show_stock_state = state.borrow().show_stock_removal;
        show_stock_check.set_active(show_stock_state);
        let state_show_stock = state.clone();
        show_stock_check.connect_toggled(move |cb| {
            state_show_stock.borrow_mut().show_stock_removal = cb.is_active();
        });
        stock_box.append(&show_stock_check);

        // Stock Settings popup
        let stock_settings_btn = Button::with_label(&t!("Stock Settings…"));
        stock_settings_btn.set_margin_top(6);
        stock_settings_btn.set_margin_start(5);
        stock_settings_btn.set_margin_end(5);

        let stock_settings_dialog = Dialog::builder()
            .title(t!("Stock Settings"))
            .modal(true)
            .resizable(true)
            .build();
        stock_settings_dialog.set_default_size(520, 520);
        stock_settings_dialog.add_button(&t!("Close"), ResponseType::Close);
        stock_settings_dialog.connect_response(|d, _| d.hide());

        let stock_dialog_content = Box::new(Orientation::Vertical, 12);
        stock_dialog_content.set_margin_start(12);
        stock_dialog_content.set_margin_end(12);
        stock_dialog_content.set_margin_top(12);
        stock_dialog_content.set_margin_bottom(12);

        let stock_header = Label::new(Some(&t!("Stock Settings")));
        stock_header.add_css_class("title-3");
        stock_header.set_halign(Align::Start);
        stock_dialog_content.append(&stock_header);

        let stock_frame = Frame::new(Some(&t!("Stock Parameters")));
        stock_frame.set_child(Some(&stock_box));
        stock_dialog_content.append(&stock_frame);

        let stock_scroller = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .min_content_width(520)
            .min_content_height(360)
            .child(&stock_dialog_content)
            .build();
        stock_settings_dialog.content_area().append(&stock_scroller);

        {
            let dlg = stock_settings_dialog.clone();
            let btn = stock_settings_btn.clone();
            stock_settings_btn.connect_clicked(move |_| {
                if let Some(root) = btn.root() {
                    if let Ok(win) = root.downcast::<gtk4::Window>() {
                        dlg.set_transient_for(Some(&win));
                    }
                }
                dlg.present();
            });
        }

        content_box.append(&stock_settings_btn);

        // Generate G-Code Button
        let generate_btn = Button::with_label(&t!("Generate G-Code"));
        generate_btn.add_css_class("suggested-action");
        generate_btn.set_margin_top(10);
        generate_btn.set_margin_bottom(10);
        generate_btn.set_margin_start(5);
        generate_btn.set_margin_end(5);
        content_box.append(&generate_btn);

        scrolled.set_child(Some(&content_box));
        main_container.append(&scrolled);

        Rc::new(Self {
            widget: main_container,
            current_tool,
            active_tool_label,
            buttons,
            tools,
            generate_btn,
            fast_shape_gallery,
            _state: state,
            _settings_controller: settings_controller,
            _current_units: current_units,
            refresh_callbacks,
        })
    }

    pub fn connect_generate_clicked<F: Fn() + 'static>(&self, f: F) {
        self.generate_btn.connect_clicked(move |_| f());
    }

    pub fn fast_shape_gallery(&self) -> Rc<FastShapeGallery> {
        self.fast_shape_gallery.clone()
    }

    pub fn current_tool(&self) -> DesignerTool {
        *self.current_tool.borrow()
    }

    pub fn active_tool_label(&self) -> Label {
        self.active_tool_label.clone()
    }

    pub fn set_tool(&self, tool: DesignerTool) {
        *self.current_tool.borrow_mut() = tool;
        self.active_tool_label
            .set_text(&format!("{} {}", t!("Active tool:"), tool.tooltip()));

        // Update button styles
        for (i, btn) in self.buttons.iter().enumerate() {
            if self.tools[i] == tool {
                btn.add_css_class("selected-tool");
            } else {
                btn.remove_css_class("selected-tool");
            }
        }
    }

    /// Refresh all tool and stock settings UI widgets from current state.
    /// Call this after loading a design file to update displayed values.
    pub fn refresh_settings(&self) {
        for callback in self.refresh_callbacks.borrow().iter() {
            callback();
        }
    }
}
