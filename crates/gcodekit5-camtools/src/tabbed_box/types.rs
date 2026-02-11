//! Type definitions for the Tabbed Box Maker

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BoxType {
    FullBox = 0,
    NoTop = 1,
    NoBottom = 2,
    NoSides = 3,
    NoFrontBack = 4,
    NoLeftRight = 5,
}

impl From<i32> for BoxType {
    fn from(value: i32) -> Self {
        match value {
            0 => BoxType::FullBox,
            1 => BoxType::NoTop,
            2 => BoxType::NoBottom,
            3 => BoxType::NoSides,
            4 => BoxType::NoFrontBack,
            5 => BoxType::NoLeftRight,
            _ => BoxType::FullBox,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FingerStyle {
    Rectangular = 0,
    Springs = 1,
    Barbs = 2,
    Snap = 3,
    Dogbone = 4,
}

impl From<i32> for FingerStyle {
    fn from(value: i32) -> Self {
        match value {
            1 => FingerStyle::Springs,
            2 => FingerStyle::Barbs,
            3 => FingerStyle::Snap,
            4 => FingerStyle::Dogbone,
            _ => FingerStyle::Rectangular,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum KeyDividerType {
    WallsAndFloor = 0,
    WallsOnly = 1,
    FloorOnly = 2,
    None = 3,
}

impl From<i32> for KeyDividerType {
    fn from(value: i32) -> Self {
        match value {
            0 => KeyDividerType::WallsAndFloor,
            1 => KeyDividerType::WallsOnly,
            2 => KeyDividerType::FloorOnly,
            3 => KeyDividerType::None,
            _ => KeyDividerType::WallsAndFloor,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerJointSettings {
    /// Width of fingers in multiples of thickness
    pub finger: f32,
    /// Space between fingers in multiples of thickness
    pub space: f32,
    /// Space at start and end in multiples of normal spaces
    pub surrounding_spaces: f32,
    /// Extra space to allow fingers to move in/out (multiples of thickness)
    pub play: f32,
    /// Extra material for burn marks (multiples of thickness)
    pub extra_length: f32,
    /// Style of fingers
    pub style: FingerStyle,
    /// Height of dimple (friction fit bump)
    pub dimple_height: f32,
    /// Length of dimple
    pub dimple_length: f32,
}

impl Default for FingerJointSettings {
    fn default() -> Self {
        Self {
            finger: 2.0,
            space: 2.0,
            surrounding_spaces: 2.0,
            play: 0.0,
            extra_length: 0.0,
            style: FingerStyle::Rectangular,
            dimple_height: 0.0,
            dimple_length: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxParameters {
    pub x: f32,
    pub y: f32,
    pub h: f32,
    pub thickness: f32,
    pub outside: bool,
    pub box_type: BoxType,
    pub finger_joint: FingerJointSettings,
    pub burn: f32,
    pub laser_passes: i32,
    pub z_step_down: f32,
    pub laser_power: i32,
    pub feed_rate: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub dividers_x: u32,
    pub dividers_y: u32,
    pub optimize_layout: bool,
    pub key_divider_type: KeyDividerType,
    /// Number of axes on the target device (default 3).
    #[serde(default = "default_num_axes")]
    pub num_axes: u8,
}

fn default_num_axes() -> u8 {
    3
}

impl Default for BoxParameters {
    fn default() -> Self {
        Self {
            x: 100.0,
            y: 100.0,
            h: 100.0,
            thickness: 3.0,
            outside: false,
            box_type: BoxType::FullBox,
            finger_joint: FingerJointSettings::default(),
            burn: 0.1,
            laser_passes: 3,
            z_step_down: 0.5,
            laser_power: 1000,
            feed_rate: 500.0,
            offset_x: 10.0,
            offset_y: 10.0,
            dividers_x: 0,
            dividers_y: 0,
            optimize_layout: false,
            key_divider_type: KeyDividerType::WallsAndFloor,
            num_axes: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

pub fn push_unique_point(path: &mut Vec<Point>, point: Point) {
    if let Some(last) = path.last() {
        if (point.x - last.x).abs() < 0.01 && (point.y - last.y).abs() < 0.01 {
            return;
        }
    }
    path.push(point);
}
