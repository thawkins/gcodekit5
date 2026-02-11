//! Jigsaw Puzzle Maker
//!
//! Generates G-code toolpaths for laser/CNC cutting jigsaw puzzles with interlocking pieces.

use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PuzzleParameters {
    pub width: f32,
    pub height: f32,
    pub pieces_across: i32,
    pub pieces_down: i32,
    pub kerf: f32,
    pub laser_passes: i32,
    pub laser_power: i32,
    pub feed_rate: f32,
    pub z_step_down: f32,
    pub seed: u32,
    pub tab_size_percent: f32,
    pub jitter_percent: f32,
    pub corner_radius: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    /// Number of axes on the target device (default 3).
    #[serde(default = "default_num_axes")]
    pub num_axes: u8,
}

fn default_num_axes() -> u8 {
    3
}

impl Default for PuzzleParameters {
    fn default() -> Self {
        Self {
            width: 200.0,
            height: 150.0,
            pieces_across: 4,
            pieces_down: 3,
            kerf: 0.5,
            laser_passes: 3,
            laser_power: 1000,
            feed_rate: 500.0,
            z_step_down: 0.5,
            seed: 42,
            tab_size_percent: 20.0,
            jitter_percent: 4.0,
            corner_radius: 2.0,
            offset_x: 10.0,
            offset_y: 10.0,
            num_axes: 3,
        }
    }
}

#[derive(Debug, Clone)]
struct Point {
    x: f32,
    y: f32,
}

impl Point {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

pub struct JigsawPuzzleMaker {
    params: PuzzleParameters,
    paths: Vec<Vec<Point>>,
    rng_state: f32,
}

impl JigsawPuzzleMaker {
    pub fn new(params: PuzzleParameters) -> Result<Self, String> {
        Self::validate_parameters(&params)?;
        Ok(Self {
            params: params.clone(),
            paths: Vec::new(),
            rng_state: params.seed as f32,
        })
    }

    fn random(&mut self) -> f32 {
        let x = (self.rng_state.sin() * 10000.0).abs();
        self.rng_state += 1.0;
        x - x.floor()
    }

    fn uniform(&mut self, min: f32, max: f32) -> f32 {
        let r = self.random();
        min + r * (max - min)
    }

    fn rbool(&mut self) -> bool {
        self.random() > 0.5
    }

    fn validate_parameters(params: &PuzzleParameters) -> Result<(), String> {
        if params.width < 50.0 || params.height < 50.0 {
            return Err("Puzzle dimensions must be at least 50mm".to_string());
        }

        if params.pieces_across < 2 || params.pieces_down < 2 {
            return Err("Must have at least 2x2 pieces".to_string());
        }

        if params.pieces_across > 20 || params.pieces_down > 20 {
            return Err("Maximum 20 pieces in any direction".to_string());
        }

        let piece_width = params.width / params.pieces_across as f32;
        let piece_height = params.height / params.pieces_down as f32;

        if piece_width < 15.0 || piece_height < 15.0 {
            return Err("Pieces too small (minimum 15mm per piece)".to_string());
        }

        if params.kerf < 0.0 || params.kerf > 2.0 {
            return Err("Kerf must be between 0 and 2mm".to_string());
        }

        if params.tab_size_percent < 10.0 || params.tab_size_percent > 30.0 {
            return Err("Tab size must be between 10% and 30%".to_string());
        }

        if params.jitter_percent < 0.0 || params.jitter_percent > 13.0 {
            return Err("Jitter must be between 0% and 13%".to_string());
        }

        if params.corner_radius < 0.0 || params.corner_radius > 10.0 {
            return Err("Corner radius must be between 0 and 10mm".to_string());
        }

        Ok(())
    }

    pub fn generate(&mut self) -> Result<(), String> {
        self.paths.clear();

        // Reset RNG state for reproducible results
        self.rng_state = self.params.seed as f32;

        let piece_width = self.params.width / self.params.pieces_across as f32;
        let piece_height = self.params.height / self.params.pieces_down as f32;

        // Use tab_size_percent parameter
        let tab_width = piece_width * (self.params.tab_size_percent / 100.0);
        let tab_height = piece_height * (self.params.tab_size_percent / 200.0);

        // Generate outer border with corner radius
        self.generate_border();

        // Generate vertical cuts with jitter
        for col in 1..self.params.pieces_across {
            let base_x = col as f32 * piece_width;
            let jitter_range = piece_width * (self.params.jitter_percent / 100.0);
            let x = base_x + self.uniform(-jitter_range, jitter_range);
            self.generate_vertical_cut(x, piece_height, tab_width, tab_height);
        }

        // Generate horizontal cuts with jitter
        for row in 1..self.params.pieces_down {
            let base_y = row as f32 * piece_height;
            let jitter_range = piece_height * (self.params.jitter_percent / 100.0);
            let y = base_y + self.uniform(-jitter_range, jitter_range);
            self.generate_horizontal_cut(y, piece_width, tab_width, tab_height);
        }

        Ok(())
    }

