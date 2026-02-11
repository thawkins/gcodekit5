# Troubleshooting Connections

## Common Issues

### Port Not Found

**Symptoms**: No serial ports appear in the dropdown.

**Solutions**:
1. Check that the USB cable is firmly connected at both ends
2. Try a different USB port on your computer
3. Verify the controller board is powered on
4. On Linux, check `dmesg | tail` for USB device detection messages
5. On Windows, check Device Manager for the COM port
6. Click **Refresh Ports** to re-scan

### Permission Denied (Linux)

**Symptoms**: Error when trying to connect on Linux.

**Solution**:
```bash
sudo usermod -aG dialout $USER
# Log out and log back in for the change to take effect
```

### Connection Timeout

**Symptoms**: Connection attempt hangs or times out.

**Solutions**:
1. Check that no other application is using the same port (close other senders)
2. Reset the controller by power-cycling it
3. Try a shorter or higher-quality USB cable
4. For TCP/IP connections, verify the IP address and port are correct
5. For WebSocket connections, ensure the controller's WiFi is active

### Garbled Characters in Console

**Symptoms**: Unreadable characters appear in the console after connecting.

**Solutions**:
1. Power-cycle the controller and reconnect
2. Verify you are connecting to the correct port
3. Check that the USB cable supports data (not charge-only)

### Alarm State After Connecting

**Symptoms**: Machine shows ALARM immediately after connecting.

**Solutions**:
1. Click the **Unlock** button to clear the alarm
2. Run a homing cycle if your firmware requires it
3. Check for triggered limit switches
4. See [Machine Control](10-machine-control.md) for alarm handling

### Auto-Detection Fails

**Symptoms**: Firmware type shows "Unknown" after connecting.

**Solutions**:
1. Ensure the controller is fully booted before connecting
2. Check the console for startup messages
3. Try disconnecting and reconnecting

## Diagnostic Steps

1. Open the **Console** panel and observe messages during connection
2. Enable debug logging: `RUST_LOG=debug ./target/release/gcodekit5`
3. Check the **Safety & Diagnostics** panel for communication health indicators
4. Verify the firmware responds to manual commands (type `$$` in the console for GRBL)

## Still Having Issues?

- [GitHub Issues](https://github.com/thawkins/gcodekit5/issues)
- [FAQ](92-faq.md)
