# GCodeKit5 Toolchain Setup

This document describes how to set up your development environment with Rust and the Wild linker for optimal build performance.

## Prerequisites

- **Linux/macOS:**
  - `curl` or `wget` for downloading
  - Git for version control
  - `build-essential` (Linux) or Xcode Command Line Tools (macOS)
- **Windows:**
  - Windows 10 or 11
  - [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (Desktop development with C++ workload)
  - Git for Windows

## Installing Rust

GCodeKit5 requires Rust 1.70 or later. We recommend using `rustup` for managing your Rust installation.

### Install Rustup (Recommended)

**Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows:**
1. Download `rustup-init.exe` from [rustup.rs](https://rustup.rs).
2. Run the installer and follow the on-screen instructions.
3. Ensure you have the **Visual Studio Build Tools** installed (required for the MSVC toolchain).

Follow the on-screen prompts to complete the installation. After installation, add Rust to your PATH:

```bash
source $HOME/.cargo/env
```

### Verify Installation

```bash
rustc --version
cargo --version
```

You should see Rust 1.70 or later:

```
rustc 1.90.0 (...)
cargo 1.90.0 (...)
```

## Installing Tokei

Tokei is a fast and accurate code statistics tool used to count lines of code in the project. It is required for updating the project statistics.

### Install via Cargo

```bash
cargo install tokei
```

### Verify Installation

```bash
tokei --version
```

## Building GCodeKit5

Once your toolchain is set up, building is straightforward:

### Debug Build (Development)
```bash
cargo build
```

Build time should be significantly faster with the wild linker.

### Release Build (Production)
```bash
cargo build --release
```

### Running Tests
```bash
cargo test
```

### Checking Code (Fast)
```bash
cargo check
```

## Build Performance

The wild linker provides significant improvements:

- **Debug builds**: 2-3x faster linking
- **Release builds**: 1.5-2x faster linking
- **Incremental builds**: Even more dramatic improvements

## Troubleshooting

### Building for Different Targets

To build for a specific target (e.g., cross-compilation):

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --target x86_64-unknown-linux-musl
```

## System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| Rust | 1.70 | 1.90+ |
| LLD | 12.0 | 15.0+ |
| RAM | 4GB | 8GB+ |
| Storage | 5GB | 20GB+ (with caches) |
| CPU | 2 cores | 4+ cores |

## Additional Resources

- [Official Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
- [LLD Documentation](https://lld.llvm.org/)
- [Rustup Documentation](https://rust-lang.github.io/rustup/)

## Development Tips

### Speed Up Builds During Development

1. **Use `cargo check` instead of `cargo build`** when you only need to verify code compiles
2. **Use `cargo check --lib`** to skip integration tests
3. **Enable incremental compilation** (enabled by default in dev profile)
4. **Use `cargo build -p <crate>`** to build specific crates

### Optimize for Your System

Edit `.cargo/config.toml` and adjust `opt-level` and `lto` settings based on your hardware:

```toml
[profile.dev]
opt-level = 1           # 0-3, higher = slower compile but faster runtime
incremental = true      # Highly recommended for dev builds
```

## Getting Help

For issues related to:
- **Rust installation**: Visit [rustup.rs](https://rustup.rs)
- **GCodeKit5 builds**: See the main [README.md](README.md)

---

Last updated: 2025-11-24
