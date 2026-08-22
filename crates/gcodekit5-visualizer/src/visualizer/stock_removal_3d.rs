//! # 3D Stock Removal Simulation
//!
//! Simulates material removal from the stock block as the toolpath
//! is executed. Uses a heightmap or voxel approach to visualize
//! the workpiece after cutting operations.

use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolpathSegmentType {
    RapidMove,
    LinearMove,
    ArcCW,
    ArcCCW,
}

#[derive(Debug, Clone)]
pub struct ToolpathSegment {
    pub segment_type: ToolpathSegmentType,
    pub start: (f32, f32, f32),
    pub end: (f32, f32, f32),
    pub center: Option<(f32, f32)>,
    pub feed_rate: f32,
    pub spindle_speed: f32,
}

#[derive(Debug, Clone)]
pub enum ToolpathSegment3D {
    RapidMove { to: Vec3 },
    LinearMove { to: Vec3 },
    ArcCW { to: Vec3, center: Vec3 },
    ArcCCW { to: Vec3, center: Vec3 },
}

/// A simple voxel grid for stock simulation
pub struct VoxelGrid {
    width: usize,
    height: usize,
    depth: usize,
    resolution: f32,
    voxels: Vec<u8>, // 255 = material present, 0 = removed
}

impl VoxelGrid {
    pub fn new(width: f32, height: f32, depth: f32, resolution: f32) -> Self {
        let w = (width / resolution).ceil() as usize;
        let h = (height / resolution).ceil() as usize;
        let d = (depth / resolution).ceil() as usize;
        let total_voxels = w * h * d;
        Self {
            width: w,
            height: h,
            depth: d,
            resolution,
            voxels: vec![255; total_voxels],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn depth(&self) -> usize {
        self.depth
    }
    pub fn resolution(&self) -> f32 {
        self.resolution
    }

    pub fn dimensions(&self) -> (usize, usize, usize) {
        (self.width, self.height, self.depth)
    }

    pub fn data(&self) -> &[u8] {
        &self.voxels
    }

    pub fn get_at_position(&self, x: usize, y: usize, z: usize) -> u8 {
        if x >= self.width || y >= self.height || z >= self.depth {
            return 0;
        }
        self.voxels[z * self.width * self.height + y * self.width + x]
    }

    pub fn set_at_position(&mut self, x: usize, y: usize, z: usize, value: u8) {
        if x >= self.width || y >= self.height || z >= self.depth {
            return;
        }
        self.voxels[z * self.width * self.height + y * self.width + x] = value;
    }

    pub fn remove_sphere(&mut self, center: Vec3, radius: f32) {
        let cx = (center.x / self.resolution) as i32;
        let cy = (center.y / self.resolution) as i32;
        let cz = (center.z / self.resolution) as i32;
        let r_grid = (radius / self.resolution).ceil() as i32;
        let r_sq = (radius / self.resolution).powi(2);

        let min_x = (cx - r_grid).max(0) as usize;
        let max_x = (cx + r_grid).min(self.width as i32 - 1) as usize;
        let min_y = (cy - r_grid).max(0) as usize;
        let max_y = (cy + r_grid).min(self.height as i32 - 1) as usize;
        let min_z = (cz - r_grid).max(0) as usize;
        let max_z = (cz + r_grid).min(self.depth as i32 - 1) as usize;

        for z in min_z..=max_z {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let dx = x as f32 - center.x / self.resolution;
                    let dy = y as f32 - center.y / self.resolution;
                    let dz = z as f32 - center.z / self.resolution;

                    if dx * dx + dy * dy + dz * dz <= r_sq {
                        self.set_at_position(x, y, z, 0);
                    }
                }
            }
        }
    }

    pub fn remove_cylinder_from_base(&mut self, base_center: Vec3, radius: f32) {
        let cx = (base_center.x / self.resolution) as i32;
        let cy = (base_center.y / self.resolution) as i32;
        let r_grid = (radius / self.resolution).ceil() as i32;
        let r_sq = (radius / self.resolution).powi(2);

        let min_x = (cx - r_grid).max(0) as usize;
        let max_x = (cx + r_grid).min(self.width as i32 - 1) as usize;
        let min_y = (cy - r_grid).max(0) as usize;
        let max_y = (cy + r_grid).min(self.height as i32 - 1) as usize;

        // The circular base at Z removes everything above that base level.
        let min_z = (base_center.z / self.resolution).ceil().max(0.0) as usize;
        if min_z >= self.depth {
            return;
        }

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = x as f32 - base_center.x / self.resolution;
                let dy = y as f32 - base_center.y / self.resolution;
                if dx * dx + dy * dy <= r_sq {
                    for z in min_z..self.depth {
                        self.set_at_position(x, y, z, 0);
                    }
                }
            }
        }
    }
}

