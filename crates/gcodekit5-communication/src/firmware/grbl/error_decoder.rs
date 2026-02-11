//! GRBL Error and Alarm Code Decoder
//! Converts numeric error and alarm codes to human-readable messages

/// Decode GRBL error code to human-readable message
pub fn decode_error(code: u8) -> String {
    match code {
        1 => "G-code words consist of a letter and a value. Letter was not found.".to_string(),
        2 => "Numeric value format is not valid or missing an expected value.".to_string(),
        3 => "Grbl '$' system command was not recognized or supported.".to_string(),
        4 => "Negative value received for an expected positive value.".to_string(),
        5 => "Homing cycle is not enabled via settings.".to_string(),
        6 => "Minimum step pulse time must be greater than 3µs.".to_string(),
        7 => "EEPROM read failed. Reset and restored to default values.".to_string(),
        8 => "Grbl '$' command cannot be used unless Grbl is IDLE. Ensures smooth operation during a job.".to_string(),
        9 => "G-code locked out during alarm or jog state.".to_string(),
        10 => "Soft limits cannot be enabled without homing also enabled.".to_string(),
        11 => "Max characters per line exceeded. Line was not processed and executed.".to_string(),
        12 => "Grbl '$' setting value exceeds the maximum step rate supported.".to_string(),
        13 => "Safety door detected as opened and door state initiated.".to_string(),
        14 => "Build info or startup line exceeded EEPROM line length limit.".to_string(),
        15 => "Jog target exceeds machine travel. Command ignored.".to_string(),
        16 => "Jog command with no '=' or contains prohibited g-code.".to_string(),
        17 => "Laser mode requires PWM output.".to_string(),
        20 => "Unsupported or invalid g-code command found in block.".to_string(),
        21 => "More than one g-code command from same modal group found in block.".to_string(),
        22 => "Feed rate has not yet been set or is undefined.".to_string(),
        23 => "G-code command in block requires an integer value.".to_string(),
        24 => "Two G-code commands that both require the use of the XYZ axis words were detected in the block.".to_string(),
        25 => "A G-code word was repeated in the block.".to_string(),
        26 => "A G-code command implicitly or explicitly requires XYZ axis words in the block, but none were detected.".to_string(),
        27 => "N line number value is not within the valid range of 1 - 9,999,999.".to_string(),
        28 => "A G-code command was sent, but is missing some required P or L value words in the line.".to_string(),
        29 => "Grbl supports six work coordinate systems G54-G59. G59.1, G59.2, and G59.3 are not supported.".to_string(),
        30 => "The G53 G-code command requires either a G0 seek or G1 feed motion mode to be active.".to_string(),
        31 => "There are unused axis words in the block and G80 motion mode cancel is active.".to_string(),
        32 => "A G2 or G3 arc was commanded but there are no XYZ axis words in the selected plane to trace the arc.".to_string(),
        33 => "The motion command has an invalid target. G2, G3, and G38.2 generates this error if the arc is impossible to generate or if the probe target is the current position.".to_string(),
        34 => "A G2 or G3 arc, traced with the radius definition, had a mathematical error when computing the arc geometry.".to_string(),
        35 => "A G2 or G3 arc, traced with the offset definition, is missing the IJK offset word in the selected plane to trace the arc.".to_string(),
        36 => "There are unused, leftover G-code words that aren't used by any command in the block.".to_string(),
        37 => "The G43.1 dynamic tool length offset command cannot apply an offset to an axis other than its configured axis.".to_string(),
        38 => "Tool number greater than max supported value.".to_string(),

        // grblHAL Extended Error Codes (39-75)
        39 => "Canned cycle is not active. G-code requires an active canned cycle (G81-G89) context.".to_string(),
        40 => "Value word (N, P, R) is missing or undefined.".to_string(),
        41 => "Value word conflict. L word cannot be used with canned cycle P word.".to_string(),
        42 => "Invalid canned cycle retract mode. R-plane must be above Z-datum for G98 or current position for G99.".to_string(),
        43 => "G-code requires RPM to be set (S-word) when spindle is enabled.".to_string(),
        44 => "PID log is full. Cannot add more data to the PID tuning log.".to_string(),
        45 => "Max step rate exceeded. Motion rate would exceed maximum step rate for any axis.".to_string(),
        46 => "Safety door already opened. Attempting to open an already open safety door.".to_string(),
        47 => "Illegal operation. Cannot jog or use G28/G30 from within a program.".to_string(),
        48 => "Unsupported P-parameter. P-value not within valid range for specific command.".to_string(),
        49 => "Value out of range. Numerical value is outside acceptable range for command.".to_string(),
        50 => "Setting step pulse min > step pulse time. Minimum step pulse time must be less than or equal to pulse time.".to_string(),
        51 => "Limits check failed. Sensor not detected at expected position during homing.".to_string(),
        52 => "Limit switch pull-off failed. Unable to clear limit switch when pulling off.".to_string(),
        53 => "Invalid file number. File number specified does not exist or is invalid.".to_string(),
        54 => "File is read-only. Cannot write to or delete a read-only file.".to_string(),
        55 => "File is empty. Cannot execute empty file or no valid G-code found.".to_string(),
        56 => "File not found. Specified file does not exist on storage device.".to_string(),
        57 => "File read failed. Error reading from storage device.".to_string(),
        58 => "Spindle at speed timeout. Spindle failed to reach commanded speed within timeout period.".to_string(),
        59 => "Spindle not running. Command requires spindle to be running (e.g., M3 or M4).".to_string(),
        60 => "Value out of range or invalid. Parameter value outside valid range or inappropriate for context.".to_string(),
        61 => "Configuration failed. Error loading or applying configuration settings.".to_string(),
        62 => "Illegal home state. Attempting to home when already in a homed state or unsafe condition.".to_string(),
        63 => "Max travel exceeded. Motion would exceed maximum travel distance for axis.".to_string(),
        64 => "Max feed rate exceeded. Commanded feed rate exceeds maximum configured feed rate.".to_string(),
        65 => "Disabled. Feature or function is disabled in configuration.".to_string(),
        66 => "Password required. Command requires authentication but no password provided.".to_string(),
        67 => "Invalid password. Provided password is incorrect.".to_string(),
        68 => "Bluetooth initialization failed. Error initializing Bluetooth hardware or stack.".to_string(),
        69 => "Homing is required. Command cannot execute until machine is homed.".to_string(),
        70 => "Invalid plane selected. G17/G18/G19 plane selection conflict or invalid for command.".to_string(),
        71 => "Tool change required. M6 tool change command required before continuing.".to_string(),
        72 => "Not allowed. Command not permitted in current state or mode.".to_string(),
        73 => "Self-test failed. Controller hardware self-test reported failure.".to_string(),
        74 => "Busy. Controller busy processing previous command.".to_string(),
        75 => "Command requires single axis. Multi-axis movement not allowed for this command.".to_string(),

        _ => format!("Unknown error code: {}", code),
    }
}

