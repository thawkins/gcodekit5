//! # GRBL streaming helpers
//!
//! Pure, GTK-free logic used by the Machine Control panel to talk to a GRBL controller:
//!
//! * [`CharCountWindow`] implements GRBL's recommended *character-counting* streaming
//!   protocol: a line is only sent when it fits, together with every line sent but not
//!   yet acknowledged, into the controller's 128-byte serial RX buffer.
//! * [`ResponseLineSplitter`] turns raw serial reads into complete response lines,
//!   carrying partial lines (and partial UTF-8 sequences) over to the next read.
//! * [`classify_response`] sorts a response line into acks, errors, alarms, status
//!   reports and other messages.
//! * Small helpers for probe reports, axis detection, jog step presets and jog feed
//!   validation.
//!
//! Depends only on `gcodekit5_core::units` and the GRBL status parser types.

use std::collections::VecDeque;

use gcodekit5_communication::ConnectionDriver;
use gcodekit5_communication::firmware::grbl::status_parser::MachinePosition;
use gcodekit5_core::units::{
    FeedRateUnits, MeasurementSystem, format_feed_rate, get_unit_label, parse_feed_rate,
};

/// Usable size of GRBL's serial RX buffer in bytes (the buffer is 128 bytes; one slot is
/// kept free by the ring buffer, so at most 127 bytes may be outstanding).
pub const GRBL_RX_BUFFER_BYTES: usize = 127;

/// Millimetres per inch.
const MM_PER_INCH: f32 = 25.4;

/// Returns the number of RX-buffer bytes a G-code line occupies once sent
/// (its bytes plus the terminating newline).
///
/// # Arguments
/// * `line` - The line without its newline.
///
/// # Returns
/// `line.len() + 1`.
pub fn line_cost(line: &str) -> usize {
    line.len() + 1
}

/// Tracks the bytes of lines that were sent to GRBL but not yet acknowledged.
///
/// Every `ok` or `error:N` reply acknowledges exactly one line, in order, so the
/// outstanding line sizes are kept in a FIFO.
#[derive(Debug, Clone)]
pub struct CharCountWindow {
    capacity: usize,
    in_flight: VecDeque<usize>,
    in_flight_bytes: usize,
}

impl Default for CharCountWindow {
    fn default() -> Self {
        Self::new(GRBL_RX_BUFFER_BYTES)
    }
}

impl CharCountWindow {
    /// Creates an empty window.
    ///
    /// # Arguments
    /// * `capacity` - Maximum outstanding bytes (normally [`GRBL_RX_BUFFER_BYTES`]).
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            in_flight: VecDeque::new(),
            in_flight_bytes: 0,
        }
    }

    /// Returns whether a line can be sent now without overflowing the RX buffer.
    ///
    /// # Arguments
    /// * `line` - The line without its newline.
    ///
    /// # Returns
    /// True if `in_flight_bytes + line.len() + 1 <= capacity`.
    pub fn can_send(&self, line: &str) -> bool {
        self.in_flight_bytes + line_cost(line) <= self.capacity
    }

    /// Returns whether a line could ever be sent (fits into an empty buffer).
    ///
    /// # Arguments
    /// * `line` - The line without its newline.
    pub fn fits_at_all(&self, line: &str) -> bool {
        line_cost(line) <= self.capacity
    }

    /// Records that a line was written to the controller.
    ///
    /// # Arguments
    /// * `line` - The line without its newline.
    pub fn on_sent(&mut self, line: &str) {
        let cost = line_cost(line);
        self.in_flight.push_back(cost);
        self.in_flight_bytes += cost;
    }

    /// Records one `ok` / `error:N` reply, which frees the oldest outstanding line.
    ///
    /// # Returns
    /// True if an outstanding line was acknowledged, false for an unexpected reply.
    pub fn on_ack(&mut self) -> bool {
        match self.in_flight.pop_front() {
            Some(cost) => {
                self.in_flight_bytes -= cost;
                true
            }
            None => false,
        }
    }

    /// Bytes currently outstanding in the controller's RX buffer.
    pub fn in_flight_bytes(&self) -> usize {
        self.in_flight_bytes
    }

    /// Number of lines sent but not yet acknowledged.
    pub fn in_flight_lines(&self) -> usize {
        self.in_flight.len()
    }

    /// True when every sent line has been acknowledged.
    pub fn is_empty(&self) -> bool {
        self.in_flight.is_empty()
    }
}

/// Splits raw serial data into complete, trimmed response lines.
///
/// Lines may end with `\n`, `\r` or `\r\n`. Incomplete data is carried over to the next
/// call as bytes, so a UTF-8 sequence split across two reads is decoded correctly.
#[derive(Debug, Default, Clone)]
pub struct ResponseLineSplitter {
    carry: Vec<u8>,
}

