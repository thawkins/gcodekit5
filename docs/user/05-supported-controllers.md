# Supported Controllers

GCodeKit5 supports six CNC firmware types. Firmware is auto-detected on connection.

## GRBL

The most widely used open-source CNC firmware, commonly found on Arduino-based controllers.

- Character-counting streaming protocol
- Real-time status reports
- Feed and spindle overrides
- Jog commands (`$J=`)
- Probing support

## grblHAL

An enhanced GRBL implementation with extended capabilities.

- All GRBL features plus network connectivity
- Extended settings and configuration
- Additional axis support
- Enhanced probing cycles

## TinyG

A JSON-based motion control firmware with advanced features.

- JSON protocol for commands and responses
- Up to 6-axis support
- Jerk-controlled motion planning
- Probing support

## g2core

The successor to TinyG, providing advanced motion control.

- JSON command protocol
- Up to 6-axis support
- Advanced motion planning
- Probing support

## FluidNC

A modern, modular controller with WiFi and WebSocket support.

- GRBL-compatible command set
- WiFi and Ethernet connectivity (WebSocket/TCP)
- Modular pin configuration via YAML
- Probing support

## Smoothieware

A versatile controller firmware supporting CNC, laser, and 3D printing.

- RepRap-dialect G-code support
- Extensive M-code library
- Network connectivity on supported boards
- Probing support

## Feature Comparison

| Feature | GRBL | grblHAL | TinyG | g2core | FluidNC | Smoothieware |
|---------|------|---------|-------|--------|---------|--------------|
| Max Axes | 3–6 | 6 | 6 | 6 | 6 | 6 |
| Arcs (G2/G3) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Probing | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Overrides | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| WiFi | ❌ | ❌ | ❌ | ❌ | ✅ | ⚠️ Board-dependent |
| TCP/IP | ❌ | ✅ | ❌ | ❌ | ✅ | ⚠️ Board-dependent |
| WebSocket | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Protocol | Text | Text | JSON | JSON | Text | Text |
