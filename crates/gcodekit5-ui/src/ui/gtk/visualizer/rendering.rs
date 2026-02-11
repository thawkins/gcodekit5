use super::*;

use gcodekit5_core::constants as core_constants;
use gcodekit5_designer::stock_removal::{SimulationResult, StockMaterial};
use gcodekit5_devicedb::DeviceManager;
use gcodekit5_visualizer::visualizer::GCodeCommand;
use gcodekit5_visualizer::Visualizer;
use std::sync::Arc;

impl GcodeVisualizer {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw(
        cr: &gtk4::cairo::Context,
        vis: &Visualizer,
        cache: &mut RenderCache,
        width: f64,
        height: f64,
        show_rapid: bool,
        show_cut: bool,
        show_grid: bool,
        show_bounds: bool,
        show_intensity: bool,
        show_laser: bool,
        show_stock_removal: bool,
        _simulation_result: &Option<SimulationResult>,
        simulation_visualization: &Option<StockRemovalVisualization>,
        _stock_material: &Option<StockMaterial>,
        current_pos: (f32, f32, f32),
        device_manager: &Option<Arc<DeviceManager>>,
        grid_spacing_mm: f64,
        grid_major_line_width: f64,
        grid_minor_line_width: f64,
        style_context: &gtk4::StyleContext,
    ) {
        // Phase 4: Calculate cache hash from visualizer state
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        vis.commands().len().hash(&mut hasher);
        show_intensity.hash(&mut hasher);
        let new_hash = hasher.finish();
        let fg_color = style_context.color();
        let accent_color = style_context
            .lookup_color("accent_color")
            .unwrap_or(gtk4::gdk::RGBA::new(0.0, 0.5, 1.0, 1.0));
        let success_color = style_context
            .lookup_color("success_color")
            .unwrap_or(gtk4::gdk::RGBA::new(0.0, 0.8, 0.0, 1.0));
        let warning_color = style_context
            .lookup_color("warning_color")
            .unwrap_or(gtk4::gdk::RGBA::new(0.0, 0.8, 1.0, 1.0));

        // Clear background
        if show_intensity {
            cr.set_source_rgb(1.0, 1.0, 1.0);
        } else {
            cr.set_source_rgb(0.15, 0.15, 0.15);
        }
        let _ = cr.paint();

        // Determine Max S Value
        let max_s_value = if let Some(manager) = device_manager {
            manager
                .get_active_profile()
                .map(|p| p.max_s_value)
                .unwrap_or(1000.0)
        } else {
            1000.0
        };

        // Apply transformations
        let center_x = width / 2.0;
        let center_y = height / 2.0;

        let _ = cr.save();
        cr.translate(center_x, center_y);
        cr.scale(vis.zoom_scale as f64, -vis.zoom_scale as f64); // Flip Y
        cr.translate(vis.x_offset as f64, vis.y_offset as f64);

        // Draw Grid
        if show_grid {
            Self::draw_grid(
                cr,
                vis,
                grid_spacing_mm.max(0.1),
                &fg_color,
                grid_major_line_width,
                grid_minor_line_width,
            );
        }

        // Draw Machine Bounds
        if show_bounds {
            if let Some(manager) = device_manager {
                if let Some(profile) = manager.get_active_profile() {
                    let min_x = profile.x_axis.min;
                    let max_x = profile.x_axis.max;
                    let min_y = profile.y_axis.min;
                    let max_y = profile.y_axis.max;
                    let width = max_x - min_x;
                    let height = max_y - min_y;

                    cr.set_source_rgba(
                        accent_color.red() as f64,
                        accent_color.green() as f64,
                        accent_color.blue() as f64,
                        1.0,
                    );
                    cr.set_line_width(3.0 / vis.zoom_scale as f64);

                    cr.rectangle(min_x, min_y, width, height);
                    let _ = cr.stroke();
                }
            }
        }

        // Draw Origin Axes (Full World Extent)
        let extent = core_constants::WORLD_EXTENT_MM;
        cr.set_line_width(1.0 / vis.zoom_scale as f64);

        // X Axis Red
        cr.set_source_rgb(1.0, 0.0, 0.0);
        cr.move_to(-extent, 0.0);
        cr.line_to(extent, 0.0);
        let _ = cr.stroke();

        // Y Axis Green
        cr.set_source_rgb(0.0, 1.0, 0.0);
        cr.move_to(0.0, -extent);
        cr.line_to(0.0, extent);
        let _ = cr.stroke();

        // Draw Stock Removal - only draw cached result, don't regenerate
        if show_stock_removal {
            if let Some(cached_viz) = simulation_visualization {
                static DRAW_COUNTER: std::sync::atomic::AtomicU32 =
                    std::sync::atomic::AtomicU32::new(0);
                let count = DRAW_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let _ = count.is_multiple_of(10);
                Self::draw_stock_removal_cached(cr, vis, cached_viz);
            }
        }

        // Draw Toolpath - Phase 1, 2 & 3 Optimization: Batched Rendering + Viewport Culling + LOD
        cr.set_line_width(1.5 / vis.zoom_scale as f64);

        // Phase 3: Level of Detail
        let pixels_per_mm = vis.zoom_scale;

        let lod_level = if pixels_per_mm >= 1.0 {
            0 // High detail
        } else if pixels_per_mm >= 0.2 {
            1 // Medium detail
        } else if pixels_per_mm >= 0.05 {
            2 // Low detail
        } else {
            3 // Minimal (bounding box)
        };

        // Phase 2: Calculate visible viewport bounds in world coordinates
        let half_width_world = (width as f32 / 2.0) / vis.zoom_scale;
        let half_height_world = (height as f32 / 2.0) / vis.zoom_scale;

        let margin = 0.1;
        let margin_x = half_width_world * margin;
        let margin_y = half_height_world * margin;

        let view_min_x = -vis.x_offset - half_width_world - margin_x;
        let view_max_x = -vis.x_offset + half_width_world + margin_x;
        let view_min_y = -vis.y_offset - half_height_world - margin_y;
        let view_max_y = -vis.y_offset + half_height_world + margin_y;

        // OPTIMIZATION: Batch rapid moves together (single stroke) + viewport culling + LOD
        if show_rapid && lod_level < 3 {
            cr.new_path();
            cr.set_source_rgba(
                warning_color.red() as f64,
                warning_color.green() as f64,
                warning_color.blue() as f64,
                0.5,
            );

            let mut line_counter = 0u32;
            for cmd in vis.commands() {
                if let GCodeCommand::Move {
                    from,
                    to,
                    rapid: true,
                    ..
                } = cmd
                {
                    let line_min_x = from.x.min(to.x);
                    let line_max_x = from.x.max(to.x);
                    let line_min_y = from.y.min(to.y);
                    let line_max_y = from.y.max(to.y);

                    if line_max_x < view_min_x
                        || line_min_x > view_max_x
                        || line_max_y < view_min_y
                        || line_min_y > view_max_y
                    {
                        continue;
                    }

                    line_counter += 1;
                    match lod_level {
                        1 => {
                            if !line_counter.is_multiple_of(2) {
                                continue;
                            }
                        }
                        2 => {
                            if !line_counter.is_multiple_of(4) {
                                continue;
                            }
                        }
                        _ => {}
                    }

                    cr.move_to(from.x as f64, from.y as f64);
                    cr.line_to(to.x as f64, to.y as f64);
                }
            }
            let _ = cr.stroke();
        }

        // OPTIMIZATION: Batch cutting moves by intensity + LOD
        if show_cut && lod_level < 3 {
            if show_intensity {
                const INTENSITY_BUCKETS: usize = 20;

                if cache.needs_rebuild(new_hash)
                    || cache.intensity_buckets.len() != INTENSITY_BUCKETS
                {
                    cache.cache_hash = new_hash;
                    cache.intensity_buckets = vec![Vec::new(); INTENSITY_BUCKETS];
                    cache.total_lines = 0;
                    cache.cut_lines = 0;

                    for cmd in vis.commands() {
                        cache.total_lines += 1;
                        if let GCodeCommand::Move {
                            from,
                            to,
                            rapid: false,
                            intensity,
                        } = cmd
                        {
                            cache.cut_lines += 1;

                            let s = intensity.unwrap_or(0.0);
                            let mut gray = 1.0 - (s as f64 / max_s_value).clamp(0.0, 1.0);
                            if s > 0.0 && gray > 0.95 {
                                gray = 0.95;
                            }

                            let bucket_idx = ((gray * (INTENSITY_BUCKETS as f64 - 1.0)).round()
                                as usize)
                                .min(INTENSITY_BUCKETS - 1);

                            cache.intensity_buckets[bucket_idx].push((
                                from.x as f64,
                                from.y as f64,
                                to.x as f64,
                                to.y as f64,
                            ));
                        }
                    }
                }

                let mut line_counter = 0u32;

                for (bucket_idx, lines) in cache.intensity_buckets.iter().enumerate() {
                    if lines.is_empty() {
                        continue;
                    }

                    let gray = (bucket_idx as f64) / ((INTENSITY_BUCKETS - 1) as f64);
                    cr.set_source_rgb(gray, gray, gray);
                    cr.new_path();

                    for (fx, fy, tx, ty) in lines {
                        let line_min_x = (*fx as f32).min(*tx as f32);
                        let line_max_x = (*fx as f32).max(*tx as f32);
                        let line_min_y = (*fy as f32).min(*ty as f32);
                        let line_max_y = (*fy as f32).max(*ty as f32);

                        if line_max_x < view_min_x
                            || line_min_x > view_max_x
                            || line_max_y < view_min_y
                            || line_min_y > view_max_y
                        {
                            continue;
                        }

                        line_counter += 1;
                        match lod_level {
                            1 => {
                                if !line_counter.is_multiple_of(2) {
                                    continue;
                                }
                            }
                            2 => {
                                if !line_counter.is_multiple_of(4) {
                                    continue;
                                }
                            }
                            _ => {}
                        }

                        cr.move_to(*fx, *fy);
                        cr.line_to(*tx, *ty);
                    }

                    let _ = cr.stroke();
                }

                // Draw arcs separately (usually fewer)
                for cmd in vis.commands() {
                    if let GCodeCommand::Arc {
                        from,
                        to,
                        center,
                        clockwise,
                        intensity,
                    } = cmd
                    {
                        let radius =
                            ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();
                        let arc_min_x = center.x - radius;
                        let arc_max_x = center.x + radius;
                        let arc_min_y = center.y - radius;
                        let arc_max_y = center.y + radius;

                        if arc_max_x < view_min_x
                            || arc_min_x > view_max_x
                            || arc_max_y < view_min_y
                            || arc_min_y > view_max_y
                        {
                            continue;
                        }

                        let s = intensity.unwrap_or(0.0);
                        let mut gray = 1.0 - (s as f64 / max_s_value).clamp(0.0, 1.0);
                        if s > 0.0 && gray > 0.95 {
                            gray = 0.95;
                        }
                        cr.set_source_rgb(gray, gray, gray);

                        let radius = radius as f64;
                        let start_angle = (from.y - center.y).atan2(from.x - center.x) as f64;
                        let end_angle = (to.y - center.y).atan2(to.x - center.x) as f64;

                        if *clockwise {
                            cr.arc_negative(
                                center.x as f64,
                                center.y as f64,
                                radius,
                                start_angle,
                                end_angle,
                            );
                        } else {
                            cr.arc(
                                center.x as f64,
                                center.y as f64,
                                radius,
                                start_angle,
                                end_angle,
                            );
                        }
                        let _ = cr.stroke();
                    }
                }
            } else {
                // Non-intensity mode: Single color, single stroke! + viewport culling + LOD
                cr.new_path();
                cr.set_source_rgba(
                    success_color.red() as f64,
                    success_color.green() as f64,
                    success_color.blue() as f64,
                    1.0,
                );

                let mut line_counter = 0u32;
                for cmd in vis.commands() {
                    match cmd {
                        GCodeCommand::Move {
                            from,
                            to,
                            rapid: false,
                            ..
                        } => {
                            let line_min_x = from.x.min(to.x);
                            let line_max_x = from.x.max(to.x);
                            let line_min_y = from.y.min(to.y);
                            let line_max_y = from.y.max(to.y);

                            if line_max_x < view_min_x
                                || line_min_x > view_max_x
                                || line_max_y < view_min_y
                                || line_min_y > view_max_y
                            {
                                continue;
                            }

                            line_counter += 1;
                            match lod_level {
                                1 => {
                                    if !line_counter.is_multiple_of(2) {
                                        continue;
                                    }
                                }
                                2 => {
                                    if !line_counter.is_multiple_of(4) {
                                        continue;
                                    }
                                }
                                _ => {}
                            }

                            cr.move_to(from.x as f64, from.y as f64);
                            cr.line_to(to.x as f64, to.y as f64);
                        }
                        GCodeCommand::Arc {
                            from,
                            to,
                            center,
                            clockwise,
                            ..
                        } => {
                            let radius =
                                ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();
                            let arc_min_x = center.x - radius;
                            let arc_max_x = center.x + radius;
                            let arc_min_y = center.y - radius;
                            let arc_max_y = center.y + radius;

                            if arc_max_x < view_min_x
                                || arc_min_x > view_max_x
                                || arc_max_y < view_min_y
                                || arc_min_y > view_max_y
                            {
                                continue;
                            }

                            let radius = radius as f64;
                            let start_angle = (from.y - center.y).atan2(from.x - center.x) as f64;
                            let end_angle = (to.y - center.y).atan2(to.x - center.x) as f64;

                            if *clockwise {
                                cr.arc_negative(
                                    center.x as f64,
                                    center.y as f64,
                                    radius,
                                    start_angle,
                                    end_angle,
                                );
                            } else {
                                cr.arc(
                                    center.x as f64,
                                    center.y as f64,
                                    radius,
                                    start_angle,
                                    end_angle,
                                );
                            }
                        }
                        _ => {}
                    }
                }
                let _ = cr.stroke();
            }
        }

        // Phase 3 + 4: LOD Level 3 (Minimal) - Draw bounding box only at extreme zoom out
        if lod_level == 3 && show_cut {
            if cache.cutting_bounds.is_none() && cache.needs_rebuild(new_hash) {
                let mut bounds_min_x = f32::MAX;
                let mut bounds_max_x = f32::MIN;
                let mut bounds_min_y = f32::MAX;
                let mut bounds_max_y = f32::MIN;
                let mut bounds_min_z = f32::MAX;
                let mut bounds_max_z = f32::MIN;
                let mut has_bounds = false;

                for cmd in vis.commands() {
                    match cmd {
                        GCodeCommand::Move {
                            from,
                            to,
                            rapid: false,
                            ..
                        } => {
                            bounds_min_x = bounds_min_x.min(from.x).min(to.x);
                            bounds_max_x = bounds_max_x.max(from.x).max(to.x);
                            bounds_min_y = bounds_min_y.min(from.y).min(to.y);
                            bounds_max_y = bounds_max_y.max(from.y).max(to.y);
                            bounds_min_z = bounds_min_z.min(from.z).min(to.z);
                            bounds_max_z = bounds_max_z.max(from.z).max(to.z);
                            has_bounds = true;
                        }
                        GCodeCommand::Arc {
                            from,
                            to: _,
                            center,
                            ..
                        } => {
                            let radius =
                                ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();
                            bounds_min_x = bounds_min_x.min(center.x - radius);
                            bounds_max_x = bounds_max_x.max(center.x + radius);
                            bounds_min_y = bounds_min_y.min(center.y - radius);
                            bounds_max_y = bounds_max_y.max(center.y + radius);
                            bounds_min_z = bounds_min_z.min(from.z);
                            bounds_max_z = bounds_max_z.max(from.z);
                            has_bounds = true;
                        }
                        _ => {}
                    }
                }

                if has_bounds {
                    cache.cutting_bounds = Some((
                        bounds_min_x,
                        bounds_max_x,
                        bounds_min_y,
                        bounds_max_y,
                        bounds_min_z,
                        bounds_max_z,
                    ));
                }
            }

            if let Some((bounds_min_x, bounds_max_x, bounds_min_y, bounds_max_y, _, _)) =
                cache.cutting_bounds
            {
                cr.set_source_rgba(1.0, 1.0, 0.0, 0.5);
                cr.rectangle(
                    bounds_min_x as f64,
                    bounds_min_y as f64,
                    (bounds_max_x - bounds_min_x) as f64,
                    (bounds_max_y - bounds_min_y) as f64,
                );
                let _ = cr.fill();

                cr.set_source_rgb(1.0, 1.0, 0.0);
                cr.set_line_width(2.0 / vis.zoom_scale as f64);
                cr.rectangle(
                    bounds_min_x as f64,
                    bounds_min_y as f64,
                    (bounds_max_x - bounds_min_x) as f64,
                    (bounds_max_y - bounds_min_y) as f64,
                );
                let _ = cr.stroke();
            }
        }

        // Draw Laser/Spindle Position
        if show_laser {
            cr.set_source_rgb(1.0, 0.0, 0.0);
            let radius = 4.0 / vis.zoom_scale as f64;
            cr.arc(
                current_pos.0 as f64,
                current_pos.1 as f64,
                radius,
                0.0,
                2.0 * std::f64::consts::PI,
            );
            let _ = cr.fill();
        }

        let _ = cr.restore();
    }

