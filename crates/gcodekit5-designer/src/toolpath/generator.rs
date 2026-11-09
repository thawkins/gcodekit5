//! Toolpath generator and related utilities.

use super::*;

// use lyon::path::iterator::PathIterator;
use crate::Ellipse;
use rusttype::{GlyphId, OutlineBuilder, Scale};
use smallvec::SmallVec;

/// Generates toolpaths from design shapes.
#[derive(Debug, Clone)]
pub struct ToolpathGenerator {
    feed_rate: f64,
    spindle_speed: u32,
    tool_diameter: f64,
    cut_depth: f64,
    start_depth: f64,
    step_in: f64,
    pocket_strategy: PocketStrategy,
    ramp_angle: f64,
    raster_fill_ratio: f64,
}

#[allow(clippy::new_without_default)]
impl ToolpathGenerator {
    /// Creates a new toolpath generator with default parameters.
    pub fn new() -> Self {
        Self {
            feed_rate: 0.0,
            spindle_speed: 0,
            tool_diameter: 3.175,
            cut_depth: 5.0,
            start_depth: 0.0,
            step_in: 1.0,
            pocket_strategy: PocketStrategy::ContourParallel,
            ramp_angle: 0.0,
            raster_fill_ratio: 0.5,
        }
    }

    pub fn with_global_settings(feed_rate: f64, spindle_speed: u32) -> Self {
        Self {
            feed_rate,
            spindle_speed,
            ..Self::new()
        }
    }

    pub fn set_pocket_strategy(&mut self, strategy: PocketStrategy) {
        self.pocket_strategy = strategy;
    }

    pub fn set_ramp_angle(&mut self, angle: f64) {
        self.ramp_angle = angle;
    }

    pub fn set_feed_rate(&mut self, feed_rate: f64) {
        debug_assert!(
            feed_rate.is_finite() && feed_rate > 0.0,
            "feed_rate must be positive and finite, got {feed_rate}"
        );
        self.feed_rate = feed_rate;
    }

    pub fn set_spindle_speed(&mut self, speed: u32) {
        self.spindle_speed = speed;
    }

    pub fn set_tool_diameter(&mut self, diameter: f64) {
        debug_assert!(
            diameter.is_finite() && diameter > 0.0,
            "tool_diameter must be positive and finite, got {diameter}"
        );
        self.tool_diameter = diameter;
    }

    pub fn set_cut_depth(&mut self, depth: f64) {
        debug_assert!(depth.is_finite(), "cut_depth must be finite, got {depth}");
        self.cut_depth = depth;
    }

    pub fn set_start_depth(&mut self, depth: f64) {
        self.start_depth = depth;
    }

    pub fn set_step_in(&mut self, step_in: f64) {
        debug_assert!(step_in.is_finite(), "step_in must be finite, got {step_in}");
        self.step_in = if step_in > 0.0 { step_in } else { 0.1 };
    }

    pub fn set_raster_fill_ratio(&mut self, ratio: f64) {
        self.raster_fill_ratio = ratio.clamp(0.0, 1.0);
    }

    pub fn raster_fill_ratio(&self) -> f64 {
        self.raster_fill_ratio
    }

    pub fn empty_toolpath(&self) -> Toolpath {
        Toolpath::new(self.tool_diameter, self.start_depth - self.cut_depth.abs())
    }

    // ==================== CONTOUR METHODS ====================

