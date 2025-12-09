use crate::MainWindow;
use gcodekit5_core::units::{MeasurementSystem, to_display_string, get_unit_label};
use std::rc::Rc;

/// Update designer UI with current shapes from state
pub fn update_designer_ui(window: &MainWindow, state: &mut gcodekit5::DesignerState) {
    // Get measurement system from window property (which is set from settings)
    // Since we don't have direct access to settings here, we'll assume Metric for now
    // or try to read it from the window if possible.
    // Better approach: Pass system or settings to this function.
    // For now, let's default to Metric if we can't get it easily, but we should try to be consistent.
    // Actually, the window has `unit-label` property which we can read if we exposed it.
    // But `unit-label` is on the DesignerPanel, not MainWindow directly (it's nested).
    // Let's assume we can get it from the unit label string if it's "in" or "mm".
    
    let unit_label = window.get_designer_unit_label();
    let system = if unit_label == "in" {
        MeasurementSystem::Imperial
    } else {
        MeasurementSystem::Metric
    };

    // Get canvas dimensions from window
    let canvas_width = window.get_designer_canvas_width().max(100.0) as u32;
    let canvas_height = window.get_designer_canvas_height().max(100.0) as u32;

    // Update viewport canvas size to match actual rendering size
    state
        .canvas
        .viewport_mut()
        .set_canvas_size(canvas_width as f64, canvas_height as f64);

    // Render canvas using SVG paths
    let crosshair_data = gcodekit5::designer::svg_renderer::render_crosshair(
        &state.canvas,
        canvas_width,
        canvas_height,
    );
    let (grid_data, grid_size) = if state.show_grid {
        gcodekit5::designer::svg_renderer::render_grid(&state.canvas, canvas_width, canvas_height)
    } else {
        (String::new(), 0.0)
    };
    let origin_data = gcodekit5::designer::svg_renderer::render_origin(
        &state.canvas,
        canvas_width,
        canvas_height,
    );
    let shapes_data = gcodekit5::designer::svg_renderer::render_shapes(
        &state.canvas,
        canvas_width,
        canvas_height,
    );
    let grouped_shapes_data = gcodekit5::designer::svg_renderer::render_grouped_shapes(
        &state.canvas,
        canvas_width,
        canvas_height,
    );
    let group_bounding_box_data = gcodekit5::designer::svg_renderer::render_group_bounding_box(
        &state.canvas,
        canvas_width,
        canvas_height,
    );
    let selected_shapes_data = gcodekit5::designer::svg_renderer::render_selected_shapes(
        &state.canvas,
        canvas_width,
        canvas_height,
    );
    let handles_data = gcodekit5::designer::svg_renderer::render_selection_handles(
        &state.canvas,
        canvas_width,
        canvas_height,
    );

    // Update UI with SVG path data
    window.set_designer_canvas_crosshair_data(crosshair_data);
    window.set_designer_canvas_grid_data(grid_data);
    window.set_designer_canvas_origin_data(origin_data);
    window.set_designer_show_grid(state.show_grid);
    if grid_size > 0.0 {
        window.set_designer_grid_size(format!("{}{}", to_display_string(grid_size as f32, system), get_unit_label(system)));
    }
    window.set_designer_canvas_shapes_data(shapes_data);
    window.set_designer_canvas_grouped_shapes_data(grouped_shapes_data);
    window.set_designer_canvas_group_bounding_box_data(
        group_bounding_box_data,
    ));
    window.set_designer_canvas_selected_shapes_data(selected_shapes_data);
    window.set_designer_canvas_handles_data(handles_data);

    // Still update shapes array for metadata (could be used for debugging/info)
    let shapes: Vec<crate::DesignerShape> = state
        .canvas
        .shapes()
        .map(|obj| {
            let (x1, y1, x2, y2) = obj.shape.bounding_box();
            let shape_type = match obj.shape.shape_type() {
                gcodekit5::ShapeType::Rectangle => 0,
                gcodekit5::ShapeType::Circle => 1,
                gcodekit5::ShapeType::Line => 2,
                gcodekit5::ShapeType::Ellipse => 3,
                gcodekit5::ShapeType::Path => 4,
                gcodekit5::ShapeType::Text => 6,
            };
            let (corner_radius, is_slot) = if let gcodekit5::Shape::Rectangle(r) = &obj.shape {
                (r.corner_radius as f32, r.is_slot)
            } else {
                (0.0, false)
            };

            crate::DesignerShape {
                id: obj.id as i32,
                group_id: obj.group_id.map(|id| id as i32).unwrap_or(0),
                name: obj.name.clone(),
                x: x1 as f32,
                y: y1 as f32,
                width: (x2 - x1).abs() as f32,
                height: (y2 - y1).abs() as f32,
                radius: (((x2 - x1).abs() / 2.0).max((y2 - y1).abs() / 2.0)) as f32,
                corner_radius,
                is_slot,
                x2: x2 as f32,
                y2: y2 as f32,
                shape_type,
                selected: obj.selected,
                step_down: obj.step_down as f32,
                step_in: obj.step_in as f32,
                pocket_strategy: match obj.pocket_strategy {
                    gcodekit5::designer::pocket_operations::PocketStrategy::Raster { .. } => 0,
                    gcodekit5::designer::pocket_operations::PocketStrategy::ContourParallel => 1,
                    gcodekit5::designer::pocket_operations::PocketStrategy::Adaptive => 2,
                },
                raster_angle:
                    if let gcodekit5::designer::pocket_operations::PocketStrategy::Raster {
                        angle,
                        ..
                    } = obj.pocket_strategy
                    {
                        angle as f32
                    } else {
                        0.0
                    },
                bidirectional:
                    if let gcodekit5::designer::pocket_operations::PocketStrategy::Raster {
                        bidirectional,
                        ..
                    } = obj.pocket_strategy
                    {
                        bidirectional
                    } else {
                        true
                    },
                use_custom_values: obj.use_custom_values,
                rotation: 0.0,
            }
        })
        .collect();
    for _ in &shapes {}
    // Force UI to recognize the change by clearing first
    window.set_designer_shapes(Vec::new());
    window.set_designer_shapes(shapes.clone());

    // Update shape indicator with selected shape info
    let selected_count = state.canvas.selected_count();
    if selected_count > 1 {
        // Multiple selection - calculate bounding box of all selected shapes
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut mixed_types = false;
        let mut first_type = None;

        for obj in state.canvas.shapes().filter(|s| s.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounding_box();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);

            let shape_type = match obj.shape.shape_type() {
                gcodekit5::ShapeType::Rectangle => 0,
                gcodekit5::ShapeType::Circle => 1,
                gcodekit5::ShapeType::Line => 2,
                gcodekit5::ShapeType::Ellipse => 3,
                gcodekit5::ShapeType::Path => 4,
                gcodekit5::ShapeType::Text => 6,
            };

            if let Some(t) = first_type {
                if t != shape_type {
                    mixed_types = true;
                }
            } else {
                first_type = Some(shape_type);
            }
        }

        let width = (max_x - min_x).abs();
        let height = (max_y - min_y).abs();

        window.set_designer_selected_shape_x(to_display_string(min_x as f32, system).into());
        window.set_designer_selected_shape_y(to_display_string(min_y as f32, system).into());
        window.set_designer_selected_shape_w(to_display_string(width as f32, system).into());
        window.set_designer_selected_shape_h(to_display_string(height as f32, system).into());
        
        // Set type to mixed (-1) or the common type
        window.set_designer_selected_shape_type(if mixed_types { -1 } else { first_type.unwrap_or(0) });
        
        // Reset specific properties
        window.set_designer_selected_shape_radius(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_corner_radius(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_is_slot(false);
        window.set_designer_selected_shape_name("Multiple Selection".to_string());
        window.set_designer_selected_shape_text_content("".to_string());
        window.set_designer_selected_shape_font_size(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_rotation(to_display_string(0.0, system).into());
        
        // For CAM properties, we could show common values or defaults
        // For now, just show defaults/mixed
        window.set_designer_selected_shape_is_pocket(false);
        window.set_designer_selected_shape_pocket_depth(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_step_down(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_step_in(to_display_string(0.0, system).into());
        
    } else if let Some(id) = state.canvas.selected_id() {
        if let Some(obj) = state.canvas.shapes().find(|o| o.id == id) {
            let (x1, y1, x2, y2) = obj.shape.bounding_box();
            let width = (x2 - x1).abs();
            let height = (y2 - y1).abs();
            let radius = if let Some(c) = obj
                .shape
                .as_any()
                .downcast_ref::<gcodekit5::designer::shapes::Circle>()
            {
                c.radius
            } else {
                0.0
            };

            let shape_type = match obj.shape.shape_type() {
                gcodekit5::ShapeType::Rectangle => 0,
                gcodekit5::ShapeType::Circle => 1,
                gcodekit5::ShapeType::Line => 2,
                gcodekit5::ShapeType::Ellipse => 3,
                gcodekit5::ShapeType::Path => 4,
                gcodekit5::ShapeType::Text => 6,
            };

            window.set_designer_selected_shape_x(to_display_string(x1 as f32, system).into());
            window.set_designer_selected_shape_y(to_display_string(y1 as f32, system).into());
            window.set_designer_selected_shape_w(to_display_string(width as f32, system).into());
            window.set_designer_selected_shape_h(to_display_string(height as f32, system).into());
            window.set_designer_selected_shape_type(shape_type);
            window.set_designer_selected_shape_radius(to_display_string(radius as f32, system).into());
            window.set_designer_selected_shape_rotation(to_display_string(obj.shape.rotation() as f32, system).into());

            if let gcodekit5::Shape::Rectangle(r) = &obj.shape {
                window.set_designer_selected_shape_corner_radius(to_display_string(r.corner_radius as f32, system).into());
                window.set_designer_selected_shape_is_slot(r.is_slot);
            } else {
                window.set_designer_selected_shape_corner_radius(to_display_string(0.0, system).into());
                window.set_designer_selected_shape_is_slot(false);
            }

            let is_pocket =
                obj.operation_type == gcodekit5::designer::shapes::OperationType::Pocket;
            window.set_designer_selected_shape_is_pocket(is_pocket);
            window.set_designer_selected_shape_pocket_depth(to_display_string(obj.pocket_depth as f32, system).into());
            window.set_designer_selected_shape_step_down(to_display_string(obj.step_down as f32, system).into());
            window.set_designer_selected_shape_step_in(to_display_string(obj.step_in as f32, system).into());

            let (strategy_idx, angle, bidir) = match obj.pocket_strategy {
                gcodekit5::designer::pocket_operations::PocketStrategy::Raster {
                    angle,
                    bidirectional,
                } => (0, angle as f32, bidirectional),
                gcodekit5::designer::pocket_operations::PocketStrategy::ContourParallel => {
                    (1, 0.0, true)
                }
                gcodekit5::designer::pocket_operations::PocketStrategy::Adaptive => (2, 0.0, true),
            };
            window.set_designer_selected_shape_pocket_strategy(strategy_idx);
            window.set_designer_selected_shape_raster_angle(to_display_string(angle, system).into());
            window.set_designer_selected_shape_bidirectional(bidir);

            if let Some(text) = obj
                .shape
                .as_any()
                .downcast_ref::<gcodekit5::designer::shapes::TextShape>()
            {
                window.set_designer_selected_shape_text_content(
                    &text.text,
                ));
                window.set_designer_selected_shape_font_size(to_display_string(text.font_size as f32, system).into());
            } else {
                window.set_designer_selected_shape_text_content("".to_string());
                window.set_designer_selected_shape_font_size(to_display_string(12.0, system).into());
            }
            window.set_designer_selected_shape_name(obj.name.clone());
        }
    } else {
        window.set_designer_selected_shape_name("".to_string());
        
        // No selection - show default properties
        let obj = &state.default_properties_shape;
        
        // Set dummy values for transform (hidden in UI)
        window.set_designer_selected_shape_x(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_y(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_w(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_h(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_type(0);
        window.set_designer_selected_shape_radius(to_display_string(0.0, system).into());
        window.set_designer_selected_shape_rotation(to_display_string(0.0, system).into());

        let is_pocket = obj.operation_type == gcodekit5::designer::shapes::OperationType::Pocket;
        window.set_designer_selected_shape_is_pocket(is_pocket);
        window.set_designer_selected_shape_pocket_depth(to_display_string(obj.pocket_depth as f32, system).into());
        window.set_designer_selected_shape_step_down(to_display_string(obj.step_down as f32, system).into());
        window.set_designer_selected_shape_step_in(to_display_string(obj.step_in as f32, system).into());

        let (strategy_idx, angle, bidir) = match obj.pocket_strategy {
            gcodekit5::designer::pocket_operations::PocketStrategy::Raster {
                angle,
                bidirectional,
            } => (0, angle as f32, bidirectional),
            gcodekit5::designer::pocket_operations::PocketStrategy::ContourParallel => {
                (1, 0.0, true)
            }
            gcodekit5::designer::pocket_operations::PocketStrategy::Adaptive => (2, 0.0, true),
        };
        window.set_designer_selected_shape_pocket_strategy(strategy_idx);
        window.set_designer_selected_shape_raster_angle(to_display_string(angle, system).into());
        window.set_designer_selected_shape_bidirectional(bidir);
        
        window.set_designer_selected_shape_text_content("".to_string());
        window.set_designer_selected_shape_font_size(to_display_string(12.0, system).into());
    }

    // Update selection count for UI features (e.g., alignment menu)
    window.set_designer_selected_count(state.selected_count() as i32);

    // Increment update counter to force UI re-render
    let mut ui_state = window.get_designer_state();
    let counter = ui_state.update_counter + 1;
    ui_state.update_counter = counter;
    ui_state.can_undo = state.can_undo();
    ui_state.can_redo = state.can_redo();
    ui_state.can_group = state.can_group();
    ui_state.can_ungroup = state.can_ungroup();
    window.set_designer_state(ui_state);
}