/// Decode GRBL alarm code to human-readable message
pub fn decode_alarm(code: u8) -> String {
    match code {
        1 => "Hard limit triggered. Machine position is likely lost due to sudden and immediate halt. Re-homing is highly recommended.".to_string(),
        2 => "Soft limit: G-code motion target exceeds machine travel. Machine position safely retained. Alarm may be unlocked.".to_string(),
        3 => "Reset while in motion. Grbl cannot guarantee position. Lost steps are likely. Re-homing is highly recommended.".to_string(),
        4 => "Probe fail. The probe is not in the expected initial state before starting probe cycle, where G38.2 and G38.3 is not triggered and G38.4 and G38.5 is triggered.".to_string(),
        5 => "Probe fail. Probe did not contact the workpiece within the programmed travel for G38.2 and G38.4.".to_string(),
        6 => "Homing fail. Reset during active homing cycle.".to_string(),
        7 => "Homing fail. Safety door was opened during active homing cycle.".to_string(),
        8 => "Homing fail. Cycle failed to clear limit switch when pulling off. Try increasing pull-off setting or check wiring.".to_string(),
        9 => "Homing fail. Could not find limit switch within search distance. Defined as 1.5 * max_travel on search and 5 * pulloff on locate phases.".to_string(),

        // grblHAL Extended Alarm Codes (10-20)
        10 => "Limit switch engaged. Cannot complete homing cycle because limit switch is already triggered.".to_string(),
        11 => "Homing required. Machine must be homed before performing this operation.".to_string(),
        12 => "E-stop asserted. Emergency stop has been triggered.".to_string(),
        13 => "Motor fault. Stepper driver reported a fault condition.".to_string(),
        14 => "Homing configuration error. Invalid homing settings or configuration.".to_string(),
        15 => "Self-test failed. Controller hardware self-test reported failure during startup.".to_string(),
        16 => "Spindle at speed timeout. Spindle failed to reach commanded speed.".to_string(),
        17 => "Probe protection triggered. Probe circuit detected unsafe condition.".to_string(),
        18 => "Spindle sync error. Spindle synchronization lost during threading or rigid tapping.".to_string(),
        19 => "Power supply fault. Input power issue detected.".to_string(),
        20 => "Controller error. Internal controller error or malfunction.".to_string(),

        _ => format!("Unknown alarm code: {}", code),
    }
}

/// Format error message with code and description
pub fn format_error(code: u8) -> String {
    format!("error:{} - {}", code, decode_error(code))
}

/// Format alarm message with code and description
pub fn format_alarm(code: u8) -> String {
    format!("ALARM:{} - {}", code, decode_alarm(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_error() {
        assert_eq!(
            decode_error(1),
            "G-code words consist of a letter and a value. Letter was not found."
        );
        assert_eq!(
            decode_error(9),
            "G-code locked out during alarm or jog state."
        );
        assert_eq!(
            decode_error(53),
            "Invalid file number. File number specified does not exist or is invalid."
        );
        assert_eq!(
            decode_error(69),
            "Homing is required. Command cannot execute until machine is homed."
        );
        assert!(decode_error(255).contains("Unknown error code"));
    }

    #[test]
    fn test_decode_alarm() {
        assert!(decode_alarm(1).contains("Hard limit"));
        assert!(decode_alarm(2).contains("Soft limit"));
        assert!(decode_alarm(12).contains("E-stop"));
        assert!(decode_alarm(15).contains("Self-test failed"));
        assert!(decode_alarm(255).contains("Unknown alarm code"));
    }

    #[test]
    fn test_format_error() {
        let msg = format_error(1);
        assert!(msg.starts_with("error:1"));
        assert!(msg.contains("Letter was not found"));
    }

    #[test]
    fn test_format_alarm() {
        let msg = format_alarm(1);
        assert!(msg.starts_with("ALARM:1"));
        assert!(msg.contains("Hard limit"));
    }
}
