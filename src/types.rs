use std::collections::VecDeque;
/// Application-level types for the root crate
#[derive(Debug, Default)]
pub struct GcodeSendState {
    pub lines: VecDeque<String>,
    pub pending_bytes: usize,
    pub line_lengths: VecDeque<usize>,
    pub sent_lines: VecDeque<String>,
    pub total_sent: usize,
    pub total_lines: usize,
    pub start_time: Option<std::time::Instant>,
}
