//! Performance benchmarks for core types: command creation, position operations,
//! state transitions, and unit conversions.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gcodekit5_core::data::{CNCPoint, Units};
use gcodekit5_core::gcode::{CommandNumberGenerator, GcodeCommand};
use gcodekit5_core::{PartialPosition, Position};

// ---------------------------------------------------------------------------
// GcodeCommand creation benchmarks
// ---------------------------------------------------------------------------

fn bench_command_creation(c: &mut Criterion) {
    c.bench_function("command_new", |b| {
        b.iter(|| GcodeCommand::new(black_box("G1 X10.5 Y20.3 F1000")));
    });
}

fn bench_command_with_sequence(c: &mut Criterion) {
    c.bench_function("command_with_sequence", |b| {
        b.iter(|| GcodeCommand::with_sequence(black_box("G0 X0 Y0 Z5"), black_box(42)));
    });
}

fn bench_command_batch_creation(c: &mut Criterion) {
    let lines = [
        "G21",
        "G90",
        "G0 Z5",
        "M3 S12000",
        "G0 X10 Y10",
        "G1 Z-1 F100",
        "G1 X50 F500",
        "G1 Y50",
        "G1 X10",
        "G1 Y10",
        "G0 Z5",
        "M5",
        "M30",
    ];
    c.bench_function("command_batch_13_lines", |b| {
        b.iter(|| {
            let cmds: Vec<_> = lines
                .iter()
                .enumerate()
                .map(|(i, line)| GcodeCommand::with_sequence(*line, i as u32))
                .collect();
            black_box(cmds);
        });
    });
}

// ---------------------------------------------------------------------------
// Command state lifecycle benchmarks
// ---------------------------------------------------------------------------

fn bench_command_lifecycle(c: &mut Criterion) {
    c.bench_function("command_lifecycle", |b| {
        b.iter(|| {
            let mut cmd = GcodeCommand::new(black_box("G1 X10 Y20 F500"));
            cmd.mark_sent();
            cmd.mark_ok();
            cmd.mark_done();
            black_box(&cmd);
        });
    });
}

fn bench_command_number_generator(c: &mut Criterion) {
    let gen = CommandNumberGenerator::new();
    c.bench_function("command_number_next_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(gen.next());
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Position operation benchmarks
// ---------------------------------------------------------------------------

fn bench_position_creation(c: &mut Criterion) {
    c.bench_function("position_new", |b| {
        b.iter(|| Position::new(black_box(10.5), black_box(20.3), black_box(-1.0)));
    });
}

fn bench_position_distance(c: &mut Criterion) {
    let p1 = Position::new(0.0, 0.0, 0.0);
    let p2 = Position::new(100.0, 200.0, 50.0);
    c.bench_function("position_distance_to", |b| {
        b.iter(|| black_box(p1.distance_to(black_box(&p2))));
    });
}

fn bench_position_arithmetic(c: &mut Criterion) {
    let p1 = Position::new(10.0, 20.0, 30.0);
    let p2 = Position::new(5.0, 15.0, 25.0);
    c.bench_function("position_add_subtract", |b| {
        b.iter(|| {
            let sum = p1.add(black_box(&p2));
            let diff = sum.subtract(black_box(&p2));
            black_box(diff);
        });
    });
}

fn bench_partial_position_apply(c: &mut Criterion) {
    let pos = Position::new(10.0, 20.0, 30.0);
    let partial = PartialPosition {
        x: Some(50.0),
        y: None,
        z: Some(-5.0),
        ..Default::default()
    };
    c.bench_function("partial_position_apply", |b| {
        b.iter(|| black_box(partial.apply_to(black_box(&pos))));
    });
}

// ---------------------------------------------------------------------------
// Unit conversion benchmarks
// ---------------------------------------------------------------------------

fn bench_unit_conversion_single(c: &mut Criterion) {
    c.bench_function("unit_convert_mm_to_inch", |b| {
        b.iter(|| black_box(Units::convert(black_box(25.4), Units::MM, Units::INCH)));
    });
}

fn bench_cncpoint_conversion(c: &mut Criterion) {
    let point = CNCPoint::with_axes(100.0, 200.0, 50.0, 0.0, 0.0, 0.0, Units::MM);
    c.bench_function("cncpoint_convert_mm_to_inch", |b| {
        b.iter(|| black_box(point.convert_to(black_box(Units::INCH))));
    });
}

fn bench_unit_conversion_batch(c: &mut Criterion) {
    let values: Vec<f64> = (0..1000).map(|i| i as f64 * 0.1).collect();
    c.bench_function("unit_convert_batch_1000", |b| {
        b.iter(|| {
            let converted: Vec<f64> = values
                .iter()
                .map(|v| Units::convert(*v, Units::MM, Units::INCH))
                .collect();
            black_box(converted);
        });
    });
}

criterion_group!(
    command_benches,
    bench_command_creation,
    bench_command_with_sequence,
    bench_command_batch_creation,
    bench_command_lifecycle,
    bench_command_number_generator,
);
criterion_group!(
    position_benches,
    bench_position_creation,
    bench_position_distance,
    bench_position_arithmetic,
    bench_partial_position_apply,
);
criterion_group!(
    conversion_benches,
    bench_unit_conversion_single,
    bench_cncpoint_conversion,
    bench_unit_conversion_batch,
);
criterion_main!(command_benches, position_benches, conversion_benches);