    pub(crate) fn draw_grid(
        cr: &gtk4::cairo::Context,
        vis: &Visualizer,
        grid_size: f64,
        fg_color: &gtk4::gdk::RGBA,
        major_line_width: f64,
        minor_line_width: f64,
    ) {
        let range = core_constants::WORLD_EXTENT_MM;

        let minor_spacing = grid_size / 5.0;

        // Minor grid lines (lighter)
        cr.set_source_rgba(
            fg_color.red() as f64,
            fg_color.green() as f64,
            fg_color.blue() as f64,
            0.2,
        );
        cr.set_line_width(minor_line_width / vis.zoom_scale as f64);

        let mut x = -range;
        while x <= range {
            if ((x / grid_size).round() - x / grid_size).abs() > 0.01 {
                cr.move_to(x, -range);
                cr.line_to(x, range);
            }
            x += minor_spacing;
        }

        let mut y = -range;
        while y <= range {
            if ((y / grid_size).round() - y / grid_size).abs() > 0.01 {
                cr.move_to(-range, y);
                cr.line_to(range, y);
            }
            y += minor_spacing;
        }

        let _ = cr.stroke();

        // Major grid lines (darker)
        cr.set_source_rgba(
            fg_color.red() as f64,
            fg_color.green() as f64,
            fg_color.blue() as f64,
            0.4,
        );
        cr.set_line_width(major_line_width / vis.zoom_scale as f64);

        let mut x = -range;
        while x <= range {
            cr.move_to(x, -range);
            cr.line_to(x, range);
            x += grid_size;
        }

        let mut y = -range;
        while y <= range {
            cr.move_to(-range, y);
            cr.line_to(range, y);
            y += grid_size;
        }

        let _ = cr.stroke();
    }

