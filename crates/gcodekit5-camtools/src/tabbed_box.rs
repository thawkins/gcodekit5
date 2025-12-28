//! Tabbed Box Maker
//!
//! Based on the superior algorithm from https://github.com/florianfesti/boxes
//! Uses finger/space multiples of thickness for automatic finger calculation

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
        }
    }
}

#[derive(Debug, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

fn push_unique_point(path: &mut Vec<Point>, point: Point) {
    if let Some(last) = path.last() {
        if (point.x - last.x).abs() < 0.01 && (point.y - last.y).abs() < 0.01 {
            return;
        }
    }
    path.push(point);
}

#[derive(Clone, Copy, Debug)]
struct LayoutCursor {
    x: f32,
    y: f32,
    spacing: f32,
}

impl LayoutCursor {
    fn new(spacing: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            spacing,
        }
    }

    fn place(&mut self, width: f32) -> (f32, f32) {
        let position = (self.x, self.y);
        self.x += width + self.spacing;
        position
    }

    fn next_row(&mut self, height: f32) {
        self.y += height + self.spacing;
        self.x = 0.0;
    }
}

pub struct TabbedBoxMaker {
    params: BoxParameters,
    x: f32,
    y: f32,
    h: f32,
    t: f32,
    paths: Vec<Vec<Point>>,
    path_groups: Vec<Vec<usize>>,
}

impl TabbedBoxMaker {
    pub fn new(params: BoxParameters) -> Result<Self, String> {
        Self::validate_parameters(&params)?;

        let mut x = params.x;
        let mut y = params.y;
        let mut h = params.h;

        let t = params.thickness;

        if params.outside {
            x = Self::adjust_size(x, t);
            y = Self::adjust_size(y, t);
            h = Self::adjust_size(h, t);
        }

        Ok(Self {
            params,
            x,
            y,
            h,
            t,
            paths: Vec::new(),
            path_groups: Vec::new(),
        })
    }

    fn validate_parameters(params: &BoxParameters) -> Result<(), String> {
        if params.x < 20.0 || params.y < 20.0 || params.h < 20.0 {
            return Err("All dimensions must be at least 20mm".to_string());
        }

        if params.thickness < 1.0 || params.thickness > 20.0 {
            return Err("Material thickness must be between 1mm and 20mm".to_string());
        }

        if (params.finger_joint.space + params.finger_joint.finger).abs() < 0.1 {
            return Err("Finger + space must not be close to zero".to_string());
        }

        Ok(())
    }

    fn adjust_size(size: f32, thickness: f32) -> f32 {
        size - 2.0 * thickness
    }

    /// Calculate number of fingers and leftover space for a given length
    pub fn calc_fingers(&self, length: f32) -> (usize, f32) {
        let settings = &self.params.finger_joint;
        let t = self.t;

        let space = settings.space * t;
        let finger = settings.finger * t;

        // Calculate number of fingers that fit
        let mut fingers = ((length - (settings.surrounding_spaces - 1.0) * space)
            / (space + finger))
            .floor() as usize;

        // Shrink surrounding space up to half thickness each side if needed
        if fingers == 0 && length > finger + 1.0 * t {
            fingers = 1;
        }

        if finger == 0.0 {
            fingers = 0;
        }

        // Calculate leftover space
        let leftover = if fingers > 0 {
            length - (fingers as f32) * (space + finger) + space
        } else {
            length
        };

        (fingers, leftover)
    }

