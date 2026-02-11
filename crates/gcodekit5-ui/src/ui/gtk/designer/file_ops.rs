//! File operations for DesignerView

use super::*;

impl DesignerView {
    pub fn new_file(&self) {
        let mut state = self.canvas.state.borrow_mut();
        state.canvas.clear();
        *self.current_file.borrow_mut() = None;
        drop(state);

        // Refresh layers
        self.layers.refresh(&self.canvas.state);
        self.canvas.widget.queue_draw();
        self.set_status(&t!("New design created"));
    }

    pub fn open_file(&self) {
        let dialog = FileChooserNative::builder()
            .title(t!("Open Design File"))
            .action(FileChooserAction::Open)
            .modal(true)
            .build();

        if let Some(root) = self.widget.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.set_transient_for(Some(window));
            }
        }

        // Set initial directory from settings
        if let Some(ref settings) = self.settings_persistence {
            if let Ok(settings_ref) = settings.try_borrow() {
                let default_dir = &settings_ref.config().file_processing.output_directory;
                if default_dir.exists() {
                    let file = gtk4::gio::File::for_path(default_dir);
                    let _ = dialog.set_current_folder(Some(&file));
                }
            }
        }

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some(&t!("GCodeKit Design Files")));
        filter.add_pattern("*.gckd");
        filter.add_pattern("*.gck5");
        dialog.add_filter(&filter);

        let all_filter = gtk4::FileFilter::new();
        all_filter.set_name(Some(&t!("All Files")));
        all_filter.add_pattern("*");
        dialog.add_filter(&all_filter);

        let canvas = self.canvas.clone();
        let current_file = self.current_file.clone();
        let layers = self.layers.clone();
        let status_label = self.status_label.clone();
        let toolbox = self.toolbox.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        match DesignFile::load_from_file(&path) {
                            Ok(design) => {
                                let mut state = canvas.state.borrow_mut();
                                state.canvas.clear();

                                let mut max_id = 0;
                                let mut restored_shapes: usize = 0;
                                for shape_data in design.shapes {
                                    let id = shape_data.id as u64;
                                    if id > max_id {
                                        max_id = id;
                                    }

                                    if let Ok(obj) =
                                        DesignFile::to_drawing_object(&shape_data, id as i32)
                                    {
                                        state.canvas.restore_shape(obj);
                                        restored_shapes += 1;
                                    }
                                }

                                state.canvas.set_next_id(max_id + 1);

                                // Restore tool settings from design file
                                state.tool_settings.feed_rate = design.toolpath_params.feed_rate;
                                state.tool_settings.spindle_speed =
                                    design.toolpath_params.spindle_speed as u32;
                                state.tool_settings.tool_diameter =
                                    design.toolpath_params.tool_diameter;
                                state.tool_settings.cut_depth = design.toolpath_params.cut_depth;

                                // Also update the toolpath generator to match
                                state
                                    .toolpath_generator
                                    .set_feed_rate(design.toolpath_params.feed_rate);
                                state
                                    .toolpath_generator
                                    .set_spindle_speed(design.toolpath_params.spindle_speed as u32);
                                state
                                    .toolpath_generator
                                    .set_tool_diameter(design.toolpath_params.tool_diameter);
                                state
                                    .toolpath_generator
                                    .set_cut_depth(design.toolpath_params.cut_depth);

                                // Restore stock parameters from design file (create if needed)
                                state.stock_material = Some(StockMaterial {
                                    width: design.toolpath_params.stock_width,
                                    height: design.toolpath_params.stock_height,
                                    thickness: design.toolpath_params.stock_thickness,
                                    origin: (0.0, 0.0, 0.0),
                                    safe_z: design.toolpath_params.safe_z_height,
                                });

                                // Update viewport (fallback to fit if invalid)
                                let zoom = design.viewport.zoom;
                                let pan_x = design.viewport.pan_x;
                                let pan_y = design.viewport.pan_y;
                                let viewport_ok = zoom.is_finite()
                                    && zoom > 0.0001
                                    && pan_x.is_finite()
                                    && pan_y.is_finite();
                                if viewport_ok {
                                    state.canvas.set_zoom(zoom);
                                    state.canvas.set_pan(pan_x, pan_y);
                                }

                                *current_file.borrow_mut() = Some(path.clone());
                                drop(state);

                                // If the saved viewport is missing/degenerate, frame the loaded geometry.
                                if restored_shapes > 0 && !viewport_ok {
                                    canvas.zoom_fit();
                                }

                                layers.refresh(&canvas.state);
                                // Refresh tool/stock settings UI to show loaded values
                                toolbox.refresh_settings();
                                canvas.widget.queue_draw();
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Loaded:"),
                                    path.display()
                                ));
                            }
                            Err(e) => {
                                error!("Error loading file: {}", e);
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Error loading file:"),
                                    e
                                ));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    pub(crate) fn import_file_internal(&self, kind: Option<&'static str>) {
        let title = match kind {
            Some("svg") => t!("Import SVG File"),
            Some("dxf") => t!("Import DXF File"),
            Some("stl") => t!("Import STL File (3D Shadow)"),
            _ => t!("Import Design File"),
        };

        let dialog = FileChooserNative::builder()
            .title(title)
            .action(FileChooserAction::Open)
            .modal(true)
            .build();

        if let Some(root) = self.widget.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.set_transient_for(Some(window));
            }
        }

        // Set initial directory from settings
        if let Some(ref settings) = self.settings_persistence {
            if let Ok(settings_ref) = settings.try_borrow() {
                let default_dir = &settings_ref.config().file_processing.output_directory;
                if default_dir.exists() {
                    let file = gtk4::gio::File::for_path(default_dir);
                    let _ = dialog.set_current_folder(Some(&file));
                }
            }
        }

        // Check STL import setting for STL support
        let enable_stl_import = if let Some(ref settings) = self.settings_persistence {
            if let Ok(settings_ref) = settings.try_borrow() {
                settings_ref.config().ui.enable_stl_import
            } else {
                false
            }
        } else {
            false
        };

        match kind {
            Some("svg") => {
                let svg_filter = gtk4::FileFilter::new();
                svg_filter.set_name(Some(&t!("SVG Files")));
                svg_filter.add_pattern("*.svg");
                dialog.add_filter(&svg_filter);
            }
            Some("dxf") => {
                let dxf_filter = gtk4::FileFilter::new();
                dxf_filter.set_name(Some(&t!("DXF Files")));
                dxf_filter.add_pattern("*.dxf");
                dialog.add_filter(&dxf_filter);
            }
            Some("stl") => {
                // Only show STL filter if STL import is enabled
                if enable_stl_import {
                    let stl_filter = gtk4::FileFilter::new();
                    stl_filter.set_name(Some(&t!("STL Files")));
                    stl_filter.add_pattern("*.stl");
                    dialog.add_filter(&stl_filter);
                }
            }
            _ => {
                let filter = gtk4::FileFilter::new();
                filter.set_name(Some(&t!("Supported Files")));
                filter.add_pattern("*.svg");
                filter.add_pattern("*.dxf");
                if enable_stl_import {
                    filter.add_pattern("*.stl");
                }
                dialog.add_filter(&filter);

                let svg_filter = gtk4::FileFilter::new();
                svg_filter.set_name(Some(&t!("SVG Files")));
                svg_filter.add_pattern("*.svg");
                dialog.add_filter(&svg_filter);

                let dxf_filter = gtk4::FileFilter::new();
                dxf_filter.set_name(Some(&t!("DXF Files")));
                dxf_filter.add_pattern("*.dxf");
                dialog.add_filter(&dxf_filter);

                if enable_stl_import {
                    let stl_filter = gtk4::FileFilter::new();
                    stl_filter.set_name(Some(&t!("STL Files")));
                    stl_filter.add_pattern("*.stl");
                    dialog.add_filter(&stl_filter);
                }
            }
        }

        let all_filter = gtk4::FileFilter::new();
        all_filter.set_name(Some(&t!("All Files")));
        all_filter.add_pattern("*");
        dialog.add_filter(&all_filter);

        let canvas = self.canvas.clone();
        let layers = self.layers.clone();
        let status_label = self.status_label.clone();
        let settings_persistence = self.settings_persistence.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        // Check STL import setting for STL processing
                        let enable_stl_import = if let Some(ref settings) = settings_persistence {
                            if let Ok(settings_ref) = settings.try_borrow() {
                                settings_ref.config().ui.enable_stl_import
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        let result = if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            match ext.to_lowercase().as_str() {
                                "svg" => match std::fs::read_to_string(&path) {
                                    Ok(content) => {
                                        let importer = gcodekit5_designer::import::SvgImporter::new(
                                            1.0, 0.0, 0.0,
                                        );
                                        importer.import_string(&content)
                                    }
                                    Err(e) => Err(anyhow::anyhow!("Failed to read file: {}", e)),
                                },
                                "dxf" => {
                                    let importer =
                                        gcodekit5_designer::import::DxfImporter::new(1.0, 0.0, 0.0);
                                    importer.import_file(path.to_str().unwrap_or(""))
                                }
                                "stl" => {
                                    // Only allow STL import if STL import is enabled
                                    if enable_stl_import {
                                        let importer =
                                            gcodekit5_designer::import::StlImporter::new()
                                                .with_scale(1.0)
                                                .with_centering(true);

                                        // Import STL and create shadow projection
                                        let result = importer.import_file(path.to_str().unwrap_or(""));

                                        // TODO(#16): Add 3D mesh to visualizer for preview
                                        // This would integrate with the new Scene3D system:
                                        // if let Ok(ref design) = result {
                                        //     if let Some(mesh_3d) = &design.mesh_3d {
                                        //         // Add to 3D scene for preview
                                        //         // Show 3D visualization panel
                                        //     }
                                        // }

                                        result
                                    } else {
                                        Err(anyhow::anyhow!("STL import requires the STL import feature to be enabled in settings"))
                                    }
                                }
                                _ => Err(anyhow::anyhow!("Unsupported file format")),
                            }
                        } else {
                            Err(anyhow::anyhow!("Unknown file extension"))
                        };

                        match result {
                            Ok(design) => {
                                let mut state = canvas.state.borrow_mut();

                                // Add imported shapes to canvas
                                for shape in design.shapes {
                                    state.add_shape_with_undo(shape);
                                }

                                drop(state);

                                // Make imported geometry visible immediately
                                canvas.zoom_fit();

                                layers.refresh(&canvas.state);
                                canvas.widget.queue_draw();
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Imported:"),
                                    path.display()
                                ));
                            }
                            Err(e) => {
                                error!("Error importing file: {}", e);
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Error importing file:"),
                                    e
                                ));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    pub fn import_file(&self) {
        self.import_file_internal(None);
    }

    pub fn import_svg_file(&self) {
        self.import_file_internal(Some("svg"));
    }

    pub fn import_dxf_file(&self) {
        self.import_file_internal(Some("dxf"));
    }

    pub fn import_stl_file(&self) {
        self.import_file_internal(Some("stl"));
    }

    pub fn save_file(&self) {
        let current_path = self.current_file.borrow().clone();

        if let Some(path) = current_path {
            self.save_to_path(path);
        } else {
            self.save_as_file();
        }
    }

    pub fn save_as_file(&self) {
        let dialog = FileChooserNative::builder()
            .title("Save Design File")
            .action(FileChooserAction::Save)
            .modal(true)
            .build();

        if let Some(root) = self.widget.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.set_transient_for(Some(window));
            }
        }

        // Set initial directory from settings
        if let Some(ref settings) = self.settings_persistence {
            if let Ok(settings_ref) = settings.try_borrow() {
                let default_dir = &settings_ref.config().file_processing.output_directory;
                if default_dir.exists() {
                    let file = gtk4::gio::File::for_path(default_dir);
                    let _ = dialog.set_current_folder(Some(&file));
                }
            }
        }

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some(&t!("GCodeKit Design Files")));
        filter.add_pattern("*.gckd");
        dialog.add_filter(&filter);

        let canvas = self.canvas.clone();
        let current_file = self.current_file.clone();
        let status_label = self.status_label.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(mut path) = file.path() {
                        if path.extension().is_none() {
                            path.set_extension("gckd");
                        }

                        // Save logic
                        let state = canvas.state.borrow();
                        let mut design =
                            DesignFile::new(path.file_stem().unwrap_or_default().to_string_lossy());

                        // Viewport
                        design.viewport.zoom = state.canvas.zoom();
                        design.viewport.pan_x = state.canvas.pan_x();
                        design.viewport.pan_y = state.canvas.pan_y();

                        // Tool settings
                        design.toolpath_params.feed_rate = state.tool_settings.feed_rate;
                        design.toolpath_params.spindle_speed =
                            state.tool_settings.spindle_speed as f64;
                        design.toolpath_params.tool_diameter = state.tool_settings.tool_diameter;
                        design.toolpath_params.cut_depth = state.tool_settings.cut_depth;

                        // Stock and toolpath parameters
                        if let Some(ref stock) = state.stock_material {
                            design.toolpath_params.stock_width = stock.width;
                            design.toolpath_params.stock_height = stock.height;
                            design.toolpath_params.stock_thickness = stock.thickness;
                            design.toolpath_params.safe_z_height = stock.safe_z;
                        }

                        // Shapes
                        for obj in state.canvas.shapes() {
                            let shape_data = DesignFile::from_drawing_object(obj);
                            design.shapes.push(shape_data);
                        }

                        match design.save_to_file(&path) {
                            Ok(_) => {
                                *current_file.borrow_mut() = Some(path.clone());
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Saved:"),
                                    path.display()
                                ));
                            }
                            Err(e) => {
                                error!("Error saving file: {}", e);
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Error saving file:"),
                                    e
                                ));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    pub(crate) fn save_to_path(&self, path: PathBuf) {
        let state = self.canvas.state.borrow();
        let mut design = DesignFile::new(path.file_stem().unwrap_or_default().to_string_lossy());

        // Viewport
        design.viewport.zoom = state.canvas.zoom();
        design.viewport.pan_x = state.canvas.pan_x();
        design.viewport.pan_y = state.canvas.pan_y();

        // Tool settings
        design.toolpath_params.feed_rate = state.tool_settings.feed_rate;
        design.toolpath_params.spindle_speed = state.tool_settings.spindle_speed as f64;
        design.toolpath_params.tool_diameter = state.tool_settings.tool_diameter;
        design.toolpath_params.cut_depth = state.tool_settings.cut_depth;

        // Stock and toolpath parameters
        if let Some(ref stock) = state.stock_material {
            design.toolpath_params.stock_width = stock.width;
            design.toolpath_params.stock_height = stock.height;
            design.toolpath_params.stock_thickness = stock.thickness;
            design.toolpath_params.safe_z_height = stock.safe_z;
        }

        // Shapes
        for obj in state.canvas.shapes() {
            let shape_data = DesignFile::from_drawing_object(obj);
            design.shapes.push(shape_data);
        }

        match design.save_to_file(&path) {
            Ok(_) => {
                self.set_status(&format!("{} {}", t!("Saved:"), path.display()));
            }
            Err(e) => {
                error!("Error saving file: {}", e);
                self.set_status(&format!("{} {}", t!("Error saving file:"), e));
            }
        }
    }

    pub fn export_gcode(&self) {
        let window = self
            .widget
            .root()
            .and_then(|w| w.downcast::<gtk4::Window>().ok());
        let dialog = FileChooserNative::new(
            Some("Export G-Code"),
            window.as_ref(),
            FileChooserAction::Save,
            Some("Export"),
            Some("Cancel"),
        );

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("G-Code Files"));
        filter.add_pattern("*.nc");
        filter.add_pattern("*.gcode");
        filter.add_pattern("*.gc");
        dialog.add_filter(&filter);

        let canvas = self.canvas.clone();
        let status_label = self.status_label.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(mut path) = file.path() {
                        if path.extension().is_none() {
                            path.set_extension("nc");
                        }

                        // Generate G-code
                        let mut state = canvas.state.borrow_mut();

                        // Copy settings to avoid borrow issues
                        let feed_rate = state.tool_settings.feed_rate;
                        let spindle_speed = state.tool_settings.spindle_speed;
                        let tool_diameter = state.tool_settings.tool_diameter;
                        let cut_depth = state.tool_settings.cut_depth;
                        let start_depth = state.tool_settings.start_depth;

                        // Update toolpath generator settings from state
                        state.toolpath_generator.set_feed_rate(feed_rate);
                        state.toolpath_generator.set_spindle_speed(spindle_speed);
                        state.toolpath_generator.set_tool_diameter(tool_diameter);
                        state.toolpath_generator.set_cut_depth(cut_depth);
                        state.toolpath_generator.set_start_depth(start_depth);
                        state.toolpath_generator.set_step_in(tool_diameter * 0.4); // Default stepover

                        let gcode = state.generate_gcode();

                        match std::fs::write(&path, gcode) {
                            Ok(_) => {
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Exported G-Code:"),
                                    path.display()
                                ));
                            }
                            Err(e) => {
                                error!("Error exporting G-Code: {}", e);
                                status_label.set_text(&format!(
                                    "{} {}",
                                    t!("Error exporting G-Code:"),
                                    e
                                ));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    pub fn export_svg(&self) {
        let window = self
            .widget
            .root()
            .and_then(|w| w.downcast::<gtk4::Window>().ok());
        let dialog = FileChooserNative::new(
            Some("Export SVG"),
            window.as_ref(),
            FileChooserAction::Save,
            Some("Export"),
            Some("Cancel"),
        );

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("SVG Files"));
        filter.add_pattern("*.svg");
        dialog.add_filter(&filter);

        let canvas = self.canvas.clone();
        let status_label = self.status_label.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(mut path) = file.path() {
                        if path.extension().is_none() {
                            path.set_extension("svg");
                        }

                        let state = canvas.state.borrow();

                        // Calculate bounds
                        let mut min_x = f64::INFINITY;
                        let mut min_y = f64::INFINITY;
                        let mut max_x = f64::NEG_INFINITY;
                        let mut max_y = f64::NEG_INFINITY;

                        let shapes: Vec<_> = state.canvas.shapes().collect();
                        if shapes.is_empty() {
                            status_label.set_text(&t!("Nothing to export"));
                            dialog.destroy();
                            return;
                        }

                        for obj in &shapes {
                            let (x1, y1, x2, y2) = obj.get_effective_shape().bounds();
                            min_x = min_x.min(x1);
                            min_y = min_y.min(y1);
                            max_x = max_x.max(x2);
                            max_y = max_y.max(y2);
                        }

                        // Add some padding
                        let padding = 10.0;
                        min_x -= padding;
                        min_y -= padding;
                        max_x += padding;
                        max_y += padding;

                        let width = max_x - min_x;
                        let height = max_y - min_y;

                        let mut svg = String::new();
                        svg.push_str(&format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<svg width="{:.2}mm" height="{:.2}mm" viewBox="{:.2} {:.2} {:.2} {:.2}" xmlns="http://www.w3.org/2000/svg">
"#, width, height, min_x, min_y, width, height));

                        for obj in &shapes {
                            let style = "fill:none;stroke:black;stroke-width:0.5";
                            let effective_shape = obj.get_effective_shape();
                            match &effective_shape {
                                Shape::Rectangle(r) => {
                                    let x = r.center.x - r.width / 2.0;
                                    let y = r.center.y - r.height / 2.0;
                                    let effective_radius = r.effective_corner_radius();
                                    svg.push_str(&format!(r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        x, y, r.width, r.height, effective_radius, style,
                                        r.rotation, r.center.x, r.center.y
                                    ));
                                }
                                Shape::Circle(c) => {
                                    svg.push_str(&format!(r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" style="{}" />"#,
                                        c.center.x, c.center.y, c.radius, style
                                    ));
                                }
                                Shape::Line(l) => {
                                    svg.push_str(&format!(r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        l.start.x, l.start.y, l.end.x, l.end.y, style,
                                        l.rotation, (l.start.x+l.end.x)/2.0, (l.start.y+l.end.y)/2.0
                                    ));
                                }
                                Shape::Triangle(t) => {
                                    let path = t.render();
                                    let d = gcodekit5_designer::model::DesignPath::from_lyon_path(&path).to_svg_path();
                                    svg.push_str(&format!(r#"<path d="{}" style="{}" />"#, d, style));
                                }
                                Shape::Polygon(p) => {
                                    let path = p.render();
                                    let d = gcodekit5_designer::model::DesignPath::from_lyon_path(&path).to_svg_path();
                                    svg.push_str(&format!(r#"<path d="{}" style="{}" />"#, d, style));
                                }
                                Shape::Ellipse(e) => {
                                    svg.push_str(&format!(r#"<ellipse cx="{:.2}" cy="{:.2}" rx="{:.2}" ry="{:.2}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        e.center.x, e.center.y, e.rx, e.ry, style,
                                        e.rotation, e.center.x, e.center.y
                                    ));
                                }
                                Shape::Path(p) => {
                                    let mut d = String::new();
                                    let path = p.render();
                                    for event in path.iter() {
                                        match event {
                                            lyon::path::Event::Begin { at } => d.push_str(&format!("M {:.2} {:.2} ", at.x, at.y)),
                                            lyon::path::Event::Line { from: _, to } => d.push_str(&format!("L {:.2} {:.2} ", to.x, to.y)),
                                            lyon::path::Event::Quadratic { from: _, ctrl, to } => d.push_str(&format!("Q {:.2} {:.2} {:.2} {:.2} ", ctrl.x, ctrl.y, to.x, to.y)),
                                            lyon::path::Event::Cubic { from: _, ctrl1, ctrl2, to } => d.push_str(&format!("C {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} ", ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y)),
                                            lyon::path::Event::End { last: _, first: _, close } => if close { d.push_str("Z "); },
                                        }
                                    }
                                    let rect = lyon::algorithms::aabb::bounding_box(&path);
                                    let cx = (rect.min.x + rect.max.x) / 2.0;
                                    let cy = (rect.min.y + rect.max.y) / 2.0;

                                    svg.push_str(&format!(r#"<path d="{}" style="{}" transform="rotate({:.2} {:.2} {:.2})" />"#,
                                        d, style, p.rotation, cx, cy
                                    ));
                                }
                                Shape::Text(t) => {
                                    svg.push_str(&format!(r#"<text x="{:.2}" y="{:.2}" font-size="{:.2}" style="fill:black;stroke:none" transform="rotate({:.2} {:.2} {:.2})">{}</text>"#,
                                        t.x, t.y, t.font_size,
                                        t.rotation, t.x, t.y,
                                        t.text
                                    ));
                                }
                                Shape::Gear(g) => {
                                    let path = g.render();
                                    let d = gcodekit5_designer::model::DesignPath::from_lyon_path(&path).to_svg_path();
                                    svg.push_str(&format!(r#"<path d="{}" style="{}" />"#, d, style));
                                }
                                Shape::Sprocket(s) => {
                                    let path = s.render();
                                    let d = gcodekit5_designer::model::DesignPath::from_lyon_path(&path).to_svg_path();
                                    svg.push_str(&format!(r#"<path d="{}" style="{}" />"#, d, style));
                                }
                            }
                            svg.push('\n');
                        }

                        svg.push_str("</svg>");

                        match std::fs::write(&path, svg) {
                            Ok(_) => {
                                status_label.set_text(&format!("{} {}", t!("Exported SVG:"), path.display()));
                            }
                            Err(e) => {
                                error!("Error exporting SVG: {}", e);
                                status_label.set_text(&format!("{} {}", t!("Error exporting SVG:"), e));
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    // TODO(#17): File operations - Implement once shape structures are aligned
    // Phase 8 infrastructure is in place but needs shape struct updates

    pub fn add_shape(&self, shape: gcodekit5_designer::model::Shape) {
        let mut state = self.canvas.state.borrow_mut();
        state.add_shape_with_undo(shape);
        drop(state);
        self.layers.refresh(&self.canvas.state);
        self.canvas.widget.queue_draw();
    }
}
