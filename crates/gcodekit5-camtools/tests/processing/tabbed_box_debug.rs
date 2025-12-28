//! Debug test to find when tabs become larger than thickness

use gcodekit5_camtools::tabbed_box::{
    BoxParameters, BoxType, FingerJointSettings, KeyDividerType, TabbedBoxMaker,
};

#[test]
fn test_various_configurations() {
    let configs = vec![
        ("No kerf, outside dims", 3.0, 0.0, false),
        ("No kerf, inside dims", 3.0, 0.0, true),
        ("With kerf, outside dims", 3.0, 0.5, false),
        ("With kerf, inside dims", 3.0, 0.5, true),
        ("Large kerf, outside", 3.0, 1.0, false),
        ("Large kerf, inside", 3.0, 1.0, true),
    ];

    for (_name, thickness, burn, outside) in configs {
        let params = BoxParameters {
            x: 100.0,
            y: 100.0,
            h: 100.0,
            thickness,
            outside,
            box_type: BoxType::FullBox,
            finger_joint: FingerJointSettings::default(),
            burn,
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

        // Extract all Y and X coordinates
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
        let _max_y = y_coords.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let min_x = x_coords.iter().cloned().fold(f32::INFINITY, f32::min);
        let _max_x = x_coords.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        if min_y < -0.1 {
            let tab_depth_y = min_y.abs();

            if (tab_depth_y - thickness).abs() > 0.1 {}
        }

        if min_x < -0.1 {
            let tab_depth_x = min_x.abs();

            if (tab_depth_x - thickness).abs() > 0.1 {}
        }
    }
}
