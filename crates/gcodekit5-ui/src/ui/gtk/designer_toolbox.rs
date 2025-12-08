use gtk4::prelude::*;
use gtk4::{Box, Button, Orientation, Image, SpinButton, Label, Adjustment, Expander, Align};
use std::cell::RefCell;
use std::rc::Rc;
use gcodekit5_designer::designer_state::DesignerState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignerTool {
    Select = 0,
    Rectangle = 1,
    Circle = 2,
    Line = 3,
    Ellipse = 4,
    Polyline = 5,
    Text = 6,
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
        }
    }
}

pub struct DesignerToolbox {
    pub widget: Box,
    current_tool: Rc<RefCell<DesignerTool>>,
    buttons: Vec<Button>,
    state: Rc<RefCell<DesignerState>>,
}

impl DesignerToolbox {
    pub fn new(state: Rc<RefCell<DesignerState>>) -> Rc<Self> {
        let container = Box::new(Orientation::Vertical, 2);
        container.set_width_request(60);
        container.add_css_class("designer-toolbox");
        container.set_margin_top(5);
        container.set_margin_bottom(5);
        container.set_margin_start(5);
        container.set_margin_end(5);
        
        let current_tool = Rc::new(RefCell::new(DesignerTool::Select));
        let mut buttons: Vec<Button> = Vec::new();
        
        let tools = [
            DesignerTool::Select,
            DesignerTool::Rectangle,
            DesignerTool::Circle,
            DesignerTool::Line,
            DesignerTool::Ellipse,
            DesignerTool::Polyline,
            DesignerTool::Text,
        ];
        
        for tool in tools.iter() {
            let btn = Button::new();
            btn.set_size_request(50, 50);
            btn.set_tooltip_text(Some(tool.tooltip()));
            
            // Use icon from compiled resources
            let icon_filename = tool.icon();
            let resource_path = format!("/com/gcodekit5/icons/{}", icon_filename);
            
            let icon = Image::from_resource(&resource_path);
            icon.set_pixel_size(24);
            btn.set_child(Some(&icon));
            
            // Fallback logic is removed as we expect resources to be present.
            // If we wanted to be safe, we could check if the icon loaded properly, 
            // but Image::from_resource doesn't return a Result.
            
            buttons.push(btn.clone());
            
            // Select tool is initially selected
            if *tool == DesignerTool::Select {
                btn.add_css_class("selected-tool");
            }
            
            container.append(&btn);
        }
        
        // Now wire up click handlers after all buttons are collected
        for (i, btn) in buttons.iter().enumerate() {
            let current_tool_clone = current_tool.clone();
            let buttons_clone = buttons.clone();
            let tool = tools[i];
            
            btn.connect_clicked(move |_| {
                *current_tool_clone.borrow_mut() = tool;
                // Update button styles
                for (j, b) in buttons_clone.iter().enumerate() {
                    if j == tool as usize {
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
        container.append(&separator);

        // Tool Settings
        let settings_box = Box::new(Orientation::Vertical, 5);
        settings_box.set_margin_start(2);
        settings_box.set_margin_end(2);

        // Helper to create labeled spin button
        let create_setting = |label_text: &str, min: f64, max: f64, step: f64, value: f64, digits: u32, tooltip: &str| -> SpinButton {
            let label = Label::builder()
                .label(label_text)
                .halign(Align::Start)
                .build();
            label.add_css_class("small-label");
            settings_box.append(&label);

            let adj = Adjustment::new(value, min, max, step, step * 10.0, 0.0);
            let spin = SpinButton::builder()
                .adjustment(&adj)
                .climb_rate(step)
                .digits(digits)
                .tooltip_text(tooltip)
                .build();
            settings_box.append(&spin);
            spin
        };

        let current_settings = state.borrow().tool_settings.clone();

        // Feed Rate
        let feed_spin = create_setting("Feed (mm/min)", 1.0, 10000.0, 10.0, current_settings.feed_rate, 0, "Feed Rate");
        let state_feed = state.clone();
        feed_spin.connect_value_changed(move |spin| {
            state_feed.borrow_mut().set_feed_rate(spin.value());
        });

        // Spindle Speed
        let speed_spin = create_setting("Speed (RPM)", 0.0, 30000.0, 100.0, current_settings.spindle_speed as f64, 0, "Spindle Speed");
        let state_speed = state.clone();
        speed_spin.connect_value_changed(move |spin| {
            state_speed.borrow_mut().set_spindle_speed(spin.value() as u32);
        });

        // Tool Diameter
        let diam_spin = create_setting("Tool Dia (mm)", 0.1, 50.0, 0.1, current_settings.tool_diameter, 2, "Tool Diameter");
        let state_diam = state.clone();
        diam_spin.connect_value_changed(move |spin| {
            state_diam.borrow_mut().set_tool_diameter(spin.value());
        });

        // Cut Depth
        let depth_spin = create_setting("Cut Depth (mm)", 0.1, 100.0, 0.1, current_settings.cut_depth, 2, "Target Cut Depth (positive)");
        let state_depth = state.clone();
        depth_spin.connect_value_changed(move |spin| {
            // UI shows positive depth, backend expects negative usually? 
            // Let's check set_cut_depth implementation.
            // ToolpathGenerator usually takes negative Z for depth.
            // But let's see what set_cut_depth does.
            // It just sets the value.
            // In generate_gcode, it uses header_depth which comes from toolpath.depth.
            // Usually depth is negative Z.
            // But UI usually shows positive "Depth".
            // Let's assume we store it as positive in settings and convert when generating if needed, 
            // OR we store as negative.
            // ToolSettings default is 5.0 (positive).
            // So let's stick to positive here.
            state_depth.borrow_mut().set_cut_depth(spin.value());
        });

        // Step Down
        let step_spin = create_setting("Step Down (mm)", 0.1, 20.0, 0.1, current_settings.step_down, 2, "Depth per pass");
        let state_step = state.clone();
        step_spin.connect_value_changed(move |spin| {
            state_step.borrow_mut().set_step_down(spin.value());
        });

        let expander = Expander::builder()
            .label("Tool Settings")
            .child(&settings_box)
            .expanded(true)
            .build();
        
        container.append(&expander);
        
        Rc::new(Self {
            widget: container,
            current_tool,
            buttons,
            state,
        })
    }
    
    pub fn current_tool(&self) -> DesignerTool {
        *self.current_tool.borrow()
    }
    
    pub fn set_tool(&self, tool: DesignerTool) {
        *self.current_tool.borrow_mut() = tool;
        
        // Update button styles
        for (i, btn) in self.buttons.iter().enumerate() {
            if i == tool as usize {
                btn.add_css_class("selected-tool");
            } else {
                btn.remove_css_class("selected-tool");
            }
        }
        // Update button styles
        for (i, btn) in self.buttons.iter().enumerate() {
            btn.remove_css_class("selected-tool");
            if i == tool as usize {
                btn.add_css_class("selected-tool");
            }
        }
    }
}
