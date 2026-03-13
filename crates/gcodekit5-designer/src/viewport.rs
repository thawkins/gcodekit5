//! Viewport and coordinate transformation for canvas rendering.
//!
//! Handles conversion between pixel coordinates (screen space) and world coordinates
//! (design space). Manages zoom and pan operations with proper coordinate mapping.

use std::fmt;

use crate::model::Point;

/// Represents the viewport transformation state (zoom and pan).
#[derive(Debug, Clone)]
pub struct Viewport {
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
    canvas_width: f64,
    canvas_height: f64,
}

impl Viewport {
    /// Creates a new viewport with initial dimensions.
    /// Sets up coordinate system with (0,0) at bottom-left with small margin.
    pub fn new(canvas_width: f64, canvas_height: f64) -> Self {
        const MARGIN: f64 = 5.0; // pixels from edge
        Self {
            zoom: 1.0,
            // Position (0,0) at bottom-left with margin
            pan_x: MARGIN,
            pan_y: MARGIN,
            canvas_width,
            canvas_height,
        }
    }

    /// Gets the canvas width.
    pub fn canvas_width(&self) -> f64 {
        self.canvas_width
    }

    /// Gets the canvas height.
    pub fn canvas_height(&self) -> f64 {
        self.canvas_height
    }

    /// Sets the canvas dimensions (typically called when window resizes).
    pub fn set_canvas_size(&mut self, width: f64, height: f64) {
        self.canvas_width = width;
        self.canvas_height = height;
    }

    /// Gets the current zoom level (1.0 = 100%).
    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// Sets the zoom level, constrained between 0.1 and 50.0.
    pub fn set_zoom(&mut self, zoom: f64) {
        if zoom > 0.1 && zoom < 50.0 {
            self.zoom = zoom;
        }
    }

    /// Zooms in by multiplying current zoom by 1.2.
    pub fn zoom_in(&mut self) {
        self.set_zoom(self.zoom * 1.2);
    }

    /// Zooms out by dividing current zoom by 1.2.
    pub fn zoom_out(&mut self) {
        self.set_zoom(self.zoom / 1.2);
    }

