//! File Processing Pipeline - Task 93
//! File Statistics - Task 94
//!
//! Task 93: File Processing Pipeline
//! - Create file processor pipeline
//! - Apply preprocessors
//! - Generate processed output
//! - Cache processed results
//!
//! Task 94: File Statistics
//! - Calculate file statistics
//! - Estimate execution time
//! - Determine bounding box
//! - Count commands by type
//! - Calculate total distance

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::gcode::arc::{angular_travel, arc_bounds, center_offset_from_radius};
use crate::gcode::words::{code_number, parse_line, word_value, Word};
use crate::utils::GcodeFileReader;

/// File processing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatistics {
    /// Total number of G-code lines
    pub total_lines: u64,
    /// Number of comment lines
    pub comment_lines: u64,
    /// Number of empty lines
    pub empty_lines: u64,
    /// Number of G0 (rapid) moves
    pub rapid_moves: u64,
    /// Number of G1 (linear) moves
    pub linear_moves: u64,
    /// Number of G2/G3 (arc) moves
    pub arc_moves: u64,
    /// Number of M-codes (miscellaneous)
    pub m_codes: u64,
    /// Total distance traveled (mm; G20 input is converted)
    pub total_distance: f32,
    /// Estimated execution time (seconds)
    pub estimated_time: u64,
    /// Bounding box (min/max coordinates)
    pub bounding_box: BoundingBox,
    /// Command counts by type (uses Cow to avoid allocating static G-code strings)
    pub command_counts: HashMap<Cow<'static, str>, u64>,
    /// Feed rate statistics
    pub feed_rate_stats: FeedRateStats,
    /// Spindle speed statistics
    pub spindle_stats: SpindleStats,
}

/// 3D Bounding box
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Minimum X coordinate
    pub min_x: f32,
    /// Maximum X coordinate
    pub max_x: f32,
    /// Minimum Y coordinate
    pub min_y: f32,
    /// Maximum Y coordinate
    pub max_y: f32,
    /// Minimum Z coordinate
    pub min_z: f32,
    /// Maximum Z coordinate
    pub max_z: f32,
}

impl BoundingBox {
    /// Create a new bounding box
    pub fn new() -> Self {
        Self {
            min_x: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            min_y: f32::INFINITY,
            max_y: f32::NEG_INFINITY,
            min_z: f32::INFINITY,
            max_z: f32::NEG_INFINITY,
        }
    }

    /// Update bounding box with a point
    pub fn update(&mut self, x: f32, y: f32, z: f32) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
        self.min_z = self.min_z.min(z);
        self.max_z = self.max_z.max(z);
    }

    /// Get width (X span)
    pub fn width(&self) -> f32 {
        if self.max_x.is_infinite() {
            0.0
        } else {
            self.max_x - self.min_x
        }
    }

    /// Get height (Y span)
    pub fn height(&self) -> f32 {
        if self.max_y.is_infinite() {
            0.0
        } else {
            self.max_y - self.min_y
        }
    }

    /// Get depth (Z span)
    pub fn depth(&self) -> f32 {
        if self.max_z.is_infinite() {
            0.0
        } else {
            self.max_z - self.min_z
        }
    }

    /// Check if bounding box is valid
    pub fn is_valid(&self) -> bool {
        !self.min_x.is_infinite() && !self.max_x.is_infinite()
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::new()
    }
}

/// Feed rate statistics
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FeedRateStats {
    /// Minimum feed rate
    pub min_feed: f64,
    /// Maximum feed rate
    pub max_feed: f64,
    /// Average feed rate
    pub avg_feed: f64,
    /// Total feed rate changes
    pub changes: u64,
}

impl FeedRateStats {
    /// Create new feed rate statistics
    pub fn new() -> Self {
        Self {
            min_feed: f64::INFINITY,
            max_feed: 0.0,
            avg_feed: 0.0,
            changes: 0,
        }
    }

    /// Update feed rate statistics
    pub fn update(&mut self, feed: f64) {
        if feed > 0.0 {
            if feed < self.min_feed {
                self.min_feed = feed;
            }
            if feed > self.max_feed {
                self.max_feed = feed;
            }
            self.changes += 1;
        }
    }
}

