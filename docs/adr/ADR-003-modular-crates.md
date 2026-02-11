# ADR-003: Modular Crates Structure

## Status
Accepted

## Context

As GCodeKit5 grew, the single-crate structure became problematic:

- **Compile Times**: Full rebuilds took several minutes due to the large codebase
- **Code Organization**: Related functionality was scattered across files
- **Testing**: Integration tests were slow and coupled
- **Reusability**: No way to use core functionality in other projects
- **Cognitive Load**: New contributors struggled to understand the codebase

## Decision

We restructured GCodeKit5 as a **Cargo workspace** with multiple focused crates:

```
gcodekit5/
├── Cargo.toml           # Workspace root
├── src/                 # Main binary crate
└── crates/
    ├── gcodekit5-core/          # Core types, traits, events
    ├── gcodekit5-communication/ # Serial, firmware protocols
    ├── gcodekit5-gcodeeditor/   # G-code editing, buffer
    ├── gcodekit5-visualizer/    # 2D/3D toolpath rendering
    ├── gcodekit5-designer/      # CAD/CAM design tools
    ├── gcodekit5-camtools/      # CAM operations
    ├── gcodekit5-devicedb/      # Device profile management
    ├── gcodekit5-settings/      # Application settings
    └── gcodekit5-ui/            # GTK4 UI components
```

### Crate Responsibilities

| Crate | Responsibility | Dependencies |
|-------|----------------|--------------|
| `core` | Types, traits, state, events | Minimal (serde, thiserror) |
| `communication` | Serial ports, firmware protocols | core |
| `gcodeeditor` | Text editing, syntax highlighting | core |
| `visualizer` | Toolpath rendering, viewport | core |
| `designer` | Shape drawing, toolpath generation | core, visualizer |
| `camtools` | Box maker, puzzle, engraver | core |
| `devicedb` | Device profiles, persistence | core |
| `settings` | App configuration | core |
| `ui` | GTK4 widgets, main window | All above |

## Consequences

### Positive
- **Faster Incremental Builds**: Only changed crates recompile (~10x faster)
- **Clear Boundaries**: Each crate has defined responsibilities
- **Independent Testing**: Unit tests run per-crate in parallel
- **Reusability**: Core crates could be used in CLI tools or other UIs
- **Onboarding**: New contributors can focus on one crate

### Negative
- **Initial Setup Complexity**: More Cargo.toml files to maintain
- **Cross-Crate Changes**: Features spanning crates need coordinated updates
- **Version Management**: All crates share a version (workspace inheritance)
- **Circular Dependencies**: Must carefully design to avoid cycles

### Neutral
- Public API surface increases (more `pub` items)
- Documentation must cover inter-crate relationships

## Alternatives Considered

1. **Single Crate with Modules**: Simpler but doesn't solve compile time issues.

2. **Feature Flags**: Could conditionally compile parts, but adds complexity and doesn't improve organization.

3. **Separate Repositories**: Maximum isolation but coordination overhead and harder to make cross-cutting changes.

## References

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Large Rust Codebase Patterns](https://matklad.github.io/2021/09/05/Rust-100k.html)
