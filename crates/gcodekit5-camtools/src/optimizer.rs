//! G-Code Optimizer - Task 63
//!
//! Removes redundant commands and optimizes G-code for efficiency.
//!
//! Lines are tokenized with [`crate::comment_processor::words`], so comments are ignored and
//! `M5` never matches `M50` (or `M5` inside a comment).

use crate::comment_processor::{system_command, words, Word};

/// The value if the line's code consists of exactly one word `letter` with an integral value.
fn single_word(line_words: &[Word], letter: char) -> Option<i64> {
    match line_words {
        [w] if w.letter == letter && w.value.fract() == 0.0 => Some(w.value as i64),
        _ => None,
    }
}

/// G-code optimization strategies
#[derive(Debug)]
pub struct GCodeOptimizer;

impl GCodeOptimizer {
    /// Remove consecutive duplicate `M5` lines.
    ///
    /// Only lines whose code is exactly `M5` (or `M05`) are candidates; lines that combine
    /// `M5` with other words are kept. Blank and comment-only lines are kept and do not
    /// break a run of `M5` lines; GRBL `$` commands are kept and do break it.
    ///
    /// # Arguments
    /// * `lines` - Program lines.
    ///
    /// # Returns
    /// The program without the redundant `M5` lines.
    pub fn remove_redundant_m5(lines: &[String]) -> Vec<String> {
        let mut result = Vec::new();
        let mut last_was_m5 = false;

        for line in lines {
            // A GRBL `$` command (`$H`, `$X`, `$J=...`) is kept and ends a run of `M5` lines.
            if system_command(line).is_some() {
                result.push(line.clone());
                last_was_m5 = false;
                continue;
            }
            let line_words = words(line);
            if line_words.is_empty() {
                result.push(line.clone());
                continue;
            }
            if single_word(&line_words, 'M') == Some(5) {
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

    /// Remove repeated selections of the tool that is already selected.
    ///
    /// Only lines whose code is exactly one `T` word are candidates.
    ///
    /// # Arguments
    /// * `lines` - Program lines.
    ///
    /// # Returns
    /// The program without the redundant tool-select lines.
    pub fn remove_redundant_tools(lines: &[String]) -> Vec<String> {
        let mut result = Vec::new();
        let mut last_tool: Option<i64> = None;

        for line in lines {
            // GRBL `$` commands are never tool selections, whatever letters they contain.
            if system_command(line).is_some() {
                result.push(line.clone());
                continue;
            }
            if let Some(tool) = single_word(&words(line), 'T') {
                if last_tool != Some(tool) {
                    result.push(line.clone());
                    last_tool = Some(tool);
                }
                continue;
            }
            result.push(line.clone());
        }

        result
    }

    /// Apply all optimizations.
    ///
    /// # Arguments
    /// * `lines` - Program lines.
    ///
    /// # Returns
    /// The optimized program.
    pub fn optimize(lines: &[String]) -> Vec<String> {
        let mut optimized = lines.to_vec();
        optimized = Self::remove_redundant_m5(&optimized);
        optimized = Self::remove_redundant_tools(&optimized);
        optimized
    }
}
