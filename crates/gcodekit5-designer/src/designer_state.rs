//! Designer state manager for UI integration.
//! Manages the designer canvas state and handles UI callbacks.

use crate::{
    shapes::{OperationType, PathShape, Shape, TextShape},
    canvas::DrawingObject,
    Canvas, Circle, DrawingMode, Line, Point, Rectangle, Ellipse, ToolpathGenerator, ToolpathToGcode,
};
use crate::commands::*;
use gcodekit5_core::Units;

/// Tool settings for the designer
#[derive(Clone, Debug)]
pub struct ToolSettings {
    pub feed_rate: f64,
    pub spindle_speed: u32,
    pub tool_diameter: f64,
    pub cut_depth: f64,
    pub step_down: f64,
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            feed_rate: 100.0,
            spindle_speed: 3000,
            tool_diameter: 3.175,
            cut_depth: 5.0,
            step_down: 1.0,
        }
    }
}

/// Designer state for UI integration
#[derive(Clone)]
pub struct DesignerState {
    pub canvas: Canvas,
    pub toolpath_generator: ToolpathGenerator,
    pub tool_settings: ToolSettings,
    pub generated_gcode: String,
    pub gcode_generated: bool,
    pub current_file_path: Option<std::path::PathBuf>,
    pub is_modified: bool,
    pub design_name: String,
    pub show_grid: bool,
    pub clipboard: Vec<crate::canvas::DrawingObject>,
    pub default_properties_shape: crate::canvas::DrawingObject,
    undo_stack: Vec<DesignerCommand>,
    redo_stack: Vec<DesignerCommand>,
}

