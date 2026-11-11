//! Regression tests for the height-map stock removal simulator: cut placement,
//! exact depths, stock size, XY origin, rapids, progress/cancel and the
//! surface mesh (watertight, merged, correctly wound).

use gcodekit5_visualizer::visualizer::{StockXyReference, StockZeroReference};
use gcodekit5_visualizer::{
    StockSimulator3D, ToolpathSegment, ToolpathSegmentType, Visualizer, toolpath_from_commands,
};

fn segment(
    kind: ToolpathSegmentType,
    start: (f32, f32, f32),
    end: (f32, f32, f32),
) -> ToolpathSegment {
    ToolpathSegment {
        segment_type: kind,
        start,
        end,
        center: None,
    }
}

fn linear(start: (f32, f32, f32), end: (f32, f32, f32)) -> ToolpathSegment {
    segment(ToolpathSegmentType::LinearMove, start, end)
}

/// Straight plunge from well above any test stock down to `z`.
fn plunge(x: f32, y: f32, z: f32) -> ToolpathSegment {
    linear((x, y, 20.0), (x, y, z))
}

/// Enclosed volume of a triangle mesh (10 floats per vertex) by the
/// divergence theorem; only correct for a closed, outward-wound mesh.
fn mesh_volume(mesh: &[f32]) -> f32 {
    let vertices: Vec<glam::Vec3> = mesh
        .as_chunks::<10>()
        .0
        .iter()
        .map(|v| glam::Vec3::new(v[0], v[1], v[2]))
        .collect();
    vertices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| t[0].dot(t[1].cross(t[2])) / 6.0)
        .sum()
}

/// Axis-aligned bounds of a mesh: (min, max).
fn mesh_bounds(mesh: &[f32]) -> (glam::Vec3, glam::Vec3) {
    mesh.as_chunks::<10>().0.iter().fold(
        (
            glam::Vec3::splat(f32::INFINITY),
            glam::Vec3::splat(f32::NEG_INFINITY),
        ),
        |(lo, hi), v| {
            let p = glam::Vec3::new(v[0], v[1], v[2]);
            (lo.min(p), hi.max(p))
        },
    )
}

#[test]
fn test_stock_removal_cut_is_centred_on_the_tool() {
    // 1 mm cells, r = 1 tool at (5, 5): the four cells whose centres
    // (4.5 / 5.5) lie within 1 mm are cut, symmetrically about the tool.
    let mut sim = StockSimulator3D::new(10.0, 10.0, 5.0, 1.0, 1.0).expect("valid stock");
    sim.simulate_toolpath(&[plunge(5.0, 5.0, -1.0)]);
    let map = sim.height_map();
    for (col, row) in [(4, 4), (5, 4), (4, 5), (5, 5)] {
        assert_eq!(map.height_at(col, row), Some(4.0), "cell {col},{row}");
    }
    for (col, row) in [(3, 4), (6, 4), (4, 3), (4, 6), (6, 6)] {
        assert_eq!(map.height_at(col, row), Some(5.0), "cell {col},{row}");
    }
}

#[test]
fn test_stock_removal_depth_is_not_quantised() {
    let mut sim = StockSimulator3D::new(10.0, 10.0, 5.0, 0.5, 1.0).expect("valid stock");
    sim.simulate_toolpath(&[plunge(5.0, 5.0, -1.23)]);
    let h = sim.height_map().height_at(10, 10).expect("inside map");
    assert!((h - 3.77).abs() < 1e-4, "height {h}");
}

#[test]
fn test_stock_removal_cut_through_stops_at_bottom() {
    let mut sim = StockSimulator3D::new(10.0, 10.0, 5.0, 0.5, 1.0).expect("valid stock");
    sim.simulate_toolpath(&[plunge(5.0, 5.0, -8.0)]);
    assert_eq!(sim.height_map().height_at(10, 10), Some(0.0));
}