    pub fn generate_rectangle_contour(&self, rect: &Rectangle, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        let (feed_rate, spindle_speed) = if rect.laser_params.use_global {
            (self.feed_rate, self.spindle_speed)
        } else {
            (
                rect.laser_params.feed_rate,
                (rect.laser_params.power_percent * 10.0) as u32,
            )
        };

        let w = rect.width;
        let h = rect.height;
        let cx = rect.center.x;
        let cy = rect.center.y;
        let x = cx - w / 2.0;
        let y = cy - h / 2.0;
        let r = rect.effective_corner_radius().min(w / 2.0).min(h / 2.0);
        let rotation = rect.rotation;

        let transform_point = |p: Point| -> Point {
            if rotation.abs() > 1e-6 {
                rotate_point(p, rect.center, rotation)
            } else {
                p
            }
        };

        if r < 0.001 {
            let corners = [
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + h),
                Point::new(x, y + h),
            ];
            let t_corners: SmallVec<[Point; 4]> =
                corners.iter().map(|&p| transform_point(p)).collect();

            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                t_corners[0],
                t_corners[0],
                feed_rate,
                spindle_speed,
            ));

            for i in 0..4 {
                let next_i = (i + 1) % 4;
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    t_corners[i],
                    t_corners[next_i],
                    feed_rate,
                    spindle_speed,
                ));
            }
        } else {
            let start_pt_raw = Point::new(x + r, y);
            let start_pt = transform_point(start_pt_raw);

            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                start_pt,
                start_pt,
                feed_rate,
                spindle_speed,
            ));

            let mut current_pt = start_pt;

            let p1_raw = Point::new(x + w - r, y);
            let p1 = transform_point(p1_raw);
            if current_pt.distance_to(&p1) > 0.001 {
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    current_pt,
                    p1,
                    feed_rate,
                    spindle_speed,
                ));
                current_pt = p1;
            }

            let p_br_end_raw = Point::new(x + w, y + r);
            let center_br_raw = Point::new(x + w - r, y + r);
            let p_br_end = transform_point(p_br_end_raw);
            let center_br = transform_point(center_br_raw);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_br_end,
                center_br,
                feed_rate,
                spindle_speed,
            ));
            current_pt = p_br_end;

            let p2_raw = Point::new(x + w, y + h - r);
            let p2 = transform_point(p2_raw);
            if current_pt.distance_to(&p2) > 0.001 {
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    current_pt,
                    p2,
                    feed_rate,
                    spindle_speed,
                ));
                current_pt = p2;
            }

            let p_tr_end_raw = Point::new(x + w - r, y + h);
            let center_tr_raw = Point::new(x + w - r, y + h - r);
            let p_tr_end = transform_point(p_tr_end_raw);
            let center_tr = transform_point(center_tr_raw);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_tr_end,
                center_tr,
                feed_rate,
                spindle_speed,
            ));
            current_pt = p_tr_end;

            let p3_raw = Point::new(x + r, y + h);
            let p3 = transform_point(p3_raw);
            if current_pt.distance_to(&p3) > 0.001 {
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    current_pt,
                    p3,
                    feed_rate,
                    spindle_speed,
                ));
                current_pt = p3;
            }

            let p_tl_end_raw = Point::new(x, y + h - r);
            let center_tl_raw = Point::new(x + r, y + h - r);
            let p_tl_end = transform_point(p_tl_end_raw);
            let center_tl = transform_point(center_tl_raw);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_tl_end,
                center_tl,
                feed_rate,
                spindle_speed,
            ));
            current_pt = p_tl_end;

            let p4_raw = Point::new(x, y + r);
            let p4 = transform_point(p4_raw);
            if current_pt.distance_to(&p4) > 0.001 {
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    current_pt,
                    p4,
                    feed_rate,
                    spindle_speed,
                ));
                current_pt = p4;
            }

            let p_bl_end_raw = Point::new(x + r, y);
            let center_bl_raw = Point::new(x + r, y + r);
            let p_bl_end = transform_point(p_bl_end_raw);
            let center_bl = transform_point(center_bl_raw);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_bl_end,
                center_bl,
                feed_rate,
                spindle_speed,
            ));
        }

        self.create_multipass_toolpaths(segments, step_down)
    }

    pub fn generate_circle_contour(&self, circle: &Circle, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        let (feed_rate, spindle_speed) = if circle.laser_params.use_global {
            (self.feed_rate, self.spindle_speed)
        } else {
            (
                circle.laser_params.feed_rate,
                (circle.laser_params.power_percent * 10.0) as u32,
            )
        };

        let rotation = circle.rotation;
        let transform_point = |p: Point| -> Point {
            if rotation.abs() > 1e-6 {
                rotate_point(p, circle.center, rotation)
            } else {
                p
            }
        };

        let start_point_raw = Point::new(circle.center.x + circle.radius, circle.center.y);
        let start_point = transform_point(start_point_raw);

        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            start_point,
            start_point,
            feed_rate,
            spindle_speed,
        ));

        let points_raw = [
            Point::new(circle.center.x, circle.center.y + circle.radius),
            Point::new(circle.center.x - circle.radius, circle.center.y),
            Point::new(circle.center.x, circle.center.y - circle.radius),
            Point::new(circle.center.x + circle.radius, circle.center.y),
        ];

        let mut current = start_point;
        for p_raw in points_raw.iter() {
            let p = transform_point(*p_raw);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current,
                p,
                circle.center,
                feed_rate,
                spindle_speed,
            ));
            current = p;
        }

        self.create_multipass_toolpaths(segments, step_down)
    }

    pub fn generate_ellipse_contour(&self, ellipse: &Ellipse, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        let (feed_rate, spindle_speed) = if ellipse.laser_params.use_global {
            (self.feed_rate, self.spindle_speed)
        } else {
            (
                ellipse.laser_params.feed_rate,
                (ellipse.laser_params.power_percent * 10.0) as u32,
            )
        };

        let center = ellipse.center;
        let rx = ellipse.rx;
        let ry = ellipse.ry;
        let rotation = ellipse.rotation;
        let steps = 64;

        let transform_point = |p: Point| -> Point {
            if rotation.abs() > 1e-6 {
                rotate_point(p, center, rotation)
            } else {
                p
            }
        };

        let start_x = center.x + rx;
        let start_y = center.y;
        let start_point = transform_point(Point::new(start_x, start_y));

        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            start_point,
            start_point,
            feed_rate,
            spindle_speed,
        ));

        let mut current = start_point;
        for i in 1..=steps {
            let t = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
            let x = center.x + rx * t.cos();
            let y = center.y + ry * t.sin();
            let p = transform_point(Point::new(x, y));

            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                current,
                p,
                feed_rate,
                spindle_speed,
            ));
            current = p;
        }

        if current.distance_to(&start_point) > 0.001 {
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                current,
                start_point,
                feed_rate,
                spindle_speed,
            ));
        }

        self.create_multipass_toolpaths(segments, step_down)
    }

    pub fn generate_line_contour(&self, line: &Line, step_down: f64) -> Vec<Toolpath> {
        let (feed_rate, spindle_speed) = if line.laser_params.use_global {
            (self.feed_rate, self.spindle_speed)
        } else {
            (
                line.laser_params.feed_rate,
                (line.laser_params.power_percent * 10.0) as u32,
            )
        };

        let segments = vec![
            ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                line.start,
                line.start,
                feed_rate,
                spindle_speed,
            ),
            ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                line.start,
                line.end,
                feed_rate,
                spindle_speed,
            ),
        ];

        self.create_multipass_toolpaths(segments, step_down)
    }

    pub fn generate_triangle_contour(&self, triangle: &Triangle, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        let (feed_rate, spindle_speed) = if triangle.laser_params.use_global {
            (self.feed_rate, self.spindle_speed)
        } else {
            (
                triangle.laser_params.feed_rate,
                (triangle.laser_params.power_percent * 10.0) as u32,
            )
        };

        // Obtener los vértices locales del triángulo usando get_local_vertices_f64()
        // Esto respeta right_angle_corner
        let [p1_local, p2_local, p3_local] = triangle.get_local_vertices_f64();

        // Aplicar rotación y traslación
        let rotation = triangle.rotation;
        let center = triangle.center;

        let transform_point = |p: Point| -> Point {
            let mut pt = p;
            if rotation.abs() > 1e-6 {
                pt = rotate_point(pt, Point::new(0.0, 0.0), rotation);
            }
            Point::new(pt.x + center.x, pt.y + center.y)
        };

        let p1 = transform_point(p1_local);
        let p2 = transform_point(p2_local);
        let p3 = transform_point(p3_local);

        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            p1,
            p1,
            feed_rate,
            spindle_speed,
        ));

        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            p1,
            p2,
            feed_rate,
            spindle_speed,
        ));

        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            p2,
            p3,
            feed_rate,
            spindle_speed,
        ));

        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            p3,
            p1,
            feed_rate,
            spindle_speed,
        ));

        self.create_multipass_toolpaths(segments, step_down)
    }

    pub fn generate_polygon_contour(&self, polygon: &Polygon, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        let (feed_rate, spindle_speed) = if polygon.laser_params.use_global {
            (self.feed_rate, self.spindle_speed)
        } else {
            (
                polygon.laser_params.feed_rate,
                (polygon.laser_params.power_percent *10.0) as u32,
            )
        };

        let sides = polygon.sides.max(3);
        let rotation = polygon.rotation;
        let center = polygon.center;
        let radius = polygon.radius;

        let transform_point = |p: Point| -> Point {
            let mut pt = p;
            if rotation.abs() > 1e-6 {
                pt = rotate_point(pt, Point::new(0.0, 0.0), rotation);
            }
            Point::new(pt.x + center.x, pt.y + center.y)
        };

        let mut points = Vec::with_capacity(sides as usize);
        for i in 0..sides {
            let theta = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
            let x = radius * theta.cos();
            let y = radius * theta.sin();
            points.push(transform_point(Point::new(x, y)));
        }

        if points.is_empty() {
            return Vec::new();
        }

        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            points[0],
            points[0],
            feed_rate,
            spindle_speed,
        ));

        for i in 0..sides as usize {
            let next_i = (i + 1) % (sides as usize);
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                points[i],
                points[next_i],
                feed_rate,
                spindle_speed,
            ));
        }

        self.create_multipass_toolpaths(segments, step_down)
    }

    pub fn generate_path_contour(&self, path_shape: &PathShape, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        let (feed_rate, spindle_speed) = if path_shape.laser_params.use_global {
            (self.feed_rate, self.spindle_speed)
        } else {
            (
                path_shape.laser_params.feed_rate,
                (path_shape.laser_params.power_percent * 10.0) as u32,
            )
        };

        let mut first_point: Option<Point> = None;
        let mut current_pos = Point::new(0.0, 0.0);

        for event in path_shape.render().iter() {
            match event {
                lyon::path::Event::Begin { at } => {
                    let p = Point::new(at.x as f64, at.y as f64);
                    segments.push(ToolpathSegment::new(
                        ToolpathSegmentType::RapidMove,
                        current_pos,
                        p,
                        feed_rate,
                        spindle_speed,
                    ));
                    current_pos = p;
                    first_point = Some(p);
                }
                lyon::path::Event::Line { to, .. } => {
                    let p = Point::new(to.x as f64, to.y as f64);
                    segments.push(ToolpathSegment::new(
                        ToolpathSegmentType::LinearMove,
                        current_pos,
                        p,
                        feed_rate,
                        spindle_speed,
                    ));
                    current_pos = p;
                }
                lyon::path::Event::Cubic {
                    ctrl1, ctrl2, to, ..
                } => {
                    let p_to = Point::new(to.x as f64, to.y as f64);
                    let p_c1 = Point::new(ctrl1.x as f64, ctrl1.y as f64);
                    let p_c2 = Point::new(ctrl2.x as f64, ctrl2.y as f64);

                    if let Some((center, is_ccw)) =
                        self.try_convert_to_arc(current_pos, p_to, p_c1, p_c2)
                    {
                        segments.push(ToolpathSegment::new_arc(
                            if is_ccw {
                                ToolpathSegmentType::ArcCCW
                            } else {
                                ToolpathSegmentType::ArcCW
                            },
                            current_pos,
                            p_to,
                            center,
                            feed_rate,
                            spindle_speed,
                        ));
                    } else {
                        let mut last_p = current_pos;
                        let steps = 32;
                        for i in 1..=steps {
                            let t = i as f64 / steps as f64;
                            let inv_t = 1.0 - t;
                            let x = inv_t.powi(3) * current_pos.x
                                + 3.0 * inv_t.powi(2) * t * p_c1.x
                                + 3.0 * inv_t * t.powi(2) * p_c2.x
                                + t.powi(3) * p_to.x;
                            let y = inv_t.powi(3) * current_pos.y
                                + 3.0 * inv_t.powi(2) * t * p_c1.y
                                + 3.0 * inv_t * t.powi(2) * p_c2.y
                                + t.powi(3) * p_to.y;
                            let next_p = Point::new(x, y);
                            segments.push(ToolpathSegment::new(
                                ToolpathSegmentType::LinearMove,
                                last_p,
                                next_p,
                                feed_rate,
                                spindle_speed,
                            ));
                            last_p = next_p;
                        }
                    }
                    current_pos = p_to;
                }
                lyon::path::Event::Quadratic { ctrl, to, .. } => {
                    let p_to = Point::new(to.x as f64, to.y as f64);
                    let p_ctrl = Point::new(ctrl.x as f64, ctrl.y as f64);

                    if let Some((center, is_ccw)) =
                        self.try_convert_quadratic_to_arc(current_pos, p_to, p_ctrl)
                    {
                        segments.push(ToolpathSegment::new_arc(
                            if is_ccw {
                                ToolpathSegmentType::ArcCCW
                            } else {
                                ToolpathSegmentType::ArcCW
                            },
                            current_pos,
                            p_to,
                            center,
                            feed_rate,
                            spindle_speed,
                        ));
                    } else {
                        let mut last_p = current_pos;
                        let steps = 32;
                        for i in 1..=steps {
                            let t = i as f64 / steps as f64;
                            let x = (1.0 - t).powi(2) * current_pos.x
                                + 2.0 * (1.0 - t) * t * p_ctrl.x
                                + t.powi(2) * p_to.x;
                            let y = (1.0 - t).powi(2) * current_pos.y
                                + 2.0 * (1.0 - t) * t * p_ctrl.y
                                + t.powi(2) * p_to.y;
                            let next_p = Point::new(x, y);
                            segments.push(ToolpathSegment::new(
                                ToolpathSegmentType::LinearMove,
                                last_p,
                                next_p,
                                feed_rate,
                                spindle_speed,
                            ));
                            last_p = next_p;
                        }
                    }
                    current_pos = p_to;
                }
                lyon::path::Event::End { close, .. } => {
                    if close {
                        if let Some(first) = first_point {
                            if current_pos.distance_to(&first) > 0.001 {
                                segments.push(ToolpathSegment::new(
                                    ToolpathSegmentType::LinearMove,
                                    current_pos,
                                    first,
                                    feed_rate,
                                    spindle_speed,
                                ));
                                current_pos = first;
                            }
                        }
                    }
                    first_point = None;
                }
            }
        }

        self.create_multipass_toolpaths(segments, step_down)
    }

    pub fn generate_gear_contour(&self, gear: &DesignGear, step_down: f64) -> Vec<Toolpath> {
        let path = gear.render();
        let path_shape = PathShape::from_lyon_path(&path);
        // Transferir laser_params del gear al path_shape
        let mut path_with_params = path_shape;
        path_with_params.laser_params = gear.laser_params;
        self.generate_path_contour(&path_with_params, step_down)
    }

    pub fn generate_sprocket_contour(
        &self,
        sprocket: &DesignSprocket,
        step_down: f64,
    ) -> Vec<Toolpath> {
        let path = sprocket.render();
        let path_shape = PathShape::from_lyon_path(&path);
        let mut path_with_params = path_shape;
        path_with_params.laser_params = sprocket.laser_params;
        self.generate_path_contour(&path_with_params, step_down)
    }

    pub fn generate_text_toolpath(&self, text_shape: &TextShape, step_down: f64) -> Vec<Toolpath> {
        let (feed_rate, spindle_speed) = if text_shape.laser_params.use_global {
            (self.feed_rate, self.spindle_speed)
        } else {
            (
                text_shape.laser_params.feed_rate,
                (text_shape.laser_params.power_percent * 10.0) as u32,
            )
        };

        let segments =
            self.build_text_outline_segments_with_params(text_shape, feed_rate, spindle_speed);
        self.create_multipass_toolpaths(segments, step_down)
    }

    // ==================== HELPER METHODS ====================

    fn create_multipass_toolpaths(
        &self,
        segments: Vec<ToolpathSegment>,
        step_down: f64,
    ) -> Vec<Toolpath> {
        let mut toolpaths = Vec::new();
        let start_z = self.start_depth;
        let target_z = start_z - self.cut_depth.abs();
        let total_dist = (start_z - target_z).abs();

        if self.ramp_angle > 0.001 && !segments.is_empty() {
            let contour_length: f64 = segments
                .iter()
                .map(|s| match s.segment_type {
                    ToolpathSegmentType::LinearMove | ToolpathSegmentType::RapidMove => {
                        s.start.distance_to(&s.end)
                    }
                    ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                        s.start.distance_to(&s.end)
                    }
                })
                .sum();

            if contour_length > 0.001 {
                let mut current_z = start_z;
                let mut loop_guard = 0;
                let max_segments = 20000;
                let path_segment_count = segments.len();

                while current_z > target_z {
                    loop_guard += 1;

                    if loop_guard * path_segment_count > max_segments {
                        let mut final_pass = Toolpath::new(self.tool_diameter, target_z);
                        final_pass.segments = segments
                            .iter()
                            .map(|s| {
                                let mut ns = s.clone();
                                ns.start_z = Some(target_z);
                                ns.z_depth = Some(target_z);
                                ns
                            })
                            .collect();
                        toolpaths.push(final_pass);
                        return toolpaths;
                    }

                    if loop_guard > 1000 {
                        break;
                    }

                    let mut pass_segments = Vec::new();
                    let mut segment_start_z = current_z;
                    let pass_start_z = current_z;

                    for seg in &segments {
                        if seg.segment_type == ToolpathSegmentType::RapidMove {
                            let mut new_seg = seg.clone();
                            new_seg.start_z = Some(segment_start_z);
                            new_seg.z_depth = Some(segment_start_z);
                            pass_segments.push(new_seg);
                            continue;
                        }

                        let seg_len = seg.start.distance_to(&seg.end);
                        let z_drop = seg_len * self.ramp_angle.to_radians().tan();

                        let mut next_z = segment_start_z - z_drop;
                        if next_z < target_z {
                            next_z = target_z;
                        }

                        let mut new_seg = seg.clone();
                        new_seg.start_z = Some(segment_start_z);
                        new_seg.z_depth = Some(next_z);
                        pass_segments.push(new_seg);

                        segment_start_z = next_z;
                    }

                    current_z = segment_start_z;

                    let mut tp = Toolpath::new(self.tool_diameter, current_z);
                    tp.segments = pass_segments;
                    toolpaths.push(tp);

                    if (current_z - target_z).abs() < 0.001 {
                        break;
                    }

                    if (pass_start_z - current_z).abs() < 0.001 {
                        let fallback_step = if step_down > 0.0 {
                            step_down
                        } else {
                            self.tool_diameter
                        };
                        current_z = (current_z - fallback_step).max(target_z);
                    }
                }

                let mut final_pass = Toolpath::new(self.tool_diameter, target_z);
                final_pass.segments = segments
                    .iter()
                    .map(|s| {
                        let mut ns = s.clone();
                        ns.start_z = Some(target_z);
                        ns.z_depth = Some(target_z);
                        ns
                    })
                    .collect();
                toolpaths.push(final_pass);

                return toolpaths;
            }
        }

        let step = if step_down <= 0.001 {
            total_dist
        } else {
            step_down
        };
        let num_passes = (total_dist / step).ceil() as usize;
        let num_passes = if num_passes == 0 { 1 } else { num_passes };

        for i in 1..=num_passes {
            let depth_step = (i as f64 * step).min(total_dist);
            let z = start_z - depth_step;

            let mut tp = Toolpath::new(self.tool_diameter, z);
            tp.segments = segments.clone();
            toolpaths.push(tp);
        }

        toolpaths
    }

