# G-Code Editor

## Overview

The G-Code Editor provides a full-featured text editor for viewing and modifying G-code files.

## Opening Files

- **File → Open** or **Ctrl+O** to open a G-code file
- Supported extensions: `.nc`, `.gcode`, `.ngc`, `.gco`
- Large files (100k+ lines) load with a progress indicator

## Syntax Highlighting

G-code elements are color-coded for readability:

| Element | Examples |
|---------|----------|
| **G-codes** | G0, G1, G2, G3, G28, G54 |
| **M-codes** | M3, M5, M8, M9, M30 |
| **Coordinates** | X, Y, Z, A, B, C values |
| **Parameters** | F (feed), S (speed), T (tool), P, R |
| **Comments** | Lines starting with `;` or text in `(parentheses)` |

## Search and Replace

Open search with **Ctrl+F**:

- **Find**: Enter text and navigate matches with Find Next / Find Previous
- **Replace**: Replace single occurrences or all at once
- **Case sensitive**: Toggle case-sensitive matching
- **Match count**: Total number of matches displayed

## File Statistics

When a file is loaded, the editor calculates:

- Total line count
- Rapid moves (G0), linear moves (G1), arc moves (G2/G3)
- M-code and comment counts
- Estimated run time (formatted as hours:minutes:seconds)

## Execution Tracking

While streaming a program to the machine:

- The **current line** is marked with a ▶ indicator
- **Executed lines** are marked with ✓
- The editor **auto-scrolls** to keep the current line visible

## Saving Files

- **Ctrl+S** — Save to the current file
- **Ctrl+Shift+S** — Save As with a new filename
- Unsaved changes are indicated in the title bar; you are prompted to save when closing

## See Also

- [Streaming G-Code](80-streaming.md) — Sending programs to the machine
- [Keyboard Shortcuts](90-shortcuts.md) — Editor shortcut reference