    /// Draw finger joint edge
    fn draw_finger_edge(&self, length: f32, positive: bool) -> Vec<Point> {
        let mut path = Vec::new();
        let settings = &self.params.finger_joint;
        let t = self.t;

        let mut space = settings.space * t;
        let mut finger = settings.finger * t;
        let play = settings.play * t;
        let extra = settings.extra_length * t;
        let kerf = self.params.burn;
        let half_kerf = kerf / 2.0;
        let dogbone = settings.style == FingerStyle::Dogbone;
        let dimple_h = settings.dimple_height;
        let dimple_l = settings.dimple_length;
        // Overcut for dogbone: usually tool radius. Assuming burn is tool diameter.
        let overcut = half_kerf;

        let (fingers, mut leftover) = self.calc_fingers(length);

        // Adjust for play
        if !positive {
            finger += play;
            space -= play;
            leftover -= play;
        }

        let mut x = 0.0;

        let finger_draw;
        let space_draw;
        let leftover_draw;
        let base_y = -half_kerf;
        let tip_y;

        if positive {
            // Fingers out
            // Finger width increases by kerf
            finger_draw = finger + kerf;
            // Space width decreases by kerf
            space_draw = space - kerf;
            // Leftover decreases by kerf (split between two ends)
            leftover_draw = leftover - kerf;

            // Tip Y
            tip_y = -t - extra - half_kerf;
        } else {
            // Notches in
            // Notch width decreases by kerf
            finger_draw = finger - kerf;
            // Space width increases by kerf
            space_draw = space + kerf;
            // Leftover increases by kerf
            leftover_draw = leftover + kerf;

            // Notch Depth Y
            tip_y = t - half_kerf;
        }

        // Helper for dimpled side
        // Draws a side from (x, y1) to (x, y2) with a dimple if configured
        let draw_side = |path: &mut Vec<Point>, x: f32, y1: f32, y2: f32| {
            if dimple_h > 0.0 && dimple_l > 0.0 && (y2 - y1).abs() > dimple_l {
                let mid_y = (y1 + y2) / 2.0;
                let half_l = dimple_l / 2.0;
                let dir = if y2 > y1 { 1.0 } else { -1.0 };

                // Start of side
                path.push(Point::new(x, y1));

                // Start of dimple
                let d_start_y = mid_y - half_l * dir;
                path.push(Point::new(x, d_start_y));

                // Dimple peak
                // Dimple sticks out from the finger side.
                // If positive (finger), side goes out (tip_y) to in (base_y) or vice versa.
                // We want the dimple to bulge OUT of the finger material.
                // If positive:
                //   Left side: base -> tip. Bulge is -x direction? No, +x is along edge.
                //   Wait, x is along the edge length. y is depth.
                //   The side is vertical (constant x).
                //   We want the dimple to change x.

                // Let's assume dimple bulges OUTWARDS from the finger center.
                // But here we are drawing the outline.
                // If positive (finger), the material is "inside" the finger.
                // Left side of finger: x increases. Side is at x. Material is at x+? No.
                // We are drawing the path.
                // Finger: (x, base) -> (x, tip) -> (x+w, tip) -> (x+w, base).
                // Left side: (x, base) -> (x, tip). Material is to the right (x increasing).
                // So bulge should be to the left (-x).
                // Right side: (x+w, tip) -> (x+w, base). Material is to the left.
                // So bulge should be to the right (+x).

                // If negative (notch):
                // (x, base) -> (x, tip) -> (x+w, tip) -> (x+w, base).
                // Left side: (x, base) -> (x, tip). Material is to the left (it's a hole).
                // So bulge should be to the right (+x) (into the hole, making the hole smaller/friction).

                // Wait, for friction fit:
                // Finger: dimple sticks OUT (wider finger).
                // Notch: dimple sticks IN (narrower hole).

                // So:
                // Left side (base->tip):
                //   Positive: Bulge Left (-x).
                //   Negative: Bulge Right (+x).
                // Right side (tip->base):
                //   Positive: Bulge Right (+x).
                //   Negative: Bulge Left (-x).

                let bulge_dir = if y2 < y1 {
                    // tip -> base (Right side)
                    if positive {
                        1.0
                    } else {
                        -1.0
                    }
                } else {
                    // base -> tip (Left side)
                    if positive {
                        -1.0
                    } else {
                        1.0
                    }
                };

                path.push(Point::new(x + dimple_h * bulge_dir, mid_y));

                // End of dimple
                let d_end_y = mid_y + half_l * dir;
                path.push(Point::new(x, d_end_y));

                // End of side
                path.push(Point::new(x, y2));
            } else {
                path.push(Point::new(x, y1));
                path.push(Point::new(x, y2));
            }
        };

        // Start point
        path.push(Point::new(x, base_y));
        x += leftover_draw / 2.0;
        path.push(Point::new(x, base_y));

        // Draw fingers
        for i in 0..fingers {
            if positive {
                // Finger protrudes
                // Left side: base -> tip
                draw_side(&mut path, x, base_y, tip_y);

                x += finger_draw;
                path.push(Point::new(x, tip_y));

                // Right side: tip -> base
                draw_side(&mut path, x, tip_y, base_y);
            } else {
                // Notch for finger
                // Left side: base -> tip
                // path.push(Point::new(x, tip_y)); // Replaced by draw_side

                if dogbone {
                    // Dogbone overcut at first corner
                    path.push(Point::new(x, base_y)); // Ensure we start at base
                    path.push(Point::new(x, tip_y + overcut)); // Go past tip
                    path.push(Point::new(x - overcut, tip_y + overcut)); // Dogbone out
                    path.push(Point::new(x, tip_y)); // Back to corner
                } else {
                    draw_side(&mut path, x, base_y, tip_y);
                }

                x += finger_draw;
                path.push(Point::new(x, tip_y));

                if dogbone {
                    // Dogbone overcut at second corner
                    path.push(Point::new(x + overcut, tip_y + overcut));
                    path.push(Point::new(x, tip_y + overcut));
                    path.push(Point::new(x, base_y)); // Back to base
                } else {
                    draw_side(&mut path, x, tip_y, base_y);
                }
            }

            // Space between fingers
            if i < fingers - 1 {
                x += space_draw;
                path.push(Point::new(x, base_y));
            }
        }

        // End with leftover/2
        x += leftover_draw / 2.0;
        path.push(Point::new(x, base_y));

        path
    }