// ---
pub fn generate_surface_mesh(grid: &VoxelGrid) -> Vec<f32> {
    let mut vertices = Vec::new();

    // Color para el stock (gris acero) - sin transparencia
    let color = [0.44, 0.50, 0.56, 1.0];

    // Color para caras internas (más oscuro para dar profundidad)
    let color_inner = [0.35, 0.40, 0.45, 1.0];

    let mut add_quad = |v1: Vec3, v2: Vec3, v3: Vec3, v4: Vec3, normal: Vec3, is_outer: bool| {
        let c = if is_outer { color } else { color_inner };

        // Triangle 1: v1, v2, v3
        vertices.push(v1.x); vertices.push(v1.y); vertices.push(v1.z);
        vertices.push(normal.x); vertices.push(normal.y); vertices.push(normal.z);
        vertices.extend_from_slice(&c);

        vertices.push(v2.x); vertices.push(v2.y); vertices.push(v2.z);
        vertices.push(normal.x); vertices.push(normal.y); vertices.push(normal.z);
        vertices.extend_from_slice(&c);

        vertices.push(v3.x); vertices.push(v3.y); vertices.push(v3.z);
        vertices.push(normal.x); vertices.push(normal.y); vertices.push(normal.z);
        vertices.extend_from_slice(&c);

        // Triangle 2: v1, v3, v4
        vertices.push(v1.x); vertices.push(v1.y); vertices.push(v1.z);
        vertices.push(normal.x); vertices.push(normal.y); vertices.push(normal.z);
        vertices.extend_from_slice(&c);

        vertices.push(v3.x); vertices.push(v3.y); vertices.push(v3.z);
        vertices.push(normal.x); vertices.push(normal.y); vertices.push(normal.z);
        vertices.extend_from_slice(&c);

        vertices.push(v4.x); vertices.push(v4.y); vertices.push(v4.z);
        vertices.push(normal.x); vertices.push(normal.y); vertices.push(normal.z);
        vertices.extend_from_slice(&c);
    };

    // Recorremos todas las celdas del grid
    for z in 0..grid.depth {
        for y in 0..grid.height {
            for x in 0..grid.width {
                if grid.get_at_position(x, y, z) == 0 {
                    continue;
                }

                let px = x as f32 * grid.resolution;
                let py = y as f32 * grid.resolution;
                let pz = z as f32 * grid.resolution;
                let s = grid.resolution;

                // Verificar si la cara es visible (borde del material)
                // Cara -X
                if x == 0 || grid.get_at_position(x - 1, y, z) == 0 {
                    add_quad(
                        Vec3::new(px, py, pz),
                        Vec3::new(px, py, pz + s),
                        Vec3::new(px, py + s, pz + s),
                        Vec3::new(px, py + s, pz),
                        Vec3::new(-1.0, 0.0, 0.0),
                        true,
                    );
                }
                // Cara +X
                if x == grid.width - 1 || grid.get_at_position(x + 1, y, z) == 0 {
                    add_quad(
                        Vec3::new(px + s, py, pz + s),
                        Vec3::new(px + s, py, pz),
                        Vec3::new(px + s, py + s, pz),
                        Vec3::new(px + s, py + s, pz + s),
                        Vec3::new(1.0, 0.0, 0.0),
                        true,
                    );
                }
                // Cara -Y
                if y == 0 || grid.get_at_position(x, y - 1, z) == 0 {
                    add_quad(
                        Vec3::new(px, py, pz),
                        Vec3::new(px + s, py, pz),
                        Vec3::new(px + s, py, pz + s),
                        Vec3::new(px, py, pz + s),
                        Vec3::new(0.0, -1.0, 0.0),
                        true,
                    );
                }
                // Cara +Y
                if y == grid.height - 1 || grid.get_at_position(x, y + 1, z) == 0 {
                    add_quad(
                        Vec3::new(px, py + s, pz + s),
                        Vec3::new(px + s, py + s, pz + s),
                        Vec3::new(px + s, py + s, pz),
                        Vec3::new(px, py + s, pz),
                        Vec3::new(0.0, 1.0, 0.0),
                        true,
                    );
                }
                // Cara -Z (inferior)
                if z == 0 || grid.get_at_position(x, y, z - 1) == 0 {
                    add_quad(
                        Vec3::new(px + s, py, pz),
                        Vec3::new(px, py, pz),
                        Vec3::new(px, py + s, pz),
                        Vec3::new(px + s, py + s, pz),
                        Vec3::new(0.0, 0.0, -1.0),
                        true,
                    );
                }
                // Cara +Z (superior)
                if z == grid.depth - 1 || grid.get_at_position(x, y, z + 1) == 0 {
                    add_quad(
                        Vec3::new(px, py, pz + s),
                        Vec3::new(px + s, py, pz + s),
                        Vec3::new(px + s, py + s, pz + s),
                        Vec3::new(px, py + s, pz + s),
                        Vec3::new(0.0, 0.0, 1.0),
                        true,
                    );
                }
            }
        }
    }
    vertices
}
// ---

