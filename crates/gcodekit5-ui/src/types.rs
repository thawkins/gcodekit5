use serde::{Deserialize, Serialize};

/// Vector engraving parameters used by the UI form layer (string fields where appropriate)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VectorEngravingParams {
    pub feed_rate: f32,
    pub travel_rate: f32,
    pub cut_power: f32,
    pub engrave_power: f32,
    pub power_scale: f32,
    pub multi_pass: bool,
    pub num_passes: i32,
    pub z_step_down: f32,
    pub invert_power: bool,
    pub desired_width: f32,
    pub offset_x: String,
    pub offset_y: String,
    pub enable_hatch: bool,
    pub hatch_angle: f32,
    pub hatch_spacing: f32,
    pub hatch_tolerance: f32,
    pub cross_hatch: bool,
    pub enable_dwell: bool,
    pub dwell_time: f32,
    pub vector_path: String,
}

/// Bitmap engraving parameter UI model
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BitmapEngravingParams {
    pub width_mm: f32,
    pub feed_rate: f32,
    pub travel_rate: f32,
    pub min_power: f32,
    pub max_power: f32,
    pub pixels_per_mm: f32,
    pub scan_direction: String,
    pub bidirectional: bool,
    pub invert: bool,
    pub line_spacing: f32,
    pub power_scale: f32,
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub rotation: String,
    pub halftone: String,
    pub halftone_dot_size: i32,
    pub halftone_threshold: i32,
    pub offset_x: String,
    pub offset_y: String,
    pub image_path: String,
}

/// Tabbed box UI params
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TabbedBoxParams {
    pub box_x: String,
    pub box_y: String,
    pub box_h: String,
    pub material_thickness: String,
    pub burn: String,
    pub finger_width: String,
    pub space_width: String,
    pub surrounding_spaces: String,
    pub play: String,
    pub extra_length: String,
    pub dimple_height: String,
    pub dimple_length: String,
    pub finger_style: i32,
    pub box_type: i32,
    pub outside_dimensions: bool,
    pub laser_passes: String,
    pub laser_power: String,
    pub feed_rate: String,
    pub offset_x: String,
    pub offset_y: String,
    pub dividers_x: String,
    pub dividers_y: String,
    pub optimize_layout: bool,
    pub key_divider_type: i32,
}

/// Jigsaw puzzle UI param model
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JigsawPuzzleParams {
    pub puzzle_width: String,
    pub puzzle_height: String,
    pub pieces_across: String,
    pub pieces_down: String,
    pub kerf: String,
    pub laser_passes: String,
    pub laser_power: String,
    pub feed_rate: String,
    pub seed: String,
    pub tab_size: String,
    pub jitter: String,
    pub corner_radius: String,
    pub offset_x: String,
    pub offset_y: String,
}
