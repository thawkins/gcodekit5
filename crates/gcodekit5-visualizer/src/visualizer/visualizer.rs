//! 2D G-Code Visualizer
//! Parses G-Code toolpaths for canvas-based visualization

use super::toolpath_cache::ToolpathCache;
use super::viewport::{Bounds, ViewportTransform};
use gcodekit5_core::constants as core_constants;
use gcodekit5_designer::toolpath::Toolpath;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{mpsc, Arc};
use tracing::{debug, trace};

const CANVAS_PADDING: f32 = core_constants::CANVAS_PADDING_PX as f32;
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

/// Colors according to power
fn intensity_to_color(intensity: f32, max_intensity: f32) -> (f32, f32, f32) {
    if max_intensity <= 0.0 {
        return (0.5, 0.5, 0.5);
    }
    // Normalize intensity but inverted: 0 = white, 1 = black
    let engraving_color = 1.0 - (intensity / max_intensity).clamp(0.0, 0.5);
    (engraving_color/ 2.0, engraving_color, engraving_color/ 2.0)
}

/// New Gcode
#[derive(Debug, Clone, Copy, PartialEq)]
enum MotionMode {
    None,
    Rapid,  // G0
    Linear, // G1
    ArcCW,  // G2
    ArcCCW, // G3
}

/// Structure for extracted parameters
struct ExtractedParams {
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
    i: Option<f32>,
    j: Option<f32>,
    s: Option<f32>,
    f: Option<f32>,
}

impl ExtractedParams {
    fn new() -> Self {
        Self {
            x: None,
            y: None,
            z: None,
            i: None,
            j: None,
            s: None,
            f: None,
        }
    }

    fn has_movement(&self) -> bool {
        self.x.is_some() || self.y.is_some() || self.z.is_some()
    }

