//! # Job Preamble Helpers
//!
//! Shared start-of-job handling for the laser generators (bitmap engraver, tabbed box,
//! jigsaw puzzle).
//!
//! Homing (`$H`) is optional and off by default. When it is enabled, the generators:
//!
//! - home the machine and clear only the G54 **X/Y** offset, so X/Y coordinates are measured
//!   from the homed corner. The Z work offset is never cleared or rewritten.
//! - never command a Z move above work Z0. Straight after `$H` the head sits at the top of
//!   its Z travel, so a positive Z move would run into the soft or hard limit.
//!
//! When homing is disabled the job runs in the active work coordinate system, whose X/Y/Z
//! zero the user has already set, and a positive safe-height move in work coordinates is
//! allowed.
//!
//! Depends only on `std`.

/// Append the work-coordinate setup for the start of a job.
///
/// # Arguments
/// * `gcode` - Program text to append to.
/// * `home_before_job` - Emit `$H` and clear the G54 X/Y offset. The Z offset is untouched.
pub fn push_work_setup(gcode: &mut String, home_before_job: bool) {
    if home_before_job {
        gcode.push_str("; Home and set work coordinate system (Z work offset is left unchanged)\n");
        gcode.push_str("$H ; Home all axes\n");
        gcode.push_str("G10 L2 P1 X0 Y0 ; Clear G54 X/Y offset\n");
        gcode.push_str("G54 ; Select work coordinate system 1\n");
    } else {
        gcode.push_str(
            "; Runs in the active work coordinate system: set the X/Y/Z work zero first\n",
        );
    }
}

/// Z height (work coordinates) to use for travel moves.
///
/// After homing the work Z0 is the highest position the job may command, so travel happens
/// at Z0. Without homing the user has set the work zero and `safe_z` above it is used.
///
/// # Arguments
/// * `home_before_job` - Whether the job homes the machine first.
/// * `safe_z` - Requested safe height above the work surface (mm).
///
/// # Returns
/// The travel height in mm, never above 0 when homing.
pub fn travel_z(home_before_job: bool, safe_z: f32) -> f32 {
    if home_before_job {
        0.0
    } else {
        safe_z.max(0.0)
    }
}
