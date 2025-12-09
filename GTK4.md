# GTK4 & Rust Insights

## Viewport & Canvas Sizing
- **Issue**: When using a `DrawingArea` with a backend `Viewport` or `Canvas` struct that manages zoom/pan, the backend needs to be explicitly updated with the widget's dimensions.
- **Solution**: Use `widget.set_draw_func` to update the backend size on every draw (which happens on resize), or use `connect_resize` / `connect_map`.
- **Gotcha**: `DrawingArea` dimensions might be 0 or default during early initialization. Use `connect_map` with a small timeout or check dimensions before applying "Fit to View" logic to ensure correct aspect ratio and padding.

## Coordinate Systems
- **Designer**: Uses Cartesian coordinates (Y-up).
- **GTK/Cairo**: Uses Screen coordinates (Y-down).
- **Transformation**: Always handle Y-flip in the `Viewport` logic.