    /// Resets zoom to 1.0 (100%).
    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
    }

    /// Gets the pan offset (X coordinate).
    pub fn pan_x(&self) -> f64 {
        self.pan_x
    }

    /// Gets the pan offset (Y coordinate).
    pub fn pan_y(&self) -> f64 {
        self.pan_y
    }

    /// Sets the pan offset.
    pub fn set_pan(&mut self, x: f64, y: f64) {
        self.pan_x = x;
        self.pan_y = y;
    }

    /// Pans by a delta amount.
    pub fn pan_by(&mut self, dx: f64, dy: f64) {
        self.pan_x += dx;
        self.pan_y += dy;
    }

    /// Resets pan to origin (0, 0).
    pub fn reset_pan(&mut self) {
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    /// Converts pixel coordinates to world coordinates.
    ///
    /// Pixel coordinates are in screen space (0,0 at top-left).
    /// World coordinates are in design space (0,0 at bottom-left).
    ///
    /// The transformation accounts for:
    /// - Pan offset (translation)
    /// - Zoom level (scaling)
    /// - Y-axis flip (screen Y down vs world Y up)
    ///
    /// Formula:
    /// ```text
    /// world_x = (pixel_x - pan_x) / zoom
    /// world_y = (canvas_height - pixel_y - pan_y) / zoom  // Flip Y-axis
    /// ```
    pub fn pixel_to_world(&self, pixel_x: f64, pixel_y: f64) -> Point {
        let world_x = (pixel_x - self.pan_x) / self.zoom;
        // Flip Y-axis: lower pixel Y (top of screen) should map to higher world Y
        let world_y = (self.canvas_height - pixel_y - self.pan_y) / self.zoom;
        Point::new(world_x, world_y)
    }

    /// Converts world coordinates to pixel coordinates.
    ///
    /// World coordinates: (0,0) at bottom-left, +Y goes up, +X goes right
    /// Pixel coordinates: (0,0) at top-left, +Y goes down, +X goes right
    ///
    /// Formula:
    /// ```text
    /// pixel_x = world_x * zoom + pan_x
    /// pixel_y = canvas_height - (world_y * zoom + pan_y)  // Flip Y-axis
    /// ```
    pub fn world_to_pixel(&self, world_x: f64, world_y: f64) -> (f64, f64) {
        let pixel_x = world_x * self.zoom + self.pan_x;
        // Flip Y-axis: higher world Y should map to lower pixel Y (up on screen)
        let pixel_y = self.canvas_height - (world_y * self.zoom + self.pan_y);
        (pixel_x, pixel_y)
    }

    /// Converts world coordinates to pixel coordinates (using Point).
    pub fn world_point_to_pixel(&self, point: &Point) -> (f64, f64) {
        self.world_to_pixel(point.x, point.y)
    }

    /// Fits the given bounding box into the viewport with padding.
    ///
    /// # Arguments
    /// * `min_x`, `min_y` - Bottom-left corner of bounding box (world coordinates)
    /// * `max_x`, `max_y` - Top-right corner of bounding box (world coordinates)
    /// * `padding` - Percentage of viewport to reserve as padding (0.0 - 1.0)
    ///
    /// Centers the content and calculates appropriate zoom level.
    pub fn fit_to_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64, padding: f64) {
        if min_x >= max_x || min_y >= max_y {
            return;
        }

        let width = max_x - min_x;
        let height = max_y - min_y;

        // Calculate zoom to fit content in viewport with padding
        let padding_factor = 1.0 - (padding * 2.0);
        let zoom_x = (self.canvas_width * padding_factor) / width;
        let zoom_y = (self.canvas_height * padding_factor) / height;

        // Use the smaller zoom to fit everything
        let new_zoom = zoom_x.min(zoom_y).clamp(0.1, 50.0);

        // Center the content
        let content_pixel_width = width * new_zoom;
        let content_pixel_height = height * new_zoom;

        let center_pixel_x = self.canvas_width / 2.0 - content_pixel_width / 2.0;
        let center_pixel_y = self.canvas_height / 2.0 - content_pixel_height / 2.0;

        // Calculate pan offsets
        // For X: pixel_x = world_x * zoom + pan_x  =>  pan_x = pixel_x - world_x * zoom
        // For Y: pixel_y = canvas_height - (world_y * zoom + pan_y)  =>  pan_y = canvas_height - pixel_y - world_y * zoom
        self.zoom = new_zoom;
        self.pan_x = center_pixel_x - min_x * new_zoom;
        self.pan_y = self.canvas_height - center_pixel_y - content_pixel_height - min_y * new_zoom;
    }

    /// Fits the viewport to show all content with optional padding.
    /// Equivalent to fit_to_bounds with 5% padding.
    pub fn fit_to_view(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.fit_to_bounds(
            min_x,
            min_y,
            max_x,
            max_y,
            gcodekit5_core::constants::VIEW_PADDING,
        );
    }

    /// Zooms to a point, maintaining that point's screen position.
    ///
    /// Useful for "zoom to cursor" functionality.
    ///
    /// # Arguments
    /// * `world_point` - The world coordinate to zoom to
    /// * `new_zoom` - The new zoom level
    pub fn zoom_to_point(&mut self, world_point: &Point, new_zoom: f64) {
        if new_zoom <= 0.1 || new_zoom >= 50.0 {
            return;
        }

        // Get pixel position of world point (at current zoom/pan)
        let (pixel_x, pixel_y) = self.world_to_pixel(world_point.x, world_point.y);

        // Calculate new pan to keep pixel position fixed
        // For X: pixel_x = world_x * zoom + pan_x  =>  pan_x = pixel_x - world_x * zoom
        // For Y: pixel_y = canvas_height - (world_y * zoom + pan_y)  =>  pan_y = canvas_height - pixel_y - world_y * zoom
        self.zoom = new_zoom;
        self.pan_x = pixel_x - world_point.x * new_zoom;
        self.pan_y = self.canvas_height - pixel_y - world_point.y * new_zoom;
    }

    /// Zooms in at a specific world point (maintaining cursor position).
    pub fn zoom_in_at(&mut self, world_point: &Point) {
        self.zoom_to_point(world_point, self.zoom * 1.2);
    }

    /// Zooms out at a specific world point (maintaining cursor position).
    pub fn zoom_out_at(&mut self, world_point: &Point) {
        self.zoom_to_point(world_point, self.zoom / 1.2);
    }

    /// Centers the viewport on a world coordinate.
    pub fn center_on(&mut self, world_x: f64, world_y: f64) {
        // Center in X: pixel = canvas_width/2, so pan_x = canvas_width/2 - world_x * zoom
        self.pan_x = self.canvas_width / 2.0 - world_x * self.zoom;
        // Center in Y: pixel = canvas_height/2, so canvas_height/2 = canvas_height - (world_y * zoom + pan_y)
        // Solving: pan_y = canvas_height/2 - world_y * zoom
        self.pan_y = self.canvas_height / 2.0 - world_y * self.zoom;
    }

    /// Centers the viewport on a point.
    pub fn center_on_point(&mut self, point: &Point) {
        self.center_on(point.x, point.y);
    }

    /// Resets viewport to default state (1:1 zoom, default pan).
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.pan_x = 5.0;
        self.pan_y = 5.0;
    }
}

impl fmt::Display for Viewport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Zoom: {:.2}x | Pan: ({:.1}, {:.1})",
               self.zoom, self.pan_x, self.pan_y
        )
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new(1200.0, 800.0)
    }
}
