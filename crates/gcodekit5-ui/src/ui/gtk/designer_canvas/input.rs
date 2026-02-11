//! Input handling and event processing for the designer canvas

use super::*;
use gcodekit5_designer::font_manager;
use gcodekit5_designer::model::{
    DesignCircle as Circle, DesignEllipse as Ellipse, DesignLine as Line, DesignPath as PathShape,
    DesignPolygon as Polygon, DesignRectangle as Rectangle, DesignText as TextShape,
    DesignTriangle as Triangle, Point, Shape,
};
use gtk4::prelude::*;
use gtk4::{
    Box, Button, CheckButton, Dialog, DropDown, Entry, Grid, Label, Orientation, Popover,
    PositionType, ResponseType, Separator, StringList,
};

impl DesignerCanvas {
    pub fn zoom_in(&self) {
        let mut state = self.state.borrow_mut();
        let current_zoom = state.canvas.zoom();
        state.canvas.set_zoom(current_zoom * 1.2);
        drop(state);
        self.widget.queue_draw();
    }

    pub fn zoom_out(&self) {
        let mut state = self.state.borrow_mut();
        let current_zoom = state.canvas.zoom();
        state.canvas.set_zoom(current_zoom / 1.2);
        drop(state);
        self.widget.queue_draw();
    }

    pub fn reset_view(&self) {
        let (pan_x, pan_y) = {
            let mut state = self.state.borrow_mut();
            state.canvas.set_zoom(1.0);
            state.canvas.set_pan(0.0, 0.0);
            (state.canvas.pan_x(), state.canvas.pan_y())
        };

        if let Some(adj) = self.hadjustment.borrow().as_ref() {
            adj.set_value(-pan_x);
        }
        if let Some(adj) = self.vadjustment.borrow().as_ref() {
            adj.set_value(pan_y);
        }

        self.widget.queue_draw();
    }

    pub fn zoom_fit(&self) {
        let (target_pan_x, target_pan_y) = {
            let mut state = self.state.borrow_mut();

            // Calculate bounds of all shapes
            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;

            let mut has_shapes = false;
            for obj in state.canvas.shapes() {
                has_shapes = true;
                let (sx, sy, ex, ey) = obj.get_total_bounds();
                min_x = min_x.min(sx);
                min_y = min_y.min(sy);
                max_x = max_x.max(ex);
                max_y = max_y.max(ey);
            }

            if has_shapes {
                // Fit content using the viewport fit-to-bounds (5% padding)
                state.canvas.fit_to_bounds(
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    core_constants::VIEW_PADDING,
                );
            } else {
                // No shapes: attempt device profile bounds, fallback to 250x250 mm
                let (min_x, min_y, max_x, max_y) = if let Some(dm) = &self.device_manager {
                    if let Some(profile) = dm.get_active_profile() {
                        (
                            profile.x_axis.min,
                            profile.y_axis.min,
                            profile.x_axis.max,
                            profile.y_axis.max,
                        )
                    } else {
                        (
                            0.0,
                            0.0,
                            core_constants::DEFAULT_WORK_WIDTH_MM,
                            core_constants::DEFAULT_WORK_HEIGHT_MM,
                        )
                    }
                } else {
                    (
                        0.0,
                        0.0,
                        core_constants::DEFAULT_WORK_WIDTH_MM,
                        core_constants::DEFAULT_WORK_HEIGHT_MM,
                    )
                };

                state.canvas.fit_to_bounds(
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    core_constants::VIEW_PADDING,
                );
            }

            (state.canvas.pan_x(), state.canvas.pan_y())
        };

        // Update adjustments safely
        if let Some(adj) = self.hadjustment.borrow().as_ref() {
            adj.set_value(-target_pan_x);
        }
        if let Some(adj) = self.vadjustment.borrow().as_ref() {
            adj.set_value(target_pan_y);
        }

        self.widget.queue_draw();
    }

