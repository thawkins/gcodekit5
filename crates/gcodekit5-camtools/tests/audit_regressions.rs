//! Regression tests for functional bugs found during the camtools audit.
//!
//! Each test pins down a concrete wrong-G-code / wrong-geometry scenario.

use gcodekit5_camtools::arc_expander::ArcExpander;
use gcodekit5_camtools::gerber::{GerberConverter, GerberLayerType, GerberParameters};
use gcodekit5_camtools::jigsaw_puzzle::{JigsawPuzzleMaker, PuzzleParameters};
use gcodekit5_camtools::laser_engraver::{BitmapImageEngraver, EngravingParameters};
use gcodekit5_camtools::probe_routines::ProbeRoutineEngine;
use gcodekit5_camtools::speeds_feeds::{CalculationInput, SpeedsFeedsCalculator};
use gcodekit5_camtools::spoilboard_grid::{SpoilboardGridGenerator, SpoilboardGridParameters};
use gcodekit5_camtools::spoilboard_surfacing::{
    SpoilboardSurfacingGenerator, SpoilboardSurfacingParameters,
};
use gcodekit5_camtools::tabbed_box::{BoxParameters, TabbedBoxMaker};
use gcodekit5_camtools::wcs_service::{WcsUpdateConfig, WcsUpdateService};
use gcodekit5_core::{Corner, Position, ProbeReport, ProbeResult, ProbeRoutine, Units};
use image::{DynamicImage, GrayImage, Luma};

/// Collect `(x, y)` from every `G0`/`G1` line that has both words.
fn xy_moves(gcode: &str) -> Vec<(f32, f32)> {
    gcode
        .lines()
        .filter(|l| l.starts_with("G0 ") || l.starts_with("G1 "))
        .filter_map(|l| {
            let word = |c: char| {
                l.split_whitespace()
                    .find(|w| w.starts_with(c))
                    .and_then(|w| w[1..].parse::<f32>().ok())
            };
            Some((word('X')?, word('Y')?))
        })
        .collect()
}

fn copper_params() -> GerberParameters {
    GerberParameters {
        layer_type: GerberLayerType::TopCopper,
        tool_diameter: 0.2,
        ..Default::default()
    }
}

#[test]
fn test_gerber_format_2_4_coordinates_scaled_correctly() {
    // X10 Y10 mm encoded in 2.4 format; used to come out 100x too large.
    let gerber = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,2.0*%\nD10*\nX100000Y100000D03*\nM02*\n";
    let gcode = GerberConverter::generate(&copper_params(), gerber)
        .expect("regression test setup should succeed");
    let moves = xy_moves(&gcode);
    assert!(!moves.is_empty());
    for (x, y) in moves {
        assert!(x < 20.0 && y < 20.0, "coordinate ({x}, {y}) not near 10,10");
    }
}

#[test]
fn test_gerber_isolation_offsets_away_from_copper() {
    // 2mm round pad at (10,10): isolation path must be at r = 1 + 0.1, not 1 - 0.1.
    let gerber = "%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,2.0*%\nD10*\nX10000000Y10000000D03*\nM02*\n";
    let gcode = GerberConverter::generate(&copper_params(), gerber)
        .expect("regression test setup should succeed");
    for (x, y) in xy_moves(&gcode) {
        let r = ((x - 10.0).powi(2) + (y - 10.0).powi(2)).sqrt();
        assert!(r > 1.05, "isolation path cuts into the pad (r = {r})");
    }
}

#[test]
fn test_gerber_isolation_hole_offsets_into_hole() {
    // Square ring of 0.5mm traces from (0,0) to (10,10): the inner isolation loop must lie
    // inside the hole, i.e. at least 0.25 + 0.1 from the trace centreline.
    let gerber = "%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX0Y0D02*\n\
                  X10000000Y0D01*\nX10000000Y10000000D01*\nX0Y10000000D01*\nX0Y0D01*\nM02*\n";
    let gcode = GerberConverter::generate(&copper_params(), gerber)
        .expect("regression test setup should succeed");
    for (x, y) in xy_moves(&gcode) {
        let inside = x > 0.0 && x < 10.0 && y > 0.0 && y < 10.0;
        if inside {
            let d = x.min(10.0 - x).min(y).min(10.0 - y);
            assert!(d > 0.3, "inner isolation at ({x}, {y}) cuts the trace");
        }
    }
}

