//! # Sender
//!
//! Common trait for protocol-specific stream senders.
//!
//! A `Sender` is responsible for streaming G-code to a machine and for the
//! protocol-specific operations that surround that streaming: pause, resume,
//! stop, and unlock.
//!
//! Each protocol (GRBL, Repetier, etc.) provides its own implementation:
//! - `GrblSender`: windowed streaming, `!`/`~`/`0x18`, `$X`, laser/CNC handling.
//! - `RepetierSender`: line-numbered streaming with ack/resend, flow control.
//!
//! The UI (`MachineControlView`) holds a `Rc<dyn Sender>` and calls these
//! methods without knowing which protocol is behind it. Adding a new protocol
//! means adding a new `Sender` implementation, not touching the UI.

/// Common interface for protocol-specific stream senders.
///
/// Implementations are expected to be cheap to clone (typically via `Rc`
/// or `Arc` internally) and to share their state with the UI through
/// thread-safe handles.
pub trait Sender {
    /// Send a complete G-code program (streaming).
    ///
    /// The implementation decides how to chunk, pace and verify the
    /// transmission according to its protocol. This call returns
    /// immediately; the actual streaming happens on a background thread
    /// or on the main loop, depending on the implementation.
    fn send_gcode(&self, content: &str);

    /// Pause the current job.
    ///
    /// GRBL: sends `!` (feed hold), optionally preceded by `M5` depending
    /// on the profile.
    /// Repetier: stops feeding new commands from the internal loop until
    /// `resume` is called.
    fn pause(&self);

    /// Resume a paused job.
    ///
    /// GRBL: sends `~` (cycle start).
    /// Repetier: resumes feeding commands from the internal loop.
    fn resume(&self);

    /// Stop the current job.
    ///
    /// Aborts streaming. Does **not** reset the controller: for a soft
    /// reset, the caller should send `0x18` explicitly (or the sender's
    /// own `unlock`/`reset` implementation should handle it if defined
    /// by a future extension of this trait).
    fn stop(&self);

    /// Release the machine from an ALARM state.
    ///
    /// GRBL: sends `$X`.
    /// Repetier: no-op (not supported by the protocol).
    fn unlock(&self);

    /// Whether the sender is currently streaming a job.
    fn is_streaming(&self) -> bool;
}