    fn generate_border(&mut self) {
        let mut path = Vec::new();
        let w = self.params.width;
        let h = self.params.height;
        let r = self.params.corner_radius;

        if r > 0.0 {
            // Rounded corners using arc approximation
            let steps = 4;

            // Bottom edge with bottom-left corner
            path.push(Point::new(r, 0.0));
            path.push(Point::new(w - r, 0.0));

            // Bottom-right corner
            for i in 0..=steps {
                let angle = (i as f32 / steps as f32) * PI / 2.0;
                path.push(Point::new(w - r + r * angle.sin(), r - r * angle.cos()));
            }

            // Right edge
            path.push(Point::new(w, r));
            path.push(Point::new(w, h - r));

            // Top-right corner
            for i in 0..=steps {
                let angle = (i as f32 / steps as f32) * PI / 2.0;
                path.push(Point::new(w - r + r * angle.cos(), h - r + r * angle.sin()));
            }

            // Top edge
            path.push(Point::new(w - r, h));
            path.push(Point::new(r, h));

            // Top-left corner
            for i in 0..=steps {
                let angle = (i as f32 / steps as f32) * PI / 2.0;
                path.push(Point::new(r - r * angle.sin(), h - r + r * angle.cos()));
            }

            // Left edge
            path.push(Point::new(0.0, h - r));
            path.push(Point::new(0.0, r));

            // Bottom-left corner
            for i in 0..=steps {
                let angle = (i as f32 / steps as f32) * PI / 2.0;
                path.push(Point::new(r - r * angle.cos(), r - r * angle.sin()));
            }

            path.push(Point::new(r, 0.0));
        } else {
            // Square corners
            path.push(Point::new(0.0, 0.0));
            path.push(Point::new(w, 0.0));
            path.push(Point::new(w, h));
            path.push(Point::new(0.0, h));
            path.push(Point::new(0.0, 0.0));
        }

        self.paths.push(path);
    }

    fn generate_vertical_cut(
        &mut self,
        x: f32,
        piece_height: f32,
        _tab_width: f32,
        _tab_height: f32,
    ) {
        let mut path = Vec::new();
        let num_tabs = self.params.pieces_down;
        let piece_width = self.params.width / self.params.pieces_across as f32;

        path.push(Point::new(x, 0.0));

        for row in 0..num_tabs {
            let y_start = row as f32 * piece_height;
            let flip = self.rbool();

            // Draw tab using Draradech's Bézier algorithm
            // Save current position
            let start_y = path.last().map(|p| p.y).unwrap_or(0.0);

            // Generate tab path starting from current Y position
            let mut tab_path = Vec::new();
            tab_path.push(Point::new(x, start_y));
            self.draw_tab_vertical(&mut tab_path, x, piece_height, piece_width, flip);

            // Offset tab to correct position and add to main path
            for point in tab_path.iter().skip(1) {
                path.push(Point::new(point.x, y_start + point.y));
            }
        }

        self.paths.push(path);
    }

    fn generate_horizontal_cut(
        &mut self,
        y: f32,
        piece_width: f32,
        _tab_width: f32,
        _tab_height: f32,
    ) {
        let mut path = Vec::new();
        let num_tabs = self.params.pieces_across;
        let piece_height = self.params.height / self.params.pieces_down as f32;

        path.push(Point::new(0.0, y));

        for col in 0..num_tabs {
            let x_start = col as f32 * piece_width;
            let flip = self.rbool();

            // Draw tab using Draradech's Bézier algorithm
            let start_x = path.last().map(|p| p.x).unwrap_or(0.0);

            let mut tab_path = Vec::new();
            tab_path.push(Point::new(start_x, y));
            self.draw_tab_horizontal(&mut tab_path, y, piece_width, piece_height, flip);

            // Offset tab to correct position and add to main path
            for point in tab_path.iter().skip(1) {
                path.push(Point::new(x_start + point.x, point.y));
            }
        }

        self.paths.push(path);
    }