pub struct StockSimulator3D {
    grid: VoxelGrid,
    tool_radius: f32,
}

impl StockSimulator3D {
    pub fn new(width: f32, height: f32, depth: f32, resolution: f32, tool_radius: f32) -> Self {
        Self {
            grid: VoxelGrid::new(width, height, depth, resolution),
            tool_radius,
        }
    }

    pub fn get_grid(&self) -> &VoxelGrid {
        &self.grid
    }

    pub fn simulate_toolpath(&mut self, toolpath: &[ToolpathSegment]) {
        let _ = self.simulate_toolpath_with_progress(toolpath, |_| true);
    }

    /// Remove stock using a flat-end cylindrical cutter where `tip.z` is the
    /// circular base plane of the cutter.
    fn remove_at_tool_tip(&mut self, tip: Vec3) {
        if tip.z <= 0.0 {
            return;
        }
        self.grid.remove_cylinder_from_base(tip, self.tool_radius);
    }

    /// Calculate the total distance of all segments in the toolpath
    fn calculate_total_distance(&self, toolpath: &[ToolpathSegment]) -> f32 {
        let mut total = 0.0;
        for segment in toolpath {
            total += self.calculate_segment_distance(segment);
        }
        total.max(0.001) // Evitar división por cero
    }

    /// Calculate the distance of a single segment
    fn calculate_segment_distance(&self, segment: &ToolpathSegment) -> f32 {
        match segment.segment_type {
            ToolpathSegmentType::RapidMove | ToolpathSegmentType::LinearMove => {
                let start = Vec3::new(segment.start.0, segment.start.1, segment.start.2);
                let end = Vec3::new(segment.end.0, segment.end.1, segment.end.2);
                start.distance(end)
            }
            ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                let start = Vec3::new(segment.start.0, segment.start.1, segment.start.2);
                let end = Vec3::new(segment.end.0, segment.end.1, segment.end.2);
                if let Some(center_tuple) = segment.center {
                    let center = Vec3::new(center_tuple.0, center_tuple.1, start.z);
                    let radius = (start - center).length();
                    if radius < 0.001 {
                        return 0.0;
                    }
                    let start_angle = (start.y - center.y).atan2(start.x - center.x);
                    let end_angle = (end.y - center.y).atan2(end.x - center.x);

                    let angle_span = if segment.segment_type == ToolpathSegmentType::ArcCW {
                        if end_angle > start_angle {
                            end_angle - start_angle - 2.0 * std::f32::consts::PI
                        } else {
                            end_angle - start_angle
                        }
                    } else {
                        if end_angle < start_angle {
                            end_angle - start_angle + 2.0 * std::f32::consts::PI
                        } else {
                            end_angle - start_angle
                        }
                    };

                    // Incluir la componente Z en la distancia
                    let dz = end.z - start.z;
                    let arc_length = radius * angle_span.abs();
                    (arc_length * arc_length + dz * dz).sqrt()
                } else {
                    0.0
                }
            }
        }
    }

    /// Calculate the number of steps for a segment based on resolution
    fn calculate_segment_steps(&self, segment: &ToolpathSegment) -> usize {
        let distance = self.calculate_segment_distance(segment);
        let steps = (distance / (self.grid.resolution * 0.5)).ceil() as usize;
        steps.max(1)
    }

    /// Simulate material removal with an optional progress callback.
    /// Progress is calculated based on actual distance traveled, not segment count.
    pub fn simulate_toolpath_with_progress<F>(
        &mut self,
        toolpath: &[ToolpathSegment],
        mut on_progress: F,
    ) -> bool
    where
        F: FnMut(f32) -> bool,
    {
        // Calcular distancia total para tener una referencia de progreso
        let total_distance = self.calculate_total_distance(toolpath);
        let mut accumulated_distance = 0.0;

        // Llamar con progreso 0% al inicio
        if !on_progress(0.0) {
            return false;
        }

        for segment in toolpath.iter() {
            let segment_distance = self.calculate_segment_distance(segment);
            let steps = self.calculate_segment_steps(segment);

            match segment.segment_type {
                ToolpathSegmentType::LinearMove => {
                    let start = Vec3::new(segment.start.0, segment.start.1, segment.start.2);
                    let end = Vec3::new(segment.end.0, segment.end.1, segment.end.2);
                    if !self.remove_linear_with_progress(
                        start,
                        end,
                        segment_distance,
                        total_distance,
                        steps,
                        &mut accumulated_distance,
                        &mut on_progress,
                    ) {
                        return false;
                    }
                }
                ToolpathSegmentType::ArcCW => {
                    let start = Vec3::new(segment.start.0, segment.start.1, segment.start.2);
                    let end = Vec3::new(segment.end.0, segment.end.1, segment.end.2);
                    if let Some(center_tuple) = segment.center {
                        let center = Vec3::new(center_tuple.0, center_tuple.1, start.z);
                        if !self.remove_arc_with_progress(
                            start,
                            end,
                            center,
                            true,
                            segment_distance,
                            total_distance,
                            steps,
                            &mut accumulated_distance,
                            &mut on_progress,
                        ) {
                            return false;
                        }
                    }
                }
                ToolpathSegmentType::ArcCCW => {
                    let start = Vec3::new(segment.start.0, segment.start.1, segment.start.2);
                    let end = Vec3::new(segment.end.0, segment.end.1, segment.end.2);
                    if let Some(center_tuple) = segment.center {
                        let center = Vec3::new(center_tuple.0, center_tuple.1, start.z);
                        if !self.remove_arc_with_progress(
                            start,
                            end,
                            center,
                            false,
                            segment_distance,
                            total_distance,
                            steps,
                            &mut accumulated_distance,
                            &mut on_progress,
                        ) {
                            return false;
                        }
                    }
                }
                _ => {
                    // Movimientos rápidos: no remueven material, pero avanzan el progreso
                    accumulated_distance += segment_distance;
                    let progress = (accumulated_distance / total_distance).clamp(0.0, 1.0);
                    if !on_progress(progress) {
                        return false;
                    }
                }
            }
        }

        // Asegurar 100% al final
        on_progress(1.0)
    }

    /// Remove material along a linear path with distance-based progress tracking
    fn remove_linear_with_progress<F>(
        &mut self,
        start: Vec3,
        end: Vec3,
        segment_distance: f32,
        total_distance: f32,
        steps: usize,
        accumulated_distance: &mut f32,
        on_progress: &mut F,
    ) -> bool
    where
        F: FnMut(f32) -> bool,
    {
        let steps = steps.max(1);
        let block_size = 8; // Bloques más pequeños para progreso más suave
        let num_blocks = ((steps + block_size - 1) / block_size).max(1);

        for block in 0..num_blocks {
            let block_start = block * block_size;
            let block_end = ((block + 1) * block_size).min(steps);

            // Calcular distancia acumulada al inicio del bloque
            let block_start_fraction = block_start as f32 / steps as f32;
            let distance_at_block_start = *accumulated_distance + segment_distance * block_start_fraction;

            // Progreso basado en distancia real recorrida
            let progress = (distance_at_block_start / total_distance).clamp(0.0, 1.0);

            // Llamar al callback con el progreso (pero no en cada iteración para no saturar)
            if !on_progress(progress) {
                return false;
            }

            for i in block_start..=block_end {
                let t = i as f32 / steps as f32;
                let pos = start.lerp(end, t);
                self.remove_at_tool_tip(pos);
            }
        }

        *accumulated_distance += segment_distance;
        true
    }

    /// Remove material along an arc path with distance-based progress tracking
    fn remove_arc_with_progress<F>(
        &mut self,
        start: Vec3,
        end: Vec3,
        center: Vec3,
        clockwise: bool,
        segment_distance: f32,
        total_distance: f32,
        steps: usize,
        accumulated_distance: &mut f32,
        on_progress: &mut F,
    ) -> bool
    where
        F: FnMut(f32) -> bool,
    {
        let radius = (start - center).length();
        if radius < 0.001 || steps == 0 {
            *accumulated_distance += segment_distance;
            return true;
        }

        let start_angle = (start.y - center.y).atan2(start.x - center.x);
        let end_angle = (end.y - center.y).atan2(end.x - center.x);

        let angle_span = if clockwise {
            if end_angle > start_angle {
                end_angle - start_angle - 2.0 * std::f32::consts::PI
            } else {
                end_angle - start_angle
            }
        } else {
            if end_angle < start_angle {
                end_angle - start_angle + 2.0 * std::f32::consts::PI
            } else {
                end_angle - start_angle
            }
        };

        let steps = steps.max(1);
        let block_size = 8; // Bloques más pequeños para progreso más suave
        let num_blocks = ((steps + block_size - 1) / block_size).max(1);

        for block in 0..num_blocks {
            let block_start = block * block_size;
            let block_end = ((block + 1) * block_size).min(steps);

            // Calcular distancia acumulada al inicio del bloque
            let block_start_fraction = block_start as f32 / steps as f32;
            let distance_at_block_start = *accumulated_distance + segment_distance * block_start_fraction;

            // Progreso basado en distancia real recorrida
            let progress = (distance_at_block_start / total_distance).clamp(0.0, 1.0);

            // Llamar al callback con el progreso
            if !on_progress(progress) {
                return false;
            }

            for i in block_start..=block_end {
                let t = i as f32 / steps as f32;
                let angle = start_angle + angle_span * t;
                let point = Vec3::new(
                    center.x + radius * angle.cos(),
                    center.y + radius * angle.sin(),
                    start.z + (end.z - start.z) * t,
                );
                self.remove_at_tool_tip(point);
            }
        }

        *accumulated_distance += segment_distance;
        true
    }

    // Métodos legacy para compatibilidad
    #[allow(dead_code)]
    fn remove_linear(&mut self, start: Vec3, end: Vec3) {
        let dist = start.distance(end);
        let steps = ((dist / (self.grid.resolution * 0.5)).ceil() as usize).max(1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = start.lerp(end, t);
            self.remove_at_tool_tip(pos);
        }
    }

    #[allow(dead_code)]
    fn remove_linear_cancellable<F>(&mut self, start: Vec3, end: Vec3, _on_progress: &mut F) -> bool
    where
        F: FnMut(f32) -> bool,
    {
        let dist = start.distance(end);
        let steps = ((dist / (self.grid.resolution * 0.5)).ceil() as usize).max(1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = start.lerp(end, t);
            self.remove_at_tool_tip(pos);
        }
        true
    }

    #[allow(dead_code)]
    fn remove_arc(&mut self, start: Vec3, end: Vec3, center: Vec3, clockwise: bool) {
        let radius = (start - center).length();
        let start_angle = (start.y - center.y).atan2(start.x - center.x);
        let end_angle = (end.y - center.y).atan2(end.x - center.x);
        let angle_span = if clockwise {
            if end_angle > start_angle {
                end_angle - start_angle - 2.0 * std::f32::consts::PI
            } else {
                end_angle - start_angle
            }
        } else {
            if end_angle < start_angle {
                end_angle - start_angle + 2.0 * std::f32::consts::PI
            } else {
                end_angle - start_angle
            }
        };
        let arc_length = radius * angle_span.abs();
        let resolution = self.grid.resolution;
        let steps = (arc_length / (resolution * 0.5)).ceil() as usize;
        let steps = steps.max(1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let angle = start_angle + angle_span * t;
            let point = Vec3::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
                start.z + (end.z - start.z) * t,
            );
            self.remove_at_tool_tip(point);
        }
    }

    #[allow(dead_code)]
    fn remove_arc_cancellable<F>(
        &mut self,
        start: Vec3,
        end: Vec3,
        center: Vec3,
        clockwise: bool,
        _on_progress: &mut F,
    ) -> bool
    where
        F: FnMut(f32) -> bool,
    {
        let radius = (start - center).length();
        let start_angle = (start.y - center.y).atan2(start.x - center.x);
        let end_angle = (end.y - center.y).atan2(end.x - center.x);
        let angle_span = if clockwise {
            if end_angle > start_angle {
                end_angle - start_angle - 2.0 * std::f32::consts::PI
            } else {
                end_angle - start_angle
            }
        } else {
            if end_angle < start_angle {
                end_angle - start_angle + 2.0 * std::f32::consts::PI
            } else {
                end_angle - start_angle
            }
        };
        let arc_length = radius * angle_span.abs();
        let resolution = self.grid.resolution;
        let steps = (arc_length / (resolution * 0.5)).ceil() as usize;
        let steps = steps.max(1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let angle = start_angle + angle_span * t;
            let point = Vec3::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
                start.z + (end.z - start.z) * t,
            );
            self.remove_at_tool_tip(point);
        }
        true
    }

    pub fn get_mesh(&self) -> Vec<f32> {
        generate_surface_mesh(&self.grid)
    }
}