    /// Draw a rectangular wall with finger joints on specified edges
    /// edges: 4-char string, each char: 'f' = finger out, 'F' = finger in, 'e' = plain edge
    /// Edges: [0]=bottom, [1]=right, [2]=top, [3]=left
    fn draw_rectangular_wall(
        &self,
        width: f32,
        height: f32,
        edges: &str,
        start_x: f32,
        start_y: f32,
    ) -> Vec<Point> {
        let mut path: Vec<Point> = Vec::new();
        let edge_chars: Vec<char> = edges.chars().collect();
        let kerf = self.params.burn;
        let half_kerf = kerf / 2.0;

        // Bottom edge: left to right (0,0) → (width,0)
        if let Some(&c) = edge_chars.get(0) {
            if c == 'f' || c == 'F' {
                let base_path = self.draw_finger_edge(width, c == 'f');
                for p in &base_path {
                    push_unique_point(&mut path, Point::new(start_x + p.x, start_y + p.y));
                }
            } else {
                // Plain edge. Offset outwards (down)
                push_unique_point(
                    &mut path,
                    Point::new(start_x - half_kerf, start_y - half_kerf),
                );
                push_unique_point(
                    &mut path,
                    Point::new(start_x + width + half_kerf, start_y - half_kerf),
                );
            }
        }
        // Ensure corner 1 is closed
        push_unique_point(
            &mut path,
            Point::new(start_x + width + half_kerf, start_y - half_kerf),
        );

        // Right edge: bottom to top (width,0) → (width,height)
        if let Some(&c) = edge_chars.get(1) {
            if c == 'f' || c == 'F' {
                let base_path = self.draw_finger_edge(height, c == 'f');
                for p in &base_path {
                    push_unique_point(&mut path, Point::new(start_x + width - p.y, start_y + p.x));
                }
            } else {
                // Plain edge. Offset outwards (right)
                push_unique_point(
                    &mut path,
                    Point::new(start_x + width + half_kerf, start_y - half_kerf),
                );
                push_unique_point(
                    &mut path,
                    Point::new(start_x + width + half_kerf, start_y + height + half_kerf),
                );
            }
        }
        // Ensure corner 2 is closed
        push_unique_point(
            &mut path,
            Point::new(start_x + width + half_kerf, start_y + height + half_kerf),
        );

        // Top edge: right to left (width,height) → (0,height)
        if let Some(&c) = edge_chars.get(2) {
            if c == 'F' || c == 'f' {
                let base_path = self.draw_finger_edge(width, c == 'f');
                for p in &base_path {
                    push_unique_point(
                        &mut path,
                        Point::new(start_x + width - p.x, start_y + height - p.y),
                    );
                }
            } else {
                // Plain edge. Offset outwards (up)
                push_unique_point(
                    &mut path,
                    Point::new(start_x + width + half_kerf, start_y + height + half_kerf),
                );
                push_unique_point(
                    &mut path,
                    Point::new(start_x - half_kerf, start_y + height + half_kerf),
                );
            }
        }
        // Ensure corner 3 is closed
        push_unique_point(
            &mut path,
            Point::new(start_x - half_kerf, start_y + height + half_kerf),
        );

        // Left edge: top to bottom (0,height) → (0,0)
        if let Some(&c) = edge_chars.get(3) {
            if c == 'f' || c == 'F' {
                let base_path = self.draw_finger_edge(height, c == 'f');
                for p in &base_path {
                    push_unique_point(&mut path, Point::new(start_x + p.y, start_y + height - p.x));
                }
            } else {
                // Plain edge. Offset outwards (left)
                push_unique_point(
                    &mut path,
                    Point::new(start_x - half_kerf, start_y + height + half_kerf),
                );
                push_unique_point(
                    &mut path,
                    Point::new(start_x - half_kerf, start_y - half_kerf),
                );
            }
        }
        // Ensure corner 4 is closed (back to start)
        push_unique_point(
            &mut path,
            Point::new(start_x - half_kerf, start_y - half_kerf),
        );

        // Ensure closed loop by connecting back to the very first point
        if let Some(first) = path.first().cloned() {
            push_unique_point(&mut path, first);
        }

        path
    }