    fn draw_tab_vertical(
        &mut self,
        path: &mut Vec<Point>,
        x: f32,
        piece_length: f32,
        piece_width: f32,
        flip: bool,
    ) {
        // Draradech algorithm: Uses Bézier curves with random jitter
        // t = tab size (as fraction), j = jitter, a-e = random values
        let t = self.params.tab_size_percent / 200.0; // Divide by 200 to get fraction of piece
        let j = self.params.jitter_percent / 100.0;

        // Generate random values for this tab (matches Draradech's next() function)
        let a = self.uniform(-j, j);
        let b = self.uniform(-j, j);
        let c = self.uniform(-j, j);
        let d = self.uniform(-j, j);
        let e = self.uniform(-j, j);

        let sign = if flip { -1.0 } else { 1.0 };

        // Convert l (length) and w (width) functions - vertical cuts
        let l = |v: f32| piece_length * v;
        let w = |v: f32| x + piece_width * v * sign;

        // 10 control points using Draradech's algorithm (p0-p9)
        // These create 3 cubic Bézier curves
        let p0 = Point::new(w(0.0), l(0.0));
        let p1 = Point::new(w(a), l(0.2));
        let p2 = Point::new(w(-t + c), l(0.5 + b + d));
        let p3 = Point::new(w(t + c), l(0.5 - t + b));
        let p4 = Point::new(w(3.0 * t + c), l(0.5 - 2.0 * t + b - d));
        let p5 = Point::new(w(3.0 * t + c), l(0.5 + 2.0 * t + b - d));
        let p6 = Point::new(w(t + c), l(0.5 + t + b));
        let p7 = Point::new(w(-t + c), l(0.5 + b + d));
        let p8 = Point::new(w(e), l(0.8));
        let p9 = Point::new(w(0.0), l(1.0));

        // Approximate cubic Bézier curves with line segments
        let steps = 10;

        // First cubic Bézier: p0-p1-p2-p3
        for i in 1..=steps {
            let t_val = i as f32 / steps as f32;
            let point = Self::cubic_bezier(p0.clone(), p1.clone(), p2.clone(), p3.clone(), t_val);
            path.push(point);
        }

        // Second cubic Bézier: p3-p4-p5-p6
        for i in 1..=steps {
            let t_val = i as f32 / steps as f32;
            let point = Self::cubic_bezier(p3.clone(), p4.clone(), p5.clone(), p6.clone(), t_val);
            path.push(point);
        }

        // Third cubic Bézier: p6-p7-p8-p9
        for i in 1..=steps {
            let t_val = i as f32 / steps as f32;
            let point = Self::cubic_bezier(p6.clone(), p7.clone(), p8.clone(), p9.clone(), t_val);
            path.push(point);
        }
    }

    fn draw_tab_horizontal(
        &mut self,
        path: &mut Vec<Point>,
        y: f32,
        piece_length: f32,
        piece_width: f32,
        flip: bool,
    ) {
        // Draradech algorithm for horizontal tabs
        let t = self.params.tab_size_percent / 200.0;
        let j = self.params.jitter_percent / 100.0;

        let a = self.uniform(-j, j);
        let b = self.uniform(-j, j);
        let c = self.uniform(-j, j);
        let d = self.uniform(-j, j);
        let e = self.uniform(-j, j);

        let sign = if flip { -1.0 } else { 1.0 };

        // For horizontal cuts, swap l and w
        let l = |v: f32| piece_length * v;
        let w = |v: f32| y + piece_width * v * sign;

        let p0 = Point::new(l(0.0), w(0.0));
        let p1 = Point::new(l(0.2), w(a));
        let p2 = Point::new(l(0.5 + b + d), w(-t + c));
        let p3 = Point::new(l(0.5 - t + b), w(t + c));
        let p4 = Point::new(l(0.5 - 2.0 * t + b - d), w(3.0 * t + c));
        let p5 = Point::new(l(0.5 + 2.0 * t + b - d), w(3.0 * t + c));
        let p6 = Point::new(l(0.5 + t + b), w(t + c));
        let p7 = Point::new(l(0.5 + b + d), w(-t + c));
        let p8 = Point::new(l(0.8), w(e));
        let p9 = Point::new(l(1.0), w(0.0));

        let steps = 10;

        for i in 1..=steps {
            let t_val = i as f32 / steps as f32;
            let point = Self::cubic_bezier(p0.clone(), p1.clone(), p2.clone(), p3.clone(), t_val);
            path.push(point);
        }

        for i in 1..=steps {
            let t_val = i as f32 / steps as f32;
            let point = Self::cubic_bezier(p3.clone(), p4.clone(), p5.clone(), p6.clone(), t_val);
            path.push(point);
        }

        for i in 1..=steps {
            let t_val = i as f32 / steps as f32;
            let point = Self::cubic_bezier(p6.clone(), p7.clone(), p8.clone(), p9.clone(), t_val);
            path.push(point);
        }
    }

    fn cubic_bezier(p0: Point, p1: Point, p2: Point, p3: Point, t: f32) -> Point {
        // Cubic Bézier curve formula: B(t) = (1-t)³P0 + 3(1-t)²tP1 + 3(1-t)t²P2 + t³P3
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        Point::new(
            mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
            mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y,
        )
    }

