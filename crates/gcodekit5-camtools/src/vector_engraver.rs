//! Laser Vector Image Engraving Tool
//!
//! Converts vector image formats (SVG, DXF) to G-code for laser cutting/engraving.
//! Supports path stroking, fill patterns, and various vector formats.

use anyhow::{Context, Result};
use image::{Rgb, RgbImage};
use lyon::algorithms::path::iterator::PathIterator;
use lyon::geom::Arc;
use lyon::math::point;
use lyon::path::Path;
use std::path::Path as StdPath;

/// Vector engraving parameters
#[derive(Debug, Clone)]
pub struct VectorEngravingParameters {
    /// Feed rate for cutting moves (mm/min)
    pub feed_rate: f32,
    /// Travel feed rate for rapid moves (mm/min)
    pub travel_rate: f32,
    /// Laser power for cutting (0-100%)
    pub cut_power: f32,
    /// Laser power for engraving/marking (0-100%)
    pub engrave_power: f32,
    /// Laser power scale (0-1000 for GRBL S parameter)
    pub power_scale: f32,
    /// Whether to perform pass cuts for thick materials
    pub multi_pass: bool,
    /// Number of passes if multi_pass is enabled
    pub num_passes: u32,
    /// Z-axis depth increment per pass (mm)
    pub z_step_down: f32,
    /// Invert cut and engrave power
    pub invert_power: bool,
    /// Desired output width in mm for scaling SVG/DXF
    pub desired_width: f32,
    /// X offset from machine origin
    pub offset_x: f32,
    /// Y offset from machine origin
    pub offset_y: f32,
    /// Enable hatching
    pub enable_hatch: bool,
    /// Hatch angle in degrees
    pub hatch_angle: f32,
    /// Hatch spacing in mm
    pub hatch_spacing: f32,
    /// Hatch tolerance (flattening)
    pub hatch_tolerance: f32,
    /// Enable laser dwell
    pub enable_dwell: bool,
    /// Dwell time in seconds
    pub dwell_time: f32,
    /// Enable cross hatching (second pass at 90 degrees offset)
    pub cross_hatch: bool,
    /// Number of axes on the target device (default 3).
    pub num_axes: u8,
}

impl Default for VectorEngravingParameters {
    fn default() -> Self {
        Self {
            feed_rate: 600.0,
            travel_rate: 3000.0,
            cut_power: 100.0,
            engrave_power: 50.0,
            power_scale: 1000.0,
            multi_pass: false,
            num_passes: 1,
            z_step_down: 0.5,
            invert_power: false,
            desired_width: 100.0,
            offset_x: 10.0,
            offset_y: 10.0,
            enable_hatch: false,
            hatch_angle: 45.0,
            hatch_spacing: 1.0,
            hatch_tolerance: 0.1,
            enable_dwell: false,
            dwell_time: 0.1,
            cross_hatch: false,
            num_axes: 3,
        }
    }
}

/// Vector engraver for SVG and DXF formats
#[derive(Debug)]
pub struct VectorEngraver {
    pub file_path: String,
    pub params: VectorEngravingParameters,
    pub paths: Vec<Path>,
    /// Scale factor from SVG units to mm
    #[allow(dead_code)]
    pub scale_factor: f32,
}

impl VectorEngraver {
    /// Create a new vector engraver from a vector file
    pub fn from_file<P: AsRef<StdPath>>(
        path: P,
        params: VectorEngravingParameters,
    ) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Validate file extension
        let ext = StdPath::new(&path_str)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .context("No file extension found")?;

        // Verify file exists
        if !StdPath::new(&path_str).exists() {
            anyhow::bail!("File not found: {}", path_str);
        }

        let (paths, scale_factor) = match ext.as_str() {
            "svg" => Self::parse_svg(&path_str)?,
            "dxf" => Self::parse_dxf(&path_str)?,
            _ => anyhow::bail!("Unsupported file format: {}. Supported: SVG, DXF", ext),
        };

