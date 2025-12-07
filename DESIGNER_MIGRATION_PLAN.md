# Designer Migration Plan: Slint to GTK4

## Overview
Migrate the Designer tool from Slint to GTK4, following the same pattern used for other panels. This document breaks down the migration into manageable phases.

## Phase 1: Basic Canvas and Drawing Infrastructure
**Goal:** Get the canvas rendering basic shapes with pan/zoom

### Tasks:
- [x] Create basic `DesignerView` struct with GTK4 DrawingArea
- [x] Implement Cairo drawing for basic shapes (Rectangle, Circle, Line, Ellipse)
- [x] Set up coordinate system transformation (Y-up Cartesian)
- [x] Implement basic viewport (pan/zoom)
- [ ] Add grid rendering with configurable spacing
- [ ] Add origin crosshair marker
- [ ] Add mouse position tracking and display

### Files to Create/Modify:
- `crates/gcodekit5-ui/src/ui/gtk/designer.rs` (partially done)
- Integration into main app

### Dependencies:
- `gcodekit5-designer` crate (already exists)
- Cairo for 2D rendering
- GTK4 DrawingArea

---

## Phase 2: Toolbox and Shape Creation
**Goal:** Add vertical toolbox with shape creation tools

### Tasks:
- [ ] Create vertical toolbox panel with tool buttons
  - Select/Move tool
  - Rectangle tool
  - Circle tool
  - Line tool
  - Ellipse tool
  - Path/Polygon tool
  - Text tool
- [ ] Implement tool selection state management
- [ ] Add tool cursors
- [ ] Implement shape creation interactions:
  - Click-drag for rectangles/ellipses
  - Click-click for circles (center + radius)
  - Click-click for lines
  - Click-click-click for polygons
- [ ] Add shape properties overlay during creation

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

### Current Status (Phase 1)
- Basic canvas rendering implemented
- Cairo shape drawing for Rectangle, Circle, Line, Ellipse, Path, Text
- Coordinate system transformation (Y-up)
- Click and drag gesture handlers (stubs)
- Grid drawing (basic)

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

### 2025-12-07
- Created phased migration plan
- Decided on 8 phases to spread complexity
- Phase 1 partially implemented (basic canvas)
- Priority: Get Phase 1 complete, then Phase 2 for basic usability

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
