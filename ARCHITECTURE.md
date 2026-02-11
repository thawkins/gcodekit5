# GCodeKit5 Architecture Overview

This document describes the high-level architecture of GCodeKit5, including crate dependencies, data flow, design patterns, and threading model.

## Table of Contents

- [Crate Structure](#crate-structure)
- [Crate Dependency Graph](#crate-dependency-graph)
- [Data Flow](#data-flow)
- [Key Design Patterns](#key-design-patterns)
- [Threading Model](#threading-model)
- [Error Handling Strategy](#error-handling-strategy)
- [Related Documentation](#related-documentation)

## Crate Structure

GCodeKit5 is organized as a Cargo workspace with 9 library crates and 1 binary crate:

```
gcodekit5/
├── src/main.rs              # Binary entry point
└── crates/
    ├── gcodekit5-core/      # Foundation types and traits
    ├── gcodekit5-communication/  # Device protocols
    ├── gcodekit5-gcodeeditor/    # Text editing
    ├── gcodekit5-visualizer/     # Toolpath rendering
    ├── gcodekit5-designer/       # CAD/CAM design
    ├── gcodekit5-camtools/       # CAM operations
    ├── gcodekit5-devicedb/       # Device profiles
    ├── gcodekit5-settings/       # Configuration
    └── gcodekit5-ui/             # GTK4 interface
```

### Crate Responsibilities

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| **core** | Shared types, traits, events, state | `MachineState`, `Position`, `GCodeLine`, `AppEvent` |
| **communication** | Serial/TCP/WebSocket protocols, firmware implementations | `Connection`, `GrblController`, `Protocol` |
| **gcodeeditor** | G-code text buffer, syntax highlighting, undo/redo | `GCodeBuffer`, `SyntaxHighlighter` |
| **visualizer** | 2D/3D toolpath rendering, viewport transforms | `ViewportTransform`, `ToolpathRenderer` |
| **designer** | Shape drawing, toolpath generation, CAD operations | `Shape`, `DesignerState`, `ToolpathGenerator` |
| **camtools** | Box maker, puzzle generator, image engraver | `TabbedBoxMaker`, `JigsawPuzzle`, `LaserEngraver` |
| **devicedb** | Device profile CRUD, persistence | `DeviceProfile`, `DeviceDatabase` |
| **settings** | Application preferences, serialization | `AppSettings`, `Preferences` |
| **ui** | GTK4 widgets, main window, panels | `MainWindow`, `MachineControlPanel`, `VisualizerPanel` |

## Crate Dependency Graph

```
                              ┌─────────────────┐
                              │    gcodekit5    │
                              │   (binary)      │
                              └────────┬────────┘
                                       │
                              ┌────────▼────────┐
                              │  gcodekit5-ui   │
                              │   (GTK4 UI)     │
                              └────────┬────────┘
                                       │
         ┌─────────────┬───────────────┼───────────────┬─────────────┐
         │             │               │               │             │
         ▼             ▼               ▼               ▼             ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│  camtools   │ │  designer   │ │ visualizer  │ │ gcodeeditor │ │communication│
└──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
       │               │               │               │               │
       │               │               │               │               │
       │        ┌──────┴───────┐       │               │               │
       │        │              │       │               │               │
       │        ▼              │       │               │               │
       │  ┌───────────┐        │       │               │               │
       │  │ devicedb  │        │       │               │               │
       │  └─────┬─────┘        │       │               │               │
       │        │              │       │               │               │
       │        │   ┌──────────┴───────┴───────────────┴───────────────┘
       │        │   │
       │        ▼   ▼
       │  ┌───────────┐
       │  │ settings  │
       │  └─────┬─────┘
       │        │
       └────────┼────────────────────────────────────────────┐
                │                                            │
                ▼                                            ▼
        ┌─────────────┐                              ┌─────────────┐
        │    core     │◄─────────────────────────────│  (all use)  │
        │ (foundation)│                              └─────────────┘
        └─────────────┘
```

### Dependency Rules

1. **core** has no internal dependencies (only external crates like `serde`, `thiserror`)
2. All crates depend on **core**
3. **ui** depends on all other crates (aggregates everything)
4. **designer** depends on **visualizer** (reuses rendering)
5. No circular dependencies allowed

## Data Flow

### 1. G-Code Loading and Display

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  File Open  │────▶│ GCodeBuffer │────▶│   Parser    │────▶│ Toolpath    │
│  (UI)       │     │ (editor)    │     │  (core)     │     │ (visualizer)│
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                           │                                       │
                           ▼                                       ▼
                    ┌─────────────┐                         ┌─────────────┐
                    │ Syntax      │                         │ Canvas      │
                    │ Highlighting│                         │ Rendering   │
                    └─────────────┘                         └─────────────┘
```

### 2. Machine Control Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  UI Button  │────▶│  Command    │────▶│ Connection  │────▶│  Serial     │
│  (Jog/Home) │     │  Builder    │     │  Manager    │     │  Port       │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                                                                   │
                                                                   ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  UI Update  │◀────│  State      │◀────│  Response   │◀────│  CNC        │
│  (DRO/Status)     │  Manager    │     │  Parser     │     │  Controller │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
```

### 3. G-Code Streaming Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Send       │────▶│ Stream      │────▶│ Character   │
│  Button     │     │ Manager     │     │ Counting    │
└─────────────┘     └─────────────┘     │ Protocol    │
                           │            └──────┬──────┘
                           │                   │
                    ┌──────▼──────┐            │
                    │ Progress    │            │
                    │ Tracking    │◀───────────┘
                    └──────┬──────┘      (ok responses)
                           │
                    ┌──────▼──────┐
                    │ UI Progress │
                    │ Bar Update  │
                    └─────────────┘
```

### 4. Designer to G-Code Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Shape      │────▶│ Toolpath    │────▶│ G-Code      │────▶│ Editor      │
│  Drawing    │     │ Generator   │     │ Emitter     │     │ Buffer      │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
       │                   │
       │                   ▼
       │            ┌─────────────┐
       │            │ Pocket/     │
       │            │ Profile/    │
       │            │ Engrave     │
       │            └─────────────┘
       │
       ▼
┌─────────────┐
│ Visualizer  │
│ Preview     │
└─────────────┘
```

## Key Design Patterns

### 1. Interior Mutability (Rc<RefCell<T>>)

Used throughout UI code for GTK callback compatibility:

```rust
// Pattern: Shared mutable state for GTK callbacks
struct PanelState {
    inner: Rc<RefCell<PanelStateInner>>,
}

impl PanelState {
    fn update_position(&self, pos: Position) {
        self.inner.borrow_mut().position = pos;
    }
    
    fn get_position(&self) -> Position {
        self.inner.borrow().position
    }
}
```

See [ADR-004: Interior Mutability](docs/adr/ADR-004-interior-mutability.md) for details.

### 2. Event-Driven Architecture

Application events flow through a central event system:

```rust
// Core event types
pub enum AppEvent {
    // Machine events
    MachineStateChanged(MachineState),
    PositionUpdated(Position),
    
    // Connection events
    Connected(DeviceInfo),
    Disconnected,
    
    // G-code events
    StreamingStarted,
    StreamingProgress(usize, usize),
    StreamingComplete,
    
    // UI events
    FileOpened(PathBuf),
    SettingsChanged,
}

// Event dispatch
fn dispatch_event(event: AppEvent) {
    // Notify all registered listeners
    for listener in &self.listeners {
        listener.on_event(&event);
    }
}
```

### 3. Builder Pattern for Configuration

Used for complex object construction:

```rust
// Builder pattern example
let toolpath = ToolpathBuilder::new()
    .feed_rate(1000.0)
    .plunge_rate(500.0)
    .safe_height(5.0)
    .step_down(2.0)
    .build()?;
```

### 4. Strategy Pattern for Firmware Protocols

Different firmware types implement a common protocol trait:

```rust
pub trait FirmwareProtocol: Send + Sync {
    fn name(&self) -> &str;
    fn parse_status(&self, response: &str) -> Option<MachineStatus>;
    fn format_command(&self, cmd: &Command) -> String;
    fn supports_feature(&self, feature: Feature) -> bool;
}

// Implementations
struct GrblProtocol;
struct TinyGProtocol;
struct FluidNCProtocol;
```

### 5. Viewport Transform for Coordinate Conversion

Centralized coordinate transformation:

```rust
pub struct ViewportTransform {
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub width: f64,
    pub height: f64,
}

impl ViewportTransform {
    /// World (mm) to screen (pixels) with Y-flip
    pub fn world_to_screen(&self, x: f64, y: f64) -> (f64, f64) {
        let sx = (x - self.pan_x) * self.zoom + self.width / 2.0;
        let sy = self.height - ((y - self.pan_y) * self.zoom + self.height / 2.0);
        (sx, sy)
    }
}
```

See [Coordinate System Documentation](docs/COORDINATE_SYSTEM.md) for details.

## Threading Model

### Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Main Thread (GTK)                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  UI Widgets │  │  Event Loop │  │  Rendering  │  │  User Input │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
         │                    │                    │
         │ spawn              │ channel            │ idle_add
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Tokio Runtime  │  │  Status Poller  │  │  File I/O       │
│  (async tasks)  │  │  (200ms timer)  │  │  (background)   │
└─────────────────┘  └─────────────────┘  └─────────────────┘
         │
         ▼
┌─────────────────┐
│  Serial Port    │
│  Read/Write     │
└─────────────────┘
```

### Thread Types

| Thread | Purpose | Communication |
|--------|---------|---------------|
| **Main (GTK)** | UI rendering, event handling | Direct access to widgets |
| **Tokio Runtime** | Async I/O, serial communication | Channels + `glib::idle_add` |
| **Status Poller** | Periodic machine status queries | Timer + channel |
| **File I/O** | Large file operations | Spawn + callback |

### Thread Safety Rules

1. **GTK widgets** must only be accessed from the main thread
2. Use `glib::idle_add()` to schedule UI updates from background threads
3. Use channels (`std::sync::mpsc` or `tokio::sync`) for thread communication
4. Use `Arc<Mutex<T>>` for shared state across threads
5. Use `Rc<RefCell<T>>` for shared state within main thread only

### Example: Background Task with UI Update

```rust
// Spawn background task
let (tx, rx) = glib::MainContext::channel(glib::Priority::DEFAULT);

std::thread::spawn(move || {
    // Do expensive work in background
    let result = expensive_computation();
    
    // Send result to main thread
    tx.send(result).expect("Channel closed");
});

// Receive on main thread
rx.attach(None, move |result| {
    // Safe to update UI here
    label.set_text(&format!("Result: {}", result));
    glib::ControlFlow::Continue
});
```

## Error Handling Strategy

### Layered Approach

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           UI Layer                                      │
│                    anyhow::Error → User Dialog                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Application Layer                                │
│              anyhow::Result with .context() for chaining                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Library Crates                                  │
│         thiserror-derived error types for specific errors               │
└─────────────────────────────────────────────────────────────────────────┘
```

### Error Types by Crate

| Crate | Error Type | Key Variants |
|-------|------------|--------------|
| **core** | `GCodeError` | `ParseError`, `ValidationError` |
| **communication** | `CommunicationError` | `PortNotFound`, `Timeout`, `ProtocolError` |
| **settings** | `SettingsError` | `IoError`, `SerializationError` |
| **devicedb** | `DeviceDbError` | `NotFound`, `DuplicateId` |

See [ADR-005: Error Handling](docs/adr/ADR-005-error-handling.md) for details.

## Related Documentation

- [ADR-001: GTK4 UI Framework](docs/adr/ADR-001-gtk4-ui-framework.md)
- [ADR-002: Coordinate System](docs/adr/ADR-002-coordinate-system.md)
- [ADR-003: Modular Crates](docs/adr/ADR-003-modular-crates.md)
- [ADR-004: Interior Mutability](docs/adr/ADR-004-interior-mutability.md)
- [ADR-005: Error Handling](docs/adr/ADR-005-error-handling.md)
- [Coordinate System Documentation](docs/COORDINATE_SYSTEM.md)
- [Developer Setup Guide](DEVELOPMENT.md)
- [Contributing Guidelines](CONTRIBUTING.md)
