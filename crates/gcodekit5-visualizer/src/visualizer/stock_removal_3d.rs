//! # 3D Stock Removal Simulation
//!
//! Simulates material removal from a rectangular stock block as the toolpath
//! is executed, and builds a surface mesh of what is left.
//!
//! ## Model
//! The cutter is a flat end mill, which removes everything above its tip.
//! Every cut therefore lowers the top of a stock column and never makes an
//! undercut, so the remaining stock is fully described by a [`HeightMap`]:
//! one material height per XY cell. Heights are exact (not quantised to the
//! cell size); a cell is cut when its centre lies under the tool.
//!
//! ## Coordinate convention
//! [`StockSimulator3D`] takes toolpaths in G-code work coordinates (mm).
//! * XY: the stock's lower-left corner is at `stock_origin_xy` (work
//!   `(0, 0)` by default), so the stock covers
//!   `origin.x ..= origin.x + width` and `origin.y ..= origin.y + height`.
//! * Z: the top face of the stock is at `stock_top_z` (Z0 by default, as most
//!   CAM output assumes; see [`StockZeroReference`] for programs zeroed on the
//!   stock bottom), so the material occupies
//!   `stock_top_z - thickness ..= stock_top_z` and material is removed only
//!   where the tool tip is below the stock top.
//!
//! The [`HeightMap`] itself uses stock-local coordinates (lower-left bottom
//! corner at the origin); [`StockSimulator3D::get_mesh`] translates the mesh
//! back into work coordinates.

use glam::Vec3;

use super::visualizer::GCodeCommand;

/// Kind of motion a [`ToolpathSegment`] describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolpathSegmentType {
    /// G0 rapid move (still cuts if it passes through the stock).
    RapidMove,
    /// G1 feed move.
    LinearMove,
    /// G2 clockwise arc in the XY plane.
    ArcCW,
    /// G3 counter-clockwise arc in the XY plane.
    ArcCCW,
}

/// One tool motion in work coordinates (mm).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolpathSegment {
    /// Motion kind.
    pub segment_type: ToolpathSegmentType,
    /// Start point (X, Y, Z).
    pub start: (f32, f32, f32),
    /// End point (X, Y, Z).
    pub end: (f32, f32, f32),
    /// Arc centre (X, Y); `None` for straight moves.
    pub center: Option<(f32, f32)>,
}

/// Convert parsed visualizer commands into simulator segments.
///
/// Dwells do not move the tool and are dropped. Arcs outside the XY plane are
/// already tessellated into straight moves by the parser.
///
/// # Arguments
/// * `commands` - Commands produced by [`super::Visualizer`].
///
/// # Returns
/// One [`ToolpathSegment`] per move or arc, in program order.
pub fn toolpath_from_commands(commands: &[GCodeCommand]) -> Vec<ToolpathSegment> {
    commands
        .iter()
        .filter_map(|cmd| match cmd {
            GCodeCommand::Move {
                from, to, rapid, ..
            } => Some(ToolpathSegment {
                segment_type: if *rapid {
                    ToolpathSegmentType::RapidMove
                } else {
                    ToolpathSegmentType::LinearMove
                },
                start: (from.x, from.y, from.z),
                end: (to.x, to.y, to.z),
                center: None,
            }),
            GCodeCommand::Arc {
                from,
                to,
                center,
                clockwise,
                ..
            } => Some(ToolpathSegment {
                segment_type: if *clockwise {
                    ToolpathSegmentType::ArcCW
                } else {
                    ToolpathSegmentType::ArcCCW
                },
                start: (from.x, from.y, from.z),
                end: (to.x, to.y, to.z),
                center: Some((center.x, center.y)),
            }),
            GCodeCommand::Dwell { .. } => None,
        })
        .collect()
}

/// Largest number of XY cells the simulation allocates (4 bytes each):
/// 2^27 cells, 512 MiB. A full 1220 x 2440 mm sheet at 0.19 mm needs
/// about 82 M cells.
pub const MAX_STOCK_CELLS: u64 = 1 << 27;

