//! G-Code Optimizer - Task 63
//!
//! Removes redundant commands and optimizes G-code for efficiency.

/// G-code optimization strategies
#[derive(Debug)]
pub struct GCodeOptimizer;

impl GCodeOptimizer {
    /// Remove consecutive duplicate M5 commands
    pub fn remove_redundant_m5(lines: &[String]) -> Vec<String> {
        let mut result = Vec::new();
        let mut last_was_m5 = false;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("M5") {
                if !last_was_m5 {
                    result.push(line.clone());
                    last_was_m5 = true;
                }
            } else {
                result.push(line.clone());
                last_was_m5 = false;
            }
        }

        result
    }

    /// Remove consecutive duplicate tool selections
    pub fn remove_redundant_tools(lines: &[String]) -> Vec<String> {
        let mut result = Vec::new();
        let mut last_tool: Option<u32> = None;

        for line in lines {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix('T') {
                if let Ok(tool) = rest.parse::<u32>() {
                    if last_tool != Some(tool) {
                        result.push(line.clone());
                        last_tool = Some(tool);
                    }
                    continue;
                }
            }
            result.push(line.clone());
        }

        result
    }

    /// Optimize G-code
    pub fn optimize(lines: &[String]) -> Vec<String> {
        let mut optimized = lines.to_vec();
        optimized = Self::remove_redundant_m5(&optimized);
        optimized = Self::remove_redundant_tools(&optimized);
        optimized
    }
}
