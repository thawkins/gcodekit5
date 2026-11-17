//! # Reply ledger
//!
//! GRBL answers every newline-terminated line it receives with exactly one
//! `ok` or `error:N`, in the order the lines were sent, and the reply names no
//! command. This ledger records one entry per line written, so the reply that
//! arrives can be matched to the line it answers. A line can carry a tag (for
//! example a settings change that waits for its `ok`); untagged lines only keep
//! the order.

use std::collections::VecDeque;

/// More outstanding lines than this means replies were lost; the ledger is
/// cleared rather than letting later replies be matched to the wrong lines.
const MAX_OUTSTANDING: usize = 4096;

/// Lines sent to the controller that have not been answered yet, oldest first.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReplyLedger {
    outstanding: VecDeque<Option<u64>>,
}

impl ReplyLedger {
    /// Creates an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `count` lines that were just written to the controller.
    ///
    /// # Arguments
    /// * `count` - Number of complete (newline-terminated) lines written.
    pub fn on_lines_sent(&mut self, count: usize) {
        if self.outstanding.len() + count > MAX_OUTSTANDING {
            tracing::warn!(
                outstanding = self.outstanding.len(),
                "Reply ledger overflow; clearing it"
            );
            self.outstanding.clear();
            return;
        }
        self.outstanding.extend(std::iter::repeat_n(None, count));
    }

    /// Tags the most recently sent line, so its reply can be recognised.
    ///
    /// # Arguments
    /// * `tag` - Caller-chosen identifier returned by [`Self::on_reply`].
    ///
    /// # Returns
    /// `false` if no line is outstanding (nothing was tagged).
    pub fn tag_last(&mut self, tag: u64) -> bool {
        match self.outstanding.back_mut() {
            Some(slot) => {
                *slot = Some(tag);
                true
            }
            None => false,
        }
    }

    /// Records one `ok` or `error:N` reply.
    ///
    /// # Returns
    /// The tag of the line this reply answers, or `None` if that line was
    /// untagged or no line was outstanding.
    pub fn on_reply(&mut self) -> Option<u64> {
        self.outstanding.pop_front().flatten()
    }

    /// Forgets every outstanding line (after a reset, connect or disconnect,
    /// when the controller will not answer them).
    pub fn clear(&mut self) {
        self.outstanding.clear();
    }

    /// Number of lines still waiting for a reply.
    pub fn len(&self) -> usize {
        self.outstanding.len()
    }

    /// Whether no line is waiting for a reply.
    pub fn is_empty(&self) -> bool {
        self.outstanding.is_empty()
    }
}
