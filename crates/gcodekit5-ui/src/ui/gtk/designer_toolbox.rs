use gtk4::prelude::*;
use gtk4::{Box, Button, Orientation, Image, Label, Expander, Align, ScrolledWindow, PolicyType, Entry};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_settings::controller::SettingsController;
use gcodekit5_core::units::MeasurementSystem;

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
}

impl DesignerTool {
    pub fn name(&self) -> &'static str {
        match self {
            DesignerTool::Select => "Select",
            DesignerTool::Rectangle => "Rectangle",
            DesignerTool::Circle => "Circle",
            DesignerTool::Line => "Line",
            DesignerTool::Ellipse => "Ellipse",
            DesignerTool::Polyline => "Polyline",
            DesignerTool::Text => "Text",
            DesignerTool::Pan => "Pan",
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
        }
    }
    
    pub fn tooltip(&self) -> &'static str {
        match self {
            DesignerTool::Select => "Select and move shapes (S)",
            DesignerTool::Rectangle => "Draw rectangle (R)",
            DesignerTool::Circle => "Draw circle (C)",
            DesignerTool::Line => "Draw line (L)",
            DesignerTool::Ellipse => "Draw ellipse (E)",
            DesignerTool::Polyline => "Draw polyline/polygon (P)",
            DesignerTool::Text => "Add text (T)",
            DesignerTool::Pan => "Pan canvas (Space)",
        }
    }
}

pub struct DesignerToolbox {
    pub widget: Box,
    current_tool: Rc<RefCell<DesignerTool>>,
    buttons: Vec<Button>,
    tools: Vec<DesignerTool>,
    generate_btn: Button,
    _state: Rc<RefCell<DesignerState>>,
    _settings_controller: Rc<SettingsController>,
    _current_units: Arc<Mutex<MeasurementSystem>>,
}

