//! UI builder methods for DesignerView

use super::*;

impl DesignerView {
    /// Creates the view controls expander for the left sidebar
    pub(crate) fn create_view_controls_expander(
        state: &Rc<RefCell<DesignerState>>,
        canvas: &Rc<DesignerCanvas>,
        settings_controller: &Rc<SettingsController>,
    ) -> gtk4::Expander {
        let view_controls_box = Box::new(Orientation::Vertical, 6);
        view_controls_box.set_margin_start(6);
        view_controls_box.set_margin_end(6);
        view_controls_box.set_margin_top(6);
        view_controls_box.set_margin_bottom(6);

        // Grid toggle
        let grid_toggle = gtk4::CheckButton::with_label(&t!("Show Grid"));
        grid_toggle.set_active(true);
        {
            let state_grid = state.clone();
            let canvas_grid = canvas.clone();
            grid_toggle.connect_toggled(move |btn| {
                state_grid.borrow_mut().show_grid = btn.is_active();
                canvas_grid.widget.queue_draw();
            });
        }
        view_controls_box.append(&grid_toggle);

        // Grid spacing
        let system = settings_controller
            .persistence
            .borrow()
            .config()
            .ui
            .measurement_system;
        let unit_label = gcodekit5_core::units::get_unit_label(system);

        let grid_spacing_combo = gtk4::ComboBoxText::new();
        grid_spacing_combo.set_hexpand(true);
        grid_spacing_combo.set_tooltip_text(Some(&t!("Grid spacing")));

        for mm in [1.0_f64, 5.0, 10.0, 25.0, 50.0] {
            let label = format!(
                "{} {}",
                gcodekit5_core::units::format_length(mm as f32, system),
                unit_label
            );
            grid_spacing_combo.append(Some(&mm.to_string()), &label);
        }

        grid_spacing_combo.set_active_id(Some("10"));
        {
            let state_grid_spacing = state.clone();
            let canvas_grid_spacing = canvas.clone();
            grid_spacing_combo.connect_changed(move |cb| {
                if let Some(id) = cb.active_id() {
                    if let Ok(mm) = id.parse::<f64>() {
                        state_grid_spacing.borrow_mut().grid_spacing_mm = mm;
                        canvas_grid_spacing.widget.queue_draw();
                    }
                }
            });
        }

        let grid_spacing_row = Box::new(Orientation::Horizontal, 6);
        grid_spacing_row.append(&Label::new(Some(&t!("Grid spacing"))));
        grid_spacing_row.append(&grid_spacing_combo);
        view_controls_box.append(&grid_spacing_row);

        // Snap controls
        let snap_toggle = gtk4::CheckButton::with_label(&t!("Snap"));
        snap_toggle.set_tooltip_text(Some(&t!("Snap to grid")));
        snap_toggle.set_active(state.borrow().snap_enabled);
        {
            let state_snap = state.clone();
            snap_toggle.connect_toggled(move |btn| {
                state_snap.borrow_mut().snap_enabled = btn.is_active();
            });
        }
        view_controls_box.append(&snap_toggle);

        let snap_threshold = gtk4::SpinButton::with_range(0.0, 5.0, 0.1);
        snap_threshold.set_tooltip_text(Some(&t!("Snap threshold")));
        let snap_display = match system {
            gcodekit5_core::units::MeasurementSystem::Metric => state.borrow().snap_threshold_mm,
            gcodekit5_core::units::MeasurementSystem::Imperial => {
                state.borrow().snap_threshold_mm / 25.4
            }
        };
        snap_threshold.set_value(snap_display);
        {
            let state_snap = state.clone();
            snap_threshold.connect_value_changed(move |sp| {
                let val = sp.value();
                let mm = match system {
                    gcodekit5_core::units::MeasurementSystem::Metric => val,
                    gcodekit5_core::units::MeasurementSystem::Imperial => val * 25.4,
                };
                state_snap.borrow_mut().snap_threshold_mm = mm.max(0.0);
            });
        }

        let snap_threshold_row = Box::new(Orientation::Horizontal, 6);
        snap_threshold_row.append(&Label::new(Some(&t!("Snap threshold"))));
        snap_threshold_row.append(&snap_threshold);
        view_controls_box.append(&snap_threshold_row);

        // Toolpath toggle
        let toolpath_toggle = gtk4::CheckButton::with_label(&t!("Show Toolpaths"));
        toolpath_toggle.set_active(false);
        {
            let state_toolpath = state.clone();
            let canvas_toolpath = canvas.clone();
            toolpath_toggle.connect_toggled(move |btn| {
                let active = btn.is_active();
                state_toolpath.borrow_mut().show_toolpaths = active;
                if active {
                    canvas_toolpath.generate_preview_toolpaths();
                } else {
                    canvas_toolpath.widget.queue_draw();
                }
            });
        }
        view_controls_box.append(&toolpath_toggle);

        // Preview generation progress + cancel
        let preview_spinner = gtk4::Spinner::new();
        preview_spinner.set_visible(false);

        let preview_cancel_btn = gtk4::Button::builder()
            .icon_name("process-stop-symbolic")
            .tooltip_text(t!("Cancel"))
            .build();
        preview_cancel_btn.set_visible(false);
        preview_cancel_btn.update_property(&[gtk4::accessible::Property::Label(&t!("Cancel"))]);

        {
            let cancel_flag = canvas.preview_cancel.clone();
            let generating = canvas.preview_generating.clone();
            preview_cancel_btn.connect_clicked(move |_| {
                cancel_flag.store(true, Ordering::SeqCst);
                generating.set(false);
            });
        }

        let preview_row = Box::new(Orientation::Horizontal, 6);
        preview_row.append(&preview_spinner);
        preview_row.append(&preview_cancel_btn);
        view_controls_box.append(&preview_row);

        // Keep widgets in sync with generating state
        {
            let generating = canvas.preview_generating.clone();
            let spinner = preview_spinner;
            let cancel_btn = preview_cancel_btn;
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                let on = generating.get();
                spinner.set_visible(on);
                cancel_btn.set_visible(on);
                if on {
                    spinner.start();
                } else {
                    spinner.stop();
                }
                gtk4::glib::ControlFlow::Continue
            });
        }

        gtk4::Expander::builder()
            .label(t!("View Controls"))
            .expanded(true)
            .child(&view_controls_box)
            .build()
    }

    /// Sets up keyboard shortcuts for the designer canvas
    pub(crate) fn setup_keyboard_shortcuts(canvas: &Rc<DesignerCanvas>) {
        let key_controller = EventControllerKey::new();
        let canvas_keys = canvas.clone();
        key_controller.connect_key_pressed(move |_, key, _code, modifiers| {
            let ctrl = modifiers.contains(ModifierType::CONTROL_MASK);
            let alt = modifiers.contains(ModifierType::ALT_MASK);

            match (key, ctrl, alt) {
                // Ctrl+Z - Undo
                (Key::z, true, _) | (Key::Z, true, _) => {
                    canvas_keys.undo();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+Y - Redo
                (Key::y, true, _) | (Key::Y, true, _) => {
                    canvas_keys.redo();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+C - Copy
                (Key::c, true, _) | (Key::C, true, _) => {
                    canvas_keys.copy_selected();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+V - Paste
                (Key::v, true, false) | (Key::V, true, false) => {
                    canvas_keys.paste();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+D - Duplicate
                (Key::d, true, _) | (Key::D, true, _) => {
                    canvas_keys.duplicate_selected();
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+G - Group (Shift+G for Ungroup)
                (Key::g, true, _) | (Key::G, true, _) => {
                    if modifiers.contains(ModifierType::SHIFT_MASK) {
                        canvas_keys.ungroup_selected();
                    } else {
                        canvas_keys.group_selected();
                    }
                    gtk4::glib::Propagation::Stop
                }
                // Ctrl+U - Ungroup
                (Key::u, true, _) | (Key::U, true, _) => {
                    canvas_keys.ungroup_selected();
                    gtk4::glib::Propagation::Stop
                }
                // Delete or Backspace - Delete selected
                (Key::Delete, _, _) | (Key::BackSpace, _, _) => {
                    canvas_keys.delete_selected();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+L - Align Left
                (Key::l, false, true) | (Key::L, false, true) => {
                    canvas_keys.align_left();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+R - Align Right
                (Key::r, false, true) | (Key::R, false, true) => {
                    canvas_keys.align_right();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+T - Align Top
                (Key::t, false, true) | (Key::T, false, true) => {
                    canvas_keys.align_top();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+B - Align Bottom
                (Key::b, false, true) | (Key::B, false, true) => {
                    canvas_keys.align_bottom();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+H - Align Center Horizontal
                (Key::h, false, true) | (Key::H, false, true) => {
                    canvas_keys.align_center_horizontal();
                    gtk4::glib::Propagation::Stop
                }
                // Alt+V - Align Center Vertical
                (Key::v, false, true) | (Key::V, false, true) => {
                    canvas_keys.align_center_vertical();
                    gtk4::glib::Propagation::Stop
                }
                // View shortcuts (no Ctrl/Alt)
                (Key::plus, false, false)
                | (Key::KP_Add, false, false)
                | (Key::equal, false, false) => {
                    canvas_keys.zoom_in();
                    gtk4::glib::Propagation::Stop
                }
                (Key::minus, false, false)
                | (Key::KP_Subtract, false, false)
                | (Key::underscore, false, false) => {
                    canvas_keys.zoom_out();
                    gtk4::glib::Propagation::Stop
                }
                (Key::f, false, false) | (Key::F, false, false) => {
                    canvas_keys.zoom_fit();
                    gtk4::glib::Propagation::Stop
                }
                (Key::r, false, false) | (Key::R, false, false) => {
                    canvas_keys.reset_view();
                    gtk4::glib::Propagation::Stop
                }
                (Key::d, false, false) | (Key::D, false, false) => {
                    canvas_keys.fit_to_device_area();
                    canvas_keys.widget.queue_draw();
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });

        canvas.widget.set_focusable(true);
        canvas.widget.set_can_focus(true);
        canvas.widget.add_controller(key_controller);

        // Grab focus on canvas when clicked
        let canvas_focus = canvas.clone();
        let click_for_focus = GestureClick::new();
        click_for_focus.set_button(1);
        click_for_focus.connect_pressed(move |_, _, _, _| {
            canvas_focus.widget.grab_focus();
        });
        canvas.widget.add_controller(click_for_focus);
    }

    /// Creates the floating zoom/view controls for the canvas overlay.
    /// Returns (floating_box, zoom_in, zoom_out, fit, reset, fit_device, scrollbars_btn)
    #[allow(clippy::type_complexity)]
    pub(crate) fn create_floating_controls(
        has_device_manager: bool,
    ) -> (
        Box,
        gtk4::Button,
        gtk4::Button,
        gtk4::Button,
        gtk4::Button,
        gtk4::Button,
        gtk4::Button,
    ) {
        let floating_box = Box::new(Orientation::Horizontal, 4);
        floating_box.add_css_class("visualizer-osd");
        floating_box.add_css_class("osd-controls");
        floating_box.set_halign(gtk4::Align::End);
        floating_box.set_valign(gtk4::Align::End);
        floating_box.set_margin_bottom(20);
        floating_box.set_margin_end(20);

        let float_zoom_out = gtk4::Button::builder()
            .icon_name("zoom-out-symbolic")
            .tooltip_text(t!("Zoom Out"))
            .build();
        float_zoom_out.update_property(&[gtk4::accessible::Property::Label(&t!("Zoom Out"))]);

        let float_fit = gtk4::Button::builder()
            .icon_name("zoom-fit-best-symbolic")
            .tooltip_text(t!("Fit to Content"))
            .build();
        float_fit.update_property(&[gtk4::accessible::Property::Label(&t!("Fit to Content"))]);

        let float_reset = gtk4::Button::builder()
            .icon_name("view-restore-symbolic")
            .tooltip_text(t!("Fit to Viewport"))
            .build();
        float_reset.update_property(&[gtk4::accessible::Property::Label(&t!("Fit to Viewport"))]);

        let float_fit_device = gtk4::Button::builder()
            .icon_name("preferences-desktop-display-symbolic")
            .tooltip_text(t!("Fit to Device Working Area"))
            .build();
        float_fit_device.update_property(&[gtk4::accessible::Property::Label(&t!(
            "Fit to Device Working Area"
        ))]);

        let scrollbars_btn = gtk4::Button::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text(t!("Toggle Scrollbars"))
            .build();
        scrollbars_btn
            .update_property(&[gtk4::accessible::Property::Label(&t!("Toggle Scrollbars"))]);

        let help_btn = gtk4::Button::builder()
            .label("?")
            .tooltip_text(t!("Keyboard Shortcuts"))
            .build();
        help_btn.update_property(&[gtk4::accessible::Property::Label(&t!("Keyboard Shortcuts"))]);

        let help_popover = Popover::new();
        help_popover.set_parent(&help_btn);
        help_popover.set_has_arrow(true);
        {
            let help_box = Box::new(Orientation::Vertical, 6);
            help_box.set_margin_start(12);
            help_box.set_margin_end(12);
            help_box.set_margin_top(12);
            help_box.set_margin_bottom(12);
            help_box.append(&Label::new(Some(&t!("Designer shortcuts"))));
            help_box.append(&Label::new(Some("Ctrl+Z / Ctrl+Y  —  Undo / Redo")));
            help_box.append(&Label::new(Some("Ctrl+C / Ctrl+V  —  Copy / Paste")));
            help_box.append(&Label::new(Some("Delete  —  Delete selection")));
            help_box.append(&Label::new(Some("+ / -  —  Zoom")));
            help_box.append(&Label::new(Some("F  —  Fit to Content")));
            help_box.append(&Label::new(Some("R  —  Fit to Viewport")));
            help_box.append(&Label::new(Some("D  —  Fit to Device Working Area")));
            help_box.append(&Label::new(Some(&t!("Right click for context menu"))));
            help_popover.set_child(Some(&help_box));
        }
        {
            let pop = help_popover.clone();
            help_btn.connect_clicked(move |_| pop.popup());
        }

        let float_zoom_in = gtk4::Button::builder()
            .icon_name("zoom-in-symbolic")
            .tooltip_text(t!("Zoom In"))
            .build();
        float_zoom_in.update_property(&[gtk4::accessible::Property::Label(&t!("Zoom In"))]);

        for b in [
            &float_zoom_out,
            &float_fit,
            &float_reset,
            &float_fit_device,
            &scrollbars_btn,
            &help_btn,
            &float_zoom_in,
        ] {
            b.set_size_request(28, 28);
        }

        floating_box.append(&float_zoom_out);
        floating_box.append(&float_fit);
        floating_box.append(&float_reset);
        if has_device_manager {
            floating_box.append(&float_fit_device);
        }
        floating_box.append(&scrollbars_btn);
        floating_box.append(&help_btn);
        floating_box.append(&float_zoom_in);

        (
            floating_box,
            float_zoom_in,
            float_zoom_out,
            float_fit,
            float_reset,
            float_fit_device,
            scrollbars_btn,
        )
    }

    /// Creates the empty state overlay shown when no shapes exist.
    /// Returns (empty_box, new_btn, open_btn, svg_btn, dxf_btn, stl_btn)
    #[allow(clippy::type_complexity)]
    pub(crate) fn create_empty_state(
        settings_controller: &Rc<SettingsController>,
    ) -> (
        Box,
        gtk4::Button,
        gtk4::Button,
        gtk4::Button,
        gtk4::Button,
        gtk4::Button,
    ) {
        let empty_box = Box::new(Orientation::Vertical, 8);
        empty_box.add_css_class("visualizer-osd");
        empty_box.set_halign(gtk4::Align::Center);
        empty_box.set_valign(gtk4::Align::Center);
        empty_box.set_margin_start(20);
        empty_box.set_margin_end(20);
        empty_box.set_margin_top(20);
        empty_box.set_margin_bottom(20);
        empty_box.append(&Label::new(Some(&t!("No shapes yet"))));
        empty_box.append(&Label::new(Some(&t!(
            "Use the toolbox on the left to start drawing."
        ))));

        let empty_actions = Box::new(Orientation::Horizontal, 8);
        empty_actions.set_halign(gtk4::Align::Center);

        let empty_new_btn = gtk4::Button::with_label(&t!("New"));
        empty_new_btn.add_css_class("suggested-action");
        let empty_open_btn = gtk4::Button::with_label(&t!("Load Design"));
        let empty_import_svg_btn = gtk4::Button::with_label(&t!("Import SVG"));
        let empty_import_dxf_btn = gtk4::Button::with_label(&t!("Import DXF"));
        let empty_import_stl_btn = gtk4::Button::with_label(&t!("Import STL"));

        let enable_stl_import = settings_controller
            .persistence
            .borrow()
            .config()
            .ui
            .enable_stl_import;
        empty_import_stl_btn.set_visible(enable_stl_import);

        empty_actions.append(&empty_new_btn);
        empty_actions.append(&empty_open_btn);
        empty_actions.append(&empty_import_svg_btn);
        empty_actions.append(&empty_import_dxf_btn);
        empty_actions.append(&empty_import_stl_btn);
        empty_box.append(&empty_actions);
        empty_box.set_visible(true);

        (
            empty_box,
            empty_new_btn,
            empty_open_btn,
            empty_import_svg_btn,
            empty_import_dxf_btn,
            empty_import_stl_btn,
        )
    }

    /// Creates the status panel for the bottom-left of the canvas.
    /// Returns (status_box, status_label, units_badge)
    pub(crate) fn create_status_panel() -> (Box, Label, Label) {
        let status_box = Box::new(Orientation::Horizontal, 4);
        status_box.add_css_class("visualizer-osd");
        status_box.set_halign(gtk4::Align::Start);
        status_box.set_valign(gtk4::Align::End);
        status_box.set_margin_bottom(20);
        status_box.set_margin_start(20);

        let status_label_osd = Label::builder().label(" ").build();
        status_label_osd.set_hexpand(true);

        let units_badge = Label::new(Some(""));
        units_badge.add_css_class("osd-units-badge");

        status_box.append(&status_label_osd);
        status_box.append(&units_badge);

        (status_box, status_label_osd, units_badge)
    }

    /// Starts the status OSD update loop
    pub(crate) fn start_status_update_loop(
        status_label: Label,
        units_badge: Label,
        empty_box: Box,
        canvas: Rc<DesignerCanvas>,
        settings_controller: Rc<SettingsController>,
    ) {
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let state = canvas.state.borrow();
            let zoom = state.canvas.zoom();
            let pan_x = state.canvas.pan_x();
            let pan_y = state.canvas.pan_y();
            let has_shapes = !state.canvas.shape_store.is_empty();
            let snap_on = state.snap_enabled;
            drop(state);

            let constraint_on = *canvas.shift_pressed.borrow();

            let width = canvas.widget.width() as f64;
            let height = canvas.widget.height() as f64;

            let center_x = ((width / 2.0) - pan_x) / zoom;
            let center_y = ((height / 2.0) - pan_y) / zoom;

            let (cursor_x, cursor_y) = *canvas.mouse_pos.borrow();

            let system = settings_controller
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system;
            let mut status = format_zoom_center_cursor(
                zoom,
                center_x as f32,
                center_y as f32,
                cursor_x as f32,
                cursor_y as f32,
                system,
            );

            if snap_on || constraint_on {
                let mut parts: Vec<String> = Vec::new();
                if snap_on {
                    parts.push(t!("Snap"));
                }
                if constraint_on {
                    parts.push(t!("Constraint"));
                }
                status.push_str(&format!("  {}", parts.join(" / ")));
            }

            status_label.set_text(&status);
            units_badge.set_text(gcodekit5_core::units::get_unit_label(system));
            empty_box.set_visible(!has_shapes);

            gtk4::glib::ControlFlow::Continue
        });
    }
}