#[test]
fn test_gerber_offset_applied_once() {
    let params = GerberParameters {
        offset_x: 20.0,
        offset_y: 30.0,
        ..copper_params()
    };
    let gerber = "%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,2.0*%\nD10*\nX10000000Y10000000D03*\nM02*\n";
    let gcode =
        GerberConverter::generate(&params, gerber).expect("regression test setup should succeed");
    // Work zero must not be shifted by the offset (it is already in every coordinate).
    assert!(!gcode.contains("G0 X20.000 Y30.000"));
    let (x, y) = xy_moves(&gcode)[0];
    assert!(
        (x - 31.1).abs() < 0.2 && (y - 40.0).abs() < 0.2,
        "got ({x}, {y})"
    );
}

fn engraver_params() -> EngravingParameters {
    EngravingParameters {
        width_mm: 0.4,
        pixels_per_mm: 10.0,
        bidirectional: true,
        offset_x: 0.0,
        offset_y: 0.0,
        ..Default::default()
    }
}

/// White image with one black (burned) column at `x`.
fn single_pixel_image(width: u32, height: u32, x: u32) -> DynamicImage {
    let mut img = GrayImage::from_pixel(width, height, Luma([255]));
    for y in 0..height {
        img.put_pixel(x, y, Luma([0]));
    }
    DynamicImage::ImageLuma8(img)
}

#[test]
fn test_laser_single_pixel_burns_full_pixel_width() {
    let engraver = BitmapImageEngraver::from_image(single_pixel_image(4, 1, 1), engraver_params())
        .expect("regression test setup should succeed");
    let gcode = engraver
        .generate_gcode()
        .expect("regression test setup should succeed");
    // Pixel 1 spans X0.1..X0.2; previously this was a zero-length G1.
    assert!(gcode.contains("G0 X0.100 Y0.000 S0"), "{gcode}");
    assert!(gcode.contains("G1 X0.200 S"), "{gcode}");
}

#[test]
fn test_laser_reverse_pass_travels_right_to_left() {
    let engraver = BitmapImageEngraver::from_image(single_pixel_image(4, 2, 1), engraver_params())
        .expect("regression test setup should succeed");
    let gcode = engraver
        .generate_gcode()
        .expect("regression test setup should succeed");
    // Second (reverse) line must start at the right edge of the pixel and burn leftward.
    assert!(gcode.contains("G0 X0.200 Y0.100 S0"), "{gcode}");
    assert!(gcode.contains("G1 X0.100 S"), "{gcode}");
}

#[test]
fn test_laser_line_spacing_skips_lines_instead_of_stretching() {
    let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(4, 4, Luma([0])));
    let params = EngravingParameters {
        line_spacing: 2.0,
        ..engraver_params()
    };
    let gcode = BitmapImageEngraver::from_image(img, params)
        .expect("regression test setup should succeed")
        .generate_gcode()
        .expect("regression test setup should succeed");
    let max_y = xy_moves(&gcode)
        .iter()
        .map(|&(_, y)| y)
        .fold(0.0f32, f32::max);
    assert!(
        max_y < 0.4,
        "image stretched beyond its 0.4mm height: {max_y}"
    );
}

#[test]
fn test_laser_rejects_invalid_parameters() {
    let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(4, 4, Luma([255])));
    for params in [
        EngravingParameters {
            line_spacing: 0.0,
            ..engraver_params()
        },
        EngravingParameters {
            pixels_per_mm: 0.0,
            ..engraver_params()
        },
        EngravingParameters {
            width_mm: 0.01,
            ..engraver_params()
        },
    ] {
        assert!(BitmapImageEngraver::from_image(img.clone(), params).is_err());
    }
}

#[test]
fn test_spoilboard_zero_stepover_or_spacing_errors_instead_of_hanging() {
    let surfacing = SpoilboardSurfacingParameters {
        width: 100.0,
        height: 100.0,
        tool_diameter: 20.0,
        feed_rate: 1000.0,
        spindle_speed: 10000.0,
        cut_depth: 0.5,
        stepover_percent: 0.0,
        safe_z: 5.0,
        home_before_job: false,
    };
    assert!(SpoilboardSurfacingGenerator::new(surfacing)
        .generate()
        .is_err());

    let grid = SpoilboardGridParameters {
        width: 100.0,
        height: 100.0,
        grid_spacing: 0.0,
        feed_rate: 1000.0,
        laser_power: 100.0,
        laser_mode: "M4".to_string(),
        home_before_job: false,
    };
    assert!(SpoilboardGridGenerator::new(grid).generate().is_err());
}

