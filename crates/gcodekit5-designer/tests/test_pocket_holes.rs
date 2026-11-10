//! Regression tests for pocketing paths with holes (even-odd nested contours).
//!
//! A path made of nested closed contours must be pocketed as regions minus holes: the
//! tool keeps a tool-radius clearance from both the outer wall and every hole, holes are
//! left uncut, and an island nested inside a hole is pocketed again.

use gcodekit5_designer::model::{DesignPath, Point};
use gcodekit5_designer::pocket_operations::PocketStrategy;
use gcodekit5_designer::{Toolpath, ToolpathGenerator, ToolpathSegment, ToolpathSegmentType};

/// Axis-aligned square `(min, max)` in mm.
type Square = (f64, f64);

/// Builds a path made of axis-aligned closed squares, alternating winding so that the
/// result is a valid nested shape regardless of fill rule.
fn nested_squares_path(squares: &[Square]) -> DesignPath {
    let mut b = lyon::path::Path::builder();
    for (i, &(lo, hi)) in squares.iter().enumerate() {
        let (lo, hi) = (lo as f32, hi as f32);
        b.begin(lyon::math::point(lo, lo));
        if i % 2 == 0 {
            b.line_to(lyon::math::point(hi, lo));
            b.line_to(lyon::math::point(hi, hi));
            b.line_to(lyon::math::point(lo, hi));
        } else {
            b.line_to(lyon::math::point(lo, hi));
            b.line_to(lyon::math::point(hi, hi));
            b.line_to(lyon::math::point(hi, lo));
        }
        b.end(true);
    }
    DesignPath::from_lyon_path(&b.build())
}

fn generator(strategy: PocketStrategy) -> ToolpathGenerator {
    let mut pocket_gen = ToolpathGenerator::new();
    pocket_gen.set_feed_rate(100.0);
    pocket_gen.set_cut_depth(1.0);
    pocket_gen.set_pocket_strategy(strategy);
    pocket_gen
}

fn cutting_segments(toolpaths: &[Toolpath]) -> Vec<&ToolpathSegment> {
    toolpaths
        .iter()
        .flat_map(|tp| tp.segments.iter())
        .filter(|s| s.segment_type != ToolpathSegmentType::RapidMove)
        .collect()
}

/// Samples the tool-centre positions along a segment (lines and arcs).
fn sample_segment(s: &ToolpathSegment) -> Vec<Point> {
    const N: usize = 64;
    match (s.segment_type, s.center) {
        (ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW, Some(c)) => {
            let r = c.distance_to(&s.start);
            let a0 = (s.start.y - c.y).atan2(s.start.x - c.x);
            let a1 = (s.end.y - c.y).atan2(s.end.x - c.x);
            let tau = std::f64::consts::TAU;
            let mut sweep = (a1 - a0).rem_euclid(tau);
            if s.segment_type == ToolpathSegmentType::ArcCW {
                sweep -= tau;
            }
            if sweep.abs() < 1e-9 {
                sweep = if s.segment_type == ToolpathSegmentType::ArcCW {
                    -tau
                } else {
                    tau
                };
            }
            (0..=N)
                .map(|k| {
                    let a = a0 + sweep * k as f64 / N as f64;
                    Point::new(c.x + r * a.cos(), c.y + r * a.sin())
                })
                .collect()
        }
        _ => (0..=N)
            .map(|k| {
                let t = k as f64 / N as f64;
                Point::new(
                    s.start.x + (s.end.x - s.start.x) * t,
                    s.start.y + (s.end.y - s.start.y) * t,
                )
            })
            .collect(),
    }
}

/// True if `p` lies strictly inside the square `(lo, hi)` grown by `grow` on every side.
fn inside_square(p: &Point, (lo, hi): Square, grow: f64) -> bool {
    p.x > lo - grow && p.x < hi + grow && p.y > lo - grow && p.y < hi + grow
}

/// Euclidean distance from `p` to the filled square `(lo, hi)` (0 inside).
fn distance_to_square(p: &Point, (lo, hi): Square) -> f64 {
    let dx = (lo - p.x).max(0.0).max(p.x - hi);
    let dy = (lo - p.y).max(0.0).max(p.y - hi);
    dx.hypot(dy)
}

/// Asserts that no cutting move enters `hole` expanded by the tool radius (minus `tol`), and
/// that every cutting move stays inside `outer` shrunk by the tool radius (minus `tol`).
fn assert_clearance(cuts: &[&ToolpathSegment], outer: Square, holes: &[Square], radius: f64) {
    let tol = 0.05;
    for s in cuts {
        for p in sample_segment(s) {
            assert!(
                inside_square(&p, outer, -(radius - tol)),
                "cut leaves the outer boundary clearance at {p:?} ({:?} -> {:?})",
                s.start,
                s.end
            );
            for &hole in holes {
                assert!(
                    distance_to_square(&p, hole) > radius - tol,
                    "cut enters hole {hole:?} clearance at {p:?} ({:?} -> {:?})",
                    s.start,
                    s.end
                );
            }
        }
    }
}

