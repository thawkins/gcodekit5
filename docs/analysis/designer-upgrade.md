# Designer Upgrade Plan: GCodeKit5 vs. E-CAM

This document outlines the functional gaps between the current GCodeKit5 Designer and the E-CAM drawing system, and provides a multi-step plan to achieve parity.

## Feature Matrix

| Feature | GCodeKit5 | E-CAM | Status |
| :--- | :---: | :---: | :--- |
| **Basic Shapes** (Line, Circle, Rect, etc.) | ✅ | ✅ | Parity |
| **Boolean Operations** (Union, Diff, Intersect) | ✅ | ✅ | Parity |
| **Parametric Templates** | ⚠️ (Core only) | ✅ (Fast Shape) | Gap |
| **Fillet / Chamfer** | ⚠️ (Rect only) | ✅ | Gap |
| **Offset / Inset** | ❌ | ✅ | Gap |
| **Mirror / Array** | ⚠️ (Array via patterns) | ✅ | Gap |
| **Spline / NURBS Support** | ❌ | ✅ | Gap |
| **DXF / SVG Import** | ✅ | ✅ | Parity |
| **DWG Import** | ❌ | ✅ | Gap |
| **3D Import** (STEP, IGES, STL) | ✅ (STL) | ✅ | Partial Parity |
| **3D Shadow Projection** | ✅ | ✅ | Parity |
| **Layers / Properties** | ✅ | ✅ | Parity |

---

## Upgrade Plan

### Step 1: Advanced 2D Geometry Operations
Implement missing fundamental CAD operations to allow complex part design without external software.
- **Fillet/Chamfer**: Add a tool to apply fillets or chamfers to any vertex in a `DesignPath` or between two lines.
- **Offset**: Implement path offsetting (contouring) for `DesignPath` objects. This is critical for manual tool compensation and inlay work.
- **Mirror**: Add a mirror tool with a user-defined axis.
- **Spline Support**: Integrate Bézier or B-Spline curves into the `DesignPath` model for organic shapes.

### Step 2: "Fast Shape" Parametric Library
Leverage the existing `parametric.rs` infrastructure to create a library of common mechanical and decorative shapes.
- **Mechanical**: Gears (Spur, Helical), Sprockets, Timing Pulleys.
- **Structural**: Tabbed Boxes (auto-generating finger joints), Slots, and Brackets.
- **UI Integration**: Create a "Fast Shape" gallery in the Designer toolbox for quick insertion.

### Step 3: 3D Model Integration (The "Shadow" Feature) ✅ **COMPLETE**
Enable users to import 3D models and convert them into 2D toolpaths, similar to E-CAM's 3D shadow projection.
- **3D Import**: ✅ Add support for STL (mesh) files with full integration into designer interface.
- **Projection Engine**: ✅ Implement a "Project to Plane" feature that generates 2D silhouette or "shadow" of 3D models with multiple view modes (orthographic, perspective, isometric).
- **Slice to Toolpath**: ✅ Allow slicing 3D models into multiple 2D layers for 2.5D machining with configurable cutting strategies.
- **3D Visualization**: ✅ OpenGL-based 3D mesh rendering with materials, lighting, and scene management.
- **Workflow Integration**: ✅ Complete STL-to-G-code pipeline with toolpath generation and export capabilities.

*See: `docs/implementation/3D_INTEGRATION_COMPLETE.md` for full implementation details.*

### Step 4: Advanced Import/Export & Interoperability
Improve compatibility with industry-standard formats.
- **DWG Support**: Integrate a library to handle DWG files directly.
- **DXF Fidelity**: Improve support for DXF layers, blocks, and complex entities (Ellipses, Splines).
- **Export**: Add SVG and DXF export for the designed parts.

### Step 5: UI/UX Refinement
Enhance the drawing experience to match professional CAD workflows.
- **Snapping**: Implement object snapping (Endpoint, Midpoint, Center, Tangent).
- **Constraints**: (Long term) Add geometric constraints (Parallel, Perpendicular, Coincident).
- **Command Bar**: Add a keyboard-driven command bar for power users (e.g., typing "L 0,0 10,10" to draw a line).

---

## Technical Considerations
- **Geometry Engine**: Consider if `csgrs` and `lyon` are sufficient for advanced offsetting and splines, or if a more robust geometry kernel (like `truck`) is needed for Step 3.
- **3D Rendering**: The `gcodekit5-visualizer` may need updates to render 3D CAD models alongside G-code toolpaths.
- **Performance**: 3D projection and complex offsets can be CPU intensive; consider offloading to worker threads.
