# GTK4 & Rust Insights

## Viewport & Canvas Sizing
- **Issue**: When using a `DrawingArea` with a backend `Viewport` or `Canvas` struct that manages zoom/pan, the backend needs to be explicitly updated with the widget's dimensions.
- **Solution**: Use `widget.set_draw_func` to update the backend size on every draw (which happens on resize), or use `connect_resize` / `connect_map`.
- **Gotcha**: `DrawingArea` dimensions might be 0 or default during early initialization. Use `connect_map` with a small timeout or check dimensions before applying "Fit to View" logic to ensure correct aspect ratio and padding.

## Coordinate Systems
- **Designer**: Uses Cartesian coordinates (Y-up).
- **GTK/Cairo**: Uses Screen coordinates (Y-down).
- **Transformation**: Always handle Y-flip in the `Viewport` logic.

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

### Unit Handling in CAM Tools
- **Dimension Inputs**: Use `create_dimension_row` helper to create input rows with dynamic unit labels.
- **Parsing**: Use `units::parse_length(text, system)` to parse user input into standard units (mm).
- **Formatting**: Use `units::format_length(value, system)` to display values in the user's preferred unit.
- **Updates**: Register a listener on `settings.persistence` to update UI labels and values when the measurement system changes.
- **Storage**: Store parameters in standard units (mm) or raw values if appropriate, but ensure consistent interpretation.