impl ResponseLineSplitter {
    /// Creates an empty splitter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds received bytes and returns every line completed by them.
    ///
    /// # Arguments
    /// * `data` - Bytes as returned by one serial read.
    ///
    /// # Returns
    /// Complete lines, trimmed, with empty lines dropped.
    pub fn push(&mut self, data: &[u8]) -> Vec<String> {
        self.carry.extend_from_slice(data);
        let mut lines = Vec::new();
        while let Some(pos) = self.carry.iter().position(|&b| b == b'\n' || b == b'\r') {
            let raw: Vec<u8> = self.carry.drain(..=pos).collect();
            let text = String::from_utf8_lossy(&raw[..raw.len() - 1]);
            let line = text.trim();
            if !line.is_empty() {
                lines.push(line.to_string());
            }
        }
        lines
    }

    /// Returns the bytes of the incomplete line still waiting for its terminator.
    pub fn pending(&self) -> &[u8] {
        &self.carry
    }

    /// Discards any incomplete data.
    pub fn clear(&mut self) {
        self.carry.clear();
    }
}

/// Classification of one GRBL response line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrblResponseLine {
    /// `ok`: one line was accepted.
    Ok,
    /// `error:N`: one line was rejected; holds the code text after `error:`.
    Error(String),
    /// `ALARM:N`: the controller entered an alarm; holds the code text.
    Alarm(String),
    /// `<...>` real-time status report (the whole line).
    Status(String),
    /// Start-up banner (`Grbl 1.1h ['$' for help]`): the controller was reset.
    Reset(String),
    /// Anything else (`[MSG:...]`, `[PRB:...]`, `$n=v`, ...).
    Other(String),
}

/// Classifies a single, trimmed GRBL response line.
///
/// Only an exact `ok` counts as an acknowledgement, so text that merely contains
/// "ok" (for example inside a `[MSG:...]`) is never mistaken for one.
///
/// # Arguments
/// * `line` - One complete response line.
///
/// # Returns
/// The line's [`GrblResponseLine`] kind.
pub fn classify_response(line: &str) -> GrblResponseLine {
    let line = line.trim();
    if line.eq_ignore_ascii_case("ok") {
        return GrblResponseLine::Ok;
    }
    let lower = line.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("error:") {
        return GrblResponseLine::Error(rest.trim().to_string());
    }
    if let Some(rest) = lower.strip_prefix("alarm:") {
        return GrblResponseLine::Alarm(rest.trim().to_string());
    }
    if line.starts_with('<') && line.ends_with('>') {
        return GrblResponseLine::Status(line.to_string());
    }
    if lower.starts_with("grbl ") || lower.starts_with("grblhal ") {
        return GrblResponseLine::Reset(line.to_string());
    }
    GrblResponseLine::Other(line.to_string())
}

/// Parses the success flag of a GRBL probe report such as `[PRB:0.000,0.000,-1.234:1]`.
///
/// # Arguments
/// * `line` - One response line.
///
/// # Returns
/// `Some(true)` for a successful probe, `Some(false)` for a failed one, `None` if the
/// line is not a probe report or its success flag is not an explicit `0` or `1`.
pub fn parse_probe_report(line: &str) -> Option<bool> {
    // The shared parser treats a missing flag as success; a quick probe must
    // only complete on an explicit `:1`.
    let inner = line.trim().strip_prefix("[PRB:")?.strip_suffix(']')?;
    let (_, flag) = inner.rsplit_once(':')?;
    if !matches!(flag.trim(), "0" | "1") {
        return None;
    }
    gcodekit5_communication::firmware::grbl::status_parser::parse_probe_report(line)
        .map(|report| report.success)
}

/// Returns how many axes a status report's position carries (3 to 6).
///
/// # Arguments
/// * `pos` - A parsed `MPos`/`WPos` position.
pub fn reported_axis_count(pos: &MachinePosition) -> u8 {
    if pos.c.is_some() {
        6
    } else if pos.b.is_some() {
        5
    } else if pos.a.is_some() {
        4
    } else {
        3
    }
}

/// One jog step preset.
#[derive(Debug, Clone, PartialEq)]
pub struct JogStepPreset {
    /// Step size in millimetres (internal unit).
    pub mm: f32,
    /// Label in the display unit, e.g. `"0.001 in"`.
    pub label: String,
}

/// Returns the jog step presets for a measurement system.
///
/// Metric offers 0.001–100 mm; imperial offers 0.0001–1 in. Values are always stored
/// in millimetres.
///
/// # Arguments
/// * `units` - The display measurement system.
pub fn jog_step_presets(units: MeasurementSystem) -> Vec<JogStepPreset> {
    let label = get_unit_label(units);
    let (values, factor): (&[f32], f32) = match units {
        MeasurementSystem::Metric => (&[0.001, 0.01, 0.1, 1.0, 10.0, 100.0], 1.0),
        MeasurementSystem::Imperial => (&[0.0001, 0.001, 0.01, 0.1, 1.0], MM_PER_INCH),
    };
    values
        .iter()
        .map(|&v| JogStepPreset {
            mm: v * factor,
            label: format!("{v} {label}"),
        })
        .collect()
}

