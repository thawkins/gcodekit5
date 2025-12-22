# GTK4 & Rust Insights

## GTK Dialogs
- **Transient Parent**: Always set a transient parent for dialogs to ensure proper window management and avoid "GtkDialog mapped without a transient parent" warnings. Use `dialog.set_transient_for(Some(&parent_window))`.
- **Sizing**: Ensure dialogs have sufficient minimum size to avoid "Trying to measure GtkDialog..." warnings if content requires more space than allocated.

## Viewport & Canvas Sizing
- **Issue**: When using a `DrawingArea` with a backend `Viewport` or `Canvas` struct that manages zoom/pan, the backend needs to be explicitly updated with the widget's dimensions.
- **Solution**: Use `widget.set_draw_func` to update the backend size on every draw (which happens on resize), or use `connect_resize` / `connect_map`.
- **Gotcha**: `DrawingArea` dimensions might be 0 or default during early initialization. Use `connect_map` with a small timeout or check dimensions before applying "Fit to View" logic to ensure correct aspect ratio and padding.

## Coordinate Systems
- **Designer**: Uses Cartesian coordinates (Y-up).
- **GTK/Cairo**: Uses Screen coordinates (Y-down).
- **Transformation**: Always handle Y-flip in the `Viewport` logic.

## Notes / Learnings
- Cairo text drawing with a Y-flip needs the rotation angle negated before flipping Y to match model/G-code orientation.

## SourceView5 Search Implementation
- **SearchContext**: The `SearchContext` is central to search operations. It runs asynchronously.
- **Counting Matches**: To implement "n of m" (current match index of total matches), you must iterate through occurrences. `context.forward()` and `context.backward()` are useful but require careful handling of iterators.
- **Iterating**: To find the "current" match index relative to the cursor:
    1. Get the start of the buffer.
    2. Loop using `context.forward(iter)` until the match start is >= the cursor position (or the current match).
    3. Count the iterations.
- **Button State**: Use the count and total matches (from `context.occurrences_count()`) to enable/disable "Next"/"Previous" buttons.
- **API Gotchas**:
    - `buffer.get_selection_bound()` in C is `buffer.selection_bound()` in the Rust bindings.
    - `buffer.get_insert()` in C is `buffer.insert()` in the Rust bindings.
    - `search_settings.set_search_text(Some("text"))` is required; passing `None` clears it.

## Useful GTK4 UI Patterns
- **libadwaita**: `adw::Window` does not support `gtk_window_set_titlebar()`; use `adw::HeaderBar` / `WindowTitle` patterns instead of manually setting a titlebar.
- **Theme palette colors in Cairo**: prefer `widget.style_context().lookup_color("accent_color"|"success_color"|"warning_color")` over hard-coded RGB.
- **OSD overlays**: `Overlay` + `Box` with a shared CSS class works well for floating controls, status, and small progress panels.
- **Non-blocking background work**:
  - Put heavy work on `std::thread::spawn`.
  - Communicate completion via `Arc<Mutex<Option<T>>>` polled with `glib::timeout_add_local`.
  - For closures connected to GTK signals (`connect_*`), clone captured values before moving into nested closures (to satisfy `Fn` requirements).
- **Hide/show panel UX**: use a normal “Hide” button inside the panel and a floating “Show …” button in an overlay; persist visibility in settings.
- **Modal on startup**: for “About on startup”, set the transient parent to the main window and center it (and optionally auto-close with `glib::timeout_add_local_once`).
- **Workspace versioning**: bump `[workspace.package].version` in the root `Cargo.toml`; `cargo check` will refresh `Cargo.lock` workspace package versions.

## In-app Help Browser (Markdown from GResources)
- **Approach**: Store help topics as markdown files in `crates/gcodekit5-ui/resources/markdown/` and add them to `resources/gresources.xml`.
- **Loading**: Use `gio::resources_lookup_data("/com/gcodekit5/help/<topic>.md", ...)` to fetch the document text.
- **Rendering**: For a lightweight solution without a full markdown renderer, convert a small markdown subset to Pango markup and display using a wrapping `gtk::Label`.
- **Navigation**: Use `Label::connect_activate_link` with `help:<topic>` links to switch topics (and keep back/forward history in-memory).


