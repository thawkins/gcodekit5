//! G-Code Visualizer
//! Parses G-Code toolpaths for canvas-based visualization

use super::toolpath_cache::ToolpathCache;
use super::viewport::Bounds;
use crate::gcode::{arc, words};
use gcodekit5_core::constants as core_constants;
use gcodekit5_designer::toolpath::Toolpath;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::mpsc;
use tracing::{debug, trace};

const _CANVAS_PADDING_2X: f32 = 40.0;
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 50.0;
const ZOOM_STEP: f32 = 1.1;
const PAN_PERCENTAGE: f32 = 0.1;
const BOUNDS_PADDING_FACTOR: f32 = core_constants::VIEW_PADDING as f32;
const _FIT_MARGIN_FACTOR: f32 = 0.05;
const _ORIGIN_CROSS_SIZE: i32 = 5;
const _MARKER_RADIUS: i32 = 4;
const _MAX_GRID_ITERATIONS: usize = 500;
const _MAX_SCALE: f32 = 100.0;
const _MIN_SCALE: f32 = 0.1;
const DEFAULT_SCALE_FACTOR: f32 = 1.0;
const _GRID_MAJOR_STEP_MM: f32 = 10.0;
const _GRID_MINOR_STEP_MM: f32 = 1.0;
const _GRID_MAJOR_VISIBILITY_SCALE: f32 = 0.3;
const _GRID_MINOR_VISIBILITY_SCALE: f32 = 1.5;

/// Decide whether `new` replaces `old` (a newly loaded program) rather than
/// being an edit of it.
///
/// Used by the UI to keep the user's view while editing and to re-fit it only
/// when a different program is loaded. The text shared by both versions is
/// measured as the common prefix plus the common suffix; an edit keeps most of
/// the old text, a new file keeps little of it.
///
/// # Arguments
/// * `old` - Text currently shown.
/// * `new` - Incoming text.
///
/// # Returns
/// `true` when `old` is blank and `new` is not, or when less than half of the
/// shorter text is shared; `false` otherwise (including when `new` is blank).
pub fn is_content_replacement(old: &str, new: &str) -> bool {
    if new.trim().is_empty() {
        return false;
    }
    if old.trim().is_empty() {
        return true;
    }
    let (a, b) = (old.as_bytes(), new.as_bytes());
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let max_suffix = a.len().min(b.len()) - prefix;
    let suffix = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(max_suffix)
        .take_while(|(x, y)| x == y)
        .count();
    let shared = prefix + suffix;
    shared * 2 < a.len().min(b.len())
}

/// Colors according to power.
///
/// Uses detected min/max S range and quantizes the gray ramp to a fixed
/// number of levels to keep rendering stable for dense raster engravings.
fn intensity_to_color(intensity: f32, max_intensity: f32) -> (f32, f32, f32) {
    if max_intensity <= 0.0 {
        return (0.5, 0.5, 0.5);
    }
    // Normalize intensity but inverted: 0 = white, 1 = black
    let engraving_color = 1.0 - (intensity / max_intensity).clamp(0.0, 0.5);
    (
        engraving_color / 2.0,
        engraving_color,
        engraving_color / 2.0,
    )
}

/// Modal motion mode (group 1).
///
/// Parsing starts in `Rapid` because GRBL powers up in G0, so axis-only lines
/// before the first explicit G0/G1 are still drawn. `None` is only entered via
/// G80 (motion mode cancel).
#[derive(Debug, Clone, Copy, PartialEq)]
enum MotionMode {
    None,
    Rapid,  // G0
    Linear, // G1
    ArcCW,  // G2
    ArcCCW, // G3
}

/// 6-Axis position including rotary axes
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RotaryPosition {
    pub a: f32, // 4th axis (rotation around X)
    pub b: f32, // 5th axis (rotation around Y)
    pub c: f32, // 6th axis (rotation around Z)
}

impl RotaryPosition {
    pub fn new(a: f32, b: f32, c: f32) -> Self {
        Self { a, b, c }
    }

    pub fn from_optional(a: Option<f32>, b: Option<f32>, c: Option<f32>) -> Self {
        Self {
            a: a.unwrap_or(0.0),
            b: b.unwrap_or(0.0),
            c: c.unwrap_or(0.0),
        }
    }

    /// Check if any rotary axis has non-zero value
    pub fn has_movement(&self) -> bool {
        self.a != 0.0 || self.b != 0.0 || self.c != 0.0
    }