fn build_text_outline_segments_with_params(
    &self,
    text_shape: &TextShape,
    feed_rate: f64,
    spindle_speed: u32,
) -> Vec<ToolpathSegment> {
    let mut all_segments = Vec::new();

    let font =
        font_manager::get_font_for(&text_shape.font_family, text_shape.bold, text_shape.italic);
    let scale = Scale::uniform(text_shape.font_size as f32);
    let v_metrics = font.v_metrics(scale);
    let line_height = v_metrics.ascent - v_metrics.descent - v_metrics.line_gap;

    let (left, bottom, right, top) = text_shape.bounds();
    let baseline_y0 = bottom as f32 + v_metrics.line_gap;
    let rotation_center = Point::new((left + right) / 2.0, (bottom + top) / 2.0);

    let mut caret_x = left as f32;
    let mut baseline_y = baseline_y0;
    let mut prev: Option<GlyphId> = None;
    let mut last_position = Point::new(0.0, 0.0);
    let mut is_first = true;

    for ch in text_shape.text.chars() {
        if ch == '\n' {
            caret_x = left as f32;
            baseline_y -= line_height;
            prev = None;
            is_first = true;
            continue;
        }

        if ch == ' ' {
            let base = font.glyph(' ');
            let scaled = base.scaled(scale);
            let advance = scaled.h_metrics().advance_width;
            caret_x += advance;
            continue;
        }

        let base = font.glyph(ch);
        let base_id = base.id();

        if let Some(prev_id) = prev {
            caret_x += font.pair_kerning(scale, prev_id, base_id);
        }

        let scaled = base.scaled(scale);
        let advance = scaled.h_metrics().advance_width;

        // Posición de inicio para esta letra
        let start_pos = Point::new(caret_x as f64, baseline_y as f64);

        // Si no es la primera letra, mover a la nueva posición
        if !is_first {
            all_segments.push(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                last_position,
                start_pos,
                feed_rate,
                spindle_speed,
            ));
        }

        // Crear builder para esta letra
        let mut builder = ToolpathBuilder::new(
            feed_rate,
            spindle_speed,
            start_pos,  // Punto inicial
            start_pos,  // Offset
            rotation_center,
            text_shape.rotation,
        );

        // Construir el outline de la letra
        scaled.build_outline(&mut builder);

        // Tomar los segmentos de la letra
        let letter_segments = builder.take_segments();
        all_segments.extend(letter_segments);

        // Guardar la última posición
        last_position = builder.current_point;

        // Cerrar el contorno si es necesario
        if let Some(first_point) = builder.start_point_opt() {
            if last_position.distance_to(&first_point) > 0.001 {
                all_segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    last_position,
                    first_point,
                    feed_rate,
                    spindle_speed,
                ));
                last_position = first_point;
            }
        }

        caret_x += advance;
        prev = Some(base_id);
        is_first = false;
    }

    all_segments
}

    // ==================== POCKET METHODS (sin cambios) ====================

    pub fn generate_ellipse_pocket(
        &self,
        ellipse: &Ellipse,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let steps = 64;
        let mut vertices = Vec::with_capacity(steps + 1);
        let center = ellipse.center;
        let rx = ellipse.rx;
        let ry = ellipse.ry;
        let rotation = ellipse.rotation;

        for i in 0..=steps {
            let t = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
            let x = center.x + rx * t.cos();
            let y = center.y + ry * t.sin();
            let mut p = Point::new(x, y);
            if rotation.abs() > 1e-6 {
                p = rotate_point(p, center, rotation);
            }
            vertices.push(p);
        }

        self.generate_polyline_pocket(&vertices, pocket_depth, step_down, step_in)
    }

    pub fn generate_rectangle_pocket(
        &self,
        rect: &Rectangle,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let r = rect
            .effective_corner_radius()
            .min(rect.width.abs() / 2.0)
            .min(rect.height.abs() / 2.0);

        if r > 0.001 || rect.rotation.abs() > 1e-6 {
            let mut vertices = Vec::new();
            let x = rect.center.x - rect.width / 2.0;
            let y = rect.center.y - rect.height / 2.0;
            let w = rect.width;
            let h = rect.height;

            if r > 0.001 {
                let segments = 32;
                let mut add_arc_points =
                    |center: Point, start_angle: f64, end_angle: f64, include_start: bool| {
                        let start_rad = start_angle.to_radians();
                        let end_rad = end_angle.to_radians();
                        let step = (end_rad - start_rad) / segments as f64;
                        let start_i = if include_start { 0 } else { 1 };
                        for i in start_i..=segments {
                            let angle = start_rad + step * i as f64;
                            vertices.push(Point::new(
                                center.x + r * angle.cos(),
                                center.y + r * angle.sin(),
                            ));
                        }
                    };
                add_arc_points(Point::new(x + w - r, y + r), 270.0, 360.0, true);
                add_arc_points(Point::new(x + w - r, y + h - r), 0.0, 90.0, false);
                add_arc_points(Point::new(x + r, y + h - r), 90.0, 180.0, false);
                add_arc_points(Point::new(x + r, y + r), 180.0, 270.0, false);
            } else {
                vertices.push(Point::new(x, y));
                vertices.push(Point::new(x + w, y));
                vertices.push(Point::new(x + w, y + h));
                vertices.push(Point::new(x, y + h));
            }

            if rect.rotation.abs() > 1e-6 {
                let center = rect.center;
                let rotation_deg = rect.rotation;
                for p in &mut vertices {
                    *p = crate::model::rotate_point(*p, center, rotation_deg);
                }
            }

            return self.generate_polyline_pocket(&vertices, pocket_depth, step_down, step_in);
        }

        let op = PocketOperation::new("rect_pocket".to_string(), pocket_depth, self.tool_diameter);
        let mut gen = PocketGenerator::new(op);
        gen.operation.set_start_depth(self.start_depth);
        gen.operation.set_ramp_angle(self.ramp_angle);
        gen.operation.raster_fill_ratio = self.raster_fill_ratio;
        let effective_step_in = if step_in > 0.0 { step_in } else { self.step_in };
        gen.operation
            .set_parameters(effective_step_in, self.feed_rate, self.spindle_speed);
        gen.generate_rectangular_pocket(rect, step_down)
    }

    pub fn generate_circle_pocket(
        &self,
        circle: &Circle,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let op = PocketOperation::new(
            "circle_pocket".to_string(),
            pocket_depth,
            self.tool_diameter,
        );
        let mut gen = PocketGenerator::new(op);
        gen.operation.set_start_depth(self.start_depth);
        gen.operation.set_ramp_angle(self.ramp_angle);
        gen.operation.raster_fill_ratio = self.raster_fill_ratio;
        let effective_step_in = if step_in > 0.0 { step_in } else { self.step_in };
        gen.operation
            .set_parameters(effective_step_in, self.feed_rate, self.spindle_speed);
        gen.generate_circular_pocket(circle, step_down)
    }

