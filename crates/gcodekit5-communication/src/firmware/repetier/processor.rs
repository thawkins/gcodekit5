//! # Repetier stream processor
//!
//! Translates the GRBL-style commands the UI emits into Repetier/Marlin
//! G-code, numbers and checksums every outgoing line, frames incoming bytes
//! into lines, and synthesises a GRBL status report so the UI's DROs keep
//! working while a Repetier machine is connected.

use std::collections::BTreeMap;

/// Structured event extracted from a firmware response
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepetierEvent {
    /// Firmware has confirmed a line: `ok` or `wait`
    Ack,
    /// Firmware requests forwarding from line N
    Resend(u32),
}

/// Result of processing incoming firmware bytes
pub struct IncomingResult {
    pub display: Vec<u8>,
    pub events: Vec<RepetierEvent>,
}

/// Longest partial line kept while waiting for its newline. Anything longer
/// is not a protocol reply, so it is passed through as-is instead of
/// growing without bound.
const MAX_PARTIAL_LINE: usize = 4096;

/// Millimetres per inch, for tracking positions commanded in G20.
const MM_PER_INCH: f32 = 25.4;

/// Numbered lines kept for `Resend:N` replays. The firmware only asks for
/// lines it has not accepted yet, which is never more than its receive
/// buffer, so this is far more than any replay needs.
const SENT_HISTORY_LEN: usize = 512;

pub struct RepetierProcessor {
    pub line_counter: u32,
    /// Tracked machine position (mm): the work position plus `offset`.
    machine: [f32; 3],
    /// Work coordinate offset (mm) set by `G92`; cleared by homing.
    offset: [f32; 3],
    /// Distance mode of the commanded moves (`G91` when true).
    relative_moves: bool,
    /// Units of the commanded moves (`G20` when true).
    inch_units: bool,
    /// Bytes of an incoming line whose newline has not arrived yet.
    rx_partial: String,
    /// The exact numbered lines sent, by line number, for resend replays.
    sent_history: BTreeMap<u32, String>,
    pub is_waiting_for_hardware: bool,
}

impl Default for RepetierProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl RepetierProcessor {
    pub fn new() -> Self {
        Self {
            line_counter: 0,
            machine: [0.0; 3],
            offset: [0.0; 3],
            relative_moves: false,
            inch_units: false,
            rx_partial: String::new(),
            sent_history: BTreeMap::new(),
            is_waiting_for_hardware: false,
        }
    }

