//! Performance benchmarks for toolpath generation, G-code generation,
//! spatial index operations, and DXF parsing.

use criterion::{Criterion, criterion_group, criterion_main};
use gcodekit5_core::Units;
use gcodekit5_designer::dxf_parser::DxfParser;
use gcodekit5_designer::gcode_gen::ToolpathToGcode;
use gcodekit5_designer::model::Point;
use gcodekit5_designer::spatial_index::SpatialIndex;
use gcodekit5_designer::toolpath::{
    Toolpath, ToolpathGenerator, ToolpathSegment, ToolpathSegmentType,
};
use gcodekit5_designer::{Bounds, Circle, Rectangle};
use std::hint::black_box;

// ---------------------------------------------------------------------------
// Toolpath generation benchmarks
// ---------------------------------------------------------------------------

fn bench_rectangle_contour(c: &mut Criterion) {
    let rect = Rectangle::new(50.0, 50.0, 100.0, 60.0);
    let mut generator = ToolpathGenerator::new();
    generator.set_tool_diameter(3.175);
    generator.set_cut_depth(2.0);
    generator.set_feed_rate(500.0);
    generator.set_spindle_speed(12000);
    c.bench_function("toolpath_rectangle_contour", |b| {
        b.iter(|| {
            let _ = generator.generate_rectangle_contour(black_box(&rect), 0.0);
        });
    });
}

fn bench_circle_contour(c: &mut Criterion) {
    let circle = Circle::new(Point::new(50.0, 50.0), 25.0);
    let mut generator = ToolpathGenerator::new();
    generator.set_tool_diameter(3.0);
    generator.set_cut_depth(1.5);
    generator.set_feed_rate(400.0);
    generator.set_spindle_speed(10000);
    c.bench_function("toolpath_circle_contour", |b| {
        b.iter(|| {
            let _ = generator.generate_circle_contour(black_box(&circle), 0.0);
        });
    });
}

fn bench_rectangle_contour_multipass(c: &mut Criterion) {
    let rect = Rectangle::new(50.0, 50.0, 100.0, 60.0);
    let mut generator = ToolpathGenerator::new();
    generator.set_tool_diameter(3.175);
    generator.set_cut_depth(6.0);
    generator.set_feed_rate(500.0);
    generator.set_spindle_speed(12000);
    c.bench_function("toolpath_rectangle_contour_multipass", |b| {
        b.iter(|| {
            let _ = generator.generate_rectangle_contour(black_box(&rect), 1.0);
        });
    });
}

// ---------------------------------------------------------------------------
// G-code generation benchmarks
// ---------------------------------------------------------------------------

fn make_large_toolpath(n_segments: usize) -> Toolpath {
    let mut tp = Toolpath::new(3.0, -2.0);
    for i in 0..n_segments {
        let x = (i as f64) * 0.5;
        let y = if i % 2 == 0 { 0.0 } else { 10.0 };
        tp.add_segment(ToolpathSegment::new(
            if i == 0 {
                ToolpathSegmentType::RapidMove
            } else {
                ToolpathSegmentType::LinearMove
            },
            Point::new(x - 0.5, if i % 2 == 0 { 10.0 } else { 0.0 }),
            Point::new(x, y),
            500.0,
            12000,
        ));
    }
    tp
}

fn bench_gcode_generation_small(c: &mut Criterion) {
    let tp = make_large_toolpath(20);
    let generator = ToolpathToGcode::new(Units::MM, 5.0);
    c.bench_function("gcode_gen_20_segments", |b| {
        b.iter(|| {
            let _ = generator.generate(black_box(&tp));
        });
    });
}

fn bench_gcode_generation_medium(c: &mut Criterion) {
    let tp = make_large_toolpath(200);
    let generator = ToolpathToGcode::new(Units::MM, 5.0);
    c.bench_function("gcode_gen_200_segments", |b| {
        b.iter(|| {
            let _ = generator.generate(black_box(&tp));
        });
    });
}

