//! Probe routine and WCS service tests.
//!
//! The relative probe programs are executed by a small G-code simulator against simulated
//! geometry (surface, edge, bore, boss), so the tests cover the generated moves themselves
//! and the result computation from the simulated probe reports.

use gcodekit5_camtools::probe_routines::{ProbeCycle, ProbeOptions, ProbeRoutineEngine};
use gcodekit5_camtools::wcs_service::{WcsUpdateConfig, WcsUpdateService};
use gcodekit5_core::{
    Corner, Position, ProbeAxis as Axis, ProbeDirection as Direction, ProbeReport, ProbeResult,
    ProbeRoutine, Units,
};

/// Result of simulating a probe program.
struct SimRun {
    /// One entry per G38.x cycle, in order (descents included).
    reports: Vec<ProbeResult>,
    /// Final position.
    end: (f32, f32, f32),
    /// `G10 L20` lines seen, with the position at which they ran.
    wcs_sets: Vec<(String, (f32, f32, f32))>,
}

/// Run `gcode` from `start`; `solid(x, y, z)` is true where the probe tip centre would
/// penetrate material. G38.2 must make contact (else the test fails, like a GRBL alarm);
/// G38.3 stops on contact or completes silently.
fn simulate(gcode: &str, start: (f32, f32, f32), solid: impl Fn(f32, f32, f32) -> bool) -> SimRun {
    let mut pos = [start.0, start.1, start.2];
    let mut relative = false;
    let mut reports = Vec::new();
    let mut wcs_sets = Vec::new();
    assert!(!solid(pos[0], pos[1], pos[2]), "start inside material");

    for raw in gcode.lines() {
        let code = raw.split(';').next().unwrap_or("").trim();
        if code.is_empty() {
            continue;
        }
        let words: Vec<&str> = code.split_whitespace().collect();
        let value = |c: char| -> Option<f32> {
            words
                .iter()
                .find(|w| w.starts_with(c))
                .and_then(|w| w[1..].parse::<f32>().ok())
        };
        match words[0] {
            "G21" => {}
            "G90" => relative = false,
            "G91" => relative = true,
            "G10" => wcs_sets.push((code.to_string(), (pos[0], pos[1], pos[2]))),
            "G0" | "G1" => {
                for (i, axis) in ['X', 'Y', 'Z'].iter().enumerate() {
                    if let Some(v) = value(*axis) {
                        pos[i] = if relative { pos[i] + v } else { v };
                    }
                }
                assert!(
                    !solid(pos[0], pos[1], pos[2]),
                    "rapid move into material at {:?}: {}",
                    pos,
                    raw
                );
            }
            "G38.2" | "G38.3" => {
                let mut target = pos;
                let mut axis_idx = None;
                for (i, axis) in ['X', 'Y', 'Z'].iter().enumerate() {
                    if let Some(v) = value(*axis) {
                        target[i] = if relative { pos[i] + v } else { v };
                        axis_idx = Some(i);
                    }
                }
                let i = axis_idx.expect("probe line without axis");
                let dist = target[i] - pos[i];
                let steps = (dist.abs() / 0.0005).ceil().max(1.0) as usize;
                let mut hit = false;
                let from = pos;
                for s in 1..=steps {
                    let mut q = from;
                    q[i] = from[i] + dist * s as f32 / steps as f32;
                    if solid(q[0], q[1], q[2]) {
                        hit = true;
                        break;
                    }
                    pos = q;
                }
                if words[0] == "G38.2" {
                    assert!(hit, "G38.2 made no contact (alarm): {}", raw);
                }
                reports.push(ProbeResult::new(
                    Position::new(pos[0], pos[1], pos[2]),
                    hit,
                    Units::MM,
                ));
            }
            other => panic!("unexpected command {}", other),
        }
    }
    assert!(!relative, "program left the controller in G91");
    SimRun {
        reports,
        end: (pos[0], pos[1], pos[2]),
        wcs_sets,
    }
}

fn report_from(routine: ProbeRoutine, reports: &[ProbeResult]) -> ProbeReport {
    let mut report = ProbeReport::new(routine);
    for r in reports {
        report.add_trigger(*r);
    }
    report
}