    /// Get the magnitude of rotation (for visualization)
    pub fn magnitude(&self) -> f32 {
        (self.a.powi(2) + self.b.powi(2) + self.c.powi(2)).sqrt()
    }
}

/// 3D Point for visualization
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3D {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// Active arc plane (G17/G18/G19)
#[derive(Debug, Clone, Copy, PartialEq)]
enum ArcPlane {
    XY, // G17
    ZX, // G18
    YZ, // G19
}

impl ArcPlane {
    /// Axis indices (first plane axis, second plane axis, linear axis) using
    /// 0 = X, 1 = Y, 2 = Z. Ordering follows GRBL so CW/CCW sense is correct.
    fn axes(self) -> (usize, usize, usize) {
        match self {
            ArcPlane::XY => (0, 1, 2),
            ArcPlane::ZX => (2, 0, 1),
            ArcPlane::YZ => (1, 2, 0),
        }
    }
}

/// Spindle / laser state (M3/M4 on, M5 off).
///
/// `Unknown` is the state before any M3/M4/M5 has been seen; moves are then
/// treated as powered so that programs that never switch the spindle (common in
/// hand-written or test files) keep their existing appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SpindleState {
    Unknown,
    On,
    Off,
}

/// Structure for extracted parameters
/// Supports 6-axis coordinates (X, Y, Z linear + A, B, C rotary)
#[derive(Default)]
struct ExtractedParams {
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
    // Rotary axis coordinates (4th, 5th, 6th axes)
    a: Option<f32>,
    b: Option<f32>,
    c: Option<f32>,
    // Arc center offsets (I, J, K) and radius (R)
    i: Option<f32>,
    j: Option<f32>,
    k: Option<f32>,
    r: Option<f32>,
    // Spindle / laser power and dwell time
    s: Option<f32>,
    p: Option<f32>,
}

impl ExtractedParams {
    /// Check if this line has any linear or rotary axis movement
    fn has_movement(&self) -> bool {
        self.x.is_some()
            || self.y.is_some()
            || self.z.is_some()
            || self.a.is_some()
            || self.b.is_some()
            || self.c.is_some()
    }

    /// Arc centre offsets indexed by axis (0 = I/X, 1 = J/Y, 2 = K/Z)
    fn ijk(&self) -> [Option<f32>; 3] {
        [self.i, self.j, self.k]
    }
}

fn point_axis(p: Point3D, axis: usize) -> f64 {
    match axis {
        0 => p.x as f64,
        1 => p.y as f64,
        _ => p.z as f64,
    }
}

fn point_from_axes(values: [f64; 3]) -> Point3D {
    Point3D::new(values[0] as f32, values[1] as f32, values[2] as f32)
}

/// Emit visualizer commands for a G2/G3 arc.
///
/// XY-plane arcs are emitted as `GCodeCommand::Arc`, split so that no single
/// command sweeps more than 180 degrees (full circles would otherwise have
/// identical start and end points, which renderers draw as nothing). Arcs in
/// the G18/G19 planes are tessellated into feed moves because
/// `GCodeCommand::Arc` is interpreted in the XY plane by all renderers.
///
/// Returns `false` when the arc is undefined (no centre information or a zero
/// radius); the caller then falls back to a straight move.
#[allow(clippy::too_many_arguments)]
fn push_arc_commands(
    commands: &mut Vec<GCodeCommand>,
    bounds: &mut Bounds,
    from: Point3D,
    to: Point3D,
    plane: ArcPlane,
    clockwise: bool,
    ijk: [Option<f32>; 3],
    radius_word: Option<f32>,
    intensity: Option<f32>,
) -> bool {
    let (a0, a1, lin) = plane.axes();
    let (sx, sy) = (point_axis(from, a0), point_axis(from, a1));
    let (ex, ey) = (point_axis(to, a0), point_axis(to, a1));

    let offsets = if ijk[a0].is_some() || ijk[a1].is_some() {
        Some((ijk[a0].unwrap_or(0.0) as f64, ijk[a1].unwrap_or(0.0) as f64))
    } else if let Some(r) = radius_word {
        arc::center_offset_from_radius(ex - sx, ey - sy, r as f64, clockwise)
    } else {
        None
    };
    let Some((oi, oj)) = offsets else {
        return false;
    };

    let radius = (oi * oi + oj * oj).sqrt();
    if !radius.is_finite() || radius < 1e-6 {
        return false;
    }
    let (cx, cy) = (sx + oi, sy + oj);
    let start_angle = (-oj).atan2(-oi);
    let sweep = arc::angular_travel((-oi, -oj), (ex - cx, ey - cy), clockwise);

    let (from_lin, to_lin) = (point_axis(from, lin), point_axis(to, lin));
    let point_at = |t: f64| -> Point3D {
        let angle = start_angle + sweep * t;
        let mut v = [0.0f64; 3];
        v[a0] = cx + radius * angle.cos();
        v[a1] = cy + radius * angle.sin();
        v[lin] = from_lin + (to_lin - from_lin) * t;
        point_from_axes(v)
    };

    if plane == ArcPlane::XY {
        let pieces = if sweep.abs() > std::f64::consts::PI + 1e-9 {
            2
        } else {
            1
        };
        let mut prev = from;
        for piece in 1..=pieces {
            let next = if piece == pieces {
                to
            } else {
                point_at(piece as f64 / pieces as f64)
            };
            commands.push(GCodeCommand::Arc {
                from: prev,
                to: next,
                center: Point3D::new(cx as f32, cy as f32, prev.z),
                clockwise,
                intensity,
            });
            prev = next;
        }

        let (min_x, min_y, max_x, max_y) = arc::arc_bounds((cx, cy), radius, start_angle, sweep);
        bounds.update(min_x as f32, min_y as f32, from.z);
        bounds.update(max_x as f32, max_y as f32, to.z);
    } else {
        // 10 degree steps, at least 2 segments
        let steps = ((sweep.abs() / (std::f64::consts::PI / 18.0)).ceil() as usize).max(2);
        let mut prev = from;
        bounds.update(from.x, from.y, from.z);
        for step in 1..=steps {
            let next = if step == steps {
                to
            } else {
                point_at(step as f64 / steps as f64)
            };
            commands.push(GCodeCommand::Move {
                from: prev,
                to: next,
                rapid: false,
                intensity,
            });
            bounds.update(next.x, next.y, next.z);
            prev = next;
        }
    }

    true
}

