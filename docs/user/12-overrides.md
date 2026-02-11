# Overrides

## Overview

Overrides let you adjust feed rate, spindle speed, and rapid speed in real-time while a program is running or during manual operations. Overrides are expressed as a percentage of the programmed value.

## Feed Rate Override

Adjusts the speed of cutting moves (G1, G2, G3).

| Control | Action |
|---------|--------|
| **+10%** | Increase feed rate by 10% |
| **+1%** | Increase feed rate by 1% |
| **-1%** | Decrease feed rate by 1% |
| **-10%** | Decrease feed rate by 10% |
| **Reset** | Return to 100% (programmed rate) |

**Range**: 0% to 200%

Use feed override to:
- Slow down when cutting sounds wrong or the machine is struggling
- Speed up when the cut quality is acceptable and you want to save time
- Fine-tune feed during a test run of a new program

## Spindle Speed Override

Adjusts the spindle RPM (or laser power in laser mode).

| Control | Action |
|---------|--------|
| **+10%** | Increase spindle speed by 10% |
| **+1%** | Increase spindle speed by 1% |
| **-1%** | Decrease spindle speed by 1% |
| **-10%** | Decrease spindle speed by 10% |
| **Reset** | Return to 100% (programmed speed) |

**Range**: 0% to 200%

## Rapid Override

Adjusts the speed of rapid positioning moves (G0). Available as fixed presets.

| Preset | Effect |
|--------|--------|
| **25%** | Quarter speed rapids — useful for cautious first runs |
| **50%** | Half speed rapids |
| **100%** | Full speed rapids (default) |

Rapid override is a safety feature. Reducing rapids gives you more time to react and hit E-Stop if the machine is heading somewhere unexpected.

## Tips

- Overrides take effect **immediately** — no need to pause the program
- The current override percentage is displayed in the overrides panel
- Overrides reset to 100% when you disconnect or send a soft reset
- During a first run of any new program, consider setting rapid override to 50%

## See Also

- [Machine Control](10-machine-control.md) — DRO, jogging, and homing
- [Streaming G-Code](80-streaming.md) — Running programs