/// Code lines without comments (to compare programs generated from different starts).
fn code_only(gcode: &str) -> Vec<String> {
    gcode
        .lines()
        .map(|l| l.split(';').next().unwrap_or("").trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn bore(diameter: f64) -> ProbeRoutine {
    ProbeRoutine::BoreCenter {
        diameter,
        safe_height: 5.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
    }
}

fn boss(diameter: f64) -> ProbeRoutine {
    ProbeRoutine::BossCenter {
        diameter,
        safe_height: 5.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
    }
}

#[test]
fn test_probe_gcode_independent_of_start_position() {
    let routines = [
        ProbeRoutine::ZTouch {
            safe_height: 5.0,
            max_depth: 20.0,
            fast_feed: 100.0,
            slow_feed: 25.0,
            backoff: 2.0,
        },
        ProbeRoutine::EdgeFind {
            axis: Axis::X,
            direction: Direction::Positive,
            safe_height: 5.0,
            probe_distance: 10.0,
            fast_feed: 100.0,
            slow_feed: 25.0,
            backoff: 2.0,
        },
        ProbeRoutine::CornerFind {
            corner: Corner::XmaxYmin,
            safe_height: 5.0,
            probe_distance: 10.0,
            fast_feed: 100.0,
            slow_feed: 25.0,
            backoff: 1.0,
        },
        bore(20.0),
        boss(20.0),
    ];
    for routine in routines {
        let engine = ProbeRoutineEngine::new(routine);
        let a = engine
            .generate(Position::new(0.0, 0.0, 5.0))
            .expect("generate at origin");
        let b = engine
            .generate(Position::new(-123.4, 56.7, -8.9))
            .expect("generate elsewhere");
        assert_eq!(code_only(&a.gcode), code_only(&b.gcode));
        assert!(a.gcode.contains("G91"));
        // No absolute coordinate may leak in: every move is G91-relative.
        let g91 = code_only(&a.gcode)
            .iter()
            .position(|l| l == "G91")
            .expect("G91 present");
        let first_move = code_only(&a.gcode)
            .iter()
            .position(|l| l.starts_with("G0") || l.starts_with("G38"))
            .expect("has moves");
        assert!(g91 < first_move, "{}", a.gcode);
    }
}

#[test]
fn test_probe_z_touch_plate_thickness_compensated() {
    let routine = ProbeRoutine::ZTouch {
        safe_height: 5.0,
        max_depth: 20.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 2.0,
    };
    let options = ProbeOptions {
        plate_thickness: 1.5,
        set_wcs: Some(55),
        ..ProbeOptions::default()
    };
    let out = ProbeRoutineEngine::with_options(routine.clone(), options)
        .generate(Position::default())
        .expect("generate");
    // Work surface at machine Z=-30, plate on top of it: plate top at -28.5.
    let run = simulate(&out.gcode, (10.0, 20.0, -20.0), |_, _, z| z < -28.5);
    assert_eq!(run.reports.len(), out.expected_triggers);
    assert!(close(run.reports[1].position.z, -28.5));
    // Retract by safe height above the plate.
    assert!(close(run.end.2, -23.5), "end {:?}", run.end);
    // G10 L20 writes the plate thickness into G55 at the trigger point.
    assert_eq!(run.wcs_sets.len(), 1);
    assert!(run.wcs_sets[0].0.starts_with("G10 L20 P2 Z1.500"));
    assert!(close(run.wcs_sets[0].1.2, -28.5));

    let result =
        ProbeRoutineEngine::compute_result_with(&report_from(routine, &run.reports), &options)
            .expect("compute");
    assert!(close(result.z, -30.0), "surface {}", result.z);
}

#[test]
fn test_probe_edge_lowers_by_probe_depth_and_compensates_tip() {
    let routine = ProbeRoutine::EdgeFind {
        axis: Axis::X,
        direction: Direction::Positive,
        safe_height: 5.0,
        probe_distance: 15.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 2.0,
    };
    let options = ProbeOptions {
        probe_depth: 4.0,
        tip_radius: 1.0,
        ..ProbeOptions::default()
    };
    let out = ProbeRoutineEngine::with_options(routine.clone(), options)
        .generate(Position::default())
        .expect("generate");
    assert!(out.gcode.contains("G38.3 Z-4.000"), "{}", out.gcode);
    assert_eq!(out.cycles[0], ProbeCycle::Descent);

    // Stock occupies x >= 50 below z = 0. Start 3 mm above the top, left of the edge.
    let solid = |x: f32, _y: f32, z: f32| x >= 50.0 - 1.0 && z < 0.0;
    let run = simulate(&out.gcode, (40.0, 0.0, 3.0), solid);
    assert_eq!(run.reports.len(), out.expected_triggers);
    // Probed sideways 1 mm below the top, then returned to the start height.
    assert!(close(run.reports[1].position.z, -1.0));
    assert!(close(run.end.2, 3.0), "end {:?}", run.end);

    let edge = ProbeRoutineEngine::compute_result_with(
        &report_from(routine.clone(), &run.reports),
        &options,
    )
    .expect("compute");
    assert!(close(edge.x, 50.0), "edge {}", edge.x);

    // A report with only the measuring probes gives the same answer.
    let measured: Vec<ProbeResult> = run.reports[1..].to_vec();
    let edge2 = ProbeRoutineEngine::compute_result_with(&report_from(routine, &measured), &options)
        .expect("compute");
    assert!(close(edge2.x, 50.0));
}

#[test]
fn test_probe_corner_relative_moves_find_inside_corner() {
    let routine = ProbeRoutine::CornerFind {
        corner: Corner::XminYmin,
        safe_height: 5.0,
        probe_distance: 20.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 1.5,
    };
    let options = ProbeOptions {
        probe_depth: 5.0,
        tip_radius: 1.0,
        set_wcs: Some(54),
        ..ProbeOptions::default()
    };
    let out = ProbeRoutineEngine::with_options(routine.clone(), options)
        .generate(Position::default())
        .expect("generate");
    // Pocket with walls at x = 10 and y = 20 (material at x < 10 or y < 20, below z = 0).
    let solid = |x: f32, y: f32, z: f32| z < 0.0 && (x < 10.0 + 1.0 || y < 20.0 + 1.0);
    let run = simulate(&out.gcode, (25.0, 30.0, 2.0), solid);
    assert_eq!(run.reports.len(), out.expected_triggers);
    let corner =
        ProbeRoutineEngine::compute_result_with(&report_from(routine, &run.reports), &options)
            .expect("compute");
    assert!(
        close(corner.x, 10.0) && close(corner.y, 20.0),
        "{:?}",
        corner
    );
    assert!(close(run.end.2, 2.0));
    // X then Y zero written into G54, tip compensated (probe moved toward -X/-Y).
    assert_eq!(run.wcs_sets.len(), 2);
    assert!(run.wcs_sets[0].0.starts_with("G10 L20 P1 X1.000"));
    assert!(run.wcs_sets[1].0.starts_with("G10 L20 P1 Y1.000"));
}

#[test]
fn test_probe_bore_center_from_off_center_start() {
    let options = ProbeOptions {
        probe_depth: 3.0,
        tip_radius: 1.0,
        ..ProbeOptions::default()
    };
    let out = ProbeRoutineEngine::with_options(bore(20.0), options)
        .generate(Position::default())
        .expect("generate");
    // 20 mm bore centred at (100, 50); material outside the circle below z = 0.
    let solid = |x: f32, y: f32, z: f32| {
        z < 0.0 && ((x - 100.0).powi(2) + (y - 50.0).powi(2)).sqrt() > 10.0 - 1.0
    };
    let run = simulate(&out.gcode, (101.5, 48.8, 1.0), solid);
    assert_eq!(run.reports.len(), out.expected_triggers);
    let report = report_from(bore(20.0), &run.reports);
    let center = ProbeRoutineEngine::compute_result_with(&report, &options).expect("center");
    assert!(
        close(center.x, 100.0) && close(center.y, 50.0),
        "{:?}",
        center
    );
    let d = ProbeRoutineEngine::compute_diameter(&report, &options).expect("diameter");
    assert!((d - 20.0).abs() < 0.05, "diameter {}", d);
    assert!(close(run.end.2, 1.0));
}

#[test]
fn test_probe_boss_center_goes_around_boss() {
    let options = ProbeOptions {
        probe_depth: 4.0,
        tip_radius: 1.0,
        ..ProbeOptions::default()
    };
    let out = ProbeRoutineEngine::with_options(boss(30.0), options)
        .generate(Position::default())
        .expect("generate");
    assert_eq!(
        out.cycles
            .iter()
            .filter(|c| matches!(c, ProbeCycle::Descent))
            .count(),
        4
    );
    // 30 mm boss centred at (-40, 25), top at z = 0 (anything above is clear).
    let solid = |x: f32, y: f32, z: f32| {
        z < 0.0 && ((x + 40.0).powi(2) + (y - 25.0).powi(2)).sqrt() < 15.0 + 1.0
    };
    let run = simulate(&out.gcode, (-41.0, 26.5, 2.0), solid);
    assert_eq!(run.reports.len(), out.expected_triggers);
    let report = report_from(boss(30.0), &run.reports);
    let center = ProbeRoutineEngine::compute_result_with(&report, &options).expect("center");
    assert!(
        close(center.x, -40.0) && close(center.y, 25.0),
        "{:?}",
        center
    );
    let d = ProbeRoutineEngine::compute_diameter(&report, &options).expect("diameter");
    assert!((d - 30.0).abs() < 0.05, "diameter {}", d);
    assert!(close(run.end.2, 2.0));
}

#[test]
fn test_probe_boss_requires_probe_depth() {
    let options = ProbeOptions {
        probe_depth: 0.0,
        ..ProbeOptions::default()
    };
    assert!(
        ProbeRoutineEngine::with_options(boss(20.0), options)
            .generate(Position::default())
            .is_err()
    );
}

#[test]
fn test_probe_rejects_negative_compensation() {
    let options = ProbeOptions {
        tip_radius: -1.0,
        ..ProbeOptions::default()
    };
    assert!(
        ProbeRoutineEngine::with_options(bore(20.0), options)
            .generate(Position::default())
            .is_err()
    );
}

#[test]
fn test_probe_parameter_validation() {
    let routine = ProbeRoutine::ZTouch {
        safe_height: 0.0, // Invalid
        max_depth: 20.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 2.0,
    };
    assert!(
        ProbeRoutineEngine::new(routine)
            .generate(Position::new(0.0, 0.0, 0.0))
            .is_err()
    );
}

#[test]
fn test_probe_tool_length_computation() {
    let routine = ProbeRoutine::ToolLength {
        plate_xy: (100.0, 100.0),
        plate_z: -50.0,
        safe_height: 10.0,
        max_depth: 20.0,
        feed_rate: 100.0,
    };
    let mut report = ProbeReport::new(routine);
    report.add_trigger(ProbeResult::new(
        Position::new(100.0, 100.0, -65.0),
        true,
        Units::MM,
    ));
    let result = ProbeRoutineEngine::compute_result(&report).expect("compute");
    assert!((result.z - (-15.0)).abs() < 0.001);
}

#[test]
fn test_probe_corner_computation_with_backoff_reports() {
    let routine = ProbeRoutine::CornerFind {
        corner: Corner::XminYmin,
        safe_height: 10.0,
        probe_distance: 10.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 2.0,
    };
    let mut report = ProbeReport::new(routine);
    for (x, y) in [(10.0, 25.0), (10.0, 25.0), (15.0, 20.0), (15.0, 20.0)] {
        report.add_trigger(ProbeResult::new(Position::new(x, y, -5.0), true, Units::MM));
    }
    let corner = ProbeRoutineEngine::compute_result(&report).expect("compute");
    assert!((corner.x - 10.0).abs() < 0.001);
    assert!((corner.y - 20.0).abs() < 0.001);
}

fn corner_report() -> ProbeReport {
    let routine = ProbeRoutine::CornerFind {
        corner: Corner::XminYmin,
        safe_height: 10.0,
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
    report
}

#[test]
fn test_wcs_g92_makes_probed_point_the_work_zero() {
    let mut service = WcsUpdateService::new(WcsUpdateConfig {
        use_temporary_offset: true,
        ..Default::default()
    });
    // Without the current machine position a correct G92 cannot be built.
    assert!(service.process_probe_report(&corner_report()).is_err());

    // Machine at (12, 22): the probed corner (10, 20) is 2 mm behind on each axis, so the
    // current position must read X2 Y2 for the corner to read X0 Y0.
    service.set_machine_position(Position::new(12.0, 22.0, 0.0));
    let result = service
        .process_probe_report(&corner_report())
        .expect("G92 result");
    assert_eq!(result.command, "G92 X2.000 Y2.000");

    // In the complete program the G92 runs before the retract move.
    let gcode = service
        .build_complete_gcode(&corner_report(), false)
        .expect("complete gcode");
    let g92 = gcode.find("G92").expect("has G92");
    let retract = gcode.find("G0 Z").expect("has retract");
    assert!(g92 < retract, "{}", gcode);
}

#[test]
fn test_wcs_tip_radius_and_plate_thickness_applied() {
    let options = ProbeOptions {
        tip_radius: 1.0,
        plate_thickness: 2.0,
        ..ProbeOptions::default()
    };
    let mut service = WcsUpdateService::new(WcsUpdateConfig {
        probe_options: options,
        ..Default::default()
    });
    let result = service
        .process_probe_report(&corner_report())
        .expect("corner");
    // Probing toward -X/-Y: surfaces are one tip radius further in the probe direction.
    assert_eq!(result.command, "G10 L2 P1 X9.000 Y19.000");

    let mut z_report = ProbeReport::new(ProbeRoutine::ZTouch {
        safe_height: 5.0,
        max_depth: 20.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 0.0,
    });
    z_report.add_trigger(ProbeResult::new(
        Position::new(0.0, 0.0, -10.0),
        true,
        Units::MM,
    ));
    let result = service.process_probe_report(&z_report).expect("z");
    assert_eq!(result.command, "G10 L2 P1 Z-12.000");
}

#[test]
fn test_wcs_z_touch_uses_slow_reprobe() {
    let mut service = WcsUpdateService::new(WcsUpdateConfig::default());
    let mut report = ProbeReport::new(ProbeRoutine::ZTouch {
        safe_height: 5.0,
        max_depth: 20.0,
        fast_feed: 100.0,
        slow_feed: 25.0,
        backoff: 2.0,
    });
    report.add_trigger(ProbeResult::new(
        Position::new(10.0, 20.0, -5.0),
        true,
        Units::MM,
    ));
    report.add_trigger(ProbeResult::new(
        Position::new(10.0, 20.0, -5.02),
        true,
        Units::MM,
    ));
    let result = service.process_probe_report(&report).expect("z");
    assert!(!result.applied);
    assert_eq!(result.command, "G10 L2 P1 Z-5.020");
    assert!(result.description.contains("G54"));
    let preview = service.generate_preview(&report).expect("preview");
    assert!(preview.contains("Z surface") && preview.contains("G54"));
}

#[test]
fn test_wcs_tool_length_only_as_g43_1() {
    let routine = ProbeRoutine::ToolLength {
        plate_xy: (100.0, 100.0),
        plate_z: -50.0,
        safe_height: 10.0,
        max_depth: 20.0,
        feed_rate: 100.0,
    };
    let mut report = ProbeReport::new(routine);
    report.add_trigger(ProbeResult::new(
        Position::new(100.0, 100.0, -65.0),
        true,
        Units::MM,
    ));
    let mut service = WcsUpdateService::new(WcsUpdateConfig::default());
    let result = service.process_probe_report(&report).expect("tool length");
    assert_eq!(result.command, "G43.1 Z-15.000");

    let mut service = WcsUpdateService::new(WcsUpdateConfig {
        apply_tool_length: false,
        ..Default::default()
    });
    assert!(service.process_probe_report(&report).is_err());
}

fn tool_length_routine() -> ProbeRoutine {
    ProbeRoutine::ToolLength {
        plate_xy: (300.0, 10.0),
        plate_z: -50.0,
        safe_height: 10.0,
        max_depth: 30.0,
        feed_rate: 100.0,
    }
}

#[test]
fn test_probe_tool_length_relative_from_any_start() {
    let engine = ProbeRoutineEngine::new(tool_length_routine());
    let a = engine
        .generate(Position::new(0.0, 0.0, 0.0))
        .expect("generate");
    let b = engine
        .generate(Position::new(299.0, 11.0, -35.0))
        .expect("generate");
    assert_eq!(code_only(&a.gcode), code_only(&b.gcode));
    // The setter location is only a comment: no absolute XY move to it.
    assert!(
        !code_only(&a.gcode)
            .iter()
            .any(|l| l.contains("X300") || l.contains("Y10.000"))
    );
    assert!(a.gcode.contains("G38.2 Z-30.000 F100.0"), "{}", a.gcode);

    // Setter surface: this tool triggers at machine Z-60, 10 mm lower than the reference
    // tool (plate_z = -50), so it is 10 mm shorter and the offset is -10.
    let run = simulate(&a.gcode, (300.0, 10.0, -40.0), |_, _, z| z < -60.0);
    assert_eq!(run.reports.len(), a.expected_triggers);
    assert!(close(run.reports[0].position.z, -60.0));
    assert!(
        close(run.end.2, -50.0),
        "retract by safe height: {:?}",
        run.end
    );

    let report = report_from(tool_length_routine(), &run.reports);
    let result =
        ProbeRoutineEngine::compute_result_with(&report, &ProbeOptions::default()).expect("tl");
    assert!(close(result.z, -10.0), "tool length offset {}", result.z);
    let mut service = WcsUpdateService::new(WcsUpdateConfig::default());
    let applied = service.process_probe_report(&report).expect("g43.1");
    assert_eq!(applied.command, "G43.1 Z-10.000");
}
