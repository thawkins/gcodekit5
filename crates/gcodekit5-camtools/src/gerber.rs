//! # Gerber File Converter
//!
//! Converts Gerber (RS-274X) PCB layout files to G-code toolpaths.
//! Supports copper layer isolation routing, rubout/hatch fill,
//! and edge-cut operations using CSG boolean geometry.
//!
//! Apertures: circle, rectangle, obround and regular-polygon apertures are supported for
//! flashes and for linear draws (a linear draw is the convex hull of the aperture at both
//! ends, which is exact for these convex shapes). Circular-arc draws are only defined for
//! circular apertures by the Gerber specification; a non-circular aperture on an arc is
//! approximated by a round stroke of its smallest dimension. Aperture macros and aperture
//! holes are not supported.

use crate::hatch_generator;
use anyhow::Result;
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use gerber_parser::parse;
use gerber_types::{
    Command, CoordinateNumber, DCode, FunctionCode, InterpolationMode, Operation, QuadrantMode,
    Unit,
};
use lyon::math::point;
use lyon::path::Path as LyonPath;
use nalgebra::{Matrix4, Vector3};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::fs;
use std::io::BufReader;
use std::panic;
use std::path::PathBuf;
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GerberLayerType {
    TopCopper,
    BottomCopper,
    TopSolderMask,
    BottomSolderMask,
    TopScreenPrint,
    BottomScreenPrint,
    DrillHoles,
    BoardOutline,
}

impl std::fmt::Display for GerberLayerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GerberLayerType::TopCopper => write!(f, "Top Copper"),
            GerberLayerType::BottomCopper => write!(f, "Bottom Copper"),
            GerberLayerType::TopSolderMask => write!(f, "Top Solder Mask"),
            GerberLayerType::BottomSolderMask => write!(f, "Bottom Solder Mask"),
            GerberLayerType::TopScreenPrint => write!(f, "Top Screen Print"),
            GerberLayerType::BottomScreenPrint => write!(f, "Bottom Screen Print"),
            GerberLayerType::DrillHoles => write!(f, "Drill Holes"),
            GerberLayerType::BoardOutline => write!(f, "Board Outline"),
        }
    }
}