/// Extend `bounds` with the full extent of a stored XY arc command.
fn update_bounds_with_arc(
    bounds: &mut Bounds,
    from: Point3D,
    to: Point3D,
    center: Point3D,
    clockwise: bool,
) {
    bounds.update(from.x, from.y, from.z);
    bounds.update(to.x, to.y, to.z);
    let center_xy = (center.x as f64, center.y as f64);
    let Some(sweep) = arc::arc_sweep(
        center_xy,
        (from.x as f64, from.y as f64),
        (to.x as f64, to.y as f64),
        clockwise,
    ) else {
        return;
    };
    let (min_x, min_y, max_x, max_y) =
        arc::arc_bounds(center_xy, sweep.radius, sweep.start_angle, sweep.sweep);
    bounds.update(min_x as f32, min_y as f32, from.z);
    bounds.update(max_x as f32, max_y as f32, to.z);
}

/// Movement command
#[derive(Debug, Clone)]
pub enum GCodeCommand {
    Move {
        from: Point3D,
        to: Point3D,
        rapid: bool,
        /// Spindle/laser power (S word) for the move; `None` when the
        /// spindle/laser was switched off (M5), i.e. a non-cutting move.
        intensity: Option<f32>,
    },
    Arc {
        from: Point3D,
        to: Point3D,
        center: Point3D,
        clockwise: bool,
        /// Spindle/laser power; `None` when the spindle/laser is off (M5).
        intensity: Option<f32>,
    },
    Dwell {
        pos: Point3D,
        duration: f32,
    },
}
/*
/// Coordinate transformation helper
#[allow(dead_code)]
struct CoordTransform {
    min_x: f32,
    min_y: f32,
    scale: f32,
    width: f32,
    height: f32,
    x_offset: f32,
    y_offset: f32,
}

// See CoordTransform struct above — methods also unused pending integration.
#[allow(dead_code)]
impl CoordTransform {
    fn new(
        min_x: f32,
        min_y: f32,
        scale: f32,
        width: f32,
        height: f32,
        x_offset: f32,
        y_offset: f32,
    ) -> Self {
        Self {
            min_x,
            min_y,
            scale,
            width,
            height,
            x_offset,
            y_offset,
        }
    }

    fn world_to_screen(&self, x: f32, y: f32) -> (i32, i32) {
        let screen_x = (x - self.min_x) * self.scale + CANVAS_PADDING + self.x_offset;
        // Flip Y axis: higher Y values should move up the screen (smaller screen_y)
        let screen_y =
            self.height - ((y - self.min_y) * self.scale + CANVAS_PADDING - self.y_offset);
        (safe_to_i32(screen_x), safe_to_i32(screen_y))
    }

    fn point_to_screen(&self, point: Point3D) -> (i32, i32) {
        self.world_to_screen(point.x, point.y)
    }
}
*/

