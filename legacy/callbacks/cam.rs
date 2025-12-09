use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use slint::{ComponentHandle, VecModel, Weak};
use crate::slint_generatedMainWindow::{
    MainWindow, TabbedBoxDialog, JigsawPuzzleDialog, SpoilboardSurfacingDialog,
    LaserEngraverDialog, VectorEngraverDialog, ErrorDialog, SuccessDialog,
    SpoilboardGridDialog,
};
use gcodekit5_ui::GcodeEditor;
use gcodekit5::{DeviceConsoleManager as ConsoleManager, DeviceMessageType};
use gcodekit5_ui::EditorBridge;
use crate::app::types::{BitmapEngravingParams, VectorEngravingParams, TabbedBoxParams, JigsawPuzzleParams};
use gcodekit5::{
    TabbedBoxMaker, JigsawPuzzleMaker, SpoilboardSurfacingGenerator,
    BoxParameters, BoxType, FingerJointSettings, FingerStyle,
    PuzzleParameters, SpoilboardSurfacingParameters,
    SpeedsFeedsCalculator, SettingsPersistence,
    SpoilboardGridGenerator, SpoilboardGridParameters,
};
use gcodekit5::ui::{MaterialsManagerBackend, ToolsManagerBackend};
use gcodekit5_devicedb::DeviceManager;
use gcodekit5_core::units::{MeasurementSystem, to_display_string, parse_from_string, get_unit_label};
use gcodekit5_core::data::tools::{ToolType, Tool};
use tracing::warn;
use std::str::FromStr;