pub fn clean_polyline(mut pline: Polyline<f64>) -> Polyline<f64> {
    pline.remove_repeat_pos(1e-5);
    if pline.is_closed()
        && pline.vertex_count() > 1
        && let (Some(first), Some(last)) = (pline.get(0), pline.get(pline.vertex_count() - 1))
        && (first.x - last.x).abs() < 1e-5
        && (first.y - last.y).abs() < 1e-5
    {
        pline.remove(pline.vertex_count() - 1);
    }
    pline
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GerberParameters {
    pub layer_type: GerberLayerType,
    pub feed_rate: f32,
    pub spindle_speed: f32,
    pub board_width: f32,
    pub board_height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub generate_alignment_holes: bool,
    pub alignment_hole_diameter: f32,
    pub alignment_hole_margin: f32,
    /// Z depth of the alignment holes (mm, negative = below the surface). Separate from
    /// `cut_depth`, which is only the isolation depth; holes normally go through the board.
    pub alignment_hole_depth: f32,
    pub cut_depth: f32,
    pub safe_z: f32,
    pub tool_diameter: f32,
    pub isolation_width: f32,
    pub rubout: bool,
    pub use_board_outline: bool,
    pub directory: Option<String>,
    /// Number of axes on the target device (default 3).
    #[serde(default = "default_gerber_num_axes")]
    pub num_axes: u8,
}

fn default_gerber_num_axes() -> u8 {
    3
}

impl Default for GerberParameters {
    fn default() -> Self {
        Self {
            layer_type: GerberLayerType::TopCopper,
            feed_rate: 100.0,
            spindle_speed: 10000.0,
            board_width: 100.0,
            board_height: 100.0,
            offset_x: 0.0,
            offset_y: 0.0,
            generate_alignment_holes: false,
            alignment_hole_diameter: 3.175,
            alignment_hole_margin: 5.0,
            // Through a 1.6 mm board plus a little into the spoilboard.
            alignment_hole_depth: -1.8,
            cut_depth: -0.1,
            safe_z: 5.0,
            tool_diameter: 0.1,
            isolation_width: 0.0,
            rubout: false,
            use_board_outline: false,
            directory: None,
            num_axes: 3,
        }
    }
}

pub struct GerberConverter;

impl GerberConverter {
    pub fn generate(params: &GerberParameters, gerber_content: &str) -> Result<String> {
        let mut gcode = String::new();

        // Initialization sequence
        writeln!(gcode, "; Initialization sequence")?;
        writeln!(gcode, "G21 ; Set units to millimeters")?;
        writeln!(gcode, "G90 ; Absolute positioning")?;
        writeln!(gcode, "G17 ; XY plane selection")?;
        writeln!(gcode)?;

        // Work zero is set at the current (jogged) tool position. The X/Y offsets are applied
        // to every emitted coordinate, so they must not also be baked into the work origin.
        writeln!(gcode, "; Set work coordinate system at current position")?;
        writeln!(gcode, "G54 ; Select work coordinate system 1")?;
        if params.num_axes >= 3 {
            writeln!(
                gcode,
                "G10 L20 P1 X0 Y0 Z0 ; Set current position as work zero"
            )?;
        } else {
            writeln!(
                gcode,
                "G10 L20 P1 X0 Y0 ; Set current position as work zero"
            )?;
        }
        if params.num_axes >= 3 {
            writeln!(gcode, "G0 Z{:.3} F500 ; Move to safe height", params.safe_z)?;
        }

        writeln!(gcode, "M3 S{:.1}", params.spindle_speed)?;

        // Unified parsing logic using gerber_parser
        let sketches = Self::parse_gerber_to_sketches(gerber_content)?;

        // If it's a copper layer, we perform isolation routing (Union + Offset)
        // Otherwise (Outline, Silk, Drill), we just trace the paths (Engrave/Cut)
        let is_copper = matches!(
            params.layer_type,
            GerberLayerType::TopCopper | GerberLayerType::BottomCopper
        );

        if is_copper {
            Self::generate_isolation_from_sketches(params, sketches.clone(), &mut gcode)?;
            if params.rubout {
                writeln!(gcode, "; Rubout (Remove Excess Copper)")?;
                Self::generate_rubout_from_sketches(params, sketches, &mut gcode)?;
            }
        } else {
            Self::generate_simple_trace_from_sketches(params, sketches, &mut gcode)?;
        }

        Self::append_alignment_holes(params, &mut gcode)?;

        writeln!(gcode, "M5")?;
        writeln!(gcode, "G0 X0 Y0")?;

        Ok(gcode)
    }

    /// Signed parallel-offset distance that moves a copper boundary away from the copper.
    ///
    /// cavalier_contours offsets positive values to the left of the travel direction, so the
    /// sign depends on ring orientation. Exterior rings must grow outward and hole rings must
    /// shrink into the hole; both mean "away from copper".
    ///
    /// # Arguments
    /// * `poly` - Closed boundary ring
    /// * `is_hole` - Whether the ring is an interior (hole) ring
    /// * `distance` - Unsigned isolation distance in mm
    ///
    /// # Returns
    /// The signed offset to pass to `parallel_offset`.
    fn outward_offset(poly: &Polyline<f64>, is_hole: bool, distance: f64) -> f64 {
        let ccw = poly.area() > 0.0;
        if is_hole == ccw { distance } else { -distance }
    }

    /// Axis-aligned rectangle covering `sketch`'s bounds grown by `margin` on every side.
    ///
    /// # Returns
    /// `None` if the sketch is empty.
    fn bounds_rectangle(sketch: &Sketch<()>, margin: f64) -> Option<Sketch<()>> {
        let bb = sketch.bounding_box();
        if !(bb.mins.x.is_finite() && bb.maxs.x.is_finite()) || bb.mins.x > bb.maxs.x {
            return None;
        }
        let (x0, y0) = (bb.mins.x - margin, bb.mins.y - margin);
        let (w, h) = (
            bb.maxs.x - bb.mins.x + 2.0 * margin,
            bb.maxs.y - bb.mins.y + 2.0 * margin,
        );
        let rect: Sketch<()> = Sketch::rectangle(w, h, None);
        Some(rect.transform(&Matrix4::new_translation(&Vector3::new(x0, y0, 0.0))))
    }

    /// Outline of a standard aperture as a convex polygon centred on the origin (mm).
    ///
    /// # Arguments
    /// * `ap` - Aperture definition.
    /// * `unit_scale` - File unit to mm factor.
    ///
    /// # Returns
    /// The polygon vertices, or `None` for unsupported apertures (macros).
    fn aperture_outline(ap: &gerber_types::Aperture, unit_scale: f64) -> Option<Vec<[f64; 2]>> {
        use gerber_types::Aperture as GAperture;
        use std::f64::consts::PI;
        let circle = |r: f64, n: usize, rot: f64| -> Vec<[f64; 2]> {
            (0..n)
                .map(|i| {
                    let a = rot + 2.0 * PI * i as f64 / n as f64;
                    [r * a.cos(), r * a.sin()]
                })
                .collect()
        };
        match ap {
            GAperture::Circle(c) => Some(circle(c.diameter * unit_scale / 2.0, 32, 0.0)),
            GAperture::Rectangle(r) => {
                let (hw, hh) = (r.x * unit_scale / 2.0, r.y * unit_scale / 2.0);
                Some(vec![[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]])
            }
            GAperture::Obround(o) => {
                // Stadium: two semicircles joined along the longer side.
                let (w, h) = (o.x * unit_scale, o.y * unit_scale);
                let r = w.min(h) / 2.0;
                let half_len = (w.max(h) / 2.0 - r).max(0.0);
                let horizontal = w >= h;
                let segs = 16;
                let mut pts = Vec::with_capacity(2 * (segs + 1));
                for (cx, start) in [(half_len, -PI / 2.0), (-half_len, PI / 2.0)] {
                    for i in 0..=segs {
                        let a = start + PI * i as f64 / segs as f64;
                        let (x, y) = (cx + r * a.cos(), r * a.sin());
                        pts.push(if horizontal { [x, y] } else { [-y, x] });
                    }
                }
                Some(pts)
            }
            GAperture::Polygon(p) => {
                let n = usize::from(p.vertices.clamp(3, 12));
                let rot = p.rotation.unwrap_or(0.0).to_radians();
                Some(circle(p.diameter * unit_scale / 2.0, n, rot))
            }
            _ => None,
        }
    }

    /// Convex hull (counter-clockwise) of a point set, Andrew's monotone chain.
    fn convex_hull(mut pts: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
        pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
        pts.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12);
        if pts.len() < 3 {
            return pts;
        }
        let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
            (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
        };
        let mut hull: Vec<[f64; 2]> = Vec::with_capacity(pts.len() * 2);
        for &p in pts.iter().chain(pts.iter().rev().skip(1)) {
            // Lower hull on the forward pass, upper hull on the reverse pass.
            while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
        hull
    }

    /// Area swept by a convex aperture moved in a straight line from `p1` to `p2`.
    fn stroke_convex(outline: &[[f64; 2]], p1: (f64, f64), p2: (f64, f64)) -> Sketch<()> {
        let pts: Vec<[f64; 2]> = outline
            .iter()
            .flat_map(|v| [[v[0] + p1.0, v[1] + p1.1], [v[0] + p2.0, v[1] + p2.1]])
            .collect();
        Sketch::polygon(&Self::convex_hull(pts), None)
    }

    fn create_thick_segment(p1: (f64, f64), p2: (f64, f64), width: f64) -> Sketch<()> {
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let len = (dx * dx + dy * dy).sqrt();

        if len < 1e-6 {
            // Just a circle at p1
            let s = Sketch::circle(width / 2.0, 16, None);
            return s.transform(&Matrix4::new_translation(&Vector3::new(p1.0, p1.1, 0.0)));
        }

        let angle = dy.atan2(dx);

        // Rectangle for the segment
        // Center of rect is at 0,0. Move to center of segment.
        let cx = (p1.0 + p2.0) / 2.0;
        let cy = (p1.1 + p2.1) / 2.0;

        let half_w = width / 2.0;
        let half_l = len / 2.0;

        let pts = vec![
            [-half_l, -half_w],
            [half_l, -half_w],
            [half_l, half_w],
            [-half_l, half_w],
        ];

        let rect: Sketch<()> = Sketch::polygon(&pts, None);

        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, angle));
        let translation = Matrix4::new_translation(&Vector3::new(cx, cy, 0.0));
        let rect = rect.transform(&(translation * rotation));

        // Circles at ends
        let c1: Sketch<()> = Sketch::circle(width / 2.0, 16, None);
        let c1 = c1.transform(&Matrix4::new_translation(&Vector3::new(p1.0, p1.1, 0.0)));

        let c2: Sketch<()> = Sketch::circle(width / 2.0, 16, None);
        let c2 = c2.transform(&Matrix4::new_translation(&Vector3::new(p2.0, p2.1, 0.0)));

        rect.union(&c1).union(&c2)
    }

    fn create_thick_arc(
        p1: (f64, f64),
        p2: (f64, f64),
        center: (f64, f64),
        width: f64,
        clockwise: bool,
    ) -> Sketch<()> {
        let radius = ((p1.0 - center.0).powi(2) + (p1.1 - center.1).powi(2)).sqrt();
        let start_angle = (p1.1 - center.1).atan2(p1.0 - center.0);
        let mut end_angle = (p2.1 - center.1).atan2(p2.0 - center.0);

        if clockwise {
            if end_angle >= start_angle {
                end_angle -= 2.0 * std::f64::consts::PI;
            }
        } else if end_angle <= start_angle {
            end_angle += 2.0 * std::f64::consts::PI;
        }

        let diff = end_angle - start_angle;
        let segments = (diff.abs() / (5.0f64.to_radians())).ceil() as usize; // 5 degree steps
        let segments = segments.max(2);

        let mut outer_pts = Vec::new();
        let mut inner_pts = Vec::new();

        let r_outer = radius + width / 2.0;
        let r_inner = (radius - width / 2.0).max(0.0);

        for i in 0..=segments {
            let t = i as f64 / segments as f64;
            let angle = start_angle + diff * t;
            let c = angle.cos();
            let s = angle.sin();

            outer_pts.push([center.0 + r_outer * c, center.1 + r_outer * s]);
            inner_pts.push([center.0 + r_inner * c, center.1 + r_inner * s]);
        }

        // Construct polygon: Outer points -> Inner points (reversed)
        let mut poly_pts = outer_pts;
        inner_pts.reverse();
        poly_pts.extend(inner_pts);

        let arc_poly: Sketch<()> = Sketch::polygon(&poly_pts, None);

        // Circles at ends
        let c1: Sketch<()> = Sketch::circle(width / 2.0, 16, None);
        let c1 = c1.transform(&Matrix4::new_translation(&Vector3::new(p1.0, p1.1, 0.0)));

        let c2: Sketch<()> = Sketch::circle(width / 2.0, 16, None);
        let c2 = c2.transform(&Matrix4::new_translation(&Vector3::new(p2.0, p2.1, 0.0)));

        arc_poly.union(&c1).union(&c2)
    }

    fn parse_gerber_to_sketches(gerber_content: &str) -> Result<Vec<Sketch<()>>> {
        // Sanitize content
        let mut sanitized = gerber_content.to_string();
        {
            let re_fs = Regex::new(r"%FS.*?\*%").expect("invalid regex pattern");
            let mut count = 0;
            sanitized = re_fs
                .replace_all(&sanitized, |caps: &regex::Captures| {
                    count += 1;
                    if count > 1 {
                        String::new()
                    } else {
                        caps[0].to_string()
                    }
                })
                .to_string();

            let re_mo = Regex::new(r"%MO.*?\*%").expect("invalid regex pattern");
            let mut count = 0;
            sanitized = re_mo
                .replace_all(&sanitized, |caps: &regex::Captures| {
                    count += 1;
                    if count > 1 {
                        String::new()
                    } else {
                        caps[0].to_string()
                    }
                })
                .to_string();
        }

        let reader = BufReader::new(sanitized.as_bytes());
        let doc = match parse(reader) {
            Ok(d) => d,
            Err((d, e)) => {
                warn!(
                    "Gerber parser returned error, attempting to use partial document. Error: {:?}",
                    e
                );
                d
            }
        };

        let mut sketches: Vec<Sketch<()>> = Vec::new();
        let mut current_x = 0.0;
        let mut current_y = 0.0;
        let mut current_aperture_code = 0;
        let mut interpolation = InterpolationMode::Linear;
        let mut quadrant_mode = QuadrantMode::Single;

        if let Some(fmt) = &doc.format_specification {
            debug!(
                "Found Format Specification: {}.{}",
                fmt.integer, fmt.decimal
            );
        } else {
            warn!("No Format Specification found in Gerber file");
        }

        let unit_scale = if let Some(units) = &doc.units {
            match units {
                Unit::Millimeters => {
                    debug!("Units: Millimeters");
                    1.0
                }
                Unit::Inches => {
                    debug!("Units: Inches");
                    25.4
                }
            }
        } else {
            warn!("No Units found, assuming Millimeters");
            1.0
        };

        // `CoordinateNumber` is stored by gerber-types with a fixed 6-decimal fixed-point
        // representation, independent of the file's %FS format (the parser already rescales
        // raw integers by 10^(6 - decimals)). Converting via `f64::from` therefore yields the
        // value in file units; only the unit scale (inch -> mm) must still be applied.
        let convert_coord =
            |c: &CoordinateNumber, _is_x: bool| -> f64 { f64::from(*c) * unit_scale };

        for command in doc.commands() {
            match command {
                Command::FunctionCode(FunctionCode::DCode(dcode)) => {
                    match dcode {
                        DCode::Operation(op) => {
                            match op {
                                Operation::Interpolate(coord, offset) => {
                                    // D01 - Draw
                                    let x = coord
                                        .as_ref()
                                        .and_then(|c| c.x.as_ref())
                                        .map(|v| convert_coord(v, true))
                                        .unwrap_or(current_x);
                                    let y = coord
                                        .as_ref()
                                        .and_then(|c| c.y.as_ref())
                                        .map(|v| convert_coord(v, false))
                                        .unwrap_or(current_y);

                                    use gerber_types::Aperture as GAperture;
                                    let aperture = doc.apertures.get(&current_aperture_code);
                                    // Round-stroke width: exact for circles, the smallest
                                    // dimension otherwise (only used for arcs and macros).
                                    let width = match aperture {
                                        Some(GAperture::Circle(c)) => c.diameter * unit_scale,
                                        Some(GAperture::Rectangle(r))
                                        | Some(GAperture::Obround(r)) => {
                                            (r.x * unit_scale).min(r.y * unit_scale)
                                        }
                                        Some(GAperture::Polygon(p)) => {
                                            // Inscribed circle of the regular polygon.
                                            let n = f64::from(p.vertices.clamp(3, 12));
                                            p.diameter
                                                * unit_scale
                                                * (std::f64::consts::PI / n).cos()
                                        }
                                        _ => 0.1,
                                    };
                                    let convex_outline = match aperture {
                                        Some(ap @ GAperture::Rectangle(_))
                                        | Some(ap @ GAperture::Obround(_))
                                        | Some(ap @ GAperture::Polygon(_)) => {
                                            Self::aperture_outline(ap, unit_scale)
                                        }
                                        _ => None,
                                    };

                                    match interpolation {
                                        InterpolationMode::Linear => {
                                            let stroke = match &convex_outline {
                                                Some(outline) => Self::stroke_convex(
                                                    outline,
                                                    (current_x, current_y),
                                                    (x, y),
                                                ),
                                                None => Self::create_thick_segment(
                                                    (current_x, current_y),
                                                    (x, y),
                                                    width,
                                                ),
                                            };
                                            sketches.push(stroke);
                                        }
                                        InterpolationMode::ClockwiseCircular
                                        | InterpolationMode::CounterclockwiseCircular => {
                                            let i_val = offset
                                                .as_ref()
                                                .and_then(|c| c.x.as_ref())
                                                .map(|v| convert_coord(v, true))
                                                .unwrap_or(0.0);
                                            let j_val = offset
                                                .as_ref()
                                                .and_then(|c| c.y.as_ref())
                                                .map(|v| convert_coord(v, false))
                                                .unwrap_or(0.0);

                                            let clockwise = matches!(
                                                interpolation,
                                                InterpolationMode::ClockwiseCircular
                                            );
                                            let (cx, cy) =
                                                if matches!(quadrant_mode, QuadrantMode::Single) {
                                                    // G74: I/J are unsigned and the arc spans
                                                    // at most 90 degrees; a zero-length arc
                                                    // is a point, not a full circle.
                                                    if (x - current_x).hypot(y - current_y) < 1e-9 {
                                                        current_x = x;
                                                        current_y = y;
                                                        continue;
                                                    }
                                                    single_quadrant_center(
                                                        (current_x, current_y),
                                                        (x, y),
                                                        (i_val, j_val),
                                                        clockwise,
                                                    )
                                                } else {
                                                    (current_x + i_val, current_y + j_val)
                                                };
                                            if convex_outline.is_some() {
                                                warn!(
                                                    aperture = current_aperture_code,
                                                    "Arc drawn with a non-circular aperture; \
                                                     approximated by a round stroke"
                                                );
                                            }
                                            sketches.push(Self::create_thick_arc(
                                                (current_x, current_y),
                                                (x, y),
                                                (cx, cy),
                                                width,
                                                clockwise,
                                            ));
                                        }
                                    }

                                    current_x = x;
                                    current_y = y;
                                }
                                Operation::Move(coord) => {
                                    // D02 - Move
                                    current_x = coord
                                        .as_ref()
                                        .and_then(|c| c.x.as_ref())
                                        .map(|v| convert_coord(v, true))
                                        .unwrap_or(current_x);
                                    current_y = coord
                                        .as_ref()
                                        .and_then(|c| c.y.as_ref())
                                        .map(|v| convert_coord(v, false))
                                        .unwrap_or(current_y);
                                }
                                Operation::Flash(coord) => {
                                    // D03 - Flash
                                    let x = coord
                                        .as_ref()
                                        .and_then(|c| c.x.as_ref())
                                        .map(|v| convert_coord(v, true))
                                        .unwrap_or(current_x);
                                    let y = coord
                                        .as_ref()
                                        .and_then(|c| c.y.as_ref())
                                        .map(|v| convert_coord(v, false))
                                        .unwrap_or(current_y);

                                    if let Some(ap) = doc.apertures.get(&current_aperture_code) {
                                        let s: Sketch<()> =
                                            match Self::aperture_outline(ap, unit_scale) {
                                                Some(outline) => Sketch::polygon(&outline, None),
                                                None => Sketch::circle(0.05, 8, None),
                                            };
                                        let s = s.transform(&Matrix4::new_translation(
                                            &Vector3::new(x, y, 0.0),
                                        ));
                                        sketches.push(s);
                                    }
                                    current_x = x;
                                    current_y = y;
                                }
                            }
                        }
                        DCode::SelectAperture(code) => {
                            current_aperture_code = *code;
                        }
                    }
                }
                Command::FunctionCode(FunctionCode::GCode(gcode)) => {
                    use gerber_types::GCode as GGCode;
                    match gcode {
                        GGCode::InterpolationMode(mode) => {
                            interpolation = *mode;
                        }
                        GGCode::QuadrantMode(mode) => {
                            quadrant_mode = *mode;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        Ok(sketches)
    }

    fn sketch_to_polylines(sketch: &Sketch<()>) -> Vec<(Polyline<f64>, bool)> {
        let mut polylines = Vec::new();
        let mp = sketch.to_multipolygon();
        for poly in mp.0 {
            let mut pline = Polyline::new();
            let mut last_p: Option<(f64, f64)> = None;

            for p in poly.exterior().0.iter() {
                if let Some(last) = last_p {
                    let dist = ((p.x - last.0).powi(2) + (p.y - last.1).powi(2)).sqrt();
                    if dist < 1e-5 {
                        continue;
                    }
                }
                pline.add_vertex(PlineVertex::new(p.x, p.y, 0.0));
                last_p = Some((p.x, p.y));
            }

            // Check closure (first vs last)
            if pline.vertex_count() > 1 {
                let first = pline.at(0).pos();
                let last = pline.at(pline.vertex_count() - 1).pos();
                let dist = ((first.x - last.x).powi(2) + (first.y - last.y).powi(2)).sqrt();
                if dist < 1e-5 {
                    pline.remove_last();
                }
            }

            if pline.vertex_count() > 1 {
                pline.set_is_closed(true);
                polylines.push((pline, false)); // false = not a hole (exterior)
            }

            for interior in poly.interiors() {
                let mut hole_pline = Polyline::new();
                let mut last_p: Option<(f64, f64)> = None;
                for p in interior.0.iter() {
                    if let Some(last) = last_p {
                        let dist = ((p.x - last.0).powi(2) + (p.y - last.1).powi(2)).sqrt();
                        if dist < 1e-5 {
                            continue;
                        }
                    }
                    hole_pline.add_vertex(PlineVertex::new(p.x, p.y, 0.0));
                    last_p = Some((p.x, p.y));
                }

                if hole_pline.vertex_count() > 1 {
                    let first = hole_pline.at(0).pos();
                    let last = hole_pline.at(hole_pline.vertex_count() - 1).pos();
                    let dist = ((first.x - last.x).powi(2) + (first.y - last.y).powi(2)).sqrt();
                    if dist < 1e-5 {
                        hole_pline.remove_last();
                    }
                }

                if hole_pline.vertex_count() > 1 {
                    hole_pline.set_is_closed(true);
                    polylines.push((hole_pline, true)); // true = hole
                }
            }
        }
        polylines
    }

    fn generate_simple_trace_from_sketches(
        params: &GerberParameters,
        sketches: Vec<Sketch<()>>,
        gcode: &mut String,
    ) -> Result<()> {
        if sketches.is_empty() {
            return Ok(());
        }

        // Union all sketches
        let mut merged = sketches[0].clone();
        for s in sketches.iter().skip(1) {
            merged = merged.union(s);
        }

        let polylines = Self::sketch_to_polylines(&merged);
        let paths = polylines.into_iter().map(|(p, _)| p).collect();
        Self::polylines_to_gcode(params, paths, gcode)
    }

    fn generate_isolation_from_sketches(
        params: &GerberParameters,
        sketches: Vec<Sketch<()>>,
        gcode: &mut String,
    ) -> Result<()> {
        if sketches.is_empty() {
            return Ok(());
        }

        // Union all sketches
        let mut merged = sketches[0].clone();
        for s in sketches.iter().skip(1) {
            merged = merged.union(s);
        }

        // Offset
        let polylines = Self::sketch_to_polylines(&merged);
        let isolation_offset = params.tool_diameter as f64 / 2.0 + params.isolation_width as f64;

        let mut isolation_paths = Vec::new();
        for (poly, is_hole) in polylines {
            let poly = clean_polyline(poly);
            let offset_val = Self::outward_offset(&poly, is_hole, isolation_offset);
            let offset_res =
                panic::catch_unwind(panic::AssertUnwindSafe(|| poly.parallel_offset(offset_val)));

            match offset_res {
                Ok(offsets) => {
                    isolation_paths.extend(offsets);
                }
                Err(_) => {
                    warn!("Panic during parallel offset of merged polygon");
                }
            }
        }

        Self::polylines_to_gcode(params, isolation_paths, gcode)
    }

    fn polyline_to_sketch(pline: &Polyline<f64>) -> Sketch<()> {
        let mut points = Vec::new();
        let count = pline.vertex_count();
        if count < 2 {
            return Sketch::new();
        }

        for i in 0..count {
            let v1 = pline.at(i);
            let v2 = pline.at((i + 1) % count);

            points.push([v1.x, v1.y]);

            if v1.bulge.abs() > 1e-5 {
                let theta = 4.0 * v1.bulge.atan();
                let chord_len = ((v2.x - v1.x).powi(2) + (v2.y - v1.y).powi(2)).sqrt();
                if chord_len > 1e-5 {
                    let radius = chord_len / (2.0 * (theta / 2.0).sin());
                    let dist_to_center = radius.abs() * (theta.abs() / 2.0).cos();
                    let dx = v2.x - v1.x;
                    let dy = v2.y - v1.y;
                    let mx = (v1.x + v2.x) / 2.0;
                    let my = (v1.y + v2.y) / 2.0;
                    let nx = -dy / chord_len;
                    let ny = dx / chord_len;
                    let sign = if v1.bulge > 0.0 { 1.0 } else { -1.0 };
                    let cx = mx + nx * dist_to_center * sign;
                    let cy = my + ny * dist_to_center * sign;
                    let start_angle = (v1.y - cy).atan2(v1.x - cx);
                    let mut end_angle = (v2.y - cy).atan2(v2.x - cx);
                    if v1.bulge > 0.0 {
                        if end_angle <= start_angle {
                            end_angle += 2.0 * std::f64::consts::PI;
                        }
                    } else if end_angle >= start_angle {
                        end_angle -= 2.0 * std::f64::consts::PI;
                    }
                    let segments = 8;
                    for j in 1..segments {
                        let t = j as f64 / segments as f64;
                        let angle = start_angle + (end_angle - start_angle) * t;
                        let ax = cx + radius.abs() * angle.cos();
                        let ay = cy + radius.abs() * angle.sin();
                        points.push([ax, ay]);
                    }
                }
            }
        }

        Sketch::polygon(&points, None)
    }

    fn generate_rubout_from_sketches(
        params: &GerberParameters,
        sketches: Vec<Sketch<()>>,
        gcode: &mut String,
    ) -> Result<()> {
        if sketches.is_empty() {
            return Ok(());
        }

        // 1. Union traces
        let mut merged = sketches[0].clone();
        for s in sketches.iter().skip(1) {
            merged = merged.union(s);
        }

        // 2. Offset traces (Isolation)
        let polylines = Self::sketch_to_polylines(&merged);
        let isolation_offset = params.tool_diameter as f64 / 2.0 + params.isolation_width as f64;

        let mut positive_sketch = Sketch::new();
        let mut negative_sketch = Sketch::new();

        for (poly, is_hole) in polylines {
            let poly = clean_polyline(poly);
            let offset_val = Self::outward_offset(&poly, is_hole, isolation_offset);
            let offset_res =
                panic::catch_unwind(panic::AssertUnwindSafe(|| poly.parallel_offset(offset_val)));

            if let Ok(offsets) = offset_res {
                for off in offsets {
                    let s = Self::polyline_to_sketch(&off);
                    if is_hole {
                        negative_sketch = negative_sketch.union(&s);
                    } else {
                        positive_sketch = positive_sketch.union(&s);
                    }
                }
            }
        }

        let isolation_sketch = positive_sketch.difference(&negative_sketch);

        // Use the isolation sketch directly as the area to protect
        // The tool diameter in the hatching will provide the clearance
        let buffered_traces = isolation_sketch.clone();

        // 3. Board Area: the outline file if one is found, otherwise the copper bounds
        // (Gerber coordinates are usually not anchored at 0,0, so a board-sized rectangle at
        // the origin could miss the copper entirely).
        let mut board: Option<Sketch<()>> = None;

        if params.use_board_outline
            && let Some(dir) = &params.directory
        {
            let path = PathBuf::from(dir);
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default()
                        .to_lowercase();
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();

                    if (ext == "gko"
                        || ext == "gm1"
                        || name.contains("edge.cuts")
                        || name.contains("outline"))
                        && let Ok(content) = fs::read_to_string(&path)
                        && let Ok(sketches) = Self::parse_gerber_to_sketches(&content)
                        && !sketches.is_empty()
                    {
                        let mut merged_outline = sketches[0].clone();
                        for s in sketches.iter().skip(1) {
                            merged_outline = merged_outline.union(s);
                        }
                        board = Some(merged_outline);
                        debug!("Using board outline from file: {:?}", path);
                        break;
                    }
                }
            }
        }

        let board = match board {
            Some(b) => b,
            None => {
                if params.use_board_outline {
                    warn!("No board outline file found; rubout uses the copper bounds");
                }
                let margin = isolation_offset + params.tool_diameter as f64;
                match Self::bounds_rectangle(&merged, margin) {
                    Some(rect) => rect,
                    None => return Ok(()),
                }
            }
        };

        // 4. Rubout Area
        debug!("Board bounds: {:?}", board.bounding_box());
        debug!(
            "Isolation sketch bounds: {:?}",
            isolation_sketch.bounding_box()
        );
        debug!(
            "Buffered traces bounds: {:?}",
            buffered_traces.bounding_box()
        );
        let rubout_area = board.difference(&buffered_traces);
        debug!("Rubout area bounds: {:?}", rubout_area.bounding_box());
        debug!(
            "Rubout area multipolygon polygon count: {}",
            rubout_area.to_multipolygon().0.len()
        );

        // 5. Hatch - Convert each polygon with holes to a single lyon path
        // Lyon's even-odd fill rule will properly handle holes if they wind opposite direction
        let mp = rubout_area.to_multipolygon();
        debug!("Rubout area multipolygon count: {}", mp.0.len());
        let mut lyon_paths = Vec::new();
        for poly in mp.0 {
            let mut builder = LyonPath::builder();
            let ext = poly.exterior();
            if ext.0.is_empty() {
                continue;
            }

            // Exterior ring - clockwise winding
            builder.begin(point(ext.0[0].x as f32, ext.0[0].y as f32));
            for p in ext.0.iter().skip(1) {
                builder.line_to(point(p.x as f32, p.y as f32));
            }
            builder.close();

            // Interior rings (holes) - counter-clockwise winding
            for int in poly.interiors() {
                if int.0.is_empty() {
                    continue;
                }
                builder.begin(point(int.0[0].x as f32, int.0[0].y as f32));
                for p in int.0.iter().skip(1) {
                    builder.line_to(point(p.x as f32, p.y as f32));
                }
                builder.close();
            }
            lyon_paths.push(builder.build());
        }

        let spacing = params.tool_diameter * 0.8;
        debug!("Generating hatch with spacing: {}", spacing);
        let mut hatch_lines = Vec::new();
        for path in lyon_paths {
            let lines = hatch_generator::generate_hatch(&path, 45.0, spacing, 0.01);
            hatch_lines.extend(lines);
        }
        debug!("Generated {} hatch lines", hatch_lines.len());

        // 6. G-Code
        let has_z = params.num_axes >= 3;
        for (i, path) in hatch_lines.iter().enumerate() {
            if i < 5 {
                debug!("Hatch line {}: {:?}", i, path.iter().collect::<Vec<_>>());
            }
            for event in path.iter() {
                match event {
                    lyon::path::Event::Begin { at } => {
                        let sx = at.x + params.offset_x;
                        let sy = at.y + params.offset_y;
                        if has_z {
                            writeln!(gcode, "G0 Z{:.3}", params.safe_z)?;
                        }
                        writeln!(gcode, "G0 X{:.3} Y{:.3}", sx, sy)?;
                        if has_z {
                            writeln!(
                                gcode,
                                "G1 Z{:.3} F{:.1}",
                                params.cut_depth, params.feed_rate
                            )?;
                        }
                    }
                    lyon::path::Event::Line { to, .. } => {
                        let tx = to.x + params.offset_x;
                        let ty = to.y + params.offset_y;
                        writeln!(gcode, "G1 X{:.3} Y{:.3} F{:.1}", tx, ty, params.feed_rate)?;
                    }
                    _ => {}
                }
            }
            if has_z {
                writeln!(gcode, "G0 Z{:.3}", params.safe_z)?;
            }
        }

        Ok(())
    }

    fn polylines_to_gcode(
        params: &GerberParameters,
        paths: Vec<Polyline<f64>>,
        gcode: &mut String,
    ) -> Result<()> {
        let has_z = params.num_axes >= 3;
        for path in paths {
            if path.vertex_count() < 2 {
                continue;
            }

            let start = path.at(0).pos();
            let sx = start.x as f32 + params.offset_x;
            let sy = start.y as f32 + params.offset_y;

            if has_z {
                writeln!(gcode, "G0 Z{:.3}", params.safe_z)?;
            }
            writeln!(gcode, "G0 X{:.3} Y{:.3}", sx, sy)?;
            if has_z {
                writeln!(
                    gcode,
                    "G1 Z{:.3} F{:.1}",
                    params.cut_depth, params.feed_rate
                )?;
            }

            let emit_segment =
                |gcode: &mut String, p1: PlineVertex<f64>, p2: PlineVertex<f64>| -> Result<()> {
                    let tx = p2.x as f32 + params.offset_x;
                    let ty = p2.y as f32 + params.offset_y;

                    if p1.bulge.abs() > 1e-5 {
                        // Linearize arc
                        let theta = 4.0 * p1.bulge.atan();
                        let chord_len = ((p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2)).sqrt();

                        if chord_len < 1e-5 {
                            writeln!(gcode, "G1 X{:.3} Y{:.3}", tx, ty)?;
                            return Ok(());
                        }

                        let radius = chord_len / (2.0 * (theta / 2.0).sin());
                        let dist_to_center = radius.abs() * (theta.abs() / 2.0).cos();

                        let dx = p2.x - p1.x;
                        let dy = p2.y - p1.y;
                        let mx = (p1.x + p2.x) / 2.0;
                        let my = (p1.y + p2.y) / 2.0;

                        // Left normal (-dy, dx)
                        let nx = -dy / chord_len;
                        let ny = dx / chord_len;

                        let sign = if p1.bulge > 0.0 { 1.0 } else { -1.0 };
                        let cx = mx + nx * dist_to_center * sign;
                        let cy = my + ny * dist_to_center * sign;

                        let start_angle = (p1.y - cy).atan2(p1.x - cx);
                        let mut end_angle = (p2.y - cy).atan2(p2.x - cx);

                        // Handle wrapping
                        if p1.bulge > 0.0 {
                            // CCW
                            if end_angle <= start_angle {
                                end_angle += 2.0 * std::f64::consts::PI;
                            }
                        } else {
                            // CW
                            if end_angle >= start_angle {
                                end_angle -= 2.0 * std::f64::consts::PI;
                            }
                        }

                        let segments = 16; // Fixed segments for now
                        for j in 1..=segments {
                            let t = j as f64 / segments as f64;
                            let angle = start_angle + (end_angle - start_angle) * t;
                            let ax = cx + radius.abs() * angle.cos();
                            let ay = cy + radius.abs() * angle.sin();

                            let tax = ax as f32 + params.offset_x;
                            let tay = ay as f32 + params.offset_y;
                            writeln!(gcode, "G1 X{:.3} Y{:.3}", tax, tay)?;
                        }
                    } else {
                        writeln!(gcode, "G1 X{:.3} Y{:.3}", tx, ty)?;
                    }
                    Ok(())
                };

            for i in 1..path.vertex_count() {
                let p1 = path.at(i - 1);
                let p2 = path.at(i);
                emit_segment(gcode, p1, p2)?;
            }

            // Close loop if needed
            if path.is_closed() {
                let p1 = path.at(path.vertex_count() - 1);
                let p2 = path.at(0);
                emit_segment(gcode, p1, p2)?;
            }
            if has_z {
                writeln!(gcode, "G0 Z{:.3}", params.safe_z)?;
            }
        }
        Ok(())
    }

    fn append_alignment_holes(params: &GerberParameters, gcode: &mut String) -> Result<()> {
        if params.generate_alignment_holes {
            let has_z = params.num_axes >= 3;
            writeln!(gcode, "; Alignment Holes")?;
            let margin = params.alignment_hole_margin;
            let width = params.board_width;
            let height = params.board_height;

            let holes = [
                (-margin, -margin),
                (width + margin, -margin),
                (width + margin, height + margin),
                (-margin, height + margin),
            ];

            if has_z
                && !(params.alignment_hole_depth.is_finite() && params.alignment_hole_depth < 0.0)
            {
                anyhow::bail!("Alignment hole depth must be below the surface (negative Z)");
            }

            for (hx, hy) in holes.iter() {
                let tx = hx + params.offset_x;
                let ty = hy + params.offset_y;

                if has_z {
                    writeln!(gcode, "G0 Z{:.3}", params.safe_z)?;
                }
                writeln!(gcode, "G0 X{:.3} Y{:.3}", tx, ty)?;
                if has_z {
                    writeln!(
                        gcode,
                        "G1 Z{:.3} F{:.1} ; Drill alignment hole",
                        params.alignment_hole_depth, params.feed_rate
                    )?;
                    writeln!(gcode, "G0 Z{:.3}", params.safe_z)?;
                }
            }
        }
        Ok(())
    }
}

/// The centre of a single-quadrant (G74) arc.
///
/// In G74 mode `I`/`J` are unsigned distances and the arc spans at most 90
/// degrees, so of the four sign combinations the centre is the one that is
/// equidistant from both endpoints with a sweep of 90 degrees or less in the
/// arc's direction.
///
/// # Arguments
/// * `start` / `end` - Arc endpoints.
/// * `offset` - The `I`, `J` values (their signs are ignored).
/// * `clockwise` - Arc direction.
pub fn single_quadrant_center(
    start: (f64, f64),
    end: (f64, f64),
    offset: (f64, f64),
    clockwise: bool,
) -> (f64, f64) {
    let (i, j) = (offset.0.abs(), offset.1.abs());
    let sweep = |c: (f64, f64)| {
        let a0 = (start.1 - c.1).atan2(start.0 - c.0);
        let a1 = (end.1 - c.1).atan2(end.0 - c.0);
        let ccw = (a1 - a0).rem_euclid(std::f64::consts::TAU);
        if clockwise {
            (std::f64::consts::TAU - ccw) % std::f64::consts::TAU
        } else {
            ccw
        }
    };
    let mismatch = |c: (f64, f64)| {
        ((start.0 - c.0).hypot(start.1 - c.1) - (end.0 - c.0).hypot(end.1 - c.1)).abs()
    };
    let candidates =
        [(i, j), (-i, j), (i, -j), (-i, -j)].map(|(di, dj)| (start.0 + di, start.1 + dj));
    let quarter = std::f64::consts::FRAC_PI_2 + 1e-6;
    candidates
        .iter()
        .copied()
        .filter(|c| sweep(*c) <= quarter)
        .min_by(|a, b| mismatch(*a).total_cmp(&mismatch(*b)))
        .or_else(|| {
            candidates
                .iter()
                .copied()
                .min_by(|a, b| mismatch(*a).total_cmp(&mismatch(*b)))
        })
        .unwrap_or((start.0 + i, start.1 + j))
}
