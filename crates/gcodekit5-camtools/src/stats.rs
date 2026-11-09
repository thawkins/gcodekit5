//! Statistics Calculator - Task 62
//!
//! Calculates G-code statistics including distance, time, and command counts.
//!
//! Lines are tokenized with [`crate::comment_processor::words`], so comments are ignored.

use crate::comment_processor::{system_command, words};

/// G-code statistics
#[derive(Debug, Clone, Default)]
pub struct Stats {
    /// Total commands
    pub total_commands: u32,
    /// Rapid movements (G0)
    pub rapid_count: u32,
    /// Linear moves (G1)
    pub linear_count: u32,
    /// Arc commands (G2/G3)
    pub arc_count: u32,
    /// Dwell commands (G4)
    pub dwell_count: u32,
    /// Min X coordinate
    pub min_x: f64,
    /// Max X coordinate
    pub max_x: f64,
    /// Min Y coordinate
    pub min_y: f64,
    /// Max Y coordinate
    pub max_y: f64,
    /// Min Z coordinate
    pub min_z: f64,
    /// Max Z coordinate
    pub max_z: f64,
}

impl Stats {
    /// Get bounding box dimensions
    pub fn bounding_box(&self) -> (f64, f64, f64) {
        (
            self.max_x - self.min_x,
            self.max_y - self.min_y,
            self.max_z - self.min_z,
        )
    }
}

/// Calculates G-code statistics
#[derive(Debug)]
pub struct StatsCalculator;

impl StatsCalculator {
    /// Calculate statistics for a program.
    ///
    /// Comments are ignored; only whole words count (`G0`/`G00` are rapids, `G38.2` is not
    /// counted as a move). GRBL `$` system commands (including `$J=` jogs) are counted in
    /// `total_commands` only; they add no moves and do not affect the bounds.
    ///
    /// # Arguments
    /// * `lines` - Program lines.
    ///
    /// # Returns
    /// Command counts and the coordinate bounds of all X/Y/Z words.
    pub fn calculate(lines: &[String]) -> Stats {
        let mut stats = Stats {
            min_x: f64::MAX,
            min_y: f64::MAX,
            min_z: f64::MAX,
            max_x: f64::MIN,
            max_y: f64::MIN,
            max_z: f64::MIN,
            ..Default::default()
        };

        for line in lines {
            // GRBL `$` commands (`$H`, `$X`, `$J=...`) count as commands but are not program
            // motion: their letters are not G-code words and a jog does not belong in the
            // program's bounds.
            if system_command(line).is_some() {
                stats.total_commands += 1;
                continue;
            }
            let line_words = words(line);
            if line_words.is_empty() {
                continue;
            }

            stats.total_commands += 1;

            for word in line_words {
                if word.value.is_nan() {
                    continue;
                }
                match word.letter {
                    'G' => {
                        // Only integral G numbers are motion codes here (G38.2 is a probe).
                        if word.value.fract() == 0.0 {
                            match word.value as i64 {
                                0 => stats.rapid_count += 1,
                                1 => stats.linear_count += 1,
                                2 | 3 => stats.arc_count += 1,
                                4 => stats.dwell_count += 1,
                                _ => {}
                            }
                        }
                    }
                    'X' => {
                        stats.min_x = stats.min_x.min(word.value);
                        stats.max_x = stats.max_x.max(word.value);
                    }
                    'Y' => {
                        stats.min_y = stats.min_y.min(word.value);
                        stats.max_y = stats.max_y.max(word.value);
                    }
                    'Z' => {
                        stats.min_z = stats.min_z.min(word.value);
                        stats.max_z = stats.max_z.max(word.value);
                    }
                    _ => {}
                }
            }
        }

        stats
    }
}