/// Heights below this (mm) count as cut through.
const CUT_THROUGH_EPSILON: f32 = 1e-4;

/// Number of cells covering `size` mm at `resolution` mm per cell. A size that
/// is a whole number of cells (up to float error) gets no extra cell.
fn cell_count(size: f32, resolution: f32) -> u64 {
    ((size / resolution) - 1e-4).ceil().max(1.0) as u64
}

/// Material height per XY cell of a rectangular stock block.
///
/// Cell `(col, row)` covers `col * resolution ..` in X and `row * resolution
/// ..` in Y, clipped to the stock size, so the last column and row may be
/// narrower than `resolution`. Heights are measured from the stock bottom and
/// lie in `0 ..= thickness`.
#[derive(Debug, Clone)]
pub struct HeightMap {
    cols: usize,
    rows: usize,
    resolution: f32,
    size_x: f32,
    size_y: f32,
    thickness: f32,
    heights: Vec<f32>,
}

impl HeightMap {
    /// Checks that a stock of `width x height x thickness` mm can be simulated
    /// at `resolution` mm per cell.
    ///
    /// # Errors
    /// A message when a dimension or the resolution is not a positive finite
    /// number, or the map would exceed [`MAX_STOCK_CELLS`].
    pub fn check_dimensions(
        width: f32,
        height: f32,
        thickness: f32,
        resolution: f32,
    ) -> Result<(), String> {
        for (name, value) in [
            ("width", width),
            ("height", height),
            ("thickness", thickness),
            ("resolution", resolution),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!(
                    "Stock {name} must be a positive number (got {value})"
                ));
            }
        }
        let total = cell_count(width, resolution).saturating_mul(cell_count(height, resolution));
        if total > MAX_STOCK_CELLS {
            return Err(format!(
                "Stock {width} x {height} mm is too large to simulate at {resolution} mm \
                 resolution ({total} cells; the limit is {MAX_STOCK_CELLS})"
            ));
        }
        Ok(())
    }

    /// Create an uncut stock block.
    ///
    /// # Arguments
    /// * `width`, `height`, `thickness` - Stock size in mm (X, Y, Z).
    /// * `resolution` - Cell edge length in mm.
    ///
    /// # Errors
    /// See [`HeightMap::check_dimensions`].
    pub fn new(width: f32, height: f32, thickness: f32, resolution: f32) -> Result<Self, String> {
        Self::check_dimensions(width, height, thickness, resolution)?;
        let cols = cell_count(width, resolution) as usize;
        let rows = cell_count(height, resolution) as usize;
        Ok(Self {
            cols,
            rows,
            resolution,
            size_x: width,
            size_y: height,
            thickness,
            heights: vec![thickness; cols * rows],
        })
    }

    /// Number of cells along X.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Number of cells along Y.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Cell edge length (mm).
    pub fn resolution(&self) -> f32 {
        self.resolution
    }

    /// Uncut stock thickness (mm).
    pub fn thickness(&self) -> f32 {
        self.thickness
    }

    /// All cell heights, row-major (`row * cols + col`).
    pub fn heights(&self) -> &[f32] {
        &self.heights
    }

    /// Material height (mm above the stock bottom) of a cell, or `None`
    /// outside the map.
    pub fn height_at(&self, col: usize, row: usize) -> Option<f32> {
        (col < self.cols && row < self.rows).then(|| self.heights[row * self.cols + col])
    }

    /// Lower edge (mm) of cell `index` along an axis of `size` mm, clipped to
    /// the stock.
    fn edge(&self, index: usize, size: f32) -> f32 {
        (index as f32 * self.resolution).min(size)
    }

    /// Range of cell indices whose span overlaps `lo ..= hi` (mm) along an
    /// axis with `len` cells, or `None` when it misses the stock.
    fn index_span(&self, lo: f32, hi: f32, len: usize) -> Option<std::ops::RangeInclusive<usize>> {
        let lo = (lo / self.resolution).floor();
        let hi = (hi / self.resolution).floor();
        if lo.is_nan() || hi.is_nan() || hi < 0.0 || lo >= len as f32 {
            return None;
        }
        Some(lo.max(0.0) as usize..=(hi as usize).min(len - 1))
    }

    /// Cut with a flat end mill: every cell whose centre lies within `radius`
    /// of `(x, y)` is lowered to `tip_z`.
    ///
    /// # Arguments
    /// * `x`, `y` - Tool axis position in stock-local mm.
    /// * `tip_z` - Tool tip height above the stock bottom (mm).
    /// * `radius` - Tool radius (mm).
    pub fn cut_flat(&mut self, x: f32, y: f32, tip_z: f32, radius: f32) {
        if tip_z.is_nan() || tip_z >= self.thickness {
            return;
        }
        let new_height = tip_z.max(0.0);
        let (Some(cols), Some(rows)) = (
            self.index_span(x - radius, x + radius, self.cols),
            self.index_span(y - radius, y + radius, self.rows),
        ) else {
            return;
        };
        let r_sq = radius * radius;
        for row in rows {
            let cy = 0.5 * (self.edge(row, self.size_y) + self.edge(row + 1, self.size_y));
            let dy = cy - y;
            for col in cols.clone() {
                let cx = 0.5 * (self.edge(col, self.size_x) + self.edge(col + 1, self.size_x));
                let dx = cx - x;
                if dx * dx + dy * dy <= r_sq {
                    let h = &mut self.heights[row * self.cols + col];
                    if *h > new_height {
                        *h = new_height;
                    }
                }
            }
        }
    }

    /// Height with cut-through cells snapped to zero; `0.0` outside the map.
    fn solid_height(&self, col: isize, row: isize) -> f32 {
        if col < 0 || row < 0 {
            return 0.0;
        }
        match self.height_at(col as usize, row as usize) {
            Some(h) if h > CUT_THROUGH_EPSILON => h,
            _ => 0.0,
        }
    }
}

