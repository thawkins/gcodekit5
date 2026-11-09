//! Step-by-step sequencing of GRBL feed and spindle overrides.
//!
//! GRBL 1.1 has no "set override to N%" command. Overrides are changed with
//! realtime bytes (reset to 100%, ±10%, ±1%), and GRBL records each request as
//! a pending flag rather than in a counted queue. Several identical bytes sent
//! back-to-back before the controller's protocol loop runs therefore collapse
//! into a single adjustment.
//!
//! To reach an exact percentage reliably, callers send **one** step at a time
//! and wait for the controller to report the new value in the `Ov:` field of a
//! status report before choosing the next step. [`next_override_step`] picks
//! that next step from the last reported value.

/// GRBL's lowest feed/spindle override percentage.
pub const MIN_OVERRIDE_PERCENT: u16 = 10;
/// GRBL's highest feed/spindle override percentage.
pub const MAX_OVERRIDE_PERCENT: u16 = 200;

/// The realtime bytes that adjust one override (feed or spindle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverrideStepCodes {
    /// Reset the override to 100%.
    pub reset: u8,
    /// Increase by 10%.
    pub plus_10: u8,
    /// Decrease by 10%.
    pub minus_10: u8,
    /// Increase by 1%.
    pub plus_1: u8,
    /// Decrease by 1%.
    pub minus_1: u8,
}

/// Feed override realtime bytes (GRBL 1.1: 0x90–0x94).
pub const FEED_OVERRIDE_CODES: OverrideStepCodes = OverrideStepCodes {
    reset: 0x90,
    plus_10: 0x91,
    minus_10: 0x92,
    plus_1: 0x93,
    minus_1: 0x94,
};

/// Spindle override realtime bytes (GRBL 1.1: 0x99–0x9D).
pub const SPINDLE_OVERRIDE_CODES: OverrideStepCodes = OverrideStepCodes {
    reset: 0x99,
    plus_10: 0x9A,
    minus_10: 0x9B,
    plus_1: 0x9C,
    minus_1: 0x9D,
};

/// Number of ±10/±1 steps needed to move an override by `delta` percent.
fn step_count(delta: i32) -> u32 {
    let magnitude = delta.unsigned_abs();
    magnitude / 10 + magnitude % 10
}

/// Returns true if `percent` is a valid GRBL feed/spindle override target.
pub fn is_valid_override_target(percent: u16) -> bool {
    (MIN_OVERRIDE_PERCENT..=MAX_OVERRIDE_PERCENT).contains(&percent)
}

/// Chooses the next single realtime byte that moves an override toward `target`.
///
/// The choice is based on `current`, the value the controller last reported in
/// its `Ov:` field. It uses ±10% steps while at least 10% away and ±1% steps
/// after that, and starts with a reset to 100% when that needs fewer steps
/// overall. Steps never overshoot the target, so GRBL's clamping at 10% and
/// 200% never comes into play.
///
/// # Arguments
/// * `current` - Override percentage last reported by the controller.
/// * `target` - Desired override percentage (10–200).
/// * `codes` - The realtime bytes for the override being adjusted.
///
/// # Returns
/// `Some(byte)` to send next, or `None` once `current` equals `target`.
pub fn next_override_step(current: u16, target: u16, codes: OverrideStepCodes) -> Option<u8> {
    if current == target {
        return None;
    }
    let direct = step_count(i32::from(target) - i32::from(current));
    let via_reset = 1 + step_count(i32::from(target) - 100);
    if current != 100 && via_reset < direct {
        return Some(codes.reset);
    }

    let delta = i32::from(target) - i32::from(current);
    Some(match (delta >= 0, delta.unsigned_abs() >= 10) {
        (true, true) => codes.plus_10,
        (true, false) => codes.plus_1,
        (false, true) => codes.minus_10,
        (false, false) => codes.minus_1,
    })
}

/// Applies the effect of `byte` to `current`, mirroring GRBL's own arithmetic.
///
/// This is used to predict the value after a step (for example in tests, or to
/// show progress) and clamps to GRBL's 10–200% range.
///
/// # Arguments
/// * `current` - Override percentage before the step.
/// * `byte` - The realtime byte sent.
/// * `codes` - The realtime bytes for the override being adjusted.
///
/// # Returns
/// The override percentage GRBL would report after processing `byte`, or
/// `current` unchanged if `byte` is not one of `codes`.
pub fn apply_override_step(current: u16, byte: u8, codes: OverrideStepCodes) -> u16 {
    let next = if byte == codes.reset {
        100
    } else if byte == codes.plus_10 {
        i32::from(current) + 10
    } else if byte == codes.minus_10 {
        i32::from(current) - 10
    } else if byte == codes.plus_1 {
        i32::from(current) + 1
    } else if byte == codes.minus_1 {
        i32::from(current) - 1
    } else {
        i32::from(current)
    };
    next.clamp(
        i32::from(MIN_OVERRIDE_PERCENT),
        i32::from(MAX_OVERRIDE_PERCENT),
    ) as u16
}