#[test]
fn test_path_pocket_square_ring_leaves_hole_uncut() {
    let path = nested_squares_path(&[(0.0, 40.0), (10.0, 30.0)]);
    let pocket_gen = generator(PocketStrategy::ContourParallel);
    let toolpaths = pocket_gen.generate_path_pocket(&path, 1.0, 1.0, 0.0);
    let cuts = cutting_segments(&toolpaths);
    assert!(!cuts.is_empty(), "expected pocket cuts in the ring");

    // Required minimum: no endpoint strictly inside the (slightly shrunk) hole and no
    // segment crossing it.
    let shrunk_hole = (10.0 + 0.1, 30.0 - 0.1);
    for s in &cuts {
        for p in [s.start, s.end] {
            assert!(
                !inside_square(&p, shrunk_hole, 0.0),
                "cut endpoint inside the hole: {p:?}"
            );
        }
        assert!(
            sample_segment(s)
                .iter()
                .all(|p| !inside_square(p, shrunk_hole, 0.0)),
            "cut crosses the hole: {:?} -> {:?}",
            s.start,
            s.end
        );
    }

    // Cuts exist on every side of the ring.
    let in_ring =
        |p: &Point| inside_square(p, (0.0, 40.0), 0.0) && !inside_square(p, (10.0, 30.0), 0.0);
    assert!(cuts.iter().all(|s| in_ring(&s.start) && in_ring(&s.end)));
    assert!(cuts.iter().any(|s| s.start.x < 10.0));
    assert!(cuts.iter().any(|s| s.start.x > 30.0));
    assert!(cuts.iter().any(|s| s.start.y < 10.0));
    assert!(cuts.iter().any(|s| s.start.y > 30.0));

    // Full tool-radius clearance from both boundaries.
    assert_clearance(&cuts, (0.0, 40.0), &[(10.0, 30.0)], 3.175 / 2.0);
}

#[test]
fn test_path_pocket_ring_respects_holes_for_every_strategy() {
    let strategies = [
        PocketStrategy::ContourParallel,
        PocketStrategy::Adaptive,
        PocketStrategy::Raster {
            angle: 0.0,
            bidirectional: true,
        },
        PocketStrategy::Raster {
            angle: 30.0,
            bidirectional: false,
        },
    ];
    for strategy in strategies {
        let path = nested_squares_path(&[(0.0, 40.0), (10.0, 30.0)]);
        let pocket_gen = generator(strategy);
        let toolpaths = pocket_gen.generate_path_pocket(&path, 1.0, 1.0, 0.0);
        let cuts = cutting_segments(&toolpaths);
        assert!(!cuts.is_empty(), "{strategy:?}: expected pocket cuts");
        assert_clearance(&cuts, (0.0, 40.0), &[(10.0, 30.0)], 3.175 / 2.0);
        // The ring is cleared on both sides of the hole.
        assert!(cuts.iter().any(|s| s.start.x < 10.0), "{strategy:?}");
        assert!(cuts.iter().any(|s| s.start.x > 30.0), "{strategy:?}");
    }
}

#[test]
fn test_path_pocket_nested_island_in_hole_is_pocketed() {
    let path = nested_squares_path(&[(0.0, 60.0), (10.0, 50.0), (20.0, 40.0)]);
    for strategy in [
        PocketStrategy::ContourParallel,
        PocketStrategy::Adaptive,
        PocketStrategy::Raster {
            angle: 0.0,
            bidirectional: true,
        },
    ] {
        let pocket_gen = generator(strategy);
        let toolpaths = pocket_gen.generate_path_pocket(&path, 1.0, 1.0, 0.0);
        let cuts = cutting_segments(&toolpaths);

        // The island (20..40) is pocketed again.
        assert!(
            cuts.iter()
                .any(|s| inside_square(&s.start, (20.0, 40.0), 0.0)
                    && inside_square(&s.end, (20.0, 40.0), 0.0)),
            "{strategy:?}: expected cuts inside the island"
        );

        // Nothing is cut in the hole band between 10..50 and 20..40.
        let in_band = |p: &Point| {
            inside_square(p, (10.0 + 0.1, 50.0 - 0.1), 0.0)
                && !inside_square(p, (20.0 - 0.1, 40.0 + 0.1), 0.0)
        };
        for s in &cuts {
            assert!(
                sample_segment(s).iter().all(|p| !in_band(p)),
                "{strategy:?}: cut in the hole band: {:?} -> {:?}",
                s.start,
                s.end
            );
        }
    }
}

#[test]
fn test_path_pocket_holes_keep_multipass_depths() {
    let path = nested_squares_path(&[(0.0, 40.0), (10.0, 30.0)]);
    let mut pocket_gen = generator(PocketStrategy::ContourParallel);
    pocket_gen.set_start_depth(0.0);
    let toolpaths = pocket_gen.generate_path_pocket(&path, 2.0, 0.5, 0.0);
    let depths: Vec<f64> = toolpaths.iter().map(|tp| tp.depth).collect();
    assert_eq!(
        depths.len(),
        4,
        "expected four 0.5 mm passes, got {depths:?}"
    );
    for (got, want) in depths.iter().zip([-0.5, -1.0, -1.5, -2.0]) {
        assert!((got - want).abs() < 1e-9, "depths {depths:?}");
    }
    for tp in &toolpaths {
        assert!(tp.segments.iter().all(|s| s.feed_rate == 100.0));
    }
}

