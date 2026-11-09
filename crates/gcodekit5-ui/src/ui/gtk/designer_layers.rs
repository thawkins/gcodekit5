//! # Designer Layers Panel (Objets)
//!
//! UI panel for managing design objects in the visual designer.
//! Supports layer visibility toggling, ordering, and selection.

use gtk4::gdk::ModifierType;
use gtk4::prelude::*;
use gtk4::{Box, Button, DrawingArea, Entry, Image, Label, ListBox, Orientation, ScrolledWindow};
use std::cell::Cell;
use std::rc::Rc;

use crate::t;
use gcodekit5_core::Shared;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::shapes::OperationType;
use gcodekit5_designer::commands::{DesignerCommand, Reorder};

pub struct LayersPanel {
    pub widget: Box,
    list_box: ListBox,
    canvas: DrawingArea,
    bring_front_btn: Button,
    bring_forward_btn: Button,
    send_backward_btn: Button,
    send_back_btn: Button,
}

impl LayersPanel {
    fn list_box_rows(list_box: &ListBox) -> Vec<gtk4::ListBoxRow> {
        let mut out = Vec::new();
        let mut child_opt = list_box.first_child();
        while let Some(child) = child_opt {
            child_opt = child.next_sibling();
            if let Ok(row) = child.downcast::<gtk4::ListBoxRow>() {
                out.push(row);
            }
        }
        out
    }

    pub fn select_row_at_index(&self, index: usize) {
        let rows = Self::list_box_rows(&self.list_box);
        if let Some(row) = rows.get(index) {
            self.list_box.select_row(Some(row));
            row.grab_focus();
        }
    }

    pub fn sync_selection(list_box: &ListBox, state: &Shared<DesignerState>) {
        let state_borrow = state.borrow();

        // 1. We clear the current selection from the list
        list_box.unselect_all();
        let rows = Self::list_box_rows(list_box);

        // 2. We iterate through the state objects to see which ones are selected
        for (idx, shape_obj) in state_borrow.canvas.shape_store.iter().enumerate() {
            if shape_obj.selected {
                if let Some(row) = rows.get(idx) {
                    // 3. We correct the GTK type error with Some::<&ListBoxRow>(row)
                    list_box.select_row(Some(row));

                    // 4. Auto-scroll: we put the focus so that the list moves to the object
                    row.grab_focus();
                }
            }
        }
    }

    pub fn new(state: Shared<DesignerState>, canvas: DrawingArea) -> Self {
        // Main container
        let widget = Box::new(Orientation::Vertical, 6);
        widget.set_margin_start(6);
        widget.set_margin_end(6);
        widget.set_margin_top(6);
        widget.set_margin_bottom(6);
        widget.set_hexpand(false);
        widget.set_width_request(238); // Fit within 250px parent with margins

        let parent_window: Option<gtk4::Window> = None;

        // Header with title and buttons
        let header = Box::new(Orientation::Horizontal, 6);
        let title = Label::new(Some(&t!("Objects")));
        title.set_halign(gtk4::Align::Start);
        title.add_css_class("heading");
        header.append(&title);

        // Spacer
        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);

        // Group button
        let group_btn = Button::builder()
        .label(t!("Group"))
        .icon_name("object-group-symbolic")
        .build();
        group_btn.set_tooltip_text(Some(&t!("Group (Ctrl+G)")));
        header.append(&group_btn);

        // Ungroup button
        let ungroup_btn = Button::builder()
        .label(t!("Ungroup"))
        .icon_name("object-ungroup-symbolic")
        .build();
        ungroup_btn.set_tooltip_text(Some(&t!("Ungroup (Ctrl+Shift+G)")));
        header.append(&ungroup_btn);

        widget.append(&header);

