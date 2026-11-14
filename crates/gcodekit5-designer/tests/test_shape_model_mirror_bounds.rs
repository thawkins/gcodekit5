//! Regression tests for path bounds consistency, path `parametric_source`
//! persistence, and exact gear/sprocket mirroring.

use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::model::{
    DesignGear, DesignPath, DesignSprocket, DesignerShape, ParametricPathSource, Point, Shape,
};
use gcodekit5_designer::serialization::{DesignFile, ShapeData};
use lyon::path::iterator::PathIterator;

/// Flattened outline points of a lyon path.
fn outline_points(path: &lyon::path::Path) -> Vec<Point> {
    path.iter()
        .flattened(0.01)
        .filter_map(|e| match e {
            lyon::path::Event::Begin { at } => Some(Point::new(at.x as f64, at.y as f64)),
            lyon::path::Event::Line { to, .. } => Some(Point::new(to.x as f64, to.y as f64)),
            _ => None,
        })
        .collect()
}

fn outline_bounds(path: &lyon::path::Path) -> (f64, f64, f64, f64) {
    outline_points(path).iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(a, b, c, d), p| (a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y)),
    )
}

fn assert_close(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64), tol: f64) {
    assert!(
        (a.0 - b.0).abs() < tol
            && (a.1 - b.1).abs() < tol
            && (a.2 - b.2).abs() < tol
            && (a.3 - b.3).abs() < tol,
        "{a:?} vs {b:?}"
    );
}

#[test]
fn test_path_bounds_match_rendered_geometry() {
    // Open polyline: the CSG sketch is a thickened stroke, so it is wider than what
    // render() draws.
    let open = DesignPath::from_points(
        &[
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ],
        false,
    );
    assert_close(open.bounds(), (0.0, 0.0, 10.0, 10.0), 1e-6);
    assert_close(open.bounds(), outline_bounds(&open.render()), 1e-6);

    // Curved closed path.
    let mut b = lyon::path::Path::builder();
    b.begin(lyon::math::point(0.0, 0.0));
    b.cubic_bezier_to(
        lyon::math::point(0.0, 20.0),
        lyon::math::point(30.0, 20.0),
        lyon::math::point(30.0, 0.0),
    );
    b.close();
    let curve = DesignPath::from_lyon_path(&b.build());
    assert_close(curve.bounds(), outline_bounds(&curve.render()), 1e-4);
}

#[test]
fn test_path_parametric_source_survives_save_load() {
    let mut path = DesignPath::from_points(
        &[
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ],
        true,
    );
    path.parametric_source = Some(ParametricPathSource::TimingPulley {
        pitch: 2.0,
        teeth: 20,
        belt_width: 6.0,
        hole_radius: 2.5,
    });
    let obj = DrawingObject::new(1, Shape::Path(path));

    let data = DesignFile::from_drawing_object(&obj);
    let json = serde_json::to_string(&data).expect("serialize shape");
    let back: ShapeData = serde_json::from_str(&json).expect("deserialize shape");
    let restored = DesignFile::to_drawing_object(&back, 1).expect("restore shape");

    let Shape::Path(p) = &restored.shape else {
        panic!("expected a path, got {:?}", restored.shape);
    };
    match &p.parametric_source {
        Some(ParametricPathSource::TimingPulley {
            pitch,
            teeth,
            belt_width,
            hole_radius,
        }) => {
            assert_eq!(*teeth, 20);
            assert!((pitch - 2.0).abs() < 1e-12);
            assert!((belt_width - 6.0).abs() < 1e-12);
            assert!((hole_radius - 2.5).abs() < 1e-12);
        }
        None => panic!("parametric_source lost on save/load"),
    }

    // Files written before the field existed still load.
    let mut legacy: serde_json::Value = serde_json::from_str(&json).expect("json value");
    legacy
        .as_object_mut()
        .expect("shape object")
        .remove("parametric_source");
    let legacy: ShapeData = serde_json::from_value(legacy).expect("legacy shape");
    assert!(legacy.parametric_source.is_none());
}

/// Asserts every point of `actual` lies on `expected` (within `tol`) and vice versa.
fn assert_same_outline(actual: &[Point], expected: &[Point], tol: f64) {
    let near = |p: &Point, set: &[Point]| set.iter().any(|q| q.distance_to(p) < tol);
    let miss_a = actual.iter().filter(|p| !near(p, expected)).count();
    let miss_e = expected.iter().filter(|p| !near(p, actual)).count();
    assert!(
        miss_a == 0 && miss_e == 0,
        "outlines differ: {miss_a} / {} actual points and {miss_e} / {} expected points unmatched",
        actual.len(),
        expected.len()
    );
}

fn mirrored_x(points: &[Point], cx: f64) -> Vec<Point> {
    points
        .iter()
        .map(|p| Point::new(2.0 * cx - p.x, p.y))
        .collect()
}

fn mirrored_y(points: &[Point], cy: f64) -> Vec<Point> {
    points
        .iter()
        .map(|p| Point::new(p.x, 2.0 * cy - p.y))
        .collect()
}

#[test]
fn test_gear_mirror_is_a_true_reflection() {
    // Odd tooth count plus a rotation: a 180 degree turn is not a reflection.
    let mut gear = DesignGear::new(Point::new(20.0, 10.0), 2.0, 7);
    gear.rotation = 10.0;
    gear.hole_radius = 2.0;
    let before = outline_points(&gear.render());

    let mut mx = Shape::Gear(gear.clone());
    mx.scale(-1.0, 1.0, Point::new(20.0, 10.0));
    assert_same_outline(
        &outline_points(&mx.render()),
        &mirrored_x(&before, 20.0),
        0.05,
    );

    let mut my = Shape::Gear(gear.clone());
    my.scale(1.0, -1.0, Point::new(20.0, 10.0));
    assert_same_outline(
        &outline_points(&my.render()),
        &mirrored_y(&before, 10.0),
        0.05,
    );

    // Mirroring twice restores the original.
    mx.scale(-1.0, 1.0, Point::new(20.0, 10.0));
    assert_same_outline(&outline_points(&mx.render()), &before, 0.05);
}

#[test]
fn test_sprocket_mirror_is_a_true_reflection() {
    let mut sprocket = DesignSprocket::new(Point::new(0.0, 0.0), 12.7, 9);
    sprocket.rotation = 7.0;
    let before = outline_points(&sprocket.render());
    let mut shape = Shape::Sprocket(sprocket);
    shape.scale(-1.0, 1.0, Point::new(0.0, 0.0));
    assert_same_outline(
        &outline_points(&shape.render()),
        &mirrored_x(&before, 0.0),
        0.05,
    );
}

#[test]
fn test_gear_mirror_flag_survives_save_load() {
    let mut gear = DesignGear::new(Point::new(0.0, 0.0), 2.0, 7);
    gear.rotation = 10.0;
    let mut shape = Shape::Gear(gear);
    shape.scale(-1.0, 1.0, Point::new(0.0, 0.0));
    let before = outline_points(&shape.render());

    let obj = DrawingObject::new(1, shape);
    let data = DesignFile::from_drawing_object(&obj);
    let json = serde_json::to_string(&data).expect("serialize");
    let back: ShapeData = serde_json::from_str(&json).expect("deserialize");
    let restored = DesignFile::to_drawing_object(&back, 1).expect("restore");
    assert_same_outline(&outline_points(&restored.shape.render()), &before, 0.05);
}
