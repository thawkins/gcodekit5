# Designer Migration Plan: Slint to GTK4

## Overview
Migrate the Designer tool from Slint to GTK4, following the same pattern used for other panels. This document breaks down the migration into manageable phases.

## Phase 1: Basic Canvas and Drawing Infrastructure ✅ COMPLETE
**Goal:** Get the canvas rendering basic shapes with pan/zoom

### Tasks:
- [x] Create basic `DesignerView` struct with GTK4 DrawingArea
- [x] Implement Cairo drawing for basic shapes (Rectangle, Circle, Line, Ellipse)
- [x] Set up coordinate system transformation (Y-up Cartesian)
- [x] Implement basic viewport (pan/zoom)
- [x] Add grid rendering with configurable spacing (10mm major, 2mm minor)
- [x] Add origin crosshair marker (red circle with cross)
- [x] Add mouse position tracking and display (real-time coordinate display)

### Files to Create/Modify:
- `crates/gcodekit5-ui/src/ui/gtk/designer.rs` (partially done)
- Integration into main app

### Dependencies:
- `gcodekit5-designer` crate (already exists)
- Cairo for 2D rendering
- GTK4 DrawingArea

---

## Phase 2: Toolbox and Shape Creation ✅ COMPLETE
**Goal:** Add vertical toolbox with shape creation tools

### Tasks:
- [x] Create vertical toolbox panel with tool buttons
  - [x] Select/Move tool
  - [x] Rectangle tool
  - [x] Circle tool
  - [x] Line tool
  - [x] Ellipse tool
  - [x] Path/Polygon tool
  - [x] Text tool
- [x] Implement tool selection state management
- [x] Add tool cursors (system cursors)
- [x] Implement shape creation interactions:
  - [x] Click-drag for rectangles/ellipses
  - [x] Click-drag for circles (center + radius)
  - [x] Click-drag for lines
  - [ ] Click-click-click for polygons (deferred to Phase 3)
- [ ] Add shape properties overlay during creation (deferred to Phase 4)

### Files to Create/Modify:
- `crates/gcodekit5-ui/src/ui/gtk/designer.rs`
- `crates/gcodekit5-ui/src/ui/gtk/designer_toolbox.rs` (new)

---

## Phase 3: Selection and Transformation
**Goal:** Enable selecting, moving, and transforming shapes

### Tasks:
- [ ] Implement shape selection:
  - Click to select single shape
  - Shift+Click for multi-select
  - Drag selection rectangle
- [ ] Add selection handles (8-point bounding box)
- [ ] Implement drag-to-move for selected shapes
- [ ] Add resize handles with live preview
- [ ] Implement rotation handles
- [ ] Add snap-to-grid functionality
- [ ] Show selection properties (position, size, rotation)

### Files to Create/Modify:
- `crates/gcodekit5-ui/src/ui/gtk/designer.rs`
- `crates/gcodekit5-ui/src/ui/gtk/designer_selection.rs` (new)
- `crates/gcodekit5-ui/src/ui/gtk/designer_handles.rs` (new)

---

## Phase 4: Properties Panel and Shape Editing
**Goal:** Add properties panel for editing shape attributes

### Tasks:
- [ ] Create right-side properties panel (stack-based)
- [ ] Implement shape property editors:
  - Position (X, Y)
  - Size (Width, Height, Radius)
  - Rotation
  - Fill/Stroke properties
  - Text properties (font, size, content)
- [ ] Add toolpath properties:
  - Step down
  - Step in/over
  - Pocket strategy
  - Raster angle
  - Bidirectional
- [ ] Live update shapes as properties change
- [ ] Add property presets/defaults
- [ ] Implement multi-shape property editing

### Files to Create/Modify:
- `crates/gcodekit5-ui/src/ui/gtk/designer_properties.rs` (new)
- Integration with designer.rs

---

## Phase 5: Layer Management and Grouping
**Goal:** Add layer/group management and operations

### Tasks:
- [ ] Create layers/objects panel (left sidebar)
- [ ] Show hierarchical list of shapes and groups
- [ ] Implement grouping operations:
  - Group selected shapes (Ctrl+G)
  - Ungroup (Ctrl+Shift+G)
  - Group selection with bounding box
- [ ] Add layer visibility toggles
- [ ] Implement Z-order operations:
  - Bring to front
  - Send to back
  - Move up/down
- [ ] Add shape naming/renaming
- [ ] Show layer thumbnails

### Files to Create/Modify:
- `crates/gcodekit5-ui/src/ui/gtk/designer_layers.rs` (new)
- Update designer.rs for grouping logic

---

## Phase 6: File Operations and Import/Export
**Goal:** Enable saving, loading, and importing designs

### Tasks:
- [ ] Implement File menu operations:
  - New design
  - Open design (.gckd format)
  - Save design
  - Save As
  - Export options
- [ ] Add import functionality:
  - SVG import
  - DXF import
  - Image import (for tracing)
- [ ] Implement export functionality:
  - SVG export
  - DXF export
  - PNG/bitmap export
- [ ] Add recent files list
- [ ] Implement auto-save/recovery
- [ ] Add unsaved changes warning

### Files to Create/Modify:
- `crates/gcodekit5-ui/src/ui/gtk/designer_file_ops.rs` (new)
- Integration with file dialogs

---

## Phase 7: Toolpath Generation and Preview
**Goal:** Generate toolpaths and preview cutting operations

