use gcodekit5_designer::render_optimizer::RenderOptimizer;
use gcodekit5_designer::spatial_index::Bounds;

#[test]
fn test_render_optimizer_creation() {
    let optimizer = RenderOptimizer::new(Bounds::new(-1000.0, -1000.0, 1000.0, 1000.0));
    assert_eq!(optimizer.stats().frame_count, 0);
}

#[test]
fn test_add_shape_and_get_visible() {
    let mut optimizer = RenderOptimizer::new(Bounds::new(-100.0, -100.0, 100.0, 100.0));

    optimizer.update_viewport(Bounds::new(-50.0, -50.0, 50.0, 50.0));
    optimizer.add_shape(0, &Bounds::new(0.0, 0.0, 10.0, 10.0));
    optimizer.add_shape(1, &Bounds::new(60.0, 60.0, 70.0, 70.0));

    let visible = optimizer.get_visible_shapes();
    assert!(visible.contains(&0)); // Inside viewport
}

#[test]
fn test_culling_efficiency() {
    let mut optimizer = RenderOptimizer::new(Bounds::new(-1000.0, -1000.0, 1000.0, 1000.0));

    optimizer.update_viewport(Bounds::new(-10.0, -10.0, 10.0, 10.0));

    // Add shapes inside and outside viewport
    for i in 0..20 {
        let x = (i as f64) * 50.0 - 500.0;
        optimizer.add_shape(i, &Bounds::new(x, x, x + 10.0, x + 10.0));
    }

    let visible = optimizer.get_visible_shapes();
    assert!(visible.len() < 20); // Should cull most shapes
}

#[test]
fn test_render_stats() {
    let mut optimizer = RenderOptimizer::new(Bounds::new(-100.0, -100.0, 100.0, 100.0));

    optimizer.update_viewport(Bounds::new(-50.0, -50.0, 50.0, 50.0));
    optimizer.add_shape(0, &Bounds::new(0.0, 0.0, 10.0, 10.0));

    let _visible = optimizer.get_visible_shapes();
    let stats = optimizer.stats();

    assert_eq!(stats.shapes_drawn, 1);
    assert_eq!(stats.frame_count, 0);
}

#[test]
fn test_next_frame() {
    let mut optimizer = RenderOptimizer::default();

    optimizer.next_frame();
    assert_eq!(optimizer.stats().frame_count, 1);

    optimizer.next_frame();
    assert_eq!(optimizer.stats().frame_count, 2);
}

#[test]
fn test_clear() {
    let mut optimizer = RenderOptimizer::new(Bounds::new(-100.0, -100.0, 100.0, 100.0));

    optimizer.add_shape(0, &Bounds::new(0.0, 0.0, 10.0, 10.0));
    optimizer.update_viewport(Bounds::new(-50.0, -50.0, 50.0, 50.0));
    let visible1 = optimizer.get_visible_shapes();
    assert!(!visible1.is_empty());

    optimizer.clear();
    let visible2 = optimizer.get_visible_shapes();
    assert!(visible2.is_empty());
}
