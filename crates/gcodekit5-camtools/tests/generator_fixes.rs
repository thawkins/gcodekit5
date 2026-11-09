//! Regression tests for laser polarity, the optional homing preamble, laser mode, tabbed-box
//! panel labels, Gerber aperture/rubout/drill fixes, the comment-aware G-code utilities and
//! unit-aware CAM presets.

use gcodekit5_camtools::cam_presets::{BitmapEngravingPreset, ProbePreset, PRESET_FORMAT_VERSION};
use gcodekit5_camtools::comment_processor::{strip_comments, words, CommentMode, CommentProcessor};
use gcodekit5_camtools::gerber::{GerberConverter, GerberLayerType, GerberParameters};
use gcodekit5_camtools::jigsaw_puzzle::{JigsawPuzzleMaker, PuzzleParameters};
use gcodekit5_camtools::laser_engraver::{
    BitmapImageEngraver, EngravingParameters, ImageTransformations,
};
use gcodekit5_camtools::optimizer::GCodeOptimizer;
use gcodekit5_camtools::spoilboard_grid::{SpoilboardGridGenerator, SpoilboardGridParameters};
use gcodekit5_camtools::stats::StatsCalculator;
use gcodekit5_camtools::tabbed_box::{BoxParameters, BoxType, TabbedBoxMaker};
use gcodekit5_camtools::validator::GCodeValidator;
use gcodekit5_core::units::MeasurementSystem;
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

/// Every Z value commanded by a G0/G1 line.
fn z_moves(gcode: &str) -> Vec<f32> {
    gcode
        .lines()
        .filter(|l| l.starts_with("G0 ") || l.starts_with("G1 "))
        .filter_map(|l| {
            l.split_whitespace()
                .find(|w| w.starts_with('Z'))
                .and_then(|w| w[1..].parse::<f32>().ok())
        })
        .collect()
}

