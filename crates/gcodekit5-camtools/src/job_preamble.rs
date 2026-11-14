//! # Job Preamble Helpers
//!
//! Shared start-of-job handling for the laser generators (bitmap engraver, tabbed box,
//! jigsaw puzzle).
//!
//! Homing (`$H`) is optional and off by default. When it is enabled, the generators:
//!
//! - home the machine and keep the G54 work offsets the user set (GRBL stores them across
//!   homing and power cycles). X/Y/Z are measured from that work zero; nothing is cleared or
//!   rewritten. Clearing X/Y would put work zero at the homing switches, and with GRBL's
//!   default homing the whole travel is in negative machine coordinates, so the job's
//!   positive X/Y moves would leave the travel range (issue #70). Work Z0 is **not**
//!   machine Z0.
//! - make every Z travel/retract move in **machine coordinates** (`G53 G0 Z-1.00`, see
//!   [`HOMED_TRAVEL_Z_MACHINE`]). GRBL homes Z to the top of its travel and machine Z0 is
//!   the homed position, so a machine Z of 0 or below can never run into the Z soft or hard
//!   limit, whatever the G54 Z offset is. A work-coordinate travel height (even `Z0`) would
//!   land above the homed position whenever the G54 Z offset is positive.
//!
//! Cutting / focus heights (plunge depth, pass heights) stay in work coordinates: they are
//! measured from the work Z0 the user set, which must be below machine Z0 for the job to be
//! runnable at all.
//!
//! When homing is disabled the job runs in the active work coordinate system, whose X/Y/Z
//! zero the user has already set, and a positive safe-height move in work coordinates is
//! allowed. Machine coordinates are not used then, because without homing they are only
//! relative to the power-on position.
//!
//! Depends only on `std`.

/// Machine Z (mm) used for every travel/retract move after homing.
///
/// GRBL sets machine Z0 at the homing switch and backs off by the pull-off distance (`$27`,
/// default 1 mm), so after `$H` the head rests at about `-$27`. `-1.0` keeps a 1 mm margin
/// below the switch trip point while staying within the soft-limit range (`0` to
/// `-$132`).
pub const HOMED_TRAVEL_Z_MACHINE: f32 = -1.0;

/// Append the work-coordinate setup for the start of a job.
///
/// # Arguments
/// * `gcode` - Program text to append to.
/// * `home_before_job` - Emit `$H`, then select G54 with its stored offsets unchanged.
pub fn push_work_setup(gcode: &mut String, home_before_job: bool) {
    if home_before_job {
        gcode.push_str("; Home, then run from the G54 work zero you set (offsets unchanged)\n");
        gcode.push_str("$H ; Home all axes\n");
        gcode.push_str("G54 ; Select work coordinate system 1\n");
        gcode.push_str(&format!(
            "; Travel moves use machine Z{:.2} (G53) so they never rise above the homed position\n",
            HOMED_TRAVEL_Z_MACHINE
        ));
    } else {
        gcode.push_str(
            "; Runs in the active work coordinate system: set the X/Y/Z work zero first\n",
        );
    }
}

/// Build one Z travel/retract line.
///
/// After homing this is a machine-coordinate move (`G53 G0 Z-1.00`), which never commands Z
/// above the homed position whatever the G54 Z offset is. Without homing it is a
/// work-coordinate move to `safe_z` (clamped to at least 0) above the work zero the user set.
///
/// # Arguments
/// * `home_before_job` - Whether the job homes the machine first.
/// * `safe_z` - Requested safe height above the work surface (mm), used without homing.
/// * `comment` - Comment text appended after `;`.
///
/// # Returns
/// The G-code line, terminated with `\n`.
pub fn travel_z_line(home_before_job: bool, safe_z: f32, comment: &str) -> String {
    if home_before_job {
        format!(
            "G53 G0 Z{:.2} ; {} (machine coordinates)\n",
            HOMED_TRAVEL_Z_MACHINE, comment
        )
    } else {
        format!("G0 Z{:.2} ; {}\n", safe_z.max(0.0), comment)
    }
}
