# Device Console

The Device Console shows raw lines received from the controller and messages sent by GCodeKit5.

## What you will see
- Connection events (connected/disconnected)
- Startup strings (e.g. `Grbl 1.2h ...`)
- Status lines, responses (`ok`), and errors (`error:n`)

## Errors
When a GRBL line includes `error:n`, the console will also show the decoded meaning after the raw text.

## Tips
- Use **Copy** to copy the visible console content.
- Use **Clear** to clear the console view.

## Related
- [Machine Control](help:machine_control)
- [Device Config](help:device_config)
- [Index](help:index)
