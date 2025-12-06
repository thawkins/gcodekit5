use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CheckButton, ComboBoxText, Entry, Label, Orientation, ScrolledWindow, 
    Stack, IconTheme, Image, GestureClick, Frame, Grid,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::rc::Rc;
use std::cell::RefCell;

pub struct CamToolsView {
    pub content: Stack,
}

impl CamToolsView {
    pub fn new() -> Self {
        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);

        // Dashboard Page
        let dashboard = Self::create_dashboard(&stack);
        stack.add_named(&dashboard, Some("dashboard"));

        // Tool Pages
        let tabbed_box = TabbedBoxMaker::new(&stack);
        stack.add_named(tabbed_box.widget(), Some("tabbed_box"));

        // Placeholders for other tools
        stack.add_named(&Self::create_placeholder("Jigsaw Puzzle Generator", &stack), Some("jigsaw"));
        stack.add_named(&Self::create_placeholder("Laser Image Engraver", &stack), Some("laser_image"));
        stack.add_named(&Self::create_placeholder("Laser Vector Engraver", &stack), Some("laser_vector"));
        stack.add_named(&Self::create_placeholder("Speeds & Feeds Calculator", &stack), Some("feeds"));
        stack.add_named(&Self::create_placeholder("Spoilboard Surfacing", &stack), Some("surfacing"));
        stack.add_named(&Self::create_placeholder("Spoilboard Grid", &stack), Some("grid"));

        Self { content: stack }
    }

    pub fn widget(&self) -> &Stack {
        &self.content
    }

    fn create_dashboard(stack: &Stack) -> ScrolledWindow {
        let container = Box::new(Orientation::Vertical, 24);
        container.set_margin_top(24);
        container.set_margin_bottom(24);
        container.set_margin_start(24);
        container.set_margin_end(24);

        let title = Label::builder()
            .label("CAM Tools")
            .css_classes(vec!["title-1"])
            .halign(Align::Start)
            .build();
        container.append(&title);

        let grid = Grid::builder()
            .column_spacing(24)
            .row_spacing(24)
            .hexpand(true)
            .build();

        // Row 1
        grid.attach(&Self::create_tool_card(
            "Tabbed Box Maker",
            "Generate G-code for laser/CNC cut boxes with finger joints",
            "object-select-symbolic", // Placeholder icon
            "tabbed_box",
            stack
        ), 0, 0, 1, 1);

        grid.attach(&Self::create_tool_card(
            "Jigsaw Puzzle Generator",
            "Create custom jigsaw puzzle patterns from images",
            "image-x-generic-symbolic",
            "jigsaw",
            stack
        ), 1, 0, 1, 1);

        grid.attach(&Self::create_tool_card(
            "Laser Image Engraver",
            "Convert raster images to G-code for laser engraving",
            "camera-photo-symbolic",
            "laser_image",
            stack
        ), 2, 0, 1, 1);

        // Row 2
        grid.attach(&Self::create_tool_card(
            "Laser Vector Engraver",
            "Convert SVG and DXF vector files to G-code",
            "draw-bezier-curves-symbolic",
            "laser_vector",
            stack
        ), 0, 1, 1, 1);

        grid.attach(&Self::create_tool_card(
            "Speeds & Feeds Calculator",
            "Calculate optimal cutting speeds and feeds for your materials",
            "accessories-calculator-symbolic",
            "feeds",
            stack
        ), 1, 1, 1, 1);

        grid.attach(&Self::create_tool_card(
            "Spoilboard Surfacing",
            "Generate surfacing toolpaths to flatten your spoilboard",
            "view-refresh-symbolic",
            "surfacing",
            stack
        ), 2, 1, 1, 1);

        // Row 3
        grid.attach(&Self::create_tool_card(
            "Create Spoilboard Grid",
            "Generate grid patterns for spoilboard alignment",
            "view-grid-symbolic",
            "grid",
            stack
        ), 0, 2, 1, 1);

        container.append(&grid);

        ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&container)
            .build()
    }

    fn create_tool_card(title: &str, desc: &str, icon: &str, target_page: &str, stack: &Stack) -> Button {
        let button = Button::builder()
            .css_classes(vec!["card"])
            .hexpand(true)
            .vexpand(false)
            .build();

        let content = Box::new(Orientation::Vertical, 12);
        content.set_margin_top(24);
        content.set_margin_bottom(24);
        content.set_margin_start(24);
        content.set_margin_end(24);
        content.set_width_request(250);
        content.set_height_request(200);

        let icon_img = Image::from_icon_name(icon);
        icon_img.set_pixel_size(64);
        icon_img.add_css_class("accent");
        
        let title_lbl = Label::builder()
            .label(title)
            .css_classes(vec!["heading"])
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .build();

        let desc_lbl = Label::builder()
            .label(desc)
            .css_classes(vec!["caption"])
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .build();

        content.append(&icon_img);
        content.append(&title_lbl);
        content.append(&desc_lbl);

        button.set_child(Some(&content));

        let stack_clone = stack.clone();
        let page_name = target_page.to_string();
        button.connect_clicked(move |_| {
            stack_clone.set_visible_child_name(&page_name);
        });

        button
    }

    fn create_placeholder(title: &str, stack: &Stack) -> Box {
        let container = Box::new(Orientation::Vertical, 0);
        
        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder()
            .icon_name("go-previous-symbolic")
            .build();
        
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });

        let title_lbl = Label::builder()
            .label(title)
            .css_classes(vec!["title-2"])
            .build();

        header.append(&back_btn);
        header.append(&title_lbl);
        container.append(&header);

        // Content
        let content = Box::new(Orientation::Vertical, 0);
        content.set_valign(Align::Center);
        content.set_halign(Align::Center);
        content.set_vexpand(true);

        content.append(&Label::new(Some("This tool is under construction.")));
        
        container.append(&content);
        container
    }
}

