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

pub struct RepetierProcessor {
    pub line_counter: u32,
    last_known_x: f32,
    last_known_y: f32,
    last_known_z: f32,
    /// Distance mode of the commanded moves (`G91` when true).
    relative_moves: bool,
    /// Units of the commanded moves (`G20` when true).
    inch_units: bool,
    /// Bytes of an incoming line whose newline has not arrived yet.
    rx_partial: String,
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
            last_known_x: 0.0,
            last_known_y: 0.0,
            last_known_z: 0.0,
            relative_moves: false,
            inch_units: false,
            rx_partial: String::new(),
            is_waiting_for_hardware: false,
        }
    }

    pub fn reset(&mut self) {
        self.line_counter = 0;
        self.is_waiting_for_hardware = false;
        self.rx_partial.clear();
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
    /// line and `Idle` otherwise. The position is the last one the firmware
    /// reported (`X:.. Y:.. Z:..`) or, failing that, the last commanded one.
    pub fn generate_mock_grbl_status(&self) -> Vec<u8> {
        let state = if self.is_waiting_for_hardware {
            "Run"
        } else {
            "Idle"
        };
        format!(
            "<{}|MPos:{:.3},{:.3},{:.3}|Bf:15,64|FS:0,0>\r\n",
            state, self.last_known_x, self.last_known_y, self.last_known_z
        )
        .into_bytes()
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

        // A line that already carries its own number and checksum keeps its
        // number; only the G-code is normalised and the checksum recomputed.
        if cleaned_upper.starts_with('N') && cleaned_upper.contains('*') {
            let numbered = cleaned_upper
                .split('*')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            if let Some((prefix_n, remainder)) = numbered.split_once(' ') {
                let normalized_gcode = normalize_words(remainder);
                self.track_position(&normalized_gcode);
                let cmd = format!("{} {} ", prefix_n.trim(), normalized_gcode);
                let ck = self.calculate_checksum(&cmd);
                return Some(format!("{}*{}\n", cmd, ck).into_bytes());
            }
            return Some(format!("{}\n", numbered).into_bytes());
        }

        let gcode = strip_line_number(&strip_comments(&cleaned_upper));
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
            let lines = translate_jog(jog);
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

    /// Numbers and checksums one G-code line, appends it to `out`, and
    /// updates the tracked position.
    fn emit(&mut self, gcode: &str, out: &mut String) {
        self.line_counter += 1;
        let cmd = format!("N{} {} ", self.line_counter, gcode);
        let ck = self.calculate_checksum(&cmd);
        out.push_str(&format!("{}*{}\n", cmd, ck));
        self.track_position(gcode);
    }

    /// Updates the tracked position from one commanded G-code line.
    fn track_position(&mut self, gcode: &str) {
        let mut axes: Vec<(char, f32)> = Vec::new();
        let mut homing = false;
        let mut set_position = false;

        for word in gcode.split_whitespace() {
            match word {
                "G90" => self.relative_moves = false,
                "G91" => self.relative_moves = true,
                "G20" => self.inch_units = true,
                "G21" => self.inch_units = false,
                "G28" => homing = true,
                "G92" => set_position = true,
                _ => {
                    let mut chars = word.chars();
                    if let Some(axis @ 'X'..='Z') = chars.next() {
                        if let Ok(value) = chars.as_str().parse::<f32>() {
                            axes.push((axis, value));
                        }
                    }
                }
            }
        }

        if homing {
            let all = axes.is_empty();
            for axis in ['X', 'Y', 'Z'] {
                if all || axes.iter().any(|(a, _)| *a == axis) {
                    *self.axis_mut(axis) = 0.0;
                }
            }
            return;
        }

        let scale = if self.inch_units { MM_PER_INCH } else { 1.0 };
        let relative = self.relative_moves && !set_position;
        for (axis, value) in axes {
            let target = self.axis_mut(axis);
            if relative {
                *target += value * scale;
            } else {
                *target = value * scale;
            }
        }
    }

    fn axis_mut(&mut self, axis: char) -> &mut f32 {
        match axis {
            'X' => &mut self.last_known_x,
            'Y' => &mut self.last_known_y,
            _ => &mut self.last_known_z,
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
            }

            if let Some(idx) = lower.find("resend:") {
                let num_str: String = lower[idx + 7..]
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(n) = num_str.parse::<u32>() {
                    events.push(RepetierEvent::Resend(n));
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

    /// Reads a Marlin/Repetier position report (`X:1.00 Y:2.00 Z:3.00 ...`).
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
            self.last_known_x = x;
            self.last_known_y = y;
            self.last_known_z = z;
        }
    }
}

/// Maps the line numbers a job used back to the job's file lines, so a
/// firmware `Resend:N` rewinds to the right place.
///
/// Line numbers are global to the connection (console commands and jogs use
/// them too), and one file line can emit several numbered lines, so the
/// mapping is recorded as each line is sent rather than computed.
#[derive(Debug, Default, Clone)]
pub struct JobLineMap {
    numbers: BTreeMap<u32, usize>,
}

impl JobLineMap {
    /// Records that file line `index` used numbers `before + 1 ..= after`.
    pub fn record(&mut self, before: u32, after: u32, index: usize) {
        for n in before.saturating_add(1)..=after {
            self.numbers.insert(n, index);
        }
    }

    /// The file line that was sent with line number `n`, if it was part of
    /// this job.
    pub fn file_index(&self, n: u32) -> Option<usize> {
        self.numbers.get(&n).copied()
    }

    /// The first line number used by file line `index`.
    pub fn first_number(&self, index: usize) -> Option<u32> {
        self.numbers
            .iter()
            .find(|(_, i)| **i == index)
            .map(|(n, _)| *n)
    }

    /// Forgets every recorded number (a new job is starting).
    pub fn clear(&mut self) {
        self.numbers.clear();
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

/// Drops a leading `N<digits>` word: Repetier needs its own consecutive
/// numbering, so a file's line numbers are replaced, not forwarded.
fn strip_line_number(line: &str) -> String {
    match line.split_once(' ') {
        Some((first, rest)) if is_line_number(first) => rest.trim().to_string(),
        _ if is_line_number(line) => String::new(),
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
fn normalize_words(gcode: &str) -> String {
    gcode
        .split_whitespace()
        .map(|word| match word {
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
/// A GRBL jog does not change the modal state, but Marlin applies `G91`
/// and `G20` to everything after it, so both are restored after the move.
/// Returns no lines when the jog has no axis words.
fn translate_jog(body: &str) -> Vec<String> {
    let words: Vec<&str> = body.split_whitespace().collect();
    let units = words.iter().find(|w| matches!(**w, "G20" | "G21")).copied();
    let incremental = words.contains(&"G91");
    let axes: Vec<&str> = words
        .iter()
        .filter(|w| w.starts_with(['X', 'Y', 'Z', 'A', 'B', 'C']))
        .copied()
        .collect();
    if axes.is_empty() {
        return Vec::new();
    }
    let feed = words
        .iter()
        .find(|w| w.starts_with('F'))
        .copied()
        .unwrap_or("F2000");

    let mut lines = Vec::new();
    if let Some(units) = units {
        lines.push(units.to_string());
    }
    if incremental {
        lines.push("G91".to_string());
    }
    lines.push(format!("G1 {} {}", axes.join(" "), feed));
    if incremental {
        lines.push("G90".to_string());
    }
    if units == Some("G20") {
        lines.push("G21".to_string());
    }
    lines
}

/// Translates a GRBL `G10 L20 P<n> X.. Y..` into a Repetier/Marlin `G92`.
///
/// `G10 L20` sets the work origin so that the current position reads as
/// the given values; `G92` with the same axis values does the same in
/// Marlin. Returns `None` for any other line, including other `G10` forms.
fn translate_g10_to_g92(gcode: &str) -> Option<String> {
    let words: Vec<&str> = gcode.split_whitespace().collect();
    if words.first() != Some(&"G10") || !words.contains(&"L20") {
        return None;
    }

    let axes: Vec<&str> = words
        .iter()
        .filter(|w| w.starts_with(['X', 'Y', 'Z', 'A', 'B', 'C']))
        .copied()
        .collect();
    if axes.is_empty() {
        return None;
    }

    Some(format!("G92 {}", axes.join(" ")))
}
