# Device Setup

## Connection Types

GCodeKit5 supports three ways to connect to your CNC controller:

### Serial / USB

The most common connection method for GRBL and similar controllers.

1. Connect your CNC controller via USB
2. Click **Refresh Ports** to detect the device
3. Select the correct port from the dropdown (e.g., `/dev/ttyUSB0` on Linux, `COM3` on Windows)
4. Click **Connect**

GCodeKit5 auto-detects the firmware type after connection.

> **Linux users**: You may need to add your user to the `dialout` group for serial port access. See [Troubleshooting](06-connection-troubleshooting.md).

### TCP/IP

For network-connected controllers (e.g., FluidNC with Ethernet, or controllers behind a serial-to-TCP bridge).

1. Switch the connection mode to **TCP/IP**
2. Enter the controller's IP address (e.g., `192.168.1.100`)
3. Enter the port number (e.g., `8080`)
4. Click **Connect**

### WebSocket

For WiFi-enabled controllers such as FluidNC.

1. Switch the connection mode to **WebSocket**
2. Enter the WebSocket URL (e.g., `ws://192.168.1.100:8080`)
3. Click **Connect**

## Auto-Reconnect

Enable **Auto-Reconnect** to automatically restore the connection if it drops. The default reconnect timeout is 10 seconds and can be adjusted in [Settings](70-settings.md).

## Verifying Connection

After connecting, you should see:

- **Machine state** in the status bar (Idle, Alarm, Run, Hold, etc.)
- **Position coordinates** in the DRO (Digital Readout)
- **Firmware identification** (e.g., "Grbl 1.1h", "FluidNC v3.x")
- **Startup messages** in the Console panel

## Device Profiles

Create profiles for different machines to quickly switch between configurations. Each profile stores the connection method, port, and firmware preferences.

## Troubleshooting

See [Troubleshooting Connections](06-connection-troubleshooting.md) for common issues.