pub fn generate_polyline_pocket(
    &self,
    vertices: &[Point],
    pocket_depth: f64,
    step_down: f64,
    step_in: f64,
) -> Vec<Toolpath> {
    let op = PocketOperation::new(
        "polyline_pocket".to_string(),
        pocket_depth,
        self.tool_diameter,
    );
    let mut gen = PocketGenerator::new(op);
    gen.operation.set_start_depth(self.start_depth);
    gen.operation.set_ramp_angle(self.ramp_angle);
    let effective_step_in = if step_in > 0.0 { step_in } else { self.step_in };
    gen.operation
        .set_parameters(effective_step_in, self.feed_rate, self.spindle_speed);
    gen.operation.set_strategy(self.pocket_strategy);
    gen.operation.raster_fill_ratio = self.raster_fill_ratio;

    // Generar toolpaths (el raster cleanup ya está desactivado en pocket_operations.rs)
    gen.generate_polygon_pocket(vertices, step_down)
}

    pub fn generate_triangle_pocket(
        &self,
        triangle: &Triangle,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let half_w = triangle.width / 2.0;
        let half_h = triangle.height / 2.0;

        let p1_local = Point::new(-half_w, -half_h);
        let p2_local = Point::new(half_w, -half_h);
        let p3_local = Point::new(-half_w, half_h);

        let rotation = triangle.rotation;
        let center = triangle.center;

        let transform_point = |p: Point| -> Point {
            let mut pt = p;
            if rotation.abs() > 1e-6 {
                pt = rotate_point(pt, Point::new(0.0, 0.0), rotation);
            }
            Point::new(pt.x + center.x, pt.y + center.y)
        };

        let vertices = vec![
            transform_point(p1_local),
            transform_point(p2_local),
            transform_point(p3_local),
        ];

        self.generate_polyline_pocket(&vertices, pocket_depth, step_down, step_in)
    }

    pub fn generate_polygon_pocket(
        &self,
        polygon: &Polygon,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let sides = polygon.sides.max(3);
        let rotation = polygon.rotation;
        let center = polygon.center;
        let radius = polygon.radius;

        let transform_point = |p: Point| -> Point {
            let mut pt = p;
            if rotation.abs() > 1e-6 {
                pt = rotate_point(pt, Point::new(0.0, 0.0), rotation);
            }
            Point::new(pt.x + center.x, pt.y + center.y)
        };

        let mut vertices = Vec::with_capacity(sides as usize);
        for i in 0..sides {
            let theta = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
            let x = radius * theta.cos();
            let y = radius * theta.sin();
            vertices.push(transform_point(Point::new(x, y)));
        }

        self.generate_polyline_pocket(&vertices, pocket_depth, step_down, step_in)
    }

    pub fn generate_path_pocket(
        &self,
        path_shape: &PathShape,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let mut vertices = Vec::new();
        let rect = lyon::algorithms::aabb::bounding_box(&path_shape.render());
        let center = Point::new(
            (rect.min.x + rect.max.x) as f64 / 2.0,
            (rect.min.y + rect.max.y) as f64 / 2.0,
        );
        let rotation = path_shape.rotation;

        for event in path_shape.render().iter() {
            match event {
                lyon::path::Event::Begin { at } => {
                    let mut p = Point::new(at.x as f64, at.y as f64);
                    if rotation.abs() > 1e-6 {
                        p = crate::model::rotate_point(p, center, rotation);
                    }
                    vertices.push(p);
                }
                lyon::path::Event::Line { from: _, to } => {
                    let mut p = Point::new(to.x as f64, to.y as f64);
                    if rotation.abs() > 1e-6 {
                        p = crate::model::rotate_point(p, center, rotation);
                    }
                    vertices.push(p);
                }
                _ => {}
            }
        }

        self.generate_polyline_pocket(&vertices, pocket_depth, step_down, step_in)
    }

    pub fn generate_gear_pocket(
        &self,
        gear: &DesignGear,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let path = gear.render();
        let path_shape = PathShape::from_lyon_path(&path);
        self.generate_path_pocket(&path_shape, pocket_depth, step_down, step_in)
    }

    pub fn generate_sprocket_pocket(
        &self,
        sprocket: &DesignSprocket,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let path = sprocket.render();
        let path_shape = PathShape::from_lyon_path(&path);
        self.generate_path_pocket(&path_shape, pocket_depth, step_down, step_in)
    }

    pub fn generate_text_pocket_toolpath(
        &self,
        text_shape: &TextShape,
        step_down: f64,
    ) -> Vec<Toolpath> {
        let (feed_rate, spindle_speed) = if text_shape.laser_params.use_global {
            (self.feed_rate, self.spindle_speed)
        } else {
            (
                text_shape.laser_params.feed_rate,
             (text_shape.laser_params.power_percent * 10.0) as u32,
            )
        };

        // Generar segmentos de contorno para cada letra
        let outline_segments = self.build_text_outline_segments_with_params(
            text_shape,
            feed_rate,
            spindle_speed,
        );

        if outline_segments.is_empty() {
            return Vec::new();
        }

        // Extraer contornos individuales (cada letra es un contorno)
        let contours = self.extract_text_contours(&outline_segments);

        if contours.is_empty() {
            // Fallback: usar contour normal
            return self.generate_text_toolpath(text_shape, step_down);
        }

        let stepover = if self.step_in > 1e-6 {
            self.step_in
        } else {
            (self.tool_diameter * 0.4).max(0.1)
        };

        let mut all_toolpaths = Vec::new();
        let pocket_depth = self.cut_depth.abs();

        // Procesar cada contorno (letra) individualmente
        for (i, contour) in contours.iter().enumerate() {
            if contour.len() < 3 {
                continue;
            }

            // Si el contorno es muy pequeño, usar contour en lugar de pocket
            let area = self.polygon_area(contour);
            let tool_area = std::f64::consts::PI * (self.tool_diameter / 2.0).powi(2);

            if area < tool_area || area < 0.01 {
                // Contorno pequeño: usar contour
                let toolpaths = self.create_multipass_toolpaths_from_contour(
                    contour,
                    step_down,
                    feed_rate,
                    spindle_speed,
                );
                all_toolpaths.extend(toolpaths);
                continue;
            }

            // Generar pocket para esta letra
            let op = PocketOperation::new(
                format!("text_pocket_{}", i),
                    pocket_depth,
                    self.tool_diameter,
            );
            let mut gen = PocketGenerator::new(op);
            gen.operation.set_start_depth(self.start_depth);
            gen.operation.set_ramp_angle(self.ramp_angle);
            gen.operation
            .set_parameters(stepover, feed_rate, spindle_speed);
            gen.operation.set_strategy(self.pocket_strategy);
            gen.operation.raster_fill_ratio = self.raster_fill_ratio;

            let pocket_toolpaths = gen.generate_polygon_pocket(contour, step_down);
            all_toolpaths.extend(pocket_toolpaths);
        }

        // Si no se generó nada, usar contour como fallback
        if all_toolpaths.is_empty() {
            return self.generate_text_toolpath(text_shape, step_down);
        }

        all_toolpaths
    }

    /// Extrae contornos individuales del texto (cada letra es un contorno separado)
    fn extract_text_contours(&self, segments: &[ToolpathSegment]) -> Vec<Vec<Point>> {
        let mut contours: Vec<Vec<Point>> = Vec::new();
        let mut current: Vec<Point> = Vec::new();

        for seg in segments {
            match seg.segment_type {
                ToolpathSegmentType::RapidMove => {
                    // Si tenemos un contorno en progreso, guardarlo
                    if !current.is_empty() && current.len() >= 3 {
                        // Cerrar el contorno
                        if let Some(first) = current.first() {
                            if let Some(last) = current.last() {
                                if last.distance_to(first) > 0.001 {
                                    current.push(*first);
                                }
                            }
                        }
                        contours.push(current.clone());
                        current.clear();
                    }
                    // Iniciar nuevo contorno con el punto de destino
                    current.push(seg.end);
                }
                ToolpathSegmentType::LinearMove => {
                    if current.is_empty() {
                        current.push(seg.start);
                    }
                    // Evitar duplicados
                    if let Some(last) = current.last() {
                        if last.distance_to(&seg.end) > 0.001 {
                            current.push(seg.end);
                        }
                    } else {
                        current.push(seg.end);
                    }
                }
                ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                    // Aproximar arcos con líneas
                    let steps = 12;
                    for i in 1..=steps {
                        let t = i as f64 / steps as f64;
                        let p = Point::new(
                            seg.start.x + t * (seg.end.x - seg.start.x),
                                           seg.start.y + t * (seg.end.y - seg.start.y),
                        );
                        if let Some(last) = current.last() {
                            if last.distance_to(&p) > 0.001 {
                                current.push(p);
                            }
                        }
                    }
                }
            }
        }

        // Guardar el último contorno
        if !current.is_empty() && current.len() >= 3 {
            if let Some(first) = current.first() {
                if let Some(last) = current.last() {
                    if last.distance_to(first) > 0.001 {
                        current.push(*first);
                    }
                }
            }
            contours.push(current);
        }

        contours
    }


    /// Calcula el área de un polígono
    fn polygon_area(&self, polygon: &[Point]) -> f64 {
        if polygon.len() < 3 {
            return 0.0;
        }
        let mut area = 0.0;
        let mut j = polygon.len() - 1;
        for i in 0..polygon.len() {
            area += (polygon[j].x + polygon[i].x) * (polygon[j].y - polygon[i].y);
            j = i;
        }
        area.abs() / 2.0
    }

    /// Genera toolpaths de contour para un contorno (cuando la letra es muy pequeña)
    fn create_multipass_toolpaths_from_contour(
        &self,
        contour: &[Point],
        step_down: f64,
        feed_rate: f64,
        spindle_speed: u32,
    ) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        if contour.len() < 2 {
            return Vec::new();
        }

        // Crear segmentos para el contorno
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            contour[0],
            contour[0],
            feed_rate,
            spindle_speed,
        ));

        for i in 0..contour.len() {
            let next_i = (i + 1) % contour.len();
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                contour[i],
                contour[next_i],
                feed_rate,
                spindle_speed,
            ));
        }

        self.create_multipass_toolpaths(segments, step_down)
    }

    // ==================== AUXILIARY METHODS ====================

    fn try_convert_quadratic_to_arc(
        &self,
        start: Point,
        end: Point,
        ctrl: Point,
    ) -> Option<(Point, bool)> {
        let mid_start_ctrl = Point::new((start.x + ctrl.x) / 2.0, (start.y + ctrl.y) / 2.0);
        let mid_ctrl_end = Point::new((ctrl.x + end.x) / 2.0, (ctrl.y + end.y) / 2.0);
        let center_candidate = Point::new(
            (mid_start_ctrl.x + mid_ctrl_end.x) / 2.0,
            (mid_start_ctrl.y + mid_ctrl_end.y) / 2.0,
        );
        let r_start = start.distance_to(&center_candidate);
        let r_end = end.distance_to(&center_candidate);
        let chord_len = start.distance_to(&end);
        let tolerance = (chord_len * 0.05).max(0.01);
        if (r_start - r_end).abs() < tolerance {
            let v1 = (start.x - center_candidate.x, start.y - center_candidate.y);
            let v2 = (end.x - center_candidate.x, end.y - center_candidate.y);
            let cross = v1.0 * v2.1 - v1.1 * v2.0;
            Some((center_candidate, cross > 0.0))
        } else {
            None
        }
    }

    fn try_convert_to_arc(
        &self,
        start: Point,
        end: Point,
        ctrl1: Point,
        ctrl2: Point,
    ) -> Option<(Point, bool)> {
        let center_candidate = Point::new((ctrl1.x + ctrl2.x) / 2.0, (ctrl1.y + ctrl2.y) / 2.0);
        let r_start = start.distance_to(&center_candidate);
        let r_end = end.distance_to(&center_candidate);
        let r_ctrl1 = ctrl1.distance_to(&center_candidate);
        let r_ctrl2 = ctrl2.distance_to(&center_candidate);
        let r_avg = (r_start + r_end + r_ctrl1 + r_ctrl2) / 4.0;
        let chord_len = start.distance_to(&end);
        let tolerance = (chord_len * 0.05).max(0.01);
        if (r_start - r_avg).abs() < tolerance
            && (r_end - r_avg).abs() < tolerance
            && (r_ctrl1 - r_avg).abs() < tolerance
            && (r_ctrl2 - r_avg).abs() < tolerance
        {
            let v_start = (start.x - center_candidate.x, start.y - center_candidate.y);
            let v_end = (end.x - center_candidate.x, end.y - center_candidate.y);
            let cross = v_start.0 * v_end.1 - v_start.1 * v_end.0;
            Some((center_candidate, cross > 0.0))
        } else {
            None
        }
    }
}