/// Visualizer state with 6-axis support
#[derive(Debug, Clone)]
pub struct Visualizer {
    // Linear axis bounds
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
    // Rotary axis bounds (4th, 5th, 6th axes)
    pub min_a: f32,
    pub max_a: f32,
    pub min_b: f32,
    pub max_b: f32,
    pub min_c: f32,
    pub max_c: f32,
    // Current position (including rotary)
    pub current_pos: Point3D,
    pub current_rotary: RotaryPosition,
    pub current_intensity: f32,
    /// Zoom/scale factor for rendering (1.0 = 100%)
    pub zoom_scale: f32,
    /// X-offset for panning the view (in pixels)
    pub x_offset: f32,
    /// Y-offset for panning the view (in pixels)
    pub y_offset: f32,
    /// Grid visibility flag
    pub show_grid: bool,
    /// Scale factor: pixels per mm (default 1.0 = 1px:1mm)
    pub scale_factor: f32,
    /// Whether to use different colors for rotary moves
    pub color_rotary_moves: bool,
    toolpath_cache: ToolpathCache,
    /// Dirty flag — set when vertex data needs regeneration
    dirty: bool,

    toolpath_receiver: Rc<mpsc::Receiver<Vec<Toolpath>>>,
    toolpath_sender: mpsc::Sender<Vec<Toolpath>>,
    current_toolpaths: Vec<Toolpath>,
    /// Colors according to intensity
    pub min_intensity: f32,
    pub max_intensity: f32,
    pub use_intensity_colors: bool,
    pub is_raster: bool,
}

impl Visualizer {
    /// Create new visualizer
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            // Linear axis bounds
            min_x: -(core_constants::WORLD_EXTENT_MM as f32),
            max_x: core_constants::WORLD_EXTENT_MM as f32,
            min_y: -(core_constants::WORLD_EXTENT_MM as f32),
            max_y: core_constants::WORLD_EXTENT_MM as f32,
            min_z: 0.0,
            max_z: 100.0,
            // Rotary axis bounds (default to 0-360 degrees)
            min_a: 0.0,
            max_a: 360.0,
            min_b: 0.0,
            max_b: 360.0,
            min_c: 0.0,
            max_c: 360.0,
            // Position tracking
            current_pos: Point3D::new(0.0, 0.0, 0.0),
            current_rotary: RotaryPosition::default(),
            current_intensity: 0.0,
            zoom_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
            show_grid: true,
            scale_factor: DEFAULT_SCALE_FACTOR,
            color_rotary_moves: true, // Enable rotary color coding by default
            toolpath_cache: ToolpathCache::new(),
            dirty: true,

            toolpath_receiver: Rc::new(receiver),
            toolpath_sender: sender,
            current_toolpaths: Vec::new(),

