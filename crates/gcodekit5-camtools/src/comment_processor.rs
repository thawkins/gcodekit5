//! Comment Processor - Task 54
//!
//! Extracts and processes G-code comments (both parentheses and semicolon styles), and
//! tokenizes the remaining code into words (`G1`, `X10.5`, `M5`, ...).
//!
//! The other line-based utilities ([`crate::validator`], [`crate::stats`],
//! [`crate::optimizer`]) use [`strip_comments`] and [`words`] so that letters inside comments
//! are never mistaken for commands or coordinates, and so `M5` does not match `M50`.
//!
//! Depends only on `std`.

/// Comment processing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentMode {
    /// Remove all comments
    Remove,
    /// Keep comments inline
    Keep,
    /// Extract comments separately
    Extract,
}

/// One G-code word: an address letter (upper-cased) and its numeric value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Word {
    /// Address letter, upper case (`'G'`, `'X'`, ...).
    pub letter: char,
    /// Numeric value of the word.
    pub value: f64,
}

/// Split a line into its code and its comments.
///
/// Parenthesised comments may appear anywhere (several per line); a `;` outside parentheses
/// starts a comment that runs to the end of the line. An unclosed `(` comments out the rest.
///
/// # Arguments
/// * `line` - One line of G-code.
///
/// # Returns
/// `(code, comments)`: the line with all comments removed (whitespace runs collapsed to a
/// single space, trimmed) and the comment texts
/// (trimmed) in order of appearance.
pub fn split_comments(line: &str) -> (String, Vec<String>) {
    let mut code = String::with_capacity(line.len());
    let mut comments = Vec::new();
    let mut current = String::new();
    let mut in_paren = false;
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        if in_paren {
            if ch == ')' {
                comments.push(current.trim().to_string());
                current.clear();
                in_paren = false;
                // Keep words on either side of an inline comment separate.
                code.push(' ');
            } else {
                current.push(ch);
            }
        } else if ch == '(' {
            in_paren = true;
        } else if ch == ';' {
            comments.push(chars.as_str().trim().to_string());
            break;
        } else {
            code.push(ch);
        }
    }
    if in_paren {
        comments.push(current.trim().to_string());
    }
    // Removing an inline comment can leave double spaces; normalise to single spaces.
    let code = code.split_whitespace().collect::<Vec<_>>().join(" ");
    (code, comments)
}

/// Remove all comments from a line.
///
/// # Arguments
/// * `line` - One line of G-code.
///
/// # Returns
/// The code part of the line, trimmed.
pub fn strip_comments(line: &str) -> String {
    split_comments(line).0
}

/// Tokenize the code part of a line into words.
///
/// Comments are stripped first. Whitespace between a letter and its number is allowed
/// (`G 1` is `G1`), as is a leading `+`. Characters that do not form a word (block delete,
/// line-number `%`, GRBL `$` commands) are skipped.
///
/// # Arguments
/// * `line` - One line of G-code.
///
/// # Returns
/// The words in order. A letter whose value cannot be parsed is returned with `value = NaN`
/// so callers can report it.
pub fn words(line: &str) -> Vec<Word> {
    let code = strip_comments(line);
    let chars: Vec<char> = code.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if !ch.is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let letter = ch.to_ascii_uppercase();
        i += 1;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_digit() || matches!(chars[i], '.' | '-' | '+'))
        {
            i += 1;
        }
        let number: String = chars[start..i].iter().collect();
        let value = number.parse::<f64>().unwrap_or(f64::NAN);
        out.push(Word { letter, value });
    }
    out
}

/// Processes comments in G-code
#[derive(Debug)]
pub struct CommentProcessor {
    mode: CommentMode,
}

impl CommentProcessor {
    /// Create a new comment processor
    pub fn new(mode: CommentMode) -> Self {
        Self { mode }
    }

    /// Process a line and extract comment if present.
    ///
    /// # Arguments
    /// * `line` - One line of G-code.
    ///
    /// # Returns
    /// The processed line (comments removed unless the mode is [`CommentMode::Keep`]) and
    /// the comment text (several comments are joined with `"; "`), if any.
    pub fn process_line(&self, line: &str) -> (String, Option<String>) {
        let (code, comments) = split_comments(line);
        let comment = if comments.is_empty() {
            None
        } else {
            Some(comments.join("; "))
        };
        let processed = match self.mode {
            CommentMode::Remove | CommentMode::Extract => code,
            CommentMode::Keep => line.to_string(),
        };
        (processed, comment)
    }
}

impl Default for CommentProcessor {
    fn default() -> Self {
        Self::new(CommentMode::Remove)
    }
}