/// Stock colour (steel grey, opaque).
const STOCK_COLOR: [f32; 4] = [0.44, 0.50, 0.56, 1.0];

/// Floats per mesh vertex: position (3), normal (3), colour (4).
pub const MESH_FLOATS_PER_VERTEX: usize = 10;

/// Append a quad as two triangles. `corners` must be counter-clockwise when
/// seen from the side `normal` points to.
fn push_quad(vertices: &mut Vec<f32>, corners: [Vec3; 4], normal: Vec3) {
    for index in [0, 1, 2, 0, 2, 3] {
        let v = corners[index];
        vertices.extend_from_slice(&[v.x, v.y, v.z, normal.x, normal.y, normal.z]);
        vertices.extend_from_slice(&STOCK_COLOR);
    }
}

/// Merge the cells of a `cols x rows` grid into rectangles of equal key.
///
/// Greedy meshing: from each unvisited cell with a key, grow a run along X,
/// then grow it along Y while the whole run matches. Cells whose key is
/// `None` are skipped.
///
/// # Returns
/// `(col0, row0, col1, row1, key)` with exclusive upper bounds.
fn greedy_rects<K: PartialEq + Copy>(
    cols: usize,
    rows: usize,
    key: impl Fn(usize, usize) -> Option<K>,
) -> Vec<(usize, usize, usize, usize, K)> {
    let mut visited = vec![false; cols * rows];
    let mut rects = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            if visited[row * cols + col] {
                continue;
            }
            let Some(k) = key(col, row) else {
                continue;
            };
            let matches = |c: usize, r: usize| !visited[r * cols + c] && key(c, r) == Some(k);
            let mut col1 = col + 1;
            while col1 < cols && matches(col1, row) {
                col1 += 1;
            }
            let mut row1 = row + 1;
            while row1 < rows && (col..col1).all(|c| matches(c, row1)) {
                row1 += 1;
            }
            for r in row..row1 {
                visited[r * cols + col..r * cols + col1].fill(true);
            }
            rects.push((col, row, col1, row1, k));
        }
    }
    rects
}

