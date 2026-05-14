//! Hatch Generator for Vector Engraving
//!
//! Generates hatch lines (fill) for vector paths using a scanline algorithm.

use lyon::algorithms::aabb::bounding_box;
use lyon::algorithms::path::iterator::PathIterator;
use lyon::math::{point, Angle, Transform};
use lyon::path::Path;

/// Generate hatch lines for a given path
///
/// # Arguments
/// * `path` - The closed path to fill
/// * `angle_degrees` - Angle of hatch lines in degrees
/// * `spacing` - Spacing between hatch lines in mm
/// * `tolerance` - Tolerance for path flattening
///
/// # Returns
/// A vector of paths representing the hatch lines
pub fn generate_hatch(path: &Path, angle_degrees: f32, spacing: f32, tolerance: f32) -> Vec<Path> {
    if spacing <= 0.0 {
        return Vec::new();
    }

    // 1. Rotate path so hatch lines are horizontal (0 degrees)
    // We rotate the path by -angle, generate horizontal lines, then rotate lines back by +angle
    let rotation = Angle::degrees(-angle_degrees);
    let transform = Transform::rotation(rotation);
    let rotated_path = path.clone().transformed(&transform);

    // 2. Calculate bounding box of rotated path
    if rotated_path.iter().count() == 0 {
        return Vec::new();
    }
    let aabb = bounding_box(rotated_path.iter());

    // 3. Generate horizontal scanlines
    let start_y = aabb.min.y;
    let end_y = aabb.max.y;
    let num_lines = ((end_y - start_y) / spacing).ceil() as usize;

    let mut hatch_paths = Vec::new();
    let inverse_transform = transform.inverse().unwrap_or(Transform::identity());

    // Flatten path to line segments for intersection testing
    let mut segments = Vec::new();
    let mut start = point(0.0, 0.0);

    for event in rotated_path.iter().flattened(tolerance) {
        match event {
            lyon::path::Event::Begin { at } => {
                start = at;
            }
            lyon::path::Event::Line { from, to } => {
                segments.push((from, to));
            }
            lyon::path::Event::End { last, close, .. } if close => {
                segments.push((last, start));
            }
            _ => {}
        }
    }

    for i in 0..=num_lines {
        let y = start_y + i as f32 * spacing;

        // Find intersections with all segments
        let mut intersections = Vec::new();

        for (p1, p2) in &segments {
            // Check if segment crosses scanline y
            // We handle horizontal segments by ignoring them (they don't contribute to crossing count for filling)
            if (p1.y <= y && p2.y > y) || (p2.y <= y && p1.y > y) {
                // Calculate X intersection
                // x = x1 + (y - y1) * (x2 - x1) / (y2 - y1)
                let t = (y - p1.y) / (p2.y - p1.y);
                let x = p1.x + t * (p2.x - p1.x);
                intersections.push(x);
            }
        }

        // Sort intersections by X
        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Create line segments from pairs of intersections (Even-Odd rule)
        for chunk in intersections.chunks(2) {
            if chunk.len() == 2 {
                let x1 = chunk[0];
                let x2 = chunk[1];

                // Skip short segments
                if (x2 - x1).abs() < 0.001 {
                    continue;
                }

                // Create horizontal line segment
                let start = point(x1, y);
                let end = point(x2, y);

                // Rotate back to original orientation
                let mut builder = Path::builder();
                builder.begin(inverse_transform.transform_point(start));
                builder.line_to(inverse_transform.transform_point(end));
                builder.end(false);
                hatch_paths.push(builder.build());
            }
        }
    }

    hatch_paths
}