    /// Returns the processor to its power-on state: numbering, flow
    /// control, receive buffer, resend history, modal state and position.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn mark_buffer_full(&mut self) {
        self.is_waiting_for_hardware = true;
    }

    pub fn mark_ack(&mut self) {
        self.is_waiting_for_hardware = false;
    }

    /// Activates Repetier mode after signature detection.
    /// Leaves the processor in a clean state to begin translation.
    pub fn activate(&mut self) {
        self.reset();
    }

    pub fn calculate_checksum(&self, command: &str) -> u8 {
        command.as_bytes().iter().fold(0u8, |acc, b| acc ^ b)
    }

    /// Builds a GRBL-style status report from the tracked machine state.
    ///
    /// The state is `Run` while the firmware has not acknowledged the last
    /// line and `Idle` otherwise. `MPos` is the tracked machine position and
    /// `WCO` the `G92` offset, so the UI can derive the work position.
    pub fn generate_mock_grbl_status(&self) -> Vec<u8> {
        let state = if self.is_waiting_for_hardware {
            "Run"
        } else {
            "Idle"
        };
        let [mx, my, mz] = self.machine;
        let [ox, oy, oz] = self.offset;
        format!(
            "<{state}|MPos:{mx:.3},{my:.3},{mz:.3}|Bf:15,64|FS:0,0|WCO:{ox:.3},{oy:.3},{oz:.3}>\r\n"
        )
        .into_bytes()
    }

    /// The numbered lines from `from` to the last one sent, exactly as they
    /// were written, for answering a firmware `Resend:N`.
    ///
    /// # Returns
    /// The lines to write again (empty when `from` is the next number not
    /// yet sent), or `None` when a requested line is no longer known, in
    /// which case the request cannot be honoured.
    pub fn resend_lines(&self, from: u32) -> Option<Vec<String>> {
        if from == 0 || from > self.line_counter.saturating_add(1) {
            return None;
        }
        (from..=self.line_counter)
            .map(|n| self.sent_history.get(&n).cloned())
            .collect()
    }

    /// Translates one outgoing command into numbered Repetier lines.
    ///
    /// # Arguments
    /// * `data` - One command as the UI or the job sender produced it.
    ///
    /// # Returns
    /// The bytes to write to the port, or `None` when the command has no
    /// Repetier equivalent (for example a comment-only line).
    pub fn process_outgoing(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        let raw_string = String::from_utf8_lossy(data);
        let cleaned_upper = raw_string.trim().to_ascii_uppercase();

        // Pass through any command that is not standard G-code. This
        // covers GRBL-style commands ($I, $$, $X, ...) and the realtime
        // bytes (? ! ~), which have no meaning in Repetier but are harmless
        // to forward as-is.
        //
        // Sending them unchanged is what makes the divergence check work:
        // if the firmware actually speaks GRBL (misconfigured profile), it
        // answers with its GRBL identity banner and the UI can reject the
        // connection. `$H` and `$J=` are translated below instead.
        if (cleaned_upper.starts_with('$')
            && !cleaned_upper.starts_with("$H")
            && !cleaned_upper.starts_with("$J="))
            || cleaned_upper == "?"
            || cleaned_upper == "!"
            || cleaned_upper == "~"
            || cleaned_upper.is_empty()
        {
            return Some(data.to_vec());
        }

        // A line that already carries a number and checksum is renumbered
        // like any other: Repetier needs one consecutive sequence per
        // connection, and only lines numbered here can be replayed on a
        // resend request.
        let unnumbered = if cleaned_upper.starts_with('N') {
            cleaned_upper.split('*').next().unwrap_or_default()
        } else {
            cleaned_upper.as_str()
        };

        let gcode = strip_line_number(&strip_comments(unnumbered));
        if gcode.is_empty() {
            return None;
        }

        let mut payload = String::new();

        if let Some(axes) = gcode.strip_prefix("$H") {
            let homing = translate_homing(axes);
            self.emit(&homing, &mut payload);
            tracing::debug!("Repetier: {} translated to {}", gcode, homing);
            return Some(payload.into_bytes());
        }

        if let Some(jog) = gcode.strip_prefix("$J=") {
            let lines = translate_jog(jog, self.relative_moves, self.inch_units);
            if lines.is_empty() {
                tracing::warn!("Repetier: jog without axis words ignored: {}", gcode);
                return None;
            }
            for line in &lines {
                self.emit(line, &mut payload);
            }
            return Some(payload.into_bytes());
        }

        // WCS zero (GRBL-style) -> G92 (Repetier/Marlin). The UI emits
        // `G10 L20 P<n> X.. Y.. Z..` for "set current position as origin";
        // Marlin's equivalent is `G92` with the same axis values.
        if let Some(g92) = translate_g10_to_g92(&gcode) {
            self.emit(&g92, &mut payload);
            tracing::debug!("Repetier: {} translated to {}", gcode, g92);
            return Some(payload.into_bytes());
        }

        // A bare coordinate line continues the current move.
        let gcode = if gcode.starts_with(['X', 'Y', 'Z']) {
            format!("G1 {}", gcode)
        } else {
            gcode
        };

        if !starts_with_word(&gcode) {
            tracing::warn!("Repetier: no Repetier equivalent for {:?}", gcode);
            return None;
        }

        self.emit(&normalize_words(&gcode), &mut payload);
        Some(payload.into_bytes())
    }

    /// Numbers and checksums one G-code line, appends it to `out`, keeps it
    /// for resend replays, and updates the tracked position.
    fn emit(&mut self, gcode: &str, out: &mut String) {
        self.line_counter += 1;
        let cmd = format!("N{} {} ", self.line_counter, gcode);
        let ck = self.calculate_checksum(&cmd);
        let line = format!("{}*{}\n", cmd, ck);
        out.push_str(&line);
        self.sent_history.insert(self.line_counter, line);
        while self.sent_history.len() > SENT_HISTORY_LEN {
            self.sent_history.pop_first();
        }
        self.track_position(gcode);
    }

    /// Updates the modal state and tracked position from one commanded
    /// G-code line.
    ///
    /// Axis words move the position only on motion (`G0`-`G3`), homing
    /// (`G28`) and set-position (`G92`) lines; on any other line (for
    /// example `M201 X1000`) they are parameters, not coordinates.
    fn track_position(&mut self, gcode: &str) {
        let mut axes: Vec<(usize, f32)> = Vec::new();
        let mut motion = false;
        let mut homing = false;
        let mut set_position = false;

        for word in words(gcode) {
            let mut chars = word.chars();
            let Some(letter) = chars.next() else {
                continue;
            };
            let Ok(value) = chars.as_str().parse::<f32>() else {
                continue;
            };
            match letter {
                'G' if value == 90.0 => self.relative_moves = false,
                'G' if value == 91.0 => self.relative_moves = true,
                'G' if value == 20.0 => self.inch_units = true,
                'G' if value == 21.0 => self.inch_units = false,
                'G' if value == 28.0 => homing = true,
                'G' if value == 92.0 => set_position = true,
                'G' if [0.0, 1.0, 2.0, 3.0].contains(&value) => motion = true,
                'X' => axes.push((0, value)),
                'Y' => axes.push((1, value)),
                'Z' => axes.push((2, value)),
                _ => {}
            }
        }

        let scale = if self.inch_units { MM_PER_INCH } else { 1.0 };

        if homing {
            // Marlin homes to machine zero and clears the G92 offset.
            let all = axes.is_empty();
            for axis in 0..3 {
                if all || axes.iter().any(|(a, _)| *a == axis) {
                    self.machine[axis] = 0.0;
                    self.offset[axis] = 0.0;
                }
            }
        } else if set_position {
            for (axis, value) in axes {
                self.offset[axis] = self.machine[axis] - value * scale;
            }
        } else if motion {
            for (axis, value) in axes {
                if self.relative_moves {
                    self.machine[axis] += value * scale;
                } else {
                    self.machine[axis] = value * scale + self.offset[axis];
                }
            }
        }
    }

    /// Frames incoming bytes into lines and extracts acks and resends.
    ///
    /// A line split across reads is held until its newline arrives, so an
    /// `ok` delivered as `"o"` then `"k\n"` still produces one ack.
    pub fn process_incoming(&mut self, raw_bytes: Vec<u8>) -> IncomingResult {
        self.rx_partial
            .push_str(&String::from_utf8_lossy(&raw_bytes));
        let mut display: Vec<u8> = Vec::new();
        let mut events: Vec<RepetierEvent> = Vec::new();

        let mut lines: Vec<String> = Vec::new();
        while let Some(idx) = self.rx_partial.find('\n') {
            lines.push(self.rx_partial.drain(..=idx).collect());
        }
        if self.rx_partial.len() > MAX_PARTIAL_LINE {
            lines.push(std::mem::take(&mut self.rx_partial));
        }

        for line in &lines {
            let l = line.trim();

            // Firmware noise and internal echoes: not displayed, no events.
            if l.is_empty()
                || l.contains("==")
                || l.contains("recv")
                || l.contains("exec")
                || l.contains("G11")
            {
                continue;
            }

            let lower = l.to_lowercase();

            if lower.starts_with("ok") || lower.contains("wait") {
                events.push(RepetierEvent::Ack);
                self.is_waiting_for_hardware = false;
            }

            if let Some(idx) = lower.find("resend:") {
                let num_str: String = lower[idx + 7..]
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(n) = num_str.parse::<u32>() {
                    events.push(RepetierEvent::Resend(n));
                    self.is_waiting_for_hardware = true;
                }
            }

            self.apply_position_report(l);

            display.extend_from_slice(l.as_bytes());
            display.extend_from_slice(b"\r\n");
        }

        // The GRBL status mock gives the UI's DROs data in Repetier mode.
        display.extend_from_slice(&self.generate_mock_grbl_status());

        IncomingResult { display, events }
    }

    /// Reads a Marlin/Repetier position report (`X:1.00 Y:2.00 Z:3.00 ...`),
    /// which is in work coordinates.
    fn apply_position_report(&mut self, line: &str) {
        let read = |axis: &str| -> Option<f32> {
            let start = line.find(axis)? + axis.len();
            let value: String = line[start..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'))
                .collect();
            value.parse().ok()
        };
        if let (Some(x), Some(y), Some(z)) = (read("X:"), read("Y:"), read("Z:")) {
            for (axis, value) in [x, y, z].into_iter().enumerate() {
                self.machine[axis] = value + self.offset[axis];
            }
        }
    }
}