/// Surface mesh of the remaining stock in stock-local coordinates.
///
/// Flat regions are merged into large quads (see [`greedy_rects`]), and walls
/// are emitted only where neighbouring columns differ in height, merged along
/// their length. Layout: [`MESH_FLOATS_PER_VERTEX`] floats per vertex, three
/// vertices per triangle, counter-clockwise front faces.
pub fn generate_surface_mesh(map: &HeightMap) -> Vec<f32> {
    let mut vertices = Vec::new();
    let (cols, rows) = (map.cols, map.rows);
    let ex = |c: usize| map.edge(c, map.size_x);
    let ey = |r: usize| map.edge(r, map.size_y);
    let solid = |c: usize, r: usize| map.solid_height(c as isize, r as isize);

    // Top faces, merged by equal height.
    for (c0, r0, c1, r1, h) in greedy_rects(cols, rows, |c, r| {
        let h = solid(c, r);
        (h > 0.0).then_some(h)
    }) {
        let (x0, x1, y0, y1) = (ex(c0), ex(c1), ey(r0), ey(r1));
        push_quad(
            &mut vertices,
            [
                Vec3::new(x0, y0, h),
                Vec3::new(x1, y0, h),
                Vec3::new(x1, y1, h),
                Vec3::new(x0, y1, h),
            ],
            Vec3::Z,
        );
    }

    // Bottom faces under every column that still has material.
    for (c0, r0, c1, r1, ()) in greedy_rects(cols, rows, |c, r| (solid(c, r) > 0.0).then_some(())) {
        let (x0, x1, y0, y1) = (ex(c0), ex(c1), ey(r0), ey(r1));
        push_quad(
            &mut vertices,
            [
                Vec3::new(x1, y0, 0.0),
                Vec3::new(x0, y0, 0.0),
                Vec3::new(x0, y1, 0.0),
                Vec3::new(x1, y1, 0.0),
            ],
            Vec3::NEG_Z,
        );
    }

    // Walls facing X: boundary `b` lies between columns b-1 and b.
    for b in 0..=cols {
        let x = ex(b);
        let mut row = 0;
        while row < rows {
            let pair = |r: usize| {
                (
                    map.solid_height(b as isize - 1, r as isize),
                    map.solid_height(b as isize, r as isize),
                )
            };
            let (left, right) = pair(row);
            let mut end = row + 1;
            while end < rows && pair(end) == (left, right) {
                end += 1;
            }
            if left != right {
                let (y0, y1) = (ey(row), ey(end));
                push_quad(
                    &mut vertices,
                    [
                        Vec3::new(x, y0, left),
                        Vec3::new(x, y0, right),
                        Vec3::new(x, y1, right),
                        Vec3::new(x, y1, left),
                    ],
                    if left > right { Vec3::X } else { Vec3::NEG_X },
                );
            }
            row = end;
        }
    }

    // Walls facing Y: boundary `b` lies between rows b-1 and b.
    for b in 0..=rows {
        let y = ey(b);
        let mut col = 0;
        while col < cols {
            let pair = |c: usize| {
                (
                    map.solid_height(c as isize, b as isize - 1),
                    map.solid_height(c as isize, b as isize),
                )
            };
            let (below, above) = pair(col);
            let mut end = col + 1;
            while end < cols && pair(end) == (below, above) {
                end += 1;
            }
            if below != above {
                let (x0, x1) = (ex(col), ex(end));
                push_quad(
                    &mut vertices,
                    [
                        Vec3::new(x0, y, below),
                        Vec3::new(x1, y, below),
                        Vec3::new(x1, y, above),
                        Vec3::new(x0, y, above),
                    ],
                    if below > above { Vec3::Y } else { Vec3::NEG_Y },
                );
            }
            col = end;
        }
    }
    vertices
}