        Ok(Self {
            file_path: path_str,
            params,
            paths,
            scale_factor,
        })
    }

    /// Parse SVG file and extract paths
    fn parse_svg(file_path: &str) -> Result<(Vec<Path>, f32)> {
        use regex::Regex;
        use std::fs;

        let path = std::path::Path::new(file_path);
        if !path.exists() {
            anyhow::bail!("SVG file does not exist: {}", file_path);
        }

        if !path.is_file() {
            anyhow::bail!("SVG path is not a file: {}", file_path);
        }

        let content = fs::read_to_string(file_path).context("Failed to read SVG file")?;

        let mut all_paths = Vec::new();
        let mut viewbox_width = 100.0f32;
        let mut _viewbox_height = 100.0f32;

        // Parse viewBox
        let re_viewbox =
            Regex::new(r#"viewBox\s*=\s*["']([^"']+)["']"#).expect("invalid viewbox regex");
        if let Some(caps) = re_viewbox.captures(&content) {
            let viewbox_str = &caps[1];
            let parts: Vec<&str> = viewbox_str.split_whitespace().collect();
            if parts.len() >= 4 {
                viewbox_width = parts[2].parse().unwrap_or(100.0);
                _viewbox_height = parts[3].parse().unwrap_or(100.0);
            }
        }

        // Parse group transform (simplified, only first one found)
        let mut group_transform = None;
        let re_g = Regex::new(r#"<g\s+([^>]+)>"#).expect("invalid g regex");
        if let Some(caps) = re_g.captures(&content) {
            let attrs = &caps[1];
            let re_transform =
                Regex::new(r#"transform\s*=\s*["']([^"']+)["']"#).expect("invalid transform regex");
            if let Some(t_caps) = re_transform.captures(attrs) {
                group_transform = Self::parse_matrix_transform(&t_caps[1]);
            }
        }

        // Parse paths
        let re_path = Regex::new(r#"<path\s+([^>]+)>"#).expect("invalid path regex");
        let re_d = Regex::new(r#"d\s*=\s*["']([^"']+)["']"#).expect("invalid d regex");

        for cap in re_path.captures_iter(&content) {
            let attrs = &cap[1];
            if let Some(d_cap) = re_d.captures(attrs) {
                let d_value = &d_cap[1];

                if let Ok(path) = Self::build_path_from_svg_data(d_value) {
                    // Apply group transform if present
                    let final_path = if let Some((a, b, c, d_coeff, e, f)) = group_transform {
                        let transform = lyon::math::Transform::new(a, b, c, d_coeff, e, f);
                        path.transformed(&transform)
                    } else {
                        path
                    };

                    all_paths.push(final_path);
                }
            }
        }

        // Mirror paths around X axis (flip Y) using center of bounding box
        if !all_paths.is_empty() {
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            let mut has_points = false;

            for path in &all_paths {
                for event in path.iter().flattened(0.1) {
                    match event {
                        lyon::path::Event::Begin { at } => {
                            min_y = min_y.min(at.y);
                            max_y = max_y.max(at.y);
                            has_points = true;
                        }
                        lyon::path::Event::Line { from, to } => {
                            min_y = min_y.min(from.y).min(to.y);
                            max_y = max_y.max(from.y).max(to.y);
                            has_points = true;
                        }
                        _ => {}
                    }
                }
            }

            if has_points && min_y < max_y {
                let center_y = (min_y + max_y) / 2.0;
                // Mirror around Y = center_y
                // y' = -y + 2*center_y
                // Transform matrix:
                // [ 1  0  0 ]
                // [ 0 -1  2*center_y ]
                let transform =
                    lyon::math::Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, 2.0 * center_y);

                let mirrored_paths: Vec<Path> = all_paths
                    .into_iter()
                    .map(|p| p.transformed(&transform))
                    .collect();

                all_paths = mirrored_paths;
            }
        }

        let scale_factor = if viewbox_width > 0.0 {
            100.0 / viewbox_width
        } else {
            0.1
        };

        Ok((all_paths, scale_factor))
    }

    /// Parse matrix transform from SVG matrix(a,b,c,d,e,f) format
    fn parse_matrix_transform(transform_str: &str) -> Option<(f32, f32, f32, f32, f32, f32)> {
        let trimmed = transform_str.trim();
        if !trimmed.starts_with("matrix(") || !trimmed.ends_with(")") {
            return None;
        }

        let inner = &trimmed[7..trimmed.len() - 1];
        let values: Result<Vec<f32>, _> =
            inner.split(',').map(|s| s.trim().parse::<f32>()).collect();

        if let Ok(vals) = values {
            if vals.len() == 6 {
                return Some((vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]));
            }
        }
        None
    }

    /// Build lyon Path from SVG path data string
    fn build_path_from_svg_data(data_str: &str) -> Result<Path> {
        let mut builder = Path::builder();
        let mut current_x = 0.0f32;
        let mut current_y = 0.0f32;
        let mut start_x = 0.0f32;
        let mut start_y = 0.0f32;
        let mut subpath_active = false;

        let commands = Self::tokenize_svg_path(data_str);
        let mut i = 0;

        while i < commands.len() {
            let cmd = &commands[i];

            match cmd.as_str() {
                "M" | "m" => {
                    if i + 2 < commands.len() {
                        let x: f32 = commands[i + 1].parse().unwrap_or(0.0);
                        let y: f32 = commands[i + 2].parse().unwrap_or(0.0);

                        if cmd == "m" {
                            current_x += x;
                            current_y += y;
                        } else {
                            current_x = x;
                            current_y = y;
                        }

                        if subpath_active {
                            builder.end(false);
                        }

                        start_x = current_x;
                        start_y = current_y;
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
                "L" | "l" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    let mut j = i + 1;
                    while j + 1 < commands.len() {
                        let x: f32 = commands[j].parse().unwrap_or(0.0);
                        let y: f32 = commands[j + 1].parse().unwrap_or(0.0);

                        if cmd == "l" {
                            current_x += x;
                            current_y += y;
                        } else {
                            current_x = x;
                            current_y = y;
                        }

                        builder.line_to(point(current_x, current_y));
                        j += 2;

                        if j >= commands.len()
                            || (commands[j].len() == 1
                                && commands[j].chars().all(|c| c.is_alphabetic()))
                            || commands[j].parse::<f32>().is_err()
                        {
                            break;
                        }
                    }
                    i = j;
                }
                "H" | "h" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    if i + 1 < commands.len() {
                        let x: f32 = commands[i + 1].parse().unwrap_or(0.0);
                        if cmd == "h" {
                            current_x += x;
                        } else {
                            current_x = x;
                        }
                        builder.line_to(point(current_x, current_y));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "V" | "v" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    if i + 1 < commands.len() {
                        let y: f32 = commands[i + 1].parse().unwrap_or(0.0);
                        if cmd == "v" {
                            current_y += y;
                        } else {
                            current_y = y;
                        }
                        builder.line_to(point(current_x, current_y));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "C" | "c" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    let mut j = i + 1;
                    while j + 5 < commands.len() {
                        let x1: f32 = commands[j].parse().unwrap_or(0.0);
                        let y1: f32 = commands[j + 1].parse().unwrap_or(0.0);
                        let x2: f32 = commands[j + 2].parse().unwrap_or(0.0);
                        let y2: f32 = commands[j + 3].parse().unwrap_or(0.0);
                        let x: f32 = commands[j + 4].parse().unwrap_or(0.0);
                        let y: f32 = commands[j + 5].parse().unwrap_or(0.0);

                        let mut cp1_x = x1;
                        let mut cp1_y = y1;
                        let mut cp2_x = x2;
                        let mut cp2_y = y2;
                        let mut end_x = x;
                        let mut end_y = y;

                        if cmd == "c" {
                            cp1_x += current_x;
                            cp1_y += current_y;
                            cp2_x += current_x;
                            cp2_y += current_y;
                            end_x += current_x;
                            end_y += current_y;
                        }

                        builder.cubic_bezier_to(
                            point(cp1_x, cp1_y),
                            point(cp2_x, cp2_y),
                            point(end_x, end_y),
                        );

                        current_x = end_x;
                        current_y = end_y;
                        j += 6;

                        if j >= commands.len()
                            || (commands[j].len() == 1
                                && commands[j].chars().all(|c| c.is_alphabetic()))
                            || commands[j].parse::<f32>().is_err()
                        {
                            break;
                        }
                    }
                    i = j;
                }
                "Q" | "q" => {
                    if !subpath_active {
                        builder.begin(point(current_x, current_y));
                        subpath_active = true;
                    }
                    let mut j = i + 1;
                    while j + 3 < commands.len() {
                        let x1: f32 = commands[j].parse().unwrap_or(0.0);
                        let y1: f32 = commands[j + 1].parse().unwrap_or(0.0);
                        let x: f32 = commands[j + 2].parse().unwrap_or(0.0);
                        let y: f32 = commands[j + 3].parse().unwrap_or(0.0);

                        let mut cp_x = x1;
                        let mut cp_y = y1;
                        let mut end_x = x;
                        let mut end_y = y;

                        if cmd == "q" {
                            cp_x += current_x;
                            cp_y += current_y;
                            end_x += current_x;
                            end_y += current_y;
                        }

                        builder.quadratic_bezier_to(point(cp_x, cp_y), point(end_x, end_y));

                        current_x = end_x;
                        current_y = end_y;
                        j += 4;

                        if j >= commands.len()
                            || (commands[j].len() == 1
                                && commands[j].chars().all(|c| c.is_alphabetic()))
                            || commands[j].parse::<f32>().is_err()
                        {
                            break;
                        }
                    }
                    i = j;
                }
                "Z" | "z" => {
                    if subpath_active {
                        builder.close();
                        subpath_active = false;
                    }
                    current_x = start_x;
                    current_y = start_y;
                    i += 1;
                }
                _ => i += 1,
            }
        }

        if subpath_active {
            builder.end(false);
        }
        Ok(builder.build())
    }

    /// Tokenize SVG path data into commands and numbers
    pub fn tokenize_svg_path(path_data: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();

        for ch in path_data.chars() {
            match ch {
                'M' | 'm' | 'L' | 'l' | 'H' | 'h' | 'V' | 'v' | 'C' | 'c' | 'S' | 's' | 'Q'
                | 'q' | 'T' | 't' | 'A' | 'a' | 'Z' | 'z' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    tokens.push(ch.to_string());
                }
                ' ' | ',' | '\n' | '\r' | '\t' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
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

    /// Parse DXF file and extract entities
    fn parse_dxf(file_path: &str) -> Result<(Vec<Path>, f32)> {
        use dxf::entities::EntityType;

        let mut file = std::fs::File::open(file_path).context("Failed to open DXF file")?;

        let drawing = dxf::Drawing::load(&mut file).context("Failed to parse DXF file")?;

        let mut all_paths = Vec::new();

        // Helper function to convert entity to Path Option
        fn entity_to_path(entity_type: &EntityType) -> Option<Path> {
            let mut builder = Path::builder();
            match entity_type {
                EntityType::Line(line) => {
                    builder.begin(point(line.p1.x as f32, line.p1.y as f32));
                    builder.line_to(point(line.p2.x as f32, line.p2.y as f32));
                    builder.end(false);
                    Some(builder.build())
                }
                EntityType::Circle(circle) => {
                    let center = point(circle.center.x as f32, circle.center.y as f32);
                    let radius = circle.radius as f32;
                    builder.add_ellipse(
                        center,
                        lyon::math::vector(radius, radius),
                        lyon::math::Angle::radians(0.0),
                        lyon::path::Winding::Positive,
                    );
                    Some(builder.build())
                }
                EntityType::Arc(arc) => {
                    let center = point(arc.center.x as f32, arc.center.y as f32);
                    let radius = arc.radius as f32;
                    let start_angle = lyon::math::Angle::degrees(arc.start_angle as f32);
                    let end_angle = lyon::math::Angle::degrees(arc.end_angle as f32);
                    let sweep_angle = end_angle - start_angle;

                    let start_point = center
                        + lyon::math::vector(
                            radius * start_angle.radians.cos(),
                            radius * start_angle.radians.sin(),
                        );

                    builder.begin(start_point);

                    let arc = Arc {
                        center,
                        radii: lyon::math::vector(radius, radius),
                        x_rotation: lyon::math::Angle::radians(0.0),
                        start_angle,
                        sweep_angle,
                    };

                    arc.for_each_cubic_bezier(&mut |ctrl| {
                        builder.cubic_bezier_to(ctrl.ctrl1, ctrl.ctrl2, ctrl.to);
                    });

                    builder.end(false);
                    Some(builder.build())
                }
                EntityType::LwPolyline(polyline) => {
                    if polyline.vertices.is_empty() {
                        return None;
                    }
                    let start = polyline.vertices[0];
                    builder.begin(point(start.x as f32, start.y as f32));
                    for v in polyline.vertices.iter().skip(1) {
                        builder.line_to(point(v.x as f32, v.y as f32));
                    }
                    // Bit 0 (value 1) indicates closed
                    if polyline.flags & 1 != 0 {
                        builder.close();
                    } else {
                        builder.end(false);
                    }
                    Some(builder.build())
                }
                EntityType::Polyline(polyline) => {
                    let vertices: Vec<_> = polyline.vertices().collect();
                    if vertices.is_empty() {
                        return None;
                    }
                    let start = &vertices[0].location;
                    builder.begin(point(start.x as f32, start.y as f32));
                    for v in vertices.iter().skip(1) {
                        let loc = &v.location;
                        builder.line_to(point(loc.x as f32, loc.y as f32));
                    }
                    // Check flags for closed polyline (bit 1 set)
                    if polyline.flags & 1 != 0 {
                        builder.close();
                    } else {
                        builder.end(false);
                    }
                    Some(builder.build())
                }
                _ => None,
            }
        }

        // Extract entities from ENTITIES section
        for entity in drawing.entities() {
            if let Some(path) = entity_to_path(&entity.specific) {
                all_paths.push(path);
            }
        }

        // Extract entities from blocks as well
        for block in drawing.blocks() {
            for entity in &block.entities {
                if let Some(path) = entity_to_path(&entity.specific) {
                    all_paths.push(path);
                }
            }
        }

        // DXF uses drawing units which default to mm, but may vary
        // Default scale: 1 unit = 1 mm
        let scale_factor = 1.0;

        Ok((all_paths, scale_factor))
    }

    /// Get file information
    pub fn file_info(&self) -> (String, String) {
        let ext = StdPath::new(&self.file_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        (self.file_path.clone(), ext.to_uppercase())
    }

    /// Estimate engraving time in seconds
    pub fn estimate_time(&self) -> f32 {
        // Calculate based on path lengths
        let total_distance: f32 = self
            .paths
            .iter()
            .map(|path| {
                let mut dist = 0.0;
                for event in path.iter().flattened(0.1) {
                    if let lyon::path::Event::Line { from, to } = event {
                        dist += (to - from).length();
                    }
                }
                dist
            })
            .sum();

        let cutting_time = (total_distance / self.params.feed_rate) * 60.0;
        let travel_time = (self.paths.len() as f32 * 10.0 / self.params.travel_rate) * 60.0;

        if self.params.multi_pass {
            (cutting_time + travel_time) * self.params.num_passes as f32
        } else {
            cutting_time + travel_time
        }
    }

    /// Calculate actual scale to apply based on desired width and current bounds
    fn calculate_actual_scale(&self) -> f32 {
        if self.paths.is_empty() {
            return 1.0;
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;

        for path in &self.paths {
            for event in path.iter().flattened(0.1) {
                match event {
                    lyon::path::Event::Begin { at } => {
                        min_x = min_x.min(at.x);
                        max_x = max_x.max(at.x);
                    }
                    lyon::path::Event::Line { from, to } => {
                        min_x = min_x.min(from.x).min(to.x);
                        max_x = max_x.max(from.x).max(to.x);
                    }
                    _ => {}
                }
            }
        }

        let current_width = max_x - min_x;
        if current_width > 0.0001 {
            self.params.desired_width / current_width
        } else {
            1.0
        }
    }

    /// Get the bounds of the vector paths (min_x, min_y, max_x, max_y)
    pub fn get_bounds(&self) -> (f32, f32, f32, f32) {
        if self.paths.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for path in &self.paths {
            for event in path.iter().flattened(0.1) {
                match event {
                    lyon::path::Event::Begin { at } => {
                        min_x = min_x.min(at.x);
                        max_x = max_x.max(at.x);
                        min_y = min_y.min(at.y);
                        max_y = max_y.max(at.y);
                    }
                    lyon::path::Event::Line { from, to } => {
                        min_x = min_x.min(from.x).min(to.x);
                        max_x = max_x.max(from.x).max(to.x);
                        min_y = min_y.min(from.y).min(to.y);
                        max_y = max_y.max(from.y).max(to.y);
                    }
                    _ => {}
                }
            }
        }

        (min_x, min_y, max_x, max_y)
    }

    /// Generate G-code for vector engraving
    pub fn generate_gcode(&self) -> Result<String> {
        self.generate_gcode_with_progress(|_| {})
    }

    /// Render paths to an image for preview
    pub fn render_preview(&self, width: u32, height: u32) -> RgbImage {
        let mut img = RgbImage::new(width, height);

        // Fill with background color (matches UI gray #808080)
        for pixel in img.pixels_mut() {
            *pixel = Rgb([128, 128, 128]);
        }

        if self.paths.is_empty() {
            return img;
        }

        // Calculate bounds
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for path in &self.paths {
            for event in path.iter().flattened(0.1) {
                match event {
                    lyon::path::Event::Begin { at } => {
                        min_x = min_x.min(at.x);
                        max_x = max_x.max(at.x);
                        min_y = min_y.min(at.y);
                        max_y = max_y.max(at.y);
                    }
                    lyon::path::Event::Line { from, to } => {
                        min_x = min_x.min(from.x).min(to.x);
                        max_x = max_x.max(from.x).max(to.x);
                        min_y = min_y.min(from.y).min(to.y);
                        max_y = max_y.max(from.y).max(to.y);
                    }
                    _ => {}
                }
            }
        }

        let data_width = max_x - min_x;
        let data_height = max_y - min_y;

        // Calculate scale to fit in image with padding
        let padding = 10.0;
        let avail_width = width as f32 - 2.0 * padding;
        let avail_height = height as f32 - 2.0 * padding;

        let scale = if data_width > 0.0 && data_height > 0.0 {
            let scale_x = avail_width / data_width;
            let scale_y = avail_height / data_height;
            scale_x.min(scale_y)
        } else {
            1.0
        };

        // Center the content
        // Note: Y axis in image is down, but Y axis in vector might be up or down.
        // Usually we want to preserve the coordinate system orientation relative to each other,
        // but fit it on screen.
        // Let's just map min_x, min_y to top-left (plus padding/centering).

        let offset_x = padding + (avail_width - data_width * scale) / 2.0 - min_x * scale;
        // For Y, if we want to flip it (standard Cartesian vs Image), we'd do:
        // y_screen = height - (y_world * scale + offset_y)
        // But let's stick to simple translation for now, assuming SVG/DXF coordinates are somewhat compatible or we just want to see the shape.
        // SVG usually has Y down. DXF usually has Y up.
        // VectorEngraver::parse_svg mirrors Y (lines 228-232).
        // So paths should be in a consistent orientation (Y up?).
        // If Y is up, we need to flip for image (Y down).

        // Let's assume we map min_y to max_y on screen (bottom to top) if we want Y up.
        // Or just map min_y to top if we treat it as image coords.
        // Let's map min_y to top (standard image coords) for simplicity,
        // but if it looks upside down for DXF we might need to adjust.
        // Since parse_svg flips Y, it might be "Y up" internally.
        // Let's try standard mapping first.

        let offset_y = padding + (avail_height - data_height * scale) / 2.0 - min_y * scale;

        // Draw paths
        let color = Rgb([255, 255, 255]); // White lines

        for path in &self.paths {
            let mut start_point = point(0.0, 0.0);
            let mut current_point = point(0.0, 0.0);

            for event in path.iter().flattened(0.5) {
                match event {
                    lyon::path::Event::Begin { at } => {
                        start_point = at;
                        current_point = at;
                    }
                    lyon::path::Event::Line { to, .. } => {
                        let x0 = (current_point.x * scale + offset_x) as i32;
                        let y0 = (current_point.y * scale + offset_y) as i32;
                        let x1 = (to.x * scale + offset_x) as i32;
                        let y1 = (to.y * scale + offset_y) as i32;

                        draw_line_segment(&mut img, x0, y0, x1, y1, color);
                        current_point = to;
                    }
                    lyon::path::Event::End { close, .. } => {
                        if close {
                            let x0 = (current_point.x * scale + offset_x) as i32;
                            let y0 = (current_point.y * scale + offset_y) as i32;
                            let x1 = (start_point.x * scale + offset_x) as i32;
                            let y1 = (start_point.y * scale + offset_y) as i32;

                            draw_line_segment(&mut img, x0, y0, x1, y1, color);
                        }
                    }
                    _ => {}
                }
            }
        }

        img
    }

    /// Generate G-code for vector engraving with progress callback
    pub fn generate_gcode_with_progress<F>(&self, mut progress_callback: F) -> Result<String>
    where
        F: FnMut(f32),
    {
        let mut gcode = String::new();

        gcode.push_str("; Laser Vector Engraving G-code\n");
        gcode.push_str(&format!(
            "; Generated: {}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        let (file_path, file_type) = self.file_info();
        gcode.push_str(&format!("; Input file: {}\n", file_path));
        gcode.push_str(&format!("; File type: {}\n", file_type));
        gcode.push_str(&format!(
            "; Feed rate: {:.0} mm/min\n",
            self.params.feed_rate
        ));
        gcode.push_str(&format!(
            "; Travel rate: {:.0} mm/min\n",
            self.params.travel_rate
        ));
        gcode.push_str(&format!("; Cut power: {:.0}%\n", self.params.cut_power));
        gcode.push_str(&format!(
            "; Engrave power: {:.0}%\n",
            self.params.engrave_power
        ));
        if self.params.multi_pass {
            gcode.push_str(&format!(
                "; Multi-pass: {} passes, {:.2} mm per pass\n",
                self.params.num_passes, self.params.z_step_down
            ));
        }
        gcode.push_str(&format!("; Number of paths: {}\n", self.paths.len()));
        gcode.push_str(&format!(
            "; Estimated time: {:.1} seconds\n",
            self.estimate_time()
        ));
        gcode.push_str(";\n");

        gcode.push_str("G21 ; Set units to millimeters\n");
        gcode.push_str("G90 ; Absolute positioning\n");
        gcode.push_str("G17 ; XY plane selection\n");
        gcode.push('\n');

        gcode.push_str("; Home and set work coordinate system\n");
        gcode.push_str("$H ; Home all axes (bottom-left corner)\n");
        gcode.push_str("G10 L2 P1 X0 Y0 Z0 ; Clear G54 offset\n");
        gcode.push_str("G54 ; Select work coordinate system 1\n");
        gcode.push_str(&format!(
            "G0 X{:.1} Y{:.1} ; Move to work origin\n",
            self.params.offset_x, self.params.offset_y
        ));
        gcode.push_str("G10 L20 P1 X0 Y0 Z0 ; Set current position as work zero\n");
        if self.params.num_axes >= 3 {
            gcode.push_str(&format!(
                "G0 Z{:.2} F{:.0} ; Move to safe height\n",
                5.0, self.params.travel_rate
            ));
        }
        gcode.push('\n');

        gcode.push_str("M5 ; Laser off\n");
        gcode.push('\n');

        progress_callback(0.1);

        let power_value = if self.params.invert_power {
            (self.params.engrave_power * self.params.power_scale / 100.0) as u32
        } else {
            (self.params.cut_power * self.params.power_scale / 100.0) as u32
        };

        // Calculate scale factor for spacing adjustment
        let scale = self.calculate_actual_scale();

        // Adjust hatch spacing to match SVG coordinate space
        // If we want 1mm spacing in output, and scale is 0.1 (10mm -> 1mm),
        // we need 10 units in SVG space. So spacing / scale.
        let effective_spacing = if scale > 0.00001 {
            self.params.hatch_spacing / scale
        } else {
            self.params.hatch_spacing
        };

        // Pre-process paths: associate hatches with their outlines
        struct ProcessedPath<'a> {
            outline: &'a Path,
            hatches: Vec<Path>,
        }

        let mut processed_paths = Vec::new();

        for path in &self.paths {
            let mut hatches = Vec::new();

            if self.params.enable_hatch {
                // First pass
                let h = crate::hatch_generator::generate_hatch(
                    path,
                    self.params.hatch_angle,
                    effective_spacing,
                    self.params.hatch_tolerance,
                );
                hatches.extend(h);

                // Second pass (Cross Hatch)
                if self.params.cross_hatch {
                    let ch = crate::hatch_generator::generate_hatch(
                        path,
                        self.params.hatch_angle + 90.0,
                        effective_spacing,
                        self.params.hatch_tolerance,
                    );
                    hatches.extend(ch);
                }
            }

            processed_paths.push(ProcessedPath {
                outline: path,
                hatches,
            });
        }

        let total_items = processed_paths.len() as f32;
        let num_passes = if self.params.multi_pass {
            self.params.num_passes as usize
        } else {
            1
        };

        // Multi-pass loop
        for pass in 0..num_passes {
            let z_depth = if self.params.multi_pass {
                -(pass as f32 * self.params.z_step_down)
            } else {
                0.0
            };

            if self.params.num_axes >= 3 {
                if pass == 0 && self.params.multi_pass {
                    gcode.push_str(&format!("G0 Z{:.2} ; Move to first pass depth\n", z_depth));
                } else if self.params.multi_pass && pass > 0 {
                    gcode.push_str(&format!("\n; Pass {} of {}\n", pass + 1, num_passes));
                    gcode.push_str(&format!(
                        "G0 Z{:.2} F{:.0} ; Move to safe height\n",
                        5.0, self.params.travel_rate
                    ));
                    gcode.push_str("; Lower Z for next pass\n");
                    gcode.push_str(&format!("G0 Z{:.2} ; Move to pass depth\n", z_depth));
                }
            }

            for (idx, item) in processed_paths.iter().enumerate() {
                // 1. Render Hatches for this path
                for hatch_path in &item.hatches {
                    let mut start_point = point(0.0, 0.0);
                    for event in hatch_path.iter().flattened(0.1) {
                        match event {
                            lyon::path::Event::Begin { at } => {
                                gcode.push_str("M5 ; Laser off\n");
                                gcode.push_str(&format!(
                                    "G0 X{:.3} Y{:.3} ; Move to hatch start\n",
                                    at.x * scale,
                                    at.y * scale
                                ));
                                start_point = at;
                            }
                            lyon::path::Event::Line { to, .. } => {
                                gcode.push_str(&format!(
                                    "G1 X{:.3} Y{:.3} F{:.0} M3 S{}\n",
                                    to.x * scale,
                                    to.y * scale,
                                    self.params.feed_rate,
                                    power_value
                                ));
                            }
                            lyon::path::Event::End { close, .. } => {
                                if close {
                                    gcode.push_str(&format!(
                                        "G1 X{:.3} Y{:.3} F{:.0} M3 S{} ; Close hatch\n",
                                        start_point.x * scale,
                                        start_point.y * scale,
                                        self.params.feed_rate,
                                        power_value
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // 2. Render Outline for this path
                let mut start_point = point(0.0, 0.0);
                for event in item.outline.iter().flattened(0.1) {
                    match event {
                        lyon::path::Event::Begin { at } => {
                            gcode.push_str("M5 ; Laser off\n");
                            gcode.push_str(&format!(
                                "G0 X{:.3} Y{:.3} ; Move to path start\n",
                                at.x * scale,
                                at.y * scale
                            ));
                            start_point = at;
                        }
                        lyon::path::Event::Line { to, .. } => {
                            gcode.push_str(&format!(
                                "G1 X{:.3} Y{:.3} F{:.0} M3 S{}\n",
                                to.x * scale,
                                to.y * scale,
                                self.params.feed_rate,
                                power_value
                            ));
                        }
                        lyon::path::Event::End { close, .. } => {
                            if close {
                                gcode.push_str(&format!(
                                    "G1 X{:.3} Y{:.3} F{:.0} M3 S{} ; Close path\n",
                                    start_point.x * scale,
                                    start_point.y * scale,
                                    self.params.feed_rate,
                                    power_value
                                ));
                            }
                        }
                        _ => {}
                    }
                }

                // Turn laser off after finishing this object (hatches + outline)
                gcode.push_str("M5 ; Laser off\n");
                if self.params.enable_dwell {
                    gcode.push_str(&format!(
                        "G4 P{:.1} ; Dwell to ensure laser fully powers down\n",
                        self.params.dwell_time
                    ));
                }

                let progress = 0.1
                    + ((pass as f32 * total_items + idx as f32)
                        / (num_passes as f32 * total_items))
                        * 0.8;
                progress_callback(progress);
            }
        }

        gcode.push_str("\n; End of engraving\n");
        gcode.push_str("M5 ; Laser off\n");
        gcode.push_str("G0 X0 Y0 ; Return to origin\n");

        progress_callback(1.0);

        Ok(gcode)
    }
}

fn draw_line_segment(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgb<u8>) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < img.width() as i32 && y >= 0 && y < img.height() as i32 {
            img.put_pixel(x as u32, y as u32, color);
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}