impl DesignerToolbox {
    pub fn new(state: Rc<RefCell<DesignerState>>, settings_controller: Rc<SettingsController>) -> Rc<Self> {
        let main_container = Box::new(Orientation::Vertical, 0);
        main_container.set_width_request(160); // Increased width for 3 columns
        main_container.set_hexpand(false);
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
        let mut buttons: Vec<Button> = Vec::new();
        
        let tools = vec![
            DesignerTool::Select,
            DesignerTool::Pan,
            DesignerTool::Rectangle,
            DesignerTool::Circle,
            DesignerTool::Line,
            DesignerTool::Ellipse,
            DesignerTool::Polyline,
            DesignerTool::Text,
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
            btn.set_tooltip_text(Some(tool.tooltip()));
            
            // Use icon from compiled resources
            let icon_filename = tool.icon();
            let resource_path = format!("/com/gcodekit5/icons/{}", icon_filename);
            
            let icon = Image::from_resource(&resource_path);
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
            
            btn.connect_clicked(move |_| {
                *current_tool_clone.borrow_mut() = tool;
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
        
        // Add separator
        let separator = gtk4::Separator::new(Orientation::Horizontal);
        separator.set_margin_top(10);
        separator.set_margin_bottom(10);
        content_box.append(&separator);

        // Tool Settings
        let settings_box = Box::new(Orientation::Vertical, 5);
        settings_box.set_margin_start(2);
        settings_box.set_margin_end(2);

        let current_units = Arc::new(Mutex::new(settings_controller.persistence.borrow().config().ui.measurement_system));

        // Helper to create labeled entry with unit support
        let create_setting = {
            let settings_controller = settings_controller.clone();
            let current_units = current_units.clone();
            let settings_box = settings_box.clone();
            
            move |label_text: &str, getter: Rc<dyn Fn() -> f64>, setter: Rc<dyn Fn(f64)>, tooltip: &str, is_length: bool| -> Entry {
                let label = Label::builder()
                    .label(label_text)
                    .halign(Align::Start)
                    .build();
                label.add_css_class("small-label");
                settings_box.append(&label);

                let entry = Entry::builder()
                    .tooltip_text(tooltip)
                    .build();
                settings_box.append(&entry);

                let update_display = {
                    let entry = entry.clone();
                    let label = label.clone();
                    let getter = getter.clone();
                    let current_units = current_units.clone();
                    let label_text = label_text.to_string();
                    
                    Rc::new(move || {
                        let val_mm = getter();
                        let units = *current_units.lock().unwrap();
                        
                        let (val_display, unit_str) = if is_length {
                            match units {
                                MeasurementSystem::Metric => (val_mm, "mm"),
                                MeasurementSystem::Imperial => (val_mm / 25.4, "in"),
                            }
                        } else {
                            (val_mm, "")
                        };
                        
                        // Update label
                        let new_label = if !unit_str.is_empty() {
                            if label_text.contains("(mm)") {
                                label_text.replace("(mm)", &format!("({})", unit_str))
                            } else if label_text.contains("(in)") {
                                label_text.replace("(in)", &format!("({})", unit_str))
                            } else if label_text.contains("mm/min") {
                                if unit_str == "in" {
                                    label_text.replace("mm/min", "in/min")
                                } else {
                                    label_text.clone()
                                }
                            } else {
                                format!("{} ({})", label_text, unit_str)
                            }
                        } else {
                            label_text.clone()
                        };
                        label.set_label(&new_label);
                        
                        // Update entry
                        entry.set_text(&format!("{:.3}", val_display));
                    })
                };
                
                // Initial update
                update_display();
                
                // Connect entry changed
                {
                    let current_units = current_units.clone();
                    let setter = setter.clone();
                    entry.connect_changed(move |e| {
                        if let Ok(val) = e.text().parse::<f64>() {
                            let units = *current_units.lock().unwrap();
                            let val_mm = if is_length {
                                match units {
                                    MeasurementSystem::Metric => val,
                                    MeasurementSystem::Imperial => val * 25.4,
                                }
                            } else {
                                val
                            };
                            setter(val_mm);
                        }
                    });
                }
                
                // Connect settings changed
                {
                    let update_display = update_display.clone();
                    let current_units = current_units.clone();
                    settings_controller.on_setting_changed(move |key, value| {
                        if key == "units.measurement_system" {
                            if let Ok(system) = serde_json::from_str::<MeasurementSystem>(&format!("\"{}\"", value)) {
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
            create_setting("Feed (mm/min)", getter, setter, "Feed Rate", true);
        }

        // Spindle Speed
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().tool_settings.spindle_speed as f64);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f64| state_setter.borrow_mut().set_spindle_speed(val as u32));
            create_setting("Speed (RPM)", getter, setter, "Spindle Speed", false);
        }

        // Tool Diameter
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().tool_settings.tool_diameter);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f64| state_setter.borrow_mut().set_tool_diameter(val));
            create_setting("Tool Dia (mm)", getter, setter, "Tool Diameter", true);
        }

        // Cut Depth
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().tool_settings.cut_depth);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f64| state_setter.borrow_mut().set_cut_depth(val));
            create_setting("Cut Depth (mm)", getter, setter, "Target Cut Depth (positive)", true);
        }

        // Step Down
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().tool_settings.step_down);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f64| state_setter.borrow_mut().set_step_down(val));
            create_setting("Step Down (mm)", getter, setter, "Depth per pass", true);
        }

        let expander = Expander::builder()
            .label("Tool Settings")
            .child(&settings_box)
            .expanded(true)
            .build();
        
        content_box.append(&expander);

        // Stock Settings
        let stock_box = Box::new(Orientation::Vertical, 5);
        stock_box.set_margin_start(2);
        stock_box.set_margin_end(2);

        // Helper for stock settings (f32)
        let create_stock_setting = {
            let settings_controller = settings_controller.clone();
            let current_units = current_units.clone();
            let stock_box = stock_box.clone();
            
            move |label_text: &str, getter: Rc<dyn Fn() -> f32>, setter: Rc<dyn Fn(f32)>, tooltip: &str| -> Entry {
                let label = Label::builder()
                    .label(label_text)
                    .halign(Align::Start)
                    .build();
                label.add_css_class("small-label");
                stock_box.append(&label);

                let entry = Entry::builder()
                    .tooltip_text(tooltip)
                    .build();
                stock_box.append(&entry);

                let update_display = {
                    let entry = entry.clone();
                    let label = label.clone();
                    let getter = getter.clone();
                    let current_units = current_units.clone();
                    let label_text = label_text.to_string();
                    
                    Rc::new(move || {
                        let val_mm = getter();
                        let units = *current_units.lock().unwrap();
                        
                        let (val_display, unit_str) = match units {
                            MeasurementSystem::Metric => (val_mm, "mm"),
                            MeasurementSystem::Imperial => (val_mm / 25.4, "in"),
                        };
                        
                        // Update label
                        let new_label = if label_text.contains("(mm)") {
                            label_text.replace("(mm)", &format!("({})", unit_str))
                        } else if label_text.contains("(in)") {
                            label_text.replace("(in)", &format!("({})", unit_str))
                        } else {
                            format!("{} ({})", label_text, unit_str)
                        };
                        label.set_label(&new_label);
                        
                        // Update entry
                        entry.set_text(&format!("{:.3}", val_display));
                    })
                };
                
                // Initial update
                update_display();
                
                // Connect entry changed
                {
                    let current_units = current_units.clone();
                    let setter = setter.clone();
                    entry.connect_changed(move |e| {
                        if let Ok(val) = e.text().parse::<f32>() {
                            let units = *current_units.lock().unwrap();
                            let val_mm = match units {
                                MeasurementSystem::Metric => val,
                                MeasurementSystem::Imperial => val * 25.4,
                            };
                            setter(val_mm);
                        }
                    });
                }
                
                // Connect settings changed
                {
                    let update_display = update_display.clone();
                    let current_units = current_units.clone();
                    settings_controller.on_setting_changed(move |key, value| {
                        if key == "units.measurement_system" {
                            if let Ok(system) = serde_json::from_str::<MeasurementSystem>(&format!("\"{}\"", value)) {
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
                state_getter.borrow().stock_material.as_ref().map(|s| s.width).unwrap_or(200.0)
            });
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f32| {
                let mut s = state_setter.borrow_mut();
                if let Some(ref mut stock) = s.stock_material {
                    stock.width = val;
                }
            });
            create_stock_setting("Stock Width (mm)", getter, setter, "Stock material width");
        }

        {
            let state_getter = state.clone();
            let getter = Rc::new(move || {
                state_getter.borrow().stock_material.as_ref().map(|s| s.height).unwrap_or(200.0)
            });
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f32| {
                let mut s = state_setter.borrow_mut();
                if let Some(ref mut stock) = s.stock_material {
                    stock.height = val;
                }
            });
            create_stock_setting("Stock Height (mm)", getter, setter, "Stock material height");
        }

        {
            let state_getter = state.clone();
            let getter = Rc::new(move || {
                state_getter.borrow().stock_material.as_ref().map(|s| s.thickness).unwrap_or(10.0)
            });
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f32| {
                let mut s = state_setter.borrow_mut();
                if let Some(ref mut stock) = s.stock_material {
                    stock.thickness = val;
                }
            });
            create_stock_setting("Stock Thickness (mm)", getter, setter, "Stock material thickness");
        }

        // Resolution
        {
            let state_getter = state.clone();
            let getter = Rc::new(move || state_getter.borrow().simulation_resolution);
            let state_setter = state.clone();
            let setter = Rc::new(move |val: f32| {
                state_setter.borrow_mut().simulation_resolution = val.max(0.01).min(2.0);
            });
            create_stock_setting("Resolution (mm)", getter, setter, "Simulation resolution (lower = more detail)");
        }

        // Show Stock Removal checkbox
        let show_stock_check = gtk4::CheckButton::with_label("Show Stock Removal");
        show_stock_check.set_tooltip_text(Some("Enable stock removal visualization"));
        show_stock_check.set_margin_top(5);
        let show_stock_state = state.borrow().show_stock_removal;
        show_stock_check.set_active(show_stock_state);
        let state_show_stock = state.clone();
        show_stock_check.connect_toggled(move |cb| {
            state_show_stock.borrow_mut().show_stock_removal = cb.is_active();
        });
        stock_box.append(&show_stock_check);

        let stock_expander = Expander::builder()
            .label("Stock Settings")
            .child(&stock_box)
            .expanded(false)
            .build();
        
        content_box.append(&stock_expander);

        // Generate G-Code Button
        let generate_btn = Button::with_label("Generate G-Code");
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
            buttons,
            tools,
            generate_btn,
            _state: state,
            _settings_controller: settings_controller,
            _current_units: current_units,
        })
    }
    
    pub fn connect_generate_clicked<F: Fn() + 'static>(&self, f: F) {
        self.generate_btn.connect_clicked(move |_| f());
    }
    
    pub fn current_tool(&self) -> DesignerTool {
        *self.current_tool.borrow()
    }
    
    pub fn set_tool(&self, tool: DesignerTool) {
        *self.current_tool.borrow_mut() = tool;
        
        // Update button styles
        for (i, btn) in self.buttons.iter().enumerate() {
            if self.tools[i] == tool {
                btn.add_css_class("selected-tool");
            } else {
                btn.remove_css_class("selected-tool");
            }
        }
    }
}