### Tasks:
- [ ] Add toolpath generation panel (bottom)
- [ ] Implement toolpath settings:
  - Tool selection from tool library
  - Feed rate, plunge rate
  - Material selection
  - Tabs/bridges
- [ ] Generate G-code from shapes:
  - Outline paths
  - Pocket operations
  - Drilling
  - V-carve
- [ ] Add toolpath preview layer
- [ ] Show estimated time and material usage
- [ ] Implement simulation/verification
- [ ] Send to G-code Editor button

### Files to Create/Modify:
- `crates/gcodekit5-ui/src/ui/gtk/designer_toolpath.rs` (new)
- Integration with gcodekit5-designer toolpath generator

---

## Phase 8: Advanced Features and Polish
**Goal:** Add advanced features and final polish

### Tasks:
- [ ] Implement undo/redo system:
  - Command pattern for all operations
  - Visual undo history
  - Ctrl+Z / Ctrl+Y shortcuts
- [ ] Add clipboard operations:
  - Copy (Ctrl+C)
  - Cut (Ctrl+X)
  - Paste (Ctrl+V)
  - Duplicate (Ctrl+D)
- [ ] Implement alignment tools:
  - Align left/right/center
  - Align top/bottom/middle
  - Distribute horizontally/vertically
- [ ] Add boolean operations:
  - Union
  - Difference
  - Intersection
- [ ] Implement arrays/patterns:
  - Linear arrays (rows/columns)
  - Circular arrays (rotational)
  - Path arrays (follow curve)
- [ ] Add measurement tools
- [ ] Implement parametric shapes with formulas
- [ ] Add templates library
- [ ] Performance optimization
- [ ] Keyboard shortcuts documentation

### Files to Create/Modify:
- `crates/gcodekit5-ui/src/ui/gtk/designer_commands.rs` (new)
- `crates/gcodekit5-ui/src/ui/gtk/designer_clipboard.rs` (new)
- `crates/gcodekit5-ui/src/ui/gtk/designer_align.rs` (new)
- `crates/gcodekit5-ui/src/ui/gtk/designer_arrays.rs` (new)

---

## Implementation Notes

### Current Status
**Phase 2: ✅ COMPLETE (2025-12-08)**
- Toolbox with 7 tools (Select, Rectangle, Circle, Line, Ellipse, Polyline, Text)
- Tool selection with visual feedback
- Shape creation via click-drag interactions
- Rectangle: drag from corner to corner
- Circle: drag from center outward (radius)
- Line: drag from start to end point
- Ellipse: drag from corner to define bounding box
- Shapes automatically added to canvas
- Zoom controls (buttons for future implementation)
- Integration with canvas and state management

**Phase 1: ✅ COMPLETE (2025-12-08)**
- Full canvas rendering with Cairo
- Shape drawing: Rectangle, Circle, Line, Ellipse, Path, Text
- Coordinate system: Y-up Cartesian (centered origin)
- Grid system: Major (10mm) and minor (2mm) gridlines with axes
- Origin marker: Red crosshair with circle
- Mouse tracking: Real-time coordinate display in status bar
- Status bar: Grid toggle, position display, status messages
- Event handlers: Click, drag, motion (stubs for future phases)
- Integration: Fully integrated into main app

### Architecture
```
DesignerView
├── Canvas (DrawingArea with Cairo)
├── Toolbox (Left vertical panel)
├── Properties (Right panel)
├── Layers (Left panel, collapsible)
└── Toolpath Panel (Bottom, collapsible)
```

### Key Technologies
- **GTK4**: UI framework
- **Cairo**: 2D drawing
- **gcodekit5-designer**: Shape models and toolpath generation
- **lyon**: Path tessellation (if needed)

### Testing Strategy
- Test each phase independently before moving to next
- Create example designs for each feature
- Performance testing with complex designs (100+ shapes)
- Integration testing with G-code Editor and Visualizer

### Migration Priority
1. **Phase 1** - Foundation (must have)
2. **Phase 2** - Core functionality (must have)
3. **Phase 3** - User interaction (must have)
4. **Phase 4** - Essential editing (must have)
5. **Phase 5** - Organization (should have)
6. **Phase 6** - Persistence (should have)
7. **Phase 7** - Output (must have)
8. **Phase 8** - Polish (nice to have)

---

## Decision Log

### 2025-12-08
- Created phased migration plan
- Decided on 8 phases to spread complexity
- **Phase 1 COMPLETED** - Canvas and Drawing Infrastructure
- **Phase 2 COMPLETED** - Toolbox and Shape Creation:
  - Vertical toolbox with 7 drawing tools
  - Tool selection state management
  - Click-drag shape creation for Rectangle, Circle, Line, Ellipse
  - Shapes persist to canvas state
  - Visual tool selection feedback
  - Integrated zoom controls (UI only)
  - Clean CSS styling for toolbox
- Next: Phase 3 - Selection and Transformation

---

## Next Steps

1. Complete Phase 1 (grid, origin, mouse tracking)
2. Test Phase 1 thoroughly
3. Begin Phase 2 (toolbox and shape creation)
4. Iterate based on user feedback

---

## Resources
- Slint reference: `../gcodekit4/crates/gcodekit4-designer/ui/designer.slint`
- Slint callbacks: `../gcodekit4/src/app/callbacks/designer.rs`
- State management: `../gcodekit4/crates/gcodekit4-designer/src/designer_state.rs`
