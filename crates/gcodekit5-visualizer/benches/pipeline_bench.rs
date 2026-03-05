//! Performance benchmarks for the G-code processing pipeline,
//! state updates, and file statistics.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;

use gcodekit5_visualizer::{
    BoundingBox, CommandProcessor, CommentProcessor, DecimalProcessor, EmptyLineRemoverProcessor,
    GcodeCommand, GcodeState, ProcessorPipeline, WhitespaceProcessor,
};

// ---------------------------------------------------------------------------
// Helper: generate realistic G-code command batches
// ---------------------------------------------------------------------------

fn make_gcode_commands(n: usize) -> Vec<GcodeCommand> {
    let lines = [
        "G21",
        "G90",
        "G0 Z5.000",
        "M3 S12000",
        "G0 X10.000 Y10.000",
        "G1 Z-1.000 F100",
        "G1 X50.000 F500",
        "G1 Y50.000",
        "G1 X10.000",
        "G1 Y10.000",
        "G2 X30.000 Y30.000 I10.000 J0.000",
        "G3 X20.000 Y20.000 I-5.000 J0.000",
        "G0 Z5.000",
        "M5",
        "M30",
    ];
    (0..n)
        .map(|i| GcodeCommand::with_sequence(lines[i % lines.len()], i as u32))
        .collect()
}

// ---------------------------------------------------------------------------
// GcodeState update benchmarks
// ---------------------------------------------------------------------------

fn bench_state_updates(c: &mut Criterion) {
    c.bench_function("state_modal_updates_cycle", |b| {
        b.iter(|| {
            let mut state = GcodeState::new();
            state.set_motion_mode(0).unwrap();
            state.set_motion_mode(1).unwrap();
            state.set_motion_mode(2).unwrap();
            state.set_motion_mode(3).unwrap();
            state.set_plane_mode(17).unwrap();
            state.set_plane_mode(18).unwrap();
            state.set_distance_mode(90).unwrap();
            state.set_distance_mode(91).unwrap();
            state.set_feed_rate_mode(93).unwrap();
            state.set_feed_rate_mode(94).unwrap();
            state.set_units_mode(20).unwrap();
            state.set_units_mode(21).unwrap();
            state.set_coordinate_system(54).unwrap();
            state.set_coordinate_system(55).unwrap();
            black_box(&state);
        });
    });
}

// ---------------------------------------------------------------------------
// ProcessorPipeline benchmarks
// ---------------------------------------------------------------------------

fn make_pipeline() -> ProcessorPipeline {
    let mut pipeline = ProcessorPipeline::new();
    pipeline.register(Arc::new(WhitespaceProcessor::new()));
    pipeline.register(Arc::new(CommentProcessor::new()));
    pipeline.register(Arc::new(EmptyLineRemoverProcessor::new()));
    pipeline.register(Arc::new(DecimalProcessor::new()));
    pipeline
}

fn bench_pipeline_single_command(c: &mut Criterion) {
    let pipeline = make_pipeline();
    let state = GcodeState::new();
    let cmd = GcodeCommand::new("  G1 X10.123456789 Y20.987654321 F500  ; move ");
    c.bench_function("pipeline_single_command", |b| {
        b.iter(|| {
            let _ = pipeline.process_command(black_box(&cmd), black_box(&state));
        });
    });
}

fn bench_pipeline_batch_15(c: &mut Criterion) {
    let pipeline = make_pipeline();
    let commands = make_gcode_commands(15);
    c.bench_function("pipeline_batch_15_commands", |b| {
        b.iter(|| {
            let mut state = GcodeState::new();
            let _ = pipeline.process_commands(black_box(&commands), &mut state);
        });
    });
}

fn bench_pipeline_batch_100(c: &mut Criterion) {
    let pipeline = make_pipeline();
    let commands = make_gcode_commands(100);
    c.bench_function("pipeline_batch_100_commands", |b| {
        b.iter(|| {
            let mut state = GcodeState::new();
            let _ = pipeline.process_commands(black_box(&commands), &mut state);
        });
    });
}

fn bench_pipeline_batch_1000(c: &mut Criterion) {
    let pipeline = make_pipeline();
    let commands = make_gcode_commands(1000);
    c.bench_function("pipeline_batch_1000_commands", |b| {
        b.iter(|| {
            let mut state = GcodeState::new();
            let _ = pipeline.process_commands(black_box(&commands), &mut state);
        });
    });
}

// ---------------------------------------------------------------------------
// BoundingBox update benchmarks
// ---------------------------------------------------------------------------

fn bench_bounding_box_updates(c: &mut Criterion) {
    c.bench_function("bounding_box_1000_updates", |b| {
        b.iter(|| {
            let mut bb = BoundingBox::new();
            for i in 0..1000 {
                let v = i as f32 * 0.1;
                bb.update(v, v * 1.5, v * 0.5);
            }
            black_box(&bb);
        });
    });
}

// ---------------------------------------------------------------------------
// Individual processor benchmarks
// ---------------------------------------------------------------------------

fn bench_whitespace_processor(c: &mut Criterion) {
    let processor = WhitespaceProcessor::new();
    let state = GcodeState::new();
    let cmd = GcodeCommand::new("   G1 X10 Y20 F500   ");
    c.bench_function("processor_whitespace", |b| {
        b.iter(|| {
            let _ = processor.process(black_box(&cmd), black_box(&state));
        });
    });
}

fn bench_comment_processor(c: &mut Criterion) {
    let processor = CommentProcessor::new();
    let state = GcodeState::new();
    let cmd = GcodeCommand::new("G1 X10 Y20 (inline comment) F500 ; line comment");
    c.bench_function("processor_comment", |b| {
        b.iter(|| {
            let _ = processor.process(black_box(&cmd), black_box(&state));
        });
    });
}

fn bench_decimal_processor(c: &mut Criterion) {
    let processor = DecimalProcessor::with_precision(3);
    let state = GcodeState::new();
    let cmd = GcodeCommand::new("G1 X10.123456789 Y20.987654321 Z-1.555555555 F500.123");
    c.bench_function("processor_decimal", |b| {
        b.iter(|| {
            let _ = processor.process(black_box(&cmd), black_box(&state));
        });
    });
}

criterion_group!(state_benches, bench_state_updates,);
criterion_group!(
    pipeline_benches,
    bench_pipeline_single_command,
    bench_pipeline_batch_15,
    bench_pipeline_batch_100,
    bench_pipeline_batch_1000,
);
criterion_group!(util_benches, bench_bounding_box_updates,);
criterion_group!(
    processor_benches,
    bench_whitespace_processor,
    bench_comment_processor,
    bench_decimal_processor,
);
criterion_main!(
    state_benches,
    pipeline_benches,
    util_benches,
    processor_benches
);