/// Removes `( ... )` comments and everything after `;`.
fn strip_comments(line: &str) -> String {
    let line = line.split(';').next().unwrap_or_default();
    let mut out = String::with_capacity(line.len());
    let mut depth = 0usize;
    for c in line.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Splits a G-code line into words (`G1`, `X-1.5`, `M4`), whether or not
/// they are separated by spaces, so `G1X10Y5` and `G1 X10 Y5` read alike.
///
/// Returns `None` when the line is not made only of letter-number words
/// (for example `M117 HELLO`), so callers fall back to whitespace words.
fn tokenize(line: &str) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if !c.is_ascii_alphabetic() {
            return None;
        }
        let mut word = c.to_string();
        while let Some(&d) = chars.peek() {
            if d.is_ascii_digit() || matches!(d, '.' | '-' | '+') {
                word.push(d);
                chars.next();
            } else {
                break;
            }
        }
        if word.len() == 1 {
            return None;
        }
        out.push(word);
    }
    Some(out)
}

/// The words of a G-code line: [`tokenize`] when the line is plain G-code,
/// whitespace-separated pieces otherwise.
fn words(line: &str) -> Vec<String> {
    tokenize(line).unwrap_or_else(|| line.split_whitespace().map(str::to_string).collect())
}

/// Drops a leading `N<digits>` word: Repetier needs its own consecutive
/// numbering, so a file's line numbers are replaced, not forwarded.
fn strip_line_number(line: &str) -> String {
    let all = words(line);
    match all.first() {
        Some(first) if is_line_number(first) => all[1..].join(" "),
        _ => line.to_string(),
    }
}