    fn has_arc_params(&self) -> bool {
        self.i.is_some() && self.j.is_some()
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

/// Movement command
#[derive(Debug, Clone)]
pub enum GCodeCommand {
    Move {
        from: Point3D,
        to: Point3D,
        rapid: bool,
        intensity: Option<f32>,
    },
    Arc {
        from: Point3D,
        to: Point3D,
        center: Point3D,
        clockwise: bool,
        intensity: Option<f32>,
    },
    Dwell {
        pos: Point3D,
        duration: f32,
    },
}

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

/// Visualizer state
#[derive(Debug, Clone)]
pub struct Visualizer {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
    pub current_pos: Point3D,
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
    toolpath_cache: ToolpathCache,
    viewport: ViewportTransform,
    /// Dirty flag — set when vertex data needs regeneration
    dirty: bool,

    toolpath_receiver: Arc<mpsc::Receiver<Vec<Toolpath>>>,
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
            min_x: -(core_constants::WORLD_EXTENT_MM as f32),
            max_x: core_constants::WORLD_EXTENT_MM as f32,
            min_y: -(core_constants::WORLD_EXTENT_MM as f32),
            max_y: core_constants::WORLD_EXTENT_MM as f32,
            min_z: 0.0,
            max_z: 100.0, // Default Z range
            current_pos: Point3D::new(0.0, 0.0, 0.0),
            current_intensity: 0.0,
            zoom_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
            show_grid: true,
            scale_factor: DEFAULT_SCALE_FACTOR,
            toolpath_cache: ToolpathCache::new(),
            viewport: ViewportTransform::new(CANVAS_PADDING),
            dirty: true,

            toolpath_receiver: Arc::new(receiver),
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

    /// Calculate and set offsets to position origin (0,0) at bottom-left of canvas
    pub fn set_default_view(&mut self, _canvas_width: f32, canvas_height: f32) {
        let (x_offset, y_offset) = self.viewport.offsets_to_place_world_point(
            self.min_x,
            self.min_y,
            self.zoom_scale,
            self.scale_factor,
            canvas_height,
            0.0,
            0.0,
            5.0,
            canvas_height - 5.0,
        );
        self.x_offset = x_offset;
        self.y_offset = y_offset;
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

    /// Calculate viewbox for the current view state
    pub fn get_viewbox(&self, width: f32, height: f32) -> (f32, f32, f32, f32) {
        self.viewport.viewbox(
            self.min_x,
            self.min_y,
            self.zoom_scale,
            self.scale_factor,
            self.x_offset,
            self.y_offset,
            width,
            height,
        )
    }

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

    /// Get number of commands parsed
    pub fn get_command_count(&self) -> usize {
        self.toolpath_cache.len()
    }

    /// Parse G-Code and extract movement commands
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
        self.current_intensity = 0.0;
        let mut bounds = Bounds::new();

        // Modal state
        let mut current_motion_mode = MotionMode::None;
        let mut _g0_count = 0;
        let mut _g1_count = 0;
        let mut _g2_count = 0;
        let mut _g3_count = 0;

        for (line_num, line) in gcode.lines().enumerate() {
            let original_line = line.trim();
            // Identify if it is a raster image
            if original_line.contains("; GcodeKit5 Image Engraving") {
                self.is_raster = true;
            }

            // Ignore empty lines and comments
            if original_line.is_empty()
                || original_line.starts_with(';')
                || original_line.starts_with('(')
            {
                continue;
            }

            // Remove inline comments (after ';')
            let line_without_comment = match original_line.find(';') {
                Some(idx) => original_line[..idx].trim(),
                None => original_line,
            };

            if line_without_comment.is_empty() {
                continue;
            }

            let upper_line = line_without_comment.to_uppercase();

            // Extract all parameters from the line
            let params = Self::extract_all_params(&upper_line);

            // Update intensity if there is an S command
            if let Some(intensity) = params.s {
                self.current_intensity = intensity;
            }

            // Check if there is a G command on this line
            let has_gcode = Self::extract_gcode_num(&upper_line).is_some();

            // If we find a G command, update the movement mode
            if let Some(gcode_num) = Self::extract_gcode_num(&upper_line) {
                trace!("Line {}: G{} command", line_num, gcode_num);
                match gcode_num {
                    0 => {
                        _g0_count += 1;
                        current_motion_mode = MotionMode::Rapid;
                    }
                    1 => {
                        _g1_count += 1;
                        current_motion_mode = MotionMode::Linear;
                    }
                    2 => {
                        _g2_count += 1;
                        current_motion_mode = MotionMode::ArcCW;
                    }
                    3 => {
                        _g3_count += 1;
                        current_motion_mode = MotionMode::ArcCCW;
                    }
                    4 => {
                        // G4 Dwell
                        let duration = params.x.unwrap_or(params.s.unwrap_or(0.0));
                        if duration > 0.0 {
                            commands.push(GCodeCommand::Dwell {
                                pos: current_pos,
                                duration,
                            });
                        }
                    }
                    _ => {}
                }
            }

            // If the line has coordinates, process the movement.
            if params.has_movement() {
                let new_x = params.x.unwrap_or(current_pos.x);
                let new_y = params.y.unwrap_or(current_pos.y);
                let new_z = params.z.unwrap_or(current_pos.z);

                // Determine the mode of movement for this line
                let motion_mode_for_line = if has_gcode {
                    // Old style: the G command is on the same line
                    current_motion_mode
                } else {
                    // Modal style: Use the current mode
                    current_motion_mode
                };

                // Process according to the movement mode
                match motion_mode_for_line {
                    MotionMode::Rapid => {
                        commands.push(GCodeCommand::Move {
                            from: current_pos,
                            to: Point3D::new(new_x, new_y, new_z),
                            rapid: true,
                            intensity: Some(self.current_intensity),
                        });
                        bounds.update(current_pos.x, current_pos.y, current_pos.z);
                        bounds.update(new_x, new_y, new_z);
                        current_pos = Point3D::new(new_x, new_y, new_z);
                    }
                    MotionMode::Linear => {
                        commands.push(GCodeCommand::Move {
                            from: current_pos,
                            to: Point3D::new(new_x, new_y, new_z),
                            rapid: false,
                            intensity: Some(self.current_intensity),
                        });
                        bounds.update(current_pos.x, current_pos.y, current_pos.z);
                        bounds.update(new_x, new_y, new_z);
                        current_pos = Point3D::new(new_x, new_y, new_z);
                    }
                    MotionMode::ArcCW | MotionMode::ArcCCW => {
                        if params.has_arc_params() {
                            let center_x = current_pos.x + params.i.unwrap_or(0.0);
                            let center_y = current_pos.y + params.j.unwrap_or(0.0);
                            let center = Point3D::new(center_x, center_y, current_pos.z);

                            let radius = ((current_pos.x - center_x).powi(2)
                                + (current_pos.y - center_y).powi(2))
                            .sqrt();
                            trace!("Arc: from=({:.2},{:.2}), to=({:.2},{:.2}), center=({:.2},{:.2}), radius={:.4}, cw={}",
                                   current_pos.x, current_pos.y, new_x, new_y, center_x, center_y, radius,
                                   motion_mode_for_line == MotionMode::ArcCW);

                            commands.push(GCodeCommand::Arc {
                                from: current_pos,
                                to: Point3D::new(new_x, new_y, new_z),
                                center,
                                clockwise: motion_mode_for_line == MotionMode::ArcCW,
                                intensity: Some(self.current_intensity),
                            });

                            bounds.update(current_pos.x, current_pos.y, current_pos.z);
                            bounds.update(new_x, new_y, new_z);
                            current_pos = Point3D::new(new_x, new_y, new_z);
                        } else {
                            // Arc without I/J parameters - treat as linear motion
                            debug!("Arc command without I/J parameters at line {}, treating as linear move", line_num);
                            commands.push(GCodeCommand::Move {
                                from: current_pos,
                                to: Point3D::new(new_x, new_y, new_z),
                                rapid: false,
                                intensity: Some(self.current_intensity),
                            });
                            bounds.update(current_pos.x, current_pos.y, current_pos.z);
                            bounds.update(new_x, new_y, new_z);
                            current_pos = Point3D::new(new_x, new_y, new_z);
                        }
                    }
                    MotionMode::None => {
                        // There is no defined movement mode, ignore it
                        debug!("Movement without motion mode at line {}", line_num);
                    }
                }
            }
        }

        debug!(
            "Parse complete: G0={}, G1={}, G2={}, G3={}, total commands={}",
            _g0_count,
            _g1_count,
            _g2_count,
            _g3_count,
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
            // Default values ​​if no intensity was detected
            self.min_intensity = 0.0;
            self.max_intensity = 1000.0;
        }

        (
            self.min_x, self.max_x, self.min_y, self.max_y, self.min_z, self.max_z,
        ) = bounds.finalize_with_padding(BOUNDS_PADDING_FACTOR);
        self.current_pos = current_pos;

        // Calculate intensity range
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
            self.min_intensity = 0.0;
            self.max_intensity = 1000.0;
        }

        // Move commands to the cache (last step)
        self.toolpath_cache.update(new_hash, commands);
        self.dirty = true;

        debug!(
            "Bounds: x=[{:.2}, {:.2}], y=[{:.2}, {:.2}], z=[{:.2}, {:.2}]",
            self.min_x, self.max_x, self.min_y, self.max_y, self.min_z, self.max_z
        );
    }

    /// Private auxiliary methods
    /// Extract all parameters from a G-code line
    fn extract_all_params(line: &str) -> ExtractedParams {
        let mut params = ExtractedParams::new();

        for part in line.split_whitespace() {
            if part.len() < 2 {
                continue;
            }

            let first_char = match part.chars().next() {
                Some(c) => c,
                None => continue,
            };

            let value_str = &part[1..];
            if let Ok(value) = value_str.parse::<f32>() {
                match first_char {
                    'X' => params.x = Some(value),
                    'Y' => params.y = Some(value),
                    'Z' => params.z = Some(value),
                    'I' => params.i = Some(value),
                    'J' => params.j = Some(value),
                    'S' => params.s = Some(value),
                    'F' => params.f = Some(value),
                    _ => {}
                }
            }
        }

        params
    }

    /// Extract the command number G from a line
    fn extract_gcode_num(line: &str) -> Option<u32> {
        if !line.starts_with('G') {
            return None;
        }
        let after_g = &line[1..];
        // Find end of number
        let end_idx = after_g
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(after_g.len());

        if end_idx == 0 {
            return None;
        }

        after_g[..end_idx].parse::<u32>().ok()
    }

    /// Get bounds information
    pub fn get_bounds(&self) -> (f32, f32, f32, f32) {
        (self.min_x, self.max_x, self.min_y, self.max_y)
    }

    pub fn toolpath_svg(&self) -> &str {
        self.toolpath_cache.toolpath_svg()
    }

    pub fn rapid_svg(&self) -> &str {
        self.toolpath_cache.rapid_svg()
    }

    pub fn g1_svg(&self) -> &str {
        self.toolpath_cache.g1_svg()
    }

    pub fn g2_svg(&self) -> &str {
        self.toolpath_cache.g2_svg()
    }

    pub fn g3_svg(&self) -> &str {
        self.toolpath_cache.g3_svg()
    }

    pub fn g4_svg(&self) -> &str {
        self.toolpath_cache.g4_svg()
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
                    from, to, center, ..
                } => {
                    // For arcs, we should strictly check the arc extents, but adding points + center is a safe approximation for now
                    bounds.update(from.x, from.y, from.z);
                    bounds.update(to.x, to.y, to.z);
                    // Including center ensures we don't clip the curve if it bows out, though it might be loose
                    let radius = ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();
                    bounds.update(center.x - radius, center.y - radius, center.z);
                    bounds.update(center.x + radius, center.y + radius, center.z);
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
                GCodeCommand::Move { to, rapid, .. } => {
                    if !*rapid {
                        bounds.update(to.x, to.y, to.z);
                        has_cutting_moves = true;
                    }
                }
                GCodeCommand::Arc { to, .. } => {
                    bounds.update(to.x, to.y, to.z);
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
        if let Some(receiver) = Arc::get_mut(&mut self.toolpath_receiver) {
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

/// Safely convert a float to i32, clamping to valid range
#[allow(dead_code)]
fn safe_to_i32(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    value.clamp(i32::MIN as f32 + 1.0, i32::MAX as f32 - 1.0) as i32
}
