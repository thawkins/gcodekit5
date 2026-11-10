//! Tests for the Designer Array dialog's input parsing.

use gcodekit5_core::units::MeasurementSystem;
use gcodekit5_ui::ui::gtk::designer_canvas::array_dialog::{
    ArrayInput, ArraySpec, parse_array_input,
};

fn input(kind: u32) -> ArrayInput {
    ArrayInput {
        kind,
        count: "4".into(),
        rows: "2".into(),
        cols: "3".into(),
        dx: "10".into(),
        dy: "5".into(),
        cx: "1".into(),
        cy: "2".into(),
        angle: "180".into(),
    }
}

#[test]
fn test_array_dialog_parses_each_kind() {
    let m = MeasurementSystem::Metric;
    assert_eq!(
        parse_array_input(&input(0), m),
        Ok(ArraySpec::Linear {
            count: 4,
            dx: 10.0,
            dy: 5.0
        })
    );
    assert_eq!(
        parse_array_input(&input(1), m),
        Ok(ArraySpec::Grid {
            rows: 2,
            cols: 3,
            dx: 10.0,
            dy: 5.0
        })
    );
    assert_eq!(
        parse_array_input(&input(2), m),
        Ok(ArraySpec::Circular {
            count: 4,
            cx: 1.0,
            cy: 2.0,
            angle: 180.0
        })
    );
}

#[test]
fn test_array_dialog_converts_inches_and_rejects_bad_values() {
    let spec = parse_array_input(&input(0), MeasurementSystem::Imperial).expect("ok");
    assert_eq!(
        spec,
        ArraySpec::Linear {
            count: 4,
            dx: 254.0,
            dy: 127.0
        }
    );
    let mut bad = input(0);
    bad.count = "1".into();
    assert!(parse_array_input(&bad, MeasurementSystem::Metric).is_err());
    let mut bad = input(2);
    bad.angle = "0".into();
    assert!(parse_array_input(&bad, MeasurementSystem::Metric).is_err());
    let mut bad = input(1);
    bad.dx = "abc".into();
    assert!(parse_array_input(&bad, MeasurementSystem::Metric).is_err());
}
