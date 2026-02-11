use super::*;

use gtk4::gdk::ModifierType;
use gtk4::prelude::*;
use gtk4::{DrawingArea, EventControllerScroll, EventControllerScrollFlags, GestureDrag};
use std::cell::RefCell;
use std::rc::Rc;

impl GcodeVisualizer {
    pub(crate) fn setup_interaction<F: Fn() + 'static>(
        da: &DrawingArea,
        vis: &Rc<RefCell<Visualizer>>,
        update_ui: F,
    ) {
        // Scroll to pan (Ctrl+Scroll to zoom)
        let scroll = EventControllerScroll::new(EventControllerScrollFlags::BOTH_AXES);
        let vis_scroll = vis.clone();
        let da_scroll = da.clone();
        let update_scroll = Rc::new(update_ui);

        let update_scroll_clone = update_scroll.clone();
        scroll.connect_scroll(move |ctrl, dx, dy| {
            let is_ctrl = ctrl
                .current_event_state()
                .contains(ModifierType::CONTROL_MASK);

            let mut vis = vis_scroll.borrow_mut();
            if is_ctrl {
                if dy > 0.0 {
                    vis.zoom_out();
                } else if dy < 0.0 {
                    vis.zoom_in();
                }
            } else {
                let pan_step = 20.0;
                vis.x_offset += (dx as f32) * pan_step;
                // Invert Y (canvas Y is flipped)
                vis.y_offset -= (dy as f32) * pan_step;
            }
            drop(vis);

            update_scroll_clone();
            da_scroll.queue_draw();
            gtk4::glib::Propagation::Stop
        });
        da.add_controller(scroll);

        // Drag to Pan
        let drag = GestureDrag::new();
        let vis_drag = vis.clone();

        // Middle mouse drag also pans (matches Designer)
        let drag_middle = GestureDrag::new();
        drag_middle.set_button(2);
        let vis_drag_middle = vis.clone();

        let start_pan = Rc::new(RefCell::new((0.0f32, 0.0f32)));
        let _da_drag = da.clone();
        let update_drag = update_scroll.clone();

        let start_pan_begin = start_pan.clone();
        drag.connect_drag_begin(move |_, _, _| {
            let vis = vis_drag.borrow();
            *start_pan_begin.borrow_mut() = (vis.x_offset, vis.y_offset);
        });

        let vis_drag_update = vis.clone();
        let da_drag_update = da.clone();
        let start_pan_update = start_pan.clone();

        drag.connect_drag_update(move |_, dx, dy| {
            let mut vis = vis_drag_update.borrow_mut();
            let (sx, sy) = *start_pan_update.borrow();
            // Invert Y for pan because canvas Y is flipped
            vis.x_offset = sx + dx as f32;
            vis.y_offset = sy - dy as f32;
            drop(vis);

            update_drag();

            da_drag_update.queue_draw();
        });
        da.add_controller(drag);

        // Middle-mouse pan uses the same logic
        let start_pan_middle = start_pan.clone();
        let start_pan_middle_begin = start_pan_middle.clone();
        drag_middle.connect_drag_begin(move |_, _, _| {
            let vis = vis_drag_middle.borrow();
            *start_pan_middle_begin.borrow_mut() = (vis.x_offset, vis.y_offset);
        });

        let vis_drag_middle_update = vis.clone();
        let da_drag_middle_update = da.clone();
        let start_pan_middle_update = start_pan_middle.clone();
        let update_drag = update_scroll.clone();
        drag_middle.connect_drag_update(move |_, dx, dy| {
            let mut vis = vis_drag_middle_update.borrow_mut();
            let (sx, sy) = *start_pan_middle_update.borrow();
            vis.x_offset = sx + dx as f32;
            vis.y_offset = sy - dy as f32;
            drop(vis);

            update_drag();
            da_drag_middle_update.queue_draw();
        });
        da.add_controller(drag_middle);
    }

    pub(crate) fn update_scrollbars(&self) {
        let width = self.drawing_area.width() as f64;
        let height = self.drawing_area.height() as f64;

        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let v = self.visualizer.borrow();
        let zoom = v.zoom_scale as f64;
        let page_size_x = width / zoom;
        let page_size_y = height / zoom;

        let center_x = -v.x_offset as f64;
        let center_y = -v.y_offset as f64;

        let val_x = center_x - page_size_x / 2.0;
        let val_y = center_y - page_size_y / 2.0;

        let (min_x, max_x, min_y, max_y) = v.get_bounds();
        let margin = 10.0;

        let lower_x = (min_x as f64 - margin).min(val_x);
        let upper_x = (max_x as f64 + margin).max(val_x + page_size_x);
        let lower_y = (min_y as f64 - margin).min(val_y);
        let upper_y = (max_y as f64 + margin).max(val_y + page_size_y);

        drop(v);

        self.hadjustment.configure(
            val_x,
            lower_x,
            upper_x,
            page_size_x * 0.1,
            page_size_x * 0.9,
            page_size_x,
        );
        self.vadjustment.configure(
            val_y,
            lower_y,
            upper_y,
            page_size_y * 0.1,
            page_size_y * 0.9,
            page_size_y,
        );
    }
}
