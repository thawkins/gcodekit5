//! # Navigation Cube
//!
//! A 3D orientation cube widget for the visualizer that shows
//! the current view direction and allows quick view changes
//! (top, front, side, isometric) via click interaction.

use gcodekit5_core::Shared;
use gcodekit5_visualizer::Camera3D;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Grid, Orientation};

pub struct NavCube {
    pub widget: Box,
    pub fit_btn: Button,
}

impl NavCube {
    pub fn new(camera: Shared<Camera3D>, gl_area: gtk4::GLArea) -> Self {
        let container = Box::new(Orientation::Vertical, 4);
        container.add_css_class("nav-cube-container");
        container.set_halign(Align::End);
        container.set_valign(Align::Start);
        container.set_margin_top(10);
        container.set_margin_end(10);

        let grid = Grid::builder()
            .row_spacing(2)
            .column_spacing(2)
            .css_classes(vec!["nav-cube"])
            .build();

        // Helper to create button
        let create_btn = |label: &str, tooltip: &str| {
            Button::builder()
                .label(label)
                .tooltip_text(tooltip)
                .css_classes(vec!["nav-btn"])
                .width_request(30)
                .height_request(30)
                .build()
        };

        // Top Row
        let btn_iso_nw = create_btn("↖", "Isometric (NW)");
        let btn_top = create_btn("T", "Top View");
        let btn_iso_ne = create_btn("↗", "Isometric (NE)");

        // Middle Row
        let btn_left = create_btn("L", "Left View");
        let btn_front = create_btn("F", "Front View");
        let btn_right = create_btn("R", "Right View");

        // Bottom Row
        let btn_iso_sw = create_btn("↙", "Isometric (SW)");
        let btn_bottom = create_btn("B", "Bottom View");
        let btn_iso_se = create_btn("↘", "Isometric (SE)");

        // Layout
        grid.attach(&btn_iso_nw, 0, 0, 1, 1);
        grid.attach(&btn_top, 1, 0, 1, 1);
        grid.attach(&btn_iso_ne, 2, 0, 1, 1);

        grid.attach(&btn_left, 0, 1, 1, 1);
        grid.attach(&btn_front, 1, 1, 1, 1);
        grid.attach(&btn_right, 2, 1, 1, 1);

        grid.attach(&btn_iso_sw, 0, 2, 1, 1);
        grid.attach(&btn_bottom, 1, 2, 1, 1);
        grid.attach(&btn_iso_se, 2, 2, 1, 1);

        container.append(&grid);

        // Zoom Controls
        let zoom_box = Box::new(Orientation::Horizontal, 2);
        zoom_box.set_halign(Align::Center);

        let btn_zoom_in = create_btn("+", "Zoom In");
        let btn_fit = create_btn("Fit", "Fit to Content");
        let btn_zoom_out = create_btn("-", "Zoom Out");

        zoom_box.append(&btn_zoom_out);
        zoom_box.append(&btn_fit);
        zoom_box.append(&btn_zoom_in);

        container.append(&zoom_box);

        // Connect signals
        let connect_view = |btn: &Button, yaw: f32, pitch: f32| {
            let cam = camera.clone();
            let area = gl_area.clone();
            btn.connect_clicked(move |_| {
                cam.borrow_mut().set_view(yaw, pitch);
                area.queue_render();
            });
        };

        connect_view(&btn_front, -90.0, 0.0);
        connect_view(&btn_right, 0.0, 0.0);
        connect_view(&btn_left, 180.0, 0.0);
        connect_view(&btn_top, -90.0, 90.0);
        connect_view(&btn_bottom, -90.0, -90.0);

        connect_view(&btn_iso_nw, 135.0, 35.264);
        connect_view(&btn_iso_ne, 45.0, 35.264);
        connect_view(&btn_iso_sw, -135.0, 35.264);
        connect_view(&btn_iso_se, -45.0, 35.264);

        // Zoom
        let cam_z_in = camera.clone();
        let area_z_in = gl_area.clone();
        btn_zoom_in.connect_clicked(move |_| {
            cam_z_in.borrow_mut().zoom(10.0);
            area_z_in.queue_render();
        });

        let cam_z_out = camera.clone();
        let area_z_out = gl_area.clone();
        btn_zoom_out.connect_clicked(move |_| {
            cam_z_out.borrow_mut().zoom(-10.0);
            area_z_out.queue_render();
        });

        Self {
            widget: container,
            fit_btn: btn_fit,
        }
    }
}
