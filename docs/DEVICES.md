# Device Profile Management Analysis

## Overview

This document outlines the design and implementation plan for a Device Profile Management system in GCodeKit5. This system will allow users to define, manage, and select different machine configurations (e.g., "My Laser Cutter", "Desktop CNC", "3D Printer") with specific workspace dimensions, controller types, and capabilities.

## Requirements

1.  **CRUD Operations**: Create, Read, Update, Delete named device profiles.
2.  **Properties**:
    *   **Name**: User-friendly identifier.
    *   **Device Type**: Laser, CNC Mill, CNC Lathe, 3D Printer.
    *   **Controller Type**: GRBL, TinyG, g2core, Smoothieware, FluidNC.
    *   **Workspace Dimensions**: Min/Max limits for X, Y, Z (and potentially A/B/C) axes in millimeters.
    *   **Connection Defaults**: (Optional) Default baud rate/port for this device.
3.  **Selection**: Ability to select the "Active Device" from the settings.
4.  **Persistence**: Save profiles to disk (likely `config.json` or `devices.json`).

## Data Model

We will define these structures in `crates/gcodekit5-settings/src/devices.rs`.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    CncMill,
    CncLathe,
    LaserCutter,
    ThreeDPrinter,
    Plotter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ControllerType {
    Grbl,
    TinyG,
    G2Core,
    Smoothieware,
    FluidNC,
    Marlin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisLimits {
    pub min: f64,
    pub max: f64,
    pub enabled: bool,
}

impl Default for AxisLimits {
    fn default() -> Self {
        Self { min: 0.0, max: 200.0, enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub id: String, // UUID
    pub name: String,
    pub description: String,
    pub device_type: DeviceType,
    pub controller_type: ControllerType,
    
    // Workspace Limits
    pub x_axis: AxisLimits,
    pub y_axis: AxisLimits,
    pub z_axis: AxisLimits,
    pub a_axis: AxisLimits, // Rotary/Aux
    
    // Capabilities
    pub has_spindle: bool,
    pub has_laser: bool,
    pub has_coolant: bool,
    pub max_feed_rate: f64,
    
    // Power
    pub cnc_spindle_watts: f64,
    pub laser_watts: f64,
}
```

## UI Design

The legacy Slint UI has been removed; the UI is now implemented in the crate's GTK-based UI under `crates/gcodekit5-devicedb/ui/device_manager` and integrated into the main application as a dedicated tab.

### Layout

1.  **Tab View**: Added a new "Device Manager" tab to the main window.
2.  **Master-Detail View**:
    *   **Left Pane (List)**: List of defined devices.
        *   "Add Device" button at the bottom.
    *   **Right Pane (Details)**: Form to edit the selected device.
        *   Name & Description fields.
        *   Dropdowns for Device Type and Controller Type.
        *   **Tabbed View for Dimensions**:
            *   Tab 1: **General** (Type, Controller, Name)
            *   Tab 2: **Dimensions** (X, Y, Z min/max inputs)
            *   Tab 3: **Capabilities** (Checkboxes for Spindle, Laser, etc., and Power Settings)
        *   "Save", "Delete", and "Set as Active" buttons.

### Active Device Selection

*   The "Set as Active" button in the Device Manager sets the current profile.
*   Changing this setting updates the global application state.

## Integration Plan

### Phase 1: Backend (`gcodekit5-devicedb`)
1.  Created `gcodekit5-devicedb` crate.
2.  Implemented `DeviceManager` and `DeviceProfile` models.
3.  Implemented persistence to `devices.json`.

### Phase 2: UI Implementation (`gcodekit5-devicedb` & `gcodekit5-ui`)
1.  Created `DeviceManagerPanel` UI component in Slint.
2.  Integrated into `MainWindow` in `ui.slint`.
3.  Bound UI events to `DeviceUiController` in `main.rs`.

### Phase 3: Application Logic (`main.rs`)
1.  On startup, load device profiles.
2.  Initialize `DeviceUiController` and bind callbacks.

## Future Considerations

*   **Export/Import**: Share device profiles as JSON files.
*   **Firmware Sync**: Button to "Read Settings from Controller" (e.g., `$130`, `$131`, `$132` in GRBL) to auto-populate workspace dimensions.
*   **Tool Library Association**: Link specific tool libraries to specific devices.

## Public Interfaces

The `gcodekit5-devicedb` crate exposes the following public interfaces:

- `DeviceManager`: The main struct for managing device profiles.
- `DeviceProfileProvider`: A trait for retrieving device profiles, useful for decoupling.
- `DeviceUiController`: A controller for the Slint UI.
- `DeviceProfileUiModel`: The UI model for a device profile.

### DeviceProfileProvider Trait

```rust
pub trait DeviceProfileProvider: Send + Sync {
    fn get_active_profile(&self) -> Option<DeviceProfile>;
    fn get_profile(&self, id: &str) -> Option<DeviceProfile>;
}
```

This trait allows other parts of the application to access device profiles without depending on the concrete `DeviceManager` implementation details.
