# Designer Migration Comparison: Slint (v4) vs GTK4 (v5)

This document compares the functionality of the Designer in `gcodekit4` (Slint) with the current implementation in `gcodekit5` (GTK4) and outlines the remaining tasks for full migration.

## 1. Canvas & Interaction

| Feature | Slint Implementation | GTK4 Implementation | Status | Task ID | Est. Time |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Rendering** | SVG Paths based on data strings | Cairo 2D drawing | ✅ Complete | - | - |
| **Navigation** | Pan/Zoom via mouse & buttons | Pan/Zoom via mouse & buttons | ✅ Complete | - | - |
| **Grid** | Configurable grid & origin | Configurable grid & origin | ✅ Complete | - | - |
| **Selection** | Click, Drag-rect, Multi-select | Click, Drag-rect, Multi-select | ✅ Complete | - | - |
| **Rubber Band** | Visual selection rectangle | Visual selection rectangle | ✅ Complete | - | - |
| **Handles** | Resize/Move handles | Resize handles implemented | ✅ Complete | - | - |
| **Snapping** | Shift key snapping | Shift key snapping | ✅ Complete | T-101 | 2h |

## 2. Toolbox & Creation

| Feature | Slint Implementation | GTK4 Implementation | Status | Task ID | Est. Time |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Basic Tools** | Select, Rect, Circle, Line, Ellipse, Polyline, Text | Same set of tools | ✅ Complete | - | - |
| **Tool Setup** | Feed, Speed, Tool Dia, Depth, Step inputs in sidebar | Added to Toolbox | ✅ Complete | T-201 | 4h |
| **Polyline** | Click-click creation | **Missing** (Placeholder only) | ❌ Missing | T-202 | 6h |
| **Text** | Text creation & editing | Basic text creation | ⚠️ Partial | T-203 | 3h |

## 3. Properties Panel

| Feature | Slint Implementation | GTK4 Implementation | Status | Task ID | Est. Time |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Transform** | X, Y, W, H, Rotation | X, Y, W, H, Rotation | ✅ Complete | - | - |
| **Shape Props** | Radius, Text content/size | Radius, Text content/size | ✅ Complete | - | - |
| **CAM Props** | Pocket (Depth, Step, Strategy), V-Carve settings | **Missing** | ❌ Missing | T-301 | 8h |
| **Name** | Rename shape | Rename shape | ✅ Complete | - | - |

## 4. Layers & Organization

| Feature | Slint Implementation | GTK4 Implementation | Status | Task ID | Est. Time |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **List** | Hierarchical list of shapes | Flat list (z-ordered) | ✅ Complete | - | - |
| **Visibility** | Toggle visibility | Toggle visibility | ✅ Complete | - | - |
| **Z-Order** | Reorder via buttons/drag | Reorder via buttons | ✅ Complete | - | - |
| **Grouping** | Group/Ungroup commands | Group/Ungroup commands | ✅ Complete | - | - |

## 5. Advanced Operations (Context Menu)

| Feature | Slint Implementation | GTK4 Implementation | Status | Task ID | Est. Time |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Clipboard** | Cut, Copy, Paste | Cut, Copy, Paste | ✅ Complete | - | - |
| **Alignment** | Align Left, Center, Right, Top, Mid, Bot | **Missing** | ❌ Missing | T-501 | 4h |
| **Arrays** | Linear, Circular, Grid Arrays | **Missing** | ❌ Missing | T-502 | 6h |
| **Conversion** | Convert to Path, Convert to Rectangle | **Missing** | ❌ Missing | T-503 | 3h |

## 6. File Operations

| Feature | Slint Implementation | GTK4 Implementation | Status | Task ID | Est. Time |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Native** | New, Open, Save (.gckd) | New, Open, Save (.gckd) | ✅ Complete | - | - |
| **Import** | DXF, SVG Import | **Missing** | ❌ Missing | T-601 | 6h |
| **Export** | G-Code, SVG Export | **Missing** | ❌ Missing | T-602 | 4h |

## 7. Toolpath & Simulation

| Feature | Slint Implementation | GTK4 Implementation | Status | Task ID | Est. Time |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Generation** | Generate G-Code button | **Missing** | ❌ Missing | T-701 | 5h |
| **Simulation** | Preview toolpath lines | **Missing** | ❌ Missing | T-702 | 6h |

## Migration Plan & Priorities

### Priority 1: Core Editing Features
1.  **T-201 Tool Setup Panel**: Essential for defining cutting parameters.
2.  **T-301 CAM Properties**: Critical for defining how shapes are cut (pockets, profiles).
3.  **T-501 Alignment Tools**: Basic layout necessity.

### Priority 2: Advanced Creation
4.  **T-502 Arrays**: Powerful creation tool.
5.  **T-202 Polyline Tool**: Complete the drawing toolset.
6.  **T-601 Import (DXF/SVG)**: Essential for external workflows.

### Priority 3: Output & Polish
7.  **T-701 Toolpath Generation**: The final output step.
8.  **T-602 Export**: Saving results.
9.  **T-503 Conversion**: Utility features.
10. **T-702 Simulation**: Visual verification.

## Total Estimated Effort
~50-60 hours of development time to reach feature parity.