pub fn register_callbacks(
    main_window: &MainWindow,
    _gcode_editor: Rc<GcodeEditor>,
    console_manager: Arc<ConsoleManager>,
    editor_bridge: Rc<EditorBridge>,
    materials_backend: Rc<RefCell<MaterialsManagerBackend>>,
    tools_backend: Rc<RefCell<ToolsManagerBackend>>,
    device_manager: Arc<DeviceManager>,
    settings_persistence: Rc<RefCell<SettingsPersistence>>,
) {
    // State for singleton dialogs
    let tabbed_box_dialog_weak = Rc::new(RefCell::new(None::<Weak<TabbedBoxDialog>>));
    let jigsaw_puzzle_dialog_weak = Rc::new(RefCell::new(None::<Weak<JigsawPuzzleDialog>>));
    let spoilboard_surfacing_dialog_weak = Rc::new(RefCell::new(None::<Weak<SpoilboardSurfacingDialog>>));
    let spoilboard_grid_dialog_weak = Rc::new(RefCell::new(None::<Weak<SpoilboardGridDialog>>));
    let laser_engraver_dialog_weak = Rc::new(RefCell::new(None::<Weak<LaserEngraverDialog>>));
    let vector_engraver_dialog_weak = Rc::new(RefCell::new(None::<Weak<VectorEngraverDialog>>));

    // Set up generate-tabbed-box callback
    let window_weak = main_window.as_weak();
    let console_manager_clone = console_manager.clone();
    let settings_persistence_tabbed_box = settings_persistence.clone();
    let dialog_weak_ref = tabbed_box_dialog_weak.clone();
    main_window.on_generate_tabbed_box(move || {
        if let Some(window) = window_weak.upgrade() {
            // Check if dialog exists
            if let Some(existing) = dialog_weak_ref.borrow().as_ref().and_then(|w| w.upgrade()) {
                existing.show().unwrap();
                return;
            }

            let dialog = TabbedBoxDialog::new().unwrap();
            *dialog_weak_ref.borrow_mut() = Some(dialog.as_weak());
            let dialog_weak = dialog.as_weak();
            let dialog_weak_generate = dialog_weak.clone();

            // Get measurement system
            let system = {
                let persistence = settings_persistence_tabbed_box.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };

            // Set unit label
            dialog.set_unit_label(get_unit_label(system).into());

            // Initialize dialog with default values
            dialog.set_box_x(to_display_string(100.0, system).into());
            dialog.set_box_y(to_display_string(100.0, system).into());
            dialog.set_box_h(to_display_string(100.0, system).into());
            dialog.set_material_thickness(to_display_string(3.0, system).into());
            dialog.set_burn(to_display_string(0.1, system).into());
            dialog.set_finger_width(to_display_string(2.0, system).into());
            dialog.set_space_width(to_display_string(2.0, system).into());
            dialog.set_surrounding_spaces(to_display_string(2.0, system).into());
            dialog.set_play(to_display_string(0.0, system).into());
            dialog.set_extra_length(to_display_string(0.0, system).into());
            dialog.set_dimple_height(to_display_string(0.0, system).into());
            dialog.set_dimple_length(to_display_string(0.0, system).into());
            dialog.set_feed_rate(to_display_string(500.0, system).into());
            dialog.set_offset_x(to_display_string(10.0, system).into());
            dialog.set_offset_y(to_display_string(10.0, system).into());
            dialog.set_laser_passes("3".into());
            dialog.set_laser_power("1000".into());
            dialog.set_dividers_x("0".into());
            dialog.set_dividers_y("0".into());

            // Set up dialog callbacks
            let console_manager_dialog = console_manager_clone.clone();
            let window_handle = window.as_weak();

            // Load params callback
            let dialog_weak_load_params = dialog.as_weak();
            let settings_persistence_load_params = settings_persistence_tabbed_box.clone();
            dialog.on_load_params(move || {
                if let Some(dlg) = dialog_weak_load_params.upgrade() {
                    let _ = dlg.hide();
                    let _ = dlg.hide();
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_load_params.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::pick_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("JSON Files", &["json"]),
                        dlg.window()
                    )
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(params) = serde_json::from_str::<TabbedBoxParams>(&content) {
                                dlg.set_box_x(params.box_x.into());
                                dlg.set_box_y(params.box_y.into());
                                dlg.set_box_h(params.box_h.into());
                                dlg.set_material_thickness(params.material_thickness.into());
                                dlg.set_burn(params.burn.into());
                                dlg.set_finger_width(params.finger_width.into());
                                dlg.set_space_width(params.space_width.into());
                                dlg.set_surrounding_spaces(params.surrounding_spaces.into());
                                dlg.set_play(params.play.into());
                                dlg.set_extra_length(params.extra_length.into());
                                dlg.set_dimple_height(params.dimple_height.into());
                                dlg.set_dimple_length(params.dimple_length.into());
                                dlg.set_finger_style(params.finger_style);
                                dlg.set_box_type(params.box_type);
                                dlg.set_outside_dimensions(params.outside_dimensions);
                                dlg.set_laser_passes(params.laser_passes.into());
                                dlg.set_laser_power(params.laser_power.into());
                                dlg.set_feed_rate(params.feed_rate.into());
                                dlg.set_offset_x(params.offset_x.into());
                                dlg.set_offset_y(params.offset_y.into());
                                dlg.set_dividers_x(params.dividers_x.into());
                                dlg.set_dividers_y(params.dividers_y.into());
                                dlg.set_optimize_layout(params.optimize_layout);
                                dlg.set_key_divider_type(params.key_divider_type);
                            }
                        }
                    }
                    let _ = dlg.show();
                }
            });

            // Save params callback
            let dialog_weak_save_params = dialog.as_weak();
            let settings_persistence_save_params = settings_persistence_tabbed_box.clone();
            dialog.on_save_params(move || {
                if let Some(dlg) = dialog_weak_save_params.upgrade() {
                    let _ = dlg.hide();
                    let _ = dlg.hide();
                    let _ = dlg.hide();
                    let _ = dlg.hide();
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_save_params.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::save_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("JSON Files", &["json"]),
                        dlg.window()
                    )
                    {
                        let params = TabbedBoxParams {
                            box_x: dlg.get_box_x().to_string(),
                            box_y: dlg.get_box_y().to_string(),
                            box_h: dlg.get_box_h().to_string(),
                            material_thickness: dlg.get_material_thickness().to_string(),
                            burn: dlg.get_burn().to_string(),
                            finger_width: dlg.get_finger_width().to_string(),
                            space_width: dlg.get_space_width().to_string(),
                            surrounding_spaces: dlg.get_surrounding_spaces().to_string(),
                            play: dlg.get_play().to_string(),
                            extra_length: dlg.get_extra_length().to_string(),
                            dimple_height: dlg.get_dimple_height().to_string(),
                            dimple_length: dlg.get_dimple_length().to_string(),
                            finger_style: dlg.get_finger_style(),
                            box_type: dlg.get_box_type(),
                            outside_dimensions: dlg.get_outside_dimensions(),
                            laser_passes: dlg.get_laser_passes().to_string(),
                            laser_power: dlg.get_laser_power().to_string(),
                            feed_rate: dlg.get_feed_rate().to_string(),
                            offset_x: dlg.get_offset_x().to_string(),
                            offset_y: dlg.get_offset_y().to_string(),
                            dividers_x: dlg.get_dividers_x().to_string(),
                            dividers_y: dlg.get_dividers_y().to_string(),
                            optimize_layout: dlg.get_optimize_layout(),
                            key_divider_type: dlg.get_key_divider_type(),
                        };
                        
                        if let Ok(content) = serde_json::to_string_pretty(&params) {
                            let _ = std::fs::write(path, content);
                        }
                    }
                    let _ = dlg.show();
                }
            });

            dialog.on_generate_box(move || {
                if let Some(d) = dialog_weak_generate.upgrade() {
                    // Get parameters from dialog
                    let width = parse_from_string(&d.get_box_x(), system).unwrap_or(100.0);
                    let height = parse_from_string(&d.get_box_y(), system).unwrap_or(100.0);
                    let depth = parse_from_string(&d.get_box_h(), system).unwrap_or(100.0);
                    let thickness = parse_from_string(&d.get_material_thickness(), system).unwrap_or(3.0);
                    let finger_width = parse_from_string(&d.get_finger_width(), system).unwrap_or(2.0);
                    let space_width = parse_from_string(&d.get_space_width(), system).unwrap_or(2.0);
                    let surrounding_spaces = parse_from_string(&d.get_surrounding_spaces(), system).unwrap_or(2.0);
                    let play = parse_from_string(&d.get_play(), system).unwrap_or(0.0);
                    let extra_length = parse_from_string(&d.get_extra_length(), system).unwrap_or(0.0);
                    let dimple_height = parse_from_string(&d.get_dimple_height(), system).unwrap_or(0.0);
                    let dimple_length = parse_from_string(&d.get_dimple_length(), system).unwrap_or(0.0);
                    let kerf = parse_from_string(&d.get_burn(), system).unwrap_or(0.1);
                    let cut_feed = parse_from_string(&d.get_feed_rate(), system).unwrap_or(500.0);
                    let laser_passes = d.get_laser_passes().parse::<i32>().unwrap_or(3);
                    let laser_power = d.get_laser_power().parse::<i32>().unwrap_or(1000);
                    let offset_x = parse_from_string(&d.get_offset_x(), system).unwrap_or(10.0) as f32;
                    let offset_y = parse_from_string(&d.get_offset_y(), system).unwrap_or(10.0) as f32;
                    let dividers_x = d.get_dividers_x().parse::<u32>().unwrap_or(0);
                    let dividers_y = d.get_dividers_y().parse::<u32>().unwrap_or(0);
                    
                    let box_type_idx = d.get_box_type();
                    let joint_type_idx = d.get_finger_style();
                    let outside = d.get_outside_dimensions();
                    let optimize = d.get_optimize_layout();
                    let key_type_idx = d.get_key_divider_type();

                    let box_type = match box_type_idx {
                        0 => BoxType::FullBox,
                        1 => BoxType::NoTop,
                        2 => BoxType::NoBottom,
                        3 => BoxType::NoSides,
                        4 => BoxType::NoFrontBack,
                        5 => BoxType::NoLeftRight,
                        _ => BoxType::FullBox,
                    };

                    let finger_style = match joint_type_idx {
                        0 => FingerStyle::Rectangular,
                        1 => FingerStyle::Springs,
                        2 => FingerStyle::Barbs,
                        3 => FingerStyle::Snap,
                        4 => FingerStyle::Dogbone,
                        _ => FingerStyle::Rectangular,
                    };
                    
                    let key_divider_type = match key_type_idx {
                        0 => gcodekit5::KeyDividerType::WallsAndFloor,
                        1 => gcodekit5::KeyDividerType::WallsOnly,
                        2 => gcodekit5::KeyDividerType::FloorOnly,
                        3 => gcodekit5::KeyDividerType::None,
                        _ => gcodekit5::KeyDividerType::WallsAndFloor,
                    };

                    let params = BoxParameters {
                        x: width as f32,
                        y: height as f32,
                        h: depth as f32,
                        thickness: thickness as f32,
                        outside,
                        box_type,
                        finger_joint: FingerJointSettings {
                            finger: finger_width as f32,
                            space: space_width as f32,
                            surrounding_spaces: surrounding_spaces as f32,
                            play: play as f32,
                            extra_length: extra_length as f32,
                            style: finger_style,
                            dimple_height: dimple_height as f32,
                            dimple_length: dimple_length as f32,
                        },
                        burn: kerf as f32,
                        laser_passes,
                        laser_power,
                        feed_rate: cut_feed as f32,
                        offset_x,
                        offset_y,
                        dividers_x,
                        dividers_y,
                        optimize_layout: optimize,
                        key_divider_type,
                    };

                    // Generate G-code
                    let mut maker = match TabbedBoxMaker::new(params) {
                        Ok(m) => m,
                        Err(e) => {
                            warn!("Failed to create box maker: {}", e);
                            console_manager_dialog.add_message(
                                DeviceMessageType::Error,
                                format!("Failed to create box maker: {}", e),
                            );
                            return;
                        }
                    };

                    match maker.generate() {
                        Ok(_) => {
                            let gcode = maker.to_gcode();
                            
                            if let Some(w) = window_handle.upgrade() {
                                // Use the centralized loader which handles editor state, undo/redo, and scrolling
                                w.invoke_load_editor_text(slint::SharedString::from(gcode.clone()));
                                
                                w.set_gcode_filename(slint::SharedString::from("tabbed_box.gcode"));
                                w.set_current_view(slint::SharedString::from("gcode-editor"));
                                w.set_gcode_focus_trigger(w.get_gcode_focus_trigger() + 1);
                                
                                console_manager_dialog.add_message(
                                    DeviceMessageType::Success,
                                    "Generated tabbed box G-code".to_string(),
                                );

                                // Show success dialog
                                let success_dialog = SuccessDialog::new().unwrap();
                                success_dialog.set_message(
                                    slint::SharedString::from("Tabbed box G-code has been generated and loaded into the editor."),
                                );

                                let success_dialog_weak = success_dialog.as_weak();
                                success_dialog.on_close_dialog(move || {
                                    if let Some(dlg) = success_dialog_weak.upgrade() {
                                        dlg.hide().ok();
                                    }
                                });

                                success_dialog.show().ok();
                            }
                            d.hide().ok();
                        }
                        Err(e) => {
                            warn!("Failed to generate box: {}", e);
                            console_manager_dialog.add_message(
                                DeviceMessageType::Error,
                                format!("Failed to generate box: {}", e),
                            );
                        }
                    }
                }
            });

            let dialog_weak_cancel = dialog_weak.clone();
            dialog.on_cancel_dialog(move || {
                if let Some(d) = dialog_weak_cancel.upgrade() {
                    d.hide().ok();
                }
            });

            dialog.show().ok();
        }
    });

    // Set up generate-jigsaw-puzzle callback
    let window_weak = main_window.as_weak();
    let console_manager_clone = console_manager.clone();
    let settings_persistence_puzzle = settings_persistence.clone();
    let dialog_weak_ref = jigsaw_puzzle_dialog_weak.clone();
    main_window.on_generate_jigsaw_puzzle(move || {
        if let Some(window) = window_weak.upgrade() {
            // Check if dialog exists
            if let Some(existing) = dialog_weak_ref.borrow().as_ref().and_then(|w| w.upgrade()) {
                existing.show().unwrap();
                return;
            }

            let dialog = JigsawPuzzleDialog::new().unwrap();
            *dialog_weak_ref.borrow_mut() = Some(dialog.as_weak());
            let dialog_weak = dialog.as_weak();
            let dialog_weak_generate = dialog_weak.clone();

            // Get measurement system
            let system = {
                let persistence = settings_persistence_puzzle.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };

            // Set unit label
            dialog.set_unit_label(get_unit_label(system).into());

            // Initialize dialog with default values (converted)
            dialog.set_puzzle_width(to_display_string(200.0, system).into());
            dialog.set_puzzle_height(to_display_string(150.0, system).into());
            dialog.set_kerf(to_display_string(0.5, system).into());
            dialog.set_corner_radius(to_display_string(2.0, system).into());
            dialog.set_offset_x(to_display_string(10.0, system).into());
            dialog.set_offset_y(to_display_string(10.0, system).into());
            // These are unitless or percentages, so just set defaults as strings
            dialog.set_pieces_across("4".into());
            dialog.set_pieces_down("3".into());
            dialog.set_laser_passes("3".into());
            dialog.set_laser_power("1000".into());
            dialog.set_feed_rate(to_display_string(500.0, system).into());
            dialog.set_seed("42".into());
            dialog.set_tab_size("20.0".into());
            dialog.set_jitter("4.0".into());

            // Set up dialog callbacks
            let console_manager_dialog = console_manager_clone.clone();
            let window_handle = window.as_weak();

            // Load params callback
            let dialog_weak_load_params = dialog.as_weak();
            let settings_persistence_load_params = settings_persistence_puzzle.clone();
            dialog.on_load_params(move || {
                if let Some(dlg) = dialog_weak_load_params.upgrade() {
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_load_params.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::pick_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("JSON Files", &["json"]),
                        dlg.window()
                    )
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(params) = serde_json::from_str::<JigsawPuzzleParams>(&content) {
                                dlg.set_puzzle_width(params.puzzle_width.into());
                                dlg.set_puzzle_height(params.puzzle_height.into());
                                dlg.set_pieces_across(params.pieces_across.into());
                                dlg.set_pieces_down(params.pieces_down.into());
                                dlg.set_kerf(params.kerf.into());
                                dlg.set_laser_passes(params.laser_passes.into());
                                dlg.set_laser_power(params.laser_power.into());
                                dlg.set_feed_rate(params.feed_rate.into());
                                dlg.set_seed(params.seed.into());
                                dlg.set_tab_size(params.tab_size.into());
                                dlg.set_jitter(params.jitter.into());
                                dlg.set_corner_radius(params.corner_radius.into());
                                dlg.set_offset_x(params.offset_x.into());
                                dlg.set_offset_y(params.offset_y.into());
                            }
                        }
                    }
                    let _ = dlg.show();
                }
            });

            // Save params callback
            let dialog_weak_save_params = dialog.as_weak();
            let settings_persistence_save_params = settings_persistence_puzzle.clone();
            dialog.on_save_params(move || {
                if let Some(dlg) = dialog_weak_save_params.upgrade() {
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_save_params.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::save_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("JSON Files", &["json"]),
                        dlg.window()
                    )
                    {
                        let params = JigsawPuzzleParams {
                            puzzle_width: dlg.get_puzzle_width().to_string(),
                            puzzle_height: dlg.get_puzzle_height().to_string(),
                            pieces_across: dlg.get_pieces_across().to_string(),
                            pieces_down: dlg.get_pieces_down().to_string(),
                            kerf: dlg.get_kerf().to_string(),
                            laser_passes: dlg.get_laser_passes().to_string(),
                            laser_power: dlg.get_laser_power().to_string(),
                            feed_rate: dlg.get_feed_rate().to_string(),
                            seed: dlg.get_seed().to_string(),
                            tab_size: dlg.get_tab_size().to_string(),
                            jitter: dlg.get_jitter().to_string(),
                            corner_radius: dlg.get_corner_radius().to_string(),
                            offset_x: dlg.get_offset_x().to_string(),
                            offset_y: dlg.get_offset_y().to_string(),
                        };
                        
                        if let Ok(content) = serde_json::to_string_pretty(&params) {
                            let _ = std::fs::write(path, content);
                        }
                    }
                    let _ = dlg.show();
                }
            });

            dialog.on_generate_puzzle(move || {
                if let Some(d) = dialog_weak_generate.upgrade() {
                    // Get parameters from dialog
                    let width = parse_from_string(&d.get_puzzle_width(), system).unwrap_or(200.0);
                    let height = parse_from_string(&d.get_puzzle_height(), system).unwrap_or(150.0);
                    let rows = d.get_pieces_down().parse::<i32>().unwrap_or(3);
                    let cols = d.get_pieces_across().parse::<i32>().unwrap_or(4);
                    let tab_size = d.get_tab_size().parse::<f64>().unwrap_or(20.0);
                    let jitter = d.get_jitter().parse::<f64>().unwrap_or(4.0);
                    let kerf = parse_from_string(&d.get_kerf(), system).unwrap_or(0.5);
                    let cut_feed = parse_from_string(&d.get_feed_rate(), system).unwrap_or(500.0);
                    let seed = d.get_seed().parse::<u32>().unwrap_or(42);
                    let corner_radius = parse_from_string(&d.get_corner_radius(), system).unwrap_or(2.0);
                    let offset_x = parse_from_string(&d.get_offset_x(), system).unwrap_or(10.0) as f32;
                    let offset_y = parse_from_string(&d.get_offset_y(), system).unwrap_or(10.0) as f32;
                    let laser_passes = d.get_laser_passes().parse::<i32>().unwrap_or(3);
                    let laser_power = d.get_laser_power().parse::<i32>().unwrap_or(1000);

                    let params = PuzzleParameters {
                        width: width as f32,
                        height: height as f32,
                        pieces_across: cols,
                        pieces_down: rows,
                        tab_size_percent: tab_size as f32,
                        jitter_percent: jitter as f32,
                        kerf: kerf as f32,
                        feed_rate: cut_feed as f32,
                        seed,
                        corner_radius: corner_radius as f32,
                        offset_x,
                        offset_y,
                        laser_passes,
                        laser_power,
                    };

                    // Generate G-code
                    let mut maker = match JigsawPuzzleMaker::new(params) {
                        Ok(m) => m,
                        Err(e) => {
                            warn!("Failed to create puzzle maker: {}", e);
                            console_manager_dialog.add_message(
                                DeviceMessageType::Error,
                                format!("Failed to create puzzle maker: {}", e),
                            );
                            return;
                        }
                    };

                    match maker.generate() {
                        Ok(_) => {
                            let gcode = maker.to_gcode(100.0, 0.0); // Default plunge and depth
                            
                            if let Some(w) = window_handle.upgrade() {
                                // Use the centralized loader which handles editor state, undo/redo, and scrolling
                                w.invoke_load_editor_text(slint::SharedString::from(gcode.clone()));
                                
                                w.set_gcode_filename(slint::SharedString::from("jigsaw_puzzle.gcode"));
                                w.set_current_view(slint::SharedString::from("gcode-editor"));
                                w.set_gcode_focus_trigger(w.get_gcode_focus_trigger() + 1);
                                
                                console_manager_dialog.add_message(
                                    DeviceMessageType::Success,
                                    "Generated jigsaw puzzle G-code".to_string(),
                                );

                                // Show success dialog
                                let success_dialog = SuccessDialog::new().unwrap();
                                success_dialog.set_message(
                                    slint::SharedString::from("Jigsaw puzzle G-code has been generated and loaded into the editor."),
                                );

                                let success_dialog_weak = success_dialog.as_weak();
                                success_dialog.on_close_dialog(move || {
                                    if let Some(dlg) = success_dialog_weak.upgrade() {
                                        dlg.hide().ok();
                                    }
                                });

                                success_dialog.show().ok();
                            }
                            d.hide().ok();
                        }
                        Err(e) => {
                            warn!("Failed to generate puzzle: {}", e);
                            console_manager_dialog.add_message(
                                DeviceMessageType::Error,
                                format!("Failed to generate puzzle: {}", e),
                            );
                        }
                    }
                }
            });

            let dialog_weak_cancel = dialog_weak.clone();
            dialog.on_cancel_dialog(move || {
                if let Some(d) = dialog_weak_cancel.upgrade() {
                    d.hide().ok();
                }
            });

            dialog.show().ok();
        }
    });

    // Set up generate-spoilboard-surfacing callback
    let window_weak = main_window.as_weak();
    let console_manager_clone = console_manager.clone();
    let dialog_weak_ref = spoilboard_surfacing_dialog_weak.clone();
    let settings_persistence_spoilboard = settings_persistence.clone();
    let device_manager_spoilboard = device_manager.clone();
    let tools_backend_spoilboard = tools_backend.clone();
    
    // State to hold filtered tools for selection
    let current_filtered_tools = Rc::new(RefCell::new(Vec::<Tool>::new()));
    
    main_window.on_generate_spoilboard_surfacing(move || {
        if let Some(window) = window_weak.upgrade() {
            // Check if dialog exists
            if let Some(existing) = dialog_weak_ref.borrow().as_ref().and_then(|w| w.upgrade()) {
                existing.show().unwrap();
                return;
            }

            let dialog = SpoilboardSurfacingDialog::new().unwrap();
            *dialog_weak_ref.borrow_mut() = Some(dialog.as_weak());
            let dialog_weak = dialog.as_weak();
            let dialog_weak_generate = dialog_weak.clone();

            // Get measurement system
            let system = {
                let persistence = settings_persistence_spoilboard.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };

            // Set unit label
            dialog.set_unit_label(get_unit_label(system).into());

            // Initialize dialog with default values (converted)
            // Defaults in mm: Width 300, Height 180, Tool 25.4, Feed 1000, SafeZ 5.0, CutDepth 0.5
            dialog.set_width_mm(to_display_string(300.0, system).into());
            dialog.set_height_mm(to_display_string(180.0, system).into());
            dialog.set_tool_diameter(to_display_string(25.4, system).into());
            dialog.set_feed_rate(to_display_string(1000.0, system).into()); // Feed rate is usually mm/min or in/min
            dialog.set_safe_z(to_display_string(5.0, system).into());
            dialog.set_cut_depth(to_display_string(0.5, system).into());
            // Spindle speed and stepover are unitless/RPM/%

            // Populate Devices
            let profiles = device_manager_spoilboard.get_all_profiles();
            let device_names: Vec<slint::SharedString> = profiles.iter()
                .map(|p| slint::SharedString::from(p.name.clone()))
                .collect();
            dialog.set_devices(slint::ModelRc::new(VecModel::from(device_names)));

            // Populate Tool Categories
            let categories = ToolType::all();
            let category_names: Vec<slint::SharedString> = categories.iter()
                .map(|c| slint::SharedString::from(c.to_string()))
                .collect();
            dialog.set_tool_categories(slint::ModelRc::new(VecModel::from(category_names)));

            // Callback: Device Selected
            let dialog_weak_device = dialog.as_weak();
            let profiles_device = profiles.clone();
            let settings_persistence_device = settings_persistence_spoilboard.clone();
            dialog.on_device_selected(move |index| {
                if let Some(dlg) = dialog_weak_device.upgrade() {
                    if index >= 0 && (index as usize) < profiles_device.len() {
                        let profile = &profiles_device[index as usize];
                        // Get system for conversion
                        let system = {
                            let persistence = settings_persistence_device.borrow();
                            let sys_str = &persistence.config().ui.measurement_system;
                            MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
                        };
                        
                        // Update dimensions
                        let width = profile.x_axis.max - profile.x_axis.min;
                        let height = profile.y_axis.max - profile.y_axis.min;
                        dlg.set_width_mm(to_display_string(width as f32, system).into());
                        dlg.set_height_mm(to_display_string(height as f32, system).into());
                    }
                }
            });

            // Callback: Category Selected
            let dialog_weak_category = dialog.as_weak();
            let tools_backend_category = tools_backend_spoilboard.clone();
            let current_filtered_tools_category = current_filtered_tools.clone();
            dialog.on_category_selected(move |index| {
                if let Some(dlg) = dialog_weak_category.upgrade() {
                    if index >= 0 {
                        let categories = ToolType::all();
                        if (index as usize) < categories.len() {
                            let category = categories[index as usize];
                            let backend = tools_backend_category.borrow();
                            let all_tools = backend.get_all_tools();
                            
                            // Filter tools
                            let filtered: Vec<Tool> = all_tools.into_iter()
                                .filter(|t| t.tool_type == category)
                                .cloned()
                                .collect();
                            
                            // Update dropdown
                            let tool_names: Vec<slint::SharedString> = filtered.iter()
                                .map(|t| slint::SharedString::from(t.name.clone()))
                                .collect();
                            dlg.set_tools(slint::ModelRc::new(VecModel::from(tool_names)));
                            
                            // Update state
                            *current_filtered_tools_category.borrow_mut() = filtered;
                            
                            // Reset selection
                            dlg.set_selected_tool_index(-1);
                        }
                    }
                }
            });

            // Callback: Tool Selected
            let dialog_weak_tool = dialog.as_weak();
            let current_filtered_tools_tool = current_filtered_tools.clone();
            let settings_persistence_tool = settings_persistence_spoilboard.clone();
            dialog.on_tool_selected(move |index| {
                if let Some(dlg) = dialog_weak_tool.upgrade() {
                    let tools = current_filtered_tools_tool.borrow();
                    if index >= 0 && (index as usize) < tools.len() {
                        let tool = &tools[index as usize];
                        
                        // Get system for conversion
                        let system = {
                            let persistence = settings_persistence_tool.borrow();
                            let sys_str = &persistence.config().ui.measurement_system;
                            MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
                        };
                        
                        // Update diameter
                        dlg.set_tool_diameter(to_display_string(tool.diameter, system).into());
                    }
                }
            });

            // Set up dialog callbacks
            let console_manager_dialog = console_manager_clone.clone();
            let window_handle = window.as_weak();

            dialog.on_generate_gcode(move || {
                if let Some(d) = dialog_weak_generate.upgrade() {
                    // Get parameters from dialog and parse
                    let width = parse_from_string(&d.get_width_mm(), system).unwrap_or(300.0);
                    let height = parse_from_string(&d.get_height_mm(), system).unwrap_or(180.0);
                    let tool_diameter = parse_from_string(&d.get_tool_diameter(), system).unwrap_or(25.4);
                    let overlap = d.get_stepover().parse::<f64>().unwrap_or(40.0);
                    
                    // Feed rate needs special handling if we want to support in/min vs mm/min conversion
                    // The generator expects mm/min.
                    // parse_from_string converts input (in current units) to mm.
                    // If input is 100 in/min, parse_from_string("100", Imperial) returns 2540 mm/min.
                    // This is correct for the generator if it expects mm/min.
                    let feed_rate = parse_from_string(&d.get_feed_rate(), system).unwrap_or(1000.0);
                    
                    let spindle_speed = d.get_spindle_speed().parse::<f64>().unwrap_or(12000.0);
                    let safe_z = parse_from_string(&d.get_safe_z(), system).unwrap_or(5.0);
                    let cut_depth = parse_from_string(&d.get_cut_depth(), system).unwrap_or(0.5);

                    let params = SpoilboardSurfacingParameters {
                        width: width as f64,
                        height: height as f64,
                        tool_diameter: tool_diameter as f64,
                        stepover_percent: overlap,
                        feed_rate: feed_rate as f64,
                        spindle_speed,
                        safe_z: safe_z as f64,
                        cut_depth: cut_depth as f64,
                    };

                    // Generate G-code
                    let generator = SpoilboardSurfacingGenerator::new(params);
                    match generator.generate() {
                        Ok(gcode) => {
                            if let Some(w) = window_handle.upgrade() {
                                // Use the centralized loader which handles editor state, undo/redo, and scrolling
                                w.invoke_load_editor_text(slint::SharedString::from(gcode.clone()));
                                
                                w.set_gcode_filename(slint::SharedString::from("spoilboard_surfacing.gcode"));
                                w.set_current_view(slint::SharedString::from("gcode-editor"));
                                w.set_gcode_focus_trigger(w.get_gcode_focus_trigger() + 1);
                                
                                console_manager_dialog.add_message(
                                    DeviceMessageType::Success,
                                    "Generated spoilboard surfacing G-code".to_string(),
                                );

                                // Show success dialog
                                let success_dialog = SuccessDialog::new().unwrap();
                                success_dialog.set_message(
                                    slint::SharedString::from("Spoilboard surfacing G-code has been generated and loaded into the editor."),
                                );

                                let success_dialog_weak = success_dialog.as_weak();
                                success_dialog.on_close_dialog(move || {
                                    if let Some(dlg) = success_dialog_weak.upgrade() {
                                        dlg.hide().ok();
                                    }
                                });

                                success_dialog.show().ok();
                            }
                            d.hide().ok();
                        }
                        Err(e) => {
                            warn!("Failed to generate surfacing: {}", e);
                            console_manager_dialog.add_message(
                                DeviceMessageType::Error,
                                format!("Failed to generate surfacing: {}", e),
                            );
                        }
                    }
                }
            });

            let dialog_weak_cancel = dialog_weak.clone();
            dialog.on_cancel_dialog(move || {
                if let Some(d) = dialog_weak_cancel.upgrade() {
                    d.hide().ok();
                }
            });

            dialog.show().ok();
        }
    });

    // Set up generate-spoilboard-grid callback
    let window_weak = main_window.as_weak();
    let console_manager_clone = console_manager.clone();
    let dialog_weak_ref = spoilboard_grid_dialog_weak.clone();
    let settings_persistence_grid = settings_persistence.clone();
    main_window.on_generate_spoilboard_grid(move || {
        if let Some(window) = window_weak.upgrade() {
            // Check if dialog exists
            if let Some(existing) = dialog_weak_ref.borrow().as_ref().and_then(|w| w.upgrade()) {
                existing.show().unwrap();
                return;
            }

            let dialog = SpoilboardGridDialog::new().unwrap();
            *dialog_weak_ref.borrow_mut() = Some(dialog.as_weak());
            let dialog_weak = dialog.as_weak();
            let dialog_weak_generate = dialog_weak.clone();

            // Get measurement system
            let system = {
                let persistence = settings_persistence_grid.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };

            // Set unit label
            dialog.set_unit_label(get_unit_label(system).into());

            // Initialize dialog with default values
            dialog.set_width_mm(to_display_string(300.0, system).into());
            dialog.set_height_mm(to_display_string(180.0, system).into());
            dialog.set_grid_spacing(to_display_string(10.0, system).into());
            dialog.set_feed_rate(to_display_string(1000.0, system).into());
            dialog.set_laser_power("255".into());

            // Set up dialog callbacks
            let console_manager_dialog = console_manager_clone.clone();
            let window_handle = window.as_weak();

            dialog.on_generate_gcode(move || {
                if let Some(d) = dialog_weak_generate.upgrade() {
                    // Get parameters from dialog
                    let width = parse_from_string(&d.get_width_mm(), system).unwrap_or(300.0);
                    let height = parse_from_string(&d.get_height_mm(), system).unwrap_or(180.0);
                    let grid_spacing = parse_from_string(&d.get_grid_spacing(), system).unwrap_or(10.0);
                    let feed_rate = parse_from_string(&d.get_feed_rate(), system).unwrap_or(1000.0);
                    let laser_power = d.get_laser_power().parse::<f64>().unwrap_or(255.0);
                    let laser_mode = d.get_laser_mode().to_string();

                    let params = SpoilboardGridParameters {
                        width: width.into(),
                        height: height.into(),
                        grid_spacing: grid_spacing.into(),
                        feed_rate: feed_rate.into(),
                        laser_power,
                        laser_mode,
                    };

                    // Generate G-code
                    let generator = SpoilboardGridGenerator::new(params);
                    match generator.generate() {
                        Ok(gcode) => {
                            if let Some(w) = window_handle.upgrade() {
                                // Use the centralized loader which handles editor state, undo/redo, and scrolling
                                w.invoke_load_editor_text(slint::SharedString::from(gcode.clone()));
                                
                                w.set_gcode_filename(slint::SharedString::from("spoilboard_grid.gcode"));
                                w.set_current_view(slint::SharedString::from("gcode-editor"));
                                w.set_gcode_focus_trigger(w.get_gcode_focus_trigger() + 1);
                                
                                console_manager_dialog.add_message(
                                    DeviceMessageType::Success,
                                    "Generated spoilboard grid G-code".to_string(),
                                );

                                // Show success dialog
                                let success_dialog = SuccessDialog::new().unwrap();
                                success_dialog.set_message(
                                    slint::SharedString::from("Spoilboard grid G-code has been generated and loaded into the editor."),
                                );

                                let success_dialog_weak = success_dialog.as_weak();
                                success_dialog.on_close_dialog(move || {
                                    if let Some(dlg) = success_dialog_weak.upgrade() {
                                        dlg.hide().ok();
                                    }
                                });

                                success_dialog.show().ok();
                            }
                            d.hide().ok();
                        }
                        Err(e) => {
                            warn!("Failed to generate grid: {}", e);
                            console_manager_dialog.add_message(
                                DeviceMessageType::Error,
                                format!("Failed to generate grid: {}", e),
                            );
                        }
                    }
                }
            });

            let dialog_weak_cancel = dialog_weak.clone();
            dialog.on_cancel_dialog(move || {
                if let Some(d) = dialog_weak_cancel.upgrade() {
                    d.hide().ok();
                }
            });

            dialog.show().ok();
        }
    });

    // Set up generate-laser-engraving callback
    let window_weak = main_window.as_weak();
    let editor_bridge_laser = editor_bridge.clone();
    let settings_persistence_laser = settings_persistence.clone();
    let dialog_weak_ref = laser_engraver_dialog_weak.clone();
    main_window.on_generate_laser_engraving(move || {
        if let Some(main_win) = window_weak.upgrade() {
            // Check if dialog exists
            if let Some(existing) = dialog_weak_ref.borrow().as_ref().and_then(|w| w.upgrade()) {
                existing.show().unwrap();
                return;
            }

            let dialog = LaserEngraverDialog::new().unwrap();
            *dialog_weak_ref.borrow_mut() = Some(dialog.as_weak());

            // Get measurement system
            let system = {
                let persistence = settings_persistence_laser.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };

            // Set unit label
            dialog.set_unit_label(get_unit_label(system).into());

            // Initialize dialog with default values
            dialog.set_width_mm(to_display_string(100.0, system).into());
            dialog.set_feed_rate(to_display_string(1000.0, system).into());
            dialog.set_travel_rate(to_display_string(3000.0, system).into());
            dialog.set_min_power(0.0);
            dialog.set_max_power(100.0);
            dialog.set_pixels_per_mm(to_display_string(10.0, system).into());
            dialog.set_scan_direction("Horizontal".into());
            dialog.set_bidirectional(true);
            dialog.set_invert(false);
            dialog.set_line_spacing(to_display_string(1.0, system).into());
            dialog.set_power_scale("1000.0".into());

            // Load params callback
            let dialog_weak_load_params = dialog.as_weak();
            let settings_persistence_load_params = settings_persistence_laser.clone();
            dialog.on_load_params(move || {
                if let Some(dlg) = dialog_weak_load_params.upgrade() {
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_load_params.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::pick_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("JSON Files", &["json"]),
                        dlg.window()
                    )
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(params) = serde_json::from_str::<BitmapEngravingParams>(&content) {
                                dlg.set_width_mm(params.width_mm.to_string().into());
                                dlg.set_feed_rate(params.feed_rate.to_string().into());
                                dlg.set_travel_rate(params.travel_rate.to_string().into());
                                dlg.set_min_power(params.min_power);
                                dlg.set_max_power(params.max_power);
                                dlg.set_pixels_per_mm(params.pixels_per_mm.to_string().into());
                                dlg.set_scan_direction(params.scan_direction.into());
                                dlg.set_bidirectional(params.bidirectional);
                                dlg.set_invert(params.invert);
                                dlg.set_line_spacing(params.line_spacing.to_string().into());
                                dlg.set_power_scale(params.power_scale.to_string().into());
                                dlg.set_mirror_x(params.mirror_x);
                                dlg.set_mirror_y(params.mirror_y);
                                dlg.set_rotation(params.rotation.into());
                                dlg.set_halftone(params.halftone.into());
                                dlg.set_halftone_dot_size(params.halftone_dot_size);
                                dlg.set_halftone_threshold(params.halftone_threshold);
                                dlg.set_offset_x(params.offset_x.into());
                                dlg.set_offset_y(params.offset_y.into());
                                
                                // Load image path and update preview
                                let image_path = params.image_path;
                                if !image_path.is_empty() {
                                    dlg.set_image_path(image_path.clone().into());
                                    
                                    // Load and convert image to Slint format for preview
                                    if let Ok(img) = image::open(&image_path) {
                                        // Convert to RGB8 for display
                                        let rgb_img = img.to_rgb8();
                                        let width = rgb_img.width();
                                        let height = rgb_img.height();

                                        // Create Slint image buffer
                                        let buffer =
                                            slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
                                                rgb_img.as_raw(),
                                                width,
                                                height,
                                            );
                                        dlg.set_preview_image(slint::Image::from_rgb8(buffer));
                                    }
                                }

                                // Trigger preview update
                                dlg.invoke_update_preview();
                            }
                        }
                    }
                }
            });

            // Save params callback
            let dialog_weak_save_params = dialog.as_weak();
            let settings_persistence_save_params = settings_persistence_laser.clone();
            dialog.on_save_params(move || {
                if let Some(dlg) = dialog_weak_save_params.upgrade() {
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_save_params.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::save_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("JSON Files", &["json"]),
                        dlg.window()
                    )
                    {
                        let params = BitmapEngravingParams {
                            width_mm: dlg.get_width_mm().parse::<f32>().unwrap_or(100.0),
                            feed_rate: dlg.get_feed_rate().parse::<f32>().unwrap_or(1000.0),
                            travel_rate: dlg.get_travel_rate().parse::<f32>().unwrap_or(3000.0),
                            min_power: dlg.get_min_power(),
                            max_power: dlg.get_max_power(),
                            pixels_per_mm: dlg.get_pixels_per_mm().parse::<f32>().unwrap_or(10.0),
                            scan_direction: dlg.get_scan_direction().to_string(),
                            bidirectional: dlg.get_bidirectional(),
                            invert: dlg.get_invert(),
                            line_spacing: dlg.get_line_spacing().parse::<f32>().unwrap_or(1.0),
                            power_scale: dlg.get_power_scale().parse::<f32>().unwrap_or(1000.0),
                            mirror_x: dlg.get_mirror_x(),
                            mirror_y: dlg.get_mirror_y(),
                            rotation: dlg.get_rotation().to_string(),
                            halftone: dlg.get_halftone().to_string(),
                            halftone_dot_size: dlg.get_halftone_dot_size(),
                            halftone_threshold: dlg.get_halftone_threshold(),
                            offset_x: dlg.get_offset_x().to_string(),
                            offset_y: dlg.get_offset_y().to_string(),
                            image_path: dlg.get_image_path().to_string(),
                        };
                        
                        if let Ok(content) = serde_json::to_string_pretty(&params) {
                            let _ = std::fs::write(path, content);
                        }
                    }
                }
            });

            // Load image callback
            let dialog_weak_load = dialog.as_weak();
            let settings_persistence_load = settings_persistence_laser.clone();
            dialog.on_load_image(move || {
                if let Some(dlg) = dialog_weak_load.upgrade() {
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_load.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    // Open file dialog to select image
                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::pick_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp", "gif", "tiff"]),
                        dlg.window()
                    )
                    {
                        dlg.set_image_path(path.display().to_string().into());

                        // Load and convert image to Slint format for preview
                        if let Ok(img) = image::open(&path) {
                            // Convert to RGB8 for display
                            let rgb_img = img.to_rgb8();
                            let width = rgb_img.width();
                            let height = rgb_img.height();

                            // Create Slint image buffer
                            let buffer =
                                slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
                                    rgb_img.as_raw(),
                                    width,
                                    height,
                                );
                            dlg.set_preview_image(slint::Image::from_rgb8(buffer));

                            // Calculate and display output size
                            let _pixels_per_mm = parse_from_string(&dlg.get_pixels_per_mm(), system).unwrap_or(10.0);
                            let width_mm = parse_from_string(&dlg.get_width_mm(), system).unwrap_or(100.0);
                            let aspect_ratio = height as f32 / width as f32;
                            let height_mm = width_mm * aspect_ratio;
                            dlg.set_output_size(
                                format!("{:.1} x {:.1} {}", width_mm, height_mm, get_unit_label(system)).into(),
                            );
                        }
                    }
                }
            });

            // Update preview callback (when parameters change)
            let dialog_weak_update = dialog.as_weak();
            dialog.on_update_preview(move || {
                if let Some(dlg) = dialog_weak_update.upgrade() {
                    let image_path = dlg.get_image_path().to_string();
                    if !image_path.is_empty() {
                        if let Ok(img) = image::open(&image_path) {
                            let width_mm = parse_from_string(&dlg.get_width_mm(), system).unwrap_or(100.0);
                            let pixels_per_mm = parse_from_string(&dlg.get_pixels_per_mm(), system).unwrap_or(10.0);
                            let feed_rate = parse_from_string(&dlg.get_feed_rate(), system).unwrap_or(1000.0);
                            let travel_rate = parse_from_string(&dlg.get_travel_rate(), system).unwrap_or(3000.0);
                            let bidirectional = dlg.get_bidirectional();
                            let line_spacing = parse_from_string(&dlg.get_line_spacing(), system).unwrap_or(1.0);

                            // Calculate output dimensions
                            let aspect_ratio = img.height() as f32 / img.width() as f32;
                            let height_mm = width_mm * aspect_ratio;
                            dlg.set_output_size(
                                format!("{:.1} x {:.1} {}", width_mm, height_mm, get_unit_label(system)).into(),
                            );

                            // Estimate engraving time
                            let num_lines = (height_mm * pixels_per_mm / line_spacing) as u32;
                            let engrave_time = (width_mm * num_lines as f32) / feed_rate * 60.0;
                            let travel_time = if bidirectional {
                                (height_mm / travel_rate) * 60.0
                            } else {
                                (width_mm * num_lines as f32) / travel_rate * 60.0
                            };
                            let total_seconds = engrave_time + travel_time;
                            let minutes = (total_seconds / 60.0) as i32;
                            let seconds = (total_seconds % 60.0) as i32;
                            dlg.set_estimated_time(format!("{}:{:02}", minutes, seconds).into());
                        }
                    }
                }
            });

            // Generate G-code callback
            let main_win_clone = main_win.as_weak();
            let dialog_weak_generate = dialog.as_weak();
            let _editor_bridge_engraver = editor_bridge_laser.clone();
            dialog.on_generate_gcode(move || {
                if let Some(window) = main_win_clone.upgrade() {
                    if let Some(dlg) = dialog_weak_generate.upgrade() {
                        let image_path = dlg.get_image_path().to_string();

                        if image_path.is_empty() {
                            let error_dialog = ErrorDialog::new().unwrap();
                            error_dialog.set_error_message(
                                "No Image Selected\n\nPlease select an image file first.".into(),
                            );

                            let error_dialog_weak = error_dialog.as_weak();
                            error_dialog.on_close_dialog(move || {
                                if let Some(dlg) = error_dialog_weak.upgrade() {
                                    dlg.hide().ok();
                                }
                            });

                            error_dialog.show().ok();
                            return;
                        }

                        // Create engraving parameters - collect all data before spawning thread
                        use gcodekit5_camtools::{
                            EngravingParameters, ImageTransformations, BitmapImageEngraver, ScanDirection,
                            RotationAngle, HalftoneMethod,
                        };

                        let width_mm = parse_from_string(&dlg.get_width_mm(), system).unwrap_or(100.0);
                        let feed_rate = parse_from_string(&dlg.get_feed_rate(), system).unwrap_or(1000.0);
                        let travel_rate = parse_from_string(&dlg.get_travel_rate(), system).unwrap_or(3000.0);
                        let min_power = dlg.get_min_power();
                        let max_power = dlg.get_max_power();
                        let pixels_per_mm = parse_from_string(&dlg.get_pixels_per_mm(), system).unwrap_or(10.0);
                        let scan_dir = dlg.get_scan_direction().to_string();
                        let bidirectional = dlg.get_bidirectional();
                        let invert = dlg.get_invert();
                        let line_spacing = parse_from_string(&dlg.get_line_spacing(), system).unwrap_or(1.0);
                        let power_scale = dlg.get_power_scale().parse::<f32>().unwrap_or(1000.0);
                        let offset_x = parse_from_string(&dlg.get_offset_x(), system).unwrap_or(10.0) as f32;
                        let offset_y = parse_from_string(&dlg.get_offset_y(), system).unwrap_or(10.0) as f32;
                        
                        // Get transformation parameters from dialog
                        let mirror_x = dlg.get_mirror_x();
                        let mirror_y = dlg.get_mirror_y();
                        let rotation_str = dlg.get_rotation().to_string();
                        let halftone_str = dlg.get_halftone();
                        let halftone_dot_size = dlg.get_halftone_dot_size() as usize;
                        let halftone_threshold = dlg.get_halftone_threshold() as u8;

                        // Show status message and initial progress
                        window.set_connection_status("Generating laser engraving G-code...".into());
                        window.set_progress_value(0.0); // Starting

                        // Close dialog immediately
                        dlg.hide().ok();

                        // Spawn thread FIRST, before any UI operations
                        let window_weak_thread = window.as_weak();
                        let image_path_clone = image_path.clone();
                        std::thread::spawn(move || {

                            let params = EngravingParameters {
                                width_mm,
                                height_mm: None,
                                feed_rate,
                                travel_rate,
                                min_power,
                                max_power,
                                pixels_per_mm,
                                scan_direction: if scan_dir == "Horizontal" {
                                    ScanDirection::Horizontal
                                } else {
                                    ScanDirection::Vertical
                                },
                                bidirectional,
                                line_spacing,
                                power_scale,
                                transformations: ImageTransformations {
                                    mirror_x,
                                    mirror_y,
                                    rotation: match rotation_str.as_str() {
                                        "90°" => RotationAngle::Degrees90,
                                        "180°" => RotationAngle::Degrees180,
                                        "270°" => RotationAngle::Degrees270,
                                        _ => RotationAngle::Degrees0,
                                    },
                                    halftone: match halftone_str.as_str() {
                                        "Threshold" => HalftoneMethod::Threshold,
                                        "Bayer 4x4" => HalftoneMethod::Bayer4x4,
                                        "Floyd-Steinberg" => HalftoneMethod::FloydSteinberg,
                                        "Atkinson" => HalftoneMethod::Atkinson,
                                        _ => HalftoneMethod::None,
                                    },
                                    halftone_dot_size,
                                    halftone_threshold: halftone_threshold as u8,
                                    invert,
                                },
                                offset_x,
                                offset_y,
                            };

                            let result = BitmapImageEngraver::from_file(&image_path_clone, params)
                                .and_then(|engraver| {
                                    // Generate G-code with progress updates (0-100%)
                                    let gcode =
                                        engraver.generate_gcode_with_progress(|progress| {
                                            // Map internal progress (0.0-1.0) to 0-100% range
                                            let overall_progress = progress * 100.0;
                                            let _ = slint::invoke_from_event_loop({
                                                let ww = window_weak_thread.clone();
                                                move || {
                                                    if let Some(w) = ww.upgrade() {
                                                        w.set_progress_value(overall_progress);
                                                    }
                                                }
                                            });
                                        })?;

                                    Ok(gcode)
                                });


                            // Update UI from the main thread using slint::invoke_from_event_loop
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(win) = window_weak_thread.upgrade() {
                                    match result {
                                        Ok(gcode) => {
                                            win.set_progress_value(95.0); // Show progress before UI update
                                            win.set_connection_status(
                                                "Loading G-code into editor...".into(),
                                            );

                                            // Load into custom editor using callbacks
                                            win.invoke_load_editor_text(slint::SharedString::from(
                                                gcode.clone(),
                                            ));

                                            // Switch to editor view
                                            win.set_current_view("gcode-editor".into());
                                            win.set_gcode_focus_trigger(win.get_gcode_focus_trigger() + 1);
                                            win.set_connection_status(
                                                "Laser engraving G-code generated successfully"
                                                    .into(),
                                            );
                                            win.set_progress_value(100.0); // 100%

                                            // Show success dialog
                                            let success_dialog = SuccessDialog::new().unwrap();
                                            success_dialog.set_message(
                                                slint::SharedString::from("Laser engraving G-code has been generated and loaded into the editor."),
                                            );

                                            let success_dialog_weak = success_dialog.as_weak();
                                            success_dialog.on_close_dialog(move || {
                                                if let Some(dlg) = success_dialog_weak.upgrade() {
                                                    dlg.hide().ok();
                                                }
                                            });

                                            success_dialog.show().ok();

                                            // Hide progress after 1 second
                                            let win_weak = win.as_weak();
                                            slint::Timer::single_shot(
                                                std::time::Duration::from_secs(1),
                                                move || {
                                                    if let Some(w) = win_weak.upgrade() {
                                                        w.set_progress_value(0.0);
                                                    }
                                                },
                                            );
                                        }
                                        Err(e) => {
                                            // Build detailed error message with full chain
                                            let mut error_details = String::new();
                                            error_details.push_str("G-code Generation Failed\n\n");
                                            
                                            // Add root error
                                            error_details.push_str(&format!("Error: {}\n", e));
                                            
                                            // Add error chain if available (anyhow provides this)
                                            let mut source = e.source();
                                            let mut depth = 0;
                                            while let Some(err) = source {
                                                depth += 1;
                                                error_details.push_str(&format!("  └─ {}: {}\n", depth, err));
                                                source = err.source();
                                            }
                                            
                                            let error_msg = format!("Failed to generate laser engraving: {}", e);
                                            win.set_connection_status(error_msg.clone().into());
                                            win.set_progress_value(0.0); // Hide progress
                                            tracing::error!("G-code generation error:\n{}", error_details);

                                            let error_dialog = ErrorDialog::new().unwrap();
                                            error_dialog.set_error_message(error_details.into());

                                            let error_dialog_weak = error_dialog.as_weak();
                                            error_dialog.on_close_dialog(move || {
                                                if let Some(dlg) = error_dialog_weak.upgrade() {
                                                    dlg.hide().ok();
                                                }
                                            });

                                            error_dialog.show().ok();
                                        }
                                    }
                                }
                            });
                        });
                    }
                }
            });

            // Close dialog callback
            let dialog_weak_close = dialog.as_weak();
            dialog.on_close_dialog(move || {
                if let Some(dlg) = dialog_weak_close.upgrade() {
                    dlg.hide().ok();
                }
            });

            dialog.show().unwrap();
        }
    });

    // Vector Image Engraver
    let window_weak = main_window.as_weak();
    let editor_bridge_vector = editor_bridge.clone();
    let settings_persistence_vector = settings_persistence.clone();
    let dialog_weak_ref = vector_engraver_dialog_weak.clone();
    main_window.on_generate_vector_engraving(move || {
        if let Some(main_win) = window_weak.upgrade() {
            // Check if dialog exists
            if let Some(existing) = dialog_weak_ref.borrow().as_ref().and_then(|w| w.upgrade()) {
                existing.show().unwrap();
                return;
            }

            let dialog = VectorEngraverDialog::new().unwrap();
            *dialog_weak_ref.borrow_mut() = Some(dialog.as_weak());

            // Get measurement system
            let system = {
                let persistence = settings_persistence_vector.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };

            // Set unit label
            dialog.set_unit_label(get_unit_label(system).into());

            // Initialize dialog with default values
            dialog.set_feed_rate(to_display_string(600.0, system).into());
            dialog.set_travel_rate(to_display_string(3000.0, system).into());
            dialog.set_cut_power(100.0);
            dialog.set_engrave_power(50.0);
            dialog.set_power_scale("1000.0".into());
            dialog.set_multi_pass(false);
            dialog.set_num_passes(1);
            dialog.set_z_increment(to_display_string(0.5, system).into());
            dialog.set_invert_power(false);
            dialog.set_desired_width(to_display_string(100.0, system).into());
            dialog.set_hatch_spacing(to_display_string(1.0, system).into());
            dialog.set_hatch_tolerance(to_display_string(0.1, system).into());
            dialog.set_dwell_time("0.1".into());

            // Load params callback
            let dialog_weak_load_params = dialog.as_weak();
            let settings_persistence_load_params = settings_persistence_vector.clone();
            dialog.on_load_params(move || {
                if let Some(dlg) = dialog_weak_load_params.upgrade() {
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_load_params.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::pick_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("JSON Files", &["json"]),
                        dlg.window()
                    )
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(params) = serde_json::from_str::<VectorEngravingParams>(&content) {
                                dlg.set_feed_rate(params.feed_rate.to_string().into());
                                dlg.set_travel_rate(params.travel_rate.to_string().into());
                                dlg.set_cut_power(params.cut_power);
                                dlg.set_engrave_power(params.engrave_power);
                                dlg.set_power_scale(params.power_scale.to_string().into());
                                dlg.set_multi_pass(params.multi_pass);
                                dlg.set_num_passes(params.num_passes);
                                dlg.set_z_increment(params.z_increment.to_string().into());
                                dlg.set_invert_power(params.invert_power);
                                dlg.set_desired_width(params.desired_width.to_string().into());
                                dlg.set_offset_x(params.offset_x.into());
                                dlg.set_offset_y(params.offset_y.into());
                                dlg.set_enable_hatch(params.enable_hatch);
                                dlg.set_hatch_angle(params.hatch_angle);
                                dlg.set_hatch_spacing(params.hatch_spacing.to_string().into());
                                dlg.set_hatch_tolerance(params.hatch_tolerance.to_string().into());
                                dlg.set_cross_hatch(params.cross_hatch);
                                dlg.set_enable_dwell(params.enable_dwell);
                                dlg.set_dwell_time(params.dwell_time.to_string().into());
                                
                                // Load vector path and update file info
                                let vector_path = params.vector_path;
                                if !vector_path.is_empty() {
                                    dlg.set_vector_path(vector_path.clone().into());
                                    
                                    let path = std::path::Path::new(&vector_path);
                                    let file_name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Unknown");
                                    let file_info = format!("{} ({})", file_name, 
                                        path.extension()
                                            .and_then(|e| e.to_str())
                                            .unwrap_or("unknown")
                                            .to_uppercase());
                                    dlg.set_file_info(file_info.into());

                                    // Load preview if possible (SVG or DXF)
                                    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                                        let ext = extension.to_lowercase();
                                        if ext == "svg" {
                                            if let Ok(image) = slint::Image::load_from_path(&path) {
                                                let size = image.size();
                                                let width = size.width as f32;
                                                let height = size.height as f32;
                                                
                                                if width > 0.0 {
                                                    let aspect_ratio = height / width;
                                                    let desired_width = parse_from_string(&dlg.get_desired_width(), system).unwrap_or(100.0);
                                                    let output_height = desired_width * aspect_ratio;
                                                    
                                                    dlg.set_output_width(desired_width);
                                                    dlg.set_output_height(output_height);
                                                }
                                                
                                                dlg.set_preview_image(image);
                                            }
                                        } else if ext == "dxf" {
                                            // DXF Preview
                                            use gcodekit5_camtools::{VectorEngraver, VectorEngravingParameters};
                                            
                                            // Parse DXF using VectorEngraver with default params
                                            if let Ok(engraver) = VectorEngraver::from_file(&path, VectorEngravingParameters::default()) {
                                                // Render paths to image (800x600 max resolution for preview)
                                                let preview_img = engraver.render_preview(800, 600);
                                                
                                                // Convert to Slint image
                                                let width = preview_img.width();
                                                let height = preview_img.height();
                                                let buffer = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
                                                    preview_img.as_raw(),
                                                    width,
                                                    height,
                                                );
                                                let image = slint::Image::from_rgb8(buffer);
                                                
                                                // Calculate aspect ratio from bounds
                                                let (min_x, min_y, max_x, max_y) = engraver.get_bounds();
                                                let data_width = max_x - min_x;
                                                let data_height = max_y - min_y;
                                                
                                                if data_width > 0.0 {
                                                    let aspect_ratio = data_height / data_width;
                                                    let desired_width = parse_from_string(&dlg.get_desired_width(), system).unwrap_or(100.0);
                                                    let output_height = desired_width * aspect_ratio;
                                                    
                                                    dlg.set_output_width(desired_width);
                                                    dlg.set_output_height(output_height);
                                                }
                                                
                                                dlg.set_preview_image(image);
                                            }
                                        } else {
                                            // Clear preview for non-image formats
                                            dlg.set_preview_image(slint::Image::default());
                                            dlg.set_output_width(0.0);
                                            dlg.set_output_height(0.0);
                                        }
                                    }
                                }

                                // Trigger preview update
                                dlg.invoke_update_preview();
                            }
                        }
                    }
                }
            });

            // Save params callback
            let dialog_weak_save_params = dialog.as_weak();
            let settings_persistence_save_params = settings_persistence_vector.clone();
            dialog.on_save_params(move || {
                if let Some(dlg) = dialog_weak_save_params.upgrade() {
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_save_params.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::save_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("JSON Files", &["json"]),
                        dlg.window()
                    )
                    {
                        let params = VectorEngravingParams {
                            feed_rate: parse_from_string(&dlg.get_feed_rate(), system).unwrap_or(600.0) as f32,
                            travel_rate: parse_from_string(&dlg.get_travel_rate(), system).unwrap_or(3000.0) as f32,
                            cut_power: dlg.get_cut_power(),
                            engrave_power: dlg.get_engrave_power(),
                            power_scale: dlg.get_power_scale().parse::<f32>().unwrap_or(1000.0),
                            multi_pass: dlg.get_multi_pass(),
                            num_passes: dlg.get_num_passes(),
                            z_increment: parse_from_string(&dlg.get_z_increment(), system).unwrap_or(0.5) as f32,
                            invert_power: dlg.get_invert_power(),
                            desired_width: parse_from_string(&dlg.get_desired_width(), system).unwrap_or(100.0) as f32,
                            offset_x: dlg.get_offset_x().to_string(),
                            offset_y: dlg.get_offset_y().to_string(),
                            enable_hatch: dlg.get_enable_hatch(),
                            hatch_angle: dlg.get_hatch_angle(),
                            hatch_spacing: parse_from_string(&dlg.get_hatch_spacing(), system).unwrap_or(1.0) as f32,
                            hatch_tolerance: parse_from_string(&dlg.get_hatch_tolerance(), system).unwrap_or(0.1) as f32,
                            cross_hatch: dlg.get_cross_hatch(),
                            enable_dwell: dlg.get_enable_dwell(),
                            dwell_time: dlg.get_dwell_time().parse::<f32>().unwrap_or(0.1),
                            vector_path: dlg.get_vector_path().to_string(),
                        };
                        
                        if let Ok(content) = serde_json::to_string_pretty(&params) {
                            let _ = std::fs::write(path, content);
                        }
                    }
                }
            });

            // Load vector file callback
            let dialog_weak_load = dialog.as_weak();
            let settings_persistence_load = settings_persistence_vector.clone();
            dialog.on_load_vector_file(move || {
                if let Some(dlg) = dialog_weak_load.upgrade() {
                    // Get default directory
                    let default_dir = {
                        let persistence = settings_persistence_load.borrow();
                        persistence.config().file_processing.output_directory.clone()
                    };

                    // Open file dialog to select vector file
                    use rfd::FileDialog;
                    if let Some(path) = crate::platform::pick_file_with_parent(
                        FileDialog::new()
                            .set_directory(&default_dir)
                            .add_filter("Vector Files", &["svg", "dxf"]),
                        dlg.window()
                    )
                    {
                        dlg.set_vector_path(path.display().to_string().into());

                        // Display file info
                        let file_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        let file_info = format!("{} ({})", file_name, 
                            path.extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("unknown")
                                .to_uppercase());
                        dlg.set_file_info(file_info.into());

                        // Load preview if possible (SVG or DXF)
                        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                            let ext = extension.to_lowercase();
                            if ext == "svg" {
                                if let Ok(image) = slint::Image::load_from_path(&path) {
                                    let size = image.size();
                                    let width = size.width as f32;
                                    let height = size.height as f32;
                                    
                                    if width > 0.0 {
                                        let aspect_ratio = height / width;
                                        let desired_width = parse_from_string(&dlg.get_desired_width(), system).unwrap_or(100.0);
                                        let output_height = desired_width * aspect_ratio;
                                        
                                        dlg.set_output_width(desired_width);
                                        dlg.set_output_height(output_height);
                                    }
                                    
                                    dlg.set_preview_image(image);
                                }
                            } else if ext == "dxf" {
                                // DXF Preview
                                use gcodekit5_camtools::{VectorEngraver, VectorEngravingParameters};
                                
                                // Parse DXF using VectorEngraver with default params
                                if let Ok(engraver) = VectorEngraver::from_file(&path, VectorEngravingParameters::default()) {
                                    // Render paths to image (800x600 max resolution for preview)
                                    let preview_img = engraver.render_preview(800, 600);
                                    
                                    // Convert to Slint image
                                    let width = preview_img.width();
                                    let height = preview_img.height();
                                    let buffer = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
                                        preview_img.as_raw(),
                                        width,
                                        height,
                                    );
                                    let image = slint::Image::from_rgb8(buffer);
                                    
                                    // Calculate aspect ratio from bounds
                                    let (min_x, min_y, max_x, max_y) = engraver.get_bounds();
                                    let data_width = max_x - min_x;
                                    let data_height = max_y - min_y;
                                    
                                    if data_width > 0.0 {
                                        let aspect_ratio = data_height / data_width;
                                        let desired_width = parse_from_string(&dlg.get_desired_width(), system).unwrap_or(100.0);
                                        let output_height = desired_width * aspect_ratio;
                                        
                                        dlg.set_output_width(desired_width);
                                        dlg.set_output_height(output_height);
                                    }
                                    
                                    dlg.set_preview_image(image);
                                }
                            } else {
                                // Clear preview for non-image formats
                                dlg.set_preview_image(slint::Image::default());
                                dlg.set_output_width(0.0);
                                dlg.set_output_height(0.0);
                            }
                        }
                    }
                }
            });

            // Update preview callback (when parameters change)
            let dialog_weak_update = dialog.as_weak();
            dialog.on_update_preview(move || {
                if let Some(dlg) = dialog_weak_update.upgrade() {
                    let vector_path = dlg.get_vector_path().to_string();
                    if !vector_path.is_empty() {
                        // Update dimensions if preview image is available
                        let image = dlg.get_preview_image();
                        let size = image.size();
                        if size.width > 0 {
                            let aspect_ratio = size.height as f32 / size.width as f32;
                            let desired_width = parse_from_string(&dlg.get_desired_width(), system).unwrap_or(100.0);
                            let output_height = desired_width * aspect_ratio;
                            dlg.set_output_width(desired_width);
                            dlg.set_output_height(output_height);
                        }

                        let _feed_rate = parse_from_string(&dlg.get_feed_rate(), system).unwrap_or(600.0);
                        let _travel_rate = parse_from_string(&dlg.get_travel_rate(), system).unwrap_or(3000.0);
                        let multi_pass = dlg.get_multi_pass();
                        let num_passes = dlg.get_num_passes();

                        // Estimate cutting time (placeholder)
                        let base_time = 60.0; // seconds - would be calculated from vector analysis
                        let total_time = if multi_pass {
                            base_time * num_passes as f32
                        } else {
                            base_time
                        };
                        let minutes = (total_time / 60.0) as i32;
                        let seconds = (total_time % 60.0) as i32;
                        dlg.set_estimated_time(format!("{}:{:02}", minutes, seconds).into());
                    }
                }
            });

            // Generate G-code callback
            let main_win_clone = main_win.as_weak();
            let dialog_weak_generate = dialog.as_weak();
            let _editor_bridge_engraver_vector = editor_bridge_vector.clone();
            dialog.on_generate_gcode(move || {
                if let Some(window) = main_win_clone.upgrade() {
                    if let Some(dlg) = dialog_weak_generate.upgrade() {
                        let vector_path = dlg.get_vector_path().to_string();

                        if vector_path.is_empty() {
                            let error_dialog = ErrorDialog::new().unwrap();
                            error_dialog.set_error_message(
                                "No Vector File Selected\n\nPlease select an SVG or DXF file first.".into(),
                            );

                            let error_dialog_weak = error_dialog.as_weak();
                            error_dialog.on_close_dialog(move || {
                                if let Some(dlg) = error_dialog_weak.upgrade() {
                                    dlg.hide().ok();
                                }
                            });

                            error_dialog.show().ok();
                            return;
                        }

                        // Create vector engraving parameters
                        use gcodekit5_camtools::{
                            VectorEngraver, VectorEngravingParameters,
                        };

                        let feed_rate = parse_from_string(&dlg.get_feed_rate(), system).unwrap_or(600.0);
                        let travel_rate = parse_from_string(&dlg.get_travel_rate(), system).unwrap_or(3000.0);
                        let cut_power = dlg.get_cut_power();
                        let engrave_power = dlg.get_engrave_power();
                        let power_scale = dlg.get_power_scale().parse::<f32>().unwrap_or(1000.0);
                        let multi_pass = dlg.get_multi_pass();
                        let num_passes = dlg.get_num_passes() as u32;
                        let z_increment = parse_from_string(&dlg.get_z_increment(), system).unwrap_or(0.5);
                        let invert_power = dlg.get_invert_power();
                        let desired_width = parse_from_string(&dlg.get_desired_width(), system).unwrap_or(100.0);
                        let offset_x = parse_from_string(&dlg.get_offset_x(), system).unwrap_or(10.0) as f32;
                        let offset_y = parse_from_string(&dlg.get_offset_y(), system).unwrap_or(10.0) as f32;
                        let enable_hatch = dlg.get_enable_hatch();
                        let hatch_angle = dlg.get_hatch_angle();
                        let hatch_spacing = parse_from_string(&dlg.get_hatch_spacing(), system).unwrap_or(1.0);
                        let hatch_tolerance = parse_from_string(&dlg.get_hatch_tolerance(), system).unwrap_or(0.1);
                        let cross_hatch = dlg.get_cross_hatch();
                        let enable_dwell = dlg.get_enable_dwell();
                        let dwell_time = dlg.get_dwell_time().parse::<f32>().unwrap_or(0.1);

                        // Show status message and initial progress
                        window.set_connection_status("Generating vector engraving G-code...".into());
                        window.set_progress_value(0.0); // Starting

                        // Close dialog immediately
                        dlg.hide().ok();

                        // Spawn thread FIRST, before any UI operations
                        let window_weak_thread = window.as_weak();
                        let vector_path_clone = vector_path.clone();
                        std::thread::spawn(move || {

                            let params = VectorEngravingParameters {
                                feed_rate,
                                travel_rate,
                                cut_power,
                                engrave_power,
                                power_scale,
                                multi_pass,
                                num_passes,
                                z_increment,
                                invert_power,
                                desired_width,
                                offset_x,
                                offset_y,
                                enable_hatch,
                                hatch_angle,
                                hatch_spacing,
                                hatch_tolerance,
                                cross_hatch,
                                enable_dwell,
                                dwell_time,
                            };

                            let result = VectorEngraver::from_file(&vector_path_clone, params)
                                .and_then(|engraver| {
                                    // Generate G-code with progress updates (0-100%)
                                    let gcode =
                                        engraver.generate_gcode_with_progress(|progress| {
                                            // Map internal progress (0.0-1.0) to 0-100% range
                                            let overall_progress = progress * 100.0;
                                            let _ = slint::invoke_from_event_loop({
                                                let ww = window_weak_thread.clone();
                                                move || {
                                                    if let Some(w) = ww.upgrade() {
                                                        w.set_progress_value(overall_progress);
                                                    }
                                                }
                                            });
                                        })?;

                                    Ok(gcode)
                                });


                            // Update UI from the main thread using slint::invoke_from_event_loop
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(win) = window_weak_thread.upgrade() {
                                    match result {
                                        Ok(gcode) => {
                                            win.set_progress_value(95.0); // Show progress before UI update
                                            win.set_connection_status(
                                                "Loading G-code into editor...".into(),
                                            );

                                            // Load into custom editor using callbacks
                                            win.invoke_load_editor_text(slint::SharedString::from(
                                                gcode.clone(),
                                            ));

                                            // Switch to editor view
                                            win.set_current_view("gcode-editor".into());
                                            win.set_gcode_focus_trigger(win.get_gcode_focus_trigger() + 1);
                                            win.set_connection_status(
                                                "Vector engraving G-code generated successfully"
                                                    .into(),
                                            );
                                            win.set_progress_value(100.0); // 100%

                                            // Trigger visualizer refresh after switching view
                                            let win_weak_viz = win.as_weak();
                                            slint::Timer::single_shot(
                                                std::time::Duration::from_millis(100),
                                                move || {
                                                    if let Some(w) = win_weak_viz.upgrade() {
                                                        let canvas_width = w.get_visualizer_canvas_width();
                                                        let canvas_height = w.get_visualizer_canvas_height();
                                                        let max_intensity = w.get_visualizer_max_intensity();
                                                        w.invoke_refresh_visualization(canvas_width, canvas_height, max_intensity);
                                                    }
                                                },
                                            );

                                            // Show success dialog
                                            let success_dialog = SuccessDialog::new().unwrap();
                                            success_dialog.set_message(
                                                slint::SharedString::from("Vector engraving G-code has been generated and loaded into the editor."),
                                            );

                                            let success_dialog_weak = success_dialog.as_weak();
                                            success_dialog.on_close_dialog(move || {
                                                if let Some(dlg) = success_dialog_weak.upgrade() {
                                                    dlg.hide().ok();
                                                }
                                            });

                                            success_dialog.show().ok();

                                            // Hide progress after 1 second
                                            let win_weak = win.as_weak();
                                            slint::Timer::single_shot(
                                                std::time::Duration::from_secs(1),
                                                move || {
                                                    if let Some(w) = win_weak.upgrade() {
                                                        w.set_progress_value(0.0);
                                                    }
                                                },
                                            );
                                        }
                                        Err(e) => {
                                            // Build detailed error message with full chain
                                            let mut error_details = String::new();
                                            error_details.push_str("Vector G-code Generation Failed\n\n");
                                            
                                            // Add root error
                                            error_details.push_str(&format!("Error: {}\n", e));
                                            
                                            // Add error chain if available
                                            let mut source = e.source();
                                            let mut depth = 0;
                                            while let Some(err) = source {
                                                depth += 1;
                                                error_details.push_str(&format!("  └─ {}: {}\n", depth, err));
                                                source = err.source();
                                            }
                                            
                                            let error_msg = format!("Failed to generate vector engraving: {}", e);
                                            win.set_connection_status(error_msg.clone().into());
                                            win.set_progress_value(0.0); // Hide progress
                                            tracing::error!("Vector G-code generation error:\n{}", error_details);

                                            let error_dialog = ErrorDialog::new().unwrap();
                                            error_dialog.set_error_message(error_details.into());

                                            let error_dialog_weak = error_dialog.as_weak();
                                            error_dialog.on_close_dialog(move || {
                                                if let Some(dlg) = error_dialog_weak.upgrade() {
                                                    dlg.hide().ok();
                                                }
                                            });

                                            error_dialog.show().ok();
                                        }
                                    }
                                }
                            });
                        });
                    }
                }
            });

            // Close dialog callback
            let dialog_weak_close = dialog.as_weak();
            dialog.on_close_dialog(move || {
                if let Some(dlg) = dialog_weak_close.upgrade() {
                    dlg.hide().ok();
                }
            });

            dialog.show().unwrap();
        }
    });

    // Speeds and Feeds Calculator
    let window_weak = main_window.as_weak();
    let materials_backend_sf = materials_backend.clone();
    let tools_backend_sf = tools_backend.clone();
    let _device_manager_sf = device_manager.clone();
    let settings_persistence_sf = settings_persistence.clone();

    main_window.on_load_speeds_feeds_data(move || {
        if let Some(window) = window_weak.upgrade() {
            // Get measurement system
            let system = {
                let persistence = settings_persistence_sf.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };

            // Set unit label
            window.set_sf_unit_label(get_unit_label(system).into());

            // Load Materials
            let backend = materials_backend_sf.borrow();
            let materials = backend.get_all_materials();
            let material_names: Vec<slint::SharedString> = materials
                .iter()
                .map(|m| slint::SharedString::from(m.name.clone()))
                .collect();
            window.set_sf_materials(slint::ModelRc::new(VecModel::from(material_names)));

            // Load Tools
            let tool_backend = tools_backend_sf.borrow();
            let tools = tool_backend.get_all_tools();
            let tool_names: Vec<slint::SharedString> = tools
                .iter()
                .map(|t| slint::SharedString::from(t.name.clone()))
                .collect();
            window.set_sf_tools(slint::ModelRc::new(VecModel::from(tool_names)));

            // Load Operations
            // let operations = vec![
            //     slint::SharedString::from("Slotting"),
            //     slint::SharedString::from("Profiling"),
            //     slint::SharedString::from("Pocketing"),
            //     slint::SharedString::from("Drilling"),
            // ];
            // window.set_sf_operations(slint::ModelRc::new(VecModel::from(operations)));
        }
    });

    let window_weak = main_window.as_weak();
    let materials_backend_sf = materials_backend.clone();
    let tools_backend_sf = tools_backend.clone();
    let device_manager_sf = device_manager.clone();
    let settings_persistence_sf_calc = settings_persistence.clone();

    main_window.on_calculate_speeds_feeds(move || {
        if let Some(window) = window_weak.upgrade() {
            let material_idx = window.get_sf_selected_material_index();
            let tool_idx = window.get_sf_selected_tool_index();
            // let op_idx = window.get_sf_selected_operation_index();

            if material_idx < 0 || tool_idx < 0 { // || op_idx < 0 {
                return;
            }

            let mat_backend = materials_backend_sf.borrow();
            let materials = mat_backend.get_all_materials();
            let material = &materials[material_idx as usize];

            let tool_backend = tools_backend_sf.borrow();
            let tools = tool_backend.get_all_tools();
            let tool = &tools[tool_idx as usize];

            // Get measurement system
            let system = {
                let persistence = settings_persistence_sf_calc.borrow();
                let sys_str = &persistence.config().ui.measurement_system;
                MeasurementSystem::from_str(sys_str).unwrap_or(MeasurementSystem::Metric)
            };

            // Get machine limits and calculate
            if let Some(profile) = device_manager_sf.get_active_profile() {
                let result = SpeedsFeedsCalculator::calculate(
                    material,
                    tool,
                    &profile,
                );

                // Update UI
                window.set_sf_result_rpm(format!("{}", result.rpm).into());
                
                // Convert feed rate if needed (result is in mm/min)
                let feed_display = to_display_string(result.feed_rate as f32, system);
                window.set_sf_result_feed(feed_display.into());
                
                // Surface speed (m/min or ft/min)
                // result.surface_speed is in m/min
                let surface_speed_display = match system {
                    MeasurementSystem::Metric => format!("{:.1} m/min", result.surface_speed),
                    MeasurementSystem::Imperial => format!("{:.1} ft/min", result.surface_speed * 3.28084),
                };
                window.set_sf_result_surface_speed(surface_speed_display.into());
                
                // Chip load (mm/tooth or in/tooth)
                // result.chip_load is in mm/tooth
                let chip_load_display = match system {
                    MeasurementSystem::Metric => format!("{:.3} mm/tooth", result.chip_load),
                    MeasurementSystem::Imperial => format!("{:.4} in/tooth", result.chip_load / 25.4),
                };
                window.set_sf_result_chip_load(chip_load_display.into());
                
                // window.set_sf_result_plunge(result.plunge_rate as i32);
                // window.set_sf_result_depth(result.depth_of_cut as f32);
                // window.set_sf_result_mrr(result.mrr as f32);
                // window.set_sf_result_power(result.power_required as f32);
            }
        }
    });
}