    fn pack_paths(&mut self) {
        if self.paths.is_empty() {
            return;
        }

        struct Item {
            group_index: usize,
            width: f32,
            height: f32,
            original_min_x: f32,
            original_min_y: f32,
        }

        let mut items = Vec::new();
        let spacing = 5.0; // Gap between parts

        // If path_groups is empty (legacy or bug), treat each path as a group
        if self.path_groups.is_empty() {
            for i in 0..self.paths.len() {
                self.path_groups.push(vec![i]);
            }
        }

        for (i, group) in self.path_groups.iter().enumerate() {
            if group.is_empty() {
                continue;
            }

            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;

            for &path_idx in group {
                if path_idx >= self.paths.len() {
                    continue;
                }
                let path = &self.paths[path_idx];
                if path.is_empty() {
                    continue;
                }

                let p_min_x = path.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
                let p_max_x = path.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
                let p_min_y = path.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                let p_max_y = path.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

                min_x = min_x.min(p_min_x);
                max_x = max_x.max(p_max_x);
                min_y = min_y.min(p_min_y);
                max_y = max_y.max(p_max_y);
            }

            if min_x == f32::INFINITY {
                continue;
            }

            items.push(Item {
                group_index: i,
                width: max_x - min_x,
                height: max_y - min_y,
                original_min_x: min_x,
                original_min_y: min_y,
            });
        }

        // Sort by height descending
        items.sort_by(|a, b| {
            b.height
                .partial_cmp(&a.height)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Estimate target width as sqrt of total area * 1.5 (aspect ratio preference)
        // Or just ensure it's at least as wide as the widest item
        let total_area: f32 = items.iter().map(|i| i.width * i.height).sum();
        let max_item_width = items.iter().map(|i| i.width).fold(0.0, f32::max);
        let target_width = (total_area.sqrt() * 1.5).max(max_item_width);

        let mut current_x = 0.0;
        let mut current_y = 0.0;
        let mut row_height: f32 = 0.0;

        // Store new positions: group_index -> (x, y)
        let mut new_positions = vec![(0.0, 0.0); self.path_groups.len()];

        for item in &items {
            if current_x > 0.0 && current_x + item.width > target_width {
                // New row
                current_x = 0.0;
                current_y += row_height + spacing;
                row_height = 0.0;
            }

            new_positions[item.group_index] = (current_x, current_y);

            row_height = row_height.max(item.height);
            current_x += item.width + spacing;
        }

        // Apply new positions
        for item in items {
            let (new_x, new_y) = new_positions[item.group_index];
            let dx = new_x - item.original_min_x;
            let dy = new_y - item.original_min_y;

            let group = &self.path_groups[item.group_index];
            for &path_idx in group {
                if path_idx < self.paths.len() {
                    for p in &mut self.paths[path_idx] {
                        p.x += dx;
                        p.y += dy;
                    }
                }
            }
        }
    }

    fn apply_slots_to_path(
        &self,
        path: Vec<Point>,
        slots: &[f32],
        depth: f32,
        slot_width: f32,
    ) -> Vec<Point> {
        let mut new_path = Vec::new();
        let mut path_iter = path.into_iter();
        let mut current_slot_idx = 0;
        let mut sorted_slots = slots.to_vec();
        sorted_slots.sort_by(|a, b| a.partial_cmp(b).unwrap());

        if let Some(start) = path_iter.next() {
            new_path.push(start.clone());
            let mut last_pt = start;

            for pt in path_iter {
                // Process segment last_pt -> pt
                loop {
                    if current_slot_idx >= sorted_slots.len() {
                        new_path.push(pt.clone());
                        last_pt = pt;
                        break;
                    }

                    let s_center = sorted_slots[current_slot_idx];
                    let s_start = s_center - slot_width / 2.0;
                    let s_end = s_center + slot_width / 2.0;

                    // If we are already past this slot, move to next slot
                    if last_pt.x >= s_end {
                        current_slot_idx += 1;
                        continue;
                    }

                    // If segment ends before this slot starts, just draw segment
                    if pt.x <= s_start {
                        new_path.push(pt.clone());
                        last_pt = pt;
                        break;
                    }

                    // Intersection with slot
                    // We are at last_pt (which is < s_end).
                    // pt is > s_start.

                    // If we are before the slot, draw to start of slot
                    if last_pt.x < s_start {
                        let t = if (pt.x - last_pt.x).abs() > 0.001 {
                            (s_start - last_pt.x) / (pt.x - last_pt.x)
                        } else {
                            0.0
                        };
                        let y_at_start = last_pt.y + t * (pt.y - last_pt.y);

                        new_path.push(Point::new(s_start, y_at_start));
                        new_path.push(Point::new(s_start, y_at_start + depth));
                        new_path.push(Point::new(s_end, y_at_start + depth));
                        new_path.push(Point::new(s_end, y_at_start));

                        last_pt = Point::new(s_end, y_at_start);
                        current_slot_idx += 1;

                        // Continue loop to check for more slots in the rest of the segment (s_end -> pt)
                    } else {
                        // We are inside the slot (last_pt.x >= s_start)
                        // Skip points until we exit
                        if pt.x <= s_end {
                            // Still inside, skip pt
                            last_pt = pt;
                            break; // Get next pt from iterator
                        } else {
                            // Exiting slot
                            // We should have drawn the slot already.
                            // Just update last_pt to s_end and continue
                            // But we need y at s_end
                            let t = if (pt.x - last_pt.x).abs() > 0.001 {
                                (s_end - last_pt.x) / (pt.x - last_pt.x)
                            } else {
                                0.0
                            };
                            let y_at_end = last_pt.y + t * (pt.y - last_pt.y);

                            last_pt = Point::new(s_end, y_at_end);
                            // We don't increment slot idx here because we might have just finished the slot we were inside?
                            // No, if we were inside, we must have entered it.
                            // If we entered it, we incremented idx.
                            // Wait, if we entered it in previous segment, we incremented idx.
                            // So current_slot_idx points to NEXT slot.
                            // So s_center is NEXT slot.
                            // So last_pt.x >= s_end check at top handles it?
                            // No, s_end is NEXT slot's end.
                            // If we are inside a slot, we need to know WHICH slot.
                            // But we only track current_slot_idx.

                            // If we are inside a slot, it means we processed the entry in a previous segment.
                            // So current_slot_idx was incremented.
                            // So `sorted_slots[current_slot_idx]` is the *next* slot.
                            // So `s_start` is the *next* slot's start.
                            // So `last_pt.x` should be < `s_start` (unless slots overlap or are very close).
                            // So we shouldn't be "inside" the *current* slot (which is the next one).

                            // So the `else` block (last_pt.x >= s_start) implies we are inside the *next* slot?
                            // Yes.
                            // This means we started inside the next slot without drawing the entry?
                            // This happens if the path starts inside a slot.
                            // In that case, we should probably just skip until we exit.

                            if pt.x <= s_end {
                                last_pt = pt;
                                break;
                            } else {
                                // Exiting the slot we started inside
                                // We can't draw the slot properly because we missed the start.
                                // Just connect to exit?
                                let t = if (pt.x - last_pt.x).abs() > 0.001 {
                                    (s_end - last_pt.x) / (pt.x - last_pt.x)
                                } else {
                                    0.0
                                };
                                let y_at_end = last_pt.y + t * (pt.y - last_pt.y);
                                last_pt = Point::new(s_end, y_at_end);
                                current_slot_idx += 1;
                                continue;
                            }
                        }
                    }
                }
            }
        }
        new_path
    }

    fn draw_divider(
        &self,
        width: f32,
        height: f32,
        edges: &str,
        start_x: f32,
        start_y: f32,
        slots: &[f32],
        slot_from_top: bool,
    ) -> Vec<Point> {
        let mut path: Vec<Point> = Vec::new();
        let edge_chars: Vec<char> = edges.chars().collect();
        let kerf = self.params.burn;
        let half_kerf = kerf / 2.0;
        let slot_depth = height / 2.0;
        let slot_width = self.t - kerf; // Slot is a hole, so subtract kerf? No, hole needs to be wider?
                                        // If we want a hole of size T, we cut T-kerf? No.
                                        // If we cut a line, the kerf makes the hole wider.
                                        // So if we want hole T, we draw T-kerf.
                                        // Yes.

        // Bottom edge (0)
        if let Some(&c) = edge_chars.get(0) {
            let mut base_path = if c == 'f' || c == 'F' {
                self.draw_finger_edge(width, c == 'f')
            } else {
                vec![Point::new(0.0, -half_kerf), Point::new(width, -half_kerf)]
            };

            if !slot_from_top && !slots.is_empty() {
                base_path = self.apply_slots_to_path(base_path, slots, slot_depth, slot_width);
            }

            for p in &base_path {
                push_unique_point(&mut path, Point::new(start_x + p.x, start_y + p.y));
            }
        }
        push_unique_point(
            &mut path,
            Point::new(start_x + width + half_kerf, start_y - half_kerf),
        );

        // Right edge (1)
        if let Some(&c) = edge_chars.get(1) {
            let base_path = if c == 'f' || c == 'F' {
                self.draw_finger_edge(height, c == 'f')
            } else {
                vec![Point::new(0.0, -half_kerf), Point::new(height, -half_kerf)]
            };
            for p in &base_path {
                push_unique_point(&mut path, Point::new(start_x + width - p.y, start_y + p.x));
            }
        }
        push_unique_point(
            &mut path,
            Point::new(start_x + width + half_kerf, start_y + height + half_kerf),
        );

        // Top edge (2)
        if let Some(&c) = edge_chars.get(2) {
            let mut base_path = if c == 'F' || c == 'f' {
                self.draw_finger_edge(width, c == 'f')
            } else {
                vec![Point::new(0.0, -half_kerf), Point::new(width, -half_kerf)]
            };

            if slot_from_top && !slots.is_empty() {
                // Slots go "into" the panel.
                // For Top edge (rotated), "into" is +Y in local frame.
                base_path = self.apply_slots_to_path(base_path, slots, slot_depth, slot_width);
            }

            for p in &base_path {
                push_unique_point(
                    &mut path,
                    Point::new(start_x + width - p.x, start_y + height - p.y),
                );
            }
        }
        push_unique_point(
            &mut path,
            Point::new(start_x - half_kerf, start_y + height + half_kerf),
        );

        // Left edge (3)
        if let Some(&c) = edge_chars.get(3) {
            let base_path = if c == 'f' || c == 'F' {
                self.draw_finger_edge(height, c == 'f')
            } else {
                vec![Point::new(0.0, -half_kerf), Point::new(height, -half_kerf)]
            };
            for p in &base_path {
                push_unique_point(&mut path, Point::new(start_x + p.y, start_y + height - p.x));
            }
        }
        push_unique_point(
            &mut path,
            Point::new(start_x - half_kerf, start_y - half_kerf),
        );

        if let Some(first) = path.first().cloned() {
            push_unique_point(&mut path, first);
        }

        path
    }

    fn draw_divider_slots(
        &mut self,
        start_x: f32,
        start_y: f32,
        length: f32,
        thickness: f32,
        vertical: bool,
    ) {
        let settings = &self.params.finger_joint;

        let (fingers, mut leftover) = self.calc_fingers(length);

        let mut space = settings.space * self.t;
        let mut finger = settings.finger * self.t;
        let play = settings.play * self.t;
        let kerf = self.params.burn;

        // Adjust for play (we are making holes, so they should be larger)
        finger += play;
        space -= play;
        leftover -= play;

        // Hole dimension = Desired - Kerf
        let slot_w = finger - kerf;
        let slot_h = thickness - kerf;

        let mut pos = leftover / 2.0;

        for _ in 0..fingers {
            let (x, y, w, h) = if vertical {
                (start_x - slot_h / 2.0, start_y + pos, slot_h, slot_w)
            } else {
                (start_x + pos, start_y - slot_h / 2.0, slot_w, slot_h)
            };

            let mut path = Vec::new();
            path.push(Point::new(x, y));
            path.push(Point::new(x + w, y));
            path.push(Point::new(x + w, y + h));
            path.push(Point::new(x, y + h));
            path.push(Point::new(x, y));

            self.add_path(path);

            pos += slot_w + kerf + space + kerf;
        }
    }

    fn add_divider_slots(
        &mut self,
        count: u32,
        total_length: f32,
        start_x: f32,
        start_y: f32,
        slot_depth: f32,
        vertical: bool,
    ) {
        if count == 0 {
            return;
        }

        let spacing = total_length / (count as f32 + 1.0);
        for idx in 1..=count {
            let offset = spacing * idx as f32;
            if vertical {
                self.draw_divider_slots(start_x + offset, start_y, slot_depth, self.t, true);
            } else {
                self.draw_divider_slots(start_x, start_y + offset, slot_depth, self.t, false);
            }
        }
    }

    pub fn paths(&self) -> &Vec<Vec<Point>> {
        &self.paths
    }

    fn start_new_group(&mut self) {
        self.path_groups.push(Vec::new());
    }

    fn add_path(&mut self, path: Vec<Point>) {
        let idx = self.paths.len();
        self.paths.push(path);
        if let Some(group) = self.path_groups.last_mut() {
            group.push(idx);
        } else {
            // If no group exists, create one (fallback)
            self.path_groups.push(vec![idx]);
        }
    }

    pub fn generate(&mut self) -> Result<(), String> {
        self.paths.clear();
        self.path_groups.clear();

        let x = self.x;
        let y = self.y;
        let h = self.h;
        let _t = self.t;

        let spacing = 5.0;
        let mut layout = LayoutCursor::new(spacing);

        let (has_top, has_bottom, has_front, has_back, has_left, has_right) =
            match self.params.box_type {
                BoxType::FullBox => (true, true, true, true, true, true),
                BoxType::NoTop => (false, true, true, true, true, true),
                BoxType::NoBottom => (true, false, true, true, true, true),
                BoxType::NoSides | BoxType::NoLeftRight => (true, true, true, true, false, false),
                BoxType::NoFrontBack => (true, true, false, false, true, true),
            };

        // Helper to format edge string
        let edges = |b, r, t, l| {
            let c = |cond, ch| if cond { ch } else { 'e' };
            format!("{}{}{}{}", c(b, 'F'), c(r, 'F'), c(t, 'F'), c(l, 'F'))
        };

        // Wall 2/4 edges need 'f' for side connections
        let edges_side = |b, r, t, l| {
            let c = |cond, ch| if cond { ch } else { 'e' };
            format!("{}{}{}{}", c(b, 'F'), c(r, 'f'), c(t, 'F'), c(l, 'f'))
        };

        // Top/Bottom edges need 'f' for all connections
        let edges_tb = |b, r, t, l| {
            let c = |cond, ch| if cond { ch } else { 'e' };
            format!("{}{}{}{}", c(b, 'f'), c(r, 'f'), c(t, 'f'), c(l, 'f'))
        };

        let key_walls = self.params.key_divider_type == KeyDividerType::WallsAndFloor
            || self.params.key_divider_type == KeyDividerType::WallsOnly;
        let key_floor = self.params.key_divider_type == KeyDividerType::WallsAndFloor
            || self.params.key_divider_type == KeyDividerType::FloorOnly;

        // Wall 1: Front (x × h)
        if has_front {
            self.start_new_group();
            let e = edges(has_bottom, has_right, has_top, has_left);
            let (start_x, start_y) = layout.place(x);
            self.add_path(self.draw_rectangular_wall(x, h, &e, start_x, start_y));

            if key_walls {
                self.add_divider_slots(self.params.dividers_x, x, start_x, start_y, h, true);
            }
        }

        // Wall 2: Right (y × h)
        if has_right {
            self.start_new_group();
            let e = edges_side(has_bottom, has_back, has_top, has_front);
            let (start_x, start_y) = layout.place(y);
            self.add_path(self.draw_rectangular_wall(y, h, &e, start_x, start_y));

            if key_walls {
                self.add_divider_slots(self.params.dividers_y, y, start_x, start_y, h, true);
            }

            layout.next_row(h);
        }

        // Wall 4: Left (y × h)
        if has_left {
            self.start_new_group();
            let e = edges_side(has_bottom, has_front, has_top, has_back);
            let (start_x, start_y) = layout.place(y);
            self.add_path(self.draw_rectangular_wall(y, h, &e, start_x, start_y));

            if key_walls {
                self.add_divider_slots(self.params.dividers_y, y, start_x, start_y, h, true);
            }
        }

        // Wall 3: Back (x × h)
        if has_back {
            self.start_new_group();
            let e = edges(has_bottom, has_left, has_top, has_right);
            let (start_x, start_y) = layout.place(x);
            self.add_path(self.draw_rectangular_wall(x, h, &e, start_x, start_y));

            if key_walls {
                self.add_divider_slots(self.params.dividers_x, x, start_x, start_y, h, true);
            }
        }

        // Top: x × y
        if has_top {
            self.start_new_group();
            let e = edges_tb(has_front, has_right, has_back, has_left);
            let (start_x, start_y) = layout.place(x);
            self.add_path(self.draw_rectangular_wall(x, y, &e, start_x, start_y));
        }

        // Bottom: x × y
        if has_bottom {
            self.start_new_group();
            let e = edges_tb(has_front, has_right, has_back, has_left);
            let (start_x, start_y) = layout.place(x);
            self.add_path(self.draw_rectangular_wall(x, y, &e, start_x, start_y));

            if key_floor {
                self.add_divider_slots(self.params.dividers_x, x, start_x, start_y, y, true);
                self.add_divider_slots(self.params.dividers_y, y, start_x, start_y, x, false);
            }
        }

        // Dividers
        let div_edges_x = match self.params.key_divider_type {
            KeyDividerType::WallsAndFloor => "ffef", // Bottom=f(Tab), Right=f(Tab), Top=e(Plain), Left=f(Tab)
            KeyDividerType::WallsOnly => "efef", // Bottom=e(Plain), Right=f(Tab), Top=e(Plain), Left=f(Tab)
            KeyDividerType::FloorOnly => "feee", // Bottom=f(Tab), Right=e(Plain), Top=e(Plain), Left=e(Plain)
            KeyDividerType::None => "eeee",
        };

        // Calculate slot positions
        let mut slots_x = Vec::new(); // For Y-dividers (spanning X)
        if self.params.dividers_x > 0 {
            let spacing_x = x / (self.params.dividers_x as f32 + 1.0);
            for i in 1..=self.params.dividers_x {
                slots_x.push(i as f32 * spacing_x);
            }
        }

        let mut slots_y = Vec::new(); // For X-dividers (spanning Y)
        if self.params.dividers_y > 0 {
            let spacing_y = y / (self.params.dividers_y as f32 + 1.0);
            for i in 1..=self.params.dividers_y {
                slots_y.push(i as f32 * spacing_y);
            }
        }

        if self.params.dividers_x > 0 {
            for _ in 0..self.params.dividers_x {
                // X-divider (Width=Y). Slots at Y-positions (slots_y).
                // Slots from Top (Plain edge).
                self.start_new_group();
                let (start_x, start_y) = layout.place(y);
                self.add_path(self.draw_divider(
                    y,
                    h,
                    div_edges_x,
                    start_x,
                    start_y,
                    &slots_y,
                    true,
                ));
            }
        }

        if self.params.dividers_y > 0 {
            for _ in 0..self.params.dividers_y {
                // Y-divider (Width=X). Slots at X-positions (slots_x).
                // Slots from Bottom (Connecting edge).
                self.start_new_group();
                let (start_x, start_y) = layout.place(x);
                self.add_path(self.draw_divider(
                    x,
                    h,
                    div_edges_x,
                    start_x,
                    start_y,
                    &slots_x,
                    false,
                ));
            }
        }

        if self.params.optimize_layout {
            self.pack_paths();
        }

        // Normalize so (offset_x, offset_y) is always the minimum XY.
        // Without this, finger joints can protrude into negative coordinates even with 0 offsets.
        // Note: guard against accidental NaNs preventing normalization.
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        for path in &self.paths {
            for p in path {
                if p.x.is_finite() {
                    min_x = min_x.min(p.x);
                }
                if p.y.is_finite() {
                    min_y = min_y.min(p.y);
                }
            }
        }
        if min_x.is_finite() && min_y.is_finite() {
            for path in &mut self.paths {
                for p in path {
                    if p.x.is_finite() {
                        p.x -= min_x;
                    }
                    if p.y.is_finite() {
                        p.y -= min_y;
                    }
                }
            }
        }

        // Apply global offset to all paths
        if self.params.offset_x != 0.0 || self.params.offset_y != 0.0 {
            for path in &mut self.paths {
                for point in path {
                    point.x += self.params.offset_x;
                    point.y += self.params.offset_y;
                }
            }
        }

        Ok(())
    }

    pub fn to_gcode(&self) -> String {
        let mut gcode = String::new();

        gcode.push_str("; Tabbed Box Maker G-code\n");
        gcode.push_str("; Based on https://github.com/florianfesti/boxes\n");
        gcode.push_str(";\n");
        gcode.push_str("; --- Box Dimensions ---\n");
        gcode.push_str(&format!(
            "; Dimensions: {}x{}x{} mm\n",
            self.params.x, self.params.y, self.params.h
        ));
        gcode.push_str(&format!("; Outside Dimensions: {}\n", self.params.outside));
        gcode.push_str(&format!("; Box Type: {:?}\n", self.params.box_type));
        gcode.push_str(&format!("; Dividers X: {}\n", self.params.dividers_x));
        gcode.push_str(&format!("; Dividers Y: {}\n", self.params.dividers_y));
        gcode.push_str(&format!(
            "; Divider Keying: {:?}\n",
            self.params.key_divider_type
        ));
        gcode.push_str(&format!(
            "; Optimize Layout: {}\n",
            self.params.optimize_layout
        ));
        gcode.push_str(";\n");

        gcode.push_str("; --- Material Settings ---\n");
        gcode.push_str(&format!(
            "; Material thickness: {} mm\n",
            self.params.thickness
        ));
        gcode.push_str(&format!("; Burn / Tool Dia: {} mm\n", self.params.burn));
        gcode.push_str(";\n");

        gcode.push_str("; --- Finger Joint Settings ---\n");
        gcode.push_str(&format!(
            "; Finger width: {} * thickness = {} mm\n",
            self.params.finger_joint.finger,
            self.params.finger_joint.finger * self.params.thickness
        ));
        gcode.push_str(&format!(
            "; Space width: {} * thickness = {} mm\n",
            self.params.finger_joint.space,
            self.params.finger_joint.space * self.params.thickness
        ));
        gcode.push_str(&format!(
            "; Surrounding spaces: {}\n",
            self.params.finger_joint.surrounding_spaces
        ));
        gcode.push_str(&format!(
            "; Play: {} mm\n",
            self.params.finger_joint.play * self.params.thickness
        ));
        gcode.push_str(&format!(
            "; Extra length: {} mm\n",
            self.params.finger_joint.extra_length * self.params.thickness
        ));
        gcode.push_str(&format!(
            "; Finger Style: {:?}\n",
            self.params.finger_joint.style
        ));
        gcode.push_str(";\n");

        gcode.push_str("; --- Laser Settings ---\n");
        gcode.push_str(&format!("; Laser passes: {}\n", self.params.laser_passes));
        gcode.push_str(&format!("; Laser power: S{}\n", self.params.laser_power));
        gcode.push_str(&format!(
            "; Feed rate: {:.0} mm/min\n",
            self.params.feed_rate
        ));
        gcode.push_str(";\n");

        gcode.push_str("; --- Work Origin Offsets ---\n");
        gcode.push_str(&format!("; Offset X: {} mm\n", self.params.offset_x));
        gcode.push_str(&format!("; Offset Y: {} mm\n", self.params.offset_y));
        gcode.push_str(";\n");

        gcode.push_str("; Initialization\n");
        gcode.push_str("G21 ; Set units to millimeters\n");
        gcode.push_str("G90 ; Absolute positioning\n");
        gcode.push_str("G17 ; XY plane selection\n");
        gcode.push_str("\n");

        gcode.push_str("; Home and set work coordinate system\n");
        gcode.push_str("$H ; Home all axes\n");
        gcode.push_str("G10 L2 P1 X0 Y0 Z0 ; Clear G54 offset\n");
        gcode.push_str("G54 ; Select work coordinate system 1\n");
        gcode.push_str(&format!(
            "G0 Z{:.2} F{:.0} ; Move to safe height\n\n",
            5.0, self.params.feed_rate
        ));

        // Ensure emitted coordinates honour user offsets (offset_x/offset_y represent desired minimum XY).
        // This also guards against any upstream layout producing negative min bounds.
        let (shift_x, shift_y) = {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            for path in &self.paths {
                for p in path {
                    if p.x.is_finite() {
                        min_x = min_x.min(p.x);
                    }
                    if p.y.is_finite() {
                        min_y = min_y.min(p.y);
                    }
                }
            }
            if min_x.is_finite() && min_y.is_finite() {
                (self.params.offset_x - min_x, self.params.offset_y - min_y)
            } else {
                (0.0, 0.0)
            }
        };

        let panel_names = ["Wall 1", "Wall 2", "Wall 4", "Wall 3", "Top", "Bottom"];

        for (i, path) in self.paths.iter().enumerate() {
            gcode.push_str(&format!(
                "; Panel {}: {}\n",
                i + 1,
                panel_names.get(i).unwrap_or(&"Unknown")
            ));

            if let Some(first_point) = path.first() {
                gcode.push_str(&format!(
                    "G0 X{:.2} Y{:.2} ; Rapid to start\n",
                    first_point.x + shift_x,
                    first_point.y + shift_y
                ));

                for pass_num in 1..=self.params.laser_passes {
                    let z_depth = -(pass_num as f32 - 1.0) * self.params.z_step_down;
                    gcode.push_str(&format!(
                        "; Pass {}/{} at Z{:.2}\n",
                        pass_num, self.params.laser_passes, z_depth
                    ));
                    
                    if pass_num > 1 {
                        gcode.push_str(&format!(
                            "G0 Z{:.2} ; Move to pass depth\n",
                            z_depth
                        ));
                    }
                    
                    gcode.push_str(&format!("M3 S{} ; Laser on\n", self.params.laser_power));

                    for (idx, point) in path.iter().skip(1).enumerate() {
                        if idx == 0 {
                            gcode.push_str(&format!(
                                "G1 X{:.2} Y{:.2} F{:.0}\n",
                                point.x + shift_x,
                                point.y + shift_y,
                                self.params.feed_rate
                            ));
                        } else {
                            gcode.push_str(&format!(
                                "G1 X{:.2} Y{:.2}\n",
                                point.x + shift_x,
                                point.y + shift_y
                            ));
                        }
                    }

                    gcode.push_str("M5 ; Laser off\n");

                    if pass_num < self.params.laser_passes {
                        gcode.push_str(&format!(
                            "G0 X{:.2} Y{:.2} ; Return to start\n",
                            first_point.x + shift_x,
                            first_point.y + shift_y
                        ));
                    }
                }
            }

            gcode.push_str("\n");
        }

        gcode.push_str("M5 ; Ensure laser off\n");
        gcode.push_str("G0 Z10.0 ; Move to safe height\n");
        gcode.push_str("G0 X0 Y0 ; Return to origin\n");
        gcode.push_str("M2 ; Program end\n");

        gcode
    }
}