#[test]
fn test_spoilboard_surfacing_keeps_z_work_offset() {
    let params = SpoilboardSurfacingParameters {
        width: 100.0,
        height: 50.0,
        tool_diameter: 20.0,
        feed_rate: 1000.0,
        spindle_speed: 10000.0,
        cut_depth: 0.5,
        stepover_percent: 50.0,
        safe_z: 5.0,
        home_before_job: true,
    };
    let gcode = SpoilboardSurfacingGenerator::new(params)
        .generate()
        .expect("regression test setup should succeed");
    assert!(!gcode
        .lines()
        .any(|l| l.starts_with("G10") && l.contains("Z0")));
}

#[test]
fn test_probe_backoff_uses_grbl_compatible_relative_moves() {
    let routine = ProbeRoutine::ZTouch {
        safe_height: 5.0,
        max_depth: 20.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 2.0,
    };
    let out = ProbeRoutineEngine::new(routine)
        .generate(Position::new(0.0, 0.0, 5.0))
        .expect("regression test setup should succeed");
    assert!(!out.gcode.contains("PRB_"), "{}", out.gcode);
    assert!(out.gcode.contains("G91"));
    assert!(out.gcode.contains("G0 Z2.000"));
    assert!(out.gcode.contains("G38.2 Z-2.500 F25.0"));
    // The whole routine is incremental; absolute mode is restored as the last step.
    let g91 = out
        .gcode
        .find("G91")
        .expect("regression test setup should succeed");
    let first_probe = out
        .gcode
        .find("G38.2")
        .expect("regression test setup should succeed");
    assert!(g91 < first_probe);
    let last_code_line = out
        .gcode
        .lines()
        .rfind(|l| !l.starts_with(';') && !l.trim().is_empty())
        .expect("regression test setup should succeed");
    assert!(last_code_line.starts_with("G90"), "{}", out.gcode);
}

#[test]
fn test_probe_corner_find_y_probe_is_not_zero_length() {
    let routine = ProbeRoutine::CornerFind {
        corner: Corner::XminYmin,
        safe_height: 5.0,
        probe_distance: 10.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 0.0,
    };
    let out = ProbeRoutineEngine::new(routine)
        .generate(Position::new(50.0, 50.0, 5.0))
        .expect("regression test setup should succeed");
    // Relative Y probe of the full probe distance toward Y-min.
    assert!(out.gcode.contains("G38.2 Y-10.000"), "{}", out.gcode);
}

#[test]
fn test_probe_corner_result_without_backoff_uses_x_trigger() {
    let routine = ProbeRoutine::CornerFind {
        corner: Corner::XminYmin,
        safe_height: 5.0,
        probe_distance: 10.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 0.0,
    };
    let mut report = ProbeReport::new(routine);
    report.add_trigger(ProbeResult::new(
        Position::new(10.0, 25.0, -5.0),
        true,
        Units::MM,
    ));
    report.add_trigger(ProbeResult::new(
        Position::new(15.0, 20.0, -5.0),
        true,
        Units::MM,
    ));
    let corner =
        ProbeRoutineEngine::compute_result(&report).expect("regression test setup should succeed");
    assert!((corner.x - 10.0).abs() < 1e-4);
    assert!((corner.y - 20.0).abs() < 1e-4);
}

#[test]
fn test_probe_boss_center_probes_toward_boss() {
    let routine = ProbeRoutine::BossCenter {
        diameter: 20.0,
        safe_height: 5.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
    };
    let out = ProbeRoutineEngine::new(routine)
        .generate(Position::new(50.0, 50.0, 5.0))
        .expect("regression test setup should succeed");
    // Move out to the left (X-12 = radius + 2 mm gap), then probe +12 back toward the
    // centre, never away from it.
    let lines: Vec<&str> = out.gcode.lines().collect();
    let left = lines
        .iter()
        .position(|l| l.starts_with("G0 X-12.000"))
        .expect("left approach move");
    let probe = lines
        .iter()
        .position(|l| l.starts_with("G38.2 X"))
        .expect("first X probe");
    assert!(left < probe, "{}", out.gcode);
    assert!(lines[probe].starts_with("G38.2 X12.000"), "{}", out.gcode);
}

#[test]
fn test_wcs_g10_uses_p1_for_g54_and_only_measured_axes() {
    let mut service = WcsUpdateService::new(WcsUpdateConfig::default());
    let routine = ProbeRoutine::BoreCenter {
        diameter: 20.0,
        safe_height: 5.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
    };
    let mut report = ProbeReport::new(routine);
    for (x, y) in [(40.0, 50.0), (60.0, 50.0), (50.0, 40.0), (50.0, 60.0)] {
        report.add_trigger(ProbeResult::new(Position::new(x, y, -5.0), true, Units::MM));
    }
    let result = service
        .process_probe_report(&report)
        .expect("regression test setup should succeed");
    assert_eq!(result.command, "G10 L2 P1 X50.000 Y50.000");
}

