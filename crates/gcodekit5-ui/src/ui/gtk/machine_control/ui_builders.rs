//! # Machine Control UI Builders
//!
//! Helper functions for constructing the Machine Control panel UI elements.

use gtk4::prelude::*;
use gtk4::{
    accessible::Property as AccessibleProperty, Align, Box, Button, Image, Label, Orientation,
    ToggleButton,
};

use crate::t;

/// Create a button with an icon and label
pub fn make_icon_label_button(icon: &str, label: &str) -> Button {
    let btn = Button::new();
    set_button_icon_label(&btn, icon, label);
    btn
}

/// Set a button's content to an icon and label
pub fn set_button_icon_label(btn: &Button, icon: &str, label: &str) {
    let content = Box::new(Orientation::Horizontal, 6);
    content.set_halign(Align::Center);
    content.set_valign(Align::Center);

    let img = Image::from_icon_name(icon);
    img.set_pixel_size(16);

    let lbl = Label::new(Some(label));
    lbl.set_valign(Align::Center);

    content.append(&img);
    content.append(&lbl);

    btn.set_child(Some(&content));
}

/// Create a toggle button with an icon and label
pub fn make_icon_label_toggle(icon: &str, label: &str) -> ToggleButton {
    let btn = ToggleButton::new();
    let content = Box::new(Orientation::Horizontal, 6);
    content.set_halign(Align::Center);
    content.set_valign(Align::Center);

    let img = Image::from_icon_name(icon);
    img.set_pixel_size(16);

    let lbl = Label::new(Some(label));
    lbl.set_valign(Align::Center);

    content.append(&img);
    content.append(&lbl);

    btn.set_child(Some(&content));
    btn
}

/// Create a section with a title and child widget
pub fn make_section(title: &str, child: &impl IsA<gtk4::Widget>) -> Box {
    let section = Box::new(Orientation::Vertical, 4);
    section.add_css_class("mc-section");

    let header = Label::new(Some(title));
    header.add_css_class("mc-section-title");
    header.set_halign(Align::Start);

    section.append(&header);
    section.append(child);
    section
}

/// Create a DRO (Digital Readout) widget for an axis
pub fn create_dro(axis: &str, display: &str) -> (Box, Label, Button) {
    let b = Box::new(Orientation::Horizontal, 8);
    b.add_css_class("dro-axis");
    b.set_height_request(38);

    let l = Label::new(Some(display));
    l.add_css_class("dro-label");
    l.add_css_class("mc-dro-label");
    l.set_width_request(52);

    let v = Label::new(Some("0.000"));
    v.add_css_class("dro-value");
    v.add_css_class("mc-dro-value");
    v.set_hexpand(true);
    v.set_halign(Align::End);

    let z = make_icon_label_button("edit-clear-symbolic", &t!("Zero"));
    z.add_css_class("circular");
    z.set_valign(Align::Center);
    let tooltip = format!("{} {axis}", t!("Set work axis to zero"));
    z.set_tooltip_text(Some(&tooltip));
    let a11y_label = format!("{} {axis}", t!("Zero"));
    z.update_property(&[AccessibleProperty::Label(&a11y_label)]);

    b.append(&l);
    b.append(&v);
    b.append(&z);
    (b, v, z)
}