    fn copy_cursor_coordinates(&self) {
        let (x, y) = *self.mouse_pos.borrow();
        let text = format!("X {:.2} mm  Y {:.2} mm", x, y);
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&text);
        }
    }

    fn toggle_grid(&self) {
        let mut state = self.state.borrow_mut();
        state.show_grid = !state.show_grid;
        drop(state);
        self.widget.queue_draw();
    }

    fn toggle_snap(&self) {
        let mut state = self.state.borrow_mut();
        state.snap_enabled = !state.snap_enabled;
    }

    fn toggle_toolpaths(&self) {
        let mut state = self.state.borrow_mut();
        state.show_toolpaths = !state.show_toolpaths;
        let show = state.show_toolpaths;
        drop(state);
        if show {
            self.generate_preview_toolpaths();
        } else {
            self.widget.queue_draw();
        }
    }

    /// Public method to fit to device working area from DesignerView
    pub(super) fn handle_right_click(&self, _x: f64, _y: f64) {
        // Check if we are actively building a polyline (tool is polyline AND we have points)
        let current_tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);
        let is_polyline_mode = matches!(current_tool, DesignerTool::Polyline);

        {
            let mut points = self.polyline_points.borrow_mut();
            if is_polyline_mode && !points.is_empty() {
                tracing::info!("Polyline mode with points - finishing polyline");
                if points.len() >= 2 {
                    // Create polyline
                    let path_shape = PathShape::from_points(&points, false); // Open polyline
                    let shape = Shape::Path(path_shape);

                    let mut state = self.state.borrow_mut();
                    state.add_shape_with_undo(shape);
                    drop(state);

                    // Refresh layers panel
                    if let Some(layers_panel) = self.layers.borrow().as_ref() {
                        layers_panel.refresh(&self.state);
                    }
                }
                points.clear();
                self.widget.queue_draw();
                return;
            }
        }

        let state_borrow = self.state.borrow();
        let has_selection = state_borrow
            .canvas
            .selection_manager
            .selected_count(&state_borrow.canvas.shape_store)
            > 0;

        // Only show context menu if we have any selected shapes
        if !has_selection {
            drop(state_borrow);
            return;
        }

        let selected_count = state_borrow.canvas.shapes().filter(|s| s.selected).count();
        let can_paste = !state_borrow.clipboard.is_empty();
        let can_group = state_borrow.can_group();
        let can_ungroup = state_borrow.can_ungroup();
        let can_align = selected_count >= 2;
        let can_boolean = selected_count >= 2;
        drop(state_borrow);

        let menu = Popover::new();
        menu.set_parent(&self.widget);
        menu.set_has_arrow(false);
        menu.set_autohide(true);

        // Set preferred position but allow GTK to adjust if needed
        let rect = gtk4::gdk::Rectangle::new(_x as i32 - 5, _y as i32 - 5, 10, 10);
        menu.set_pointing_to(Some(&rect));
        menu.set_position(PositionType::Bottom); // Prefer bottom-right, but allow adjustment

        // Create menu content
        let menu_box = Box::new(Orientation::Vertical, 0);
        menu_box.add_css_class("context-menu");

        // Always available actions
        let cut_button = Button::with_label("Cut");
        let copy_button = Button::with_label("Copy");
        let delete_button = Button::with_label("Delete");

        menu_box.append(&cut_button);
        menu_box.append(&copy_button);
        if can_paste {
            let paste_button = Button::with_label("Paste");
            menu_box.append(&paste_button);
        }
        menu_box.append(&delete_button);

        // Conditional actions - only show if applicable
        if can_group {
            let separator = Separator::new(Orientation::Horizontal);
            separator.add_css_class("menu-separator");
            menu_box.append(&separator);

            let group_button = Button::with_label("Group");
            menu_box.append(&group_button);
        }

        if can_ungroup {
            if !can_group {
                // Add separator if not already added
                let separator = Separator::new(Orientation::Horizontal);
                separator.add_css_class("menu-separator");
                menu_box.append(&separator);
            }
            let ungroup_button = Button::with_label("Ungroup");
            menu_box.append(&ungroup_button);
        }

        if can_align {
            let separator = Separator::new(Orientation::Horizontal);
            separator.add_css_class("menu-separator");
            menu_box.append(&separator);

            let align_button = Button::with_label("Align");
            menu_box.append(&align_button);
        }

        if can_boolean {
            let separator = Separator::new(Orientation::Horizontal);
            separator.add_css_class("menu-separator");
            menu_box.append(&separator);

            let union_button = Button::with_label("Union");
            let difference_button = Button::with_label("Difference");
            let intersection_button = Button::with_label("Intersection");
            menu_box.append(&union_button);
            menu_box.append(&difference_button);
            menu_box.append(&intersection_button);
        }

        menu.set_child(Some(&menu_box));
        menu.present();

        // Don't constrain position at all - let GTK place it wherever it fits

        let vbox = Box::new(Orientation::Vertical, 0);
        vbox.add_css_class("context-menu");

        // Helper to create menu items
        let create_item = |label: &str, action: &str| {
            let btn = gtk4::Button::builder()
                .label(label)
                .has_frame(false)
                .halign(gtk4::Align::Start)
                .build();

            let canvas = self.clone();
            let menu_clone = menu.clone();
            let action_name = action.to_string();

            btn.connect_clicked(move |_| {
                menu_clone.popdown();
                match action_name.as_str() {
                    "fit_content" => canvas.zoom_fit(),
                    "fit_viewport" => canvas.reset_view(),
                    "fit_device" => {
                        canvas.fit_to_device_area();
                        canvas.widget.queue_draw();
                    }
                    "copy_cursor" => canvas.copy_cursor_coordinates(),
                    "toggle_grid" => canvas.toggle_grid(),
                    "toggle_snap" => canvas.toggle_snap(),
                    "toggle_toolpaths" => canvas.toggle_toolpaths(),
                    "cut" => canvas.cut(),
                    "copy" => canvas.copy_selected(),
                    "paste" => canvas.paste(),
                    "delete" => canvas.delete_selected(),
                    "group" => canvas.group_selected(),
                    "ungroup" => canvas.ungroup_selected(),
                    "boolean_union" => canvas.boolean_union(),
                    "boolean_difference" => canvas.boolean_difference(),
                    "boolean_intersection" => canvas.boolean_intersection(),
                    "convert_to_path" => canvas.convert_to_path(),
                    "convert_to_rectangle" => canvas.convert_to_rectangle(),
                    "mirror_x" => canvas.mirror_x(),
                    "mirror_y" => canvas.mirror_y(),
                    _ => {}
                }
            });

            btn
        };

        // Edit - only show items that are actionable
        vbox.append(&create_item("Cut", "cut"));
        vbox.append(&create_item("Copy", "copy"));
        if can_paste {
            vbox.append(&create_item("Paste", "paste"));
        }
        vbox.append(&create_item("Delete", "delete"));

        vbox.append(&Separator::new(Orientation::Horizontal));

        if can_group {
            vbox.append(&create_item("Group", "group"));
        }
        if can_ungroup {
            vbox.append(&create_item("Ungroup", "ungroup"));
        }

        if can_group || can_ungroup {
            vbox.append(&Separator::new(Orientation::Horizontal));
        }

        if can_boolean {
            vbox.append(&create_item("Union", "boolean_union"));
            vbox.append(&create_item("Diff", "boolean_difference"));
            vbox.append(&create_item("Inter", "boolean_intersection"));
            vbox.append(&Separator::new(Orientation::Horizontal));
        }

        vbox.append(&create_item("Mirror on X", "mirror_x"));
        vbox.append(&create_item("Mirror on Y", "mirror_y"));

        // Rotate menu is always shown since we have a selection
        {
            let rotate_btn = gtk4::Button::builder()
                .label("Rotate ▸")
                .has_frame(false)
                .halign(gtk4::Align::Start)
                .build();

            let rotate_menu = Popover::new();
            rotate_menu.set_parent(&rotate_btn);
            rotate_menu.set_has_arrow(false);
            rotate_menu.set_position(gtk4::PositionType::Right);

            let rotate_vbox = Box::new(Orientation::Vertical, 0);
            rotate_vbox.add_css_class("context-menu");

            // Helper for rotate items
            let create_rotate_item = |label: &str, angle_degrees: f64| {
                let btn = gtk4::Button::builder()
                    .label(label)
                    .has_frame(false)
                    .halign(gtk4::Align::Start)
                    .build();

                let canvas = self.clone();
                let menu_clone = menu.clone(); // Main menu
                let rotate_menu_clone = rotate_menu.clone();
                let angle_radians = angle_degrees.to_radians();

                btn.connect_clicked(move |_| {
                    rotate_menu_clone.popdown();
                    menu_clone.popdown();
                    canvas.set_selected_rotation(angle_radians);
                });
                btn
            };

            rotate_vbox.append(&create_rotate_item("90° CW", -90.0));
            rotate_vbox.append(&create_rotate_item("90° CCW", 90.0));
            rotate_vbox.append(&create_rotate_item("45° CW", -45.0));
            rotate_vbox.append(&create_rotate_item("45° CCW", 45.0));
            rotate_vbox.append(&create_rotate_item("180°", 180.0));

            rotate_menu.set_child(Some(&rotate_vbox));

            rotate_btn.connect_clicked(move |_| {
                rotate_menu.popup();
            });

            vbox.append(&rotate_btn);
        }

        if can_align {
            let align_btn = gtk4::Button::builder()
                .label("Align ▸")
                .has_frame(false)
                .halign(gtk4::Align::Start)
                .build();

            let align_menu = Popover::new();
            align_menu.set_parent(&align_btn);
            align_menu.set_has_arrow(false);
            align_menu.set_position(gtk4::PositionType::Right);

            let align_vbox = Box::new(Orientation::Vertical, 0);
            align_vbox.add_css_class("context-menu");

            // Helper for align items
            let create_align_item = |label: &str, action: &str| {
                let btn = gtk4::Button::builder()
                    .label(label)
                    .has_frame(false)
                    .halign(gtk4::Align::Start)
                    .build();

                let canvas = self.clone();
                let menu_clone = menu.clone(); // Main menu
                let align_menu_clone = align_menu.clone();
                let action_name = action.to_string();

                btn.connect_clicked(move |_| {
                    align_menu_clone.popdown();
                    menu_clone.popdown();
                    match action_name.as_str() {
                        "align_left" => canvas.align_left(),
                        "align_right" => canvas.align_right(),
                        "align_top" => canvas.align_top(),
                        "align_bottom" => canvas.align_bottom(),
                        "align_center_x" => canvas.align_center_horizontal(),
                        "align_center_y" => canvas.align_center_vertical(),
                        _ => {}
                    }
                });
                btn
            };

            align_vbox.append(&create_align_item("Align Left", "align_left"));
            align_vbox.append(&create_align_item("Align Right", "align_right"));
            align_vbox.append(&create_align_item("Align Top", "align_top"));
            align_vbox.append(&create_align_item("Align Bottom", "align_bottom"));
            align_vbox.append(&create_align_item("Align Center X", "align_center_x"));
            align_vbox.append(&create_align_item("Align Center Y", "align_center_y"));

            align_menu.set_child(Some(&align_vbox));

            align_btn.connect_clicked(move |_| {
                align_menu.popup();
            });

            vbox.append(&align_btn);
        }

        vbox.append(&Separator::new(Orientation::Horizontal));
        vbox.append(&create_item("Convert to Path", "convert_to_path"));
        vbox.append(&create_item("Convert to Rectangle", "convert_to_rectangle"));

        menu.set_child(Some(&vbox));
        menu.popup();
    }

    fn snap_canvas_point(&self, x: f64, y: f64) -> (f64, f64) {
        let state = self.state.borrow();
        if !state.snap_enabled {
            return (x, y);
        }
        let spacing = state.grid_spacing_mm;
        if spacing <= 0.0 {
            return (x, y);
        }
        let threshold = state.snap_threshold_mm.max(0.0);

        let sx = (x / spacing).round() * spacing;
        let sy = (y / spacing).round() * spacing;

        let out_x = if (sx - x).abs() <= threshold { sx } else { x };
        let out_y = if (sy - y).abs() <= threshold { sy } else { y };
        (out_x, out_y)
    }

    fn open_text_tool_dialog(&self, canvas_x: f64, canvas_y: f64) {
        *self.text_tool_pending_pos.borrow_mut() = Some((canvas_x, canvas_y));

        if self.text_tool_dialog.borrow().is_none() {
            let dialog = Dialog::builder()
                .title(t!("Add Text"))
                .modal(true)
                .resizable(false)
                .build();
            dialog.add_button(&t!("Cancel"), ResponseType::Cancel);
            dialog.add_button(&t!("Add"), ResponseType::Ok);
            dialog.set_default_response(ResponseType::Ok);

            let content = Box::new(Orientation::Vertical, 10);
            content.set_margin_top(12);
            content.set_margin_bottom(12);
            content.set_margin_start(12);
            content.set_margin_end(12);

            let header = Label::new(Some(&t!("Text")));
            header.add_css_class("title-3");
            header.set_halign(gtk4::Align::Start);
            content.append(&header);

            let entry = Entry::new();
            entry.set_placeholder_text(Some(&t!("Enter text")));
            entry.set_activates_default(true);
            content.append(&entry);

            // Font + attributes
            let grid = Grid::builder().row_spacing(8).column_spacing(8).build();

            let font_label = Label::new(Some(&t!("Font:")));
            font_label.set_halign(gtk4::Align::Start);

            let font_model = StringList::new(&[]);
            font_model.append("Sans");
            for fam in font_manager::list_font_families() {
                if fam != "Sans" {
                    font_model.append(&fam);
                }
            }
            let font_combo = DropDown::new(Some(font_model), None::<gtk4::Expression>);
            font_combo.set_hexpand(true);

            let size_label = Label::new(Some(&t!("Size:")));
            size_label.set_halign(gtk4::Align::Start);
            let size_entry = Entry::new();
            size_entry.set_hexpand(true);
            let size_unit = Label::new(Some("pt"));
            size_unit.set_width_chars(4);
            size_unit.set_halign(gtk4::Align::End);
            size_unit.set_xalign(1.0);

            let bold_check = CheckButton::with_label(&t!("Bold"));
            let italic_check = CheckButton::with_label(&t!("Italic"));
            let style_box = Box::new(Orientation::Horizontal, 8);
            style_box.append(&bold_check);
            style_box.append(&italic_check);

            let style_label = Label::new(Some(&t!("Style:")));
            style_label.set_halign(gtk4::Align::Start);

            grid.attach(&font_label, 0, 0, 1, 1);
            grid.attach(&font_combo, 1, 0, 2, 1);
            grid.attach(&size_label, 0, 1, 1, 1);
            grid.attach(&size_entry, 1, 1, 1, 1);
            grid.attach(&size_unit, 2, 1, 1, 1);
            grid.attach(&style_label, 0, 2, 1, 1);
            grid.attach(&style_box, 1, 2, 2, 1);

            content.append(&grid);

            dialog.content_area().append(&content);

            let canvas = self.clone();
            let entry_clone = entry.clone();
            let font_combo_clone = font_combo.clone();
            let bold_clone = bold_check.clone();
            let italic_clone = italic_check.clone();
            let size_entry_clone = size_entry.clone();

            dialog.connect_response(move |d, resp| {
                if resp == ResponseType::Ok {
                    let text = entry_clone.text().trim().to_string();
                    if !text.is_empty() {
                        if let Some((x, y)) = *canvas.text_tool_pending_pos.borrow() {
                            let family = font_combo_clone
                                .selected_item()
                                .and_downcast::<gtk4::StringObject>()
                                .map(|s| s.string().to_string())
                                .unwrap_or_else(|| "Sans".to_string());
                            let bold = bold_clone.is_active();
                            let italic = italic_clone.is_active();
                            let size_mm = parse_font_points_mm(&size_entry_clone.text())
                                .unwrap_or_else(|| pt_to_mm(20.0));

                            *canvas.text_tool_last_font_family.borrow_mut() = family.clone();
                            *canvas.text_tool_last_bold.borrow_mut() = bold;
                            *canvas.text_tool_last_italic.borrow_mut() = italic;
                            *canvas.text_tool_last_size_mm.borrow_mut() = size_mm;

                            let mut state = canvas.state.borrow_mut();
                            let mut shape = TextShape::new(text, x, y, size_mm);
                            shape.font_family = family;
                            shape.bold = bold;
                            shape.italic = italic;
                            let id = state.add_shape_with_undo(Shape::Text(shape));
                            state.canvas.deselect_all();
                            state.canvas.select_shape(id, false);
                            drop(state);

                            canvas.widget.queue_draw();

                            if let Some(ref props) = *canvas.properties.borrow() {
                                props.update_from_selection();
                            }
                            if let Some(ref layers) = *canvas.layers.borrow() {
                                layers.refresh(&canvas.state);
                            }
                        }
                    }
                }

                entry_clone.set_text("");
                d.hide();
            });

            *self.text_tool_dialog.borrow_mut() = Some((
                dialog,
                entry,
                font_combo,
                bold_check,
                italic_check,
                size_entry,
            ));
        }

        if let Some((dialog, entry, font_combo, bold_check, italic_check, size_entry)) =
            self.text_tool_dialog.borrow().as_ref()
        {
            if let Some(root) = self.widget.root() {
                if let Ok(win) = root.downcast::<gtk4::Window>() {
                    dialog.set_transient_for(Some(&win));
                }
            }

            // Restore last-used values
            let family = self.text_tool_last_font_family.borrow().clone();
            let mut family_idx = 0;
            if let Some(model) = font_combo.model().and_downcast::<gtk4::StringList>() {
                for i in 0..model.n_items() {
                    if let Some(obj) = model.item(i) {
                        if let Ok(s) = obj.downcast::<gtk4::StringObject>() {
                            if s.string() == family {
                                family_idx = i;
                                break;
                            }
                        }
                    }
                }
            }
            font_combo.set_selected(family_idx);
            bold_check.set_active(*self.text_tool_last_bold.borrow());
            italic_check.set_active(*self.text_tool_last_italic.borrow());
            size_entry.set_text(&format_font_points(*self.text_tool_last_size_mm.borrow()));

            dialog.present();
            entry.grab_focus();
        }
    }

    pub(super) fn handle_click(&self, x: f64, y: f64, ctrl_pressed_arg: bool, n_press: i32) {
        // Combine gesture modifier state with tracked keyboard state for reliability
        let ctrl_pressed = ctrl_pressed_arg || *self.ctrl_pressed.borrow();

        // Reset drag flag
        *self.did_drag.borrow_mut() = false;

        // Clear properties panel focus when user clicks on canvas
        if let Some(ref props) = *self.properties.borrow() {
            props.clear_focus();
        }

        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);

        // Convert screen coords to canvas coords
        let _width = self.widget.width() as f64;
        let height = self.widget.height() as f64;

        let state = self.state.borrow();
        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();
        drop(state);

        let y_flipped = height - y;
        let raw_canvas_x = (x - pan_x) / zoom;
        let raw_canvas_y = (y_flipped - pan_y) / zoom;
        let (snapped_x, snapped_y) = self.snap_canvas_point(raw_canvas_x, raw_canvas_y);

        // Use raw coordinates for selection to ensure we can click handles/shapes even if they are off-grid.
        // Use snapped coordinates for drawing tools.
        let (canvas_x, canvas_y) = if tool == DesignerTool::Select {
            (raw_canvas_x, raw_canvas_y)
        } else {
            (snapped_x, snapped_y)
        };

        match tool {
            DesignerTool::Select => {
                // Handle selection
                let mut state = self.state.borrow_mut();
                let point = Point::new(canvas_x, canvas_y);

                // If the click is on a resize handle for the current selection, do not
                // change selection here. Handles extend outside shapes, and a normal
                // empty-space click would deselect and prevent resizing.
                let selected_count = state.canvas.shapes().filter(|s| s.selected).count();
                if selected_count > 0 {
                    let bounds_opt = if selected_count > 1 {
                        let mut min_x = f64::INFINITY;
                        let mut min_y = f64::INFINITY;
                        let mut max_x = f64::NEG_INFINITY;
                        let mut max_y = f64::NEG_INFINITY;
                        let mut any = false;

                        for obj in state.canvas.shapes().filter(|s| s.selected) {
                            let (x1, y1, x2, y2) = Self::selection_bounds(&obj.shape);
                            min_x = min_x.min(x1);
                            min_y = min_y.min(y1);
                            max_x = max_x.max(x2);
                            max_y = max_y.max(y2);
                            any = true;
                        }

                        if any {
                            Some((min_x, min_y, max_x, max_y))
                        } else {
                            None
                        }
                    } else if let Some(selected_id) = state.canvas.selection_manager.selected_id() {
                        state
                            .canvas
                            .shapes()
                            .find(|s| s.id == selected_id)
                            .map(|obj| Self::selection_bounds(&obj.shape))
                    } else {
                        None
                    };

                    if let Some(bounds) = bounds_opt {
                        if self
                            .get_resize_handle_at(canvas_x, canvas_y, &bounds, zoom)
                            .is_some()
                        {
                            return;
                        }
                    }
                }

                // Check if we clicked on an existing shape
                let mut clicked_shape_id = None;
                let tolerance = 3.0;
                for obj in state.canvas.shapes() {
                    if obj.contains_point(&point, tolerance) {
                        clicked_shape_id = Some(obj.id);
                    }
                }

                if let Some(id) = clicked_shape_id {
                    // Check if it's already selected
                    let is_selected = state.canvas.selection_manager.selected_id() == Some(id)
                        || state.canvas.shapes().any(|s| s.id == id && s.selected);

                    if is_selected && !ctrl_pressed {
                        // Clicked on already selected item, and no Ctrl.
                        // Do NOT change selection yet. Wait for release.
                        // This allows dragging the current selection group.
                        return;
                    }
                }

                // Try to select shape at click point with multi-select if Ctrl is held
                if let Some(_selected_id) = state.canvas.select_at(&point, tolerance, ctrl_pressed)
                {
                    // Shape selected
                } else if !ctrl_pressed {
                    // Click on empty space without Ctrl - deselect all
                    state.canvas.deselect_all();
                }

                drop(state);
                self.widget.queue_draw();

                // Update properties panel
                if let Some(ref props) = *self.properties.borrow() {
                    props.update_from_selection();
                }

                // Update layers panel
                if let Some(ref layers) = *self.layers.borrow() {
                    layers.refresh(&self.state);
                }
            }
            DesignerTool::Polyline => {
                if n_press == 2 {
                    // Double click - finish
                    let mut points = self.polyline_points.borrow_mut();
                    if points.len() >= 2 {
                        // Create polyline
                        let path_shape = PathShape::from_points(&points, false);
                        let shape = Shape::Path(path_shape);

                        let mut state = self.state.borrow_mut();
                        state.add_shape_with_undo(shape);
                        drop(state);
                    }
                    points.clear();
                    self.widget.queue_draw();
                } else {
                    let mut points = self.polyline_points.borrow_mut();
                    points.push(Point::new(canvas_x, canvas_y));
                    drop(points);
                    self.widget.queue_draw();
                }
            }
            DesignerTool::Text => {
                // Click-to-place text.
                self.open_text_tool_dialog(canvas_x, canvas_y);
            }
            _ => {
                // Other tools handled by drag
            }
        }
    }

    pub(super) fn handle_release(&self, x: f64, y: f64, ctrl_pressed_arg: bool) {
        let ctrl_pressed = ctrl_pressed_arg || *self.ctrl_pressed.borrow();

        if *self.did_drag.borrow() {
            return;
        }

        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);

        // Convert screen coords to canvas coords
        let _width = self.widget.width() as f64;
        let height = self.widget.height() as f64;

        let state = self.state.borrow();
        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();
        drop(state);

        let y_flipped = height - y;
        let raw_canvas_x = (x - pan_x) / zoom;
        let raw_canvas_y = (y_flipped - pan_y) / zoom;
        let (snapped_x, snapped_y) = self.snap_canvas_point(raw_canvas_x, raw_canvas_y);

        // Use raw coordinates for selection to ensure we can click handles/shapes even if they are off-grid.
        // Use snapped coordinates for drawing tools.
        let (canvas_x, canvas_y) = if tool == DesignerTool::Select {
            (raw_canvas_x, raw_canvas_y)
        } else {
            (snapped_x, snapped_y)
        };

        if tool == DesignerTool::Select {
            let mut state = self.state.borrow_mut();
            let point = Point::new(canvas_x, canvas_y);

            // Check if we clicked on an existing shape
            let mut clicked_shape_id = None;
            let tolerance = 3.0;
            for obj in state.canvas.shapes() {
                if obj.contains_point(&point, tolerance) {
                    clicked_shape_id = Some(obj.id);
                }
            }

            if let Some(id) = clicked_shape_id {
                let is_selected = state.canvas.shapes().any(|s| s.id == id && s.selected);

                if is_selected && !ctrl_pressed {
                    // We clicked on a selected item and didn't drag.
                    // Now we select ONLY this item (deselect others).
                    state.canvas.deselect_all();
                    state.canvas.select_shape(id, false);

                    drop(state);
                    self.widget.queue_draw();

                    // Update properties panel
                    if let Some(ref props) = *self.properties.borrow() {
                        props.update_from_selection();
                    }

                    // Update layers panel
                    if let Some(ref layers) = *self.layers.borrow() {
                        layers.refresh(&self.state);
                    }
                }
            }
        }
    }

    pub(super) fn handle_drag_begin(&self, x: f64, y: f64) {
        // Set drag flag
        *self.did_drag.borrow_mut() = true;

        // Clear properties panel focus when user drags on canvas
        if let Some(ref props) = *self.properties.borrow() {
            props.clear_focus();
        }

        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);

        // Convert screen coords to canvas coords
        let _width = self.widget.width() as f64;
        let height = self.widget.height() as f64;

        let state = self.state.borrow();
        let zoom = state.canvas.zoom();
        let pan_x = state.canvas.pan_x();
        let pan_y = state.canvas.pan_y();
        drop(state);

        let y_flipped = height - y;
        let raw_canvas_x = (x - pan_x) / zoom;
        let raw_canvas_y = (y_flipped - pan_y) / zoom;
        let (snapped_x, snapped_y) = self.snap_canvas_point(raw_canvas_x, raw_canvas_y);

        // Use raw coordinates for selection to ensure we can click handles/shapes even if they are off-grid.
        // Use snapped coordinates for drawing tools.
        let (canvas_x, canvas_y) = if tool == DesignerTool::Select {
            (raw_canvas_x, raw_canvas_y)
        } else {
            (snapped_x, snapped_y)
        };

        match tool {
            DesignerTool::Select => {
                // Check if we're clicking on a resize handle first
                let (selected_id_opt, bounds_opt, is_group_resize) = {
                    let state = self.state.borrow();

                    let selected_count = state.canvas.shapes().filter(|s| s.selected).count();
                    if selected_count > 1 {
                        let mut min_x = f64::INFINITY;
                        let mut min_y = f64::INFINITY;
                        let mut max_x = f64::NEG_INFINITY;
                        let mut max_y = f64::NEG_INFINITY;
                        let mut any = false;

                        for obj in state.canvas.shapes().filter(|s| s.selected) {
                            let (x1, y1, x2, y2) = Self::selection_bounds(&obj.shape);
                            min_x = min_x.min(x1);
                            min_y = min_y.min(y1);
                            max_x = max_x.max(x2);
                            max_y = max_y.max(y2);
                            any = true;
                        }

                        if any {
                            (Some(0u64), Some((min_x, min_y, max_x, max_y)), true)
                        } else {
                            (None, None, false)
                        }
                    } else if let Some(selected_id) = state.canvas.selection_manager.selected_id() {
                        if let Some(obj) = state.canvas.shapes().find(|s| s.id == selected_id) {
                            let bounds = Self::selection_bounds(&obj.shape);
                            (Some(selected_id), Some(bounds), false)
                        } else {
                            (None, None, false)
                        }
                    } else {
                        (None, None, false)
                    }
                };

                if let (Some(selected_id), Some(bounds)) = (selected_id_opt, bounds_opt) {
                    if let Some(handle) =
                        self.get_resize_handle_at(canvas_x, canvas_y, &bounds, zoom)
                    {
                        // Start resizing
                        *self.active_resize_handle.borrow_mut() = Some((handle, selected_id));
                        let (min_x, min_y, max_x, max_y) = bounds;
                        *self.resize_original_bounds.borrow_mut() =
                            Some((min_x, min_y, max_x - min_x, max_y - min_y));

                        // Snapshot original shapes so resizing doesn't compound on each drag update.
                        // This matters for group resize and for path/text scaling.
                        let originals: Vec<(u64, Shape)> = {
                            let state = self.state.borrow();
                            state
                                .canvas
                                .shapes()
                                .filter(|s| s.selected)
                                .map(|s| (s.id, s.shape.clone()))
                                .collect()
                        };
                        *self.resize_original_shapes.borrow_mut() = Some(originals);

                        *self.creation_start.borrow_mut() = Some((canvas_x, canvas_y));
                        if is_group_resize {
                            // For group resize, we keep moving behavior the same but scale on drag updates.
                        }
                        return;
                    }
                }

                // Check if clicking on a selected shape for moving
                let has_selected = {
                    let state = self.state.borrow();
                    state.canvas.selection_manager.selected_id().is_some()
                };

                if has_selected {
                    // Start dragging selected shapes
                    *self.creation_start.borrow_mut() = Some((canvas_x, canvas_y));
                    *self.last_drag_offset.borrow_mut() = (0.0, 0.0); // Reset offset tracker
                } else {
                    // Start selection rectangle (future implementation)
                    *self.creation_start.borrow_mut() = Some((canvas_x, canvas_y));
                }
            }
            DesignerTool::Pan => {
                *self.creation_start.borrow_mut() = Some((x, y)); // Screen coords for pan
                *self.last_drag_offset.borrow_mut() = (0.0, 0.0); // Reset offset tracker (offsets start at 0)
                self.widget.set_cursor_from_name(Some("grabbing"));
            }
            _ => {
                // Start shape creation
                *self.creation_start.borrow_mut() = Some((canvas_x, canvas_y));
                *self.creation_current.borrow_mut() = Some((canvas_x, canvas_y));
            }
        }
    }

    pub(super) fn handle_drag_update(&self, offset_x: f64, offset_y: f64) {
        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);
        let shift_pressed = *self.shift_pressed.borrow();

        // Get start point without holding the borrow
        let start_opt = *self.creation_start.borrow();

        if let Some(start) = start_opt {
            let state = self.state.borrow();
            let zoom = state.canvas.zoom();
            drop(state);

            // Convert offsets to canvas units
            let canvas_offset_x = offset_x / zoom;
            let canvas_offset_y = offset_y / zoom;

            // Update current position (offset is from drag start)
            let mut current_x = start.0 + canvas_offset_x;
            let mut current_y = start.1 - canvas_offset_y; // Flip Y offset

            // Apply Shift key constraints for creation
            if shift_pressed && tool != DesignerTool::Select {
                let dx = current_x - start.0;
                let dy = current_y - start.1;

                if tool == DesignerTool::Rectangle || tool == DesignerTool::Ellipse {
                    // Square/Circle constraint (1:1 aspect ratio)
                    let max_dim = dx.abs().max(dy.abs());
                    current_x = start.0 + max_dim * dx.signum();
                    current_y = start.1 + max_dim * dy.signum();
                } else if tool == DesignerTool::Line || tool == DesignerTool::Polyline {
                    // Snap to 45 degree increments
                    let angle = dy.atan2(dx);
                    let snap_angle = (angle / (std::f64::consts::PI / 4.0)).round()
                        * (std::f64::consts::PI / 4.0);
                    let dist = (dx * dx + dy * dy).sqrt();
                    current_x = start.0 + dist * snap_angle.cos();
                    current_y = start.1 + dist * snap_angle.sin();
                }
            }

            if tool != DesignerTool::Pan {
                (current_x, current_y) = self.snap_canvas_point(current_x, current_y);
                *self.creation_current.borrow_mut() = Some((current_x, current_y));
            }

            // If in select mode, handle resizing or moving
            if tool == DesignerTool::Select {
                // Check if we're resizing
                if let Some((handle, shape_id)) = *self.active_resize_handle.borrow() {
                    self.apply_resize(handle, shape_id, current_x, current_y, shift_pressed);
                } else {
                    let mut state = self.state.borrow_mut();
                    // Check if we have a selection - if so, move it; otherwise, marquee select
                    if state.canvas.selection_manager.selected_id().is_some() {
                        // Calculate delta from last update (incremental movement)
                        let last_offset = *self.last_drag_offset.borrow();
                        let mut delta_x = (offset_x - last_offset.0) / zoom;
                        let mut delta_y = (offset_y - last_offset.1) / zoom;

                        if shift_pressed {
                            // Constrain movement to X or Y axis based on total drag
                            let total_dx = current_x - start.0;
                            let total_dy = current_y - start.1;

                            if total_dx.abs() > total_dy.abs() {
                                delta_y = 0.0;
                            } else {
                                delta_x = 0.0;
                            }
                        }

                        // Apply incremental movement directly to canvas (without undo)
                        // We'll create the undo command when drag ends
                        state.canvas.move_selected(delta_x, -delta_y);

                        // Update last offset
                        *self.last_drag_offset.borrow_mut() = (offset_x, offset_y);
                    }
                    // Marquee selection is shown by the preview rectangle (handled in draw)
                }
            } else if tool == DesignerTool::Pan {
                // Handle panning
                // offset_x/y are total offsets from drag start.
                // We need incremental change from last frame.
                // But wait, handle_drag_update gives total offset from start.
                // In handle_drag_begin for Pan, I set last_drag_offset to (x, y) (start pos).
                // But offset_x/y here are relative to start.
                // So current screen pos = start + offset.
                // Previous screen pos = start + previous_offset.
                // Delta = current - previous.

                // Actually, let's use last_drag_offset to store the *previous offset value*.
                // In begin, offset is 0. So last_drag_offset = (0,0).

                let last_offset = *self.last_drag_offset.borrow();
                let delta_x = offset_x - last_offset.0;
                let delta_y = offset_y - last_offset.1;

                let mut state = self.state.borrow_mut();
                // Drag right (+x) -> move content right -> pan_x increases
                // Drag down (+y) -> move content down -> pan_y increases (because Y is flipped)
                // Wait, if I drag down, y increases.
                // Screen Y increases.
                // Content should move down on screen.
                // Content Y (world) is up.
                // Moving content down on screen means moving it to lower Y in world? No.
                // Screen Y = height - (world Y * zoom + pan Y).
                // If I want Screen Y to increase (move down), I can decrease pan Y?
                // height - (wy*z + (py - d)) = height - wy*z - py + d = old_sy + d.
                // So decreasing pan_y moves content down on screen.
                // So drag down (+dy) -> pan_y -= dy.

                // Let's verify with scrollbars.
                // Scrollbar down -> value increases.
                // v_adj value -> state.canvas.set_pan(px, val).
                // So pan_y increases.
                // If pan_y increases, Screen Y = height - (wy*z + py).
                // Screen Y decreases. Content moves UP.
                // So scrollbar down moves content UP. This is standard scrolling (view moves down).

                // Panning with hand: Drag UP -> content moves UP.
                // Drag UP means dy is negative.
                // If dy is negative, we want content to move UP (Screen Y decreases).
                // So pan_y should increase.
                // So pan_y -= dy (since dy is negative, -= is +=).

                // Drag DOWN means dy is positive.
                // We want content to move DOWN (Screen Y increases).
                // So pan_y should decrease.
                // So pan_y -= dy.

                // Drag RIGHT means dx is positive.
                // We want content to move RIGHT (Screen X increases).
                // Screen X = (wx * z + px).
                // So pan_x should increase.
                // So pan_x += dx.

                state.canvas.pan_by(delta_x, -delta_y);
                let new_pan_x = state.canvas.pan_x();
                let new_pan_y = state.canvas.pan_y();
                drop(state);

                // Update scrollbars
                if let Some(adj) = self.hadjustment.borrow().as_ref() {
                    adj.set_value(-new_pan_x);
                }
                if let Some(adj) = self.vadjustment.borrow().as_ref() {
                    adj.set_value(new_pan_y);
                }

                *self.last_drag_offset.borrow_mut() = (offset_x, offset_y);
            }

            self.widget.queue_draw();
        }
    }

    pub(super) fn handle_drag_end(&self, offset_x: f64, offset_y: f64) {
        let tool = self
            .toolbox
            .as_ref()
            .map(|t| t.current_tool())
            .unwrap_or(DesignerTool::Select);

        if tool == DesignerTool::Pan {
            *self.creation_start.borrow_mut() = None;
            *self.last_drag_offset.borrow_mut() = (0.0, 0.0);
            self.widget.set_cursor_from_name(Some("grab"));
            return;
        }

        // Get start point and release the borrow immediately
        let start_opt = *self.creation_start.borrow();

        if let Some(start) = start_opt {
            let state = self.state.borrow();
            let zoom = state.canvas.zoom();
            drop(state);

            let canvas_offset_x = offset_x / zoom;
            let canvas_offset_y = offset_y / zoom;

            let end_x = start.0 + canvas_offset_x;
            let end_y = start.1 - canvas_offset_y; // Flip Y offset

            match tool {
                DesignerTool::Select => {
                    // Check if we were resizing and need to create undo command
                    let was_resizing = self.active_resize_handle.borrow().is_some();
                    let resize_originals = self.resize_original_shapes.borrow().clone();

                    // Clear resize state
                    *self.active_resize_handle.borrow_mut() = None;
                    *self.resize_original_bounds.borrow_mut() = None;
                    *self.resize_original_shapes.borrow_mut() = None;

                    let mut state = self.state.borrow_mut();

                    // If we were resizing, create an undo command for the resize
                    if was_resizing {
                        if let Some(originals) = resize_originals {
                            let mut commands = Vec::new();

                            for (id, old_shape) in originals {
                                if let Some(obj) = state.canvas.get_shape(id) {
                                    if obj.selected {
                                        commands.push(gcodekit5_designer::commands::DesignerCommand::ResizeShape(
                                        gcodekit5_designer::commands::ResizeShape {
                                            id,
                                            handle: 0, // Not used in undo/redo
                                            dx: 0.0,   // Not used in undo/redo
                                            dy: 0.0,   // Not used in undo/redo
                                            old_shape: Some(old_shape),
                                            new_shape: Some(obj.shape.clone()),
                                        }
                                    ));
                                    }
                                }
                            }

                            if !commands.is_empty() {
                                let cmd =
                                    gcodekit5_designer::commands::DesignerCommand::CompositeCommand(
                                        gcodekit5_designer::commands::CompositeCommand {
                                            commands,
                                            name: "Resize Shapes".to_string(),
                                        },
                                    );
                                state.record_command(cmd);
                            }
                        }
                    }
                    // If we were moving (not resizing, not marquee selecting), create undo command
                    else if state.canvas.selection_manager.selected_id().is_some() {
                        let last_offset = *self.last_drag_offset.borrow();
                        if last_offset.0.abs() > 0.1 || last_offset.1.abs() > 0.1 {
                            // We moved - calculate total movement from start
                            let total_dx = canvas_offset_x;
                            let total_dy = -canvas_offset_y;

                            if total_dx.abs() > 0.01 || total_dy.abs() > 0.01 {
                                let ids: Vec<u64> = state
                                    .canvas
                                    .shapes()
                                    .filter(|s| s.selected)
                                    .map(|s| s.id)
                                    .collect();

                                if !ids.is_empty() {
                                    let cmd =
                                        gcodekit5_designer::commands::DesignerCommand::MoveShapes(
                                            gcodekit5_designer::commands::MoveShapes {
                                                ids,
                                                dx: total_dx,
                                                dy: total_dy,
                                            },
                                        );
                                    state.record_command(cmd);
                                }
                            }
                        }
                    }

                    // Reset drag offset
                    *self.last_drag_offset.borrow_mut() = (0.0, 0.0);

                    // If we didn't have a selection and we dragged, perform marquee selection
                    if state.canvas.selection_manager.selected_id().is_none() {
                        // Calculate selection rectangle
                        let min_x = start.0.min(end_x);
                        let max_x = start.0.max(end_x);
                        let min_y = start.1.min(end_y);
                        let max_y = start.1.max(end_y);

                        // Find all shapes intersecting the marquee rectangle
                        let selected_shapes: Vec<_> = state
                            .canvas
                            .shapes()
                            .filter(|obj| {
                                let (shape_min_x, shape_min_y, shape_max_x, shape_max_y) =
                                    obj.get_total_bounds();
                                // Check if bounding boxes intersect
                                !(shape_max_x < min_x
                                    || shape_min_x > max_x
                                    || shape_max_y < min_y
                                    || shape_min_y > max_y)
                            })
                            .map(|obj| obj.id)
                            .collect();

                        // Select the shapes
                        if !selected_shapes.is_empty() {
                            // Deselect all shapes first
                            for obj in state.canvas.shape_store.iter_mut() {
                                obj.selected = false;
                            }

                            // Then select the marquee-selected shapes
                            for &shape_id in &selected_shapes {
                                if let Some(shape) = state.canvas.shape_store.get_mut(shape_id) {
                                    shape.selected = true;
                                }
                            }

                            // Set primary selection to first selected shape
                            state
                                .canvas
                                .selection_manager
                                .set_selected_id(selected_shapes.first().copied());
                        }
                    }
                }
                _ => {
                    // Create the shape for drawing tools
                    self.create_shape(tool, start, (end_x, end_y));
                }
            }

            // Clear creation state (now safe - no borrows held)
            *self.creation_start.borrow_mut() = None;
            *self.creation_current.borrow_mut() = None;

            // Update properties panel after resize/move
            if let Some(ref props) = *self.properties.borrow() {
                props.update_from_selection();
            }

            // Update toolpaths if enabled
            let show_toolpaths = self.state.borrow().show_toolpaths;
            if show_toolpaths {
                self.generate_preview_toolpaths();
            }

            // Queue draw after clearing state
            self.widget.queue_draw();
        }
    }

    fn create_shape(&self, tool: DesignerTool, start: (f64, f64), end: (f64, f64)) {
        // Scope the borrow to release it before queue_draw
        {
            let mut state = self.state.borrow_mut();

            let shape = match tool {
                DesignerTool::Rectangle => {
                    let x = start.0.min(end.0);
                    let y = start.1.min(end.1);
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();

                    if width > 1.0 && height > 1.0 {
                        Some(Shape::Rectangle(Rectangle::new(x, y, width, height)))
                    } else {
                        None
                    }
                }
                DesignerTool::Circle => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let radius = width.min(height) / 2.0;

                    if radius > 1.0 {
                        Some(Shape::Circle(Circle::new(Point::new(cx, cy), radius)))
                    } else {
                        None
                    }
                }
                DesignerTool::Line => Some(Shape::Line(Line::new(
                    Point::new(start.0, start.1),
                    Point::new(end.0, end.1),
                ))),
                DesignerTool::Ellipse => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let rx = (end.0 - start.0).abs() / 2.0;
                    let ry = (end.1 - start.1).abs() / 2.0;

                    if rx > 1.0 && ry > 1.0 {
                        Some(Shape::Ellipse(Ellipse::new(Point::new(cx, cy), rx, ry)))
                    } else {
                        None
                    }
                }
                DesignerTool::Triangle => {
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;

                    if width > 1.0 && height > 1.0 {
                        Some(Shape::Triangle(Triangle::new(
                            Point::new(cx, cy),
                            width,
                            height,
                        )))
                    } else {
                        None
                    }
                }
                DesignerTool::Polygon => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let radius = width.min(height) / 2.0;

                    if radius > 1.0 {
                        Some(Shape::Polygon(Polygon::new(Point::new(cx, cy), radius, 6)))
                    } else {
                        None
                    }
                }
                DesignerTool::Gear => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let radius = width.min(height) / 2.0;

                    if radius > 1.0 {
                        // Default to module 2.0, 20 teeth
                        Some(Shape::Gear(gcodekit5_designer::model::DesignGear::new(
                            Point::new(cx, cy),
                            2.0,
                            20,
                        )))
                    } else {
                        None
                    }
                }
                DesignerTool::Sprocket => {
                    let cx = (start.0 + end.0) / 2.0;
                    let cy = (start.1 + end.1) / 2.0;
                    let width = (end.0 - start.0).abs();
                    let height = (end.1 - start.1).abs();
                    let radius = width.min(height) / 2.0;

                    if radius > 1.0 {
                        // Default to 12.7mm pitch (ANSI 40), 15 teeth
                        Some(Shape::Sprocket(
                            gcodekit5_designer::model::DesignSprocket::new(
                                Point::new(cx, cy),
                                12.7,
                                15,
                            ),
                        ))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(shape) = shape {
                state.add_shape_with_undo(shape);
            }
        } // Drop the mutable borrow here

        // Refresh layers panel
        if let Some(layers_panel) = self.layers.borrow().as_ref() {
            layers_panel.refresh(&self.state);
        }
    }
}