## Materials/Tools List Patterns
- **Empty state panes**: For editor-style tabs, a `gtk4::Stack` with an `empty` page + `edit` page makes selection-driven UIs feel much clearer.
- **ListBox placeholders**: `ListBox::set_placeholder(Some(&Label))` is an easy way to show “No results” when filtering/searching.
- **Store row metadata**: Prefer `ListBoxRow::set_data("key", value)` over hidden widgets for IDs; retrieve via `row.data::<T>("key")`.
- **Icon+label buttons**: Build a `Box` with `Image::from_icon_name` + `Label` and set it as `Button::set_child` for consistent look (avoid emoji labels).

## GTK Label Selection
- **Issue**: `gtk::Label` with `selectable` set to true might show all text selected by default or when content changes if not handled carefully.
- **Solution**: Use `glib::idle_add_local` to defer `label.select_region(0, 0)` after setting markup. Immediate calls might be overridden by layout or focus events.
- **Gotcha**: Toggling `set_selectable(false)` then `true` might not clear selection or might trigger selection behavior in some contexts. Explicitly clearing selection is safer.

## Versioning
- **Workspace**: Bump `[workspace.package].version` in root `Cargo.toml`.
- **Lockfile**: Run `cargo check` to update `Cargo.lock`.

### Event Handling
- **Right-Click Selection**: `GestureClick` for right-click (button 3) does NOT automatically select the item under the cursor. You must manually perform hit testing and update the selection in the handler if you want right-click to select the item before showing a context menu.
- **Hit Testing**: Ensure `contains_point` logic for shapes checks the *interior* of closed shapes (like rectangles and circles), not just the boundary, if you want users to be able to select them by clicking inside. This is critical for intuitive interaction.

### Inspector Properties
- **Extensibility**: When adding new shape types (e.g., `Shape::Path`), ensure they are handled in `PropertiesPanel::update_from_selection` (to display values) and `PropertiesPanel::update_shape_position_and_size` (to apply changes).
- **Path Handling**: For complex shapes like paths, use bounding box center for position (X/Y) and bounding box dimensions for size (Width/Height). Implement scaling and translation relative to the bounding box center to support parametric-like editing.

## FFI and Unsafe Code
- **Panic in Callbacks**: Panics inside GTK callbacks (FFI boundaries) often result in "panic in a function that cannot unwind" and abort the process.
- **Avoid Unsafe**: Avoid `unsafe { std::mem::transmute_copy }` when dealing with opaque types from external crates (like `gerber-types`). Use safe alternatives like `Debug` formatting or `serde` serialization if available, even if less efficient, to prevent undefined behavior and hard-to-debug crashes.
- **Gerber Types**: `gerber-types` `CoordinateNumber` is opaque. Use `format!("{:?}", c)` to extract values if other traits are missing.

### Parsing Opaque Types via Debug Trait
When working with external crates that expose opaque types (private fields) but implement `Debug`, you can sometimes parse the `Debug` output as a workaround. However, be aware that the `Debug` format can vary (e.g., `Struct { field: value }` vs `Struct(value)`). Always handle multiple formats or inspect the actual output carefully.
- Example: `gerber_types::CoordinateNumber` outputs `CoordinateNumber { nano: 123456 }` but we were expecting `CoordinateNumber(123456)`.
- Solution: Use regex or robust string parsing to extract values from the `Debug` string.

## Cavalier Contours
- `cavalier_contours` operations (like `boolean` and `parallel_offset`) can panic on invalid or degenerate geometry.
- Always wrap these operations in `std::panic::catch_unwind` to prevent application crashes.
- Use `std::panic::AssertUnwindSafe` if necessary.
- Sanitize input geometry (remove duplicates, check orientation) before passing to `cavalier_contours`.
- **Prefer Arcs over Segments**: When creating shapes with rounded parts (like circles or stadiums) for boolean operations, use `PlineVertex` with `bulge` (arcs) instead of linearizing them into many small segments. This significantly reduces vertex count (e.g., 4 vertices vs 36+ for a stadium) and improves stability, preventing panics like `EndPointOnFinalOffsetVertex` during boolean or offset operations.


