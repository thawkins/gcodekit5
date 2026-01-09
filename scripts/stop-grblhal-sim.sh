#!/bin/bash
# Stop the grblHAL simulator virtual TTY

echo "Stopping grblHAL simulator..."
killall socat 2>/dev/null

if [ $? -eq 0 ]; then
    echo "grblHAL simulator stopped."
    # Clean up the symlink if it still exists
    if [ -e "/dev/ttyGRBL" ]; then
        rm -f /dev/ttyGRBL 2>/dev/null || true
    fi
else
    echo "No grblHAL simulator process found."
fi