            min_intensity: 0.0,
            max_intensity: 1000.0,
            use_intensity_colors: true,
            is_raster: false,
        }
    }

    /// Returns true if vertex data needs regeneration
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear the dirty flag after vertex data has been regenerated
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Toggle grid visibility
    pub fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
    }

    /// Set grid visibility
    pub fn set_grid_visible(&mut self, visible: bool) {
        self.show_grid = visible;
    }

    /// Get grid visibility
    pub fn is_grid_visible(&self) -> bool {
        self.show_grid
    }

    /// Get scale factor
    pub fn get_scale_factor(&self) -> f32 {
        self.scale_factor
    }
    /*
        /// Extract multiple parameters from G-Code line
        // Deprecated: Use direct parsing in parse_linear_move/parse_arc_move instead
        #[allow(dead_code)]
        fn extract_params(line: &str, param_names: &[char]) -> HashMap<char, f32> {
            let mut params = HashMap::new();

            for part in line.split_whitespace() {
                if part.len() < 2 {
                    continue;
                }
                let Some(first_char) = part.chars().next() else {
                    continue;
                };
                if param_names.contains(&first_char) {
                    if let Ok(value) = part[1..].parse::<f32>() {
                        params.insert(first_char, value);
                    }
                }
            }

            params
        }
    */
    /// Get number of commands parsed
    pub fn get_command_count(&self) -> usize {
        self.toolpath_cache.len()
    }

    /// Parse G-Code and extract movement commands
    ///
    /// Honours modal motion (G0-G3, G80), distance mode (G90/G91), units
    /// (G20/G21, converted to mm), arc planes (G17/G18/G19), I/J/K and R arcs
    /// (including negative R and full circles), dwell (G4 P), and non-motion
    /// axis words (G10, G28.1, G30.1, G43.1, G92). Comments in parentheses and
    /// after `;` are ignored and several G words may share one line.
    pub fn parse_gcode(&mut self, gcode: &str) {
        debug!("Starting G-code parse, input size: {} bytes", gcode.len());

        let mut hasher = DefaultHasher::new();
        gcode.hash(&mut hasher);
        let new_hash = hasher.finish();

        if !self.toolpath_cache.needs_update(new_hash) {
            debug!("G-code hash unchanged, skipping parse");
            return;
        }

        debug!("Parsing new G-code (hash: {})", new_hash);

        let mut commands = Vec::new();

        self.is_raster = false;

        let mut current_pos = Point3D::new(0.0, 0.0, 0.0);
        let mut rotary = RotaryPosition::default();
        self.current_intensity = 0.0;
        let mut bounds = Bounds::new();

        // Modal state
        let mut current_motion_mode = MotionMode::Rapid;
        let mut spindle = SpindleState::Unknown;
        let mut plane = ArcPlane::XY;
        let mut absolute = true;
        let mut inches = false;
        let mut motion_counts = [0usize; 4];

        for (line_num, line) in gcode.lines().enumerate() {
            let original_line = line.trim();
            // Identify if it is a raster image
            if original_line.contains("; GcodeKit5 Image Engraving") {
                self.is_raster = true;
            }

            let words = words::parse_line(original_line);
            if words.is_empty() {
                continue;
            }

            let mut params = ExtractedParams::default();
            let mut dwell = false;
            let mut set_position = false;
            let mut home_move = false;
            let mut axes_not_motion = false;

            for word in &words {
                let value = word.value as f32;
                match word.letter {
                    'G' => {
                        let (major, minor) = words::code_number(word.value);
                        trace!("Line {}: G{}.{} command", line_num, major, minor);
                        match (major, minor) {
                            (0, 0) => current_motion_mode = MotionMode::Rapid,
                            (1, 0) => current_motion_mode = MotionMode::Linear,
                            (2, 0) => current_motion_mode = MotionMode::ArcCW,
                            (3, 0) => current_motion_mode = MotionMode::ArcCCW,
                            (4, 0) => dwell = true,
                            (10, _) | (28, 1) | (30, 1) | (43, 1) => axes_not_motion = true,
                            (17, 0) => plane = ArcPlane::XY,
                            (18, 0) => plane = ArcPlane::ZX,
                            (19, 0) => plane = ArcPlane::YZ,
                            (20, 0) => inches = true,
                            (21, 0) => inches = false,
                            (28, 0) | (30, 0) => home_move = true,
                            (80, 0) => current_motion_mode = MotionMode::None,
                            (90, 0) => absolute = true,
                            (91, 0) => absolute = false,
                            (92, 0) => set_position = true,
                            _ => {}
                        }
                    }
                    'M' => match words::code_number(word.value) {
                        (3, 0) | (4, 0) => spindle = SpindleState::On,
                        (5, 0) => spindle = SpindleState::Off,
                        _ => {}
                    },
                    'X' => params.x = Some(value),
                    'Y' => params.y = Some(value),
                    'Z' => params.z = Some(value),
                    'A' => params.a = Some(value),
                    'B' => params.b = Some(value),
                    'C' => params.c = Some(value),
                    'I' => params.i = Some(value),
                    'J' => params.j = Some(value),
                    'K' => params.k = Some(value),
                    'R' => params.r = Some(value),
                    'S' => params.s = Some(value),
                    'P' => params.p = Some(value),
                    _ => {}
                }
            }

            // Update intensity if there is an S command
            if let Some(intensity) = params.s {
                self.current_intensity = intensity;
            }
            // Moves made while the spindle/laser is off (after M5) carry no
            // intensity, so renderers draw them as travel, not as cuts.
            let move_intensity = if spindle == SpindleState::Off {
                None
            } else {
                Some(self.current_intensity)
            };

            // Convert linear words to mm (G20 applies to the whole line)
            let scale = if inches { 25.4f32 } else { 1.0 };
            for v in [
                &mut params.x,
                &mut params.y,
                &mut params.z,
                &mut params.i,
                &mut params.j,
                &mut params.k,
                &mut params.r,
            ] {
                *v = v.map(|val| val * scale);
            }

            if dwell {
                // G4 P<seconds> (GRBL/LinuxCNC); X/S accepted for other dialects.
                // Axis words on a dwell line are not a move.
                let duration = params
                    .p
                    .or(params.x.map(|x| x / scale))
                    .or(params.s)
                    .unwrap_or(0.0);
                if duration > 0.0 {
                    commands.push(GCodeCommand::Dwell {
                        pos: current_pos,
                        duration,
                    });
                }
                continue;
            }

            if axes_not_motion || !params.has_movement() {
                continue;
            }

            if set_position {
                // G92: the given axes take the new values without moving.
                current_pos = Point3D::new(
                    params.x.unwrap_or(current_pos.x),
                    params.y.unwrap_or(current_pos.y),
                    params.z.unwrap_or(current_pos.z),
                );
                rotary = RotaryPosition::new(
                    params.a.unwrap_or(rotary.a),
                    params.b.unwrap_or(rotary.b),
                    params.c.unwrap_or(rotary.c),
                );
                continue;
            }

            let resolve = |current: f32, value: Option<f32>| -> f32 {
                match value {
                    Some(v) if absolute => v,
                    Some(v) => current + v,
                    None => current,
                }
            };
            let target = Point3D::new(
                resolve(current_pos.x, params.x),
                resolve(current_pos.y, params.y),
                resolve(current_pos.z, params.z),
            );
            rotary = RotaryPosition::new(
                resolve(rotary.a, params.a),
                resolve(rotary.b, params.b),
                resolve(rotary.c, params.c),
            );
            bounds.update_rotary(
                params.a.map(|_| rotary.a),
                params.b.map(|_| rotary.b),
                params.c.map(|_| rotary.c),
            );

            // G28/G30: the axis words are an intermediate point reached at rapid
            // rate; the final (stored) home position is unknown here.
            let motion_mode_for_line = if home_move {
                MotionMode::Rapid
            } else {
                current_motion_mode
            };

            match motion_mode_for_line {
                MotionMode::Rapid | MotionMode::Linear => {
                    let rapid = motion_mode_for_line == MotionMode::Rapid;
                    motion_counts[if rapid { 0 } else { 1 }] += 1;
                    commands.push(GCodeCommand::Move {
                        from: current_pos,
                        to: target,
                        rapid,
                        intensity: move_intensity,
                    });
                    bounds.update(current_pos.x, current_pos.y, current_pos.z);
                    bounds.update(target.x, target.y, target.z);
                    current_pos = target;
                }
                MotionMode::ArcCW | MotionMode::ArcCCW => {
                    let clockwise = motion_mode_for_line == MotionMode::ArcCW;
                    motion_counts[if clockwise { 2 } else { 3 }] += 1;
                    let drawn = push_arc_commands(
                        &mut commands,
                        &mut bounds,
                        current_pos,
                        target,
                        plane,
                        clockwise,
                        params.ijk(),
                        params.r,
                        move_intensity,
                    );
                    if !drawn {
                        debug!(
                            "Arc without usable I/J/K or R at line {}, treating as linear move",
                            line_num
                        );
                        commands.push(GCodeCommand::Move {
                            from: current_pos,
                            to: target,
                            rapid: false,
                            intensity: move_intensity,
                        });
                        bounds.update(current_pos.x, current_pos.y, current_pos.z);
                        bounds.update(target.x, target.y, target.z);
                    }
                    current_pos = target;
                }
                MotionMode::None => {
                    // There is no defined movement mode, ignore it
                    debug!("Movement without motion mode at line {}", line_num);
                }
            }
        }

        debug!(
            "Parse complete: G0={}, G1={}, G2={}, G3={}, total commands={}",
            motion_counts[0],
            motion_counts[1],
            motion_counts[2],
            motion_counts[3],
            commands.len()
        );

        // Calculate intensity range BEFORE moving commands to the cache
        let mut min_s = f32::MAX;
        let mut max_s = f32::MIN;

        for cmd in &commands {
            let intensity = match cmd {
                GCodeCommand::Move { intensity, .. } => intensity.unwrap_or(0.0),
                GCodeCommand::Arc { intensity, .. } => intensity.unwrap_or(0.0),
                GCodeCommand::Dwell { .. } => continue,
            };

            if intensity > 0.0 {
                min_s = min_s.min(intensity);
                max_s = max_s.max(intensity);
            }
        }

        if min_s != f32::MAX && max_s != f32::MIN {
            self.min_intensity = min_s;
            self.max_intensity = max_s;
            debug!("Intensity range: S=[{:.1}, {:.1}]", min_s, max_s);
        } else {
            // Default values if no intensity was detected
            self.min_intensity = 0.0;
            self.max_intensity = 1000.0;
        }

        self.current_pos = current_pos;
        self.current_rotary = rotary;

        // Move commands to the cache (last step)
        self.toolpath_cache.update(new_hash, commands);
        self.dirty = true;

        // Update visualizer bounds from parsed bounds
        (
            self.min_x, self.max_x, self.min_y, self.max_y, self.min_z, self.max_z,
        ) = bounds.finalize_with_padding(BOUNDS_PADDING_FACTOR);

        // Update rotary bounds
        if bounds.has_rotary_movement() {
            let ((min_a, max_a), (min_b, max_b), (min_c, max_c)) = bounds.rotary_ranges();
            self.min_a = min_a;
            self.max_a = max_a;
            self.min_b = min_b;
            self.max_b = max_b;
            self.min_c = min_c;
            self.max_c = max_c;

            debug!(
                "Rotary bounds: A=[{:.2}°, {:.2}°], B=[{:.2}°, {:.2}°], C=[{:.2}°, {:.2}°]",
                self.min_a, self.max_a, self.min_b, self.max_b, self.min_c, self.max_c
            );
        }

        debug!(
            "Bounds: x=[{:.2}, {:.2}], y=[{:.2}, {:.2}], z=[{:.2}, {:.2}]",
            self.min_x, self.max_x, self.min_y, self.max_y, self.min_z, self.max_z
        );
    }

    /// Get bounds information
    pub fn get_bounds(&self) -> (f32, f32, f32, f32) {
        (self.min_x, self.max_x, self.min_y, self.max_y)
    }

    pub fn commands(&self) -> &[GCodeCommand] {
        self.toolpath_cache.commands()
    }

    /// Increase zoom by 10%
    pub fn zoom_in(&mut self) {
        self.zoom_scale = (self.zoom_scale * ZOOM_STEP).min(MAX_ZOOM);
    }

    /// Decrease zoom by 10%
    pub fn zoom_out(&mut self) {
        self.zoom_scale = (self.zoom_scale / ZOOM_STEP).max(MIN_ZOOM);
    }

    /// Reset zoom to default (100%)
    pub fn reset_zoom(&mut self) {
        self.zoom_scale = 1.0;
    }

    /// Get current zoom scale as percentage
    pub fn get_zoom_percent(&self) -> u32 {
        (self.zoom_scale * 100.0).round() as u32
    }

    /// Pan view to the right by 10% of canvas width
    pub fn pan_right(&mut self, canvas_width: f32) {
        self.x_offset += canvas_width * PAN_PERCENTAGE;
    }

    /// Pan view to the left by 10% of canvas width
    pub fn pan_left(&mut self, canvas_width: f32) {
        self.x_offset -= canvas_width * PAN_PERCENTAGE;
    }

    /// Pan view down by 10% of canvas height
    pub fn pan_down(&mut self, canvas_height: f32) {
        self.y_offset -= canvas_height * PAN_PERCENTAGE;
    }

    /// Pan view up by 10% of canvas height
    pub fn pan_up(&mut self, canvas_height: f32) {
        self.y_offset += canvas_height * PAN_PERCENTAGE;
    }

    /// Reset pan to center (offset = 0)
    pub fn reset_pan(&mut self) {
        self.x_offset = 0.0;
        self.y_offset = 0.0;
    }

    /// Calculate zoom and offset to fit all cutting commands in view with margin
    pub fn fit_to_view(&mut self, canvas_width: f32, canvas_height: f32) {
        let mut bounds = Bounds::new();
        let mut has_content = false;

        // Collect bounds of all cutting moves
        for cmd in self.toolpath_cache.commands() {
            match cmd {
                GCodeCommand::Move {
                    from, to, rapid, ..
                } => {
                    if !rapid {
                        bounds.update(from.x, from.y, from.z);
                        bounds.update(to.x, to.y, to.z);
                        has_content = true;
                    }
                }
                GCodeCommand::Arc {
                    from,
                    to,
                    center,
                    clockwise,
                    ..
                } => {
                    update_bounds_with_arc(&mut bounds, *from, *to, *center, *clockwise);
                    has_content = true;
                }
                GCodeCommand::Dwell { pos, .. } => {
                    bounds.update(pos.x, pos.y, pos.z);
                    has_content = true;
                }
            }
        }

        if !has_content || !bounds.is_valid() {
            self.zoom_scale = 1.0;
            self.x_offset = 0.0;
            self.y_offset = 0.0;
            return;
        }

        let content_width = bounds.max_x - bounds.min_x;
        let content_height = bounds.max_y - bounds.min_y;

        // Apply 5% margin
        let margin_percent = core_constants::VIEW_PADDING as f32;
        let available_width = canvas_width * (1.0 - margin_percent * 2.0);
        let available_height = canvas_height * (1.0 - margin_percent * 2.0);

        if content_width == 0.0 || content_height == 0.0 {
            // Point content
            self.zoom_scale = 1.0;
            self.x_offset = -(bounds.min_x + bounds.max_x) / 2.0;
            self.y_offset = -(bounds.min_y + bounds.max_y) / 2.0;
            return;
        }

        // Calculate zoom to fit
        let scale_x = available_width / content_width;
        let scale_y = available_height / content_height;
        self.zoom_scale = scale_x.min(scale_y);

        // Center the content
        // The draw function applies: translate(screen_center) -> scale -> translate(offset)
        // So offset needs to be the negative center of the content to bring it to (0,0) before scaling/centering on screen
        let center_x = (bounds.min_x + bounds.max_x) / 2.0;
        let center_y = (bounds.min_y + bounds.max_y) / 2.0;

        self.x_offset = -center_x;
        self.y_offset = -center_y;
    }

    /// Get bounds of cutting moves only (excluding rapid moves)
    pub fn get_cutting_bounds(&self) -> Option<(f32, f32, f32, f32, f32, f32)> {
        let mut bounds = Bounds::new();
        let mut has_cutting_moves = false;

        for cmd in self.toolpath_cache.commands() {
            match cmd {
                GCodeCommand::Move {
                    from, to, rapid, ..
                } => {
                    if !*rapid {
                        bounds.update(from.x, from.y, from.z);
                        bounds.update(to.x, to.y, to.z);
                        has_cutting_moves = true;
                    }
                }
                GCodeCommand::Arc {
                    from,
                    to,
                    center,
                    clockwise,
                    ..
                } => {
                    update_bounds_with_arc(&mut bounds, *from, *to, *center, *clockwise);
                    has_cutting_moves = true;
                }
                GCodeCommand::Dwell { pos, .. } => {
                    bounds.update(pos.x, pos.y, pos.z);
                    has_cutting_moves = true;
                }
            }
        }

        if has_cutting_moves && bounds.is_valid() {
            Some((
                bounds.min_x,
                bounds.max_x,
                bounds.min_y,
                bounds.max_y,
                bounds.min_z,
                bounds.max_z,
            ))
        } else {
            None
        }
    }

    /// Get the start point of the toolpath (for debugging/testing)
    pub fn get_start_point(&self) -> Option<Point3D> {
        self.toolpath_cache.commands().first().map(|cmd| match cmd {
            GCodeCommand::Move { from, .. } => *from,
            GCodeCommand::Arc { from, .. } => *from,
            GCodeCommand::Dwell { pos, .. } => *pos,
        })
    }

    pub fn update(&mut self) {
        if let Some(receiver) = Rc::get_mut(&mut self.toolpath_receiver) {
            while let Ok(toolpaths) = receiver.try_recv() {
                self.current_toolpaths = toolpaths;
                self.dirty = true;
            }
        }
    }

    pub fn get_sender(&self) -> mpsc::Sender<Vec<Toolpath>> {
        self.toolpath_sender.clone()
    }

    /// Obtain the color for a given intensity.
    pub fn get_color_for_intensity(&self, intensity: f32) -> (f32, f32, f32) {
        if !self.use_intensity_colors {
            return (1.0, 1.0, 0.0);
        }

        if self.max_intensity <= self.min_intensity {
            return (1.0, 1.0, 0.0);
        }

        intensity_to_color(intensity, self.max_intensity)
    }

    /// Activate/deactivate colors by intensity
    pub fn toggle_intensity_colors(&mut self) {
        self.use_intensity_colors = !self.use_intensity_colors;
        self.dirty = true;
    }

    /// Obtain the detected intensity range
    pub fn get_intensity_range(&self) -> (f32, f32) {
        (self.min_intensity, self.max_intensity)
    }
}

impl Default for Visualizer {
    fn default() -> Self {
        Self::new()
    }
}
/*
/// Safely convert a float to i32, clamping to valid range
#[allow(dead_code)]
fn safe_to_i32(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    value.clamp(i32::MIN as f32 + 1.0, i32::MAX as f32 - 1.0) as i32
}
*/
