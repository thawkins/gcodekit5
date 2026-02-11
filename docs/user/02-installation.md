# Installation

## Prerequisites

- **Rust toolchain** 1.88 or later (install from [rustup.rs](https://rustup.rs))
- **GTK4** and **libadwaita** development libraries
- **OpenGL 3.3+** capable graphics driver

## Linux

### Dependencies

```bash
# Fedora
sudo dnf install gtk4-devel libadwaita-devel

# Ubuntu / Debian
sudo apt install libgtk-4-dev libadwaita-1-dev

# Arch Linux
sudo pacman -S gtk4 libadwaita
```

### Build from Source

```bash
git clone https://github.com/thawkins/gcodekit5.git
cd gcodekit5
cargo build --release
./target/release/gcodekit5
```

### Serial Port Permissions

To access serial devices without root:

```bash
sudo usermod -aG dialout $USER
# Log out and log back in for the change to take effect
```

### Flatpak

```bash
# When available on Flathub
flatpak install flathub com.github.thawkins.gcodekit5
```

## macOS

See [macOS Build Guide](../MACOS_BUILD.md) for detailed instructions.

```bash
brew install gtk4 libadwaita
git clone https://github.com/thawkins/gcodekit5.git
cd gcodekit5
cargo build --release
```

## Windows

See [Windows Build Guide](../WINDOWS_BUILD.md) for detailed instructions.

## Verifying Installation

After building, launch the application:

```bash
./target/release/gcodekit5
```

You should see the main GCodeKit5 window with the connection panel, DRO, and console.

## Next Steps

- [Quick Start](03-quick-start.md) — Get running in 5 minutes
- [Device Setup](04-device-setup.md) — Connect your CNC machine
