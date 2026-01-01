use gtk4::gdk::ModifierType;
use gtk4::prelude::*;
use gtk4::{Box, Button, DrawingArea, Entry, Image, Label, ListBox, Orientation, ScrolledWindow};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::t;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::shapes::OperationType;

pub struct LayersPanel {
    pub widget: Box,
    list_box: ListBox,
    // Z-order control buttons for sensitivity updates
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

    pub fn new(state: Rc<RefCell<DesignerState>>, canvas: DrawingArea) -> Self {
        // Main container
        let widget = Box::new(Orientation::Vertical, 6);
        widget.set_margin_start(6);
        widget.set_margin_end(6);
        widget.set_margin_top(6);
        widget.set_margin_bottom(6);
        widget.set_hexpand(false);
        widget.set_width_request(238); // Fit within 250px parent with margins

        // Header with title and buttons
        let header = Box::new(Orientation::Horizontal, 6);
        let title = Label::new(Some(&t!("Layers")));
        title.set_halign(gtk4::Align::Start);
        title.add_css_class("heading");
        header.append(&title);

        // Spacer
        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);

        // Group button
        let group_btn = Button::builder()
            .label(&t!("Group"))
            .icon_name("object-group-symbolic")
            .build();
        group_btn.set_tooltip_text(Some(&t!("Group (Ctrl+G)")));
        header.append(&group_btn);