    pub fn to_gcode(&self, plunge_rate: f32, cut_depth: f32) -> String {
        let mut gcode = String::new();

        gcode.push_str("; Jigsaw Puzzle Maker G-code\n");
        gcode.push_str("; Enhanced with features from https://github.com/Draradech/jigsaw\n");
        gcode.push_str(&format!(
            "; Puzzle: {}x{} mm\n",
            self.params.width, self.params.height
        ));
        gcode.push_str(&format!(
            "; Pieces: {}x{} ({})\n",
            self.params.pieces_across,
            self.params.pieces_down,
            self.params.pieces_across * self.params.pieces_down
        ));
        gcode.push_str(&format!("; Kerf: {} mm\n", self.params.kerf));
        gcode.push_str(&format!("; Laser passes: {}\n", self.params.laser_passes));
        gcode.push_str(&format!("; Laser power: S{}\n", self.params.laser_power));
        gcode.push_str(&format!(
            "; Feed rate: {:.0} mm/min\n",
            self.params.feed_rate
        ));
        gcode.push_str(";\n");

        gcode.push_str("; Puzzle Parameters:\n");
        gcode.push_str(&format!(
            ";   Seed: {} (for reproducible patterns)\n",
            self.params.seed
        ));
        gcode.push_str(&format!(
            ";   Tab size: {:.1}%\n",
            self.params.tab_size_percent
        ));
        gcode.push_str(&format!(
            ";   Jitter: {:.1}% (randomness in piece positions)\n",
            self.params.jitter_percent
        ));
        gcode.push_str(&format!(
            ";   Corner radius: {:.1} mm\n",
            self.params.corner_radius
        ));
        gcode.push_str(";\n");

        gcode.push_str("; Puzzle Layout:\n");
        gcode.push_str(&format!(
            ";   Total pieces: {}\n",
            self.params.pieces_across * self.params.pieces_down
        ));
        let piece_w = self.params.width / self.params.pieces_across as f32;
        let piece_h = self.params.height / self.params.pieces_down as f32;
        gcode.push_str(&format!(
            ";   Piece size: {:.1}x{:.1} mm\n",
            piece_w, piece_h
        ));
        gcode.push('\n');

        gcode.push_str("; Initialization sequence\n");
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
                5.0, self.params.feed_rate
            ));
        }
        gcode.push('\n');

        for (i, path) in self.paths.iter().enumerate() {
            if i == 0 {
                gcode.push_str("; Outer border\n");
            } else if i <= self.params.pieces_across as usize {
                gcode.push_str(&format!("; Vertical cut {}\n", i));
            } else {
                gcode.push_str(&format!(
                    "; Horizontal cut {}\n",
                    i - self.params.pieces_across as usize
                ));
            }

            if let Some(first_point) = path.first() {
                gcode.push_str(&format!(
                    "G0 X{:.2} Y{:.2} ; Rapid to start\n",
                    first_point.x, first_point.y
                ));
                if self.params.num_axes >= 3 {
                    gcode.push_str(&format!(
                        "G1 Z{:.2} F{:.0} ; Plunge\n",
                        -cut_depth, plunge_rate
                    ));
                }

                for pass_num in 1..=self.params.laser_passes {
                    let z_depth = -(pass_num as f32 - 1.0) * self.params.z_step_down;
                    gcode.push_str(&format!(
                        "; Pass {}/{} at Z{:.2}\n",
                        pass_num, self.params.laser_passes, z_depth
                    ));

                    if pass_num > 1 && self.params.num_axes >= 3 {
                        gcode.push_str(&format!("G0 Z{:.2} ; Move to pass depth\n", z_depth));
                    }

                    gcode.push_str(&format!("M3 S{} ; Laser on\n", self.params.laser_power));

                    for point in path.iter().skip(1) {
                        gcode.push_str(&format!(
                            "G1 X{:.2} Y{:.2} F{:.0}\n",
                            point.x, point.y, self.params.feed_rate
                        ));
                    }

                    gcode.push_str("M5 ; Laser off\n");

                    if pass_num < self.params.laser_passes {
                        gcode.push_str(&format!(
                            "G0 X{:.2} Y{:.2} ; Return to start for next pass\n",
                            first_point.x, first_point.y
                        ));
                    }
                }
            }

            if self.params.num_axes >= 3 {
                gcode.push_str(&format!("G0 Z{:.2} ; Retract\n\n", 5.0));
            }
        }

        gcode.push_str("M5 ; Ensure laser off\n");
        if self.params.num_axes >= 3 {
            gcode.push_str("G0 Z10.0 ; Move to safe height\n");
        }
        gcode.push_str("G0 X0 Y0 ; Return to origin\n");
        gcode.push_str("M2 ; Program end\n");

        gcode
    }
}