## Gerber Processing
- **Trace Generation**: Generating individual shapes (rectangles/stadiums) for each Gerber segment (`D01`) leads to disjoint geometry that fails to merge correctly, especially at sharp corners.
- **Solution**: Buffer consecutive `D01` commands into a single continuous `Polyline` (center line). When the path ends (e.g., `D02`, `D03`, aperture change), generate the "stroke" by offsetting the polyline by `aperture_radius` on both sides (`parallel_offset(r)` and `parallel_offset(-r)`), then joining the ends with arcs to form a closed loop. This ensures correct corner handling and continuous geometry.
- **Cavalier Contours**:
    - `parallel_offset(offset)` returns `Vec<Polyline>`. For simple traces, it returns one polyline per side.
    - To form a closed loop from offsets: take right offset, take left offset, invert left offset (`invert_direction_mut`), and connect them with semi-circle caps (bulge = 1.0).
    - `Polyline::set(index, x, y, bulge)` is used to update vertices/bulges.
    - `Polyline::invert()` creates a new inverted polyline. `invert_direction_mut()` inverts in place.
- **Duplicate Vertices**: `cavalier_contours` functions like `parallel_offset` and boolean operations can panic if the input polyline contains duplicate vertices (vertices with the same position). Always call `remove_repeat_pos(epsilon)` (e.g., `1e-4`) on polylines before processing them, especially after boolean operations or when constructing paths from external data.
- **Boolean Operations**: When merging polylines, the result might contain artifacts or duplicate vertices. Clean the result before further processing.
- **Panic Handling**: Wrap `cavalier_contours` operations in `panic::catch_unwind` to prevent the entire application from crashing due to geometry errors. Log the error and skip the problematic polygon if possible.

## Cavalier Contours Insights
- **Duplicate Vertices Panic**: `cavalier_contours` can panic with `bug: input assumed to not have repeat position vertexes` if a polyline has consecutive duplicate vertices or if a closed polyline has the last vertex equal to the first vertex.
- **Fix**: Always clean polylines before performing operations like `parallel_offset` or boolean operations.
  - Use `remove_repeat_pos(epsilon)` to remove consecutive duplicates.
  - For closed polylines, explicitly check if the last vertex equals the first vertex and remove it if so.
  - A helper function `clean_polyline` is recommended to encapsulate this logic.
- **Epsilon**: Use a small epsilon (e.g., `1e-5`) for vertex comparison to handle floating point inaccuracies.
- **Boolean Operations**: Boolean operations can also produce polylines that need cleaning before further processing.
- **FFI Boundaries**: Panics inside GTK callbacks (FFI) cause immediate aborts. It is critical to wrap *all* fallible geometry operations (like `parallel_offset` inside helper closures) in `panic::catch_unwind` to ensure the application survives geometry bugs.

## CSGRS vs Cavalier Contours
- **Stability**: `cavalier_contours` is excellent for offsetting but can be fragile (panics) with degenerate inputs (zero-length segments, duplicate vertices) during boolean operations.
- **Alternative**: `csgrs` (Constructive Solid Geometry for Rust) provides robust boolean operations (Union, Difference, Intersection) that are less prone to panics on complex or degenerate 2D geometry.
- **Strategy**:
    - Use `csgrs` for boolean operations (merging shapes).
    - Use `cavalier_contours` for offsetting (buffering/isolation) on the *clean* result of boolean operations.
    - For "thickening" lines/arcs (Gerber traces), manually construct polygons (rectangles + circles) and Union them with `csgrs` instead of relying on `cavalier_contours` offset of raw centerlines, which is more error-prone.
- **Integration**: `csgrs` uses `nalgebra` for transformations. Ensure version compatibility between `csgrs` and `nalgebra` in your project.

## Gerber Tool Improvements
- Implemented "Remove Excess Copper" (Rubout) feature.
- Uses `csgrs` for boolean difference (Board - Traces).
- Uses `hatch_generator` (scanline fill) to clear the excess area.
- Converts between `cavalier_contours` (Polyline), `csgrs` (Sketch/Geo), and `lyon` (Path) types.
- **Insight**: `cavalier_contours` is great for offsetting, `csgrs` for boolean ops, and `lyon` for path manipulation/hatching. Combining them requires careful type conversion.