        // Ungroup button
        let ungroup_btn = Button::builder()
            .label(&t!("Ungroup"))
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
        }

        scrolled.set_child(Some(&list_box));

        widget.append(&scrolled);

        // Z-order controls
        // Note: The list displays shapes in draw order (first row = back/bottom, last row = front/top)
        // So "Move Up" in the list means moving toward the first row (toward back in z-order)
        // And "Move Down" in the list means moving toward the last row (toward front in z-order)
        let z_order_box = Box::new(Orientation::Horizontal, 6);

        let bring_front_btn = Button::from_icon_name("go-top-symbolic");
        bring_front_btn.set_tooltip_text(Some(&t!("Move to First")));
        bring_front_btn
            .update_property(&[gtk4::accessible::Property::Label(&t!("Move to First"))]);
        bring_front_btn.set_sensitive(false);
        z_order_box.append(&bring_front_btn);

        let bring_forward_btn = Button::from_icon_name("go-up-symbolic");
        bring_forward_btn.set_tooltip_text(Some(&t!("Move Up")));
        bring_forward_btn
            .update_property(&[gtk4::accessible::Property::Label(&t!("Move Up"))]);
        bring_forward_btn.set_sensitive(false);
        z_order_box.append(&bring_forward_btn);

        let send_backward_btn = Button::from_icon_name("go-down-symbolic");
        send_backward_btn.set_tooltip_text(Some(&t!("Move Down")));
        send_backward_btn
            .update_property(&[gtk4::accessible::Property::Label(&t!("Move Down"))]);
        send_backward_btn.set_sensitive(false);
        z_order_box.append(&send_backward_btn);

        let send_back_btn = Button::from_icon_name("go-bottom-symbolic");
        send_back_btn.set_tooltip_text(Some(&t!("Move to Last")));
        send_back_btn.update_property(&[gtk4::accessible::Property::Label(&t!("Move to Last"))]);
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
                Self::refresh_list_box(&list_box_refresh, &state_clone);
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
                Self::refresh_list_box(&list_box_refresh, &state_clone);
                canvas_refresh.queue_draw();
            });
        }

        // Connect bring to front (go-top) -> moves to first in list (back in z-order)
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            bring_front_btn.connect_clicked(move |_| {
                Self::send_to_back(&state_clone);
                Self::refresh_list_box(&list_box_refresh, &state_clone);
                canvas_refresh.queue_draw();
            });
        }

        // Connect bring forward (go-up) -> moves up in list (backward in z-order)
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            bring_forward_btn.connect_clicked(move |_| {
                Self::send_backward(&state_clone);
                Self::refresh_list_box(&list_box_refresh, &state_clone);
                canvas_refresh.queue_draw();
            });
        }

        // Connect send backward (go-down) -> moves down in list (forward in z-order)
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            send_backward_btn.connect_clicked(move |_| {
                Self::bring_forward(&state_clone);
                Self::refresh_list_box(&list_box_refresh, &state_clone);
                canvas_refresh.queue_draw();
            });
        }

        // Connect send to back (go-bottom) -> moves to last in list (front in z-order)
        {
            let state_clone = state.clone();
            let list_box_refresh = list_box.clone();
            let canvas_refresh = canvas.clone();
            send_back_btn.connect_clicked(move |_| {
                Self::bring_to_front(&state_clone);
                Self::refresh_list_box(&list_box_refresh, &state_clone);
                canvas_refresh.queue_draw();
            });
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
                    // Single selection: enable based on position
                    let selected_idx = rows[0].index() as usize;
                    let is_first = selected_idx == 0;
                    let is_last = selected_idx == total_rows.saturating_sub(1);
                    
                    // Up/First buttons: enabled only if NOT first
                    bring_front_btn_clone.set_sensitive(!is_first);
                    bring_forward_btn_clone.set_sensitive(!is_first);
                    
                    // Down/Last buttons: enabled only if NOT last
                    send_backward_btn_clone.set_sensitive(!is_last);
                    send_back_btn_clone.set_sensitive(!is_last);
                } else if has_selection {
                    // Multiple selection: enable all (operations work on primary selection)
                    bring_front_btn_clone.set_sensitive(true);
                    bring_forward_btn_clone.set_sensitive(true);
                    send_backward_btn_clone.set_sensitive(true);
                    send_back_btn_clone.set_sensitive(true);
                } else {
                    // No selection: disable all
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
            bring_front_btn,
            bring_forward_btn,
            send_backward_btn,
            send_back_btn,
        }
    }

    pub fn refresh(&self, state: &Rc<RefCell<DesignerState>>) {
        Self::refresh_list_box(&self.list_box, state);
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

    fn refresh_list_box(list_box: &ListBox, state: &Rc<RefCell<DesignerState>>) {
        // Don't hold a RefCell borrow across GTK mutations, because clearing the list triggers
        // selection-change signals which may borrow_mut() the same state.
        let shapes: Vec<(u64, String, Option<u64>, String, OperationType)> = {
            let state_ref = state.borrow();
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
                    };
                    (
                        shape_obj.id,
                        shape_obj.name.clone(),
                        shape_obj.group_id,
                        shape_type,
                        shape_obj.operation_type,
                    )
                })
                .collect()
        };

        // Clear existing rows
        while let Some(row) = list_box.first_child() {
            list_box.remove(&row);
        }

        // Rebuild rows
        for (shape_id, shape_name, group_id, shape_type, operation_type) in shapes {
            let row_box = Box::new(Orientation::Horizontal, 6);
            row_box.set_margin_start(6);
            row_box.set_margin_end(6);
            row_box.set_margin_top(3);
            row_box.set_margin_bottom(3);

            // CAM operation icon
            let (op_icon, op_tooltip) = match operation_type {
                OperationType::Pocket => ("selection-mode-symbolic", t!("Pocket operation")),
                OperationType::Profile => ("emblem-documents-symbolic", t!("Profile operation")),
            };
            let op_image = Image::from_icon_name(op_icon);
            op_image.set_tooltip_text(Some(&op_tooltip));
            op_image.set_pixel_size(16);
            row_box.append(&op_image);

            // Shape type icon/label
            let type_label = Label::new(Some(&shape_type));
            type_label.set_width_chars(5);
            type_label.set_xalign(0.0);
            row_box.append(&type_label);

            // ID Label
            let id_label = Label::new(Some(&format!("#{}", shape_id)));
            id_label.set_width_chars(4);
            id_label.set_xalign(0.0);
            row_box.append(&id_label);

            // Name Entry
            let name_entry = Entry::new();
            name_entry.set_text(&shape_name);
            name_entry.set_hexpand(true);

            let state_clone = state.clone();
            name_entry.connect_changed(move |entry| {
                let text = entry.text();
                let mut state_mut = state_clone.borrow_mut();
                if let Some(obj) = state_mut.canvas.get_shape_mut(shape_id) {
                    obj.name = text.to_string();
                }
            });

            // Stop propagation of click events to prevent row selection when clicking entry
            let gesture = gtk4::GestureClick::new();
            gesture.connect_pressed(|gesture, _, _, _| {
                gesture.set_state(gtk4::EventSequenceState::Claimed);
            });
            name_entry.add_controller(gesture);

            row_box.append(&name_entry);

            // Group ID Label
            let group_text = if let Some(gid) = group_id {
                format!("G:{}", gid)
            } else {
                "-".to_string()
            };
            let group_label = Label::new(Some(&group_text));
            group_label.set_width_chars(6);
            group_label.set_xalign(1.0);
            row_box.append(&group_label);

            // Create a list row and set its name to the shape ID
            let list_row = gtk4::ListBoxRow::new();
            list_row.set_widget_name(&shape_id.to_string());
            list_row.set_child(Some(&row_box));

            list_box.append(&list_row);
        }
    }

    fn group_selected_shapes(state: &Rc<RefCell<DesignerState>>) {
        state.borrow_mut().group_selected();
    }

    fn ungroup_selected_shapes(state: &Rc<RefCell<DesignerState>>) {
        state.borrow_mut().ungroup_selected();
    }

    fn bring_to_front(state: &Rc<RefCell<DesignerState>>) {
        let mut state_mut = state.borrow_mut();
        if let Some(shape_id) = state_mut.canvas.selection_manager.selected_id() {
            state_mut.canvas.shape_store.bring_to_front(shape_id);
            state_mut.is_modified = true;
            state_mut.gcode_generated = false;
        }
    }

    fn bring_forward(state: &Rc<RefCell<DesignerState>>) {
        let mut state_mut = state.borrow_mut();
        if let Some(shape_id) = state_mut.canvas.selection_manager.selected_id() {
            state_mut.canvas.shape_store.bring_forward(shape_id);
            state_mut.is_modified = true;
            state_mut.gcode_generated = false;
        }
    }

    fn send_backward(state: &Rc<RefCell<DesignerState>>) {
        let mut state_mut = state.borrow_mut();
        if let Some(shape_id) = state_mut.canvas.selection_manager.selected_id() {
            state_mut.canvas.shape_store.send_backward(shape_id);
            state_mut.is_modified = true;
            state_mut.gcode_generated = false;
        }
    }

    fn send_to_back(state: &Rc<RefCell<DesignerState>>) {
        let mut state_mut = state.borrow_mut();
        if let Some(shape_id) = state_mut.canvas.selection_manager.selected_id() {
            state_mut.canvas.shape_store.send_to_back(shape_id);
            state_mut.is_modified = true;
            state_mut.gcode_generated = false;
        }
    }
}
