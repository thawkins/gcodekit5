#!/bin/bash
# Start grblHAL simulator with virtual TTY device
# This creates a virtual serial port at /dev/ttyGRBL

WORK_DIR="${HOME}/Projects/gcodekit5/target/temp"
STEP_FILE="${WORK_DIR}/grblhal-step.out"
BLOCK_FILE="${WORK_DIR}/grblhal-block.out"
TTY_DEVICE="${WORK_DIR}/ttyGRBL"

# Ensure temp directory exists
mkdir -p "${WORK_DIR}"

# Check if simulator is already running
if [ -e "${TTY_DEVICE}" ]; then
    echo "Virtual TTY device ${TTY_DEVICE} already exists."
    echo "The simulator may already be running."
    echo "To stop it, run: killall socat"
    exit 1
fi

# Check if grblHAL_sim is available
if ! command -v grblHAL_sim &> /dev/null; then
    echo "Error: grblHAL_sim not found in PATH"
    echo "Please ensure grblHAL simulator is installed."
    exit 1
fi

echo "Starting grblHAL simulator with virtual TTY at ${TTY_DEVICE}"
echo "Step output: ${STEP_FILE}"
echo "Block output: ${BLOCK_FILE}"
echo ""
echo "To stop the simulator, press Ctrl+C or run: killall socat"
echo ""

# Start socat to create virtual serial port connected to grblHAL simulator
# The simulator will run with no comment prefixes (-n), and output steps/blocks to files
socat PTY,raw,link=${TTY_DEVICE},echo=0,group-late=dialout,mode=660 \
    "EXEC:'grblHAL_sim -n -s ${STEP_FILE} -b ${BLOCK_FILE}',pty,raw,echo=0"