impl DesignerState {
    /// Creates a new designer state.
    pub fn new() -> Self {
        Self {
            canvas: Canvas::with_size(800.0, 600.0),
            toolpath_generator: ToolpathGenerator::new(),
            tool_settings: ToolSettings::default(),
            generated_gcode: String::new(),
            gcode_generated: false,
            current_file_path: None,
            is_modified: false,
            design_name: "Untitled".to_string(),
            show_grid: true,
            clipboard: Vec::new(),
            default_properties_shape: crate::canvas::DrawingObject::new(0, crate::shapes::Shape::Rectangle(crate::shapes::Rectangle::new(0.0, 0.0, 0.0, 0.0))),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Saves current state to history
    /// Pushes a command to the undo stack and executes it.
    pub fn push_command(&mut self, mut cmd: DesignerCommand) {
        cmd.apply(&mut self.canvas);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        // Limit stack size to 50
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.is_modified = true;
        self.gcode_generated = false;
    }

    /// Undo last change
    /// Undo last change
    pub fn undo(&mut self) {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(&mut self.canvas);
            self.redo_stack.push(cmd);
            self.gcode_generated = false;
            self.is_modified = true;
        }
    }

    /// Redo last undo
    /// Redo last undo
    pub fn redo(&mut self) {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.apply(&mut self.canvas);
            self.undo_stack.push(cmd);
            self.gcode_generated = false;
            self.is_modified = true;
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Check if grouping is possible (at least 2 items selected, and at least one is not already in a group)
    pub fn can_group(&self) -> bool {
        let selected: Vec<_> = self.canvas.shapes().filter(|s| s.selected).collect();
        if selected.len() < 2 {
            return false;
        }
        // "activate if there are selected items that do not have groupids"
        // Interpreted as: at least one selected item is not in a group.
        // If all are already grouped, maybe we shouldn't group them again?
        // Or maybe we can merge groups?
        // For now, let's follow the prompt's implication:
        selected.iter().any(|s| s.group_id.is_none())
    }

    /// Check if ungrouping is possible (any selected item is in a group)
    pub fn can_ungroup(&self) -> bool {
        self.canvas.shapes().filter(|s| s.selected).any(|s| s.group_id.is_some())
    }

    /// Clear history stacks
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Toggle grid visibility
    pub fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
    }

    /// Sets the drawing mode.
    pub fn set_mode(&mut self, mode: i32) {
        let drawing_mode = match mode {
            0 => DrawingMode::Select,
            1 => DrawingMode::Rectangle,
            2 => DrawingMode::Circle,
            3 => DrawingMode::Line,
            4 => DrawingMode::Ellipse,
            5 => DrawingMode::Polyline,
            6 => DrawingMode::Text,
            _ => DrawingMode::Select,
        };
        self.canvas.set_mode(drawing_mode);
    }

    /// Zooms in on the canvas.
    pub fn zoom_in(&mut self) {
        let current = self.canvas.zoom();
        let new_zoom = (current * 1.2).min(50.0);
        self.canvas.set_zoom(new_zoom);
    }

    /// Zooms out on the canvas.
    pub fn zoom_out(&mut self) {
        let current = self.canvas.zoom();
        let new_zoom = (current / 1.2).max(0.1);
        self.canvas.set_zoom(new_zoom);
    }

    /// Zoom to fit all shapes.
    pub fn zoom_fit(&mut self) {
        self.canvas.fit_all_shapes();
    }

    /// Reset view to default (origin at bottom-left with padding)
    pub fn reset_view(&mut self) {
        // Reset zoom to 100%
        self.canvas.set_zoom(1.0);

        // Reset pan to place origin at bottom-left with 5px padding
        // We need to access the viewport to set this up correctly
        // Since we don't have direct access to viewport dimensions here easily without passing them,
        // we'll rely on the viewport's internal size which should be updated by update_designer_ui
        let _height = self.canvas.viewport().canvas_height();

        // In screen coordinates, (0, height) is bottom-left.
        // We want world (0,0) to be at screen (5, height-5).
        // world_to_screen(0,0) = (pan_x, pan_y) usually (depending on implementation)
        // Let's assume standard: screen_x = world_x * zoom + pan_x
        // screen_y = height - (world_y * zoom + pan_y)  <-- typical for Y-up world, Y-down screen
        // If we want screen_x = 5, screen_y = height - 5 for world(0,0):
        // 5 = 0 * 1.0 + pan_x  => pan_x = 5
        // height - 5 = height - (0 * 1.0 + pan_y) => 5 = pan_y

        // So we set pan to (5, 5)
        self.canvas.set_pan(5.0, 5.0);
    }

    /// Deletes the selected shape(s).
    pub fn delete_selected(&mut self) {
        let ids: Vec<u64> = self.canvas.shapes().filter(|s| s.selected).map(|s| s.id).collect();
        if ids.is_empty() {
            return;
        }
        
        let mut commands = Vec::new();
        for id in ids {
            commands.push(DesignerCommand::RemoveShape(RemoveShape { id, object: None }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Delete Shapes".to_string(),
        });
        self.push_command(cmd);
    }

    /// Get number of selected shapes
    pub fn selected_count(&self) -> usize {
        self.canvas.selected_count()
    }

    /// Copies selected shapes to clipboard
    pub fn copy_selected(&mut self) {
        self.clipboard = self.canvas.shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();
    }

    /// Pastes shapes from clipboard at specified location
    pub fn paste_at_location(&mut self, x: f64, y: f64) {
        if self.clipboard.is_empty() {
            return;
        }

        // Calculate bounding box of clipboard items
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for obj in &self.clipboard {
            let (x1, y1, x2, y2) = obj.shape.bounding_box();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
        }

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        // Calculate offset to move center to (x, y)
        let dx = x - center_x;
        let dy = y - center_y;

        self.canvas.deselect_all();

        let mut new_ids = Vec::new();
        let mut pasted_objects = Vec::new();
        let mut group_map = std::collections::HashMap::new();

        for obj in &self.clipboard {
            let mut new_obj = obj.clone();
            let id = self.canvas.generate_id();
            new_obj.id = id;
            
            // Handle group ID mapping
            if let Some(gid) = obj.group_id {
                let new_gid = *group_map.entry(gid).or_insert_with(|| {
                    self.canvas.generate_id()
                });
                new_obj.group_id = Some(new_gid);
            }

            // Apply offset
            new_obj.shape.translate(dx, dy);
            new_obj.selected = true;
            
            new_ids.push(id);
            pasted_objects.push(Some(new_obj));
        }
        
        // Update selected_id to the last pasted object
        if let Some(last_id) = new_ids.last() {
            self.canvas.set_selected_id(Some(*last_id));
        }

        let cmd = DesignerCommand::PasteShapes(PasteShapes {
            ids: new_ids,
            objects: pasted_objects,
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their left edges
    pub fn align_selected_horizontal_left(&mut self) {
        let deltas = self.canvas.calculate_alignment_deltas(crate::canvas::Alignment::Left);
        if deltas.is_empty() { return; }
        
        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes { ids: vec![id], dx, dy }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Left".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their horizontal centers
    pub fn align_selected_horizontal_center(&mut self) {
        let deltas = self.canvas.calculate_alignment_deltas(crate::canvas::Alignment::CenterHorizontal);
        if deltas.is_empty() { return; }
        
        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes { ids: vec![id], dx, dy }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Horizontal Center".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their right edges
    pub fn align_selected_horizontal_right(&mut self) {
        let deltas = self.canvas.calculate_alignment_deltas(crate::canvas::Alignment::Right);
        if deltas.is_empty() { return; }
        
        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes { ids: vec![id], dx, dy }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Right".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their top edges
    pub fn align_selected_vertical_top(&mut self) {
        let deltas = self.canvas.calculate_alignment_deltas(crate::canvas::Alignment::Top);
        if deltas.is_empty() { return; }
        
        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes { ids: vec![id], dx, dy }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Top".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their vertical centers
    pub fn align_selected_vertical_center(&mut self) {
        let deltas = self.canvas.calculate_alignment_deltas(crate::canvas::Alignment::CenterVertical);
        if deltas.is_empty() { return; }
        
        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes { ids: vec![id], dx, dy }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Vertical Center".to_string(),
        });
        self.push_command(cmd);
    }

    /// Align selected shapes by their bottom edges
    pub fn align_selected_vertical_bottom(&mut self) {
        let deltas = self.canvas.calculate_alignment_deltas(crate::canvas::Alignment::Bottom);
        if deltas.is_empty() { return; }
        
        let mut commands = Vec::new();
        for (id, dx, dy) in deltas {
            commands.push(DesignerCommand::MoveShapes(MoveShapes { ids: vec![id], dx, dy }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Align Bottom".to_string(),
        });
        self.push_command(cmd);
    }

    /// Clears all shapes from the canvas.
    pub fn clear_canvas(&mut self) {
        if self.canvas.shape_count() > 0 {
            let ids: Vec<u64> = self.canvas.shapes().map(|s| s.id).collect();
            let mut commands = Vec::new();
            for id in ids {
                commands.push(DesignerCommand::RemoveShape(RemoveShape { id, object: None }));
            }
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Clear Canvas".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Generates G-code from the current design.
    pub fn generate_gcode(&mut self) -> String {
        let mut gcode = String::new();
        let gcode_gen = ToolpathToGcode::new(Units::MM, 10.0);
        let mut toolpaths = Vec::new();

        for shape in self.canvas.shapes() {
            // Set strategy for this shape
            self.toolpath_generator
                .set_pocket_strategy(shape.pocket_strategy);

            let shape_toolpaths = match &shape.shape {
                crate::shapes::Shape::Rectangle(rect) => {
                    if shape.operation_type == OperationType::Pocket {
                        self.toolpath_generator.generate_rectangle_pocket(
                            rect,
                            shape.pocket_depth,
                            shape.step_down as f64,
                            shape.step_in as f64,
                        )
                    } else {
                        vec![self.toolpath_generator.generate_rectangle_contour(rect)]
                    }
                }
                crate::shapes::Shape::Circle(circle) => {
                    if shape.operation_type == OperationType::Pocket {
                        self.toolpath_generator.generate_circle_pocket(
                            circle,
                            shape.pocket_depth,
                            shape.step_down as f64,
                            shape.step_in as f64,
                        )
                    } else {
                        vec![self.toolpath_generator.generate_circle_contour(circle)]
                    }
                }
                crate::shapes::Shape::Line(line) => {
                    vec![self.toolpath_generator.generate_line_contour(line)]
                }
                crate::shapes::Shape::Ellipse(ellipse) => {
                    // Ellipse contour generation might need a Circle conversion or specific handler
                    // The original code converted it to a Circle based on bounding box?
                    // "let radius = ((x2 - x1).abs().max((y2 - y1).abs())) / 2.0;"
                    // This seems to approximate ellipse as circle for toolpath?
                    // Let's keep original logic for now but use the ellipse data
                    let (x1, y1, x2, y2) = ellipse.bounding_box();
                    let cx = (x1 + x2) / 2.0;
                    let cy = (y1 + y2) / 2.0;
                    let radius = ((x2 - x1).abs().max((y2 - y1).abs())) / 2.0;
                    let circle = Circle::new(Point::new(cx, cy), radius);
                    vec![self.toolpath_generator.generate_circle_contour(&circle)]
                }
                crate::shapes::Shape::Path(path_shape) => {
                    if shape.operation_type == OperationType::Pocket {
                        self.toolpath_generator.generate_path_pocket(
                            path_shape,
                            shape.pocket_depth,
                            shape.step_down as f64,
                            shape.step_in as f64,
                        )
                    } else {
                        vec![self.toolpath_generator.generate_path_contour(path_shape)]
                    }
                }
                crate::shapes::Shape::Text(text) => {
                    vec![self.toolpath_generator.generate_text_toolpath(text)]
                }
            };
            toolpaths.extend(shape_toolpaths);
        }

        let total_length: f64 = toolpaths.iter().map(|tp| tp.total_length()).sum();

        // Use settings from first toolpath if available, or defaults
        let (header_speed, header_feed, header_diam, header_depth) =
            if let Some(first) = toolpaths.first() {
                let s = first
                    .segments
                    .first()
                    .map(|seg| seg.spindle_speed)
                    .unwrap_or(3000);
                let f = first
                    .segments
                    .first()
                    .map(|seg| seg.feed_rate)
                    .unwrap_or(100.0);
                (s, f, first.tool_diameter, first.depth)
            } else {
                (3000, 100.0, 3.175, -5.0)
            };

        gcode.push_str(&gcode_gen.generate_header(
            header_speed,
            header_feed,
            header_diam,
            header_depth,
            total_length,
        ));

        let mut line_number = 10;
        for toolpath in toolpaths {
            gcode.push_str(&gcode_gen.generate_body(&toolpath, line_number));
            // Estimate line count increment (rough)
            line_number += (toolpath.segments.len() as u32) * 10;
        }

        gcode.push_str(&gcode_gen.generate_footer());

        self.generated_gcode = gcode.clone();
        self.gcode_generated = self.canvas.shape_count() > 0;
        gcode
    }

    /// Sets feed rate for toolpath generation.
    pub fn set_feed_rate(&mut self, rate: f64) {
        self.tool_settings.feed_rate = rate;
        self.toolpath_generator.set_feed_rate(rate);
    }

    /// Sets spindle speed for toolpath generation.
    pub fn set_spindle_speed(&mut self, speed: u32) {
        self.tool_settings.spindle_speed = speed;
        self.toolpath_generator.set_spindle_speed(speed);
    }

    /// Sets tool diameter for toolpath generation.
    pub fn set_tool_diameter(&mut self, diameter: f64) {
        self.tool_settings.tool_diameter = diameter;
        self.toolpath_generator.set_tool_diameter(diameter);
    }

    /// Sets cut depth for toolpath generation.
    pub fn set_cut_depth(&mut self, depth: f64) {
        self.tool_settings.cut_depth = depth;
        self.toolpath_generator.set_cut_depth(depth);
    }

    /// Sets step down for toolpath generation.
    pub fn set_step_down(&mut self, step: f64) {
        self.tool_settings.step_down = step;
        // self.toolpath_generator.set_step_down(step); // Assuming this exists or will be used
    }

    /// Adds a test rectangle to the canvas.
    pub fn add_test_rectangle(&mut self) {
        let id = self.canvas.generate_id();
        let rect = Rectangle::new(10.0, 10.0, 50.0, 40.0);
        let obj = DrawingObject::new(id, Shape::Rectangle(rect));
        let cmd = DesignerCommand::AddShape(AddShape { id, object: Some(obj) });
        self.push_command(cmd);
    }

    /// Adds a test circle to the canvas.
    pub fn add_test_circle(&mut self) {
        let id = self.canvas.generate_id();
        let circle = Circle::new(Point::new(75.0, 75.0), 20.0);
        let obj = DrawingObject::new(id, Shape::Circle(circle));
        let cmd = DesignerCommand::AddShape(AddShape { id, object: Some(obj) });
        self.push_command(cmd);
    }

    /// Adds a test line to the canvas.
    pub fn add_test_line(&mut self) {
        let id = self.canvas.generate_id();
        let line = Line::new(Point::new(10.0, 10.0), Point::new(100.0, 100.0));
        let obj = DrawingObject::new(id, Shape::Line(line));
        let cmd = DesignerCommand::AddShape(AddShape { id, object: Some(obj) });
        self.push_command(cmd);
    }

    /// Groups the selected shapes.
    pub fn group_selected(&mut self) {
        let ids: Vec<u64> = self.canvas.shapes().filter(|s| s.selected).map(|s| s.id).collect();
        if ids.len() < 2 {
            return;
        }
        
        let group_id = self.canvas.generate_id();
        let cmd = DesignerCommand::GroupShapes(GroupShapes { ids, group_id });
        self.push_command(cmd);
    }

    /// Ungroups the selected shapes.
    pub fn ungroup_selected(&mut self) {
        let mut group_map: std::collections::HashMap<u64, Vec<u64>> = std::collections::HashMap::new();
        for obj in self.canvas.shapes() {
            if obj.selected {
                if let Some(gid) = obj.group_id {
                    group_map.entry(gid).or_default().push(obj.id);
                }
            }
        }
        
        if group_map.is_empty() {
            return;
        }
        
        let mut commands = Vec::new();
        for (gid, ids) in group_map {
            commands.push(DesignerCommand::UngroupShapes(UngroupShapes { ids, group_id: gid }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Ungroup Shapes".to_string(),
        });
        self.push_command(cmd);
    }

    /// Adds a shape to the canvas at the specified position based on current mode.
    pub fn add_shape_at(&mut self, x: f64, y: f64, multi_select: bool) {
        match self.canvas.mode() {
            DrawingMode::Select => {
                // Select mode - just select shape at position
                self.canvas.select_at(&Point::new(x, y), multi_select);
            }
            DrawingMode::Rectangle => {
                let id = self.canvas.generate_id();
                let rect = Rectangle::new(x, y, 60.0, 40.0);
                let obj = DrawingObject::new(id, Shape::Rectangle(rect));
                let cmd = DesignerCommand::AddShape(AddShape { id, object: Some(obj) });
                self.push_command(cmd);
            }
            DrawingMode::Circle => {
                let id = self.canvas.generate_id();
                let circle = Circle::new(Point::new(x, y), 25.0);
                let obj = DrawingObject::new(id, Shape::Circle(circle));
                let cmd = DesignerCommand::AddShape(AddShape { id, object: Some(obj) });
                self.push_command(cmd);
            }
            DrawingMode::Line => {
                let id = self.canvas.generate_id();
                let line = Line::new(Point::new(x, y), Point::new(x + 50.0, y));
                let obj = DrawingObject::new(id, Shape::Line(line));
                let cmd = DesignerCommand::AddShape(AddShape { id, object: Some(obj) });
                self.push_command(cmd);
            }
            DrawingMode::Ellipse => {
                let id = self.canvas.generate_id();
                let ellipse = Ellipse::new(Point::new(x, y), 40.0, 25.0);
                let obj = DrawingObject::new(id, Shape::Ellipse(ellipse));
                let cmd = DesignerCommand::AddShape(AddShape { id, object: Some(obj) });
                self.push_command(cmd);
            }
            DrawingMode::Polyline => {
                let id = self.canvas.generate_id();
                let center = Point::new(x, y);
                let radius = 30.0;
                let sides = 6;
                let mut vertices = Vec::with_capacity(sides);
                for i in 0..sides {
                    let angle = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
                    let vx = center.x + radius * angle.cos();
                    let vy = center.y + radius * angle.sin();
                    vertices.push(Point::new(vx, vy));
                }
                let path_shape = PathShape::from_points(&vertices, true);
                let obj = DrawingObject::new(id, Shape::Path(path_shape));
                let cmd = DesignerCommand::AddShape(AddShape { id, object: Some(obj) });
                self.push_command(cmd);
                // I'll check canvas.rs add_polyline.
            }
            DrawingMode::Text => {
                let id = self.canvas.generate_id();
                let text = TextShape::new("Text".to_string(), x, y, 20.0);
                let obj = DrawingObject::new(id, Shape::Text(text));
                let cmd = DesignerCommand::AddShape(AddShape { id, object: Some(obj) });
                self.push_command(cmd);
            }
        }
    }

    /// Selects shapes within the given rectangle.
    pub fn select_in_rect(&mut self, x: f64, y: f64, width: f64, height: f64, multi_select: bool) {
        if self.canvas.mode() == DrawingMode::Select {
            self.canvas.select_in_rect(x, y, width, height, multi_select);
        }
    }

    /// Moves the selected shape by (dx, dy).
    pub fn move_selected(&mut self, dx: f64, dy: f64) {
        let ids: Vec<u64> = self.canvas.shapes().filter(|s| s.selected).map(|s| s.id).collect();
        if ids.is_empty() {
            return;
        }
        
        let cmd = DesignerCommand::MoveShapes(MoveShapes { ids, dx, dy });
        self.push_command(cmd);
    }

    /// Resizes the selected shape via handle drag.
    /// handle: 0=TL, 1=TR, 2=BL, 3=BR, 4=Center (move)
    pub fn resize_selected(&mut self, handle: usize, dx: f64, dy: f64) {
        let ids: Vec<u64> = self.canvas.shapes().filter(|s| s.selected).map(|s| s.id).collect();
        if ids.is_empty() {
            return;
        }

        // Calculate bounding box of ALL selected shapes
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for id in &ids {
            if let Some(obj) = self.canvas.get_shape(*id) {
                let (x1, y1, x2, y2) = obj.shape.bounding_box();
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
            }
        }

        // If handle is 4 (move), we just translate all selected shapes
        if handle == 4 {
            self.move_selected(dx, dy);
            return;
        }

        // Calculate new bounding box based on handle movement
        let (new_min_x, new_min_y, new_max_x, new_max_y) = match handle {
            0 => (min_x + dx, min_y + dy, max_x, max_y), // Top-left
            1 => (min_x, min_y + dy, max_x + dx, max_y), // Top-right
            2 => (min_x + dx, min_y, max_x, max_y + dy), // Bottom-left
            3 => (min_x, min_y, max_x + dx, max_y + dy), // Bottom-right
            _ => (min_x, min_y, max_x, max_y),
        };

        let old_width = max_x - min_x;
        let old_height = max_y - min_y;
        let new_width = (new_max_x - new_min_x).abs();
        let new_height = (new_max_y - new_min_y).abs();

        // Calculate scale factors
        let sx = if old_width.abs() > 1e-6 { new_width / old_width } else { 1.0 };
        let sy = if old_height.abs() > 1e-6 { new_height / old_height } else { 1.0 };

        // Center of scaling
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;
        
        let new_center_x = (new_min_x + new_max_x) / 2.0;
        let new_center_y = (new_min_y + new_max_y) / 2.0;
        
        let t_dx = new_center_x - center_x;
        let t_dy = new_center_y - center_y;
        
        let mut commands = Vec::new();
        for id in ids {
            if let Some(obj) = self.canvas.get_shape(id) {
                let old_shape = obj.shape.clone();
                let mut new_shape = old_shape.clone();
                
                // Scale relative to the center of the SELECTION bounding box
                new_shape.scale(sx, sy, Point::new(center_x, center_y));
                
                // Translate to new center
                new_shape.translate(t_dx, t_dy);
                
                commands.push(DesignerCommand::ResizeShape(ResizeShape {
                    id,
                    handle,
                    dx,
                    dy,
                    old_shape: Some(old_shape),
                    new_shape: Some(new_shape),
                }));
            }
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Resize Shapes".to_string(),
        });
        self.push_command(cmd);
    }

    /// Snaps the selected shape to whole millimeters
    /// Snaps the selected shape to whole millimeters
    pub fn snap_selected_to_mm(&mut self) {
        let updates = self.canvas.calculate_snapped_shapes();
        if updates.is_empty() { return; }
        
        let mut commands = Vec::new();
        for (id, new_obj) in updates {
            let old_obj = self.canvas.get_shape(id).unwrap().clone();
            commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                id,
                old_state: old_obj,
                new_state: new_obj,
            }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Snap to Grid".to_string(),
        });
        self.push_command(cmd);
    }

    pub fn set_selected_use_custom_values(&mut self, use_custom: bool) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if obj.selected {
                let mut new_obj = obj.clone();
                new_obj.use_custom_values = use_custom;
                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id: obj.id,
                    old_state: obj.clone(),
                    new_state: new_obj,
                }));
            }
        }
        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Change Use Custom Values".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Deselects all shapes.
    pub fn deselect_all(&mut self) {
        self.canvas.deselect_all();
    }

    /// Selects all shapes.
    pub fn select_all(&mut self) {
        self.canvas.select_all();
    }

    pub fn set_selected_position_and_size(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.set_selected_position_and_size_with_flags(x, y, w, h, true, true);
    }

    /// Sets the position and size of the selected shape with flags for which properties to update.
    pub fn set_selected_position_and_size_with_flags(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        update_position: bool,
        update_size: bool,
    ) {
        let updates = self.canvas.calculate_position_and_size_updates(x, y, w, h, update_position, update_size);
        if updates.is_empty() { return; }
        
        let mut commands = Vec::new();
        for (id, new_obj) in updates {
            let old_obj = self.canvas.get_shape(id).unwrap().clone();
            commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                id,
                old_state: old_obj,
                new_state: new_obj,
            }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Resize/Move Shape".to_string(),
        });
        self.push_command(cmd);
    }

    /// Save design to file
    pub fn save_to_file(&mut self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        use crate::serialization::DesignFile;

        let mut design = DesignFile::new(&self.design_name);

        // Save viewport state
        design.viewport.zoom = self.canvas.zoom();
        design.viewport.pan_x = self.canvas.pan_x();
        design.viewport.pan_y = self.canvas.pan_y();

        // Save all shapes
        for obj in self.canvas.shapes() {
            design.shapes.push(DesignFile::from_drawing_object(obj));
        }

        // Save default properties
        design.default_properties = Some(DesignFile::from_drawing_object(&self.default_properties_shape));

        // Save to file
        design.save_to_file(&path)?;

        // Update state
        self.current_file_path = Some(path.as_ref().to_path_buf());
        self.is_modified = false;

        Ok(())
    }

    /// Load design from file
    pub fn load_from_file(&mut self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        use crate::serialization::DesignFile;

        let design = DesignFile::load_from_file(&path)?;

        // Clear existing shapes
        self.canvas.clear();

        // Restore viewport
        self.canvas.set_zoom(design.viewport.zoom);
        self.canvas
            .set_pan(design.viewport.pan_x, design.viewport.pan_y);

        // Restore shapes
        let mut next_id = 1;
        for shape_data in &design.shapes {
            if let Ok(obj) = DesignFile::to_drawing_object(shape_data, next_id) {
                self.canvas.add_shape(obj.shape);
                next_id += 1;
            }
        }

        // Restore default properties
        if let Some(default_props) = &design.default_properties {
            if let Ok(obj) = DesignFile::to_drawing_object(default_props, 0) {
                self.default_properties_shape = obj;
            }
        }

        // Update state
        self.design_name = design.metadata.name.clone();
        self.current_file_path = Some(path.as_ref().to_path_buf());
        self.is_modified = false;
        self.clear_history();

        Ok(())
    }

    /// Create new design (clear all)
    pub fn new_design(&mut self) {
        self.canvas.clear();
        self.generated_gcode.clear();
        self.gcode_generated = false;
        self.current_file_path = None;
        self.is_modified = false;
        self.design_name = "Untitled".to_string();
        self.clear_history();
    }

    /// Mark design as modified
    pub fn mark_modified(&mut self) {
        self.is_modified = true;
    }

    /// Get display name for the design
    pub fn display_name(&self) -> String {
        let name = if let Some(path) = &self.current_file_path {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&self.design_name)
        } else {
            &self.design_name
        };

        if self.is_modified {
            format!("{}*", name)
        } else {
            name.to_string()
        }
    }

    pub fn set_selected_pocket_properties(&mut self, is_pocket: bool, depth: f64) {
        let mut commands = Vec::new();
        let new_type = if is_pocket {
            OperationType::Pocket
        } else {
            OperationType::Profile
        };

        for obj in self.canvas.shapes().filter(|s| s.selected) {
            if obj.operation_type != new_type || (obj.pocket_depth - depth).abs() > f64::EPSILON {
                let mut new_obj = obj.clone();
                new_obj.operation_type = new_type;
                new_obj.pocket_depth = depth;
                
                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id: obj.id,
                    old_state: obj.clone(),
                    new_state: new_obj,
                }));
            }
        }
        
        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Change Pocket Properties".to_string(),
            });
            self.push_command(cmd);
        }
    }

    pub fn set_selected_step_down(&mut self, step_down: f64) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes().filter(|s| s.selected) {
            if (obj.step_down as f64 - step_down).abs() > f64::EPSILON {
                let mut new_obj = obj.clone();
                new_obj.step_down = step_down as f32;
                
                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id: obj.id,
                    old_state: obj.clone(),
                    new_state: new_obj,
                }));
            }
        }
        
        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Change Step Down".to_string(),
            });
            self.push_command(cmd);
        }
    }

    pub fn set_selected_step_in(&mut self, step_in: f64) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes().filter(|s| s.selected) {
            if (obj.step_in as f64 - step_in).abs() > f64::EPSILON {
                let mut new_obj = obj.clone();
                new_obj.step_in = step_in as f32;
                
                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id: obj.id,
                    old_state: obj.clone(),
                    new_state: new_obj,
                }));
            }
        }
        
        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Change Step In".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Sets the text properties of the selected shape.
    pub fn set_selected_text_properties(&mut self, content: String, font_size: f64) {
        let updates = self.canvas.calculate_text_property_updates(&content, font_size);
        if updates.is_empty() { return; }
        
        let mut commands = Vec::new();
        for (id, new_obj) in updates {
            let old_obj = self.canvas.get_shape(id).unwrap().clone();
            commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                id,
                old_state: old_obj,
                new_state: new_obj,
            }));
        }
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Change Text Properties".to_string(),
        });
        self.push_command(cmd);
    }

    pub fn set_selected_corner_radius(&mut self, radius: f64) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if obj.selected {
                if let crate::shapes::Shape::Rectangle(mut rect) = obj.shape {
                    let max_radius = rect.width.min(rect.height) / 2.0;
                    let new_radius = radius.min(max_radius).max(0.0);
                    
                    if (rect.corner_radius - new_radius).abs() > f64::EPSILON {
                        rect.corner_radius = new_radius;
                        let mut new_obj = obj.clone();
                        new_obj.shape = crate::shapes::Shape::Rectangle(rect);
                        
                        commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                            id: obj.id,
                            old_state: obj.clone(),
                            new_state: new_obj,
                        }));
                    }
                }
            }
        }
        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Change Corner Radius".to_string(),
            });
            self.push_command(cmd);
        }
    }

    pub fn set_selected_rotation(&mut self, rotation: f64) {
        let selected_count = self.selected_count();
        
        if selected_count > 1 {
            // Multiple selection: Rotate around group center
            // 'rotation' is treated as a delta because UI resets to 0
            let angle_delta = rotation;
            
            // Calculate group center using local bounding boxes (unrotated) to ensure stability
            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            let mut has_selection = false;
            
            for obj in self.canvas.shapes().filter(|s| s.selected) {
                let (x1, y1, x2, y2) = obj.shape.local_bounding_box();
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
                has_selection = true;
            }
            
            if !has_selection { return; }
            
            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;
            
            let mut commands = Vec::new();
            
            // We need to collect updates first to avoid borrowing issues if we were doing complex things,
            // but here we iterate mutably which is fine.
            for obj in self.canvas.shapes_mut() {
                if obj.selected {
                    let mut new_obj = obj.clone();
                    
                    // Calculate shape center using local bounding box (pivot point)
                    let (sx1, sy1, sx2, sy2) = obj.shape.local_bounding_box();
                    let shape_center_x = (sx1 + sx2) / 2.0;
                    let shape_center_y = (sy1 + sy2) / 2.0;
                    
                    // Calculate distance and angle from group center
                    let dx = shape_center_x - center_x;
                    let dy = shape_center_y - center_y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    let current_angle = dy.atan2(dx);
                    
                    // Calculate new angle
                    let angle_delta_rad = angle_delta.to_radians();
                    let new_angle = current_angle + angle_delta_rad;
                    
                    // Calculate new position
                    let new_center_x = center_x + distance * new_angle.cos();
                    let new_center_y = center_y + distance * new_angle.sin();
                    
                    // Translate shape to new position
                    let trans_x = new_center_x - shape_center_x;
                    let trans_y = new_center_y - shape_center_y;
                    new_obj.shape.translate(trans_x, trans_y);
                    
                    // Update shape rotation
                    match &mut new_obj.shape {
                        crate::shapes::Shape::Rectangle(s) => s.rotation += angle_delta,
                        crate::shapes::Shape::Circle(s) => s.rotation += angle_delta,
                        crate::shapes::Shape::Line(s) => s.rotation += angle_delta,
                        crate::shapes::Shape::Ellipse(s) => s.rotation += angle_delta,
                        crate::shapes::Shape::Path(s) => s.rotation += angle_delta,
                        crate::shapes::Shape::Text(s) => s.rotation += angle_delta,
                    }
                    
                    commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                        id: obj.id,
                        old_state: obj.clone(),
                        new_state: new_obj,
                    }));
                }
            }
            
            if !commands.is_empty() {
                let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                    commands,
                    name: "Rotate Selection".to_string(),
                });
                self.push_command(cmd);
            }
            
        } else {
            let mut commands = Vec::new();
            for obj in self.canvas.shapes_mut() {
                if obj.selected {
                    let mut new_obj = obj.clone();
                    match &mut new_obj.shape {
                        crate::shapes::Shape::Rectangle(s) => s.rotation = rotation,
                        crate::shapes::Shape::Circle(s) => s.rotation = rotation,
                        crate::shapes::Shape::Line(s) => s.rotation = rotation,
                        crate::shapes::Shape::Ellipse(s) => s.rotation = rotation,
                        crate::shapes::Shape::Path(s) => s.rotation = rotation,
                        crate::shapes::Shape::Text(s) => s.rotation = rotation,
                    }
                    
                    if (obj.shape.rotation() - rotation).abs() > f64::EPSILON {
                         commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                            id: obj.id,
                            old_state: obj.clone(),
                            new_state: new_obj.clone(),
                        }));
                        *obj = new_obj;
                    }
                }
            }
            
            if !commands.is_empty() {
                let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                    commands,
                    name: "Change Rotation".to_string(),
                });
                self.push_command(cmd);
            }
        }
    }

    pub fn set_selected_is_slot(&mut self, is_slot: bool) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if obj.selected {
                if let crate::shapes::Shape::Rectangle(mut rect) = obj.shape {
                    if rect.is_slot != is_slot {
                        rect.is_slot = is_slot;
                        if is_slot {
                            rect.corner_radius = rect.width.min(rect.height) / 2.0;
                        }
                        
                        let mut new_obj = obj.clone();
                        new_obj.shape = crate::shapes::Shape::Rectangle(rect);
                        
                        commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                            id: obj.id,
                            old_state: obj.clone(),
                            new_state: new_obj,
                        }));
                    }
                }
            }
        }
        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Change Is Slot".to_string(),
            });
            self.push_command(cmd);
        }
    }

    pub fn set_selected_name(&mut self, name: String) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if obj.selected {
                if obj.name != name {
                    let mut new_obj = obj.clone();
                    new_obj.name = name.clone();
                    
                    commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                        id: obj.id,
                        old_state: obj.clone(),
                        new_state: new_obj,
                    }));
                }
            }
        }
        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Change Name".to_string(),
            });
            self.push_command(cmd);
        }
    }

    pub fn select_next_shape(&mut self) {
        let selected_id = self.canvas.selected_id();
        let ids: Vec<u64> = self.canvas.shape_store.draw_order_iter().collect();
        
        if ids.is_empty() {
            return;
        }
        
        let new_id = if let Some(id) = selected_id {
            if let Some(pos) = ids.iter().position(|&x| x == id) {
                if pos + 1 < ids.len() {
                    ids[pos + 1]
                } else {
                    ids[ids.len() - 1]
                }
            } else {
                ids[0]
            }
        } else {
            ids[0]
        };
        
        self.canvas.select_shape(new_id, false);
    }

    pub fn select_previous_shape(&mut self) {
        let selected_id = self.canvas.selected_id();
        let ids: Vec<u64> = self.canvas.shape_store.draw_order_iter().collect();
        
        if ids.is_empty() {
            return;
        }
        
        let new_id = if let Some(id) = selected_id {
            if let Some(pos) = ids.iter().position(|&x| x == id) {
                if pos > 0 {
                    ids[pos - 1]
                } else {
                    ids[0]
                }
            } else {
                ids[0]
            }
        } else {
            ids[0]
        };
        
        self.canvas.select_shape(new_id, false);
    }

    pub fn set_selected_pocket_strategy(
        &mut self,
        strategy: crate::pocket_operations::PocketStrategy,
    ) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes().filter(|s| s.selected) {
            if obj.pocket_strategy != strategy {
                let mut new_obj = obj.clone();
                new_obj.pocket_strategy = strategy;
                
                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id: obj.id,
                    old_state: obj.clone(),
                    new_state: new_obj,
                }));
            }
        }
        
        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Change Pocket Strategy".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Converts selected shapes to a single bounding rectangle.
    pub fn convert_selected_to_rectangle(&mut self) {
        let selected: Vec<_> = self.canvas.shapes().filter(|s| s.selected).cloned().collect();
        if selected.is_empty() { return; }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for obj in &selected {
            let (x1, y1, x2, y2) = obj.shape.bounding_box();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
        }
        
        let rect = Rectangle::new(min_x, min_y, max_x - min_x, max_y - min_y);
        let new_id = self.canvas.generate_id();
        let mut new_obj = DrawingObject::new(new_id, Shape::Rectangle(rect));
        new_obj.selected = true;
        
        let mut commands = Vec::new();
        for obj in selected {
            commands.push(DesignerCommand::RemoveShape(RemoveShape { id: obj.id, object: Some(obj) }));
        }
        commands.push(DesignerCommand::AddShape(AddShape { id: new_id, object: Some(new_obj) }));
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Convert to Rectangle".to_string(),
        });
        self.push_command(cmd);
    }

    /// Converts selected shapes to a single path.
    pub fn convert_selected_to_path(&mut self) {
        let selected: Vec<_> = self.canvas.shapes().filter(|s| s.selected).cloned().collect();
        if selected.is_empty() { return; }

        let mut builder = lyon::path::Path::builder();
        
        for obj in &selected {
            let path_shape = obj.shape.to_path_shape();
            for event in path_shape.path.iter() {
                match event {
                    lyon::path::Event::Begin { at } => { builder.begin(at); }
                    lyon::path::Event::Line { from: _, to } => { builder.line_to(to); }
                    lyon::path::Event::Quadratic { from: _, ctrl, to } => { builder.quadratic_bezier_to(ctrl, to); }
                    lyon::path::Event::Cubic { from: _, ctrl1, ctrl2, to } => { builder.cubic_bezier_to(ctrl1, ctrl2, to); }
                    lyon::path::Event::End { last: _, first: _, close } => {
                        if close { builder.close(); } else { builder.end(false); }
                    }
                }
            }
        }
        
        let new_path = PathShape { path: builder.build(), rotation: 0.0 };
        let new_id = self.canvas.generate_id();
        let mut new_obj = DrawingObject::new(new_id, Shape::Path(new_path));
        new_obj.selected = true;
        
        let mut commands = Vec::new();
        for obj in selected {
            commands.push(DesignerCommand::RemoveShape(RemoveShape { id: obj.id, object: Some(obj) }));
        }
        commands.push(DesignerCommand::AddShape(AddShape { id: new_id, object: Some(new_obj) }));
        
        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Convert to Path".to_string(),
        });
        self.push_command(cmd);
    }
    /// Creates an array of copies for the selected shapes.
    pub fn create_array(&mut self, operation: crate::arrays::ArrayOperation) {
        let selected: Vec<_> = self.canvas.shapes().filter(|s| s.selected).cloned().collect();
        if selected.is_empty() { return; }

        let (is_circular, center) = if let crate::arrays::ArrayOperation::Circular(params) = &operation {
            (true, params.center)
        } else {
            (false, Point::new(0.0, 0.0))
        };

        let offsets = match crate::arrays::ArrayGenerator::generate(&operation) {
            Ok(offsets) => offsets,
            Err(e) => {
                eprintln!("Failed to generate array offsets: {}", e);
                return;
            }
        };

        let mut commands = Vec::new();
        // Create a single group ID for the entire array
        let array_group_id = self.canvas.generate_id();

        // Deselect original shapes (will be re-selected as part of the group)
        self.canvas.deselect_all();

        for obj in &selected {
            let (x1, y1, x2, y2) = obj.shape.bounding_box();
            let orig_x = (x1 + x2) / 2.0;
            let orig_y = (y1 + y2) / 2.0;
            
            for (i, (off_x, off_y)) in offsets.iter().enumerate() {
                let (dx, dy) = if is_circular {
                    // Circular: off_x, off_y are positions relative to center
                    let target_x = center.x + off_x;
                    let target_y = center.y + off_y;
                    (target_x - orig_x, target_y - orig_y)
                } else {
                    // Linear/Grid: off_x, off_y are deltas
                    (*off_x, *off_y)
                };

                if i == 0 {
                    // Modify original shape
                    let mut new_original = obj.clone();
                    new_original.group_id = Some(array_group_id);
                    new_original.selected = true;
                    new_original.shape.translate(dx, dy);
                    
                    // For circular arrays, rotate the shape to match the position angle
                    if is_circular {
                        if let crate::arrays::ArrayOperation::Circular(params) = &operation {
                            // Calculate angle of this copy
                            let angle_step = params.angle_step();
                            let angle_delta = if params.clockwise {
                                -(i as f64) * angle_step
                            } else {
                                (i as f64) * angle_step
                            };
                            
                            match &mut new_original.shape {
                                crate::shapes::Shape::Rectangle(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Circle(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Line(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Ellipse(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Path(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Text(s) => s.rotation += angle_delta,
                            }
                        }
                    }
                    
                    commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                        id: obj.id,
                        old_state: obj.clone(),
                        new_state: new_original,
                    }));
                } else {
                    // Create copy
                    let mut new_obj = obj.clone();
                    let id = self.canvas.generate_id();
                    new_obj.id = id;
                    new_obj.group_id = Some(array_group_id);
                    new_obj.selected = true;

                    new_obj.shape.translate(dx, dy);
                    
                    // For circular arrays, rotate the shape to match the position angle
                    if is_circular {
                        if let crate::arrays::ArrayOperation::Circular(params) = &operation {
                            // Calculate angle of this copy
                            let angle_step = params.angle_step();
                            let angle_delta = if params.clockwise {
                                -(i as f64) * angle_step
                            } else {
                                (i as f64) * angle_step
                            };
                            
                            match &mut new_obj.shape {
                                crate::shapes::Shape::Rectangle(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Circle(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Line(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Ellipse(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Path(s) => s.rotation += angle_delta,
                                crate::shapes::Shape::Text(s) => s.rotation += angle_delta,
                            }
                        }
                    }

                    commands.push(DesignerCommand::AddShape(AddShape { id, object: Some(new_obj) }));
                }
            }
        }
        
        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Create Array".to_string(),
            });
            self.push_command(cmd);
        }
    }
}

impl Default for DesignerState {
    fn default() -> Self {
        Self::new()
    }
}