fn is_line_number(word: &str) -> bool {
    word.len() > 1 && word.starts_with('N') && word[1..].chars().all(|c| c.is_ascii_digit())
}

/// True when the line opens with a G-code word: a letter followed by a
/// number (`G1`, `M3`, `F1000`, `E2.5`, `T0`).
fn starts_with_word(line: &str) -> bool {
    let mut chars = line.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(letter), Some(next))
            if letter.is_ascii_alphabetic()
                && (next.is_ascii_digit() || matches!(next, '-' | '+' | '.'))
    )
}

/// Applies the word substitutions Repetier needs: `M4` (dynamic laser,
/// unsupported) becomes `M3`, and zero coordinates get explicit decimals.
/// Only an exact `M4`/`M04` matches; `M400` and `M42` are left alone.
fn normalize_words(gcode: &str) -> String {
    words(gcode)
        .iter()
        .map(|word| match word.as_str() {
            "M4" | "M04" => "M3",
            "X0" | "X0.0" | "X-0" => "X0.00",
            "Y0" | "Y0.0" | "Y-0" => "Y0.00",
            "Z0" | "Z0.0" | "Z-0" => "Z0.00",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Translates GRBL `$H` (home all) or `$H<axes>` (e.g. `$HZ`) into `G28`.
fn translate_homing(axes: &str) -> String {
    let words: Vec<String> = axes
        .chars()
        .filter(|c| matches!(c, 'X'..='Z'))
        .map(|c| format!("{}0", c))
        .collect();
    if words.is_empty() {
        "G28".to_string()
    } else {
        format!("G28 {}", words.join(" "))
    }
}

/// Translates the body of a GRBL jog (`$J=G21 G91 X10 F1000`) into Repetier
/// lines.
///
/// A GRBL jog does not change the modal state, but Marlin applies `G90`,
/// `G91` and `G20` to everything after them, so the jog's own distance and
/// units words are sent before the move and the modes in force before the
/// jog (`relative_now`, `inch_now`) are restored after it.
/// Returns no lines when the jog has no axis words.
fn translate_jog(body: &str, relative_now: bool, inch_now: bool) -> Vec<String> {
    let words = words(body);
    let has = |w: &str| words.iter().any(|x| x == w);
    let units = ["G20", "G21"].into_iter().find(|w| has(w));
    let distance = ["G90", "G91"].into_iter().find(|w| has(w));
    let axes: Vec<&str> = words
        .iter()
        .filter(|w| w.starts_with(['X', 'Y', 'Z', 'A', 'B', 'C']))
        .map(String::as_str)
        .collect();
    if axes.is_empty() {
        return Vec::new();
    }
    let feed = words
        .iter()
        .find(|w| w.starts_with('F'))
        .map(String::as_str)
        .unwrap_or("F2000");

    let mut lines = Vec::new();
    if let Some(units) = units {
        lines.push(units.to_string());
    }
    if let Some(distance) = distance {
        lines.push(distance.to_string());
    }
    lines.push(format!("G1 {} {}", axes.join(" "), feed));
    if let Some(distance) = distance
        && (distance == "G91") != relative_now
    {
        lines.push(if relative_now { "G91" } else { "G90" }.to_string());
    }
    if let Some(units) = units
        && (units == "G20") != inch_now
    {
        lines.push(if inch_now { "G20" } else { "G21" }.to_string());
    }
    lines
}

/// Translates a GRBL `G10 L20 P<n> X.. Y..` into a Repetier/Marlin `G92`.
///
/// `G10 L20` sets the work origin so that the current position reads as
/// the given values; `G92` with the same axis values does the same in
/// Marlin. Returns `None` for any other line, including other `G10` forms.
fn translate_g10_to_g92(gcode: &str) -> Option<String> {
    let words = words(gcode);
    if words.first().map(String::as_str) != Some("G10") || !words.iter().any(|w| w == "L20") {
        return None;
    }

    let axes: Vec<&str> = words
        .iter()
        .filter(|w| w.starts_with(['X', 'Y', 'Z', 'A', 'B', 'C']))
        .map(String::as_str)
        .collect();
    if axes.is_empty() {
        return None;
    }

    Some(format!("G92 {}", axes.join(" ")))
}
