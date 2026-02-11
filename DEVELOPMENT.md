# Developer Setup Guide

This guide will help you set up a development environment for GCodeKit5.

## Prerequisites

### Required Software

| Software | Minimum Version | Purpose |
|----------|-----------------|---------|
| **Rust** | 1.70+ | Compiler and cargo |
| **GTK4** | 4.6+ | UI framework |
| **libadwaita** | 1.2+ | Modern GTK widgets |
| **pkg-config** | - | Library discovery |
| **Git** | 2.0+ | Version control |

### Platform-Specific Setup

#### Linux (Fedora/RHEL)

```bash
# Install development tools
sudo dnf groupinstall "Development Tools"

# Install GTK4 and dependencies
sudo dnf install gtk4-devel libadwaita-devel \
    glib2-devel cairo-devel pango-devel \
    gdk-pixbuf2-devel graphene-devel \
    openssl-devel sqlite-devel pkg-config

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### Linux (Ubuntu/Debian)

```bash
# Install development tools
sudo apt update
sudo apt install build-essential

# Install GTK4 and dependencies
sudo apt install libgtk-4-dev libadwaita-1-dev \
    libglib2.0-dev libcairo2-dev libpango1.0-dev \
    libgdk-pixbuf-2.0-dev libgraphene-1.0-dev \
    libssl-dev libsqlite3-dev pkg-config

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### macOS

```bash
# Install Homebrew (if not installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install GTK4 and dependencies
brew install gtk4 libadwaita pkg-config

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

See [docs/MACOS_BUILD.md](docs/MACOS_BUILD.md) for detailed macOS instructions.

#### Windows

See [docs/WINDOWS_BUILD.md](docs/WINDOWS_BUILD.md) for Windows setup using MSYS2.

## Clone and Build

### 1. Clone the Repository

```bash
git clone https://github.com/thawkins/gcodekit5.git
cd gcodekit5
```

### 2. Configure Git Hooks

```bash
# Enable pre-commit hooks for code quality
git config core.hooksPath .githooks
```

### 3. Build the Project

```bash
# Debug build (faster compilation, includes debug symbols)
cargo build

# Release build (optimized, slower to compile)
cargo build --release
```

**Note**: First build downloads dependencies and may take 5-10 minutes. Subsequent builds are much faster due to incremental compilation.

### 4. Verify the Build

```bash
# Check that the binary runs
cargo run --release -- --version
```

## Running the Application

### Development Mode

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with trace logging (very verbose)
RUST_LOG=trace cargo run

# Run specific module logging
RUST_LOG=gcodekit5_communication=debug cargo run
```

### Release Mode

```bash
cargo run --release
```

### Using the Development Container

If you have Podman installed, you can use the development container:

```bash
# Build and enter the container
podman build -t gcodekit5-dev .devcontainer/
podman run -it --rm \
    --userns=keep-id \
    --security-opt label=disable \
    -v "$(pwd):/workspace:Z" \
    gcodekit5-dev

# Inside container
cargo build
cargo run
```

Or open in VS Code with the Dev Containers extension (set `"dev.containers.dockerPath": "podman"` in settings).

## Running Tests

### All Tests

```bash
# Run all tests
cargo test

# Run with output visible
cargo test -- --nocapture

# Run with longer timeout (some tests are slow)
timeout 600 cargo test
```

### Specific Tests

```bash
# Run tests for a specific crate
cargo test -p gcodekit5-core

# Run a specific test by name
cargo test test_grbl_status_parser

# Run tests matching a pattern
cargo test gcode
```

### Test Categories

```bash
# Library tests only (skip integration tests)
cargo test --lib

# Integration tests only
cargo test --test '*'

# Doc tests only
cargo test --doc
```

## Code Quality

### Formatting

```bash
# Format all code
cargo fmt

# Check formatting without changes
cargo fmt --check
```

### Linting

```bash
# Run Clippy linter
cargo clippy

# Clippy with all warnings as errors
cargo clippy -- -D warnings

# Fix auto-fixable issues
cargo clippy --fix
```

### Pre-Commit Checks

The git hooks run automatically, but you can run them manually:

```bash
# Run the same checks as pre-commit
cargo fmt --check && cargo clippy && cargo test --lib
```

## Project Structure