/// Largest S value on burn (G1) lines.
fn max_burn_power(gcode: &str) -> u32 {
    gcode
        .lines()
        .filter(|l| l.starts_with("G1 "))
        .filter_map(|l| {
            l.split_whitespace()
                .find(|w| w.starts_with('S'))
                .and_then(|w| w[1..].parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0)
}

fn small_engraver_params() -> EngravingParameters {
    EngravingParameters {
        width_mm: 0.4,
        pixels_per_mm: 10.0,
        offset_x: 0.0,
        offset_y: 0.0,
        ..Default::default()
    }
}

fn engrave(img: GrayImage, params: EngravingParameters) -> String {
    BitmapImageEngraver::from_image(DynamicImage::ImageLuma8(img), params)
        .expect("engraver setup")
        .generate_gcode()
        .expect("gcode")
}

// ---- Item 1: bitmap polarity ----

#[test]
fn test_laser_dark_pixels_get_more_power_by_default() {
    assert!(!ImageTransformations::default().invert);
    let black = engrave(
        GrayImage::from_pixel(4, 4, Luma([0])),
        small_engraver_params(),
    );
    let white = engrave(
        GrayImage::from_pixel(4, 4, Luma([255])),
        small_engraver_params(),
    );
    assert_eq!(max_burn_power(&black), 1000, "black must burn at max power");
    assert_eq!(max_burn_power(&white), 0, "white must not burn");
}

#[test]
fn test_laser_invert_engraves_light_areas() {
    let mut params = small_engraver_params();
    params.transformations.invert = true;
    let white = engrave(GrayImage::from_pixel(4, 4, Luma([255])), params.clone());
    let black = engrave(GrayImage::from_pixel(4, 4, Luma([0])), params);
    assert_eq!(max_burn_power(&white), 1000);
    assert_eq!(max_burn_power(&black), 0);
}

// ---- Item 5: homing preamble ----

#[test]
fn test_laser_no_homing_or_z_moves_by_default_and_offsets_applied() {
    let params = EngravingParameters {
        offset_x: 20.0,
        offset_y: 30.0,
        ..small_engraver_params()
    };
    let gcode = engrave(GrayImage::from_pixel(4, 4, Luma([0])), params);
    assert!(!gcode.contains("$H"), "{gcode}");
    assert!(z_moves(&gcode).is_empty(), "engraver must not move Z");
    for (x, y) in xy_moves(&gcode)
        .into_iter()
        .filter(|&(x, y)| x != 0.0 || y != 0.0)
    {
        assert!(
            (20.0..=20.5).contains(&x) && (30.0..=30.5).contains(&y),
            "({x},{y})"
        );
    }
}

#[test]
fn test_laser_homing_preamble_keeps_z_offset() {
    let params = EngravingParameters {
        home_before_job: true,
        ..small_engraver_params()
    };
    let gcode = engrave(GrayImage::from_pixel(4, 4, Luma([0])), params);
    assert!(gcode.contains("$H"));
    assert!(gcode.contains("G10 L2 P1 X0 Y0 "));
    assert!(!gcode
        .lines()
        .any(|l| l.starts_with("G10") && l.contains('Z')));
    assert!(z_moves(&gcode).is_empty());
}

fn box_gcode(params: BoxParameters) -> String {
    let mut maker = TabbedBoxMaker::new(params).expect("box setup");
    maker.generate().expect("box generate");
    maker.to_gcode()
}

#[test]
fn test_tabbed_box_homing_optional_and_never_above_z0() {
    let default = box_gcode(BoxParameters::default());
    assert!(!default.contains("$H"));

    let homed = box_gcode(BoxParameters {
        home_before_job: true,
        ..Default::default()
    });
    assert!(homed.contains("$H"));
    assert!(!homed
        .lines()
        .any(|l| l.starts_with("G10") && l.contains('Z')));
    for z in z_moves(&homed) {
        assert!(z <= 0.0, "Z{z} above work zero after homing");
    }
}

#[test]
fn test_jigsaw_homing_optional_and_never_above_z0() {
    let make = |home: bool| {
        let mut maker = JigsawPuzzleMaker::new(PuzzleParameters {
            home_before_job: home,
            ..Default::default()
        })
        .expect("jigsaw setup");
        maker.generate().expect("jigsaw generate");
        maker.to_gcode(500.0, 1.0)
    };
    let default = make(false);
    assert!(!default.contains("$H"));
    // Without homing the user has set the work zero, so a positive travel height is allowed.
    assert!(z_moves(&default).iter().any(|&z| z > 0.0));

    let homed = make(true);
    assert!(homed.contains("$H"));
    assert!(!homed
        .lines()
        .any(|l| l.starts_with("G10") && l.contains('Z')));
    for z in z_moves(&homed) {
        assert!(z <= 0.0, "Z{z} above work zero after homing");
    }
}

#[test]
fn test_jigsaw_offsets_applied_to_coordinates() {
    let mut maker = JigsawPuzzleMaker::new(PuzzleParameters {
        offset_x: 25.0,
        offset_y: 35.0,
        ..Default::default()
    })
    .expect("jigsaw setup");
    maker.generate().expect("jigsaw generate");
    let gcode = maker.to_gcode(500.0, 1.0);
    let moves: Vec<(f32, f32)> = xy_moves(&gcode)
        .into_iter()
        .filter(|&(x, y)| x != 0.0 || y != 0.0)
        .collect();
    assert!(!moves.is_empty());
    assert!(moves.iter().all(|&(x, y)| x >= 24.0 && y >= 34.0));
}

#[test]
fn test_spoilboard_grid_does_not_change_machine_settings() {
    let gcode = SpoilboardGridGenerator::new(SpoilboardGridParameters {
        width: 20.0,
        height: 20.0,
        grid_spacing: 10.0,
        feed_rate: 1000.0,
        laser_power: 100.0,
        laser_mode: "M4".to_string(),
    })
    .generate()
    .expect("grid");
    assert!(!gcode
        .lines()
        .any(|l| l.trim_start().starts_with('$') && l.contains('=')));
}

// ---- Item 6: M4 for laser cutting ----

#[test]
fn test_tabbed_box_and_jigsaw_use_dynamic_laser_power() {
    let boxed = box_gcode(BoxParameters::default());
    assert!(boxed.lines().any(|l| l.starts_with("M4 ")));
    assert!(!boxed.lines().any(|l| l.starts_with("M3")));

    let mut maker = JigsawPuzzleMaker::new(PuzzleParameters::default()).expect("jigsaw setup");
    maker.generate().expect("jigsaw generate");
    let jig = maker.to_gcode(500.0, 1.0);
    assert!(jig.lines().any(|l| l.starts_with("M4 ")));
    assert!(!jig.lines().any(|l| l.starts_with("M3")));
}

// ---- Item 9: tabbed box panel labels ----

#[test]
fn test_tabbed_box_labels_follow_panels_and_slots() {
    let params = BoxParameters {
        box_type: BoxType::NoTop,
        dividers_x: 1,
        ..Default::default()
    };
    let mut maker = TabbedBoxMaker::new(params).expect("box setup");
    maker.generate().expect("box generate");
    let labels = maker.path_labels();
    assert_eq!(labels.len(), maker.paths().len());
    assert_eq!(labels[0], "Wall 1 (front)");
    assert!(
        labels.iter().all(|l| l != "Top"),
        "no top panel in NoTop box"
    );
    assert!(labels.iter().any(|l| l == "Bottom"));
    assert!(labels.iter().any(|l| l.starts_with("Wall 1 (front) slot ")));
    assert!(labels.iter().any(|l| l == "X divider 1"));
    let back = labels
        .iter()
        .position(|l| l == "Wall 3 (back)")
        .expect("back wall");
    // A wall outline is never mislabelled with a fixed-index name.
    assert!(maker.paths()[back].len() > 5);

    let gcode = maker.to_gcode();
    assert!(!gcode.contains("Unknown"));
    // Slots of a panel are cut before its outline.
    let first_slot = gcode.find("Wall 1 (front) slot 1\n").expect("slot comment");
    let outline = gcode.find(": Wall 1 (front)\n").expect("outline comment");
    assert!(first_slot < outline);
}

// ---- Item 7: Gerber ----

fn copper_params() -> GerberParameters {
    GerberParameters {
        layer_type: GerberLayerType::TopCopper,
        tool_diameter: 0.2,
        ..Default::default()
    }
}

#[test]
fn test_gerber_alignment_holes_use_drill_depth() {
    let params = GerberParameters {
        generate_alignment_holes: true,
        alignment_hole_depth: -2.0,
        ..copper_params()
    };
    let gerber = "%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,2.0*%\nD10*\nX10000000Y10000000D03*\nM02*\n";
    let gcode = GerberConverter::generate(&params, gerber).expect("gerber");
    let holes = gcode
        .lines()
        .filter(|l| l.contains("Drill alignment hole"))
        .collect::<Vec<_>>();
    assert_eq!(holes.len(), 4);
    assert!(holes.iter().all(|l| l.starts_with("G1 Z-2.000")));

    let bad = GerberParameters {
        alignment_hole_depth: 0.5,
        ..params
    };
    assert!(GerberConverter::generate(&bad, gerber).is_err());
}

#[test]
fn test_gerber_rubout_without_outline_uses_copper_bounds() {
    let params = GerberParameters {
        rubout: true,
        board_width: 10.0,
        board_height: 10.0,
        ..copper_params()
    };
    // Two pads far from the origin.
    let gerber = "%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,1.0*%\nD10*\n\
                  X50000000Y50000000D03*\nX60000000Y50000000D03*\nM02*\n";
    let gcode = GerberConverter::generate(&params, gerber).expect("gerber");
    let rubout = &gcode[gcode.find("; Rubout").expect("rubout section")..];
    let moves = xy_moves(rubout);
    let hatch: Vec<_> = moves
        .iter()
        .filter(|&&(x, y)| x != 0.0 || y != 0.0)
        .collect();
    assert!(!hatch.is_empty(), "no rubout generated");
    for &&(x, y) in &hatch {
        assert!(
            (48.0..=62.0).contains(&x) && (48.0..=52.0).contains(&y),
            "rubout at ({x}, {y}) is outside the copper bounds"
        );
    }
}

#[test]
fn test_gerber_rectangle_aperture_trace_has_square_ends() {
    // 2.0 x 0.5 rectangle dragged from (10,10) to (20,10): copper spans x 9..21.
    let gerber = "%FSLAX46Y46*%\n%MOMM*%\n%ADD11R,2.0X0.5*%\nD11*\n\
                  X10000000Y10000000D02*\nX20000000Y10000000D01*\nM02*\n";
    let gcode = GerberConverter::generate(&copper_params(), gerber).expect("gerber");
    let moves = xy_moves(&gcode);
    let max_x = moves.iter().map(|m| m.0).fold(f32::MIN, f32::max);
    let min_x = moves
        .iter()
        .filter(|m| m.0 != 0.0)
        .map(|m| m.0)
        .fold(f32::MAX, f32::min);
    assert!(
        max_x > 21.05,
        "right end at {max_x}, rectangle stroke not honoured"
    );
    assert!(
        min_x < 8.95,
        "left end at {min_x}, rectangle stroke not honoured"
    );
    // The stroke is 2.0 high (the aperture's y size), not 0.5 (round approximation).
    let max_y = moves.iter().map(|m| m.1).fold(f32::MIN, f32::max);
    assert!(max_y > 10.2 && max_y < 10.4, "top at {max_y}");
}

#[test]
fn test_gerber_obround_flash_is_rounded() {
    let gerber = "%FSLAX46Y46*%\n%MOMM*%\n%ADD12O,2.0X1.0*%\nD12*\nX10000000Y10000000D03*\nM02*\n";
    let gcode = GerberConverter::generate(&copper_params(), gerber).expect("gerber");
    let moves: Vec<_> = xy_moves(&gcode)
        .into_iter()
        .filter(|&(x, y)| x != 0.0 || y != 0.0)
        .collect();
    assert!(!moves.is_empty());
    // A rectangle would put isolation points near the corners (11.1, 10.6).
    for (x, y) in moves {
        if (x - 10.0).abs() > 0.95 {
            assert!((y - 10.0).abs() < 0.5, "square corner at ({x}, {y})");
        }
    }
}

// ---- Item 8: comment-aware utilities ----

#[test]
fn test_comment_processor_strips_all_comments() {
    assert_eq!(strip_comments("G1 (move X5) X10 ; to Y20"), "G1 X10");
    let (code, comment) = CommentProcessor::new(CommentMode::Extract).process_line("G0 X1 (a) (b)");
    assert_eq!(code, "G0 X1");
    assert_eq!(comment.as_deref(), Some("a; b"));
    let w = words("g1x-1.5 (Y9) f300");
    assert_eq!(w.len(), 3);
    assert_eq!((w[0].letter, w[0].value), ('G', 1.0));
    assert_eq!((w[1].letter, w[1].value), ('X', -1.5));
    assert_eq!((w[2].letter, w[2].value), ('F', 300.0));
}

#[test]
fn test_validator_ignores_comment_letters() {
    let validator = GCodeValidator::default();
    let ok = vec![
        "G0 X10 (FIXTURE Z99999)".to_string(),
        "G1 Y5 F100 ; F-1 in comment".to_string(),
    ];
    assert!(validator.validate(&ok).is_ok());
    let bad = vec!["G1 X1 F-5".to_string()];
    assert!(validator.validate(&bad).is_err());
}

#[test]
fn test_stats_ignores_comments_and_whole_words() {
    let lines: Vec<String> = [
        "; G0 X999",
        "G0 X1 Y2 (G1 X-500)",
        "G38.2 Z-5 F100",
        "G01 X3",
        "M5",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let stats = StatsCalculator::calculate(&lines);
    assert_eq!(stats.rapid_count, 1);
    assert_eq!(stats.linear_count, 1);
    assert_eq!(stats.total_commands, 4);
    assert_eq!(stats.max_x, 3.0);
    assert_eq!(stats.min_x, 1.0);
}

#[test]
fn test_optimizer_m5_matches_whole_word_only() {
    let lines: Vec<String> = [
        "M5",
        "M50",
        "M5 ; again",
        "(note) M5",
        "M5 (M5)",
        "G0 X1 M5",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let out = GCodeOptimizer::remove_redundant_m5(&lines);
    // M50 breaks the run; the next M5 is kept; the two after it are duplicates.
    assert_eq!(out, vec!["M5", "M50", "M5 ; again", "G0 X1 M5"]);

    let tools: Vec<String> = ["T1", "T1 (T2)", "T10", "T1"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        GCodeOptimizer::remove_redundant_tools(&tools),
        vec!["T1", "T10", "T1"]
    );
}

// ---- Item 10: unit-aware presets ----

#[test]
fn test_bitmap_preset_stores_mm_and_round_trips() {
    let preset = BitmapEngravingPreset {
        width_mm: 50.8,
        offset_x: 25.4,
        invert: true,
        ..Default::default()
    };
    let json = preset.to_json().expect("serialize");
    assert!(json.contains("\"units\": \"mm\""));
    assert!(json.contains("\"width_mm\": 50.8"));
    // Loading under imperial does not change the stored mm values.
    let loaded =
        BitmapEngravingPreset::from_json(&json, MeasurementSystem::Imperial).expect("load");
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.preset, preset);
    assert_eq!(loaded.preset.format_version, PRESET_FORMAT_VERSION);
}

#[test]
fn test_bitmap_preset_legacy_text_and_polarity_converted() {
    let legacy = r#"{"width_mm":"2","offset_x":"1","feed_rate":"1500","invert":false,
                     "pixels_per_mm":"abc","halftone":"floyd"}"#;
    let loaded =
        BitmapEngravingPreset::from_json(legacy, MeasurementSystem::Imperial).expect("load");
    assert!((loaded.preset.width_mm - 50.8).abs() < 1e-3);
    assert!((loaded.preset.offset_x - 25.4).abs() < 1e-3);
    assert_eq!(loaded.preset.feed_rate, 1500.0);
    assert_eq!(loaded.preset.halftone, "floyd");
    // Old invert=false burned light areas: the same output now needs invert=true.
    assert!(loaded.preset.invert);
    // The unreadable field is reported, not silently dropped.
    assert!(loaded.warnings.iter().any(|w| w.contains("pixels_per_mm")));
    assert_eq!(
        loaded.preset.pixels_per_mm,
        BitmapEngravingPreset::default().pixels_per_mm
    );
}

#[test]
fn test_presets_report_invalid_files() {
    assert!(BitmapEngravingPreset::from_json("not json", MeasurementSystem::Metric).is_err());
    assert!(BitmapEngravingPreset::from_json(
        r#"{"format_version":2,"units":"in"}"#,
        MeasurementSystem::Metric
    )
    .is_err());
    assert!(ProbePreset::from_json(r#"{"format_version":2,"safe_height":"x"}"#).is_err());
    assert!(ProbePreset::from_json(r#"{"format_version":99}"#).is_err());
}

#[test]
fn test_probe_preset_round_trip_and_legacy() {
    let preset = ProbePreset {
        probe_depth: 4.5,
        tip_radius: 1.0,
        plate_thickness: 1.6,
        ..Default::default()
    };
    let loaded = ProbePreset::from_json(&preset.to_json().expect("json")).expect("load");
    assert_eq!(loaded.preset, preset);

    let legacy = r#"{"routine":"bore_center","safe_height":"7.5","diameter":"x"}"#;
    let loaded = ProbePreset::from_json(legacy).expect("load");
    assert_eq!(loaded.preset.routine.as_deref(), Some("bore_center"));
    assert_eq!(loaded.preset.safe_height, 7.5);
    assert_eq!(loaded.warnings.len(), 1);
}
