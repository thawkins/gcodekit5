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
/// line-number `%`, the `$` sigil) are skipped.
///
/// GRBL system commands (`$H`, `$X`, `$$`, `$J=...`) are not G-code: their letters would be
/// returned as words with a `NaN` value. Callers classify such lines with
/// [`system_command`] first.
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

/// A GRBL system command: a line whose code starts with `$`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemCommand {
    /// A known non-motion command (`$`, `$$`, `$#`, `$G`, `$I`, `$N`, `$C`, `$X`, `$H`,
    /// `$HX`, `$SLP`, `$RST=$|#|*`, `$I=...`, `$N0=...`, `$100=80`, ...). Holds the
    /// normalised command text (upper case, no whitespace).
    Known(String),
    /// A `$J=` jog. Holds the G-code payload after `$J=` (upper case, no whitespace).
    Jog(String),
    /// Any other `$` line. Holds the normalised command text.
    Unknown(String),
}

/// `true` for a GRBL setting assignment body such as `100=80.0` (without the `$`).
fn is_setting_assignment(body: &str) -> bool {
    match body.split_once('=') {
        Some((number, value)) => {
            !number.is_empty()
                && number.chars().all(|c| c.is_ascii_digit())
                && number.parse::<u16>().is_ok()
                && value.parse::<f64>().is_ok_and(f64::is_finite)
        }
        None => false,
    }
}

/// Classify a GRBL system (`$`) command line.
///
/// Comments are stripped first. Like GRBL, whitespace is ignored and letters are compared
/// in upper case.
///
/// # Arguments
/// * `line` - One line of G-code or a GRBL command.
///
/// # Returns
/// `None` if the line's code does not start with `$` (it is ordinary G-code), otherwise the
/// classified [`SystemCommand`].
pub fn system_command(line: &str) -> Option<SystemCommand> {
    let code = strip_comments(line);
    if !code.starts_with('$') {
        return None;
    }
    let normalised: String = code
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let body = &normalised[1..];

    if let Some(payload) = body.strip_prefix("J=") {
        return Some(SystemCommand::Jog(payload.to_string()));
    }
    let known = matches!(
        body,
        "" | "$" | "#" | "G" | "I" | "N" | "C" | "X" | "H" | "SLP" | "RST=$" | "RST=#" | "RST=*"
    ) || matches!(body, "HX" | "HY" | "HZ" | "HA" | "HB" | "HC")
        || body.starts_with("I=")
        || body.starts_with("N0=")
        || body.starts_with("N1=")
        || is_setting_assignment(body);
    Some(if known {
        SystemCommand::Known(normalised)
    } else {
        SystemCommand::Unknown(normalised)
    })
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
