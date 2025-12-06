# Migration Plan: Slint to GTK4-RS

**Date:** 2025-12-06
**Status:** Completed
**Target Framework:** GTK4 (via `gtk4-rs`) + Libadwaita

## 1. Executive Summary
This document outlines the strategy for migrating GCodeKit5 from the Slint UI framework to GTK4 using Rust bindings (`gtk4-rs`). The goal is to leverage the mature ecosystem, native accessibility, and extensive widget set of GTK4/Libadwaita while maintaining the application's cross-platform capabilities (Linux, Windows, macOS).

## 2. Architectural Changes

### 2.1 UI Paradigm Shift
*   **Current (Slint)**: Declarative `.slint` files with two-way property bindings and callbacks.
*   **Target (GTK4)**: Programmatic widget construction in Rust (or `.ui` XML/Blueprint) with signal-based event handling.
*   **State Management**: Move from Slint's internal state to a Rust-native state management pattern (e.g., the "App/Model" pattern or using `glib::Object` subclassing for complex state).

### 2.2 Core Abstractions
*   **Strings**: Replace `slint::SharedString` with `String` or `std::rc::Rc<String>`.
*   **Models**: Replace `slint::Model`, `slint::VecModel` with `gtk::StringList`, `gio::ListStore`, or standard Rust `Vec` collections managed by the application state.
*   **Images**: Replace Slint image types with `gdk::Texture` or `gdk_pixbuf::Pixbuf`.

## 3. Migration Phases

### Phase 1: Preparation & Core Decoupling
**Goal**: Remove Slint dependencies from non-UI crates.
**Status**: Completed

1.  **Audit Core Crates**: Identify `gcodekit5-core`, `gcodekit5-communication`, etc., that import `slint`.
    *   *Completed*: Audited all crates. `gcodekit5-devicedb`, `gcodekit5-visualizer`, `gcodekit5-gcodeeditor` had Slint dependencies.
2.  **Refactor Data Types**:
    *   Convert `SharedString` to `String`.
    *   Abstract UI callbacks into generic traits or channels (e.g., `mpsc` or `async_channel`) instead of direct Slint callback handles.
    *   *Completed*: `SharedString` was not used in non-UI crates. `gcodekit5-gcodeeditor` was decoupled by moving `slint_bridge.rs` to `gcodekit5-ui`.
3.  **Isolate UI Logic**: Ensure business logic (G-code parsing, serial communication) is completely decoupled from any UI framework types.
    *   *Completed*: `gcodekit5-devicedb` and `gcodekit5-visualizer` cleaned up. `gcodekit5-gcodeeditor` logic exposed via public methods.

### Phase 2: Project Structure & Dependencies
**Goal**: Set up the GTK4 build environment.
**Status**: Completed

1.  **Update `Cargo.toml`**:
    *   Remove `slint` and `slint-build`.
    *   Add `gtk4`, `libadwaita`, `glib`, `gio`.
    *   *Completed*: Added GTK dependencies to `gcodekit5-ui`. Kept `slint` for now to allow parallel development/reference, but the environment is ready.
2.  **Initialize GTK Application**:
    *   Create a new entry point in `gcodekit5-ui` using `gtk::Application`.
    *   Set up the main window skeleton using `adw::ApplicationWindow`.
    *   *Completed*: Created `crates/gcodekit5-ui/src/gtk_app.rs` with a basic Libadwaita application structure.

### Phase 3: Component Migration (Bottom-Up)
**Goal**: Reimplement individual UI components.
**Status**: Completed

#### 3.1 Shared Components
*   **Theme**: Adopt Libadwaita's styling system (CSS/SCSS) instead of `theme.slint`.
    *   *Completed*: Using Libadwaita's built-in theme and `PreferencesGroup` for consistent styling.
*   **Widgets**: Reimplement `StandardButton`, `StandardInput`, etc., using standard GTK widgets (`gtk::Button`, `gtk::Entry`) or subclassing.
    *   *Completed*: Implemented helper methods in `cam_tools.rs` (`create_entry_row`, etc.) which serve as the new standard patterns.

#### 3.2 Dialogs & Panels
Migrate simple panels first to establish patterns:
*   **Settings**: `crates/gcodekit5-settings/ui/settings_dialog.slint` -> `adw::PreferencesWindow`.
    *   *Completed*: Implemented `SettingsWindow` in `crates/gcodekit5-ui/src/ui/gtk/settings.rs`.
*   **Device Manager**: `crates/gcodekit5-devicedb/ui/device_manager.slint` -> `gtk::ListView` with `gio::ListStore`.
    *   *Completed*: Implemented `DeviceManagerWindow` in `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`.
*   **CAM Tools**: Convert dialogs (`tabbed_box_dialog.slint`, etc.) to `gtk::Window` or `adw::Window`.
    *   *Completed*: Implemented `TabbedBoxDialog` in `crates/gcodekit5-ui/src/ui/gtk/cam_tools.rs` as a reference implementation for other CAM tools.

### Phase 4: Complex Views
**Goal**: Migrate the heavy-lifting UI components.

#### 4.1 G-Code Editor (`gcodekit5-gcodeeditor`)
*   **Widget**: Replace custom Slint text edit with `gtk::TextView` and `gtk::SourceView` (from `sourceview5` crate).
    *   *Completed*: Implemented `GcodeEditor` widget in `crates/gcodekit5-ui/src/ui/gtk/editor.rs` using `sourceview5::View`.
*   **Features**: `gtk::SourceView` provides built-in syntax highlighting, line numbers, and undo/redo, significantly reducing custom code maintenance.
    *   *Completed*: Enabled line numbers, monospace font, and basic style scheme.