/// Which face of the stock block the program's work Z0 is set on.
///
/// Most CAM output zeroes Z on the top of the stock, so the material occupies
/// `-thickness ..= 0` and cuts go below Z0. GCodeKit's own designer (CNC mode)
/// zeroes Z on the bottom of the stock, so the material occupies
/// `0 ..= thickness`. The convention cannot be told reliably from the toolpath
/// itself (a top-zeroed program that only feeds at Z0, or air-cuts above the
/// stock, looks like a bottom-zeroed one), so it is a user setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StockZeroReference {
    /// Work Z0 is the top face of the stock (default; most CAM output).
    #[default]
    Top,
    /// Work Z0 is the bottom face of the stock (machine bed / spoilboard).
    Bottom,
}

impl StockZeroReference {
    /// Work Z (mm) of the stock top face for this convention.
    ///
    /// # Arguments
    /// * `thickness` - Stock thickness in mm.
    ///
    /// # Returns
    /// `0.0` for [`StockZeroReference::Top`], `thickness` for
    /// [`StockZeroReference::Bottom`].
    pub fn stock_top_z(self, thickness: f32) -> f32 {
        match self {
            Self::Top => 0.0,
            Self::Bottom => thickness,
        }
    }

    /// Stable index of this variant, for UI selectors (`Top` = 0, `Bottom` = 1).
    pub fn index(self) -> u32 {
        match self {
            Self::Top => 0,
            Self::Bottom => 1,
        }
    }

    /// Variant for a UI selector index; anything other than 1 maps to `Top`.
    ///
    /// # Arguments
    /// * `index` - Selector index as returned by [`StockZeroReference::index`].
    pub fn from_index(index: u32) -> Self {
        if index == 1 { Self::Bottom } else { Self::Top }
    }
}

/// Where on the stock the program's work X0 Y0 is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StockXyReference {
    /// Work X0 Y0 is the stock's lower-left corner (default).
    #[default]
    LowerLeft,
    /// Work X0 Y0 is the centre of the stock.
    Centre,
}

impl StockXyReference {
    /// Work XY (mm) of the stock's lower-left corner for this convention.
    ///
    /// # Arguments
    /// * `width`, `height` - Stock size in mm.
    pub fn stock_origin_xy(self, width: f32, height: f32) -> (f32, f32) {
        match self {
            Self::LowerLeft => (0.0, 0.0),
            Self::Centre => (-width / 2.0, -height / 2.0),
        }
    }

    /// Stable index of this variant, for UI selectors (`LowerLeft` = 0,
    /// `Centre` = 1).
    pub fn index(self) -> u32 {
        match self {
            Self::LowerLeft => 0,
            Self::Centre => 1,
        }
    }

    /// Variant for a UI selector index; anything other than 1 maps to
    /// `LowerLeft`.
    pub fn from_index(index: u32) -> Self {
        if index == 1 {
            Self::Centre
        } else {
            Self::LowerLeft
        }
    }
}

/// Header comment emitted by the designer for CNC (non-laser) programs, which
/// are always zeroed on the stock bottom.
const DESIGNER_CNC_HEADER: &str = "Generated G-code from Designer tool";

/// Prefix of an explicit stock-origin marker comment, e.g. `; Stock Z0: top`.
const STOCK_Z0_MARKER: &str = "stock z0:";