#[test]
fn test_stock_removal_mesh_matches_stock_size_exactly() {
    // 10 mm is not a whole number of 0.19 mm cells; the mesh must not grow.
    let sim = StockSimulator3D::new(10.0, 10.0, 10.0, 0.19, 1.0).expect("valid stock");
    let (lo, hi) = mesh_bounds(&sim.get_mesh());
    assert!(
        lo.abs_diff_eq(glam::Vec3::new(0.0, 0.0, -10.0), 1e-4),
        "{lo}"
    );
    assert!(
        hi.abs_diff_eq(glam::Vec3::new(10.0, 10.0, 0.0), 1e-4),
        "{hi}"
    );
}

#[test]
fn test_stock_removal_uncut_stock_is_one_box() {
    // Greedy merging: an uncut block is 6 quads = 36 vertices.
    let sim = StockSimulator3D::new(200.0, 200.0, 10.0, 0.19, 1.0).expect("valid stock");
    let mesh = sim.get_mesh();
    assert_eq!(mesh.len(), 36 * 10);
    assert!((mesh_volume(&mesh) - 200.0 * 200.0 * 10.0).abs() < 1.0);
}

#[test]
fn test_stock_removal_mesh_is_closed_and_outward() {
    // A pocket and a through-cut: the mesh volume must equal the summed
    // column volumes, which only holds for a watertight, outward-wound mesh.
    let mut sim = StockSimulator3D::new(20.0, 10.0, 5.0, 0.5, 1.5).expect("valid stock");
    sim.simulate_toolpath(&[
        linear((3.0, 3.0, -2.0), (15.0, 7.0, -2.0)),
        plunge(17.0, 5.0, -6.0),
    ]);
    let map = sim.height_map();
    let cell_area = map.resolution() * map.resolution();
    let expected: f32 = map.heights().iter().map(|h| h * cell_area).sum();
    let volume = mesh_volume(&sim.get_mesh());
    assert!((volume - expected).abs() < 1e-2, "{volume} vs {expected}");
    assert!(volume < 20.0 * 10.0 * 5.0);
}

#[test]
fn test_stock_removal_centre_xy_reference_places_stock_around_origin() {
    let toolpath = [plunge(-3.5, -3.5, -1.0)];

    let mut centred = StockSimulator3D::new(10.0, 10.0, 5.0, 1.0, 0.6)
        .expect("valid stock")
        .with_stock_xy_reference(StockXyReference::Centre);
    centred.simulate_toolpath(&toolpath);
    // Work (-3.5, -3.5) is local (1.5, 1.5), the centre of cell (1, 1).
    assert_eq!(centred.height_map().height_at(1, 1), Some(4.0));
    let (lo, hi) = mesh_bounds(&centred.get_mesh());
    assert!(
        lo.abs_diff_eq(glam::Vec3::new(-5.0, -5.0, -5.0), 1e-4),
        "{lo}"
    );
    assert!(hi.abs_diff_eq(glam::Vec3::new(5.0, 5.0, 0.0), 1e-4), "{hi}");

    let mut lower_left = StockSimulator3D::new(10.0, 10.0, 5.0, 1.0, 0.6)
        .expect("valid stock")
        .with_stock_xy_reference(StockXyReference::LowerLeft);
    lower_left.simulate_toolpath(&toolpath);
    assert!(lower_left.height_map().heights().iter().all(|&h| h == 5.0));
}

#[test]
fn test_stock_removal_xy_reference_index_round_trips() {
    assert_eq!(StockXyReference::default(), StockXyReference::LowerLeft);
    for r in [StockXyReference::LowerLeft, StockXyReference::Centre] {
        assert_eq!(StockXyReference::from_index(r.index()), r);
    }
}

#[test]
fn test_stock_removal_rapid_through_stock_cuts() {
    let mut sim = StockSimulator3D::new(10.0, 10.0, 5.0, 0.5, 1.0).expect("valid stock");
    sim.simulate_toolpath(&[segment(
        ToolpathSegmentType::RapidMove,
        (2.0, 5.0, -1.0),
        (8.0, 5.0, -1.0),
    )]);
    assert_eq!(sim.height_map().height_at(10, 10), Some(4.0));
}

