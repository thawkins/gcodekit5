//! Property setters for selected shapes in designer state.

use super::DesignerState;
use crate::canvas::DrawingObject;
use crate::commands::*;
use crate::model::{DesignerShape, ParametricPathSource, Shape};
use crate::shapes::OperationType;
use crate::{Point, Rectangle};

use csgrs::traits::CSG; // Para poder usar .transform()

impl DesignerState {
    /// Sets the use_custom_values flag for selected shapes.
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

    /// Sets the pocket properties of selected shapes.
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

    /// Sets the step down for selected shapes.
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

    /// Sets the step in for selected shapes.
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

    /// Sets the ramp angle for selected shapes.
    pub fn set_selected_ramp_angle(&mut self, ramp_angle: f64) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes().filter(|s| s.selected) {
            if (obj.ramp_angle as f64 - ramp_angle).abs() > f64::EPSILON {
                let mut new_obj = obj.clone();
                new_obj.ramp_angle = ramp_angle as f32;

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
                name: "Change Ramp Angle".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Sets the start depth for selected shapes.
    pub fn set_selected_start_depth(&mut self, start_depth: f64) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes().filter(|s| s.selected) {
            if (obj.start_depth - start_depth).abs() > f64::EPSILON {
                let mut new_obj = obj.clone();
                new_obj.start_depth = start_depth;

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
                name: "Change Start Depth".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Sets the Z offset for selected shapes.
    pub fn set_selected_z_offset(&mut self, z_offset: f64) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes().filter(|s| s.selected) {
            if (obj.z_offset - z_offset).abs() > f64::EPSILON {
                let mut new_obj = obj.clone();
                new_obj.z_offset = z_offset;

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
                name: "Change Z Offset".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Sets the text properties of the selected shape.
    pub fn set_selected_text_properties(&mut self, content: String, font_size: f64) {
        let updates = self
            .canvas
            .calculate_text_property_updates(&content, font_size);
        if updates.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        for (id, new_obj) in updates {
            if let Some(old_obj) = self.canvas.get_shape(id) {
                commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                    id,
                    old_state: old_obj.clone(),
                    new_state: new_obj,
                }));
            }
        }

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Change Text Properties".to_string(),
        });
        self.push_command(cmd);
    }

    /// Sets corner radius for supported selected shapes.
    pub fn set_selected_corner_radius(&mut self, radius: f64) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if obj.selected {
                match &obj.shape {
                    crate::model::Shape::Rectangle(rect) => {
                        let max_radius = rect.width.min(rect.height) / 2.0;
                        let new_radius = radius.clamp(0.0, max_radius);

                        if (rect.corner_radius - new_radius).abs() > f64::EPSILON {
                            let mut new_obj = obj.clone();
                            if let crate::model::Shape::Rectangle(new_rect) = &mut new_obj.shape {
                                new_rect.corner_radius = new_radius;
                            }

                            commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                                id: obj.id,
                                old_state: obj.clone(),
                                new_state: new_obj,
                            }));
                        }
                    }
                    crate::model::Shape::Path(_) => {
                        let new_radius = radius.max(0.0);

                        if (obj.fillet - new_radius).abs() > f64::EPSILON
                            || obj.chamfer.abs() > f64::EPSILON
                        {
                            let mut new_obj = obj.clone();
                            new_obj.fillet = new_radius;
                            // Corner radius and chamfer are mutually exclusive on path geometry.
                            new_obj.chamfer = 0.0;

                            commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                                id: obj.id,
                                old_state: obj.clone(),
                                new_state: new_obj,
                            }));
                        }
                    }
                    _ => {}
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

    /// Sets the rotation of selected shapes.
    pub fn set_selected_rotation(&mut self, rotation: f64) {
        let selected_count = self.selected_count();

        if selected_count > 1 {
            // Multiple selection: Rotate around group center
            let angle_delta = rotation;

            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            let mut has_selection = false;

            for obj in self.canvas.shapes().filter(|s| s.selected) {
                let (x1, y1, x2, y2) = obj.shape.bounds();
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
                has_selection = true;
            }

            if !has_selection {
                return;
            }

            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;

            let mut commands = Vec::new();

            for obj in self.canvas.shapes_mut() {
                if obj.selected {
                    let mut new_obj = obj.clone();

                    let (sx1, sy1, sx2, sy2) = obj.shape.bounds();
                    let shape_center_x = (sx1 + sx2) / 2.0;
                    let shape_center_y = (sy1 + sy2) / 2.0;

                    let dx = shape_center_x - center_x;
                    let dy = shape_center_y - center_y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    let current_angle = dy.atan2(dx);

                    let angle_delta_rad = angle_delta.to_radians();
                    let new_angle = current_angle + angle_delta_rad;

                    let new_center_x = center_x + distance * new_angle.cos();
                    let new_center_y = center_y + distance * new_angle.sin();

                    if let crate::model::Shape::Line(line) = &mut new_obj.shape {
                        line.rotate_about(angle_delta, shape_center_x, shape_center_y);
                    }

                    let trans_x = new_center_x - shape_center_x;
                    let trans_y = new_center_y - shape_center_y;
                    new_obj.shape.translate(trans_x, trans_y);

                    match &mut new_obj.shape {
                        crate::model::Shape::Path(s) => {
                            // Obtain real limits from DXF
                            let (x1, _, x2, _) = s.bounds();

                            // Calculate the actual CURRENT CENTER of the object
                            let current_center_x = (x1 + x2) / 2.0;

                            // The displacement (dx) is the difference between
                            // what the Inspector (center_x) asks for and where the center
                            let dx = center_x - current_center_x;

                            if dx.abs() > f64::EPSILON {
                                // Physically move all points that distance dx
                                let translation = nalgebra::Matrix4::new_translation(
                                    &nalgebra::Vector3::new(dx, 0.0, 0.0),
                                );
                                s.sketch = s.sketch.transform(&translation);
                            }
                            // Obtain the vertical limits
                            let (_, y1, _, y2) = s.bounds();

                            // Calculate the current VERTICAL CENTER
                            let current_center_y = (y1 + y2) / 2.0;

                            // Displacement (dy) is: Destination (center_y) - Origin (current center)
                            let dy = center_y - current_center_y;

                            if dy.abs() > f64::EPSILON {
                                // Move on the Y axis (second parameter of Vector3)
                                let translation = nalgebra::Matrix4::new_translation(
                                    &nalgebra::Vector3::new(0.0, dy, 0.0),
                                );
                                s.sketch = s.sketch.transform(&translation);
                            }
                            s.rotation = rotation;
                        }

                        crate::model::Shape::Rectangle(s) => s.rotation += angle_delta,
                        crate::model::Shape::Circle(s) => s.rotation += angle_delta,
                        crate::model::Shape::Line(s) => s.rotation = s.current_angle_degrees(),
                        crate::model::Shape::Ellipse(s) => s.rotation += angle_delta,
                        crate::model::Shape::Text(s) => s.rotation += angle_delta,
                        crate::model::Shape::Triangle(s) => s.rotation += angle_delta,
                        crate::model::Shape::Polygon(s) => s.rotation += angle_delta,
                        crate::model::Shape::Gear(s) => s.rotation += angle_delta,
                        crate::model::Shape::Sprocket(s) => s.rotation += angle_delta,
                        crate::model::Shape::RasterImage(s) => s.rotation += angle_delta,
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
                        crate::model::Shape::Rectangle(s) => s.rotation = rotation,
                        crate::model::Shape::Circle(s) => s.rotation = rotation,
                        crate::model::Shape::Line(s) => {
                            let (sx1, sy1, sx2, sy2) = obj.shape.bounds();
                            let cx = (sx1 + sx2) / 2.0;
                            let cy = (sy1 + sy2) / 2.0;
                            let current = s.current_angle_degrees();
                            let delta = rotation - current;
                            s.rotate_about(delta, cx, cy);
                            s.rotation = rotation;
                        }
                        crate::model::Shape::Ellipse(s) => s.rotation = rotation,
                        crate::model::Shape::Path(s) => s.rotation = rotation,
                        crate::model::Shape::Text(s) => s.rotation = rotation,
                        crate::model::Shape::Triangle(s) => s.rotation = rotation,
                        crate::model::Shape::Polygon(s) => s.rotation = rotation,
                        crate::model::Shape::Gear(s) => s.rotation = rotation,
                        crate::model::Shape::Sprocket(s) => s.rotation = rotation,
                        crate::model::Shape::RasterImage(s) => s.rotation = rotation,
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

    /// Sets the is_slot flag for selected rectangles.
    pub fn set_selected_is_slot(&mut self, is_slot: bool) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if obj.selected {
                if let crate::model::Shape::Rectangle(rect) = &obj.shape {
                    if rect.is_slot != is_slot {
                        let mut new_obj = obj.clone();
                        if let crate::model::Shape::Rectangle(new_rect) = &mut new_obj.shape {
                            new_rect.is_slot = is_slot;
                        }

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

    /// Sets the name of selected shapes.
    pub fn set_selected_name(&mut self, name: String) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if obj.selected && obj.name != name {
                let mut new_obj = obj.clone();
                new_obj.name = name.clone();

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
                name: "Change Name".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Sets gear properties for selected gear shapes.
    pub fn set_selected_gear_properties(&mut self, module: f64, teeth: usize, pressure_angle: f64) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if obj.selected {
                if let crate::model::Shape::Gear(gear) = &obj.shape {
                    if (gear.module - module).abs() > f64::EPSILON
                        || gear.teeth != teeth
                        || (gear.pressure_angle_deg - pressure_angle).abs() > f64::EPSILON
                    {
                        let mut new_obj = obj.clone();
                        if let crate::model::Shape::Gear(new_gear) = &mut new_obj.shape {
                            new_gear.module = module;
                            new_gear.teeth = teeth;
                            new_gear.pressure_angle_deg = pressure_angle;
                        }

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
                name: "Change Gear Properties".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Sets sprocket properties for selected sprocket shapes.
    pub fn set_selected_sprocket_properties(
        &mut self,
        pitch: f64,
        teeth: usize,
        roller_diameter: f64,
    ) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if obj.selected {
                if let crate::model::Shape::Sprocket(sprocket) = &obj.shape {
                    if (sprocket.pitch - pitch).abs() > f64::EPSILON
                        || sprocket.teeth != teeth
                        || (sprocket.roller_diameter - roller_diameter).abs() > f64::EPSILON
                    {
                        let mut new_obj = obj.clone();
                        if let crate::model::Shape::Sprocket(new_sprocket) = &mut new_obj.shape {
                            new_sprocket.pitch = pitch;
                            new_sprocket.teeth = teeth;
                            new_sprocket.roller_diameter = roller_diameter;
                        }

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
                name: "Change Sprocket Properties".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Sets timing pulley properties for selected parametric path shapes.
    pub fn set_selected_timing_pulley_properties(
        &mut self,
        pitch: f64,
        teeth: usize,
        belt_width: f64,
        hole_radius: f64,
    ) {
        let mut commands = Vec::new();
        for obj in self.canvas.shapes_mut() {
            if !obj.selected {
                continue;
            }

            let Shape::Path(path) = &obj.shape else {
                continue;
            };

            let Some(ParametricPathSource::TimingPulley {
                pitch: old_pitch,
                teeth: old_teeth,
                belt_width: old_belt_width,
                hole_radius: old_hole_radius,
            }) = &path.parametric_source
            else {
                continue;
            };

            if (old_pitch - pitch).abs() <= f64::EPSILON
                && *old_teeth == teeth
                && (old_belt_width - belt_width).abs() <= f64::EPSILON
                && (old_hole_radius - hole_radius).abs() <= f64::EPSILON
            {
                continue;
            }

            let (x1, y1, x2, y2) = path.bounds();
            let center = crate::model::Point::new((x1 + x2) * 0.5, (y1 + y2) * 0.5);
            let regenerated = crate::parametric_shapes::generate_timing_pulley(
                center,
                pitch,
                teeth,
                belt_width,
                hole_radius,
            );

            let mut new_path = crate::model::DesignPath::from_lyon_path(&regenerated);
            new_path.rotation = path.rotation;
            new_path.closed = path.closed;
            new_path.lock_aspect_ratio = path.lock_aspect_ratio;
            new_path.laser_params = path.laser_params;
            new_path.parametric_source = Some(ParametricPathSource::TimingPulley {
                pitch,
                teeth,
                belt_width,
                hole_radius,
            });

            let mut new_obj = obj.clone();
            new_obj.shape = Shape::Path(new_path);

            commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                id: obj.id,
                old_state: obj.clone(),
                new_state: new_obj,
            }));
        }

        if !commands.is_empty() {
            let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
                commands,
                name: "Change Timing Pulley Properties".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Sets the pocket strategy for selected shapes.
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

    /// Sets the raster fill ratio for selected shapes.
    pub fn set_selected_raster_fill_ratio(&mut self, ratio: f64) {
        let clamped = ratio.clamp(0.0, 1.0);
        let mut commands = Vec::new();
        for obj in self.canvas.shapes().filter(|s| s.selected) {
            if (obj.raster_fill_ratio - clamped).abs() > f64::EPSILON {
                let mut new_obj = obj.clone();
                new_obj.raster_fill_ratio = clamped;

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
                name: "Change Raster Fill".to_string(),
            });
            self.push_command(cmd);
        }
    }

    /// Converts selected shapes to a single bounding rectangle.
    pub fn convert_selected_to_rectangle(&mut self) {
        let selected: Vec<_> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();
        if selected.is_empty() {
            return;
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for obj in &selected {
            let (x1, y1, x2, y2) = obj.shape.bounds();
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
            commands.push(DesignerCommand::RemoveShape(RemoveShape {
                id: obj.id,
                object: Some(obj),
            }));
        }
        commands.push(DesignerCommand::AddShape(AddShape {
            id: new_id,
            object: Some(new_obj),
        }));

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Convert to Rectangle".to_string(),
        });
        self.push_command(cmd);
    }

    /// Converts selected shapes to a single path.
    pub fn convert_selected_to_path(&mut self) {
        let selected: Vec<_> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();
        if selected.is_empty() {
            return;
        }

        let mut builder = lyon::path::Path::builder();

        for obj in &selected {
            let path_shape = obj.shape.to_path_shape();
            for event in path_shape.render().iter() {
                match event {
                    lyon::path::Event::Begin { at } => {
                        builder.begin(at);
                    }
                    lyon::path::Event::Line { from: _, to } => {
                        builder.line_to(to);
                    }
                    lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                        builder.quadratic_bezier_to(ctrl, to);
                    }
                    lyon::path::Event::Cubic {
                        from: _,
                        ctrl1,
                        ctrl2,
                        to,
                    } => {
                        builder.cubic_bezier_to(ctrl1, ctrl2, to);
                    }
                    lyon::path::Event::End {
                        first: _,
                        last: _,
                        close,
                    } => {
                        builder.end(close);
                    }
                }
            }
        }

        let path = builder.build();
        let new_path = crate::model::DesignPath::from_lyon_path(&path);
        let new_id = self.canvas.generate_id();
        let mut new_obj = DrawingObject::new(new_id, Shape::Path(new_path));
        new_obj.selected = true;

        let mut commands = Vec::new();
        for obj in selected {
            commands.push(DesignerCommand::RemoveShape(RemoveShape {
                id: obj.id,
                object: Some(obj),
            }));
        }
        commands.push(DesignerCommand::AddShape(AddShape {
            id: new_id,
            object: Some(new_obj),
        }));

        let cmd = DesignerCommand::CompositeCommand(CompositeCommand {
            commands,
            name: "Convert to Path".to_string(),
        });
        self.push_command(cmd);
    }

    /// Creates an array of copies for the selected shapes.
    pub fn create_array(&mut self, operation: crate::arrays::ArrayOperation) {
        use tracing::error;

        let selected: Vec<_> = self
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .cloned()
            .collect();
        if selected.is_empty() {
            return;
        }

        let (is_circular, center) =
            if let crate::arrays::ArrayOperation::Circular(params) = &operation {
                (true, params.center)
            } else {
                (false, Point::new(0.0, 0.0))
            };

        let offsets = match crate::arrays::ArrayGenerator::generate(&operation) {
            Ok(offsets) => offsets,
            Err(e) => {
                error!("Failed to generate array offsets: {}", e);
                return;
            }
        };

        let mut commands = Vec::new();
        let array_group_id = self.canvas.generate_id();

        self.canvas.deselect_all();

        for obj in &selected {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            let orig_x = (x1 + x2) / 2.0;
            let orig_y = (y1 + y2) / 2.0;

            for (i, (off_x, off_y)) in offsets.iter().enumerate() {
                let (dx, dy) = if is_circular {
                    let target_x = center.x + off_x;
                    let target_y = center.y + off_y;
                    (target_x - orig_x, target_y - orig_y)
                } else {
                    (*off_x, *off_y)
                };

                if i == 0 {
                    let mut new_original = obj.clone();
                    new_original.group_id = Some(array_group_id);
                    new_original.selected = true;
                    new_original.shape.translate(dx, dy);

                    if is_circular {
                        if let crate::arrays::ArrayOperation::Circular(params) = &operation {
                            let angle_step = params.angle_step();
                            let angle_delta = if params.clockwise {
                                -(i as f64) * angle_step
                            } else {
                                (i as f64) * angle_step
                            };

                            Self::apply_rotation_delta(&mut new_original.shape, angle_delta);
                        }
                    }

                    commands.push(DesignerCommand::ChangeProperty(ChangeProperty {
                        id: obj.id,
                        old_state: obj.clone(),
                        new_state: new_original,
                    }));
                } else {
                    let mut new_obj = obj.clone();
                    let id = self.canvas.generate_id();
                    new_obj.id = id;
                    new_obj.group_id = Some(array_group_id);
                    new_obj.selected = true;

                    new_obj.shape.translate(dx, dy);

                    if is_circular {
                        if let crate::arrays::ArrayOperation::Circular(params) = &operation {
                            let angle_step = params.angle_step();
                            let angle_delta = if params.clockwise {
                                -(i as f64) * angle_step
                            } else {
                                (i as f64) * angle_step
                            };

                            Self::apply_rotation_delta(&mut new_obj.shape, angle_delta);
                        }
                    }

                    commands.push(DesignerCommand::AddShape(AddShape {
                        id,
                        object: Some(new_obj),
                    }));
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

    /// Helper to apply rotation delta to a shape.
    fn apply_rotation_delta(shape: &mut Shape, angle_delta: f64) {
        match shape {
            crate::model::Shape::Rectangle(s) => s.rotation += angle_delta,
            crate::model::Shape::Circle(s) => s.rotation += angle_delta,
            crate::model::Shape::Line(s) => s.rotation += angle_delta,
            crate::model::Shape::Ellipse(s) => s.rotation += angle_delta,
            crate::model::Shape::Path(s) => s.rotation += angle_delta,
            crate::model::Shape::Text(s) => s.rotation += angle_delta,
            crate::model::Shape::Triangle(s) => s.rotation += angle_delta,
            crate::model::Shape::Polygon(s) => s.rotation += angle_delta,
            crate::model::Shape::Gear(s) => s.rotation += angle_delta,
            crate::model::Shape::Sprocket(s) => s.rotation += angle_delta,
            crate::model::Shape::RasterImage(s) => s.rotation += angle_delta,
        }
    }

    pub fn set_selected_x(&mut self, new_x: f64) {
        for obj in self.canvas.shapes_mut().filter(|s| s.selected) {
            let (current_x, _, _, _) = obj.shape.bounds();
            let dx = new_x - current_x;

            if dx.abs() > f64::EPSILON {
                match &mut obj.shape {
                    crate::model::Shape::Path(s) => {
                        // For imported objects: We move the physical points
                        let translation = nalgebra::Matrix4::new_translation(
                            &nalgebra::Vector3::new(dx, 0.0, 0.0),
                        );
                        s.sketch = s.sketch.transform(&translation);
                    }
                    _ => {
                        // For native objects: We use translate with the exact jump (0.0 on Y so that it doesn't jump)
                        obj.shape.translate(dx, 0.0);
                    }
                }
            }
        }
    }

    pub fn set_selected_y(&mut self, new_y: f64) {
        for obj in self.canvas.shapes_mut().filter(|s| s.selected) {
            let (_, current_y, _, _) = obj.shape.bounds();
            let dy = new_y - current_y;

            if dy.abs() > f64::EPSILON {
                match &mut obj.shape {
                    crate::model::Shape::Path(s) => {
                        let translation = nalgebra::Matrix4::new_translation(
                            &nalgebra::Vector3::new(0.0, dy, 0.0),
                        );
                        s.sketch = s.sketch.transform(&translation);
                    }
                    _ => {
                        obj.shape.translate(0.0, dy);
                    }
                }
            }
        }
    }

    pub fn set_selected_width(&mut self, width: f64) {
        for obj in self.canvas.shapes_mut().filter(|s| s.selected) {
            match &mut obj.shape {
                crate::model::Shape::Rectangle(r) => {
                    r.width = width;
                }
                crate::model::Shape::Ellipse(e) => {
                    e.rx = width / 2.0;
                }
                crate::model::Shape::Triangle(t) => {
                    t.width = width;
                }
                _ => {
                    let (x1, y1, x2, y2) = obj.shape.bounds();
                    let current_width = x2 - x1;
                    if current_width > 0.0 {
                        let scale_factor = width / current_width;
                        let center = crate::model::Point::new((x1 + x2) / 2.0, (y1 + y2) / 2.0);
                        obj.shape.scale(scale_factor, 1.0, center);
                    }
                }
            }
        }
    }

    pub fn set_selected_height(&mut self, height: f64) {
        for obj in self.canvas.shapes_mut().filter(|s| s.selected) {
            match &mut obj.shape {
                crate::model::Shape::Rectangle(r) => {
                    r.height = height;
                }
                crate::model::Shape::Ellipse(e) => {
                    e.ry = height / 2.0;
                }
                crate::model::Shape::Triangle(t) => {
                    t.height = height;
                }
                _ => {
                    let (x1, y1, x2, y2) = obj.shape.bounds();
                    let current_height = y2 - y1;
                    if current_height > 0.0 {
                        let scale_factor = height / current_height;
                        let center = crate::model::Point::new((x1 + x2) / 2.0, (y1 + y2) / 2.0);
                        obj.shape.scale(1.0, scale_factor, center);
                    }
                }
            }
        }
    }
}