/// Read the stock Z0 convention recorded in a program's leading comments.
///
/// Only the header (the comment and blank lines before the first command) is
/// examined, and only explicit records are trusted; the toolpath's Z values are
/// never used. Recognised records:
/// * `; Stock Z0: top` / `; Stock Z0: bottom` (also in `( ... )` comments).
/// * The designer's CNC header `; Generated G-code from Designer tool`, whose
///   programs are zeroed on the stock bottom. This is only a fallback: an
///   explicit `Stock Z0` marker anywhere in the header takes precedence.
///
/// # Arguments
/// * `gcode` - Full program text.
///
/// # Returns
/// The recorded convention, or `None` when the program does not record one
/// (the caller should then keep the user's setting).
pub fn detect_stock_zero_reference(gcode: &str) -> Option<StockZeroReference> {
    // An explicit `Stock Z0` marker anywhere in the header wins; the designer's
    // title line is only a fallback, so an edited or re-zeroed program that
    // records its own origin is honoured.
    let mut designer_fallback = None;
    for line in gcode.lines() {
        let line = line.trim();
        if line.is_empty() || line == "%" {
            continue;
        }
        let comment = if let Some(rest) = line.strip_prefix(';') {
            rest
        } else if let Some(rest) = line.strip_prefix('(') {
            rest.strip_suffix(')').unwrap_or(rest)
        } else {
            // A non-comment line is the first command: the header is over.
            break;
        };
        let comment = comment.trim();
        if comment.contains(DESIGNER_CNC_HEADER) {
            designer_fallback = Some(StockZeroReference::Bottom);
            continue;
        }
        let lower = comment.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix(STOCK_Z0_MARKER) {
            match value.trim() {
                "top" => return Some(StockZeroReference::Top),
                "bottom" => return Some(StockZeroReference::Bottom),
                other => {
                    tracing::warn!(value = other, "Unrecognised stock Z0 marker value");
                }
            }
        }
    }
    designer_fallback
}

/// Report progress at most once per this many tool positions.
const PROGRESS_STRIDE: usize = 64;

/// Tool positions sampled per cell edge length along a segment.
const SAMPLES_PER_CELL: f32 = 2.0;

/// A segment prepared for sampling, in work coordinates.
enum SegmentPath {
    Line {
        start: Vec3,
        end: Vec3,
    },
    Arc {
        center: (f32, f32),
        start: Vec3,
        end: Vec3,
        r0: f32,
        r1: f32,
        a0: f32,
        span: f32,
    },
}

impl SegmentPath {
    /// Prepare `segment`; `None` for an arc without a centre or with zero
    /// radius, which cannot be sampled.
    fn new(segment: &ToolpathSegment) -> Option<Self> {
        let start = Vec3::from(segment.start);
        let end = Vec3::from(segment.end);
        match segment.segment_type {
            ToolpathSegmentType::RapidMove | ToolpathSegmentType::LinearMove => {
                Some(Self::Line { start, end })
            }
            ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                let center = segment.center?;
                let (sx, sy) = (start.x - center.0, start.y - center.1);
                let (ex, ey) = (end.x - center.0, end.y - center.1);
                let r0 = sx.hypot(sy);
                if r0.is_nan() || r0 < 0.001 {
                    return None;
                }
                let span = gcodekit5_core::gcode::arc::angular_travel(
                    (f64::from(sx), f64::from(sy)),
                    (f64::from(ex), f64::from(ey)),
                    segment.segment_type == ToolpathSegmentType::ArcCW,
                ) as f32;
                Some(Self::Arc {
                    center,
                    start,
                    end,
                    r0,
                    r1: ex.hypot(ey),
                    a0: sy.atan2(sx),
                    span,
                })
            }
        }
    }

    /// Path length (mm), including Z travel.
    fn length(&self) -> f32 {
        match *self {
            Self::Line { start, end } => start.distance(end),
            Self::Arc {
                start,
                end,
                r0,
                r1,
                span,
                ..
            } => {
                let arc = 0.5 * (r0 + r1) * span.abs();
                arc.hypot(end.z - start.z)
            }
        }
    }

    /// Lowest Z reached along the path.
    fn min_z(&self) -> f32 {
        match *self {
            Self::Line { start, end } | Self::Arc { start, end, .. } => start.z.min(end.z),
        }
    }

    /// Point at parameter `t` in `0 ..= 1`. Arcs interpolate the radius
    /// linearly so they end exactly on `end` even when the programmed start
    /// and end radii differ slightly.
    fn point(&self, t: f32) -> Vec3 {
        match *self {
            Self::Line { start, end } => start.lerp(end, t),
            Self::Arc {
                center,
                start,
                end,
                r0,
                r1,
                a0,
                span,
            } => {
                let r = r0 + (r1 - r0) * t;
                let a = a0 + span * t;
                Vec3::new(
                    center.0 + r * a.cos(),
                    center.1 + r * a.sin(),
                    start.z + (end.z - start.z) * t,
                )
            }
        }
    }
}

