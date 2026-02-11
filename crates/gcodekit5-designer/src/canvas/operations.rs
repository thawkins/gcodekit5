//! Shape manipulation operations for Canvas.

use super::types::{Alignment, DrawingObject};
use crate::model::{
    DesignCircle as Circle, DesignEllipse as Ellipse, DesignLine as Line, DesignPath as PathShape,
    DesignPolygon as Polygon, DesignRectangle as Rectangle, DesignText as TextShape,
    DesignTriangle as Triangle, DesignerShape, Point, Shape, ShapeType,
};
use crate::spatial_index::Bounds;

use super::Canvas;

impl Canvas {
    /// Moves the selected shape by (dx, dy).
    pub fn move_selected(&mut self, dx: f64, dy: f64) {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if obj.selected {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();

                obj.shape.translate(dx, dy);

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }
    }

    /// Updates the geometry modifiers for a shape.
    pub fn update_shape_geometry(&mut self, id: u64, offset: f64, fillet: f64, chamfer: f64) {
        if let Some(obj) = self.shape_store.get_mut(id) {
            let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();
            let old_bounds = Bounds::new(old_x1, old_y1, old_x2, old_y2);

            obj.offset = offset;
            obj.fillet = fillet;
            obj.chamfer = chamfer;

            let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
            let new_bounds = Bounds::new(new_x1, new_y1, new_x2, new_y2);

            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }
    }

