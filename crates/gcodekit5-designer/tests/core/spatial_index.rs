use gcodekit5_designer::spatial_index::{Bounds, SpatialIndex};

#[test]
fn test_bounds_creation() {
    let bounds = Bounds::new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(bounds.min_x, 0.0);
    assert_eq!(bounds.max_x, 10.0);
    assert_eq!(bounds.width(), 10.0);
    assert_eq!(bounds.height(), 10.0);
}

#[test]
fn test_bounds_center() {
    let bounds = Bounds::new(0.0, 0.0, 10.0, 10.0);
    let (cx, cy) = bounds.center();
    assert_eq!(cx, 5.0);
    assert_eq!(cy, 5.0);
}

#[test]
fn test_bounds_contains_point() {
    let bounds = Bounds::new(0.0, 0.0, 10.0, 10.0);
    assert!(bounds.contains_point(5.0, 5.0));
    assert!(bounds.contains_point(0.0, 0.0));
    assert!(bounds.contains_point(10.0, 10.0));
    assert!(!bounds.contains_point(11.0, 5.0));
    assert!(!bounds.contains_point(5.0, -1.0));
}

#[test]
fn test_bounds_intersection() {
    let b1 = Bounds::new(0.0, 0.0, 10.0, 10.0);
    let b2 = Bounds::new(5.0, 5.0, 15.0, 15.0);
    let b3 = Bounds::new(20.0, 20.0, 30.0, 30.0);

    assert!(b1.intersects(&b2));
    assert!(b2.intersects(&b1));
    assert!(!b1.intersects(&b3));
}

#[test]
fn test_spatial_index_creation() {
    let index = SpatialIndex::new(Bounds::new(-100.0, -100.0, 100.0, 100.0), 8, 16);
    let stats = index.stats();
    assert_eq!(stats.total_items, 0);
    // Wait, root is private. I should check if I can access it.
    // The original test accessed index.root.items.len().
    // If root is private, I can't access it here.
    // I should check the source code again.
    // The source code shows `root: QuadtreeNode` is private.
    // But the test was inside the module, so it could access private fields.
    // Now I'm outside, so I can only use public methods.
    // I'll use `stats()` instead.
    let stats = index.stats();
    assert_eq!(stats.total_items, 0);
}

#[test]
fn test_spatial_index_insert_and_query() {
    let mut index = SpatialIndex::new(Bounds::new(-100.0, -100.0, 100.0, 100.0), 8, 16);

    let bounds1 = Bounds::new(0.0, 0.0, 10.0, 10.0);
    let bounds2 = Bounds::new(5.0, 5.0, 15.0, 15.0);

    index.insert(0, &bounds1);
    index.insert(1, &bounds2);

    let results = index.query(&Bounds::new(7.0, 7.0, 12.0, 12.0));
    assert!(!results.is_empty());
}

#[test]
fn test_spatial_index_query_point() {
    let mut index = SpatialIndex::new(Bounds::new(-100.0, -100.0, 100.0, 100.0), 8, 16);

    let bounds = Bounds::new(0.0, 0.0, 10.0, 10.0);
    index.insert(0, &bounds);

    let results = index.query_point(5.0, 5.0);
    assert!(results.contains(&0));

    // Query far away - outside root bounds shouldn't match
    let results2 = index.query_point(150.0, 150.0);
    assert!(!results2.contains(&0));
}

#[test]
fn test_spatial_index_clear() {
    let mut index = SpatialIndex::new(Bounds::new(-100.0, -100.0, 100.0, 100.0), 8, 16);

    let bounds = Bounds::new(0.0, 0.0, 10.0, 10.0);
    index.insert(0, &bounds);
    assert!(!index.query_point(5.0, 5.0).is_empty());

    index.clear();
    assert!(index.query_point(5.0, 5.0).is_empty());
}

#[test]
fn test_spatial_index_stats() {
    let mut index = SpatialIndex::new(Bounds::new(-100.0, -100.0, 100.0, 100.0), 8, 16);

    for i in 0..20 {
        let bounds = Bounds::new(
            (i as f64) * 5.0,
            (i as f64) * 5.0,
            (i as f64) * 5.0 + 10.0,
            (i as f64) * 5.0 + 10.0,
        );
        index.insert(i, &bounds);
    }

    let stats = index.stats();
    assert!(stats.total_nodes > 1);
    assert!(stats.total_items >= 20);
}

#[test]
fn test_spatial_index_stress() {
    let mut index = SpatialIndex::new(Bounds::new(-10000.0, -10000.0, 10000.0, 10000.0), 8, 16);

    // Insert 1000 shapes
    for i in 0..1000 {
        let x = ((i as f64) % 50.0) * 10.0;
        let y = ((i as f64) / 50.0) * 10.0;
        let bounds = Bounds::new(x, y, x + 5.0, y + 5.0);
        index.insert(i, &bounds);
    }

    let stats = index.stats();
    assert!(stats.total_items >= 1000);

    // Query should be much faster than scanning all
    let query_bounds = Bounds::new(0.0, 0.0, 100.0, 100.0);
    let results = index.query(&query_bounds);
    assert!(!results.is_empty());
}

#[test]
fn test_spatial_index_large_coordinates() {
    // Use default constructor which should now have large bounds
    let mut index = SpatialIndex::default();

    // Insert item at large coordinates (previously failed at +/- 1000)
    let bounds = Bounds::new(1000.0, 1000.0, 1010.0, 1010.0);
    index.insert(1, &bounds);

    // Query point inside
    let results = index.query_point(1005.0, 1005.0);
    assert!(results.contains(&1), "Should find item at (1000, 1000)");

    // Insert item at very large coordinates
    let bounds2 = Bounds::new(50000.0, -50000.0, 50010.0, -49990.0);
    index.insert(2, &bounds2);

    // Query point inside
    let results2 = index.query_point(50005.0, -49995.0);
    assert!(results2.contains(&2), "Should find item at (50000, -50000)");
}
