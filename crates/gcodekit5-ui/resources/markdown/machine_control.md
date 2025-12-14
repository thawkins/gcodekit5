# Machine Control

Machine Control is the primary place to connect to your controller and run jobs.

## Connection
1. Select the serial port.
2. Click **Connect**.
3. Watch the **Device Console** for startup messages.

If you have connection trouble:
- Verify permissions for the serial device (e.g. `/dev/ttyACM0`).
- Confirm the correct baud rate for your firmware.

## Jogging
- Use the on-screen jog pad.
- Use keyboard jogging (if enabled) for quick positioning.
- Set **Step (mm)** and **Jog Feed** to control movement.

## Homing / Unlock / Reset
- **Home** runs the firmware homing cycle (requires `$22=1` on GRBL).
- **Unlock** clears alarms if the controller is in Alarm state.
- **Reset** performs a soft reset.

## Streaming a job
1. Load or generate G-code.
2. Click **Send** to start streaming.
3. Use **Pause/Resume** as needed.
4. **Stop** aborts the stream.

## Screenshot
- [Machine Control screenshot (placeholder)](resource:///com/gcodekit5/help/images/placeholder.png)

## Related
- [Device Console](help:device_console)
- [Visualizer](help:visualizer)
- [Index](help:index)


