use super::visualizer::{GCodeCommand, Point3D};
use std::fmt::Write;
use tracing::{debug, trace};

#[derive(Debug, Default, Clone)]
pub struct ToolpathCache {
    content_hash: u64,
    commands: Vec<GCodeCommand>,
    cached_path: String,
    cached_rapid_path: String,
    cached_g1_path: String,
    cached_g2_path: String,
    cached_g3_path: String,
    cached_g4_path: String,
}

impl ToolpathCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn needs_update(&self, new_hash: u64) -> bool {
        self.content_hash != new_hash || self.commands.is_empty()
    }

    pub fn update(&mut self, new_hash: u64, commands: Vec<GCodeCommand>) {
        self.content_hash = new_hash;
        self.commands = commands;
        self.rebuild_paths();
    }

    pub fn commands(&self) -> &[GCodeCommand] {
        &self.commands
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn toolpath_svg(&self) -> &str {
        &self.cached_path
    }

    pub fn rapid_svg(&self) -> &str {
        &self.cached_rapid_path
    }

    pub fn g1_svg(&self) -> &str {
        &self.cached_g1_path
    }

    pub fn g2_svg(&self) -> &str {
        &self.cached_g2_path
    }

    pub fn g3_svg(&self) -> &str {
        &self.cached_g3_path
    }

    pub fn g4_svg(&self) -> &str {
        &self.cached_g4_path
    }

    fn rebuild_paths(&mut self) {
        debug!("Rebuilding SVG paths from {} commands", self.commands.len());

        self.cached_path.clear();
        self.cached_rapid_path.clear();
        self.cached_g1_path.clear();
        self.cached_g2_path.clear();
        self.cached_g3_path.clear();
        self.cached_g4_path.clear();

        if self.commands.is_empty() {
            debug!("No commands to render");
            return;
        }

        self.cached_path.reserve(self.commands.len() * 25);
        self.cached_rapid_path.reserve(self.commands.len() * 10);
        self.cached_g1_path.reserve(self.commands.len() * 15);
        self.cached_g2_path.reserve(self.commands.len() * 15);
        self.cached_g3_path.reserve(self.commands.len() * 15);
        self.cached_g4_path.reserve(self.commands.len() * 5);

        let mut last_pos: Option<Point3D> = None;
        let mut last_g1_pos: Option<Point3D> = None;
        let mut last_g2_pos: Option<Point3D> = None;
        let mut last_g3_pos: Option<Point3D> = None;
        let mut arc_count = 0;
        let mut invalid_arc_count = 0;

        for (cmd_idx, cmd) in self.commands.iter().enumerate() {
            match cmd {
                GCodeCommand::Move {
                    from,
                    to,
                    rapid,
                    intensity: _,
                } => {
                    if *rapid {
                        let _ = write!(
                            self.cached_rapid_path,
                            "M {:.2} {:.2} L {:.2} {:.2} ",
                            from.x, -from.y, to.x, -to.y
                        );
                        last_pos = None;
                        continue;
                    }

                    // Update combined path
                    if last_pos.is_none() || last_pos != Some(*from) {
                        let _ = write!(self.cached_path, "M {:.2} {:.2} ", from.x, -from.y);
                    }
                    let _ = write!(self.cached_path, "L {:.2} {:.2} ", to.x, -to.y);
                    last_pos = Some(*to);

                    // Update G1 path
                    if last_g1_pos.is_none() || last_g1_pos != Some(*from) {
                        let _ = write!(self.cached_g1_path, "M {:.2} {:.2} ", from.x, -from.y);
                    }
                    let _ = write!(self.cached_g1_path, "L {:.2} {:.2} ", to.x, -to.y);
                    last_g1_pos = Some(*to);
                }
                GCodeCommand::Arc {
                    from,
                    to,
                    center,
                    clockwise,
                    intensity: _,
                } => {
                    arc_count += 1;
                    let radius = ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();

                    // Update combined path
                    if last_pos.is_none() || last_pos != Some(*from) {
                        let _ = write!(self.cached_path, "M {:.2} {:.2} ", from.x, -from.y);
                    }

                    // Skip invalid arcs (radius is zero, NaN, or Infinity)
                    // Treat them as line segments instead
                    if radius.is_finite() && radius > 0.001 {
                        trace!("Arc[{}]: valid radius={:.4}", cmd_idx, radius);

                        let sweep = if *clockwise { 0 } else { 1 };

                        use std::f32::consts::PI;
                        let start_angle = (from.y - center.y).atan2(from.x - center.x);
                        let end_angle = (to.y - center.y).atan2(to.x - center.x);
                        let mut angle_diff = if *clockwise {
                            start_angle - end_angle
                        } else {
                            end_angle - start_angle
                        };

                        while angle_diff < 0.0 {
                            angle_diff += 2.0 * PI;
                        }
                        while angle_diff >= 2.0 * PI {
                            angle_diff -= 2.0 * PI;
                        }

                        let large_arc = if angle_diff > PI { 1 } else { 0 };

                        let _ = write!(
                            self.cached_path,
                            "A {:.2} {:.2} 0 {} {} {:.2} {:.2} ",
                            radius, radius, large_arc, sweep, to.x, -to.y
                        );
                        last_pos = Some(*to);

                        // Update G2/G3 path
                        let (target_path, last_target_pos) = if *clockwise {
                            (&mut self.cached_g2_path, &mut last_g2_pos)
                        } else {
                            (&mut self.cached_g3_path, &mut last_g3_pos)
                        };

                        if last_target_pos.is_none() || *last_target_pos != Some(*from) {
                            let _ = write!(target_path, "M {:.2} {:.2} ", from.x, -from.y);
                        }

                        let _ = write!(
                            target_path,
                            "A {:.2} {:.2} 0 {} {} {:.2} {:.2} ",
                            radius, radius, large_arc, sweep, to.x, -to.y
                        );
                        *last_target_pos = Some(*to);
                    } else {
                        invalid_arc_count += 1;
                        trace!(
                            "Arc[{}]: invalid radius={:.4} (finite={}), treating as line segment",
                            cmd_idx,
                            radius,
                            radius.is_finite()
                        );

                        // Invalid arc - treat as a line segment
                        let _ = write!(self.cached_path, "L {:.2} {:.2} ", to.x, -to.y);
                        last_pos = Some(*to);

                        // Update G2/G3 path
                        let (target_path, last_target_pos) = if *clockwise {
                            (&mut self.cached_g2_path, &mut last_g2_pos)
                        } else {
                            (&mut self.cached_g3_path, &mut last_g3_pos)
                        };

                        if last_target_pos.is_none() || *last_target_pos != Some(*from) {
                            let _ = write!(target_path, "M {:.2} {:.2} ", from.x, -from.y);
                        }
                        let _ = write!(target_path, "L {:.2} {:.2} ", to.x, -to.y);
                        *last_target_pos = Some(*to);
                    }
                }
                GCodeCommand::Dwell { pos, duration: _ } => {
                    // Draw a small circle (radius 0.5mm) at dwell position
                    let r = 0.5;
                    let _ = write!(
                        self.cached_g4_path,
                        "M {:.2} {:.2} m -{:.2} 0 a {:.2} {:.2} 0 1 0 {:.2} 0 a {:.2} {:.2} 0 1 0 -{:.2} 0 ",
                        pos.x, -pos.y, r, r, r, r * 2.0, r, r, r * 2.0
                    );
                }
            }
        }

        debug!("Paths rebuilt: {} arcs, {} invalid arcs - total path sizes: toolpath={}, rapid={}, g1={}, g2={}, g3={}, g4={}",
               arc_count, invalid_arc_count,
               self.cached_path.len(), self.cached_rapid_path.len(),
               self.cached_g1_path.len(), self.cached_g2_path.len(),
               self.cached_g3_path.len(), self.cached_g4_path.len());
    }
}