        // Scrolled list of shapes
        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_height_request(150);

        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Multiple);
        list_box.set_focusable(true);
        list_box.set_can_focus(true);

        // Canvas events
        let click_on_canvas = gtk4::GestureClick::new();
        let list_box_to_sync = list_box.clone();
        let state_to_sync = state.clone();

        let canvas_clone = canvas.clone();
        let state_clone = state.clone();

        click_on_canvas.connect_released(move |gesture, _n_press, x, y| {
            let mods = gesture.current_event_state();
            let ctrl = mods.contains(ModifierType::CONTROL_MASK);
            let shift = mods.contains(ModifierType::SHIFT_MASK);
            let multi = ctrl || shift;

            let state_borrow = state_clone.borrow();
            let world_point = state_borrow.canvas.pixel_to_world(x, y);
            let zoom = state_borrow.canvas.zoom();
            let tolerance = 15.0 / zoom;
            drop(state_borrow);

            // Select on canvas
            let mut state_mut = state_clone.borrow_mut();
            state_mut
            .canvas
            .select_at(&world_point, tolerance, multi, false);

            drop(state_mut);

            // Update ONLY the list, not canvas
            Self::refresh_list_box(&list_box_to_sync, &state_to_sync, &canvas_clone);

            // Force canvas redraw to show selection
            canvas_clone.queue_draw();
        });

        canvas.add_controller(click_on_canvas);

        // Make single-click replace selection; Ctrl toggles; Shift selects range.
        let selection_anchor: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
        {
            let list_box_click = list_box.clone();
            let selection_anchor = selection_anchor.clone();
            let click = gtk4::GestureClick::new();
            click.connect_pressed(move |gesture, _, _, y| {
                let mods = gesture.current_event_state();
                let ctrl = mods.contains(ModifierType::CONTROL_MASK);
                let shift = mods.contains(ModifierType::SHIFT_MASK);

                if let Some(row) = list_box_click.row_at_y(y as i32) {
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                    list_box_click.grab_focus();

                    if shift {
                        let anchor = selection_anchor
                        .get()
                        .or_else(|| list_box_click.selected_rows().first().map(|r| r.index()));
                        if let Some(anchor_idx) = anchor {
                            let target_idx = row.index();
                            let (min_i, max_i) = if anchor_idx <= target_idx {
                                (anchor_idx, target_idx)
                            } else {
                                (target_idx, anchor_idx)
                            };

                            list_box_click.unselect_all();
                            for r in Self::list_box_rows(&list_box_click) {
                                let idx = r.index();
                                if (min_i..=max_i).contains(&idx) {
                                    list_box_click.select_row(Some(&r));
                                }
                            }
                        } else {
                            list_box_click.unselect_all();
                            list_box_click.select_row(Some(&row));
                        }
                    } else if ctrl {
                        if row.is_selected() {
                            list_box_click.unselect_row(&row);
                        } else {
                            list_box_click.select_row(Some(&row));
                        }
                    } else {
                        list_box_click.unselect_all();
                        list_box_click.select_row(Some(&row));
                    }

                    selection_anchor.set(Some(row.index()));
                }
            });
            list_box.add_controller(click);
            list_box.set_css_classes(&["compact-list"]);
        }

        scrolled.set_child(Some(&list_box));

        widget.append(&scrolled);

        // Z-order controls
        let z_order_box = Box::new(Orientation::Horizontal, 6);

//        let bring_front_btn = Button::from_icon_name("go-top-symbolic");

        let bring_front_btn = Button::new();
        let img = gtk4::Image::from_resource("/com/gcodekit5/icons/move_to_top.svg");
        img.set_pixel_size(24);
        bring_front_btn.set_child(Some(&img));

        bring_front_btn.set_tooltip_text(Some(&t!("Move to start")));
        bring_front_btn
        .update_property(&[gtk4::accessible::Property::Label(&t!("Move to start"))]);
        bring_front_btn.set_sensitive(false);
        z_order_box.append(&bring_front_btn);

        let bring_forward_btn = Button::from_icon_name("go-up-symbolic");
        bring_forward_btn.set_tooltip_text(Some(&t!("Move up")));
        bring_forward_btn.update_property(&[gtk4::accessible::Property::Label(&t!("Move up"))]);
        bring_forward_btn.set_sensitive(false);
        z_order_box.append(&bring_forward_btn);

        let send_backward_btn = Button::from_icon_name("go-down-symbolic");
        send_backward_btn.set_tooltip_text(Some(&t!("Move down")));
        send_backward_btn
        .update_property(&[gtk4::accessible::Property::Label(&t!("Move down"))]);
        send_backward_btn.set_sensitive(false);
        z_order_box.append(&send_backward_btn);

