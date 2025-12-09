use crate::app::designer::update_designer_ui;
use crate::app::helpers::snap_to_mm;
use crate::MainWindow;
use gcodekit5::{DesignerState, SettingsPersistence};
use gcodekit5_core::units::{MeasurementSystem, to_display_string, parse_from_string, get_unit_label};
use gcodekit5_ui::EditorBridge;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

pub fn setup_designer_callbacks(
    main_window: &MainWindow,
    designer_mgr: Rc<RefCell<DesignerState>>,
    editor_bridge: Rc<EditorBridge>,
    shift_pressed: Rc<RefCell<bool>>,
    settings_persistence: Rc<RefCell<SettingsPersistence>>,
) {
    // Designer: Set Mode callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let settings_persistence_mode = settings_persistence.clone();
    main_window.on_designer_set_mode(move |mode| {
        let mut state = designer_mgr_clone.borrow_mut();
        state.set_mode(mode);
        if let Some(window) = window_weak.upgrade() {
            // Update unit label
            let system = {
                let persistence = settings_persistence_mode.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };
            window.set_designer_unit_label(get_unit_label(system).into());
            
            update_designer_ui(&window, &mut state);
            // Create UI state struct from Rust state
            let ui_state = crate::DesignerState {
                mode: state.canvas.mode() as i32,
                zoom: state.canvas.zoom() as f32,
                pan_x: state.canvas.pan_offset().0 as f32,
                pan_y: state.canvas.pan_offset().1 as f32,
                selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                update_counter: 0,
                can_undo: state.can_undo(),
                can_redo: state.can_redo(),
                can_group: state.can_group(),
                can_ungroup: state.can_ungroup(),
            };
            window.set_designer_state(ui_state);
        }
    });

    // Designer: Zoom In callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_zoom_in(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.zoom_in();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            // Create UI state struct from Rust state
            let ui_state = crate::DesignerState {
                mode: state.canvas.mode() as i32,
                zoom: state.canvas.zoom() as f32,
                pan_x: state.canvas.pan_offset().0 as f32,
                pan_y: state.canvas.pan_offset().1 as f32,
                selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                update_counter: 0,
                can_undo: state.can_undo(),
                can_redo: state.can_redo(),
                can_group: state.can_group(),
                can_ungroup: state.can_ungroup(),
            };
            window.set_designer_state(ui_state);
        }
    });

    // Designer: Zoom Out callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_zoom_out(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.zoom_out();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            let ui_state = crate::DesignerState {
                mode: state.canvas.mode() as i32,
                zoom: state.canvas.zoom() as f32,
                pan_x: state.canvas.pan_offset().0 as f32,
                pan_y: state.canvas.pan_offset().1 as f32,
                selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                update_counter: 0,
                can_undo: state.can_undo(),
                can_redo: state.can_redo(),
                can_group: state.can_group(),
                can_ungroup: state.can_ungroup(),
            };
            window.set_designer_state(ui_state);
        }
    });

    // Designer: Zoom Fit callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_zoom_fit(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.zoom_fit();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            let ui_state = crate::DesignerState {
                mode: state.canvas.mode() as i32,
                zoom: state.canvas.zoom() as f32,
                pan_x: state.canvas.pan_offset().0 as f32,
                pan_y: state.canvas.pan_offset().1 as f32,
                selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                update_counter: 0,
                can_undo: state.can_undo(),
                can_redo: state.can_redo(),
                can_group: state.can_group(),
                can_ungroup: state.can_ungroup(),
            };
            window.set_designer_state(ui_state);
        }
    });

    // Designer: Reset View callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_reset_view(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.reset_view();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            let ui_state = crate::DesignerState {
                mode: state.canvas.mode() as i32,
                zoom: state.canvas.zoom() as f32,
                pan_x: state.canvas.pan_offset().0 as f32,
                pan_y: state.canvas.pan_offset().1 as f32,
                selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                update_counter: 0,
                can_undo: state.can_undo(),
                can_redo: state.can_redo(),
                can_group: state.can_group(),
                can_ungroup: state.can_ungroup(),
            };
            window.set_designer_state(ui_state);
        }
    });

    // Designer: Delete Selected callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_delete_selected(move || {
        let count = {
            let state = designer_mgr_clone.borrow();
            state.selected_count()
        };

        if count > 1 {
            // Show confirmation dialog
            if let Some(window) = window_weak.upgrade() {
                window.invoke_show_delete_confirmation(count as i32);
            }
        } else if count == 1 {
            // Just delete
            let mut state = designer_mgr_clone.borrow_mut();
            state.delete_selected();
            if let Some(window) = window_weak.upgrade() {
                update_designer_ui(&window, &mut state);
                window.set_connection_status(slint::SharedString::from(format!(
                    "Shapes: {}",
                    state.canvas.shape_count()
                )));
            }
        }
    });

    // Designer: Confirm Delete callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_confirm_delete(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.delete_selected();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            window.set_connection_status(slint::SharedString::from(format!(
                "Shapes: {}",
                state.canvas.shape_count()
            )));
        }
    });

    // Designer: Align Horizontal Left callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_align_horizontal_left(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.align_selected_horizontal_left();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Align Horizontal Center callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_align_horizontal_center(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.align_selected_horizontal_center();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Align Horizontal Right callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_align_horizontal_right(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.align_selected_horizontal_right();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Align Vertical Top callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_align_vertical_top(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.align_selected_vertical_top();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Align Vertical Center callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_align_vertical_center(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.align_selected_vertical_center();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Align Vertical Bottom callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_align_vertical_bottom(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.align_selected_vertical_bottom();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Clear Canvas callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_clear_canvas(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.clear_canvas();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            window.set_designer_gcode_generated(false);
            window.set_connection_status(slint::SharedString::from("Canvas cleared"));
        }
    });

    // Designer: G-code generated callback (called from thread via invoke_from_event_loop)
    let window_weak = main_window.as_weak();
    let editor_bridge_designer = editor_bridge.clone();
    main_window.on_invoke_designer_gcode_generated(move |gcode| {
        if let Some(window) = window_weak.upgrade() {
            let gcode_str = gcode.to_string();

            window.set_designer_generated_gcode(gcode.clone());
            window.set_designer_gcode_generated(true);

            // Load into editor and switch view
            editor_bridge_designer.load_text(&gcode_str);
            window.set_total_lines(editor_bridge_designer.line_count() as i32);
            // update_visible_lines(&window, &editor_bridge_designer); // Need to expose this or move it

            window.set_gcode_content(gcode);
            window.set_current_view(slint::SharedString::from("gcode-editor"));
            window.set_gcode_focus_trigger(window.get_gcode_focus_trigger() + 1);

            window.set_connection_status(slint::SharedString::from(
                "G-code generated and loaded into editor",
            ));
            window.set_is_busy(false);
        }
    });

    // Designer: Generate Toolpath callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_generate_toolpath(move || {
        if let Some(window) = window_weak.upgrade() {
            window.set_is_busy(true);

            let designer_mgr_inner = designer_mgr_clone.clone();
            let window_weak_inner = window.as_weak();

            // Clone state to offload to thread
            let mut state_clone = {
                let state = designer_mgr_inner.borrow();
                state.clone()
            };

            std::thread::spawn(move || {
                let gcode = state_clone.generate_gcode();
                let gcode_shared = slint::SharedString::from(gcode);

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = window_weak_inner.upgrade() {
                        window.invoke_invoke_designer_gcode_generated(gcode_shared);
                    }
                });
            });
        }
    });

    // Designer: Import DXF callback
    let window_weak = main_window.as_weak();
    let designer_mgr_clone = designer_mgr.clone();
    let settings_persistence_dxf = settings_persistence.clone();
    main_window.on_designer_import_dxf(move || {
        use gcodekit5::designer::{DxfImporter, DxfParser};
        use rfd::FileDialog;

        let default_dir = {
            let persistence = settings_persistence_dxf.borrow();
            persistence.config().file_processing.output_directory.clone()
        };

        if let Some(window) = window_weak.upgrade() {
            if let Some(path) = crate::platform::pick_file_with_parent(
                FileDialog::new()
                    .set_directory(&default_dir)
                    .add_filter("DXF Files", &["dxf"])
                    .set_title("Import DXF File"),
                window.window()
            )
            {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match DxfParser::parse(&content) {
                        Ok(dxf_file) => {
                            let importer = DxfImporter::new(1.0, 0.0, 0.0);
                            match importer.import_string(&content) {
                                Ok(design) => {
                                    let mut state = designer_mgr_clone.borrow_mut();
                                    state.canvas.clear();
                                    for shape in design.shapes {
                                        state.canvas.add_shape(shape);
                                    }
                                    
                                    // Auto-fit to view
                                    state.zoom_fit();

                                    window.set_connection_status(slint::SharedString::from(
                                        format!(
                                            "DXF imported: {} entities from {} layers",
                                            dxf_file.entity_count(),
                                            dxf_file.layer_names().len()
                                        ),
                                    ));
                                    update_designer_ui(&window, &mut state);

                                    // Update UI state with new zoom/pan values
                                    let ui_state = crate::DesignerState {
                                        mode: state.canvas.mode() as i32,
                                        zoom: state.canvas.zoom() as f32,
                                        pan_x: state.canvas.pan_offset().0 as f32,
                                        pan_y: state.canvas.pan_offset().1 as f32,
                                        selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                                        update_counter: 0, 
                                        can_undo: state.can_undo(), 
                                        can_redo: state.can_redo(), 
                                        can_group: state.can_group(), 
                                        can_ungroup: state.can_ungroup(),
                                    };
                                    window.set_designer_state(ui_state);
                                }
                                Err(e) => {
                                    window.set_connection_status(slint::SharedString::from(
                                        format!("DXF import failed: {}", e),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            window.set_connection_status(slint::SharedString::from(format!(
                                "DXF parse error: {}",
                                e
                            )));
                        }
                    },
                    Err(e) => {
                        window.set_connection_status(slint::SharedString::from(format!(
                            "Failed to read file: {}",
                            e
                        )));
                    }
                }
            }
        }
    });

    // Designer: Import SVG callback
    let window_weak = main_window.as_weak();
    let designer_mgr_clone = designer_mgr.clone();
    let settings_persistence_svg = settings_persistence.clone();
    main_window.on_designer_import_svg(move || {
        use gcodekit5::designer::SvgImporter;
        use rfd::FileDialog;

        let default_dir = {
            let persistence = settings_persistence_svg.borrow();
            persistence.config().file_processing.output_directory.clone()
        };

        if let Some(window) = window_weak.upgrade() {
            if let Some(path) = crate::platform::pick_file_with_parent(
                FileDialog::new()
                    .set_directory(&default_dir)
                    .add_filter("SVG Files", &["svg"])
                    .set_title("Import SVG File"),
                window.window()
            )
            {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let importer = SvgImporter::new(1.0, 0.0, 0.0);
                        match importer.import_string(&content) {
                            Ok(design) => {
                                let shape_count = design.shapes.len();
                                let layer_count = design.layer_count;
                                let mut state = designer_mgr_clone.borrow_mut();
                                state.canvas.clear();
                                for shape in design.shapes {
                                    state.canvas.add_shape(shape);
                                }
                                
                                // Auto-fit to view after import
                                state.zoom_fit();
                                
                                window.set_connection_status(slint::SharedString::from(format!(
                                    "SVG imported: {} shapes from {} layers",
                                    shape_count, layer_count
                                )));
                                update_designer_ui(&window, &mut state);
                                
                                // Update UI state with new zoom/pan values
                                let ui_state = crate::DesignerState {
                                    mode: state.canvas.mode() as i32,
                                    zoom: state.canvas.zoom() as f32,
                                    pan_x: state.canvas.pan_offset().0 as f32,
                                    pan_y: state.canvas.pan_offset().1 as f32,
                                    selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                                    update_counter: 0, 
                                    can_undo: state.can_undo(), 
                                    can_redo: state.can_redo(), 
                                    can_group: state.can_group(), 
                                    can_ungroup: state.can_ungroup(),
                                };
                                window.set_designer_state(ui_state);
                            }
                            Err(e) => {
                                window.set_connection_status(slint::SharedString::from(format!(
                                    "SVG import failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        window.set_connection_status(slint::SharedString::from(format!(
                            "Failed to read file: {}",
                            e
                        )));
                    }
                }
            }
        }
    });

    // Designer: Add DXF callback
    let window_weak = main_window.as_weak();
    let designer_mgr_clone = designer_mgr.clone();
    let settings_persistence_add_dxf = settings_persistence.clone();
    main_window.on_designer_add_dxf(move || {
        use gcodekit5::designer::{DxfImporter, DxfParser};
        use rfd::FileDialog;

        let default_dir = {
            let persistence = settings_persistence_add_dxf.borrow();
            persistence.config().file_processing.output_directory.clone()
        };

        if let Some(window) = window_weak.upgrade() {
            if let Some(path) = crate::platform::pick_file_with_parent(
                FileDialog::new()
                    .set_directory(&default_dir)
                    .add_filter("DXF Files", &["dxf"])
                    .set_title("Add DXF File"),
                window.window()
            )
            {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match DxfParser::parse(&content) {
                        Ok(dxf_file) => {
                            let importer = DxfImporter::new(1.0, 0.0, 0.0);
                            match importer.import_string(&content) {
                                Ok(design) => {
                                    let mut state = designer_mgr_clone.borrow_mut();
                                    let group_id = state.canvas.generate_id();
                                    for shape in design.shapes {
                                        let id = state.canvas.add_shape(shape);
                                        if let Some(obj) = state.canvas.get_shape_mut(id) {
                                            obj.group_id = Some(group_id);
                                        }
                                    }
                                    
                                    // Auto-fit to view
                                    state.zoom_fit();

                                    window.set_connection_status(slint::SharedString::from(
                                        format!(
                                            "DXF added: {} entities from {} layers",
                                            dxf_file.entity_count(),
                                            dxf_file.layer_names().len()
                                        ),
                                    ));
                                    update_designer_ui(&window, &mut state);

                                    // Update UI state with new zoom/pan values
                                    let ui_state = crate::DesignerState {
                                        mode: state.canvas.mode() as i32,
                                        zoom: state.canvas.zoom() as f32,
                                        pan_x: state.canvas.pan_offset().0 as f32,
                                        pan_y: state.canvas.pan_offset().1 as f32,
                                        selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                                        update_counter: 0, 
                                        can_undo: state.can_undo(), 
                                        can_redo: state.can_redo(), 
                                        can_group: state.can_group(), 
                                        can_ungroup: state.can_ungroup(),
                                    };
                                    window.set_designer_state(ui_state);
                                }
                                Err(e) => {
                                    window.set_connection_status(slint::SharedString::from(
                                        format!("DXF import failed: {}", e),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            window.set_connection_status(slint::SharedString::from(format!(
                                "DXF parse error: {}",
                                e
                            )));
                        }
                    },
                    Err(e) => {
                        window.set_connection_status(slint::SharedString::from(format!(
                            "Failed to read file: {}",
                            e
                        )));
                    }
                }
            }
        }
    });

    // Designer: Add SVG callback
    let window_weak = main_window.as_weak();
    let designer_mgr_clone = designer_mgr.clone();
    let settings_persistence_add_svg = settings_persistence.clone();
    main_window.on_designer_add_svg(move || {
        use gcodekit5::designer::SvgImporter;
        use rfd::FileDialog;

        let default_dir = {
            let persistence = settings_persistence_add_svg.borrow();
            persistence.config().file_processing.output_directory.clone()
        };

        if let Some(window) = window_weak.upgrade() {
            if let Some(path) = crate::platform::pick_file_with_parent(
                FileDialog::new()
                    .set_directory(&default_dir)
                    .add_filter("SVG Files", &["svg"])
                    .set_title("Add SVG File"),
                window.window()
            )
            {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let importer = SvgImporter::new(1.0, 0.0, 0.0);
                        match importer.import_string(&content) {
                            Ok(design) => {
                                let shape_count = design.shapes.len();
                                let layer_count = design.layer_count;
                                let mut state = designer_mgr_clone.borrow_mut();
                                let group_id = state.canvas.generate_id();
                                for shape in design.shapes {
                                    let id = state.canvas.add_shape(shape);
                                    if let Some(obj) = state.canvas.get_shape_mut(id) {
                                        obj.group_id = Some(group_id);
                                    }
                                }
                                
                                // Auto-fit to view
                                state.zoom_fit();
                                
                                window.set_connection_status(slint::SharedString::from(format!(
                                    "SVG added: {} shapes from {} layers",
                                    shape_count, layer_count
                                )));
                                update_designer_ui(&window, &mut state);
                                
                                // Update UI state with new zoom/pan values
                                let ui_state = crate::DesignerState {
                                    mode: state.canvas.mode() as i32,
                                    zoom: state.canvas.zoom() as f32,
                                    pan_x: state.canvas.pan_offset().0 as f32,
                                    pan_y: state.canvas.pan_offset().1 as f32,
                                    selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                                    update_counter: 0, 
                                    can_undo: state.can_undo(), 
                                    can_redo: state.can_redo(), 
                                    can_group: state.can_group(), 
                                    can_ungroup: state.can_ungroup(),
                                };
                                window.set_designer_state(ui_state);
                            }
                            Err(e) => {
                                window.set_connection_status(slint::SharedString::from(format!(
                                    "SVG import failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        window.set_connection_status(slint::SharedString::from(format!(
                            "Failed to read file: {}",
                            e
                        )));
                    }
                }
            }
        }
    });

    // Designer: Export Design callback
    let window_weak = main_window.as_weak();
    main_window.on_designer_export_design(move || {
        if let Some(window) = window_weak.upgrade() {
            window.set_connection_status(slint::SharedString::from("Design export: Ready to save"));
        }
    });

    // Designer: File New callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_file_new(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        
        if state.canvas.shape_count() > 0 {
            let should_clear = rfd::MessageDialog::new()
                .set_title("Clear Design?")
                .set_description("The design contains shapes. Are you sure you want to clear it?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .set_level(rfd::MessageLevel::Warning)
                .show();
            
            if should_clear != rfd::MessageDialogResult::Yes {
                return;
            }
        }
        
        state.new_design();

        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            window.set_connection_status(slint::SharedString::from("New design created"));
        }
    });

    // Designer: File Open callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let settings_persistence_open = settings_persistence.clone();
    main_window.on_designer_file_open(move || {
        let default_dir = {
            let persistence = settings_persistence_open.borrow();
            persistence.config().file_processing.output_directory.clone()
        };

        if let Some(window) = window_weak.upgrade() {
            if let Some(path) = crate::platform::pick_file_with_parent(
                rfd::FileDialog::new()
                    .set_directory(&default_dir)
                    .add_filter("GCodeKit4 Design", &["gck4", "json"]),
                window.window()
            )
            {
                let mut state = designer_mgr_clone.borrow_mut();
                match state.load_from_file(&path) {
                    Ok(()) => {
                        update_designer_ui(&window, &mut state);
                        window.set_connection_status(slint::SharedString::from(format!(
                            "Opened: {}",
                            path.display()
                        )));
                    }
                    Err(e) => {
                        window.set_connection_status(slint::SharedString::from(format!(
                            "Error opening file: {}",
                            e
                        )));
                    }
                }
            }
        }
    });

    // Designer: Edit Default Properties callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let settings_persistence_edit = settings_persistence.clone();
    main_window.on_designer_edit_default_properties(move || {
        let state = designer_mgr_clone.borrow();
        if let Some(window) = window_weak.upgrade() {
            let system = {
                let persistence = settings_persistence_edit.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };
            
            let defaults = &state.default_properties_shape;
            
            window.set_designer_selected_shape_is_pocket(defaults.operation_type == gcodekit5::designer::shapes::OperationType::Pocket);
            window.set_designer_selected_shape_pocket_depth(to_display_string(defaults.pocket_depth as f32, system).into());
            window.set_designer_selected_shape_step_down(to_display_string(defaults.step_down as f32, system).into());
            window.set_designer_selected_shape_step_in(to_display_string(defaults.step_in as f32, system).into());
            
            match defaults.pocket_strategy {
                gcodekit5::designer::pocket_operations::PocketStrategy::Raster { angle, bidirectional } => {
                    window.set_designer_selected_shape_pocket_strategy(0); // Raster
                    window.set_designer_selected_shape_raster_angle(to_display_string(angle as f32, system).into());
                    window.set_designer_selected_shape_bidirectional(bidirectional);
                }
                gcodekit5::designer::pocket_operations::PocketStrategy::ContourParallel => {
                    window.set_designer_selected_shape_pocket_strategy(1); // Contour
                    window.set_designer_selected_shape_raster_angle(to_display_string(0.0, system).into());
                    window.set_designer_selected_shape_bidirectional(true);
                }
                gcodekit5::designer::pocket_operations::PocketStrategy::Adaptive => {
                    window.set_designer_selected_shape_pocket_strategy(2); // Adaptive
                    window.set_designer_selected_shape_raster_angle(to_display_string(0.0, system).into());
                    window.set_designer_selected_shape_bidirectional(true);
                }
            }
            
            window.set_designer_selected_shape_text_content(slint::SharedString::from(""));
            window.set_designer_selected_shape_font_size(to_display_string(12.0, system).into());
            window.set_designer_selected_shape_use_custom_values(defaults.use_custom_values);
            
            window.set_designer_is_editing_defaults(true);
        }
    });

    // Designer: File Save callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let settings_persistence_save = settings_persistence.clone();
    main_window.on_designer_file_save(move || {
        let mut state = designer_mgr_clone.borrow_mut();

        // If no current file, prompt for filename
        let path = if let Some(existing_path) = &state.current_file_path {
            existing_path.clone()
        } else {
            let default_dir = {
                let persistence = settings_persistence_save.borrow();
                persistence.config().file_processing.output_directory.clone()
            };

            if let Some(window) = window_weak.upgrade() {
                if let Some(new_path) = crate::platform::save_file_with_parent(
                    rfd::FileDialog::new()
                        .set_directory(&default_dir)
                        .add_filter("GCodeKit4 Design", &["gck4"])
                        .set_file_name("design.gck4"),
                    window.window()
                )
                {
                    new_path
                } else {
                    return; // User cancelled
                }
            } else {
                return;
            }
        };

        match state.save_to_file(&path) {
            Ok(()) => {
                if let Some(window) = window_weak.upgrade() {
                    window.set_connection_status(slint::SharedString::from(format!(
                        "Saved: {}",
                        path.display()
                    )));
                }
            }
            Err(e) => {
                if let Some(window) = window_weak.upgrade() {
                    window.set_connection_status(slint::SharedString::from(format!(
                        "Error saving file: {}",
                        e
                    )));
                }
            }
        }
    });

    // Designer: File Save As callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let settings_persistence_save_as = settings_persistence.clone();
    main_window.on_designer_file_save_as(move || {
        let default_dir = {
            let persistence = settings_persistence_save_as.borrow();
            persistence.config().file_processing.output_directory.clone()
        };

        if let Some(window) = window_weak.upgrade() {
            if let Some(path) = crate::platform::save_file_with_parent(
                rfd::FileDialog::new()
                    .set_directory(&default_dir)
                    .add_filter("GCodeKit4 Design", &["gck4"])
                    .set_file_name("design.gck4"),
                window.window()
            )
            {
                let mut state = designer_mgr_clone.borrow_mut();
                match state.save_to_file(&path) {
                    Ok(()) => {
                        window.set_connection_status(slint::SharedString::from(format!(
                            "Saved as: {}",
                            path.display()
                        )));
                    }
                    Err(e) => {
                        window.set_connection_status(slint::SharedString::from(format!(
                            "Error saving file: {}",
                            e
                        )));
                    }
                }
            }
        }
    });

    // Designer: Canvas Click callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let shift_pressed_clone = shift_pressed.clone();
    main_window.on_designer_canvas_click(move |x: f32, y: f32| {
        let mut state = designer_mgr_clone.borrow_mut();

        // Convert pixel coordinates to world coordinates
        let world_point = state.canvas.pixel_to_world(x as f64, y as f64);

        let multi_select = *shift_pressed_clone.borrow();
        state.add_shape_at(world_point.x, world_point.y, multi_select);

        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);

            // Update UI state with selected shape ID
            let mut ui_state = window.get_designer_state();
            ui_state.selected_id = state.canvas.selected_id().unwrap_or(0) as i32;
            window.set_designer_state(ui_state);

            window.set_connection_status(slint::SharedString::from(format!(
                "Shapes: {}",
                state.canvas.shape_count()
            )));
        }
    });

    // Designer: Select in Rect callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_select_in_rect(move |x1: f32, y1: f32, x2: f32, y2: f32, multi: bool| {
        let mut state = designer_mgr_clone.borrow_mut();
        
        // Convert pixel coordinates to world coordinates
        let p1 = state.canvas.pixel_to_world(x1 as f64, y1 as f64);
        let p2 = state.canvas.pixel_to_world(x2 as f64, y2 as f64);
        
        // Calculate world bounds
        let min_x = p1.x.min(p2.x);
        let min_y = p1.y.min(p2.y);
        let width = (p1.x - p2.x).abs();
        let height = (p1.y - p2.y).abs();
        
        state.select_in_rect(min_x, min_y, width, height, multi);
        
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            
            // Update UI state with selected shape ID
            let mut ui_state = window.get_designer_state();
            ui_state.selected_id = state.canvas.selected_id().unwrap_or(0) as i32;
            window.set_designer_state(ui_state);
        }
    });

    // Designer: Shape drag callback (move selected shape)
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_shape_drag(move |_shape_id: i32, dx: f32, dy: f32| {
        let mut state = designer_mgr_clone.borrow_mut();

        // Convert pixel delta to world delta using viewport zoom
        // At zoom level Z, moving 1 pixel is equivalent to 1/Z world units
        // Note: Y-axis is flipped - positive pixel dy (down) = negative world dy
        let viewport = state.canvas.viewport();
        let world_dx = dx as f64 / viewport.zoom();
        let world_dy = -(dy as f64) / viewport.zoom(); // Flip Y direction

        state.move_selected(world_dx, world_dy);

        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Detect handle callback (check if click is on a resize handle)
    // Returns handle index (-1 if not on a handle): 0=TL, 1=TR, 2=BL, 3=BR, 4=Center
    let designer_mgr_clone = designer_mgr.clone();
    main_window.on_designer_detect_handle(move |x: f32, y: f32| -> i32 {
        let state = designer_mgr_clone.borrow();
        let mut dragging_handle = -1;

        // Convert pixel coordinates to world coordinates
        let world_point = state.canvas.pixel_to_world(x as f64, y as f64);

        // Calculate composite bounding box of all selected shapes
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_selected = false;

        for obj in state.canvas.shapes() {
            if obj.selected {
                has_selected = true;
                let (x1, y1, x2, y2) = obj.shape.bounding_box();
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
            }
        }

        if has_selected {
            // Handle size in world coordinates (scaled by zoom)
            let viewport = state.canvas.viewport();
            let handle_size = 12.0 / viewport.zoom(); // Increased from 8.0 for easier cursor positioning
            let center_handle_size = handle_size * 1.25;
            
            let cx = (min_x + max_x) / 2.0;
            let cy = (min_y + max_y) / 2.0;

            // PRIORITY 1: Check resize handles (0-4) around composite box
            // We check this first so handles are accessible even if inside the selection bounding box
            if (world_point.x - min_x).abs() < handle_size && (world_point.y - min_y).abs() < handle_size {
                dragging_handle = 0; // TL
            } else if (world_point.x - max_x).abs() < handle_size && (world_point.y - min_y).abs() < handle_size {
                dragging_handle = 1; // TR
            } else if (world_point.x - min_x).abs() < handle_size && (world_point.y - max_y).abs() < handle_size {
                dragging_handle = 2; // BL
            } else if (world_point.x - max_x).abs() < handle_size && (world_point.y - max_y).abs() < handle_size {
                dragging_handle = 3; // BR
            } else if (world_point.x - cx).abs() < center_handle_size && (world_point.y - cy).abs() < center_handle_size {
                dragging_handle = 4; // Center
            }

            // PRIORITY 2: Check if inside the composite bounding box of the selection
            // This allows moving multiple selected shapes (or groups) by dragging anywhere in their combined bounds
            if dragging_handle == -1 {
                if world_point.x >= min_x && world_point.x <= max_x && world_point.y >= min_y && world_point.y <= max_y {
                    dragging_handle = 5; // Body (move)
                }
            }
        }

        dragging_handle
    });

    // Designer: Handle drag callback (move or resize via handles)
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let shift_pressed_clone = shift_pressed.clone();
    main_window.on_designer_handle_drag(move |_shape_id: i32, handle: i32, dx: f32, dy: f32| {
        let mut state = designer_mgr_clone.borrow_mut();

        // Convert pixel delta to world delta using viewport zoom
        // Note: Y-axis is flipped - positive pixel dy (down) = negative world dy
        let viewport = state.canvas.viewport();
        let mut world_dx = dx as f64 / viewport.zoom();
        let mut world_dy = -(dy as f64) / viewport.zoom(); // Flip Y direction

        // If Shift is pressed and this is a MOVE (not resize), snap deltas to whole mm
        if *shift_pressed_clone.borrow() && (handle == -1 || handle == 4 || handle == 5) {
            world_dx = snap_to_mm(world_dx);
            world_dy = snap_to_mm(world_dy);
        }

        if handle == -1 || handle == 4 || handle == 5 {
            // handle=-1 or handle=4 (center handle) or handle=5 (body) means move the entire shape
            state.move_selected(world_dx, world_dy);

            // For moves, also snap the final position to whole mm if Shift is pressed
            if *shift_pressed_clone.borrow() {
                state.snap_selected_to_mm();
            }
        } else {
            // Resize via specific handle (0=top-left, 1=top-right, 2=bottom-left, 3=bottom-right)
            state.resize_selected(handle as usize, world_dx, world_dy);

            // For resizes, also snap the final dimensions to whole mm if Shift is pressed
            if *shift_pressed_clone.borrow() {
                state.snap_selected_to_mm();
            }
        }

        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Deselect all callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_deselect_all(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.deselect_all();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);

            // Update UI state with no selected shape
            let mut ui_state = window.get_designer_state();
            ui_state.selected_id = 0;
            window.set_designer_state(ui_state);
        }
    });

    // Designer: Select All callback
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_select_all(move || {
        let mut state = designer_mgr_clone.borrow_mut();
        state.select_all();
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            
            // Update UI state
            let mut ui_state = window.get_designer_state();
            ui_state.selected_id = state.canvas.selected_id().unwrap_or(0) as i32;
            window.set_designer_state(ui_state);
        }
    });

    // Designer: Shift key state callback
    let shift_pressed_clone = shift_pressed.clone();
    main_window.on_designer_set_shift_pressed(move |pressed: bool| {
        *shift_pressed_clone.borrow_mut() = pressed;
    });

    // Designer: Update shape properties immediately
    let designer_mgr_prop = designer_mgr.clone();
    let window_weak_prop = main_window.as_weak();
    let settings_persistence_prop = settings_persistence.clone();
    main_window.on_designer_update_shape_property(move |prop_id: i32, value_str: slint::SharedString| {
        if let Some(window) = window_weak_prop.upgrade() {
            let mut state = designer_mgr_prop.borrow_mut();
            let system = {
                let persistence = settings_persistence_prop.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };
            
            let value = parse_from_string(&value_str, system).unwrap_or(0.0);
            
            // Get current values from state (union of all selected)
            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            let mut has_selected = false;

            for obj in state.canvas.shapes().filter(|s| s.selected) {
                let (x1, y1, x2, y2) = obj.shape.bounding_box();
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
                has_selected = true;
            }

            let (cur_x, cur_y, cur_w, cur_h) = if has_selected {
                (min_x, min_y, (max_x - min_x).abs(), (max_y - min_y).abs())
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

            match prop_id {
                0 => state.set_selected_position_and_size_with_flags(value as f64, cur_y, cur_w, cur_h, true, false),
                1 => state.set_selected_position_and_size_with_flags(cur_x, value as f64, cur_w, cur_h, true, false),
                2 => state.set_selected_position_and_size_with_flags(cur_x, cur_y, value as f64, cur_h, false, true),
                3 => state.set_selected_position_and_size_with_flags(cur_x, cur_y, cur_w, value as f64, false, true),
                4 => state.set_selected_corner_radius(value as f64),
                5 => {
                    let is_pocket = state.canvas.shapes().find(|s| s.selected).map(|s| s.operation_type == gcodekit5::designer::shapes::OperationType::Pocket).unwrap_or(false);
                    state.set_selected_pocket_properties(is_pocket, value as f64);
                },
                6 => {
                    let content = if let Some(id) = state.canvas.selected_id() {
                         if let Some(obj) = state.canvas.get_shape(id) {
                             if let Some(text) = obj.shape.as_any().downcast_ref::<gcodekit5::designer::shapes::TextShape>() {
                                 text.text.clone()
                             } else { "".to_string() }
                         } else { "".to_string() }
                    } else { "".to_string() };
                    state.set_selected_text_properties(content, value as f64);
                },
                7 => state.set_selected_step_down(value as f64),
                8 => state.set_selected_step_in(value as f64),
                9 => {
                    let strategy = match value as i32 {
                        0 => gcodekit5::designer::pocket_operations::PocketStrategy::Raster { angle: 0.0, bidirectional: true },
                        1 => gcodekit5::designer::pocket_operations::PocketStrategy::ContourParallel,
                        2 => gcodekit5::designer::pocket_operations::PocketStrategy::Adaptive,
                        _ => gcodekit5::designer::pocket_operations::PocketStrategy::ContourParallel,
                    };
                    state.set_selected_pocket_strategy(strategy);
                },
                10 => {
                    // Raster Angle - preserve bidirectional if possible
                    let current_bidir = if let Some(id) = state.canvas.selected_id() {
                        if let Some(obj) = state.canvas.get_shape(id) {
                            if let gcodekit5::designer::pocket_operations::PocketStrategy::Raster { bidirectional, .. } = obj.pocket_strategy {
                                bidirectional
                            } else { true }
                        } else { true }
                    } else { true };
                    
                    let strategy = gcodekit5::designer::pocket_operations::PocketStrategy::Raster { 
                        angle: value as f64, 
                        bidirectional: current_bidir 
                    };
                    state.set_selected_pocket_strategy(strategy);
                }
                11 => state.set_selected_rotation(value as f64),
                _ => {}
            }
            update_designer_ui(&window, &mut state);
        }
    });

    let designer_mgr_bool = designer_mgr.clone();
    let window_weak_bool = main_window.as_weak();
    main_window.on_designer_update_shape_property_bool(move |prop_id: i32, value: bool| {
        if let Some(window) = window_weak_bool.upgrade() {
            let mut state = designer_mgr_bool.borrow_mut();
            match prop_id {
                0 => {
                    let depth = state.canvas.shapes().find(|s| s.selected).map(|s| s.pocket_depth).unwrap_or(0.0);
                    state.set_selected_pocket_properties(value, depth);
                },
                1 => {
                    // Bidirectional - preserve angle if possible
                    let current_angle = if let Some(id) = state.canvas.selected_id() {
                        if let Some(obj) = state.canvas.get_shape(id) {
                            if let gcodekit5::designer::pocket_operations::PocketStrategy::Raster { angle, .. } = obj.pocket_strategy {
                                angle
                            } else { 0.0 }
                        } else { 0.0 }
                    } else { 0.0 };
                    
                    let strategy = gcodekit5::designer::pocket_operations::PocketStrategy::Raster { 
                        angle: current_angle, 
                        bidirectional: value 
                    };
                    state.set_selected_pocket_strategy(strategy);
                },
                2 => state.set_selected_use_custom_values(value),
                3 => state.set_selected_is_slot(value),
                _ => {}
            }
            update_designer_ui(&window, &mut state);
        }
    });

    let designer_mgr_string = designer_mgr.clone();
    let window_weak_string = main_window.as_weak();
    main_window.on_designer_update_shape_property_string(move |prop_id: i32, value: slint::SharedString| {
        if let Some(window) = window_weak_string.upgrade() {
            let mut state = designer_mgr_string.borrow_mut();
            match prop_id {
                0 => {
                    let font_size = if let Some(id) = state.canvas.selected_id() {
                         if let Some(obj) = state.canvas.get_shape(id) {
                             if let Some(text) = obj.shape.as_any().downcast_ref::<gcodekit5::designer::shapes::TextShape>() {
                                 text.font_size
                             } else { 12.0 }
                         } else { 12.0 }
                    } else { 12.0 };
                    state.set_selected_text_properties(value.to_string(), font_size);
                },
                1 => {
                    state.set_selected_name(value.to_string());
                },
                _ => {}
            }
            update_designer_ui(&window, &mut state);
        }
    });

    // Removed on_designer_save_shape_properties as updates are now immediate
    main_window.on_designer_save_shape_properties(move || {});

    // Designer: Confirm Convert
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    main_window.on_designer_confirm_convert(move |conversion_type: slint::SharedString| {
        let mut state = designer_mgr_clone.borrow_mut();
        match conversion_type.as_str() {
            "Rectangle" => state.convert_selected_to_rectangle(),
            "Path" => state.convert_selected_to_path(),
            _ => {}
        }
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Canvas pan callback (drag on empty canvas)
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let shift_pressed_clone = shift_pressed.clone();
    main_window.on_designer_canvas_pan(move |dx: f32, dy: f32| {
        let mut state = designer_mgr_clone.borrow_mut();

        // Pan is in pixel space - direct pan offset adjustment
        // Note: Since Y-axis is flipped in world coordinates, pan_y follows screen coordinates
        // Dragging down (positive dy) increases pan_y to show content that was higher up
        // No need to flip Y for panning - pan offsets are in screen space
        let mut pan_dx = dx as f64;
        let mut pan_dy = dy as f64;

        // Apply snapping to whole pixels if Shift is pressed
        if *shift_pressed_clone.borrow() {
            pan_dx = pan_dx.round();
            pan_dy = pan_dy.round();
        }

        state.canvas.pan_by(pan_dx, pan_dy);

        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
            // Update UI state with new pan values
            let ui_state = crate::DesignerState {
                mode: state.canvas.mode() as i32,
                zoom: state.canvas.zoom() as f32,
                pan_x: state.canvas.pan_offset().0 as f32,
                pan_y: state.canvas.pan_offset().1 as f32,
                selected_id: state.canvas.selected_id().unwrap_or(0) as i32,
                update_counter: 0, can_undo: state.can_undo(), can_redo: state.can_redo(), can_group: state.can_group(), can_ungroup: state.can_ungroup(),
            };
            window.set_designer_state(ui_state);
        }
    });

    // Designer: Update feed rate
    let designer_mgr_clone = designer_mgr.clone();
    let settings_persistence_feed = settings_persistence.clone();
    main_window.on_designer_update_feed_rate(move |rate_str: slint::SharedString| {
        let mut state = designer_mgr_clone.borrow_mut();
        let system = {
            let persistence = settings_persistence_feed.borrow();
            let sys_str = &persistence.config().ui.measurement_system;
            MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
        };
        let rate = parse_from_string(&rate_str, system).unwrap_or(120.0);
        state.toolpath_generator.set_feed_rate(rate as f64);
    });

    // Designer: Update spindle speed
    let designer_mgr_clone = designer_mgr.clone();
    main_window.on_designer_update_spindle_speed(move |speed_str: slint::SharedString| {
        let mut state = designer_mgr_clone.borrow_mut();
        let speed = speed_str.parse::<f64>().unwrap_or(3000.0);
        state.toolpath_generator.set_spindle_speed(speed as u32);
    });

    // Designer: Update tool diameter
    let designer_mgr_clone = designer_mgr.clone();
    let settings_persistence_tool = settings_persistence.clone();
    main_window.on_designer_update_tool_diameter(move |diameter_str: slint::SharedString| {
        let mut state = designer_mgr_clone.borrow_mut();
        let system = {
            let persistence = settings_persistence_tool.borrow();
            let sys_str = &persistence.config().ui.measurement_system;
            MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
        };
        let diameter = parse_from_string(&diameter_str, system).unwrap_or(3.175);
        state.toolpath_generator.set_tool_diameter(diameter as f64);
    });

    // Designer: Update cut depth
    let designer_mgr_clone = designer_mgr.clone();
    let settings_persistence_depth = settings_persistence.clone();
    main_window.on_designer_update_cut_depth(move |depth_str: slint::SharedString| {
        let mut state = designer_mgr_clone.borrow_mut();
        let system = {
            let persistence = settings_persistence_depth.borrow();
            let sys_str = &persistence.config().ui.measurement_system;
            MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
        };
        let depth = parse_from_string(&depth_str, system).unwrap_or(-5.0);
        state.toolpath_generator.set_cut_depth(depth as f64);
    });

    // Designer: Update step in
    let designer_mgr_clone = designer_mgr.clone();
    let settings_persistence_step = settings_persistence.clone();
    main_window.on_designer_update_step_in(move |step_in_str: slint::SharedString| {
        let mut state = designer_mgr_clone.borrow_mut();
        let system = {
            let persistence = settings_persistence_step.borrow();
            let sys_str = &persistence.config().ui.measurement_system;
            MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
        };
        let step_in = parse_from_string(&step_in_str, system).unwrap_or(1.0);
        state.toolpath_generator.set_step_in(step_in as f64);
    });

    // Handle designer toggle grid
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_toggle_grid(move || {
            if let Some(window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.toggle_grid();
                update_designer_ui(&window, &mut state);
            }
        });
    }

    // Handle designer group selected
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_group_selected(move || {
            if let Some(window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.canvas.group_selected();
                update_designer_ui(&window, &mut state);
            }
        });
    }

    // Handle designer ungroup selected
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_ungroup_selected(move || {
            if let Some(window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.canvas.ungroup_selected();
                update_designer_ui(&window, &mut state);
            }
        });
    }

    // Handle designer interaction start (save history)
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_interaction_start(move || {
            if let Some(_window) = window_weak.upgrade() {
                let _state = designer_mgr.borrow_mut();
                // History is automatically saved by command pattern
            }
        });
    }

    // Handle designer undo
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_undo(move || {
            if let Some(window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.undo();
                update_designer_ui(&window, &mut state);
            }
        });
    }

    // Handle designer redo
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_redo(move || {
            if let Some(window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.redo();
                update_designer_ui(&window, &mut state);
            }
        });
    }

    // Handle designer copy selected
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_copy_selected(move || {
            if let Some(_window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.copy_selected();
            }
        });
    }

    // Handle designer paste at location
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_paste_at_location(move |x, y| {
            if let Some(window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.paste_at_location(x as f64, y as f64);
                update_designer_ui(&window, &mut state);
            }
        });
    }

    // Handle designer select shape
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_select_shape(move |id, multi| {
            if let Some(window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.canvas.select_shape(id as u64, multi);
                update_designer_ui(&window, &mut state);
            }
        });
    }

    // Handle designer select next shape
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_select_next_shape(move || {
            if let Some(window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.select_next_shape();
                update_designer_ui(&window, &mut state);
            }
        });
    }

    // Handle designer select previous shape
    {
        let designer_mgr = designer_mgr.clone();
        let window_weak = main_window.as_weak();
        main_window.on_designer_select_previous_shape(move || {
            if let Some(window) = window_weak.upgrade() {
                let mut state = designer_mgr.borrow_mut();
                state.select_previous_shape();
                update_designer_ui(&window, &mut state);
            }
        });
    }

    // Designer: Create Linear Array
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let settings_persistence_linear = settings_persistence.clone();
    main_window.on_designer_create_linear_array(move |count_x, count_y, spacing_x_str, spacing_y_str| {
        let mut state = designer_mgr_clone.borrow_mut();
        let system = {
            let persistence = settings_persistence_linear.borrow();
            let sys_str = &persistence.config().ui.measurement_system;
            MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
        };
        let spacing_x = parse_from_string(&spacing_x_str, system).unwrap_or(10.0);
        let spacing_y = parse_from_string(&spacing_y_str, system).unwrap_or(10.0);
        
        let params = gcodekit5_designer::LinearArrayParams::new(
            count_x as u32,
            count_y as u32,
            spacing_x as f64,
            spacing_y as f64,
        );
        state.create_array(gcodekit5_designer::ArrayOperation::Linear(params));
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Create Circular Array
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let settings_persistence_circular = settings_persistence.clone();
    main_window.on_designer_create_circular_array(move |count, center_x_str, center_y_str, radius_str, start_angle_str, clockwise| {
        let mut state = designer_mgr_clone.borrow_mut();
        let system = {
            let persistence = settings_persistence_circular.borrow();
            let sys_str = &persistence.config().ui.measurement_system;
            MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
        };
        let center_x = parse_from_string(&center_x_str, system).unwrap_or(0.0);
        let center_y = parse_from_string(&center_y_str, system).unwrap_or(0.0);
        let radius = parse_from_string(&radius_str, system).unwrap_or(50.0);
        let start_angle = parse_from_string(&start_angle_str, system).unwrap_or(0.0);
        
        let params = gcodekit5_designer::CircularArrayParams::new(
            count as u32,
            gcodekit5_designer::Point::new(center_x as f64, center_y as f64),
            radius as f64,
            start_angle as f64,
            clockwise,
        );
        state.create_array(gcodekit5_designer::ArrayOperation::Circular(params));
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });

    // Designer: Create Grid Array
    let designer_mgr_clone = designer_mgr.clone();
    let window_weak = main_window.as_weak();
    let settings_persistence_grid = settings_persistence.clone();
    main_window.on_designer_create_grid_array(move |columns, rows, col_spacing_str, row_spacing_str| {
        let mut state = designer_mgr_clone.borrow_mut();
        let system = {
            let persistence = settings_persistence_grid.borrow();
            let sys_str = &persistence.config().ui.measurement_system;
            MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
        };
        let col_spacing = parse_from_string(&col_spacing_str, system).unwrap_or(10.0);
        let row_spacing = parse_from_string(&row_spacing_str, system).unwrap_or(10.0);
        
        let params = gcodekit5_designer::GridArrayParams::new(
            columns as u32,
            rows as u32,
            col_spacing as f64,
            row_spacing as f64,
        );
        state.create_array(gcodekit5_designer::ArrayOperation::Grid(params));
        if let Some(window) = window_weak.upgrade() {
            update_designer_ui(&window, &mut state);
        }
    });
}
