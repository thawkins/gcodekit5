#!/bin/bash
# Test the grblHAL simulator virtual TTY
# This demonstrates how to communicate with the simulator

TTY_DEVICE="${HOME}/Projects/gcodekit5/target/temp/ttyGRBL"

# Check if simulator is running
if [ ! -e "${TTY_DEVICE}" ]; then
    echo "Error: grblHAL simulator not running"
    echo "Start it with: ./scripts/start-grblhal-sim.sh"
    exit 1
fi

echo "Testing grblHAL simulator at ${TTY_DEVICE}"
echo ""

# Function to send command and read response
send_command() {
    local cmd="$1"
    echo "Sending: $cmd"
    echo "$cmd" > "${TTY_DEVICE}"
    sleep 0.3
    timeout 1 cat "${TTY_DEVICE}" 2>/dev/null | head -10
    echo ""
}

# Test various grblHAL commands
send_command "?"           # Status query
send_command "\$\$"        # View settings
send_command "\$I"         # Build info
send_command "\$G"         # View parser state

echo "Test complete!"
echo ""
echo "Your gcodekit5 application can connect to: ${TTY_DEVICE}"