impl Default for FeedRateStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Spindle speed statistics
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpindleStats {
    /// Minimum spindle speed (RPM)
    pub min_speed: f64,
    /// Maximum spindle speed (RPM)
    pub max_speed: f64,
    /// Average spindle speed (RPM)
    pub avg_speed: f64,
    /// Total spindle on time (seconds)
    pub on_time: u64,
    /// Number of spindle on commands
    pub on_count: u64,
}

impl SpindleStats {
    /// Create new spindle statistics
    pub fn new() -> Self {
        Self {
            min_speed: f64::INFINITY,
            max_speed: 0.0,
            avg_speed: 0.0,
            on_time: 0,
            on_count: 0,
        }
    }

    /// Update spindle statistics
    pub fn update(&mut self, speed: f64) {
        if speed > 0.0 {
            if speed < self.min_speed {
                self.min_speed = speed;
            }
            if speed > self.max_speed {
                self.max_speed = speed;
            }
            self.on_count += 1;
        }
    }
}

impl Default for SpindleStats {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStatistics {
    /// Create new file statistics
    pub fn new() -> Self {
        Self {
            total_lines: 0,
            comment_lines: 0,
            empty_lines: 0,
            rapid_moves: 0,
            linear_moves: 0,
            arc_moves: 0,
            m_codes: 0,
            total_distance: 0.0,
            estimated_time: 0,
            bounding_box: BoundingBox::new(),
            command_counts: HashMap::new(),
            feed_rate_stats: FeedRateStats::new(),
            spindle_stats: SpindleStats::new(),
        }
    }

    /// Get total motion commands
    pub fn total_motion_commands(&self) -> u64 {
        self.rapid_moves + self.linear_moves + self.arc_moves
    }

    /// Get formatted execution time
    pub fn formatted_time(&self) -> String {
        let hours = self.estimated_time / 3600;
        let minutes = (self.estimated_time % 3600) / 60;
        let seconds = self.estimated_time % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    /// Get statistics summary
    pub fn summary(&self) -> String {
        format!(
            "Lines: {} | Motion: {} (R:{} L:{} A:{}) | Time: {} | Distance: {:.2}",
            self.total_lines,
            self.total_motion_commands(),
            self.rapid_moves,
            self.linear_moves,
            self.arc_moves,
            self.formatted_time(),
            self.total_distance
        )
    }
}

impl Default for FileStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// File processor result
#[derive(Debug, Clone)]
pub struct ProcessedFile {
    /// Original file path
    pub source_path: PathBuf,
    /// Processed content
    pub content: String,
    /// File statistics
    pub statistics: FileStatistics,
    /// Original line count (all lines, including comments and blanks)
    pub original_lines: u64,
    /// Processed line count
    pub processed_lines: u64,
}

/// File processing pipeline
pub struct FileProcessingPipeline {
    /// Cached processed files
    cache: HashMap<PathBuf, ProcessedFile>,
    /// Cache enabled
    cache_enabled: bool,
}

impl FileProcessingPipeline {
    /// Create new processing pipeline
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            cache_enabled: true,
        }
    }

    /// Enable or disable caching
    pub fn set_cache_enabled(&mut self, enabled: bool) {
        self.cache_enabled = enabled;
        if !enabled {
            self.cache.clear();
        }
    }

    /// Check if file is cached
    pub fn is_cached(&self, path: &Path) -> bool {
        self.cache_enabled && self.cache.contains_key(path)
    }