/// Returns the index of the preset closest (by ratio) to a step size.
///
/// # Arguments
/// * `presets` - Presets from [`jog_step_presets`].
/// * `step_mm` - The current step in millimetres.
///
/// # Returns
/// The closest preset's index, or `None` if `presets` is empty.
/// The jog step choices for `units` with `step_mm` selectable exactly: the
/// presets, plus a custom entry when `step_mm` is not one of them.
///
/// # Returns
/// The choices and the index of the one equal to `step_mm`.
pub fn jog_step_choices(units: MeasurementSystem, step_mm: f32) -> (Vec<JogStepPreset>, usize) {
    let mut presets = jog_step_presets(units);
    if let Some(i) = presets
        .iter()
        .position(|p| (p.mm - step_mm).abs() <= p.mm * 1e-3)
    {
        return (presets, i);
    }
    let display = match units {
        MeasurementSystem::Metric => step_mm,
        MeasurementSystem::Imperial => step_mm / MM_PER_INCH,
    };
    let custom = JogStepPreset {
        mm: step_mm,
        label: format!("{} {}", display, get_unit_label(units)),
    };
    let at = presets.partition_point(|p| p.mm < step_mm);
    presets.insert(at, custom);
    (presets, at)
}

pub fn nearest_jog_step_preset(presets: &[JogStepPreset], step_mm: f32) -> Option<usize> {
    let target = if step_mm.is_finite() && step_mm > 0.0 {
        step_mm.ln()
    } else {
        0.0
    };
    presets
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = (a.mm.ln() - target).abs();
            let db = (b.mm.ln() - target).abs();
            da.total_cmp(&db)
        })
        .map(|(i, _)| i)
}

/// Parses a jog feed entry into mm/min.
///
/// # Arguments
/// * `text` - The entry text in the display feed unit.
/// * `units` - The display feed unit.
///
/// # Returns
/// The feed in mm/min, or an error message if the text is not a positive number.
pub fn parse_jog_feed(text: &str, units: FeedRateUnits) -> Result<f32, String> {
    let value = parse_feed_rate(text, units)?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(format!(
            "jog feed must be a positive number, got '{}'",
            text.trim()
        ))
    }
}

/// Formats a probe move for display in the user's units.
///
/// The command itself is always sent in millimetres (G21); this is only for the
/// console log and tooltips.
///
/// # Arguments
/// * `axis_move` - Signed distance in mm and axis letter, e.g. `('Z', -20.0)`.
/// * `feed_mm_per_min` - Probe feed in mm/min.
/// * `units` - Display length unit.
/// * `feed_units` - Display feed unit.
///
/// # Returns
/// Text such as `"Z-0.79 in at 3.94 in/min"` (2 decimal places).
pub fn format_probe_move(
    axis_move: (char, f32),
    feed_mm_per_min: f32,
    units: MeasurementSystem,
    feed_units: FeedRateUnits,
) -> String {
    let (axis, distance_mm) = axis_move;
    let distance = match units {
        MeasurementSystem::Metric => distance_mm,
        MeasurementSystem::Imperial => distance_mm / MM_PER_INCH,
    };
    let feed: f32 = format_feed_rate(feed_mm_per_min, feed_units)
        .parse()
        .unwrap_or(feed_mm_per_min);
    format!(
        "{axis}{distance:.2} {} at {feed:.2} {feed_units}",
        get_unit_label(units)
    )
}

/// Maps a device profile's connection type to a network driver.
///
/// # Arguments
/// * `connection_type` - The profile's connection type ("Serial", "TCP/IP", "WebSocket").
///
/// # Returns
/// `Some(Tcp)` or `Some(WebSocket)` for network profiles, `None` for serial ones.
pub fn network_driver_for(connection_type: &str) -> Option<ConnectionDriver> {
    let kind = connection_type.trim().to_ascii_lowercase();
    if kind.contains("websocket") {
        Some(ConnectionDriver::WebSocket)
    } else if kind.contains("tcp") || kind == "network" || kind == "telnet" {
        Some(ConnectionDriver::Tcp)
    } else {
        None
    }
}

/// Prepares a program for streaming according to Preferences > File
/// Processing: removes comments (`(...)` and `;...`) unless
/// `preserve_comments` is set, drops lines left empty, and rejects a line
/// longer than `max_line_length` characters, which the controller would
/// refuse (GRBL's line buffer is 80 characters, grblHAL's 256).
///
/// # Returns
/// The program to send, or an error naming the first line that is too long.
pub fn prepare_program(
    content: &str,
    preserve_comments: bool,
    max_line_length: usize,
) -> Result<String, String> {
    let mut out = String::with_capacity(content.len());
    for (index, raw) in content.lines().enumerate() {
        let line = if preserve_comments {
            raw.trim().to_string()
        } else {
            strip_gcode_comments(raw)
        };
        if line.is_empty() {
            continue;
        }
        let length = line.chars().count();
        if max_line_length > 0 && length > max_line_length {
            return Err(format!(
                "line {} has {length} characters; the maximum is {max_line_length}",
                index + 1
            ));
        }
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

/// Removes `( ... )` and `; ...` comments from one G-code line and trims it.
fn strip_gcode_comments(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut depth = 0usize;
    for c in line.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            // A `;` inside parentheses is part of that comment, not the start
            // of an end-of-line comment.
            ';' if depth == 0 => break,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}