```
gcodekit5/
├── Cargo.toml              # Workspace root
├── src/                    # Main binary
│   └── main.rs
├── crates/                 # Library crates
│   ├── gcodekit5-core/     # Core types, traits, events
│   ├── gcodekit5-communication/  # Serial, firmware protocols
│   ├── gcodekit5-gcodeeditor/    # G-code text editing
│   ├── gcodekit5-visualizer/     # 2D/3D rendering
│   ├── gcodekit5-designer/       # CAD/CAM tools
│   ├── gcodekit5-camtools/       # Box maker, puzzle, etc.
│   ├── gcodekit5-devicedb/       # Device profiles
│   ├── gcodekit5-settings/       # App configuration
│   └── gcodekit5-ui/             # GTK4 UI components
├── docs/                   # Documentation
├── assets/                 # Icons, images
├── flatpak/                # Flatpak packaging
└── scripts/                # Build/utility scripts
```

## Debugging Tips

### Enable Rust Backtraces

```bash
RUST_BACKTRACE=1 cargo run
RUST_BACKTRACE=full cargo run  # Full backtrace with source locations
```

### Debug Logging Levels

```bash
# Levels: error, warn, info, debug, trace
RUST_LOG=debug cargo run

# Multiple modules with different levels
RUST_LOG=gcodekit5=debug,gcodekit5_communication=trace cargo run

# All modules at debug except noisy ones
RUST_LOG=debug,hyper=warn cargo run
```

### VS Code Debugging

1. Install the "CodeLLDB" extension
2. Create/update `.vscode/launch.json`:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug GCodeKit5",
            "cargo": {
                "args": ["build", "--bin=gcodekit5", "--package=gcodekit5"],
                "filter": {
                    "name": "gcodekit5",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}",
            "env": {
                "RUST_LOG": "debug",
                "RUST_BACKTRACE": "1"
            }
        }
    ]
}
```

3. Set breakpoints and press F5 to debug

### GDB/LLDB Command Line

```bash
# Build debug binary
cargo build

# Debug with GDB
gdb target/debug/gcodekit5

# Debug with LLDB
lldb target/debug/gcodekit5
```

### GTK Inspector

Enable the GTK Inspector to debug UI:

```bash
GTK_DEBUG=interactive cargo run
```

Press Ctrl+Shift+I in the running application to open the inspector.

## Common Issues

### Build Fails: Missing GTK4

**Error**: `Package gtk4 was not found`

**Solution**: Install GTK4 development packages (see Prerequisites above)

```bash
# Fedora
sudo dnf install gtk4-devel

# Ubuntu
sudo apt install libgtk-4-dev
```

### Build Fails: pkg-config Not Found

**Error**: `pkg-config: command not found`

**Solution**:
```bash
# Fedora
sudo dnf install pkg-config

# Ubuntu
sudo apt install pkg-config

# macOS
brew install pkg-config
```

### Linker Errors on macOS

**Error**: Various linker errors about missing symbols

**Solution**: Ensure PKG_CONFIG_PATH is set:
```bash
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:$PKG_CONFIG_PATH"
```

Add to your `~/.zshrc` or `~/.bashrc` for persistence.

### Tests Timeout

**Error**: Tests hang or timeout

**Solution**: Some tests may need more time:
```bash
# Increase timeout
timeout 600 cargo test

# Or skip slow tests during development
cargo test --lib  # Only unit tests, skip integration
```

### Serial Port Permission Denied (Linux)

**Error**: `Permission denied` when connecting to CNC controller

**Solution**: Add your user to the dialout group:
```bash
sudo usermod -aG dialout $USER
# Log out and back in for changes to take effect
```

### UI Looks Wrong / Missing Styles

**Error**: Application appears unstyled or broken

**Solution**: Ensure libadwaita is installed and you're running in a GTK-compatible desktop:
```bash
# Check libadwaita
pkg-config --modversion libadwaita-1

# Force a theme (if needed)
GTK_THEME=Adwaita:dark cargo run
```

### Flatpak Build Fails

**Error**: Flatpak manifest errors

**Solution**: Install flatpak-builder:
```bash
# Fedora
sudo dnf install flatpak-builder

# Ubuntu
sudo apt install flatpak-builder
```

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/thawkins/gcodekit5/issues)
- **Documentation**: See `docs/` folder
- **Architecture**: See [docs/adr/](docs/adr/) for design decisions
- **Coordinate System**: See [docs/COORDINATE_SYSTEM.md](docs/COORDINATE_SYSTEM.md)

## Next Steps

1. Read the [Architecture Decision Records](docs/adr/) to understand design choices
2. Explore the crate structure in `crates/`
3. Run the application and explore the UI
4. Pick an issue labeled "good first issue" to start contributing
