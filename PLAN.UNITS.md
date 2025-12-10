# Plan: Switchable Units (Metric/Imperial)

This plan outlines the steps to implement switchable measurement units in the GCodeKit5 application. The system will internally use millimeters (mm) for consistency but allow the user to view and interact with the application in either Metric (mm) or Imperial (inches) units based on their preference.

## Specifications
- **Internal Units**: Floating point millimeters (mm).
- **Display Units**:
    - **Metric**: Millimeters, rounded to 2 decimal places (e.g., `100.00`).
    - **Imperial**: Inches, rounded to 3 decimal places (e.g., `3.937`).
- **Feed Rates**: Support display in units/min or units/sec.
- **Preference**: Use `measurement_system` from the Settings ("Metric" or "Imperial").
- **Jog Steps**: Context-aware step sizes (e.g., 0.1mm vs 0.01in).
- **UI Annotation**: All relevant fields MUST show their units explicitly (e.g., "X: 100.00 mm", "Feed: 500 in/min").
- **Inputs**: Any numeric input field (e.g. for moves, settings) inside the `MachineControlView` must interpret the value in the *currently selected unit system* and convert to mm before sending commands.

## Tasks

### 1. Create Unit Conversion Helper
- [ ] **Implement `UnitUtils` in `crates/gcodekit5-ui/src/helpers.rs`**.
    - Define enum `MeasurementSystem { Metric, Imperial }`.
    - Create struct `UnitManager` or static helper functions.
    - Implement `format_length(value_mm: f64, system: MeasurementSystem) -> String`.
        - Metric: `format!("{:.2}", value_mm)`
        - Imperial: `format!("{:.3}", value_mm / 25.4)`
    - Implement `format_feedrate(value_mm_per_min: f64, system: MeasurementSystem) -> String`.
        - Metric: Returns "mm/min"
        - Imperial: Returns "in/min" (converted values)
    - Implement `get_unit_label(system: MeasurementSystem) -> &'static str`.
        - Returns "mm" or "in".
    - Implement `to_mm(value: f64, system: MeasurementSystem) -> f64` (for inputs).
    - Implement `from_mm(value: f64, system: MeasurementSystem) -> f64` (for display).

### 2. Update Machine Control View
- [ ] **Modify `MachineControlView` struct in `crates/gcodekit5-ui/src/ui/gtk/machine_control.rs`**.
    - Add a field to track the current `MeasurementSystem`.
    - Add a method `set_measurement_system(&self, system: MeasurementSystem)` to update the view state.
- [ ] **Refactor DRO Updates**.
    - Update DRO labels to include unit suffixes.
    - `StatusParser` processing logic: Convert mpos/wpos from mm (internal) to display value using `UnitUtils::from_mm`.
    - Update the DRO value label text using `UnitUtils::format_length` AND append `UnitUtils::get_unit_label`.
- [ ] **Refactor Jog Controls**.
    - Change step size button labels dynamically in `set_measurement_system`.
    - **Metric Steps**: "0.1 mm", "1.0 mm", "10 mm", "50 mm".
    - **Imperial Steps**: "0.001 in", "0.01 in", "0.1 in", "1.0 in".
    - Update logic to handle conversions: The clicked value is in user units -> convert `to_mm` -> send G91 move.
- [ ] **Refactor Inputs**.
    - Identify any entry fields used for manual moves or parameter setting (e.g., Console, specific coordinate inputs if any).
    - Ensure inputs are parsed as the current unit system, validated, and converted to mm before use.

### 3. Update Status Bar
- [ ] **Modify `StatusBar` in `crates/gcodekit5-ui/src/ui/gtk/status_bar.rs`**.
    - Add `set_measurement_system` method.
    - Update `set_position` to display units (e.g. `X: 0.00 mm`).
    - Update `set_feed` to display feed units (e.g. `F: 1000 mm/min` or `40 in/min`).
    
### 4. Wire Up Preferences
- [ ] **Connect Settings to UI in `gtk_app.rs`**.
    - Initialize `MachineControlView` with unit preference.
    - Connect signals so that changing the preference immediately calls `set_measurement_system`.

### 5. Verification
- [ ] **Visual Check**: Verify "mm", "in", "mm/min", "in/min" labels appear correctly on DROs, Status Bar, and Jog buttons.
- [ ] **Functional Check**: Verify entering "1" in imperial mode moves machine 25.4mm.
- [ ] **Display Check**: Verify position reported as 25.4mm displays as "1.000 in" in Imperial mode.
