use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, Label, Orientation, ScrolledWindow, Window,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::rc::Rc;

pub struct TabbedBoxDialog {
    pub content: Box,
}

impl TabbedBoxDialog {
    pub fn new() -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&content_box)
            .build();

        // Dimensions Group
        let dim_group = PreferencesGroup::builder().title("Dimensions").build();
        dim_group.add(&Self::create_entry_row("Width", "100.0"));
        dim_group.add(&Self::create_entry_row("Height", "100.0"));
        dim_group.add(&Self::create_entry_row("Depth", "50.0"));
        dim_group.add(&Self::create_check_row("Outside Dimensions", true));
        content_box.append(&dim_group);

        // Material Settings
        let mat_group = PreferencesGroup::builder().title("Material Settings").build();
        mat_group.add(&Self::create_entry_row("Thickness", "3.0"));
        mat_group.add(&Self::create_entry_row("Burn / Tool Dia", "0.1"));
        content_box.append(&mat_group);

        // Finger Joint Settings
        let finger_group = PreferencesGroup::builder().title("Finger Joint Settings").build();
        finger_group.add(&Self::create_entry_row("Finger Width", "2.0"));
        finger_group.add(&Self::create_entry_row("Space Width", "2.0"));
        finger_group.add(&Self::create_entry_row("Surrounding Spaces", "2.0"));
        finger_group.add(&Self::create_entry_row("Play (tolerance)", "0.0"));
        finger_group.add(&Self::create_combo_row("Finger Style", &["Rectangular", "Springs", "Barbs", "Snap", "Dogbone"]));
        content_box.append(&finger_group);

        // Box Configuration
        let box_group = PreferencesGroup::builder().title("Box Configuration").build();
        box_group.add(&Self::create_combo_row("Box Type", &["Full Box", "No Top", "No Bottom", "No Sides", "No Front/Back", "No Left/Right"]));
        box_group.add(&Self::create_entry_row("Dividers X", "0"));
        box_group.add(&Self::create_entry_row("Dividers Y", "0"));
        box_group.add(&Self::create_check_row("Optimize Layout", false));
        content_box.append(&box_group);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let generate_btn = Button::with_label("Generate");
        generate_btn.add_css_class("suggested-action");

        action_box.append(&generate_btn);

        let main_layout = Box::new(Orientation::Vertical, 0);
        main_layout.append(&scrolled);
        main_layout.append(&action_box);

        Self { content: main_layout }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn create_entry_row(title: &str, default: &str) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        let entry = Entry::builder().text(default).valign(Align::Center).build();
        row.add_suffix(&entry);
        row
    }

    fn create_check_row(title: &str, active: bool) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        let check = CheckButton::builder().active(active).valign(Align::Center).build();
        row.add_suffix(&check);
        row
    }

    fn create_combo_row(title: &str, options: &[&str]) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        let combo = ComboBoxText::new();
        for opt in options {
            combo.append(Some(opt), opt);
        }
        combo.set_active(Some(0));
        combo.set_valign(Align::Center);
        row.add_suffix(&combo);
        row
    }
}