//        let send_back_btn = Button::from_icon_name("go-bottom-symbolic");
        let send_back_btn = Button::new();
        let img = gtk4::Image::from_resource("/com/gcodekit5/icons/move_to_bottom.svg");
        img.set_pixel_size(24);
        send_back_btn.set_child(Some(&img));

        send_back_btn.set_tooltip_text(Some(&t!("Move to end")));
        send_back_btn.update_property(&[gtk4::accessible::Property::Label(&t!("Move to end"))]);
        send_back_btn.set_sensitive(false);
        z_order_box.append(&send_back_btn);

        widget.append(&z_order_box);

        // Connect group button
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            group_btn.connect_clicked(move |_| {
                Self::group_selected_shapes(&state_clone);
                Self::refresh_list_box(&list_box_refresh, &state_clone, &canvas_refresh);
                canvas_refresh.queue_draw();
            });
        }

        // Connect ungroup button
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            ungroup_btn.connect_clicked(move |_| {
                Self::ungroup_selected_shapes(&state_clone);
                Self::refresh_list_box(&list_box_refresh, &state_clone, &canvas_refresh);
                canvas_refresh.queue_draw();
            });
        }

        // go-top: mover al principio (en bloque)
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            let parent_for_click = parent_window.clone();
            bring_front_btn.connect_clicked(move |_| {
                let applied = state_clone.borrow_mut().move_selected_to_top();
                if !applied {
                    show_cannot_reorder_dialog(parent_for_click.as_ref());
                    return;
                }
                Self::refresh_list_box(&list_box_refresh, &state_clone, &canvas_refresh);
                canvas_refresh.queue_draw();
            });
        }

        // go-up: subir (en bloque)
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            let parent_for_click = parent_window.clone();
            bring_forward_btn.connect_clicked(move |_| {
                let applied = state_clone.borrow_mut().move_selected_up();
                if !applied {
                    show_cannot_reorder_dialog(parent_for_click.as_ref());
                    return;
                }
                Self::refresh_list_box(&list_box_refresh, &state_clone, &canvas_refresh);
                canvas_refresh.queue_draw();
            });
        }

        // go-down: bajar (en bloque)
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            let parent_for_click = parent_window.clone();
            send_backward_btn.connect_clicked(move |_| {
                let applied = state_clone.borrow_mut().move_selected_down();
                if !applied {
                    show_cannot_reorder_dialog(parent_for_click.as_ref());
                    return;
                }
                Self::refresh_list_box(&list_box_refresh, &state_clone, &canvas_refresh);
                canvas_refresh.queue_draw();
            });
        }

        // go-bottom: mover al final (en bloque)
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            let parent_for_click = parent_window.clone();
            send_back_btn.connect_clicked(move |_| {
                let applied = state_clone.borrow_mut().move_selected_to_bottom();
                if !applied {
                    show_cannot_reorder_dialog(parent_for_click.as_ref());
                    return;
                }
                Self::refresh_list_box(&list_box_refresh, &state_clone, &canvas_refresh);
                canvas_refresh.queue_draw();
            });
        }

        // Delete key in object list: deletes the selection
        {
            let state_clone = state.clone();
            let canvas_clone = canvas.clone();
            let list_box_clone = list_box.clone();

            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_controller, key, _code, _modifiers| {
                match key {
                    gtk4::gdk::Key::Delete | gtk4::gdk::Key::BackSpace => {
                        // Borrar los objetos seleccionados
                        let mut state_mut = state_clone.borrow_mut();
                        let selected_ids: Vec<u64> = state_mut
                        .canvas
                        .shapes()
                        .filter(|s| s.selected)
                        .map(|s| s.id)
                        .collect();
                        if selected_ids.is_empty() {
                            return gtk4::glib::Propagation::Proceed;
                        }
                        for id in selected_ids {
                            let cmd = gcodekit5_designer::commands::DesignerCommand::RemoveShape(
                                gcodekit5_designer::commands::RemoveShape { id, object: None },
                            );
                            state_mut.push_command(cmd);
                        }
                        drop(state_mut);

                        // Refrescar lista y canvas
                        Self::refresh_list_box(
                            &list_box_clone,
                            &state_clone,
                            &canvas_clone,
                        );
                        canvas_clone.queue_draw();

                        gtk4::glib::Propagation::Stop
                    }
                    _ => gtk4::glib::Propagation::Proceed,
                }
            });

            list_box.add_controller(key_controller);
        }

        // Connect list selection to shape selection and update button sensitivity
        {
            let state_clone = state.clone();
            let canvas_refresh = canvas.clone();
            let bring_front_btn_clone = bring_front_btn.clone();
            let bring_forward_btn_clone = bring_forward_btn.clone();
            let send_backward_btn_clone = send_backward_btn.clone();
            let send_back_btn_clone = send_back_btn.clone();

            list_box.connect_selected_rows_changed(move |list| {
                let rows = list.selected_rows();
                let mut state_mut = state_clone.borrow_mut();
                let canvas = &mut state_mut.canvas;

                canvas
                .selection_manager
                .deselect_all(&mut canvas.shape_store);

                let mut first: Option<u64> = None;
                for row in &rows {
                    let id_str = row.widget_name();
                    if let Ok(shape_id) = id_str.as_str().parse::<u64>() {
                        if first.is_none() {
                            first = Some(shape_id);
                        }
                        if let Some(obj) = canvas.shape_store.get_mut(shape_id) {
                            obj.selected = true;
                        }
                    }
                }

                canvas.selection_manager.set_selected_id(first);

                // Update button sensitivity based on selection position
                let total_rows = Self::list_box_rows(list).len();
                let has_selection = !rows.is_empty();

                if has_selection && rows.len() == 1 {
                    let selected_idx = rows[0].index() as usize;
                    let is_first = selected_idx == 0;
                    let is_last = selected_idx == total_rows.saturating_sub(1);

                    bring_front_btn_clone.set_sensitive(!is_first);
                    bring_forward_btn_clone.set_sensitive(!is_first);
                    send_backward_btn_clone.set_sensitive(!is_last);
                    send_back_btn_clone.set_sensitive(!is_last);
                } else if has_selection {
                    bring_front_btn_clone.set_sensitive(true);
                    bring_forward_btn_clone.set_sensitive(true);
                    send_backward_btn_clone.set_sensitive(true);
                    send_back_btn_clone.set_sensitive(true);
                } else {
                    bring_front_btn_clone.set_sensitive(false);
                    bring_forward_btn_clone.set_sensitive(false);
                    send_backward_btn_clone.set_sensitive(false);
                    send_back_btn_clone.set_sensitive(false);
                }

                drop(state_mut);
                canvas_refresh.queue_draw();
            });
        }

        Self {
            widget,
            list_box,
            canvas,
            bring_front_btn,
            bring_forward_btn,
            send_backward_btn,
            send_back_btn,
        }
    }

    pub fn refresh(&self, state: &Shared<DesignerState>) {
        Self::refresh_list_box(&self.list_box, state, &self.canvas);
        self.update_button_sensitivity();
    }

    /// Update button sensitivity based on current list selection
    fn update_button_sensitivity(&self) {
        let rows = self.list_box.selected_rows();
        let total_rows = Self::list_box_rows(&self.list_box).len();
        let has_selection = !rows.is_empty();

        if has_selection && rows.len() == 1 {
            let selected_idx = rows[0].index() as usize;
            let is_first = selected_idx == 0;
            let is_last = selected_idx == total_rows.saturating_sub(1);

            self.bring_front_btn.set_sensitive(!is_first);
            self.bring_forward_btn.set_sensitive(!is_first);
            self.send_backward_btn.set_sensitive(!is_last);
            self.send_back_btn.set_sensitive(!is_last);
        } else if has_selection {
            self.bring_front_btn.set_sensitive(true);
            self.bring_forward_btn.set_sensitive(true);
            self.send_backward_btn.set_sensitive(true);
            self.send_back_btn.set_sensitive(true);
        } else {
            self.bring_front_btn.set_sensitive(false);
            self.bring_forward_btn.set_sensitive(false);
            self.send_backward_btn.set_sensitive(false);
            self.send_back_btn.set_sensitive(false);
        }
    }

    fn refresh_list_box(list_box: &ListBox, state: &Shared<DesignerState>, canvas: &DrawingArea) {
        // 1. Save selected objects BEFORE rebuilding
        let selected_ids: Vec<u64> = {
            let state_ref = state.borrow();
            state_ref
            .canvas
            .shape_store
            .iter()
            .filter(|obj| obj.selected)
            .map(|obj| obj.id)
            .collect()
        };

        // 2. Obtain shapes with their current position (index in draw_order)
        let shapes_with_position: Vec<(usize, u64, String, Option<u64>, String, OperationType)> = {
            let state_ref = state.borrow();
            let draw_order = state_ref.canvas.shape_store.draw_order();
            let mut positions = std::collections::HashMap::new();
            for (pos, &id) in draw_order.iter().enumerate() {
                positions.insert(id, pos + 1);
            }

            state_ref
            .canvas
            .shape_store
            .iter()
            .map(|shape_obj| {
                let shape_type = match &shape_obj.shape {
                    gcodekit5_designer::model::Shape::Rectangle(_) => t!("Rect"),
                 gcodekit5_designer::model::Shape::Circle(_) => t!("Circ"),
                 gcodekit5_designer::model::Shape::Line(_) => t!("Line"),
                 gcodekit5_designer::model::Shape::Ellipse(_) => t!("Ellip"),
                 gcodekit5_designer::model::Shape::Path(_) => t!("Path"),
                 gcodekit5_designer::model::Shape::Text(_) => t!("Text"),
                 gcodekit5_designer::model::Shape::Triangle(_) => t!("Tri"),
                 gcodekit5_designer::model::Shape::Polygon(_) => t!("Poly"),
                 gcodekit5_designer::model::Shape::Gear(_) => t!("Gear"),
                 gcodekit5_designer::model::Shape::Sprocket(_) => t!("Spro"),
                 gcodekit5_designer::model::Shape::RasterImage(_) => t!("Image"),
                };

                let position = positions.get(&shape_obj.id).copied().unwrap_or(0);

                (
                    position,
                 shape_obj.id,
                 shape_obj.name.clone(),
                 shape_obj.group_id,
                 shape_type,
                 shape_obj.operation_type,
                )
            })
            .collect()
        };

        let mut shapes = shapes_with_position;
        shapes.sort_by_key(|(pos, _, _, _, _, _)| *pos);

        // Clear existing rows
        while let Some(row) = list_box.first_child() {
            list_box.remove(&row);
        }

        // Rebuild rows
        for (position, shape_id, shape_name, group_id, shape_type, operation_type) in shapes {
            let row_box = Box::new(Orientation::Horizontal, 4);
            row_box.set_margin_start(4);
            row_box.set_margin_end(4);
            row_box.set_margin_top(1);
            row_box.set_margin_bottom(1);

            // CAM operation icon
            let (op_icon, op_tooltip) = match operation_type {
                OperationType::Pocket => ("selection-mode-symbolic", t!("Pocket operation")),
                OperationType::Profile => ("emblem-documents-symbolic", t!("Profile operation")),
            };
            let op_image = Image::from_icon_name(op_icon);
            op_image.set_tooltip_text(Some(&op_tooltip));
            op_image.set_pixel_size(14);
            row_box.append(&op_image);

            // EDITABLE ORDER NUMBER
            let position_entry = Entry::new();
            position_entry.set_text(&position.to_string());
            position_entry.set_width_chars(3);
            position_entry.set_max_width_chars(3);
            position_entry.set_halign(gtk4::Align::Center);
            position_entry.set_css_classes(&["position-entry"]);
            position_entry.set_margin_top(0);
            position_entry.set_margin_bottom(0);

            let shape_id_for_move = shape_id;
            let list_box_clone = list_box.clone();
            let state_for_refresh = state.clone();
            let canvas_for_refresh = canvas.clone();

            let state_clone = state.clone();
            position_entry.connect_activate(move |entry| {
                if let Ok(new_position) = entry.text().parse::<usize>() {
                    let mut state_mut = state_clone.borrow_mut();
                    let total_objects = state_mut.canvas.shape_store.len();

                    if new_position >= 1 && new_position <= total_objects {
                        let target_idx = new_position - 1;
                        let current_pos = state_mut
                        .canvas
                        .shape_store
                        .draw_order()
                        .iter()
                        .position(|&id| id == shape_id_for_move);

                        if let Some(current_idx) = current_pos {
                            if current_idx != target_idx {
                                if state_mut
                                    .canvas
                                    .shape_store
                                    .would_split_group(shape_id_for_move, target_idx)
                                    {
                                        // Rechazar: restaurar el valor original
                                        let original = (current_idx + 1).to_string();
                                        entry.set_text(&original);
                                        drop(state_mut);

                                        // Resolver el parent aquí, en el momento del click
                                        let parent = list_box_clone
                                        .root()
                                        .and_then(|r| r.downcast::<gtk4::Window>().ok());
                                        show_cannot_reorder_dialog(parent.as_ref());
                                        return;
                                    }

                                    // Hacer:
                                    let before = state_mut.canvas.shape_store.draw_order().clone();
                                    state_mut
                                    .canvas
                                    .shape_store
                                    .move_to_position(shape_id_for_move, target_idx);
                                    let after = state_mut.canvas.shape_store.draw_order().clone();
                                    // Deshacer para que el comando lo aplique
                                    state_mut.canvas.shape_store.apply_order(before.clone());
                                    state_mut.push_command(DesignerCommand::Reorder(Reorder {
                                        before,
                                        after,
                                        name: "Reorder".to_string(),
                                    }));


                                    state_mut.is_modified = true;
                                    state_mut.gcode_generated = false;
                                    drop(state_mut);
                                    Self::refresh_list_box(
                                        &list_box_clone,
                                        &state_for_refresh,
                                        &canvas_for_refresh,
                                    );
                            }
                        }
                    } else {
                        // Número fuera de rango: restaurar
                        let current_pos = state_mut
                        .canvas
                        .shape_store
                        .draw_order()
                        .iter()
                        .position(|&id| id == shape_id_for_move)
                        .map(|p| (p + 1).to_string())
                        .unwrap_or_else(|| "?".to_string());
                        entry.set_text(&current_pos);
                    }
                } else {
                    // No es un número: restaurar
                    let state_borrow = state_clone.borrow();
                    let current_pos = state_borrow
                    .canvas
                    .shape_store
                    .draw_order()
                    .iter()
                    .position(|&id| id == shape_id_for_move)
                    .map(|p| (p + 1).to_string())
                    .unwrap_or_else(|| "?".to_string());
                    entry.set_text(&current_pos);
                }
            });

            row_box.append(&position_entry);

            // Shape type label
            let type_label = Label::new(Some(&shape_type));
            type_label.set_width_chars(4);
            type_label.set_xalign(0.0);
            type_label.set_margin_start(2);
            type_label.set_margin_end(2);
            row_box.append(&type_label);

            // ID Label
            let id_label = Label::new(Some(&format!("#{}", shape_id)));
            id_label.set_width_chars(3);
            id_label.set_xalign(0.0);
            id_label.set_margin_start(2);
            id_label.set_margin_end(2);
            row_box.append(&id_label);

            // Name Entry
            let name_entry = Entry::new();
            name_entry.set_text(&shape_name);
            name_entry.set_hexpand(true);
            name_entry.set_height_request(14);
            name_entry.set_margin_top(0);
            name_entry.set_margin_bottom(0);

            let state_clone = state.clone();
            name_entry.connect_changed(move |entry| {
                let text = entry.text();
                let mut state_mut = state_clone.borrow_mut();
                if let Some(obj) = state_mut.canvas.get_shape_mut(shape_id) {
                    obj.name = text.to_string();
                }
            });

            let gesture = gtk4::GestureClick::new();
            gesture.connect_pressed(|gesture, _, _, _| {
                gesture.set_state(gtk4::EventSequenceState::Claimed);
            });
            name_entry.add_controller(gesture);

            row_box.append(&name_entry);

            // Group ID Label — clickeable si tiene grupo
            let group_text = if let Some(gid) = group_id {
                format!("G:{}", gid)
            } else {
                "-".to_string()
            };
            let group_label = Label::new(Some(&group_text));
            group_label.set_width_chars(5);
            group_label.set_xalign(1.0);
            group_label.set_margin_start(2);
            group_label.set_margin_end(2);

            if let Some(gid) = group_id {
                group_label.add_css_class("group-badge");
                group_label.set_tooltip_text(Some(&t!("Click to select the whole group")));

                let state_for_click = state.clone();
                let list_box_for_click = list_box.clone();
                let canvas_for_click = canvas.clone();

                let click = gtk4::GestureClick::new();
                click.set_button(1);
                click.connect_pressed(move |gesture, _n_press, _x, _y| {
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                    let mods = gesture.current_event_state();
                    let additive = mods.contains(ModifierType::CONTROL_MASK);

                    let mut state_mut = state_for_click.borrow_mut();

                    {
                        let canvas = &mut state_mut.canvas;

                        if !additive {
                            canvas
                            .selection_manager
                            .deselect_all(&mut canvas.shape_store);
                        }

                        let mut first_of_group: Option<u64> = None;
                        for obj in canvas.shape_store.iter_mut() {
                            if obj.group_id == Some(gid) {
                                obj.selected = true;
                                if first_of_group.is_none() {
                                    first_of_group = Some(obj.id);
                                }
                            }
                        }
                        canvas.selection_manager.set_selected_id(first_of_group);
                    }

                    drop(state_mut);

                    Self::refresh_list_box(
                        &list_box_for_click,
                        &state_for_click,
                        &canvas_for_click,
                    );
                    canvas_for_click.queue_draw();
                    list_box_for_click.grab_focus();
                });
                group_label.add_controller(click);
            }

            row_box.append(&group_label);
            type_label.set_height_request(14);
            id_label.set_height_request(14);
            group_label.set_height_request(14);

            let list_row = gtk4::ListBoxRow::new();
            list_row.set_widget_name(&shape_id.to_string());
            list_row.set_child(Some(&row_box));

            list_box.append(&list_row);

            if selected_ids.contains(&shape_id) {
                list_box.select_row(Some(&list_row));
            }
        }
    }

    fn group_selected_shapes(state: &Shared<DesignerState>) {
        state.borrow_mut().group_selected();
    }

    fn ungroup_selected_shapes(state: &Shared<DesignerState>) {
        state.borrow_mut().ungroup_selected();
    }
}

/// Muestra un diálogo modal avisando de que la operación no se puede aplicar
/// porque partiría un grupo.
fn show_cannot_reorder_dialog(parent: Option<&gtk4::Window>) {
    let dialog = gtk4::AlertDialog::builder()
    .modal(true)
    .message(t!("It cannot be moved"))
    .detail(t!(
        "The move would split a group. The members of a group must remain contiguous."
    ))
    .build();

    dialog.choose(
        parent,
        gtk4::gio::Cancellable::NONE,
        move |_result| {
        },
    );
}