/// Material-removal simulator for a rectangular stock block.
///
/// See the module docs for the coordinate convention.
#[derive(Debug, Clone)]
pub struct StockSimulator3D {
    map: HeightMap,
    tool_radius: f32,
    /// Work Z (mm) of the top face of the stock.
    stock_top_z: f32,
    /// Work XY (mm) of the stock's lower-left corner.
    origin_xy: (f32, f32),
}

impl StockSimulator3D {
    /// Create a simulator whose stock top face is at work Z0 and whose
    /// lower-left corner is at work X0 Y0.
    ///
    /// # Arguments
    /// * `width`, `height`, `thickness` - Stock size in mm (X, Y, Z).
    /// * `resolution` - Cell edge length in mm.
    /// * `tool_radius` - Flat end mill radius in mm.
    ///
    /// # Errors
    /// A message when the stock is invalid or too large (see
    /// [`HeightMap::check_dimensions`]) or the tool radius is not a positive
    /// finite number.
    pub fn new(
        width: f32,
        height: f32,
        thickness: f32,
        resolution: f32,
        tool_radius: f32,
    ) -> Result<Self, String> {
        if !tool_radius.is_finite() || tool_radius <= 0.0 {
            return Err(format!(
                "Tool diameter must be a positive number (got {})",
                tool_radius * 2.0
            ));
        }
        Ok(Self {
            map: HeightMap::new(width, height, thickness, resolution)?,
            tool_radius,
            stock_top_z: 0.0,
            origin_xy: (0.0, 0.0),
        })
    }

    /// Set the work Z (mm) of the top face of the stock (default 0).
    ///
    /// # Arguments
    /// * `stock_top_z` - Work Z of the stock top; use `thickness` for programs
    ///   that zero on the machine bed / bottom of the stock. Non-finite values
    ///   are ignored.
    ///
    /// # Returns
    /// The simulator, for chaining.
    pub fn with_stock_top_z(mut self, stock_top_z: f32) -> Self {
        if stock_top_z.is_finite() {
            self.stock_top_z = stock_top_z;
        }
        self
    }

    /// Place the stock for a program zeroed on the given stock face.
    ///
    /// # Arguments
    /// * `reference` - Which stock face is work Z0.
    ///
    /// # Returns
    /// The simulator, for chaining.
    pub fn with_stock_zero_reference(self, reference: StockZeroReference) -> Self {
        let top = reference.stock_top_z(self.map.thickness);
        self.with_stock_top_z(top)
    }

    /// Set the work XY (mm) of the stock's lower-left corner (default 0, 0).
    ///
    /// # Arguments
    /// * `x`, `y` - Work coordinates of the corner; use `-width / 2`,
    ///   `-height / 2` for a program zeroed on the stock centre. Non-finite
    ///   values are ignored.
    ///
    /// # Returns
    /// The simulator, for chaining.
    pub fn with_stock_origin_xy(mut self, x: f32, y: f32) -> Self {
        if x.is_finite() && y.is_finite() {
            self.origin_xy = (x, y);
        }
        self
    }

    /// Place the stock for a program whose work X0 Y0 is at `reference`.
    ///
    /// # Returns
    /// The simulator, for chaining.
    pub fn with_stock_xy_reference(self, reference: StockXyReference) -> Self {
        let (x, y) = reference.stock_origin_xy(self.map.size_x, self.map.size_y);
        self.with_stock_origin_xy(x, y)
    }