#[test]
fn test_text_pocket_leaves_letter_counter_uncut() {
    use gcodekit5_designer::TextShape;
    use gcodekit5_designer::model::DesignerShape;

    // A large "O": its counter (the inside of the letter) is a hole in the glyph and
    // must not be pocketed. The letter's centre lies in the middle of the counter.
    let text = TextShape::new("O".to_string(), 0.0, 0.0, 40.0);
    let (x1, y1, x2, y2) = text.bounds();
    let (cx, cy) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);

    let mut pocket_gen = ToolpathGenerator::new();
    pocket_gen.set_tool_diameter(1.0);
    pocket_gen.set_feed_rate(500.0);
    pocket_gen.set_cut_depth(1.0);
    pocket_gen.set_step_in(0.4);
    let toolpaths = pocket_gen.generate_text_pocket_toolpath(&text, 1.0);

    let cuts: Vec<_> = toolpaths
        .iter()
        .flat_map(|tp| tp.segments.iter())
        .filter(|s| s.segment_type != ToolpathSegmentType::RapidMove)
        .collect();
    assert!(!cuts.is_empty(), "the letter stroke should be pocketed");
    for s in cuts {
        for p in [s.start, s.end] {
            let d = ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt();
            assert!(
                d > 3.0,
                "cut at ({:.2}, {:.2}) is inside the counter of the O (centre {cx:.2}, {cy:.2})",
                p.x,
                p.y
            );
        }
    }
}

#[test]
fn test_raster_ring_pocket_bounds_retract_count() {
    // A 40 mm square ring with a 20 mm hole. Every scanline that crosses the hole is
    // split into two spans; cutting them line by line would retract on each such line
    // (about two dozen per pass). Finishing one side of the ring before the other must
    // keep the retracts per pass to a small constant.
    for bidirectional in [true, false] {
        let path = nested_squares_path(&[(0.0, 40.0), (10.0, 30.0)]);
        let mut pocket_gen = generator(PocketStrategy::Raster {
            angle: 0.0,
            bidirectional,
        });
        pocket_gen.set_start_depth(0.0);
        let toolpaths = pocket_gen.generate_path_pocket(&path, 2.0, 1.0, 1.0);
        assert_eq!(toolpaths.len(), 2, "expected two depth passes");

        for tp in &toolpaths {
            let retracts = tp
                .segments
                .iter()
                .filter(|s| s.segment_type == ToolpathSegmentType::RapidMove)
                .count();
            assert!(
                retracts <= 4,
                "bidirectional={bidirectional}: {retracts} retracts in one pass"
            );
        }

        // Ordering must not lose coverage or clearance.
        let cuts = cutting_segments(&toolpaths);
        assert_clearance(&cuts, (0.0, 40.0), &[(10.0, 30.0)], 3.175 / 2.0);
        assert!(cuts.iter().any(|s| s.start.x < 10.0));
        assert!(cuts.iter().any(|s| s.start.x > 30.0));
    }
}

#[test]
fn test_raster_large_ring_pocket_orders_spans_quickly() {
    // A 400 mm square ring with a 200 mm hole at a 0.01 mm stepover gives about 40,000
    // scanlines, half of them split by the hole: roughly 60,000 spans. Ordering them by
    // scanning every remaining span for every emitted span took ~1.8 billion visits (about
    // 4 s for the whole pocket in the test profile); indexing the spans by scanline only
    // examines the adjacent scanlines for each connected link.
    let path = nested_squares_path(&[(0.0, 400.0), (100.0, 300.0)]);
    let mut pocket_gen = generator(PocketStrategy::Raster {
        angle: 0.0,
        bidirectional: true,
    });
    pocket_gen.set_start_depth(0.0);

    let started = std::time::Instant::now();
    let toolpaths = pocket_gen.generate_path_pocket(&path, 1.0, 1.0, 0.01);
    let elapsed = started.elapsed();

    assert_eq!(toolpaths.len(), 1, "expected a single depth pass");
    let cuts = cutting_segments(&toolpaths)
        .into_iter()
        .filter(|s| (s.start.y - s.end.y).abs() < 1e-9 && (s.start.x - s.end.x).abs() > 1.0)
        .count();
    assert!(
        cuts >= 10_000,
        "expected at least 10,000 raster spans, got {cuts}"
    );
    let retracts = toolpaths[0]
        .segments
        .iter()
        .filter(|s| s.segment_type == ToolpathSegmentType::RapidMove)
        .count();
    assert!(retracts <= 4, "{retracts} retracts in one pass");
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "raster pocket with {cuts} spans took {elapsed:?}"
    );
}
