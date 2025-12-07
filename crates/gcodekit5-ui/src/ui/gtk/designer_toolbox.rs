use gtk4::prelude::*;
use gtk4::{Box, Button, Orientation, Image, Label};
use std::cell::RefCell;
use std::rc::Rc;

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
            DesignerTool::Select => "object-select-symbolic",
            DesignerTool::Rectangle => "insert-object-symbolic",
            DesignerTool::Circle => "draw-circle-symbolic",
            DesignerTool::Line => "draw-line-symbolic",
            DesignerTool::Ellipse => "draw-ellipse-symbolic",
            DesignerTool::Polyline => "draw-polygon-symbolic",
            DesignerTool::Text => "insert-text-symbolic",
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
}

impl DesignerToolbox {
    pub fn new() -> Rc<Self> {
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
            
            // Try to use icon, fall back to label if icon not available
            let icon_name = tool.icon();
            if gtk4::IconTheme::for_display(&gtk4::gdk::Display::default().unwrap())
                .has_icon(icon_name) 
            {
                let icon = Image::from_icon_name(icon_name);
                btn.set_child(Some(&icon));
            } else {
                // Fallback: use text label
                let label = Label::new(Some(tool.name()));
                btn.set_child(Some(&label));
            }
            
            let tool_clone = *tool;
            let current_tool_clone = current_tool.clone();
            let buttons_clone = buttons.clone();
            let btn_clone = btn.clone();
            
            btn.connect_clicked(move |_| {
                *current_tool_clone.borrow_mut() = tool_clone;
                // Update button styles
                for b in &buttons_clone {
                    b.remove_css_class("selected-tool");
                }
                btn_clone.add_css_class("selected-tool");
            });
            
            // Select tool is initially selected
            if *tool == DesignerTool::Select {
                btn.add_css_class("selected-tool");
            }
            
            container.append(&btn);
            buttons.push(btn);
        }
        
        // Add separator
        let separator = gtk4::Separator::new(Orientation::Horizontal);
        separator.set_margin_top(10);
        separator.set_margin_bottom(10);
        container.append(&separator);
        
        // Add zoom controls (placeholder for Phase 1 viewport)
        let zoom_label = Label::new(Some("Zoom"));
        zoom_label.add_css_class("dim-label");
        container.append(&zoom_label);
        
        let zoom_in = Button::with_label("+");
        zoom_in.set_tooltip_text(Some("Zoom in"));
        container.append(&zoom_in);
        
        let zoom_out = Button::with_label("-");
        zoom_out.set_tooltip_text(Some("Zoom out"));
        container.append(&zoom_out);
        
        let zoom_fit = Button::with_label("Fit");
        zoom_fit.set_tooltip_text(Some("Fit to view"));
        container.append(&zoom_fit);
        
        Rc::new(Self {
            widget: container,
            current_tool,
            buttons,
        })
    }
    
    pub fn current_tool(&self) -> DesignerTool {
        *self.current_tool.borrow()
    }
    
    pub fn set_tool(&self, tool: DesignerTool) {
        *self.current_tool.borrow_mut() = tool;
        // Update button styles
        for (i, btn) in self.buttons.iter().enumerate() {
            btn.remove_css_class("selected-tool");
            if i == tool as usize {
                btn.add_css_class("selected-tool");
            }
        }
    }
}
