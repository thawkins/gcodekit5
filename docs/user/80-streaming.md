# Streaming G-Code

## Overview

Streaming sends your G-code program to the CNC controller line by line, managing the communication buffer for smooth execution.

## Starting a Job

1. Load a G-code file in the [Editor](20-gcode-editor.md) or generate G-code from a [CAM Tool](50-camtools.md)
2. Ensure the machine is **connected** and in **Idle** state
3. Click **Send to Device** to begin streaming

## Job Controls

| Control | Shortcut | Action |
|---------|----------|--------|
| **Pause** | Space | Feed hold — machine decelerates to a stop, position retained |
| **Resume** | Space | Resume from pause point |
| **Stop** | Escape | Stop streaming and clear the queue |

## Progress Monitoring

While streaming, monitor progress through:

- **Status bar** — Shows percentage complete and current line number
- **Editor** — Current line highlighted with ▶, executed lines marked with ✓
- **Visualizer** — Executed toolpath highlighted, tool marker shows real-time position
- **File statistics** — Estimated remaining time

## Feed Rate and Spindle Overrides

Adjust [Overrides](12-overrides.md) in real-time during streaming to modify feed rate, spindle speed, or rapid speed without pausing the job.

## Safety During Streaming

- Keep your hand near the **E-Stop** (or Escape key) during cutting
- Start with **Rapid Override at 50%** for first runs of new programs
- Watch the [Safety & Diagnostics](85-safety-diagnostics.md) panel for communication issues
- Jogging is disabled while a program is running

## File Validation

Before streaming, GCodeKit5 calculates file statistics including:
- Move counts (G0 rapid, G1 linear, G2/G3 arc)
- M-code and comment counts
- Estimated run time

Review these to sanity-check your program before cutting.

## See Also

- [Machine Control](10-machine-control.md) — Manual control and DRO
- [G-Code Editor](20-gcode-editor.md) — Viewing and editing programs
- [Overrides](12-overrides.md) — Real-time speed adjustments
