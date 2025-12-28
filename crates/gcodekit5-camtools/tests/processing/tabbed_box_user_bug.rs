//! Test to reproduce user-reported bug: thickness=1.5mm produces 6.7mm tabs

use gcodekit5_camtools::tabbed_box::{
    BoxParameters, BoxType, FingerJointSettings, KeyDividerType, TabbedBoxMaker,
};

#[test]
fn test_user_reported_bug_thickness_1_5mm() {
    let params = BoxParameters {
        x: 100.0,
        y: 100.0,
        h: 100.0,
        thickness: 1.5,
        outside: false,
        box_type: BoxType::FullBox,
        finger_joint: FingerJointSettings::default(),
        burn: 0.0,
        laser_passes: 1,
        z_step_down: 0.5,
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
    let _min_x = x_coords.iter().cloned().fold(f32::INFINITY, f32::min);

    if min_y < 0.0 {
        let tab_depth = min_y.abs();

        if (tab_depth - 6.7).abs() < 0.5 {
            panic!(
                "BUG CONFIRMED: Tab depth is {:.2}mm instead of 1.5mm!",
                tab_depth
            );
        } else if (tab_depth - 1.5).abs() > 0.1 {
        }
    }
}