## CSG and Polylines
- `csgrs` (and underlying `cavalier_contours`) can be sensitive to duplicate vertices (start == end) in polygons.
- When converting between `Polyline` and `Sketch`, ensure vertices are not duplicated if the library expects implicit closure.
- `Sketch::polygon` likely expects implicit closure (no duplicate start/end point).
- **Sketch::rectangle**: `Sketch::rectangle(w, h, None)` creates a rectangle from `(0, 0)` to `(w, h)`, NOT centered at `(0, 0)`. If you need a centered rectangle, you must translate it by `(-w/2, -h/2)`. If you assume it's centered and translate by `(w/2, h/2)`, you will end up with a rectangle at `(w/2, h/2)` to `(3w/2, 3h/2)`.

## Recent Features
- **Gerber Rubout with Board Outline**: Added an option to use the board outline file (e.g., `*.gko`, `*Edge_Cuts*`) as the boundary for the rubout operation, instead of a simple bounding box. This allows for non-rectangular boards.

## Serde Deserialization
- When adding new fields to a struct that is deserialized from JSON (e.g., configuration files), always consider backward compatibility.
- Use `#[serde(default)]` on the struct or specific fields to allow deserialization to succeed even if the fields are missing in the JSON file. This uses the `Default` implementation for the struct or type.
- Always handle deserialization errors gracefully and log them to help with debugging. Silent failures can be very confusing for users.

### Cavalier Contours Offset Orientation
- `parallel_offset` with a positive value "inflates" the polygon.
- For CCW polygons (standard exterior), this means offsetting outwards (away from center).
- For CW polygons (standard holes), this means offsetting outwards (away from center), which makes the hole *larger*.
- To "shrink" a hole (offset into the void, preserving the solid material around it), you must use a **negative** offset.
- When processing polygons from `csgrs` or `geo`, ensure you distinguish between Exterior (CCW) and Interior (CW) loops and apply the appropriate offset sign.

### Lyon Path Construction for Hatching with Holes
- **CORRECTION**: Do NOT use `.with_svg()` - it returns `WithSvg<Builder>` which has different methods.
- Use regular `Path::builder()` which provides `begin()`, `line_to()`, `close()`, and `build()`.
- **Each polygon (with holes) should become a SEPARATE lyon Path**:
  - Create a new builder for each polygon
  - Build exterior ring: `begin(first_point)`, `line_to()` for rest, `close()`
  - Build interior rings (holes): repeat `begin()`, `line_to()`, `close()` for each hole
  - Call `build()` to finish the path
  - Pass all paths as a Vec to `generate_hatch`
- This ensures the hatch generator properly respects holes - if you put multiple polygons in one Vec with shared builders, the holes won't be recognized.
- Lyon's even-odd fill rule requires each polygon to be its own path for proper hole handling.

## Machine Control UI
- **G53 Button Removal**: The "Use G53 (Machine Coords)" button was removed from the UI as it was deemed redundant or confusing. G53 commands can still be sent via the console if needed.

- **Device Console Logging**:
  - All manual commands (jog, WCS, zeroing, home, unlock, overrides) are now logged to the device console.
  - Streaming G-code commands are logged if they are sent via the kickstart or polling loop.
  - Initialization commands ($I, $$, $10=47) are logged on connection.
  - Soft reset (Ctrl-X) is logged.
  - Pause (!) and Resume (~) are logged.

## Best Practices

## Toolpath Generation
- **Ramping Logic**: When implementing ramping (helical entry or ramp along profile), ensure that the Z depth decreases by a non-zero amount in each pass. If the ramp angle is small or segments are short/rapid-only, the Z drop might be negligible, leading to an infinite loop if the loop condition is `current_z > target_z`. Always add a safety break (max loops or max segments) or fallback to standard step-down if progress is stalled.
- **Pocket Ramping**: For pockets, ramping is typically applied to the entry (helical entry) rather than the entire clearing path. Use `ToolpathSegment` with `start_z` and `z_depth` to define 3D moves (helical arcs or linear ramps).
- **Rotated Shapes**: Toolpath generation for contours now explicitly handles rotated rectangles and circles by generating geometry in unrotated space and applying the rotation transform to the resulting toolpath segments. This ensures correct offsets and geometry for rotated shapes.

