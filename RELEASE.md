## [0.55.0-alpha.1] - 2026-05-16

### Added
- **Milestone 8 — Settings & Configuration** (per 6AXISPLAN.md)
  - **New `Machine` settings category** in `SettingsCategory` enum for axis configuration
  - **Machine settings panel** with complete 6-axis configuration support:
    - Jog settings: Linear step size, rotary step size, jog feed rate
    - Axis limits: X, Y, Z limits (mm) and A, B, C limits (degrees)
    - Steps per degree: Configuration for A, B, C rotary axes
    - Direction inversion: Toggle for all 6 axes (X, Y, Z, A, B, C)
    - Rotary calibration: Offset values for A, B, C axes
  - **SettingsPersistence** extended with:
    - `add_machine_settings()`: Populates dialog with 6-axis machine settings
    - `update_machine_settings()`: Saves machine settings from dialog to config
  - **UI Settings panel** updated to include Machine tab
  - **SettingsController** updated with Machine category mapping
  - **ProbeSettings** fixed with proper `Default` implementation for validation
  - All 35 settings crate tests passing

### Added
- **Milestone 9 — Firmware Support** (per 6AXISPLAN.md)
  - **New capability flags** in `capabilities.rs`: `Axis4Support`, `Axis5Support`, `Axis6Support`
  - **GRBL 6-axis status report parsing**: Verified 3/4/6-axis position parsing in `MachinePosition`, `WorkPosition`, `WorkCoordinateOffset`
  - **grblHAL 6-axis support**: Already has `axes: 6` with `supports_axis()` for A/B/C
  - **FluidNC 6-axis support**: Already has `axes: 6` with full axis support
  - **TinyG 6-axis JSON parsing**: JSON responses already support 6 axes
  - **Smoothieware 6-axis support**: Already has axis support up to 5-6 axes
  - **g2core 6-axis support**: Native 6-axis with rotational axes support
  - **6-axis capability detection**: New `Capability` enum variants for axis support
  - **Unit tests** in `grbl/test.rs` covering:
    - 3/4/6-axis machine position parsing
    - 6-axis work position parsing
    - 6-axis work coordinate offset parsing
    - GRBL capabilities max_axes verification

### Changed
- Updated `6AXISPLAN.md`: Milestone 9 marked as complete with all tasks done