#[test]
fn test_jigsaw_passes_step_down_from_plunge_depth() {
    let mut maker = JigsawPuzzleMaker::new(PuzzleParameters::default())
        .expect("regression test setup should succeed");
    maker
        .generate()
        .expect("regression test setup should succeed");
    let gcode = maker.to_gcode(500.0, 1.0);
    // Default: 3 passes, 0.5 step: Z-1.00 (plunge), Z-1.50, Z-2.00 — never back up to Z-0.50.
    assert!(gcode.contains("G0 Z-1.50 ; Move to pass depth"));
    assert!(gcode.contains("G0 Z-2.00 ; Move to pass depth"));
    assert!(!gcode.contains("G0 Z-0.50 ; Move to pass depth"));
}

#[test]
fn test_tabbed_box_each_panel_starts_at_first_pass_height() {
    let mut maker = TabbedBoxMaker::new(BoxParameters::default())
        .expect("regression test setup should succeed");
    maker
        .generate()
        .expect("regression test setup should succeed");
    let gcode = maker.to_gcode();
    let mut z = 0.0f32;
    for line in gcode.lines() {
        if line.starts_with("; Pass 1/") {
            assert!(z.abs() < 1e-6, "panel started at Z{z} instead of Z0");
        }
        if let Some(rest) = line.strip_prefix("G0 Z") {
            z = rest
                .split_whitespace()
                .next()
                .expect("regression test setup should succeed")
                .parse()
                .expect("regression test setup should succeed");
            if line.contains("safe height") {
                z = 0.0; // initial safe-height move precedes the first panel
            }
        }
    }
}

#[test]
fn test_tabbed_box_divider_slot_pitch_matches_fingers() {
    let params = BoxParameters {
        dividers_x: 1,
        burn: 0.2,
        ..Default::default()
    };
    let t = params.thickness;
    let kerf = params.burn;
    let pitch = (params.finger_joint.finger + params.finger_joint.space) * t;
    let slot_h = params.finger_joint.finger * t - kerf;
    let mut maker = TabbedBoxMaker::new(params).expect("regression test setup should succeed");
    maker
        .generate()
        .expect("regression test setup should succeed");

    // Vertical slot rectangles in the first wall: width t - kerf, height finger - kerf
    let mut slots: Vec<(f32, f32)> = maker
        .paths()
        .iter()
        .filter(|p| p.len() == 5)
        .filter_map(|p| {
            let w = p[1].x - p[0].x;
            let h = p[2].y - p[1].y;
            ((w - (t - kerf)).abs() < 1e-3 && (h - slot_h).abs() < 1e-3).then_some((p[0].x, p[0].y))
        })
        .collect();
    assert!(slots.len() >= 2);
    let first_x = slots[0].0;
    slots.retain(|s| (s.0 - first_x).abs() < 1e-3);
    slots.sort_by(|a, b| a.1.total_cmp(&b.1));
    for pair in slots.windows(2) {
        let d = pair[1].1 - pair[0].1;
        assert!(
            (d - pitch).abs() < 1e-3,
            "slot pitch {d} != finger pitch {pitch}"
        );
    }
}

#[test]
fn test_speeds_feeds_rpm_clamp_preserves_chip_load() {
    let mut input = CalculationInput::default();
    input.device.max_feed_rate = 1.0e6;
    input.device.max_spindle_speed_rpm = 100_000;
    let free = SpeedsFeedsCalculator::calculate(&input);
    assert!(
        free.rpm > 5000,
        "test assumes unclamped rpm > 5000, got {}",
        free.rpm
    );

    input.device.max_spindle_speed_rpm = 5000;
    let clamped = SpeedsFeedsCalculator::calculate(&input);
    assert_eq!(clamped.rpm, 5000);

    let flutes = input.tool.flutes as f32;
    let load_free = free.feed_rate / (free.rpm as f32 * flutes);
    let load_clamped = clamped.feed_rate / (clamped.rpm as f32 * flutes);
    assert!(
        (load_free - load_clamped).abs() / load_free < 0.01,
        "chip load changed from {load_free} to {load_clamped}"
    );
}

#[test]
fn test_arc_expander_full_circle() {
    let pts = ArcExpander::default().expand_arc(10.0, 0.0, 10.0, 0.0, 0.0, 0.0, false);
    // A full circle must visit the opposite side (-10, 0), not collapse onto the start point.
    assert!(pts.iter().any(|&(x, _)| x < -9.0));
}
