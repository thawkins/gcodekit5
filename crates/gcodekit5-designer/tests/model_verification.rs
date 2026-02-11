use gcodekit5_designer::model::DesignerShape;
use gcodekit5_designer::model::{DesignCircle, DesignRectangle, Point, Shape, ShapeType};
use gcodekit5_designer::ops::{perform_boolean, BooleanOp};

#[test]
fn test_parametric_editing() {
    // 1. Create Rect
    let mut rect = DesignRectangle::new(0.0, 0.0, 100.0, 50.0);
    let shape = Shape::Rectangle(rect.clone());

    // Verify initial bounds
    let (x1, y1, x2, y2) = shape.bounds();
    assert!((x1 - 0.0).abs() < 1e-6, "x1 was {}", x1);
    assert!((y1 - 0.0).abs() < 1e-6, "y1 was {}", y1);
    assert!((x2 - 100.0).abs() < 1e-6, "x2 was {}", x2);
    assert!((y2 - 50.0).abs() < 1e-6, "y2 was {}", y2);

    // 2. Change Width (Parametric edit)
    rect.width = 200.0;
    let shape = Shape::Rectangle(rect);

    // Verify new bounds
    let (_x1, _y1, x2, _y2) = shape.bounds();
    // Center is (50, 25). Width is 200.
    // x1 = 50 - 100 = -50.
    // x2 = 50 + 100 = 150.
    assert!((x2 - 150.0).abs() < 1e-6, "New width x2 was {}", x2);
}

#[test]
fn test_boolean_transition() {
    // 1. Create Rect and Circle
    // Rect from 0,0 to 100,100
    let rect = Shape::Rectangle(DesignRectangle::new(0.0, 0.0, 100.0, 100.0));

    // Circle at 100,50 with radius 50. Extends from 50 to 150 in X, 0 to 100 in Y.
    let circle = Shape::Circle(DesignCircle::new(Point::new(100.0, 50.0), 50.0));

    // 2. Perform Union
    let result = perform_boolean(&rect, &circle, BooleanOp::Union);

    // 3. Verify result is Shape::Path
    match result {
        Shape::Path(_) => {}
        _ => panic!("Result of boolean operation should be Shape::Path"),
    }

    assert_eq!(result.shape_type(), ShapeType::Path);

    // 4. Verify bounds of result (should be roughly 0,0 to 150,100)
    let (x1, y1, x2, y2) = result.bounds();

    assert!((x1 - 0.0).abs() < 1.0, "x1 was {}", x1);
    assert!((y1 - 0.0).abs() < 1.0, "y1 was {}", y1);
    assert!((x2 - 150.0).abs() < 1.0, "x2 was {}", x2);
    assert!((y2 - 100.0).abs() < 1.0, "y2 was {}", y2);
}
