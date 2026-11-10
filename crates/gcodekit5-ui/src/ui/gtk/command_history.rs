//! # Command History Panel
//!
//! Displays a scrollable list of previously sent G-code commands
//! with timestamps and status indicators. Supports command re-send
//! and history search.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

/// Most commands kept; older ones are dropped.
pub const MAX_HISTORY: usize = 250;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistory {
    commands: VecDeque<String>,
    #[serde(skip)]
    current_index: Option<usize>,
    #[serde(skip)]
    temp_input: String,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            commands: VecDeque::new(),
            current_index: None,
            temp_input: String::new(),
        }
    }

    /// Number of commands in the history.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The command at `index` (0 = oldest kept).
    pub fn get(&self, index: usize) -> Option<&str> {
        self.commands.get(index).map(String::as_str)
    }

    pub fn add(&mut self, command: String) {
        if command.trim().is_empty() {
            return;
        }

        // Don't add duplicates of the most recent command
        if let Some(last) = self.commands.back()
            && last == &command
        {
            return;
        }

        self.commands.push_back(command);

        // Trim to max size
        while self.commands.len() > MAX_HISTORY {
            self.commands.pop_front();
        }

        // Reset navigation
        self.current_index = None;
        self.temp_input.clear();
    }

    pub fn previous(&mut self, current_input: &str) -> Option<String> {
        if self.commands.is_empty() {
            return None;
        }

        match self.current_index {
            None => {
                // First time navigating up, save current input
                self.temp_input = current_input.to_string();
                self.current_index = Some(self.commands.len() - 1);
                Some(self.commands[self.commands.len() - 1].clone())
            }
            Some(idx) => {
                if idx > 0 {
                    self.current_index = Some(idx - 1);
                    Some(self.commands[idx - 1].clone())
                } else {
                    // Already at oldest, stay there
                    Some(self.commands[idx].clone())
                }
            }
        }
    }

    // Custom `new()` does not match Default semantics (requires controller ref).
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<String> {
        match self.current_index {
            None => None,
            Some(idx) => {
                if idx < self.commands.len() - 1 {
                    self.current_index = Some(idx + 1);
                    Some(self.commands[idx + 1].clone())
                } else {
                    // Back to current input
                    self.current_index = None;
                    Some(self.temp_input.clone())
                }
            }
        }
    }

    pub fn reset_navigation(&mut self) {
        self.current_index = None;
        self.temp_input.clear();
    }

    fn get_config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gcodekit5");

        fs::create_dir_all(&config_dir).ok();
        config_dir.join("command_history.json")
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();

        if let Ok(contents) = fs::read_to_string(&path)
            && let Ok(history) = serde_json::from_str::<Self>(&contents)
        {
            return history;
        }

        Self::new()
    }

    pub fn save(&self) {
        let path = Self::get_config_path();

        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, json);
        }
    }
}