    /// Calculates the deltas (dx, dy) required to align each selected shape according to the specified alignment.
    /// Returns a vector of (shape_id, dx, dy) for each selected shape that needs to move.
    pub fn calculate_alignment_deltas(&self, alignment: Alignment) -> Vec<(u64, f64, f64)> {
        let selected: Vec<_> = self.shape_store.iter().filter(|o| o.selected).collect();
        if selected.is_empty() {
            return Vec::new();
        }

        // 1. Calculate target value
        let target = match alignment {
            Alignment::Left => selected
                .iter()
                .map(|o| o.shape.bounds().0)
                .fold(f64::INFINITY, f64::min),
            Alignment::Right => selected
                .iter()
                .map(|o| o.shape.bounds().2)
                .fold(f64::NEG_INFINITY, f64::max),
            Alignment::CenterHorizontal => {
                let (min_x, max_x) =
                    selected
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), o| {
                            let (x1, _, x2, _) = o.shape.bounds();
                            (min.min(x1), max.max(x2))
                        });
                if min_x.is_infinite() {
                    f64::INFINITY
                } else {
                    (min_x + max_x) / 2.0
                }
            }
            Alignment::Top => selected
                .iter()
                .map(|o| o.shape.bounds().3)
                .fold(f64::NEG_INFINITY, f64::max),
            Alignment::Bottom => selected
                .iter()
                .map(|o| o.shape.bounds().1)
                .fold(f64::INFINITY, f64::min),
            Alignment::CenterVertical => {
                let (min_y, max_y) =
                    selected
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), o| {
                            let (_, y1, _, y2) = o.shape.bounds();
                            (min.min(y1), max.max(y2))
                        });
                if min_y.is_infinite() {
                    f64::INFINITY
                } else {
                    (min_y + max_y) / 2.0
                }
            }
        };

        if target.is_infinite() {
            return Vec::new();
        }

        let mut deltas = Vec::new();

        for obj in selected {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            let (dx, dy) = match alignment {
                Alignment::Left => (target - x1, 0.0),
                Alignment::Right => (target - x2, 0.0),
                Alignment::CenterHorizontal => (target - (x1 + x2) / 2.0, 0.0),
                Alignment::Top => (0.0, target - y2),
                Alignment::Bottom => (0.0, target - y1),
                Alignment::CenterVertical => (0.0, target - (y1 + y2) / 2.0),
            };

            if dx.abs() > f64::EPSILON || dy.abs() > f64::EPSILON {
                deltas.push((obj.id, dx, dy));
            }
        }

        deltas
    }

    /// Pastes objects onto the canvas with an offset.
    /// Returns the IDs of the new objects.
    pub fn paste_objects(
        &mut self,
        objects: &[DrawingObject],
        offset_x: f64,
        offset_y: f64,
    ) -> Vec<u64> {
        let mut new_ids = Vec::new();
        let mut group_map = std::collections::HashMap::new();

        self.selection_manager.deselect_all(&mut self.shape_store);

        for obj in objects {
            let id = self.shape_store.generate_id();

            let mut new_shape = obj.shape.clone();
            new_shape.translate(offset_x, offset_y);
            let (min_x, min_y, max_x, max_y) = new_shape.bounds();

            // Handle group ID mapping
            let new_group_id = if let Some(old_gid) = obj.group_id {
                if let std::collections::hash_map::Entry::Vacant(e) = group_map.entry(old_gid) {
                    let new_gid = self.shape_store.generate_id();
                    e.insert(new_gid);
                    Some(new_gid)
                } else {
                    Some(group_map[&old_gid])
                }
            } else {
                None
            };

            let new_obj = DrawingObject {
                id,
                group_id: new_group_id,
                name: obj.name.clone(),
                shape: new_shape,
                selected: true, // Select the new object
                operation_type: obj.operation_type,
                use_custom_values: obj.use_custom_values,
                pocket_depth: obj.pocket_depth,
                start_depth: obj.start_depth,
                step_down: obj.step_down,
                step_in: obj.step_in,
                ramp_angle: obj.ramp_angle,
                pocket_strategy: obj.pocket_strategy,
                raster_fill_ratio: obj.raster_fill_ratio,
                offset: obj.offset,
                fillet: obj.fillet,
                chamfer: obj.chamfer,
                lock_aspect_ratio: obj.lock_aspect_ratio,
            };

            self.shape_store.insert(id, new_obj);
            self.spatial_manager
                .insert_bounds(id, &Bounds::new(min_x, min_y, max_x, max_y));
            new_ids.push(id);
        }

        // Update selected_id to the last pasted object if any
        if let Some(last_id) = new_ids.last() {
            self.selection_manager.set_selected_id(Some(*last_id));
        }

        new_ids
    }

    /// Resizes the selected shape. Handles: 0=TL, 1=TR, 2=BL, 3=BR, 4=Center (moves)
    pub fn resize_selected(&mut self, handle: usize, dx: f64, dy: f64) {
        // Calculate bounding box of ALL selected shapes
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_selected = false;

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
            has_selected = true;
        }

        if !has_selected {
            return;
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
        let sx = if old_width.abs() > 1e-6 {
            new_width / old_width
        } else {
            1.0
        };
        let sy = if old_height.abs() > 1e-6 {
            new_height / old_height
        } else {
            1.0
        };

        // Center of scaling
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        let new_center_x = (new_min_x + new_max_x) / 2.0;
        let new_center_y = (new_min_y + new_max_y) / 2.0;

        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if obj.selected {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();

                // Scale relative to the center of the SELECTION bounding box
                obj.shape.scale(sx, sy, Point::new(center_x, center_y));

                // Translate to new center
                let t_dx = new_center_x - center_x;
                let t_dy = new_center_y - center_y;
                obj.shape.translate(t_dx, t_dy);

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }
    }

    /// Calculates the snapped shapes without modifying the canvas.
    pub fn calculate_snapped_shapes(&self) -> Vec<(u64, DrawingObject)> {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            let width = x2 - x1;
            let height = y2 - y1;

            // Snap the top-left corner and dimensions to whole mm
            let snapped_x1 = (x1 + 0.5).floor();
            let snapped_y1 = (y1 + 0.5).floor();
            let snapped_width = (width + 0.5).floor();
            let snapped_height = (height + 0.5).floor();

            // Replace the shape with snapped position and dimensions
            let shape = &obj.shape;
            let new_shape: Shape = match shape {
                Shape::Rectangle(_) => Shape::Rectangle(Rectangle::new(
                    snapped_x1,
                    snapped_y1,
                    snapped_width,
                    snapped_height,
                )),
                Shape::Circle(_) => {
                    let radius = snapped_width / 2.0;
                    Shape::Circle(Circle::new(
                        Point::new(snapped_x1 + radius, snapped_y1 + radius),
                        radius,
                    ))
                }
                Shape::Line(_) => Shape::Line(Line::new(
                    Point::new(snapped_x1, snapped_y1),
                    Point::new(snapped_x1 + snapped_width, snapped_y1 + snapped_height),
                )),
                Shape::Ellipse(_) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    Shape::Ellipse(Ellipse::new(
                        center,
                        snapped_width / 2.0,
                        snapped_height / 2.0,
                    ))
                }
                Shape::Path(path_shape) => {
                    let (path_x1, path_y1, path_x2, path_y2) = path_shape.bounds();
                    let path_w = path_x2 - path_x1;
                    let path_h = path_y2 - path_y1;

                    let scale_x = if path_w.abs() > 1e-6 {
                        snapped_width / path_w
                    } else {
                        1.0
                    };
                    let scale_y = if path_h.abs() > 1e-6 {
                        snapped_height / path_h
                    } else {
                        1.0
                    };

                    let center_x = (path_x1 + path_x2) / 2.0;
                    let center_y = (path_y1 + path_y2) / 2.0;

                    let mut scaled = path_shape.clone();
                    scaled.scale(scale_x, scale_y, Point::new(center_x, center_y));

                    let new_center_x = snapped_x1 + snapped_width / 2.0;
                    let new_center_y = snapped_y1 + snapped_height / 2.0;

                    let dx = new_center_x - center_x;
                    let dy = new_center_y - center_y;

                    scaled.translate(dx, dy);
                    Shape::Path(scaled)
                }
                Shape::Text(text) => Shape::Text(TextShape::new(
                    text.text.clone(),
                    snapped_x1,
                    snapped_y1,
                    text.font_size,
                )),
                Shape::Triangle(_triangle) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    Shape::Triangle(Triangle::new(center, snapped_width, snapped_height))
                }
                Shape::Polygon(polygon) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    let radius = snapped_width.min(snapped_height) / 2.0;
                    Shape::Polygon(Polygon::new(center, radius, polygon.sides))
                }
                Shape::Gear(gear) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    let module = snapped_width / gear.teeth as f64;
                    Shape::Gear(crate::model::DesignGear::new(center, module, gear.teeth))
                }
                Shape::Sprocket(sprocket) => {
                    let center = Point::new(
                        snapped_x1 + snapped_width / 2.0,
                        snapped_y1 + snapped_height / 2.0,
                    );
                    let pitch =
                        snapped_width * (std::f64::consts::PI / sprocket.teeth as f64).sin();
                    Shape::Sprocket(crate::model::DesignSprocket::new(
                        center,
                        pitch,
                        sprocket.teeth,
                    ))
                }
            };

            let mut new_obj = obj.clone();
            new_obj.shape = new_shape;
            updates.push((obj.id, new_obj));
        }

        updates
    }

    /// Snaps the selected shape's position to whole millimeters
    pub fn snap_selected_to_mm(&mut self) {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if obj.selected {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();
                let (x1, y1, x2, y2) = obj.shape.bounds();
                let width = x2 - x1;
                let height = y2 - y1;

                // Snap the top-left corner and dimensions to whole mm
                let snapped_x1 = (x1 + 0.5).floor();
                let snapped_y1 = (y1 + 0.5).floor();
                let snapped_width = (width + 0.5).floor();
                let snapped_height = (height + 0.5).floor();

                // Replace the shape with snapped position and dimensions
                let shape = &obj.shape;
                let new_shape: Shape = match shape.shape_type() {
                    ShapeType::Rectangle => Shape::Rectangle(Rectangle::new(
                        snapped_x1,
                        snapped_y1,
                        snapped_width,
                        snapped_height,
                    )),
                    ShapeType::Circle => {
                        let radius = snapped_width / 2.0;
                        Shape::Circle(Circle::new(
                            Point::new(snapped_x1 + radius, snapped_y1 + radius),
                            radius,
                        ))
                    }
                    ShapeType::Line => Shape::Line(Line::new(
                        Point::new(snapped_x1, snapped_y1),
                        Point::new(snapped_x1 + snapped_width, snapped_y1 + snapped_height),
                    )),
                    ShapeType::Ellipse => {
                        let rx = snapped_width / 2.0;
                        let ry = snapped_height / 2.0;
                        Shape::Ellipse(Ellipse::new(
                            Point::new(snapped_x1 + rx, snapped_y1 + ry),
                            rx,
                            ry,
                        ))
                    }
                    ShapeType::Path => {
                        if let Some(path_shape) = shape.as_any().downcast_ref::<PathShape>() {
                            let sx = if width.abs() > 1e-6 {
                                snapped_width / width
                            } else {
                                1.0
                            };
                            let sy = if height.abs() > 1e-6 {
                                snapped_height / height
                            } else {
                                1.0
                            };

                            let center_x = (x1 + x2) / 2.0;
                            let center_y = (y1 + y2) / 2.0;

                            let mut new_path_shape = path_shape.clone();
                            new_path_shape.scale(sx, sy, Point::new(center_x, center_y));

                            let new_center_x = snapped_x1 + snapped_width / 2.0;
                            let new_center_y = snapped_y1 + snapped_height / 2.0;
                            let t_dx = new_center_x - center_x;
                            let t_dy = new_center_y - center_y;

                            new_path_shape.translate(t_dx, t_dy);
                            Shape::Path(new_path_shape)
                        } else {
                            shape.clone()
                        }
                    }
                    ShapeType::Text => {
                        if let Some(text) = shape.as_any().downcast_ref::<TextShape>() {
                            Shape::Text(TextShape::new(
                                text.text.clone(),
                                snapped_x1,
                                snapped_y1,
                                text.font_size,
                            ))
                        } else {
                            shape.clone()
                        }
                    }
                    ShapeType::Triangle => {
                        let center = Point::new(
                            snapped_x1 + snapped_width / 2.0,
                            snapped_y1 + snapped_height / 2.0,
                        );
                        Shape::Triangle(Triangle::new(center, snapped_width, snapped_height))
                    }
                    ShapeType::Polygon => {
                        if let Some(poly) = shape.as_any().downcast_ref::<Polygon>() {
                            let radius = snapped_width.min(snapped_height) / 2.0;
                            let center = Point::new(
                                snapped_x1 + snapped_width / 2.0,
                                snapped_y1 + snapped_height / 2.0,
                            );
                            Shape::Polygon(Polygon::new(center, radius, poly.sides))
                        } else {
                            shape.clone()
                        }
                    }
                    ShapeType::Gear => {
                        if let Some(gear) =
                            shape.as_any().downcast_ref::<crate::model::DesignGear>()
                        {
                            let center = Point::new(
                                snapped_x1 + snapped_width / 2.0,
                                snapped_y1 + snapped_height / 2.0,
                            );
                            let module = snapped_width / gear.teeth as f64;
                            Shape::Gear(crate::model::DesignGear::new(center, module, gear.teeth))
                        } else {
                            shape.clone()
                        }
                    }
                    ShapeType::Sprocket => {
                        if let Some(sprocket) = shape
                            .as_any()
                            .downcast_ref::<crate::model::DesignSprocket>()
                        {
                            let center = Point::new(
                                snapped_x1 + snapped_width / 2.0,
                                snapped_y1 + snapped_height / 2.0,
                            );
                            let pitch = snapped_width
                                * (std::f64::consts::PI / sprocket.teeth as f64).sin();
                            Shape::Sprocket(crate::model::DesignSprocket::new(
                                center,
                                pitch,
                                sprocket.teeth,
                            ))
                        } else {
                            shape.clone()
                        }
                    }
                };
                obj.shape = new_shape;

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }
    }

    /// Calculates position and size updates without modifying the canvas.
    pub fn calculate_position_and_size_updates(
        &self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        update_position: bool,
        update_size: bool,
    ) -> Vec<(u64, DrawingObject)> {
        let mut updates = Vec::new();

        // 1. Calculate union bounding box of all selected items
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_selected = false;

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
            has_selected = true;
        }

        if !has_selected {
            return updates;
        }

        let old_w = max_x - min_x;
        let old_h = max_y - min_y;

        // Calculate the current center
        let old_center_x = min_x + old_w / 2.0;
        let old_center_y = min_y + old_h / 2.0;

        // 2. Determine target values
        // When update_position is false, preserve the current center (x,y from UI are center coords)
        // When update_position is true, use the new center from x,y
        let target_center_x = if update_position { x } else { old_center_x };
        let target_center_y = if update_position { y } else { old_center_y };
        let target_w = if update_size { w } else { old_w };
        let target_h = if update_size { h } else { old_h };

        // Calculate target top-left from center
        let target_x = target_center_x - target_w / 2.0;
        let target_y = target_center_y - target_h / 2.0;

        // 3. Calculate scale factors
        let sx = if update_size && old_w.abs() > 1e-6 {
            target_w / old_w
        } else {
            1.0
        };
        let sy = if update_size && old_h.abs() > 1e-6 {
            target_h / old_h
        } else {
            1.0
        };

        // Center of the original group
        let group_center_x = min_x + old_w / 2.0;
        let group_center_y = min_y + old_h / 2.0;

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let mut new_obj = obj.clone();

            // Apply scaling relative to group center
            new_obj
                .shape
                .scale(sx, sy, Point::new(group_center_x, group_center_y));

            // Calculate translation to move to target position
            // The group's new center after scaling is still (group_center_x, group_center_y)
            // But its size is (target_w, target_h), so its new top-left is:
            let current_new_x = group_center_x - target_w / 2.0;
            let current_new_y = group_center_y - target_h / 2.0;

            let dx = target_x - current_new_x;
            let dy = target_y - current_new_y;

            new_obj.shape.translate(dx, dy);

            updates.push((obj.id, new_obj));
        }

        updates
    }

    pub fn set_selected_position_and_size_with_flags(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        update_position: bool,
        update_size: bool,
    ) -> bool {
        // 1. Calculate union bounding box
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_selected = false;

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            let (x1, y1, x2, y2) = obj.shape.bounds();
            min_x = min_x.min(x1);
            min_y = min_y.min(y1);
            max_x = max_x.max(x2);
            max_y = max_y.max(y2);
            has_selected = true;
        }

        if !has_selected {
            return false;
        }

        let old_w = max_x - min_x;
        let old_h = max_y - min_y;

        // Calculate the current center
        let old_center_x = min_x + old_w / 2.0;
        let old_center_y = min_y + old_h / 2.0;

        // 2. Determine target values
        // When update_position is false, preserve the current center (x,y from UI are center coords)
        // When update_position is true, use the new center from x,y
        let target_center_x = if update_position { x } else { old_center_x };
        let target_center_y = if update_position { y } else { old_center_y };
        let target_w = if update_size { w } else { old_w };
        let target_h = if update_size { h } else { old_h };

        // Calculate target top-left from center
        let target_x = target_center_x - target_w / 2.0;
        let target_y = target_center_y - target_h / 2.0;

        // 3. Calculate scale factors
        let sx = if update_size && old_w.abs() > 1e-6 {
            target_w / old_w
        } else {
            1.0
        };
        let sy = if update_size && old_h.abs() > 1e-6 {
            target_h / old_h
        } else {
            1.0
        };

        // Center of the original group
        let group_center_x = min_x + old_w / 2.0;
        let group_center_y = min_y + old_h / 2.0;

        let mut changed_any = false;
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if !obj.selected {
                continue;
            }

            let (old_x, old_y, old_x2, old_y2) = obj.get_total_bounds();

            // Apply scaling relative to group center
            obj.shape
                .scale(sx, sy, Point::new(group_center_x, group_center_y));

            // Calculate translation to move to target position
            let current_new_x = group_center_x - target_w / 2.0;
            let current_new_y = group_center_y - target_h / 2.0;

            let dx = target_x - current_new_x;
            let dy = target_y - current_new_y;

            obj.shape.translate(dx, dy);

            changed_any = true;

            let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
            updates.push((
                obj.id,
                Bounds::new(old_x, old_y, old_x2, old_y2),
                Bounds::new(new_x1, new_y1, new_x2, new_y2),
            ));
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }

        changed_any
    }

    /// Calculates text property updates without modifying the canvas.
    pub fn calculate_text_property_updates(
        &self,
        content: &str,
        font_size: f64,
    ) -> Vec<(u64, DrawingObject)> {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            if let Some(text) = obj.shape.as_any().downcast_ref::<TextShape>() {
                let (x, y) = (text.x, text.y);

                let mut new_obj = obj.clone();
                new_obj.shape = Shape::Text(TextShape::new(content.to_string(), x, y, font_size));
                updates.push((obj.id, new_obj));
            }
        }

        updates
    }

    pub fn set_selected_text_properties(&mut self, content: &str, font_size: f64) -> bool {
        let mut changed = false;
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if !obj.selected {
                continue;
            }
            if let Some(text) = obj.shape.as_any().downcast_ref::<TextShape>() {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();
                let (x, y) = (text.x, text.y);

                obj.shape = Shape::Text(TextShape::new(content.to_string(), x, y, font_size));
                changed = true;

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }

        changed
    }

    /// Calculates rectangle property updates without modifying the canvas.
    pub fn calculate_rectangle_property_updates(
        &self,
        corner_radius: f64,
        is_slot: bool,
    ) -> Vec<(u64, DrawingObject)> {
        let mut updates = Vec::new();

        for obj in self.shape_store.iter().filter(|o| o.selected) {
            if let Some(rect) = obj.shape.as_any().downcast_ref::<Rectangle>() {
                let mut new_rect = rect.clone();
                new_rect.corner_radius = corner_radius;
                new_rect.is_slot = is_slot;

                // Re-constrain radius
                let max_radius = new_rect.width.min(new_rect.height).abs() / 2.0;
                if new_rect.is_slot {
                    new_rect.corner_radius = max_radius;
                } else {
                    new_rect.corner_radius = new_rect.corner_radius.min(max_radius);
                }

                let mut new_obj = obj.clone();
                new_obj.shape = Shape::Rectangle(new_rect);
                updates.push((obj.id, new_obj));
            }
        }

        updates
    }

    pub fn set_selected_rectangle_properties(&mut self, corner_radius: f64, is_slot: bool) -> bool {
        let mut changed = false;
        let mut updates = Vec::new();

        for obj in self.shape_store.iter_mut() {
            if !obj.selected {
                continue;
            }
            if let Some(rect) = obj.shape.as_any().downcast_ref::<Rectangle>() {
                let (old_x1, old_y1, old_x2, old_y2) = obj.get_total_bounds();

                let mut new_rect = rect.clone();
                new_rect.corner_radius = corner_radius;
                new_rect.is_slot = is_slot;

                // Re-constrain radius
                let max_radius = new_rect.width.min(new_rect.height).abs() / 2.0;
                if new_rect.is_slot {
                    new_rect.corner_radius = max_radius;
                } else {
                    new_rect.corner_radius = new_rect.corner_radius.min(max_radius);
                }

                obj.shape = Shape::Rectangle(new_rect);
                changed = true;

                let (new_x1, new_y1, new_x2, new_y2) = obj.get_total_bounds();
                updates.push((
                    obj.id,
                    Bounds::new(old_x1, old_y1, old_x2, old_y2),
                    Bounds::new(new_x1, new_y1, new_x2, new_y2),
                ));
            }
        }

        for (id, old_bounds, new_bounds) in updates {
            self.spatial_manager.remove_bounds(id, &old_bounds);
            self.spatial_manager.insert_bounds(id, &new_bounds);
        }

        changed
    }
}
