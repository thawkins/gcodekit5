//! # G-code word tokenizer
//!
//! Shared, allocation-light tokenizer used by the modal-state parser, the
//! toolpath visualizer and the file statistics pipeline so that every consumer
//! interprets a line the same way.
//!
//! A line is first stripped of comments (`( ... )` and `; ...`), then split into
//! words: a letter (normalised to upper case) followed by an optional sign and a
//! decimal number, e.g. `g1x-10.5 F1200` -> `G1`, `X-10.5`, `F1200`. Whitespace
//! between a letter and its number is tolerated, and several G words may appear
//! on one line (`G90 G1 X10`). Words without a valid number are skipped.
//!
//! No dependencies beyond `std`.

use std::borrow::Cow;

/// A single G-code word such as `X10.5` (`letter = 'X'`, `value = 10.5`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Word {
    /// Upper-case word letter.
    pub letter: char,
    /// Numeric value of the word.
    pub value: f64,
}

/// Remove parenthesised and semicolon comments from a G-code line.
///
/// Text inside `( ... )` is replaced by a single space so that adjacent words
/// stay separated (`G1(c)X10` -> `G1 X10`). Everything after a `;` outside of a
/// parenthesised comment is dropped, as is everything after an unclosed `(`.
///
/// # Arguments
/// * `line` - Raw G-code line.
///
/// # Returns
/// The line without comments (borrowed when no comment is present).
pub fn strip_comments(line: &str) -> Cow<'_, str> {
    if !line.contains(['(', ';']) {
        return Cow::Borrowed(line);
    }

    let mut out = String::with_capacity(line.len());
    let mut in_paren = false;
    for ch in line.chars() {
        if in_paren {
            if ch == ')' {
                in_paren = false;
                out.push(' ');
            }
            continue;
        }
        match ch {
            '(' => in_paren = true,
            ';' => break,
            _ => out.push(ch),
        }
    }
    Cow::Owned(out)
}

/// Split a comment-free G-code line into words.
///
/// # Arguments
/// * `code` - Line content with comments already removed (see [`strip_comments`]).
///
/// # Returns
/// The words in the order they appear on the line.
pub fn parse_words(code: &str) -> Vec<Word> {
    let bytes = code.as_bytes();
    let mut words = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let c = bytes[i];
        if !c.is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let letter = c.to_ascii_uppercase() as char;
        i += 1;

        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }

        let start = i;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let mut seen_dot = false;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_digit() {
                i += 1;
            } else if b == b'.' && !seen_dot {
                seen_dot = true;
                i += 1;
            } else {
                break;
            }
        }

        // All consumed bytes are ASCII, so `start..i` is a valid char boundary range.
        if let Ok(value) = code[start..i].parse::<f64>()
            && value.is_finite()
        {
            words.push(Word { letter, value });
        }
    }

    words
}

/// Strip comments and tokenize a raw G-code line in one step.
///
/// # Arguments
/// * `line` - Raw G-code line, possibly containing comments.
///
/// # Returns
/// The words found on the line.
pub fn parse_line(line: &str) -> Vec<Word> {
    parse_words(&strip_comments(line))
}

/// Get the value of the last word with the given letter.
///
/// # Arguments
/// * `words` - Words of a line.
/// * `letter` - Upper-case letter to search for.
///
/// # Returns
/// The value, or `None` when the letter is absent.
pub fn word_value(words: &[Word], letter: char) -> Option<f64> {
    words
        .iter()
        .rev()
        .find(|w| w.letter == letter)
        .map(|w| w.value)
}

/// Split a G/M code value into its major number and single-digit sub-code.
///
/// `61.1` -> `(61, 1)`, `38.2` -> `(38, 2)`, `1` -> `(1, 0)`.
///
/// # Arguments
/// * `value` - The numeric value of a G or M word.
///
/// # Returns
/// `(major, minor)`; negative values yield `(u32::MAX, 0)` so they never match a code.
pub fn code_number(value: f64) -> (u32, u32) {
    if value < 0.0 || !value.is_finite() {
        return (u32::MAX, 0);
    }
    let rounded = (value * 10.0).round() as u64;
    ((rounded / 10) as u32, (rounded % 10) as u32)
}
