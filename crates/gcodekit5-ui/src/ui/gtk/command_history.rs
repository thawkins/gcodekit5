use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY: usize = 250;

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

    pub fn add(&mut self, command: String) {
        if command.trim().is_empty() {
            return;
        }

        // Don't add duplicates of the most recent command
        if let Some(last) = self.commands.back() {
            if last == &command {
                return;
            }
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
        
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(history) = serde_json::from_str::<Self>(&contents) {
                return history;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_command() {
        let mut history = CommandHistory::new();
        history.add("G0 X10".to_string());
        history.add("G0 Y20".to_string());
        
        assert_eq!(history.commands.len(), 2);
    }

    #[test]
    fn test_no_duplicate_consecutive() {
        let mut history = CommandHistory::new();
        history.add("G0 X10".to_string());
        history.add("G0 X10".to_string());
        
        assert_eq!(history.commands.len(), 1);
    }

    #[test]
    fn test_navigate_up() {
        let mut history = CommandHistory::new();
        history.add("cmd1".to_string());
        history.add("cmd2".to_string());
        history.add("cmd3".to_string());
        
        let prev = history.previous("current");
        assert_eq!(prev, Some("cmd3".to_string()));
        
        let prev = history.previous("current");
        assert_eq!(prev, Some("cmd2".to_string()));
    }

    #[test]
    fn test_navigate_down() {
        let mut history = CommandHistory::new();
        history.add("cmd1".to_string());
        history.add("cmd2".to_string());
        
        history.previous("current");
        history.previous("current");
        
        let next = history.next();
        assert_eq!(next, Some("cmd2".to_string()));
        
        let next = history.next();
        assert_eq!(next, Some("current".to_string()));
    }

    #[test]
    fn test_max_history() {
        let mut history = CommandHistory::new();
        
        for i in 0..300 {
            history.add(format!("cmd{}", i));
        }
        
        assert_eq!(history.commands.len(), MAX_HISTORY);
        assert_eq!(history.commands[0], "cmd50");
    }
}