pub struct TabbedBoxMaker {
    pub content: Box,
}

impl TabbedBoxMaker {
    pub fn new(stack: &Stack) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);
        
        // Header with Back Button
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder()
            .icon_name("go-previous-symbolic")
            .build();
        
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });

        let title_lbl = Label::builder()
            .label("Tabbed Box Maker")
            .css_classes(vec!["title-2"])
            .build();

        header.append(&back_btn);
        header.append(&title_lbl);
        content_box.append(&header);

        // Scrollable Content
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Dimensions Group
        let dim_group = PreferencesGroup::builder().title("Dimensions").build();
        dim_group.add(&Self::create_entry_row("Width", "100.0"));
        dim_group.add(&Self::create_entry_row("Height", "100.0"));
        dim_group.add(&Self::create_entry_row("Depth", "50.0"));
        dim_group.add(&Self::create_check_row("Outside Dimensions", true));
        scroll_content.append(&dim_group);

        // Material Settings
        let mat_group = PreferencesGroup::builder().title("Material Settings").build();
        mat_group.add(&Self::create_entry_row("Thickness", "3.0"));
        mat_group.add(&Self::create_entry_row("Burn / Tool Dia", "0.1"));
        scroll_content.append(&mat_group);

        // Finger Joint Settings
        let finger_group = PreferencesGroup::builder().title("Finger Joint Settings").build();
        finger_group.add(&Self::create_entry_row("Finger Width", "2.0"));
        finger_group.add(&Self::create_entry_row("Space Width", "2.0"));
        finger_group.add(&Self::create_entry_row("Surrounding Spaces", "2.0"));
        finger_group.add(&Self::create_entry_row("Play (tolerance)", "0.0"));
        finger_group.add(&Self::create_combo_row("Finger Style", &["Rectangular", "Springs", "Barbs", "Snap", "Dogbone"]));
        scroll_content.append(&finger_group);

        // Box Configuration
        let box_group = PreferencesGroup::builder().title("Box Configuration").build();
        box_group.add(&Self::create_combo_row("Box Type", &["Full Box", "No Top", "No Bottom", "No Sides", "No Front/Back", "No Left/Right"]));
        box_group.add(&Self::create_entry_row("Dividers X", "0"));
        box_group.add(&Self::create_entry_row("Dividers Y", "0"));
        box_group.add(&Self::create_check_row("Optimize Layout", false));
        scroll_content.append(&box_group);

        content_box.append(&scrolled);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let generate_btn = Button::with_label("Generate G-Code");
        generate_btn.add_css_class("suggested-action");

        action_box.append(&generate_btn);
        content_box.append(&action_box);

        Self { content: content_box }
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