#[test]
fn test_stock_removal_rejects_bad_tool_diameter() {
    for radius in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(
            StockSimulator3D::new(10.0, 10.0, 5.0, 0.5, radius).is_err(),
            "{radius}"
        );
    }
}

#[test]
fn test_stock_removal_cancel_leaves_stock_untouched() {
    let mut sim = StockSimulator3D::new(10.0, 10.0, 5.0, 0.5, 1.0).expect("valid stock");
    let completed = sim
        .simulate_toolpath_with_progress(&[linear((2.0, 5.0, -2.0), (8.0, 5.0, -2.0))], |_| false);
    assert!(!completed);
    assert!(sim.height_map().heights().iter().all(|&h| h == 5.0));
}

#[test]
fn test_stock_removal_progress_is_monotonic_and_completes() {
    let mut sim = StockSimulator3D::new(50.0, 50.0, 5.0, 0.2, 1.0).expect("valid stock");
    let mut reports = Vec::new();
    let completed = sim.simulate_toolpath_with_progress(
        &[
            linear((0.0, 10.0, -1.0), (50.0, 10.0, -1.0)),
            segment(
                ToolpathSegmentType::RapidMove,
                (50.0, 10.0, 5.0),
                (0.0, 40.0, 5.0),
            ),
            linear((0.0, 40.0, -1.0), (50.0, 40.0, -1.0)),
        ],
        |p| {
            reports.push(p);
            true
        },
    );
    assert!(completed);
    assert_eq!(reports.first(), Some(&0.0));
    assert_eq!(reports.last(), Some(&1.0));
    assert!(reports.windows(2).all(|w| w[0] <= w[1]), "{reports:?}");
}

#[test]
fn test_stock_removal_arc_cuts_along_the_arc() {
    // Half circle of radius 4 around (5, 5), from (1, 5) to (9, 5) via (5, 9).
    let mut sim = StockSimulator3D::new(10.0, 10.0, 5.0, 0.5, 0.5).expect("valid stock");
    sim.simulate_toolpath(&[ToolpathSegment {
        segment_type: ToolpathSegmentType::ArcCW,
        start: (1.0, 5.0, -1.0),
        end: (9.0, 5.0, -1.0),
        center: Some((5.0, 5.0)),
    }]);
    let map = sim.height_map();
    // (5, 9) lies on the arc (cell 10, 18 has centre 5.25, 9.25); the centre
    // and the unswept lower half are untouched.
    assert_eq!(map.height_at(10, 18), Some(4.0));
    assert_eq!(map.height_at(10, 10), Some(5.0));
    assert_eq!(map.height_at(10, 2), Some(5.0));
}

#[test]
fn test_toolpath_from_commands_keeps_motion_and_drops_dwells() {
    let mut vis = Visualizer::new();
    vis.parse_gcode("G0 X0 Y0 Z5\nG1 Z-1 F100\nG4 P1\nG2 X10 Y0 I5 J0\n");
    let segments = toolpath_from_commands(vis.commands());
    assert!(segments.len() >= 3, "{segments:?}");
    assert_eq!(segments[0].segment_type, ToolpathSegmentType::RapidMove);
    assert_eq!(segments[1].segment_type, ToolpathSegmentType::LinearMove);
    assert!(
        segments[2..]
            .iter()
            .all(|s| s.segment_type == ToolpathSegmentType::ArcCW && s.center.is_some())
    );
    // The arc pieces join up and end at the programmed end point.
    let end = segments.last().expect("arc").end;
    assert!((end.0 - 10.0).abs() < 1e-3 && end.1.abs() < 1e-3, "{end:?}");
}

#[test]
fn test_stock_zero_reference_still_applies_with_xy_reference() {
    let mut sim = StockSimulator3D::new(10.0, 10.0, 10.0, 1.0, 1.0)
        .expect("valid stock")
        .with_stock_zero_reference(StockZeroReference::Bottom)
        .with_stock_xy_reference(StockXyReference::Centre);
    sim.simulate_toolpath(&[plunge(0.0, 0.0, 7.0)]);
    assert_eq!(sim.stock_top_z(), 10.0);
    assert_eq!(sim.height_map().height_at(5, 5), Some(7.0));
}
