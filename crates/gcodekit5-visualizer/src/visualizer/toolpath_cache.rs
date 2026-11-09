//! # Toolpath Command Cache
//!
//! Holds the parsed visualizer commands together with the hash of the G-code
//! they were parsed from, so an unchanged program is not parsed again.
//! Renderers build their own vertex data from [`ToolpathCache::commands`].

use super::visualizer::GCodeCommand;

/// Parsed commands keyed by the hash of their source G-code.
#[derive(Debug, Default, Clone)]
pub struct ToolpathCache {
    content_hash: u64,
    commands: Vec<GCodeCommand>,
}

impl ToolpathCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether G-code with hash `new_hash` must be parsed again.
    ///
    /// # Arguments
    /// * `new_hash` - Hash of the candidate G-code text.
    ///
    /// # Returns
    /// `true` when the hash differs from the cached one or the cache is empty.
    pub fn needs_update(&self, new_hash: u64) -> bool {
        self.content_hash != new_hash || self.commands.is_empty()
    }

    /// Replace the cached commands.
    ///
    /// # Arguments
    /// * `new_hash` - Hash of the G-code the commands were parsed from.
    /// * `commands` - Parsed commands.
    pub fn update(&mut self, new_hash: u64, commands: Vec<GCodeCommand>) {
        self.content_hash = new_hash;
        self.commands = commands;
    }

    /// Cached commands.
    pub fn commands(&self) -> &[GCodeCommand] {
        &self.commands
    }

    /// Number of cached commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether the cache holds no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