    pub(crate) fn draw_stock_removal_cached(
        cr: &gtk4::cairo::Context,
        vis: &Visualizer,
        cached_viz: &StockRemovalVisualization,
    ) {
        cr.set_line_width(1.5 / vis.zoom_scale as f64);

        for layer in &cached_viz.contour_layers {
            cr.set_source_rgba(
                layer.color.0 as f64,
                layer.color.1 as f64,
                layer.color.2 as f64,
                0.7,
            );

            for contour in &layer.contours {
                if contour.len() < 2 {
                    continue;
                }

                cr.move_to(contour[0].0 as f64, contour[0].1 as f64);
                for point in &contour[1..] {
                    cr.line_to(point.0 as f64, point.1 as f64);
                }
                let _ = cr.stroke();
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn generate_stock_visualization(
        result: &SimulationResult,
        stock: &StockMaterial,
    ) -> StockRemovalVisualization {
        use gcodekit5_designer::stock_removal::visualization::generate_2d_contours;

        let num_contours = 3;
        let mut contour_layers = Vec::new();

        for i in 0..num_contours {
            let t = i as f32 / (num_contours - 1) as f32;
            let z_height = stock.thickness * t;

            let r = 1.0 - t;
            let g = 0.7 - t * 0.5;
            let b = t;

            let contours = generate_2d_contours(&result.height_map, z_height);

            let contours: Vec<Vec<(f32, f32)>> = contours
                .into_iter()
                .map(|contour| contour.into_iter().map(|p| (p.x, p.y)).collect())
                .collect();

            contour_layers.push(ContourLayer {
                _z_height: z_height,
                color: (r, g, b),
                contours,
            });
        }

        StockRemovalVisualization { contour_layers }
    }
}
