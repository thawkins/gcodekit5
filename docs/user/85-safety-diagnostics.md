# Safety & Diagnostics

## Overview

The Safety & Diagnostics panel provides real-time monitoring of safety systems and communication health.

## Emergency Stop (E-Stop)

The E-Stop system has four states:

| State | Meaning |
|-------|---------|
| **Armed** | Normal operating state — E-Stop is ready |
| **Triggered** | E-Stop activated — all motion immediately halted |
| **Resetting** | Recovering from a triggered state |
| **Stopped** | Machine is fully stopped and locked |

**Trigger E-Stop**: Press **Escape** or click the E-Stop button. This immediately halts all motion and puts the machine into alarm state.

**Recovery**: Click **Reset/Unlock** to clear the alarm and return to Armed state. You may need to re-home the machine after an E-Stop.

## Motion Interlock

The motion interlock prevents accidental machine movement:

| State | Meaning |
|-------|---------|
| **Enabled** | Motion commands are allowed |
| **Disabled** | All motion commands are blocked |
| **Waiting** | Interlock is waiting for confirmation |

## Feed Hold

Feed hold pauses program execution while retaining the machine position for a clean resume:

| State | Meaning |
|-------|---------|
| **Normal** | Program running normally |
| **Held** | Execution paused — machine stopped at current position |
| **Resuming** | Resuming from held state |

Press **Space** to toggle feed hold during streaming.

## Communication Diagnostics

Monitor the health of the connection to your controller:

| Metric | Description |
|--------|-------------|
| **Bytes Sent/Received** | Total data transferred |
| **Port** | Current serial port or network address |
| **Connection Status** | Connected, disconnected, or reconnecting |
| **Last Error** | Most recent communication error |
| **Error Count (1 min)** | Errors in the last rolling minute |

## Buffer Diagnostics

Monitor the controller's command buffer:

| Metric | Description |
|--------|-------------|
| **Buffer Size** | Total buffer capacity |
| **Buffer Used** | Current fill percentage |
| **Peak Usage** | Highest fill percentage observed |
| **Commands Pending** | Number of commands waiting in the buffer |
| **Overflow Count** | Number of buffer overflow events |

## Performance Diagnostics

| Metric | Description |
|--------|-------------|
| **Commands/sec** | Throughput of commands sent to the controller |
| **Avg Latency** | Average round-trip time in milliseconds |
| **Max Latency** | Peak round-trip time |
| **Memory Usage** | Application memory consumption |
| **CPU Usage** | Application CPU utilization |
| **Uptime** | Time since application start |

## Soft Limits

Soft limits prevent the machine from moving beyond its configured travel:

- Per-axis min/max boundaries (in mm)
- Enable or disable globally without losing the configured values
- Feedback shown when a command would exceed limits

Configure soft limits in the **Advanced Features** section of the settings or directly in the firmware (`$20`, `$130`–`$132` for GRBL).

## See Also

- [Machine Control](10-machine-control.md) — DRO, jogging, and alarm handling
- [Streaming G-Code](80-streaming.md) — Running programs safely
- [Console](15-console.md) — Device communication log