fn bench_gcode_generation_large(c: &mut Criterion) {
    let tp = make_large_toolpath(2000);
    let generator = ToolpathToGcode::new(Units::MM, 5.0);
    c.bench_function("gcode_gen_2000_segments", |b| {
        b.iter(|| {
            let _ = generator.generate(black_box(&tp));
        });
    });
}

// ---------------------------------------------------------------------------
// Spatial index benchmarks
// ---------------------------------------------------------------------------

fn bench_spatial_index_insert(c: &mut Criterion) {
    c.bench_function("spatial_index_insert_500", |b| {
        b.iter(|| {
            let world = Bounds::new(0.0, 0.0, 2000.0, 1500.0);
            let mut idx = SpatialIndex::new(world, 8, 16);
            for i in 0..500u64 {
                let x = (i as f64) * 2.0;
                let y = (i as f64) * 1.5;
                let bounds = Bounds::new(x, y, x + 10.0, y + 8.0);
                idx.insert(i, &bounds);
            }
        });
    });
}

fn bench_spatial_index_query(c: &mut Criterion) {
    let world = Bounds::new(0.0, 0.0, 2000.0, 1500.0);
    let mut idx = SpatialIndex::new(world, 8, 16);
    for i in 0..500u64 {
        let x = (i as f64) * 2.0;
        let y = (i as f64) * 1.5;
        let bounds = Bounds::new(x, y, x + 10.0, y + 8.0);
        idx.insert(i, &bounds);
    }

    let query_bounds = Bounds::new(100.0, 75.0, 200.0, 150.0);

    c.bench_function("spatial_index_query_500", |b| {
        b.iter(|| {
            let _ = idx.query(black_box(&query_bounds));
        });
    });
}

// ---------------------------------------------------------------------------
// DXF parsing benchmarks
// ---------------------------------------------------------------------------

fn make_dxf_content(n_entities: usize) -> String {
    let mut content = String::new();
    content.push_str("0\nSECTION\n2\nHEADER\n0\nENDSEC\n");
    content.push_str("0\nSECTION\n2\nENTITIES\n");
    for i in 0..n_entities {
        let x = (i as f64) * 5.0;
        let y = (i as f64) * 3.0;
        content.push_str(&format!(
            "0\nLINE\n8\n0\n10\n{:.3}\n20\n{:.3}\n11\n{:.3}\n21\n{:.3}\n",
            x,
            y,
            x + 10.0,
            y + 10.0
        ));
    }
    content.push_str("0\nENDSEC\n0\nEOF\n");
    content
}

fn bench_dxf_parse_small(c: &mut Criterion) {
    let content = make_dxf_content(10);
    c.bench_function("dxf_parse_10_entities", |b| {
        b.iter(|| {
            let _ = DxfParser::parse(black_box(&content));
        });
    });
}

fn bench_dxf_parse_medium(c: &mut Criterion) {
    let content = make_dxf_content(100);
    c.bench_function("dxf_parse_100_entities", |b| {
        b.iter(|| {
            let _ = DxfParser::parse(black_box(&content));
        });
    });
}

fn bench_dxf_parse_large(c: &mut Criterion) {
    let content = make_dxf_content(1000);
    c.bench_function("dxf_parse_1000_entities", |b| {
        b.iter(|| {
            let _ = DxfParser::parse(black_box(&content));
        });
    });
}

criterion_group!(
    toolpath_benches,
    bench_rectangle_contour,
    bench_circle_contour,
    bench_rectangle_contour_multipass,
);
criterion_group!(
    gcode_benches,
    bench_gcode_generation_small,
    bench_gcode_generation_medium,
    bench_gcode_generation_large,
);
criterion_group!(
    spatial_benches,
    bench_spatial_index_insert,
    bench_spatial_index_query,
);
criterion_group!(
    dxf_benches,
    bench_dxf_parse_small,
    bench_dxf_parse_medium,
    bench_dxf_parse_large,
);
criterion_main!(
    toolpath_benches,
    gcode_benches,
    spatial_benches,
    dxf_benches
);