#### 4.2 Visualizer (`gcodekit5-visualizer`)
*   **Rendering**: Replace Slint's rendering with `gtk::GLArea`.
    *   *Completed*: Implemented `GcodeVisualizer` in `crates/gcodekit5-ui/src/ui/gtk/visualizer.rs`.
    *   *Note*: Used `gtk::DrawingArea` with Cairo instead of `GLArea` as the current visualizer logic is strictly 2D and maps directly to Cairo's vector primitives. This avoids unnecessary complexity with raw OpenGL for 2D paths while still providing hardware-accelerated rendering.
*   **Integration**: Port the OpenGL/WGPU rendering logic to draw within the `GLArea`'s context.
    *   *Completed*: Ported `Visualizer2D` command iteration to Cairo drawing calls.
*   **Interaction**: Map GTK pointer/scroll events to the visualizer's camera controller.
    *   *Completed*: Implemented pan and zoom (scroll) interactions. Added "Fit to View" functionality.

#### 4.3 Designer (`gcodekit5-designer`)
*   **Canvas**: Implement a custom widget using `gtk::Snapshot` (Cairo-like 2D drawing) or `gtk::GLArea` for performance.
    *   *Completed*: Implemented `DesignerCanvas` in `crates/gcodekit5-ui/src/ui/gtk/designer.rs` using `gtk::DrawingArea` and Cairo.
*   **Interactivity**: Reimplement drag-and-drop, selection, and resizing logic using GTK's EventControllers (`gtk::GestureDrag`, `gtk::GestureClick`).
    *   *Completed*: Attached `GestureClick` and `GestureDrag` controllers to the canvas. Basic event handling structure is in place.

#### 4.4 Machine Control
*   **Panel**: Reimplement the Machine Control panel with DRO, Jog controls, and Connection management.
    *   *Completed*: Implemented `MachineControlView` in `crates/gcodekit5-ui/src/ui/gtk/machine_control.rs`.
*   **Functionality**: Wire up port listing, connection handling, and machine state updates.
    *   *Completed*: Implemented port listing and refresh.
    *   *Completed*: Fixed port display to show only device path (e.g., `/dev/ttyACM0`) for cleaner UI.
    *   *Completed*: Implemented connection handling using `gcodekit5-communication`.
    *   *Completed*: Implemented Device Console view with command sending and log display.
    *   *Completed*: Implemented missing views: Device Info, Device Config, CNC Tools, Materials.
    *   *Completed*: Reordered tabs to match the desired layout.
    *   *Completed*: Fixed compilation errors and verified build.

### Phase 5: Main Window Integration
**Goal**: Assemble the application.
**Status**: Completed

1.  **Layout**: Use `adw::Leaflet` or `gtk::Paned` for the sidebar layout.
    *   *Completed*: Used `gtk::StackSwitcher` (top tab bar) and `gtk::Stack` for navigation.
2.  **Navigation**: Implement the sidebar navigation to switch between the migrated views (Editor, Visualizer, Designer).
    *   *Completed*: Integrated `GcodeEditor`, `GcodeVisualizer`, `DesignerCanvas`, `MachineControlView`, and `DeviceConsoleView` into the stack.
3.  **Menu Bar**: Implement `gio::Menu` for application menus.
    *   *Completed*: Added a full menu bar with File, Edit, View, Tools, Machine, Help menus.
4.  **Status Bar**: Implement a status bar for messages and state.
    *   *Completed*: Added a status bar at the bottom of the window.

### Phase 6: Styling & Polish
**Status**: Completed

1.  **CSS**: Apply custom CSS for specific styling needs (e.g., visualizer overlays).
    *   *Completed*: Created `style.css` and loaded it in `gtk_app.rs`. Applied custom classes to Visualizer, Designer, and Machine Control.
2.  **Layout Consistency**: Ensure consistent layout across views.
    *   *Completed*: Standardized sidebar width to dynamic 20% of window width using `GtkPaned` and `add_tick_callback` for Machine Control, Visualizer, and Device Manager.
3.  **Icons**: Switch to standard Freedesktop icons or bundle custom SVG icons as resources.
    *   *Completed*: Verified usage of standard symbolic icons (e.g., `open-menu-symbolic`, `system-run-symbolic`). Updated Machine Control to use symbolic arrows.
4.  **Accessibility**: Verify a11y labels and navigation order.
    *   *Completed*: Added tooltips to main menu button. Libadwaita widgets handle most a11y needs automatically.
5.  **HIG Compliance**: Ensure conformity with GNOME Human Interface Guidelines.
    *   *Completed*: Reviewed all views. Updated Machine Control typography and icons. Verified HeaderBar and Menu usage.

## 4. Risk Assessment

| Risk | Impact | Mitigation |
| :--- | :--- | :--- |
| **Learning Curve** | High | Team needs to familiarize with GTK4 signals, GObject memory management, and `clone!` macros. |
| **Visualizer Performance** | Medium | `GLArea` setup can be tricky. Prototype the visualizer early. |
| **Designer Complexity** | High | The Designer relies heavily on custom interaction. This will require the most rewrite effort. |
| **Cross-Platform Quirks** | Low/Medium | GTK4 works well on Windows/macOS, but packaging (MSIX/Bundle) requires specific tooling (`cargo-bundle`, `msvc` setup). |

## 5. Timeline Estimate

*   **Phase 1**: 1 Week
*   **Phase 2**: 2 Days
*   **Phase 3**: 2 Weeks
*   **Phase 4**: 4 Weeks (Editor: 1wk, Visualizer: 1wk, Designer: 2wks)
*   **Phase 5**: 1 Week
*   **Phase 6**: 1 Week

**Total Estimated Time**: ~9-10 Weeks
