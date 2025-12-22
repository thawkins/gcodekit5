use gcodekit5_designer::{TextShape, ToolpathGenerator};

#[test]
fn test_text_toolpath_advances_characters() {
    let gen = ToolpathGenerator::new();

    let text = TextShape {
        text: "AB".to_string(),
        x: 0.0,
        y: 0.0,
        font_size: 10.0,
        rotation: 0.0,
        font_family: "Sans".to_string(),
        bold: false,
        italic: false,
    };

    let toolpaths = gen.generate_text_toolpath(&text, 1.0);

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;

    for tp in toolpaths {
        for seg in tp.segments {
            min_x = min_x.min(seg.start.x).min(seg.end.x);
            max_x = max_x.max(seg.start.x).max(seg.end.x);
        }
    }

    // Two glyphs should span more than a couple of mm at 10mm font size.
    assert!(max_x - min_x > 5.0);
}

#[test]
fn test_text_toolpath_contains_many_segments_for_curves() {
    let gen = ToolpathGenerator::new();

    let text = TextShape {
        text: "S".to_string(),
        x: 0.0,
        y: 0.0,
        font_size: 12.0,
        rotation: 0.0,
        font_family: "Sans".to_string(),
        bold: false,
        italic: false,
    };

    let toolpaths = gen.generate_text_toolpath(&text, 1.0);
    let seg_count: usize = toolpaths.iter().map(|tp| tp.segments.len()).sum();

    // Curvy glyphs should not collapse down to a handful of straight segments.
    assert!(seg_count > 25);
}

#[test]
fn test_text_pocket_toolpath_generates_infill() {
    let mut gen = ToolpathGenerator::new();
    gen.set_tool_diameter(2.0);
    gen.set_step_in(0.8);
    gen.set_cut_depth(1.0);

    let text = TextShape {
        text: "O".to_string(),
        x: 0.0,
        y: 0.0,
        font_size: 20.0,
        rotation: 0.0,
        font_family: "Sans".to_string(),
        bold: false,
        italic: false,
    };

    let toolpaths = gen.generate_text_pocket_toolpath(&text, 1.0);
    let segs: Vec<_> = toolpaths.iter().flat_map(|tp| tp.segments.iter()).collect();

    let profile_toolpaths = gen.generate_text_toolpath(&text, 1.0);
    let profile_segs: Vec<_> = profile_toolpaths
        .iter()
        .flat_map(|tp| tp.segments.iter())
        .collect();
    let (mut pmin_y, mut pmax_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for seg in &profile_segs {
        pmin_y = pmin_y.min(seg.start.y).min(seg.end.y);
        pmax_y = pmax_y.max(seg.start.y).max(seg.end.y);
    }
    let linear_count: usize = segs
        .iter()
        .filter(|seg| {
            seg.segment_type == gcodekit5_designer::toolpath::ToolpathSegmentType::LinearMove
        })
        .count();

    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for seg in &segs {
        min_y = min_y.min(seg.start.y).min(seg.end.y);
        max_y = max_y.max(seg.start.y).max(seg.end.y);
    }

    // Pocketing should produce a bunch of infill segments, not just a tiny outline.
    assert!(
        linear_count > 10,
        "pocket segs={}, linear={}, y=[{:.3},{:.3}] profile segs={} profile y=[{:.3},{:.3}]",
        segs.len(),
        linear_count,
        min_y,
        max_y,
        profile_segs.len(),
        pmin_y,
        pmax_y
    );
}