## Toolpath Generation for Rotated Shapes
- When generating toolpaths for rotated shapes (especially rectangles/slots), ensure that the rotation is applied around the correct center point.
- For `DesignRectangle`, the vertices are generated relative to the center, so rotation should be applied around `rect.center`.
- `cavalier_contours` handles offsetting of arbitrary polygons, so as long as the input polygon is correctly rotated and positioned, the output toolpath will be correct.
- Ensure that `rotate_point` logic matches the rendering logic (usually CCW rotation).
- Be careful with `Transform` order in `lyon` (usually `T * R` means Rotate then Translate if applied to vectors, or Translate then Rotate if applied to coordinate system? `then_rotate` appends rotation, so `T * R`).

## Ramping and Helical Entry
- Ramping logic should be applied to the generated toolpath segments.
- Ensure that ramping doesn't cause infinite loops if the path is too short or step down is too small.
- Helical entry for pockets requires generating a spiral path.

### Toolpath Generation for Rotated Shapes (Fix)
- **Issue**: Toolpaths for rotated rectangles were being generated axis-aligned.
- **Cause**: The `generate_rectangle_pocket` function was correctly rotating vertices, but potentially the `generate_rectangular_pocket` (axis-aligned optimization) was being called incorrectly or the rotation was not being propagated.
- **Fix**: Verified that `generate_rectangle_pocket` in `toolpath.rs` correctly handles rotation by converting to a polygon and rotating vertices. Added debug logging to trace execution. Ensure that `rect.rotation` is correctly set in the model.
- **Note**: If `rect.rotation` is 0 but the shape is rotated in the UI, check if the `DesignerShape` wrapper has the rotation but the inner `DesignRectangle` does not. The `DesignerState` updates the inner shape's rotation, so this should be correct.

## Toolpath Generation
- **Rotation Handling**: The `rotate_point` function in `model.rs` expects rotation in DEGREES, but `lyon` and internal shape rotation are stored in RADIANS. Always convert radians to degrees before calling `rotate_point` (e.g. `rotation.to_degrees()`). Failure to do so results in negligible rotation (e.g. 45 degrees -> 0.785 radians -> 0.785 degrees).


## Recent Changes (Designer Tools)
- Added `DesignTriangle` and `DesignPolygon` shapes.
- Implemented `DesignerShape` trait for new shapes.
- Updated `Canvas`, `Renderer`, `SvgRenderer`, and `ToolpathGenerator` to support new shapes.
- Added UI tools for Triangle and Polygon in `DesignerToolbox`.
- Updated `PropertiesPanel` to show properties for new shapes.
- **Icons**: Use standard GTK icons (e.g., `media-playback-start-symbolic` for triangle, `emblem-shared-symbolic` for polygon) when custom SVG resources are not available.
- **Shape Creation**: Ensure shapes are created within the bounding box defined by the drag start and end points. For regular polygons, calculate the radius such that the shape fits within the box (e.g., `radius = min(width, height) / 2.0`).
- **Polygon Tool**: Added n-sided polygon tool with configurable side count.
  - **Fix**: Ensure "Sides" property is visible in inspector when a polygon is selected.
  - **Fix**: Ensure polygon creation respects the marquee bounds correctly (radius vs diameter).
  - **Fix**: Polygon rotation offset - Fixed by applying rotation BEFORE translation. When rendering, use `transform.then_rotate(...).then_translate(...)` to rotate around the origin first, then translate to the center position.

## Pocket G-code Generation
- **Rapid Moves Issue**: When generating raster pockets for polygons, the tool was performing a rapid move to (0,0) between scanlines. This was due to a hardcoded `Point::new(0.0, 0.0)` in the `generate_raster_pocket` function that should have been the last point of the toolpath.
- **Fix**: Changed line 843 in `pocket_operations.rs` from using a hardcoded origin to using the last point from the toolpath (`toolpath.segments.last().map(|s| s.end).unwrap_or(Point::new(0.0, 0.0))`). This ensures continuous tool movement without unnecessary rapid returns to origin between passes.