    /// Get cached file
    pub fn get_cached(&self, path: &Path) -> Option<&ProcessedFile> {
        if self.cache_enabled {
            self.cache.get(path)
        } else {
            None
        }
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Process a G-code file
    pub fn process_file(&mut self, path: impl AsRef<Path>) -> Result<ProcessedFile> {
        let path = path.as_ref();

        // Check cache
        if self.cache_enabled {
            if let Some(cached) = self.cache.get(path) {
                return Ok(cached.clone());
            }
        }

        // Read and process file
        let reader = GcodeFileReader::new(path)?;
        let mut statistics = FileStatistics::new();
        let mut processed_lines = Vec::new();
        let mut tracker = MotionTracker::default();
        let mut feed_sum = 0.0f64;
        let mut speed_sum = 0.0f64;

        reader.read_lines(|line| {
            let trimmed = line.trim();
            statistics.total_lines += 1;

            // Count comments and empty lines
            if trimmed.is_empty() {
                statistics.empty_lines += 1;
                return Ok(());
            }

            let words = parse_line(trimmed);
            if words.is_empty() {
                // Comment-only (or otherwise word-less) line
                statistics.comment_lines += 1;
                return Ok(());
            }

            for word in &words {
                match word.letter {
                    'M' => {
                        statistics.m_codes += 1;
                        let (major, minor) = code_number(word.value);
                        let key = if minor == 0 {
                            format!("M{}", major)
                        } else {
                            format!("M{}.{}", major, minor)
                        };
                        *statistics
                            .command_counts
                            .entry(Cow::Owned(key))
                            .or_insert(0) += 1;
                    }
                    'F' => {
                        statistics.feed_rate_stats.update(word.value);
                        if word.value > 0.0 {
                            feed_sum += word.value;
                        }
                    }
                    'S' => {
                        statistics.spindle_stats.update(word.value);
                        if word.value > 0.0 {
                            speed_sum += word.value;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(motion) = tracker.apply(&words) {
                let key = match motion.kind {
                    MotionKind::Rapid => {
                        statistics.rapid_moves += 1;
                        "G0"
                    }
                    MotionKind::Linear => {
                        statistics.linear_moves += 1;
                        "G1"
                    }
                    MotionKind::ArcCw => {
                        statistics.arc_moves += 1;
                        "G2"
                    }
                    MotionKind::ArcCcw => {
                        statistics.arc_moves += 1;
                        "G3"
                    }
                };
                *statistics
                    .command_counts
                    .entry(Cow::Borrowed(key))
                    .or_insert(0) += 1;

                let (min, max) = motion.bounds;
                statistics
                    .bounding_box
                    .update(min[0] as f32, min[1] as f32, min[2] as f32);
                statistics
                    .bounding_box
                    .update(max[0] as f32, max[1] as f32, max[2] as f32);
                statistics.total_distance += motion.length as f32;
            }

            // Add to processed output
            processed_lines.push(trimmed.to_string());

            Ok(())
        })?;

        if statistics.feed_rate_stats.changes > 0 {
            statistics.feed_rate_stats.avg_feed =
                feed_sum / statistics.feed_rate_stats.changes as f64;
        }
        if statistics.spindle_stats.on_count > 0 {
            statistics.spindle_stats.avg_speed =
                speed_sum / statistics.spindle_stats.on_count as f64;
        }

        // Estimated time: feed moves at their programmed feed rate (mm/min, or
        // inverse time under G93), rapids at DEFAULT_RAPID_RATE_MM_PER_MIN, plus dwells.
        statistics.estimated_time = tracker.time_seconds.max(0.0).round() as u64;

        let processed_content = processed_lines.join("\n");
        let processed_result = ProcessedFile {
            source_path: path.to_path_buf(),
            content: processed_content,
            original_lines: statistics.total_lines,
            statistics,
            processed_lines: processed_lines.len() as u64,
        };

        // Cache result
        if self.cache_enabled {
            self.cache
                .insert(path.to_path_buf(), processed_result.clone());
        }

        Ok(processed_result)
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for FileProcessingPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Assumed rapid (G0) traverse rate used for time estimation, in mm/min.
///
/// The real value is machine specific (`$110`-`$112` on GRBL); this is a
/// conservative default for hobby CNC and laser machines.
pub const DEFAULT_RAPID_RATE_MM_PER_MIN: f64 = 3000.0;

/// Kind of motion produced by a G-code line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionKind {
    Rapid,
    Linear,
    ArcCw,
    ArcCcw,
}

/// A single motion with its geometry, used for statistics
#[derive(Debug, Clone, Copy)]
struct Motion {
    kind: MotionKind,
    /// Path length in mm
    length: f64,
    /// Axis-aligned bounds (min, max) of the motion in mm
    bounds: ([f64; 3], [f64; 3]),
}

/// Minimal modal interpreter for statistics (positions kept in mm)
#[derive(Debug, Clone)]
struct MotionTracker {
    position: [f64; 3],
    motion: Option<MotionKind>,
    /// Arc plane axes (first, second, linear) with 0 = X, 1 = Y, 2 = Z
    plane: (usize, usize, usize),
    absolute: bool,
    inches: bool,
    inverse_time: bool,
    /// Current feed rate in mm/min
    feed_mm_per_min: f64,
    /// Accumulated execution time in seconds
    time_seconds: f64,
}

impl Default for MotionTracker {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            motion: None,
            plane: (0, 1, 2),
            absolute: true,
            inches: false,
            inverse_time: false,
            feed_mm_per_min: 0.0,
            time_seconds: 0.0,
        }
    }
}

impl MotionTracker {
    /// Apply one line of words; returns the motion it produced, if any.
    fn apply(&mut self, words: &[Word]) -> Option<Motion> {
        let mut dwell = false;
        let mut home_move = false;
        let mut set_position = false;
        let mut axes_not_motion = false;

        for word in words.iter().filter(|w| w.letter == 'G') {
            match code_number(word.value) {
                (0, 0) => self.motion = Some(MotionKind::Rapid),
                (1, 0) => self.motion = Some(MotionKind::Linear),
                (2, 0) => self.motion = Some(MotionKind::ArcCw),
                (3, 0) => self.motion = Some(MotionKind::ArcCcw),
                (4, 0) => dwell = true,
                (10, _) | (28, 1) | (30, 1) | (43, 1) => axes_not_motion = true,
                (17, 0) => self.plane = (0, 1, 2),
                (18, 0) => self.plane = (2, 0, 1),
                (19, 0) => self.plane = (1, 2, 0),
                (20, 0) => self.inches = true,
                (21, 0) => self.inches = false,
                (28, 0) | (30, 0) => home_move = true,
                (80, 0) => self.motion = None,
                (90, 0) => self.absolute = true,
                (91, 0) => self.absolute = false,
                (92, 0) => set_position = true,
                (93, 0) => self.inverse_time = true,
                (94, 0) | (95, 0) => self.inverse_time = false,
                _ => {}
            }
        }

        let scale = if self.inches { 25.4 } else { 1.0 };
        let feed_word = word_value(words, 'F');
        if let Some(f) = feed_word {
            if !self.inverse_time {
                self.feed_mm_per_min = f * scale;
            }
        }

        if dwell {
            if let Some(p) = word_value(words, 'P') {
                self.time_seconds += p.max(0.0);
            }
            return None;
        }

        let axis_words = [
            word_value(words, 'X'),
            word_value(words, 'Y'),
            word_value(words, 'Z'),
        ];
        if axes_not_motion || axis_words.iter().all(Option::is_none) {
            return None;
        }

        if set_position {
            for (axis, value) in axis_words.iter().enumerate() {
                if let Some(v) = value {
                    self.position[axis] = v * scale;
                }
            }
            return None;
        }

        let mut target = self.position;
        for (axis, value) in axis_words.iter().enumerate() {
            if let Some(v) = value {
                target[axis] = if self.absolute {
                    v * scale
                } else {
                    target[axis] + v * scale
                };
            }
        }

        let kind = if home_move {
            MotionKind::Rapid
        } else {
            self.motion?
        };

        let start = self.position;
        let mut min = [0.0; 3];
        let mut max = [0.0; 3];
        for axis in 0..3 {
            min[axis] = start[axis].min(target[axis]);
            max[axis] = start[axis].max(target[axis]);
        }
        let chord = ((target[0] - start[0]).powi(2)
            + (target[1] - start[1]).powi(2)
            + (target[2] - start[2]).powi(2))
        .sqrt();
        let mut length = chord;

        if matches!(kind, MotionKind::ArcCw | MotionKind::ArcCcw) {
            let clockwise = kind == MotionKind::ArcCw;
            let (a0, a1, lin) = self.plane;
            let offset_word = |axis: usize| -> Option<f64> {
                word_value(words, ['I', 'J', 'K'][axis]).map(|v| v * scale)
            };
            let (dx, dy) = (target[a0] - start[a0], target[a1] - start[a1]);
            let offsets = match (offset_word(a0), offset_word(a1)) {
                (None, None) => word_value(words, 'R')
                    .and_then(|r| center_offset_from_radius(dx, dy, r * scale, clockwise)),
                (i, j) => Some((i.unwrap_or(0.0), j.unwrap_or(0.0))),
            };
            if let Some((oi, oj)) = offsets {
                let radius = (oi * oi + oj * oj).sqrt();
                if radius > 1e-9 {
                    let sweep = angular_travel((-oi, -oj), (dx - oi, dy - oj), clockwise);
                    let planar = radius * sweep.abs();
                    let helical = target[lin] - start[lin];
                    length = (planar * planar + helical * helical).sqrt();
                    let start_angle = (-oj).atan2(-oi);
                    let center = (start[a0] + oi, start[a1] + oj);
                    let (bx0, by0, bx1, by1) = arc_bounds(center, radius, start_angle, sweep);
                    min[a0] = min[a0].min(bx0);
                    min[a1] = min[a1].min(by0);
                    max[a0] = max[a0].max(bx1);
                    max[a1] = max[a1].max(by1);
                }
            }
        }

        self.time_seconds += match kind {
            MotionKind::Rapid => length / DEFAULT_RAPID_RATE_MM_PER_MIN * 60.0,
            _ if self.inverse_time => match feed_word {
                // G93: F is the inverse of the move duration in minutes
                Some(f) if f > 0.0 => 60.0 / f,
                _ => 0.0,
            },
            _ if self.feed_mm_per_min > 0.0 => length / self.feed_mm_per_min * 60.0,
            _ => 0.0,
        };

        self.position = target;
        Some(Motion {
            kind,
            length,
            bounds: (min, max),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_new() {
        let bb = BoundingBox::new();
        assert!(!bb.is_valid());
    }

    #[test]
    fn test_bounding_box_update() {
        let mut bb = BoundingBox::new();
        bb.update(0.0, 0.0, 0.0);
        bb.update(10.0, 20.0, 5.0);

        assert_eq!(bb.min_x, 0.0);
        assert_eq!(bb.max_x, 10.0);
        assert_eq!(bb.width(), 10.0);
        assert_eq!(bb.height(), 20.0);
        assert_eq!(bb.depth(), 5.0);
    }

    #[test]
    fn test_feed_rate_stats() {
        let mut stats = FeedRateStats::new();
        stats.update(100.0);
        stats.update(200.0);
        stats.update(150.0);

        assert_eq!(stats.min_feed, 100.0);
        assert_eq!(stats.max_feed, 200.0);
        assert_eq!(stats.changes, 3);
    }

    #[test]
    fn test_spindle_stats() {
        let mut stats = SpindleStats::new();
        stats.update(1000.0);
        stats.update(2000.0);
        stats.update(500.0);

        assert_eq!(stats.min_speed, 500.0);
        assert_eq!(stats.max_speed, 2000.0);
        assert_eq!(stats.on_count, 3);
    }

    #[test]
    fn test_file_statistics_new() {
        let stats = FileStatistics::new();
        assert_eq!(stats.total_lines, 0);
        assert_eq!(stats.total_motion_commands(), 0);
    }

    #[test]
    fn test_file_statistics_formatted_time() {
        let mut stats = FileStatistics::new();
        stats.estimated_time = 3661;
        assert_eq!(stats.formatted_time(), "1h 1m 1s");

        stats.estimated_time = 125;
        assert_eq!(stats.formatted_time(), "2m 5s");

        stats.estimated_time = 45;
        assert_eq!(stats.formatted_time(), "45s");
    }

    #[test]
    fn test_processing_pipeline_new() {
        let pipeline = FileProcessingPipeline::new();
        assert!(pipeline.cache_enabled);
        assert_eq!(pipeline.cache_size(), 0);
    }

    #[test]
    fn test_processing_pipeline_cache() {
        let mut pipeline = FileProcessingPipeline::new();
        let test_path = PathBuf::from("/test/file.nc");

        assert!(!pipeline.is_cached(&test_path));

        pipeline.set_cache_enabled(false);
        assert!(!pipeline.is_cached(&test_path));

        pipeline.set_cache_enabled(true);
        assert!(!pipeline.is_cached(&test_path));
    }

    #[test]
    fn test_processed_file() {
        let processed = ProcessedFile {
            source_path: PathBuf::from("test.nc"),
            content: "G0 X10\nG1 Y20".to_string(),
            statistics: FileStatistics::new(),
            original_lines: 100,
            processed_lines: 2,
        };

        assert_eq!(processed.original_lines, 100);
        assert_eq!(processed.processed_lines, 2);
    }

    #[test]
    fn test_file_statistics_summary() {
        let mut stats = FileStatistics::new();
        stats.total_lines = 100;
        stats.rapid_moves = 5;
        stats.linear_moves = 20;
        stats.arc_moves = 3;
        stats.total_distance = 150.5;
        stats.estimated_time = 300;

        let summary = stats.summary();
        assert!(summary.contains("100"));
        assert!(summary.contains("5m 0s"));
    }
}