struct ToolpathBuilder {
    segments: Vec<ToolpathSegment>,
    current_point: Point,
    start_point: Point,
    started: bool,
    feed_rate: f64,
    spindle_speed: u32,
    offset: Point,
    rotation_center: Point,
    rotation_deg: f64,
}

impl ToolpathBuilder {
    fn new(
        feed_rate: f64,
        spindle_speed: u32,
        initial_point: Point,
        offset: Point,
        rotation_center: Point,
        rotation_deg: f64,
    ) -> Self {
        Self {
            segments: Vec::new(),
            current_point: initial_point,
            start_point: initial_point,
            started: false,
            feed_rate,
            spindle_speed,
            offset,
            rotation_center,
            rotation_deg,
        }
    }

    fn start_point_opt(&self) -> Option<Point> {
        if self.started {
            Some(self.start_point)
        } else {
            None
        }
    }

    fn take_segments(&mut self) -> Vec<ToolpathSegment> {
        std::mem::take(&mut self.segments)
    }

    fn map_point(&self, x: f32, y: f32) -> Point {
        let p = Point::new(x as f64 + self.offset.x, self.offset.y - y as f64);
        rotate_point(p, self.rotation_center, -self.rotation_deg)
    }
}

impl OutlineBuilder for ToolpathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.map_point(x, y);
        if !self.started {
            self.started = true;
        }
        self.segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            self.current_point,
            p,
            self.feed_rate,
            self.spindle_speed,
        ));
        self.current_point = p;
        self.start_point = p;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.map_point(x, y);
        self.segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            self.current_point,
            p,
            self.feed_rate,
            self.spindle_speed,
        ));
        self.current_point = p;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p0 = self.current_point;
        let p1 = self.map_point(x1, y1);
        let p2 = self.map_point(x, y);
        let approx_len = (p0.x - p1.x).hypot(p0.y - p1.y) + (p1.x - p2.x).hypot(p1.y - p2.y);
        let max_seg_len = 0.5_f64;
        let steps = ((approx_len / max_seg_len).ceil() as usize).clamp(4, 64);
        for i in 1..=steps {
            let t = (i as f64) / (steps as f64);
            let mt = 1.0 - t;
            let px = mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x;
            let py = mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y;
            let p = Point::new(px, py);
            self.segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                self.current_point,
                p,
                self.feed_rate,
                self.spindle_speed,
            ));
            self.current_point = p;
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p0 = self.current_point;
        let p1 = self.map_point(x1, y1);
        let p2 = self.map_point(x2, y2);
        let p3 = self.map_point(x, y);
        let approx_len = (p0.x - p1.x).hypot(p0.y - p1.y)
            + (p1.x - p2.x).hypot(p1.y - p2.y)
            + (p2.x - p3.x).hypot(p2.y - p3.y);
        let max_seg_len = 0.5_f64;
        let steps = ((approx_len / max_seg_len).ceil() as usize).clamp(8, 128);
        for i in 1..=steps {
            let t = (i as f64) / (steps as f64);
            let mt = 1.0 - t;
            let px = mt * mt * mt * p0.x
                + 3.0 * mt * mt * t * p1.x
                + 3.0 * mt * t * t * p2.x
                + t * t * t * p3.x;
            let py = mt * mt * mt * p0.y
                + 3.0 * mt * mt * t * p1.y
                + 3.0 * mt * t * t * p2.y
                + t * t * t * p3.y;
            let p = Point::new(px, py);
            self.segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                self.current_point,
                p,
                self.feed_rate,
                self.spindle_speed,
            ));
            self.current_point = p;
        }
    }

    fn close(&mut self) {
        if !self.started {
            return;
        }
        if self.current_point.distance_to(&self.start_point) > 1e-6 {
            self.segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                self.current_point,
                self.start_point,
                self.feed_rate,
                self.spindle_speed,
            ));
        }
        self.current_point = self.start_point;
    }
}
