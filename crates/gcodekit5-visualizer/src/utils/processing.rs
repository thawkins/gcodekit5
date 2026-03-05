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

use crate::utils::GcodeFileReader;
use gcodekit5_core::Position;

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
    /// Total distance traveled (in current units)
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
    /// Original line count
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
        let mut current_position = Position::default();
        let mut last_position = Position::default();

        reader.read_lines(|line| {
            let trimmed = line.trim();
            statistics.total_lines += 1;

            // Count comments and empty lines
            if trimmed.is_empty() {
                statistics.empty_lines += 1;
                return Ok(());
            }

            if trimmed.starts_with(';') || trimmed.starts_with('(') {
                statistics.comment_lines += 1;
                return Ok(());
            }

            // Parse and count commands
            let upper = trimmed.to_uppercase();

            // Count motion commands
            if upper.contains("G00") || upper.starts_with("G0 ") {
                statistics.rapid_moves += 1;
                *statistics
                    .command_counts
                    .entry(Cow::Borrowed("G0"))
                    .or_insert(0) += 1;
            } else if upper.contains("G01") || upper.starts_with("G1 ") {
                statistics.linear_moves += 1;
                *statistics
                    .command_counts
                    .entry(Cow::Borrowed("G1"))
                    .or_insert(0) += 1;
            } else if upper.contains("G02") || upper.contains("G2 ") {
                statistics.arc_moves += 1;
                *statistics
                    .command_counts
                    .entry(Cow::Borrowed("G2"))
                    .or_insert(0) += 1;
            } else if upper.contains("G03") || upper.contains("G3 ") {
                statistics.arc_moves += 1;
                *statistics
                    .command_counts
                    .entry(Cow::Borrowed("G3"))
                    .or_insert(0) += 1;
            }

            // Count M-codes
            if upper.contains('M') {
                statistics.m_codes += 1;
                if let Some(m_pos) = upper.find('M') {
                    if m_pos + 3 <= upper.len() {
                        let m_code = &upper[m_pos..m_pos + 3];
                        *statistics
                            .command_counts
                            .entry(Cow::Owned(m_code.to_string()))
                            .or_insert(0) += 1;
                    }
                }
            }

            // Extract feed rate
            if let Some(f_pos) = upper.find('F') {
                if let Ok(feed_str) = upper[f_pos + 1..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<f64>()
                {
                    statistics.feed_rate_stats.update(feed_str);
                }
            }

            // Extract spindle speed
            if let Some(s_pos) = upper.find('S') {
                if let Ok(speed_str) = upper[s_pos + 1..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<f64>()
                {
                    statistics.spindle_stats.update(speed_str);
                }
            }

            // Extract coordinates
            if let Some(x_pos) = upper.find('X') {
                if let Ok(x) = extract_number(&upper, x_pos) {
                    current_position.x = x;
                }
            }
            if let Some(y_pos) = upper.find('Y') {
                if let Ok(y) = extract_number(&upper, y_pos) {
                    current_position.y = y;
                }
            }
            if let Some(z_pos) = upper.find('Z') {
                if let Ok(z) = extract_number(&upper, z_pos) {
                    current_position.z = z;
                }
            }

            // Update bounding box
            statistics.bounding_box.update(
                current_position.x,
                current_position.y,
                current_position.z,
            );

            // Calculate distance
            let dx = current_position.x - last_position.x;
            let dy = current_position.y - last_position.y;
            let dz = current_position.z - last_position.z;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            statistics.total_distance += distance;

            last_position = current_position;

            // Add to processed output
            processed_lines.push(trimmed.to_string());

            Ok(())
        })?;

        // Estimate time (simplified: assume 60 mm/min for rapid, 20 mm/min for feed)
        // This is a rough estimate - actual time depends on many factors
        let rapid_distance = (statistics.rapid_moves as f32) * 10.0; // assume 10mm per rapid move avg
        let feed_distance = statistics.total_distance - rapid_distance;
        let rapid_time = (rapid_distance / 60.0) * 60.0; // 60 mm/min -> seconds
        let feed_time = if statistics.feed_rate_stats.max_feed > 0.0 {
            (feed_distance / (statistics.feed_rate_stats.max_feed as f32)) * 60.0
        } else {
            0.0
        };
        statistics.estimated_time = ((rapid_time + feed_time) as f64) as u64;

        let processed_content = processed_lines.join("\n");
        let processed_result = ProcessedFile {
            source_path: path.to_path_buf(),
            content: processed_content,
            statistics,
            original_lines: reader.file_size(),
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

/// Extract numeric value from G-code
fn extract_number(line: &str, pos: usize) -> Result<f32> {
    let end = pos
        + 1
        + line[pos + 1..]
            .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
            .unwrap_or(line.len() - pos - 1);

    let num_str = &line[pos + 1..end].trim();
    num_str.parse::<f32>().map_err(|e| anyhow::anyhow!(e))
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
