# Quick Start Guide

Get up and running with GCodeKit5 in 5 minutes.

## Step 1: Launch the Application

```bash
./target/release/gcodekit5
# or during development:
cargo run --release
```

## Step 2: Connect to Your Machine

1. Click **Refresh Ports** to detect serial devices
2. Select your CNC controller's port from the dropdown
3. Click **Connect**

GCodeKit5 auto-detects the firmware type (GRBL, grblHAL, FluidNC, TinyG, g2core, or Smoothieware) and configures itself accordingly.

> **Tip**: For network-connected controllers (FluidNC WiFi), switch to TCP/IP or WebSocket connection mode in the connection panel. See [Device Setup](04-device-setup.md) for details.

## Step 3: Home Your Machine

1. Click the **Home** button (⌂) to run the homing cycle
2. Wait for the homing cycle to complete
3. The DRO (Digital Readout) will show your machine position

## Step 4: Jog to Test

1. Select a step size (1 mm recommended to start)
2. Use the jog buttons or keyboard shortcuts:
   - **W** / **S** — Y+ / Y-
   - **A** / **D** — X- / X+
   - **Q** / **Z** — Z+ / Z-
3. Verify the machine moves as expected

## Step 5: Load G-Code

1. **File → Open** or drag-and-drop a `.nc` / `.gcode` file
2. The code appears in the G-Code Editor with syntax highlighting
3. The toolpath displays in the 3D Visualizer

## Step 6: Run Your Program

1. Click **Send to Device** to start streaming
2. Monitor progress in the status bar and visualizer
3. Use **Space** to pause or **Escape** to stop

## What's Next?

- [Machine Control](10-machine-control.md) — DRO, jogging, homing, and WCS
- [G-Code Editor](20-gcode-editor.md) — Edit your programs
- [Toolpath Visualizer](30-visualizer.md) — Preview before cutting
- [CAM Tools](50-camtools.md) — Generate G-code from built-in tools
- [Designer](40-designer.md) — Design parts from scratch