    /// Work Z (mm) of the top face of the stock.
    pub fn stock_top_z(&self) -> f32 {
        self.stock_top_z
    }

    /// Work XY (mm) of the stock's lower-left corner.
    pub fn stock_origin_xy(&self) -> (f32, f32) {
        self.origin_xy
    }

    /// The material height map, in stock-local coordinates.
    pub fn height_map(&self) -> &HeightMap {
        &self.map
    }

    /// Run the whole toolpath without progress reporting.
    pub fn simulate_toolpath(&mut self, toolpath: &[ToolpathSegment]) {
        let _ = self.simulate_toolpath_with_progress(toolpath, |_| true);
    }

    /// Cut with the tool tip at `tip` (work coordinates).
    fn remove_at_tool_tip(&mut self, tip: Vec3) {
        let stock_bottom_z = self.stock_top_z - self.map.thickness;
        self.map.cut_flat(
            tip.x - self.origin_xy.0,
            tip.y - self.origin_xy.1,
            tip.z - stock_bottom_z,
            self.tool_radius,
        );
    }

    /// Simulate material removal, reporting progress by distance travelled.
    ///
    /// Every move cuts, rapids included: a rapid through the stock removes
    /// material just as it would on the machine.
    ///
    /// # Arguments
    /// * `toolpath` - Segments in work coordinates.
    /// * `on_progress` - Called with the completed fraction (`0.0 ..= 1.0`);
    ///   return `false` to cancel.
    ///
    /// # Returns
    /// `false` if the run was cancelled.
    pub fn simulate_toolpath_with_progress<F>(
        &mut self,
        toolpath: &[ToolpathSegment],
        mut on_progress: F,
    ) -> bool
    where
        F: FnMut(f32) -> bool,
    {
        let paths: Vec<Option<(SegmentPath, f32)>> = toolpath
            .iter()
            .map(|segment| {
                let path = SegmentPath::new(segment)?;
                let length = path.length();
                length.is_finite().then_some((path, length))
            })
            .collect();
        let total: f32 = paths.iter().flatten().map(|(_, length)| length).sum();
        let total = total.max(0.001);
        let step = self.map.resolution / SAMPLES_PER_CELL;
        let mut done = 0.0;
        let mut since_report = 0;

        if !on_progress(0.0) {
            return false;
        }
        for (path, length) in paths.iter().flatten() {
            // A segment that stays at or above the stock top cannot cut.
            if path.min_z() < self.stock_top_z {
                let steps = ((length / step).ceil() as usize).max(1);
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    self.remove_at_tool_tip(path.point(t));
                    since_report += 1;
                    if since_report >= PROGRESS_STRIDE {
                        since_report = 0;
                        if !on_progress(((done + length * t) / total).min(1.0)) {
                            return false;
                        }
                    }
                }
            }
            done += length;
        }
        on_progress(1.0)
    }

    /// Surface mesh of the remaining stock in work coordinates.
    ///
    /// Same layout as [`generate_surface_mesh`] ([`MESH_FLOATS_PER_VERTEX`]
    /// floats per vertex), translated so the stock lower-left corner is at
    /// `stock_origin_xy` and its top at `stock_top_z`.
    pub fn get_mesh(&self) -> Vec<f32> {
        let mut mesh = generate_surface_mesh(&self.map);
        let offset = Vec3::new(
            self.origin_xy.0,
            self.origin_xy.1,
            self.stock_top_z - self.map.thickness,
        );
        if offset != Vec3::ZERO {
            for vertex in mesh.as_chunks_mut::<MESH_FLOATS_PER_VERTEX>().0 {
                vertex[0] += offset.x;
                vertex[1] += offset.y;
                vertex[2] += offset.z;
            }
        }
        mesh
    }
}
