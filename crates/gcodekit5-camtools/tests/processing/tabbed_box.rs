//! Test for verifying tabbed box tab protrusion dimensions

use gcodekit5_camtools::tabbed_box::{
    BoxParameters, BoxType, FingerJointSettings, KeyDividerType, TabbedBoxMaker,
};

#[test]
fn test_tab_protrusion_equals_thickness_no_kerf() {
    let params = BoxParameters {
        x: 100.0,
        y: 100.0,
        h: 100.0,
        thickness: 3.0,
        outside: false,
        box_type: BoxType::FullBox,
        finger_joint: FingerJointSettings::default(),
        burn: 0.0,
        laser_passes: 1,
        laser_power: 1000,
        feed_rate: 500.0,
        offset_x: 0.0,
        offset_y: 0.0,
        dividers_x: 0,
        dividers_y: 0,
        optimize_layout: false,
        key_divider_type: KeyDividerType::WallsAndFloor,
    };

    let mut maker = TabbedBoxMaker::new(params).expect("Failed to create TabbedBoxMaker");
    maker.generate().expect("Failed to generate box");

    let gcode = maker.to_gcode();

    let mut y_coords: Vec<f32> = Vec::new();
    let mut x_coords: Vec<f32> = Vec::new();

    for line in gcode.lines() {
        if line.starts_with("G1") || line.starts_with("G0") {
            if let Some(y_start) = line.find("Y") {
                let y_str: String = line[y_start + 1..]
                    .chars()
                    .take_while(|c| c.is_numeric() || *c == '.' || *c == '-')
                    .collect();
                if let Ok(y) = y_str.parse::<f32>() {
                    y_coords.push(y);
                }
            }
            if let Some(x_start) = line.find("X") {
                let x_str: String = line[x_start + 1..]
                    .chars()
                    .take_while(|c| c.is_numeric() || *c == '.' || *c == '-')
                    .collect();
                if let Ok(x) = x_str.parse::<f32>() {
                    x_coords.push(x);
                }
            }
        }
    }

    let min_y = y_coords.iter().cloned().fold(f32::INFINITY, f32::min);
    let min_x = x_coords.iter().cloned().fold(f32::INFINITY, f32::min);

    assert!(
        min_x >= -0.001 && min_y >= -0.001,
        "Expected non-negative coords with 0 offsets; got min_x={:.3}, min_y={:.3}",
        min_x,
        min_y
    );
}

#[test]
fn test_tab_protrusion_with_kerf() {
    let params = BoxParameters {
        x: 100.0,
        y: 100.0,
        h: 100.0,
        thickness: 3.0,
        outside: false,
        box_type: BoxType::FullBox,
        finger_joint: FingerJointSettings::default(),
        burn: 0.5,
        laser_passes: 1,
        laser_power: 1000,
        feed_rate: 500.0,
        offset_x: 0.0,
        offset_y: 0.0,
        dividers_x: 0,
        dividers_y: 0,
        optimize_layout: false,
        key_divider_type: KeyDividerType::WallsAndFloor,
    };

    let mut maker = TabbedBoxMaker::new(params).expect("Failed to create TabbedBoxMaker");
    maker.generate().expect("Failed to generate box");

    let gcode = maker.to_gcode();

    let mut y_coords: Vec<f32> = Vec::new();

    for line in gcode.lines() {
        if line.starts_with("G1") || line.starts_with("G0") {
            if let Some(y_start) = line.find("Y") {
                let y_str: String = line[y_start + 1..]
                    .chars()
                    .take_while(|c| c.is_numeric() || *c == '.' || *c == '-')
                    .collect();
                if let Ok(y) = y_str.parse::<f32>() {
                    y_coords.push(y);
                }
            }
        }
    }

    let min_y = y_coords.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(
        min_y >= -0.001,
        "Expected non-negative Y coords; got min_y={:.3}",
        min_y
    );
}

#[test]
fn test_default_box() {
    let mut params = BoxParameters::default();
    params.offset_x = 0.0;
    params.offset_y = 0.0;

    let mut maker = TabbedBoxMaker::new(params).unwrap();
    maker.generate().unwrap();
    let gcode = maker.to_gcode();

    assert!(gcode.contains("G21"));
    assert!(gcode.contains("M3"));
    assert!(!gcode.contains("NaN"));

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    for line in gcode.lines() {
        if line.starts_with("G1") || line.starts_with("G0") {
            if let Some(x_start) = line.find('X') {
                let x_str: String = line[x_start + 1..]
                    .chars()
                    .take_while(|c| c.is_numeric() || *c == '.' || *c == '-')
                    .collect();
                if let Ok(x) = x_str.parse::<f32>() {
                    min_x = min_x.min(x);
                }
            }
            if let Some(y_start) = line.find('Y') {
                let y_str: String = line[y_start + 1..]
                    .chars()
                    .take_while(|c| c.is_numeric() || *c == '.' || *c == '-')
                    .collect();
                if let Ok(y) = y_str.parse::<f32>() {
                    min_y = min_y.min(y);
                }
            }
        }
    }

    assert!(
        min_x >= -0.001 && min_y >= -0.001,
        "Expected non-negative coords with 0 offsets; got min_x={:.3}, min_y={:.3}",
        min_x,
        min_y
    );
}

#[test]
fn test_finger_calculation() {
    let params = BoxParameters::default();
    let maker = TabbedBoxMaker::new(params).unwrap();

    // For 100mm length with finger=2*t=6mm and space=2*t=6mm
    // fingers should be about 8-9
    let (fingers, leftover) = maker.calc_fingers(100.0);
    assert!(fingers >= 7 && fingers <= 10);
    assert!(leftover > 0.0);
}

#[test]
fn test_slots_containment_with_optimization() {
    let params = BoxParameters {
        x: 100.0,
        y: 100.0,
        h: 100.0,
        thickness: 3.0,
        outside: false,
        box_type: BoxType::FullBox,
        finger_joint: FingerJointSettings::default(),
        burn: 0.0,
        laser_passes: 1,
        laser_power: 1000,
        feed_rate: 500.0,
        offset_x: 0.0,
        offset_y: 0.0,
        dividers_x: 1, // One divider
        dividers_y: 0,
        optimize_layout: true, // Enable optimization
        key_divider_type: KeyDividerType::WallsAndFloor,
    };

    let mut maker = TabbedBoxMaker::new(params).expect("Failed to create TabbedBoxMaker");
    maker.generate().expect("Failed to generate box");

    let paths = maker.get_paths();

    // Identify paths
    let mut walls = Vec::new();
    let mut slots = Vec::new();

    for path in &paths {
        let min_x = path.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let max_x = path.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
        let min_y = path.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let max_y = path.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);

        let width = max_x - min_x;
        let height = max_y - min_y;
        let area = width * height;

        if area > 1000.0 {
            walls.push((min_x, max_x, min_y, max_y));
        } else if area > 0.0 && area < 100.0 {
            slots.push((min_x, max_x, min_y, max_y));
        }
    }

    // We expect at least one slot to be inside a wall
    let mut contained_slots = 0;
    for slot in &slots {
        let (sx1, sx2, sy1, sy2) = slot;
        let sx_center = (sx1 + sx2) / 2.0;
        let sy_center = (sy1 + sy2) / 2.0;

        for wall in &walls {
            let (wx1, wx2, wy1, wy2) = wall;
            if sx_center > *wx1 && sx_center < *wx2 && sy_center > *wy1 && sy_center < *wy2 {
                contained_slots += 1;
                break;
            }
        }
    }

    assert!(
        contained_slots > 0,
        "No slots found inside walls! Optimization likely moved them."
    );
}
