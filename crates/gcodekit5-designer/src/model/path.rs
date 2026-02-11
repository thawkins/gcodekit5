use lyon::math::{point, Transform};
use lyon::path::iterator::*;
use lyon::path::Path;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use csgrs::io::svg::{FromSVG, ToSVG};
use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::Matrix4;

use super::{DesignerShape, Point, Property, PropertyValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignPath {
    #[serde(
        serialize_with = "serialize_sketch",
        deserialize_with = "deserialize_sketch"
    )]
    pub sketch: Sketch<()>,
    pub rotation: f64,
}

impl DesignPath {
    pub fn from_csg(sketch: Sketch<()>) -> Self {
        Self {
            sketch,
            rotation: 0.0,
        }
    }

    pub fn from_svg_path(d: &str) -> Option<Self> {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000" viewBox="0 0 1000 1000"><path d="{}"/></svg>"#,
            d
        );
        if let Ok(sketch) = Sketch::from_svg(&svg) {
            return Some(Self {
                sketch,
                rotation: 0.0,
            });
        }

        let lyon_path = Self::build_lyon_path_from_svg_data(d)?;
        Some(Self::from_lyon_path(&lyon_path))
    }

    pub fn from_points(points: &[Point], _closed: bool) -> Self {
        let pts: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
        let sketch = Sketch::polygon(&pts, None);
        Self {
            sketch,
            rotation: 0.0,
        }
    }

    pub fn from_lyon_path(path: &Path) -> Self {
        let tolerance = 0.1;
        let flattened = path.iter().flattened(tolerance);
        let mut polygons: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut current_poly: Vec<[f64; 2]> = Vec::new();

        for event in flattened {
            match event {
                lyon::path::Event::Begin { at } => {
                    current_poly.clear();
                    current_poly.push([at.x as f64, at.y as f64]);
                }
                lyon::path::Event::Line { to, .. } => {
                    current_poly.push([to.x as f64, to.y as f64]);
                }
                lyon::path::Event::End { .. } => {
                    if !current_poly.is_empty() {
                        polygons.push(current_poly.clone());
                    }
                }
                _ => {}
            }
        }

        if polygons.is_empty() && !current_poly.is_empty() {
            polygons.push(current_poly);
        }

        let mut sketch = Sketch::new();
        for poly in polygons {
            let s = Sketch::polygon(&poly, None);
            sketch = sketch.union(&s);
        }

        Self {
            sketch,
            rotation: 0.0,
        }
    }

    pub fn to_svg_path(&self) -> String {
        let path = self.render();
        let mut svg = String::new();
        for event in path.iter() {
            match event {
                lyon::path::Event::Begin { at } => svg.push_str(&format!("M {} {} ", at.x, at.y)),
                lyon::path::Event::Line { to, .. } => {
                    svg.push_str(&format!("L {} {} ", to.x, to.y))
                }
                lyon::path::Event::Quadratic { ctrl, to, .. } => {
                    svg.push_str(&format!("Q {} {} {} {} ", ctrl.x, ctrl.y, to.x, to.y))
                }
                lyon::path::Event::Cubic {
                    ctrl1, ctrl2, to, ..
                } => svg.push_str(&format!(
                    "C {} {} {} {} {} {} ",
                    ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y
                )),
                lyon::path::Event::End { close, .. } => {
                    if close {
                        svg.push_str("Z ");
                    }
                }
            }
        }
        svg
    }

    /// Build a `lyon::path::Path` from SVG path data.
    ///
    /// This is a fallback for cases where `csgrs::Sketch::from_svg` cannot parse
    /// certain SVG path features. It supports a practical subset of SVG commands:
    /// `m/l/h/v/c/s/q/t/a/z` (and their uppercase forms).
    fn build_lyon_path_from_svg_data(data_str: &str) -> Option<Path> {
        let mut builder = Path::builder();
        let mut current_x = 0.0f32;
        let mut current_y = 0.0f32;
        let mut start_x = 0.0f32;
        let mut start_y = 0.0f32;
        let mut subpath_active = false;

        // Previous control points for smooth commands.
        let mut prev_cubic_ctrl: Option<(f32, f32)> = None;
        let mut prev_quad_ctrl: Option<(f32, f32)> = None;
        let mut prev_cmd: Option<char> = None;

        let tokens = Self::tokenize_svg_path(data_str);
        let mut i = 0usize;

        fn is_cmd_token(s: &str) -> bool {
            s.len() == 1
                && s.chars()
                    .next()
                    .map(|c| c.is_ascii_alphabetic())
                    .unwrap_or(false)
        }

        fn parse_f32(s: &str) -> Option<f32> {
            s.parse::<f32>().ok()
        }

        fn reflect(p: (f32, f32), around: (f32, f32)) -> (f32, f32) {
            (2.0 * around.0 - p.0, 2.0 * around.1 - p.1)
        }

        fn angle_between(u: (f32, f32), v: (f32, f32)) -> f32 {
            let dot = u.0 * v.0 + u.1 * v.1;
            let det = u.0 * v.1 - u.1 * v.0;
            det.atan2(dot)
        }

        fn unit_vector_angle(v: (f32, f32)) -> f32 {
            angle_between((1.0, 0.0), v)
        }

        #[allow(clippy::too_many_arguments)]
        fn ellipse_transform_point(
            cx: f32,
            cy: f32,
            rx: f32,
            ry: f32,
            cos_phi: f32,
            sin_phi: f32,
            u: f32,
            v: f32,
        ) -> (f32, f32) {
            // [x;y] = [cx;cy] + R(phi) * [rx*u; ry*v]
            let x = cx + cos_phi * (rx * u) - sin_phi * (ry * v);
            let y = cy + sin_phi * (rx * u) + cos_phi * (ry * v);
            (x, y)
        }

        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn arc_to_cubics(
            x1: f32,
            y1: f32,
            x2: f32,
            y2: f32,
            mut rx: f32,
            mut ry: f32,
            phi_deg: f32,
            large_arc: bool,
            sweep: bool,
        ) -> Option<Vec<((f32, f32), (f32, f32), (f32, f32))>> {
            if rx.abs() < f32::EPSILON || ry.abs() < f32::EPSILON {
                return Some(vec![((x1, y1), (x2, y2), (x2, y2))]);
            }

            rx = rx.abs();
            ry = ry.abs();

            let phi = phi_deg.to_radians();
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();

            // Step 1: Compute (x1', y1')
            let dx2 = (x1 - x2) / 2.0;
            let dy2 = (y1 - y2) / 2.0;
            let x1p = cos_phi * dx2 + sin_phi * dy2;
            let y1p = -sin_phi * dx2 + cos_phi * dy2;

            // Step 2: Ensure radii are large enough
            let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
            if lambda > 1.0 {
                let scale = lambda.sqrt();
                rx *= scale;
                ry *= scale;
            }

            // Step 3: Compute (cx', cy')
            let rx2 = rx * rx;
            let ry2 = ry * ry;
            let x1p2 = x1p * x1p;
            let y1p2 = y1p * y1p;
            let denom = rx2 * y1p2 + ry2 * x1p2;
            if denom.abs() < f32::EPSILON {
                return None;
            }

            let mut numer = rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2;
            if numer < 0.0 {
                // Numeric precision; clamp.
                numer = 0.0;
            }

            let sign = if large_arc == sweep { -1.0 } else { 1.0 };
            let coef = sign * (numer / denom).sqrt();
            let cxp = coef * (rx * y1p / ry);
            let cyp = coef * (-ry * x1p / rx);

            // Step 4: Compute (cx, cy)
            let cx = cos_phi * cxp - sin_phi * cyp + (x1 + x2) / 2.0;
            let cy = sin_phi * cxp + cos_phi * cyp + (y1 + y2) / 2.0;

            // Step 5: Angles
            let ux = (x1p - cxp) / rx;
            let uy = (y1p - cyp) / ry;
            let vx = (-x1p - cxp) / rx;
            let vy = (-y1p - cyp) / ry;

            let mut theta1 = unit_vector_angle((ux, uy));
            let mut delta = angle_between((ux, uy), (vx, vy));

            if !sweep && delta > 0.0 {
                delta -= std::f32::consts::TAU;
            } else if sweep && delta < 0.0 {
                delta += std::f32::consts::TAU;
            }

            // Step 6: Split into <= 90deg segments
            let segment_count = (delta.abs() / (std::f32::consts::FRAC_PI_2)).ceil() as i32;
            let segment_count = segment_count.max(1);
            let delta_seg = delta / (segment_count as f32);

            let mut cubics = Vec::with_capacity(segment_count as usize);
            for _ in 0..segment_count {
                let t0 = theta1;
                let t1 = theta1 + delta_seg;
                let dt = t1 - t0;

                let k = 4.0 / 3.0 * (dt / 4.0).tan();

                // Unit circle points
                let (c0, s0) = (t0.cos(), t0.sin());
                let (c1, s1) = (t1.cos(), t1.sin());
                let p0 = (c0, s0);
                let p3 = (c1, s1);
                let p1 = (c0 - k * s0, s0 + k * c0);
                let p2 = (c1 + k * s1, s1 - k * c1);

                // Transform to ellipse
                let cp1 = ellipse_transform_point(cx, cy, rx, ry, cos_phi, sin_phi, p1.0, p1.1);
                let cp2 = ellipse_transform_point(cx, cy, rx, ry, cos_phi, sin_phi, p2.0, p2.1);
                let end = ellipse_transform_point(cx, cy, rx, ry, cos_phi, sin_phi, p3.0, p3.1);
                let _start = ellipse_transform_point(cx, cy, rx, ry, cos_phi, sin_phi, p0.0, p0.1);

                cubics.push((cp1, cp2, end));
                theta1 = t1;
            }

            Some(cubics)
        }

        while i < tokens.len() {
            let cmd_token = &tokens[i];
            if !is_cmd_token(cmd_token) {
                i += 1;
                continue;
            }

            let cmd = cmd_token.chars().next()?;
            let is_relative = cmd.is_ascii_lowercase();
            let cmd_upper = cmd.to_ascii_uppercase();
            i += 1;

            match cmd_upper {
                'M' => {
                    // One or more pairs; first is moveto, rest are implicit lineto.
                    let mut first = true;
                    while i + 1 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x = parse_f32(&tokens[i])?;
                        let y = parse_f32(&tokens[i + 1])?;
                        i += 2;

                        let nx = if is_relative { current_x + x } else { x };
                        let ny = if is_relative { current_y + y } else { y };

                        if first {
                            if subpath_active {
                                builder.end(false);
                            }
                            builder.begin(point(nx, ny));
                            subpath_active = true;
                            start_x = nx;
                            start_y = ny;
                            first = false;
                        } else {
                            if !subpath_active {
                                builder.begin(point(current_x, current_y));
                                subpath_active = true;
                                start_x = current_x;
                                start_y = current_y;
                            }
                            builder.line_to(point(nx, ny));
                        }

                        current_x = nx;
                        current_y = ny;
                    }
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                'L' => {
                    while i + 1 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x = parse_f32(&tokens[i])?;
                        let y = parse_f32(&tokens[i + 1])?;
                        i += 2;

                        let nx = if is_relative { current_x + x } else { x };
                        let ny = if is_relative { current_y + y } else { y };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.line_to(point(nx, ny));
                        current_x = nx;
                        current_y = ny;
                    }
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                'H' => {
                    while i < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x = parse_f32(&tokens[i])?;
                        i += 1;
                        let nx = if is_relative { current_x + x } else { x };
                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.line_to(point(nx, current_y));
                        current_x = nx;
                    }
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                'V' => {
                    while i < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let y = parse_f32(&tokens[i])?;
                        i += 1;
                        let ny = if is_relative { current_y + y } else { y };
                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.line_to(point(current_x, ny));
                        current_y = ny;
                    }
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                'C' => {
                    while i + 5 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x1 = parse_f32(&tokens[i])?;
                        let y1 = parse_f32(&tokens[i + 1])?;
                        let x2 = parse_f32(&tokens[i + 2])?;
                        let y2 = parse_f32(&tokens[i + 3])?;
                        let x = parse_f32(&tokens[i + 4])?;
                        let y = parse_f32(&tokens[i + 5])?;
                        i += 6;

                        let (cp1_x, cp1_y, cp2_x, cp2_y, end_x, end_y) = if is_relative {
                            (
                                current_x + x1,
                                current_y + y1,
                                current_x + x2,
                                current_y + y2,
                                current_x + x,
                                current_y + y,
                            )
                        } else {
                            (x1, y1, x2, y2, x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.cubic_bezier_to(
                            point(cp1_x, cp1_y),
                            point(cp2_x, cp2_y),
                            point(end_x, end_y),
                        );
                        current_x = end_x;
                        current_y = end_y;
                        prev_cubic_ctrl = Some((cp2_x, cp2_y));
                        prev_quad_ctrl = None;
                    }
                }
                'S' => {
                    while i + 3 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x2 = parse_f32(&tokens[i])?;
                        let y2 = parse_f32(&tokens[i + 1])?;
                        let x = parse_f32(&tokens[i + 2])?;
                        let y = parse_f32(&tokens[i + 3])?;
                        i += 4;

                        let cp1 = if matches!(prev_cmd, Some('C' | 'c' | 'S' | 's')) {
                            if let Some(prev) = prev_cubic_ctrl {
                                reflect(prev, (current_x, current_y))
                            } else {
                                (current_x, current_y)
                            }
                        } else {
                            (current_x, current_y)
                        };

                        let (cp2_x, cp2_y, end_x, end_y) = if is_relative {
                            (current_x + x2, current_y + y2, current_x + x, current_y + y)
                        } else {
                            (x2, y2, x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.cubic_bezier_to(
                            point(cp1.0, cp1.1),
                            point(cp2_x, cp2_y),
                            point(end_x, end_y),
                        );
                        current_x = end_x;
                        current_y = end_y;
                        prev_cubic_ctrl = Some((cp2_x, cp2_y));
                        prev_quad_ctrl = None;
                    }
                }
                'Q' => {
                    while i + 3 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x1 = parse_f32(&tokens[i])?;
                        let y1 = parse_f32(&tokens[i + 1])?;
                        let x = parse_f32(&tokens[i + 2])?;
                        let y = parse_f32(&tokens[i + 3])?;
                        i += 4;

                        let (cp_x, cp_y, end_x, end_y) = if is_relative {
                            (current_x + x1, current_y + y1, current_x + x, current_y + y)
                        } else {
                            (x1, y1, x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.quadratic_bezier_to(point(cp_x, cp_y), point(end_x, end_y));
                        current_x = end_x;
                        current_y = end_y;
                        prev_quad_ctrl = Some((cp_x, cp_y));
                        prev_cubic_ctrl = None;
                    }
                }
                'T' => {
                    while i + 1 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let x = parse_f32(&tokens[i])?;
                        let y = parse_f32(&tokens[i + 1])?;
                        i += 2;

                        let cp = if matches!(prev_cmd, Some('Q' | 'q' | 'T' | 't')) {
                            if let Some(prev) = prev_quad_ctrl {
                                reflect(prev, (current_x, current_y))
                            } else {
                                (current_x, current_y)
                            }
                        } else {
                            (current_x, current_y)
                        };

                        let (end_x, end_y) = if is_relative {
                            (current_x + x, current_y + y)
                        } else {
                            (x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }
                        builder.quadratic_bezier_to(point(cp.0, cp.1), point(end_x, end_y));
                        current_x = end_x;
                        current_y = end_y;
                        prev_quad_ctrl = Some(cp);
                        prev_cubic_ctrl = None;
                    }
                }
                'A' => {
                    while i + 6 < tokens.len() && !is_cmd_token(&tokens[i]) {
                        let rx = parse_f32(&tokens[i])?;
                        let ry = parse_f32(&tokens[i + 1])?;
                        let x_axis_rotation = parse_f32(&tokens[i + 2])?;
                        let large_arc_flag = tokens[i + 3].parse::<i32>().ok()? != 0;
                        let sweep_flag = tokens[i + 4].parse::<i32>().ok()? != 0;
                        let x = parse_f32(&tokens[i + 5])?;
                        let y = parse_f32(&tokens[i + 6])?;
                        i += 7;

                        let (end_x, end_y) = if is_relative {
                            (current_x + x, current_y + y)
                        } else {
                            (x, y)
                        };

                        if !subpath_active {
                            builder.begin(point(current_x, current_y));
                            subpath_active = true;
                            start_x = current_x;
                            start_y = current_y;
                        }

                        if let Some(cubics) = arc_to_cubics(
                            current_x,
                            current_y,
                            end_x,
                            end_y,
                            rx,
                            ry,
                            x_axis_rotation,
                            large_arc_flag,
                            sweep_flag,
                        ) {
                            for (cp1, cp2, end) in cubics {
                                builder.cubic_bezier_to(
                                    point(cp1.0, cp1.1),
                                    point(cp2.0, cp2.1),
                                    point(end.0, end.1),
                                );
                            }
                        } else {
                            builder.line_to(point(end_x, end_y));
                        }

                        current_x = end_x;
                        current_y = end_y;
                        prev_cubic_ctrl = None;
                        prev_quad_ctrl = None;
                    }
                }
                'Z' => {
                    if subpath_active {
                        builder.close();
                        subpath_active = false;
                    }
                    current_x = start_x;
                    current_y = start_y;
                    prev_cubic_ctrl = None;
                    prev_quad_ctrl = None;
                }
                _ => {
                    // Unsupported command - bail out so caller can treat as unparseable.
                    return None;
                }
            }

            prev_cmd = Some(cmd);
        }

        if subpath_active {
            builder.end(false);
        }

        Some(builder.build())
    }

    /// Tokenize SVG path data into commands and numeric strings.
    ///
    /// This handles commas/whitespace and also splits on `+`/`-` when they begin a
    /// new number (e.g. `10-5` -> `10`, `-5`), while preserving scientific notation.
    fn tokenize_svg_path(path_data: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();

        for ch in path_data.chars() {
            match ch {
                'M' | 'm' | 'L' | 'l' | 'H' | 'h' | 'V' | 'v' | 'C' | 'c' | 'S' | 's' | 'Q'
                | 'q' | 'T' | 't' | 'A' | 'a' | 'Z' | 'z' => {
                    if !current_token.is_empty() {
                        tokens.push(std::mem::take(&mut current_token));
                    }
                    tokens.push(ch.to_string());
                }
                ' ' | ',' | '\n' | '\r' | '\t' => {
                    if !current_token.is_empty() {
                        tokens.push(std::mem::take(&mut current_token));
                    }
                }
                '-' | '+' => {
                    if current_token.is_empty() {
                        current_token.push(ch);
                        continue;
                    }

                    // If the previous char indicates scientific notation, keep the sign.
                    if matches!(current_token.chars().last(), Some('e' | 'E')) {
                        current_token.push(ch);
                    } else {
                        tokens.push(std::mem::take(&mut current_token));
                        current_token.push(ch);
                    }
                }
                _ => current_token.push(ch),
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        tokens
    }
}

fn serialize_sketch<S>(sketch: &Sketch<()>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let svg = sketch.to_svg();
    serializer.serialize_str(&svg)
}

fn deserialize_sketch<'de, D>(deserializer: D) -> Result<Sketch<()>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Sketch::from_svg(&s).map_err(serde::de::Error::custom)
}

impl DesignerShape for DesignPath {
    fn render(&self) -> Path {
        let mut builder = Path::builder();

        let mp = self.sketch.to_multipolygon();
        for poly in mp.0 {
            let exterior = poly.exterior();
            let mut first = true;
            for coord in exterior.0.iter() {
                let p = point(coord.x as f32, coord.y as f32);
                if first {
                    builder.begin(p);
                    first = false;
                } else {
                    builder.line_to(p);
                }
            }
            builder.close();

            for interior in poly.interiors() {
                let mut first = true;
                for coord in interior.0.iter() {
                    let p = point(coord.x as f32, coord.y as f32);
                    if first {
                        builder.begin(p);
                        first = false;
                    } else {
                        builder.line_to(p);
                    }
                }
                builder.close();
            }
        }

        builder.build()
    }

    fn as_csg(&self) -> Sketch<()> {
        self.sketch.clone()
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        let bb = CSG::bounding_box(&self.sketch);
        (bb.mins.x, bb.mins.y, bb.maxs.x, bb.maxs.y)
    }

    fn transform(&mut self, t: &Transform) {
        let m = Matrix4::new(
            t.m11 as f64,
            t.m21 as f64,
            0.0,
            t.m31 as f64,
            t.m12 as f64,
            t.m22 as f64,
            0.0,
            t.m32 as f64,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        );

        self.sketch = self.sketch.transform(&m);

        let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64;
        self.rotation += angle_deg;
    }

    fn properties(&self) -> Vec<Property> {
        vec![
            Property {
                name: "Type".to_string(),
                value: PropertyValue::String("Path".to_string()),
            },
            Property {
                name: "Rotation".to_string(),
                value: PropertyValue::Number(self.rotation),
            },
        ]
    }

    fn contains_point(&self, p: Point, tolerance: f64) -> bool {
        let (x1, y1, x2, y2) = self.bounds();
        p.x >= x1 - tolerance
            && p.x <= x2 + tolerance
            && p.y >= y1 - tolerance
            && p.y <= y2 + tolerance
    }

    fn resize(&mut self, handle: usize, dx: f64, dy: f64) {
        if handle == 4 {
            self.translate(dx, dy);
            return;
        }
        let (x1, y1, x2, y2) = self.bounds();
        let w = x2 - x1;
        let h = y2 - y1;

        let (new_x1, new_y1, new_x2, new_y2) = match handle {
            0 => (x1 + dx, y1 + dy, x2, y2),
            1 => (x1, y1 + dy, x2 + dx, y2),
            2 => (x1 + dx, y1, x2, y2 + dy),
            3 => (x1, y1, x2 + dx, y2 + dy),
            _ => (x1, y1, x2, y2),
        };

        let new_w = (new_x2 - new_x1).abs();
        let new_h = (new_y2 - new_y1).abs();

        let sx = if w.abs() > 1e-6 { new_w / w } else { 1.0 };
        let sy = if h.abs() > 1e-6 { new_h / h } else { 1.0 };

        let cx = (x1 + x2) / 2.0;
        let cy = (y1 + y2) / 2.0;

        self.scale(sx, sy, Point::new(cx, cy));

        let new_cx = (new_x1 + new_x2) / 2.0;
        let new_cy = (new_y1 + new_y2) / 2.0;
        self.translate(new_cx - cx, new_cy - cy);
    }
}
