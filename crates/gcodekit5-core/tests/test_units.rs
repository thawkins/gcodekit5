//! Tests for `gcodekit5_core::units` (moved from the inline test module).

use gcodekit5_core::units::*;

#[test]
fn test_metric_conversion() {
    assert_eq!(format_length(10.5, MeasurementSystem::Metric), "10.500");
    assert_eq!(
        parse_length("10.5", MeasurementSystem::Metric).expect("parse failed"),
        10.5
    );
}

#[test]
fn test_imperial_decimal() {
    // 1 inch = 25.4 mm
    assert_eq!(format_length(25.4, MeasurementSystem::Imperial), "1.000");
    assert_eq!(
        parse_length("1", MeasurementSystem::Imperial).expect("parse failed"),
        25.4
    );

    // 0.5 inch = 12.7 mm
    assert_eq!(format_length(12.7, MeasurementSystem::Imperial), "0.500");
    assert_eq!(
        parse_length("0.5", MeasurementSystem::Imperial).expect("parse failed"),
        12.7
    );
}

#[test]
fn test_imperial_fraction() {
    // 1 1/2 inch = 1.5 inch = 38.1 mm
    assert_eq!(
        parse_length("1 1/2", MeasurementSystem::Imperial).expect("parse failed"),
        38.1
    );

    // 5 1/8 inch = 5.125 inch = 130.175 mm
    assert_eq!(
        parse_length("5 1/8", MeasurementSystem::Imperial).expect("parse failed"),
        130.175
    );

    // Just fraction: 1/4 inch = 0.25 inch = 6.35 mm
    assert_eq!(
        parse_length("1/4", MeasurementSystem::Imperial).expect("parse failed"),
        6.35
    );
}

#[test]
fn test_feed_rate_conversion() {
    // 1000 mm/min
    assert_eq!(
        format_feed_rate(1000.0, FeedRateUnits::MmPerMin),
        "1000.000"
    );
    assert_eq!(
        parse_feed_rate("1000", FeedRateUnits::MmPerMin).expect("parse failed"),
        1000.0
    );

    // 1000 mm/min = 16.667 mm/sec
    assert_eq!(format_feed_rate(1000.0, FeedRateUnits::MmPerSec), "16.667");
    assert_eq!(
        parse_feed_rate("16.666666", FeedRateUnits::MmPerSec)
            .expect("parse failed")
            .round(),
        1000.0
    );

    // 1000 mm/min = 39.370 in/min
    assert_eq!(format_feed_rate(1000.0, FeedRateUnits::InPerMin), "39.370");
    assert_eq!(
        parse_feed_rate("39.370078", FeedRateUnits::InPerMin)
            .expect("parse failed")
            .round(),
        1000.0
    );
}

#[test]
fn test_unit_labels() {
    assert_eq!(get_unit_label(MeasurementSystem::Metric), "mm");
    assert_eq!(get_unit_label(MeasurementSystem::Imperial), "in");
}

#[test]
fn test_negative_values() {
    assert_eq!(
        parse_length("-10.5", MeasurementSystem::Metric).expect("parse failed"),
        -10.5
    );
    assert_eq!(
        parse_length("-1", MeasurementSystem::Imperial).expect("parse failed"),
        -25.4
    );
    assert_eq!(
        parse_length("-1/2", MeasurementSystem::Imperial).expect("parse failed"),
        -12.7
    );
}

#[test]
fn test_zero_values() {
    assert_eq!(
        parse_length("0", MeasurementSystem::Metric).expect("parse failed"),
        0.0
    );
    assert_eq!(
        parse_length("0", MeasurementSystem::Imperial).expect("parse failed"),
        0.0
    );
    assert_eq!(
        parse_length("", MeasurementSystem::Metric).expect("parse failed"),
        0.0
    );
}

#[test]
fn test_whitespace_handling() {
    assert_eq!(
        parse_length("  10.5  ", MeasurementSystem::Metric).expect("parse failed"),
        10.5
    );
    assert_eq!(
        parse_length("  1  1/2  ", MeasurementSystem::Imperial).expect("parse failed"),
        38.1
    );
}

#[test]
fn test_invalid_inputs() {
    assert!(parse_length("abc", MeasurementSystem::Metric).is_err());
    assert!(parse_length("1/0", MeasurementSystem::Imperial).is_err()); // Division by zero
    assert!(parse_length("1/2/3", MeasurementSystem::Imperial).is_err()); // Invalid fraction
}
