use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, ButtonsType, CheckButton, ComboBoxText, Entry, FileChooserAction,
    FileChooserDialog, Image, Label, MessageDialog, MessageType, Orientation, Overlay, Paned,
    ResponseType, ScrolledWindow, Stack,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use tracing::warn;

use crate::ui::gtk::help_browser;
use gcodekit5_core::units::{self};
use gcodekit5_settings::SettingsController;

use gcodekit5_camtools::gerber::{GerberConverter, GerberLayerType, GerberParameters};
use gcodekit5_camtools::jigsaw_puzzle::{JigsawPuzzleMaker, PuzzleParameters};
use gcodekit5_camtools::laser_engraver::{
    BitmapImageEngraver, EngravingParameters, HalftoneMethod, ImageTransformations, RotationAngle,
    ScanDirection,
};
use gcodekit5_camtools::spoilboard_grid::{SpoilboardGridGenerator, SpoilboardGridParameters};
use gcodekit5_camtools::spoilboard_surfacing::{
    SpoilboardSurfacingGenerator, SpoilboardSurfacingParameters,
};
use gcodekit5_camtools::tabbed_box::{
    BoxParameters, BoxType, KeyDividerType, TabbedBoxMaker as Generator,
};
use gcodekit5_camtools::vector_engraver::{VectorEngraver, VectorEngravingParameters};

fn set_paned_initial_fraction(paned: &Paned, fraction: f64) {
    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    paned.connect_map(move |paned| {
        if done2.replace(true) {
            return;
        }
        let paned = paned.clone();
        glib::idle_add_local_once(move || {
            let width = paned.width();
            if width > 0 {
                paned.set_position((width as f64 * fraction) as i32);
            }
        });
    });
}

fn create_dimension_row(
    title: &str,
    initial_mm: f64,
    settings: &Rc<SettingsController>,
) -> (ActionRow, Entry, Label) {
    let row = ActionRow::builder().title(title).build();
    let box_ = Box::new(Orientation::Horizontal, 6);

    let system = settings.persistence.borrow().config().ui.measurement_system;
    let initial_text = units::format_length(initial_mm as f32, system);

    let entry = Entry::builder()
        .text(&initial_text)
        .valign(Align::Center)
        .width_chars(8)
        .build();

    let label = Label::new(Some(units::get_unit_label(system)));
    label.add_css_class("dim-label");

    box_.append(&entry);
    box_.append(&label);
    row.add_suffix(&box_);

    (row, entry, label)
}

pub struct CamToolsView {
    pub content: Stack,
}

impl CamToolsView {
    fn show_error_dialog(title: &str, message: &str) {
        let dialog = MessageDialog::builder()
            .message_type(MessageType::Error)
            .buttons(ButtonsType::Ok)
            .text(title)
            .secondary_text(message)
            .build();
        dialog.connect_response(|d, _| d.close());
        dialog.show();
    }

    pub fn new<F: Fn(String) + 'static>(settings: Rc<SettingsController>, on_generate: F) -> Self {
        let on_generate = Rc::new(on_generate);
        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);

        // Dashboard Page
        let dashboard = Self::create_dashboard(&stack);
        stack.add_named(&dashboard, Some("dashboard"));

        // Tool Pages
        let tabbed_box = TabbedBoxMaker::new(&stack, settings.clone(), on_generate.clone());
        stack.add_named(tabbed_box.widget(), Some("tabbed_box"));

        // Placeholders for other tools
        // Jigsaw Puzzle Tool
        let jigsaw_tool = JigsawTool::new(&stack, settings.clone(), on_generate.clone());
        stack.add_named(jigsaw_tool.widget(), Some("jigsaw"));

        // Bitmap Engraving Tool
        let bitmap_tool = BitmapEngravingTool::new(&stack, settings.clone(), on_generate.clone());
        stack.add_named(bitmap_tool.widget(), Some("laser_image"));

        // Vector Engraving Tool
        let vector_tool = VectorEngravingTool::new(&stack, settings.clone(), on_generate.clone());
        stack.add_named(vector_tool.widget(), Some("laser_vector"));

        // Speeds & Feeds Calculator
        let feeds_tool = SpeedsFeedsTool::new(&stack, settings.clone());
        stack.add_named(feeds_tool.widget(), Some("feeds"));

        // Spoilboard Surfacing
        let surfacing_tool =
            SpoilboardSurfacingTool::new(&stack, settings.clone(), on_generate.clone());
        stack.add_named(surfacing_tool.widget(), Some("surfacing"));

        // Spoilboard Grid
        let grid_tool = SpoilboardGridTool::new(&stack, settings.clone(), on_generate.clone());
        stack.add_named(grid_tool.widget(), Some("grid"));

        // Gerber Tool
        let gerber_tool = GerberTool::new(&stack, settings.clone(), on_generate.clone());
        stack.add_named(gerber_tool.widget(), Some("gerber"));

        Self { content: stack }
    }

    pub fn widget(&self) -> &Stack {
        &self.content
    }

    fn create_dashboard(stack: &Stack) -> Box {
        // Compact dashboard: tool list (left) + details panel (right)
        // with search + category filtering.
        #[derive(Clone, Copy)]
        struct Tool {
            page: &'static str,
            title: &'static str,
            desc: &'static str,
            icon: &'static str,
            category: &'static str,
        }

        const TOOLS: &[Tool] = &[
            Tool {
                page: "tabbed_box",
                title: "Tabbed Box Maker",
                desc: "Generate G-code for laser/CNC cut boxes with finger joints",
                icon: "object-select-symbolic",
                category: "generators",
            },
            Tool {
                page: "jigsaw",
                title: "Jigsaw Puzzle Generator",
                desc: "Create custom jigsaw puzzle patterns from images",
                icon: "image-x-generic-symbolic",
                category: "generators",
            },
            Tool {
                page: "laser_image",
                title: "Laser Image Engraver",
                desc: "Convert raster images to G-code for laser engraving",
                icon: "camera-photo-symbolic",
                category: "engraving",
            },
            Tool {
                page: "laser_vector",
                title: "Laser Vector Engraver",
                desc: "Convert SVG and DXF vector files to G-code",
                icon: "insert-image-symbolic",
                category: "engraving",
            },
            Tool {
                page: "feeds",
                title: "Speeds & Feeds Calculator",
                desc: "Calculate cutting speeds and feeds for your materials",
                icon: "accessories-calculator-symbolic",
                category: "calculators",
            },
            Tool {
                page: "surfacing",
                title: "Spoilboard Surfacing",
                desc: "Generate surfacing toolpaths to flatten your spoilboard",
                icon: "view-refresh-symbolic",
                category: "maintenance",
            },
            Tool {
                page: "grid",
                title: "Create Spoilboard Grid",
                desc: "Generate grid patterns for spoilboard alignment",
                icon: "view-grid-symbolic",
                category: "maintenance",
            },
            Tool {
                page: "gerber",
                title: "Gerber to G-Code",
                desc: "Convert Gerber files to G-Code for PCB milling",
                icon: "media-floppy-symbolic",
                category: "generators",
            },
        ];

        fn apply_filters(list: &gtk4::ListBox, query: &str, category: &str) {
            let q = query.trim().to_lowercase();

            let mut child = list.first_child();
            while let Some(w) = child {
                child = w.next_sibling();
                let Ok(row) = w.downcast::<gtk4::ListBoxRow>() else {
                    continue;
                };
                let idx = unsafe {
                    row.data::<u32>("camtool-index")
                        .map(|p| *p.as_ref() as usize)
                };
                let Some(idx) = idx else {
                    continue;
                };
                let Some(tool) = TOOLS.get(idx) else {
                    continue;
                };

                let matches_text = q.is_empty()
                    || tool.title.to_lowercase().contains(&q)
                    || tool.desc.to_lowercase().contains(&q);
                let matches_cat = category == "all" || tool.category == category;
                row.set_visible(matches_text && matches_cat);
            }
        }

        let container = Box::new(Orientation::Vertical, 12);
        container.set_margin_top(24);
        container.set_margin_bottom(24);
        container.set_margin_start(24);
        container.set_margin_end(24);
        container.set_hexpand(true);
        container.set_vexpand(true);

        let header = Box::new(Orientation::Vertical, 6);
        let title = Label::builder()
            .label("CAM Tools")
            .css_classes(vec!["title-1"])
            .halign(Align::Start)
            .build();
        header.append(&title);

        // Toolbar: Search + Category filter
        let toolbar = Box::new(Orientation::Horizontal, 12);
        toolbar.set_hexpand(true);

        let search = gtk4::SearchEntry::builder()
            .placeholder_text("Search tools…")
            .hexpand(true)
            .build();

        let category = ComboBoxText::new();
        category.append(Some("all"), "All");
        category.append(Some("generators"), "Generators");
        category.append(Some("engraving"), "Engraving");
        category.append(Some("calculators"), "Calculators");
        category.append(Some("maintenance"), "Maintenance");
        category.set_active_id(Some("all"));

        toolbar.append(&search);
        toolbar.append(&category);
        header.append(&toolbar);
        container.append(&header);

        // Main: tool list + details.
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        let list = gtk4::ListBox::new();
        list.add_css_class("boxed-list");

        for (idx, tool) in TOOLS.iter().enumerate() {
            let row = gtk4::ListBoxRow::new();
            row.set_selectable(true);
            unsafe {
                row.set_data("camtool-index", idx as u32);
            }

            let h = Box::new(Orientation::Horizontal, 12);
            h.set_margin_top(10);
            h.set_margin_bottom(10);
            h.set_margin_start(12);
            h.set_margin_end(12);

            let icon = Image::from_icon_name(tool.icon);
            icon.set_pixel_size(24);
            icon.set_valign(Align::Start);

            let v = Box::new(Orientation::Vertical, 2);
            let t = Label::new(Some(tool.title));
            t.set_xalign(0.0);
            t.add_css_class("heading");

            let d = Label::new(Some(tool.desc));
            d.set_xalign(0.0);
            d.set_wrap(true);
            d.add_css_class("caption");

            v.append(&t);
            v.append(&d);
            h.append(&icon);
            h.append(&v);

            row.set_child(Some(&h));
            list.append(&row);
        }

        let list_scroller = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&list)
            .min_content_width(320)
            .build();
        list_scroller.set_vexpand(true);
        list_scroller.set_hexpand(true);

        let details = Box::new(Orientation::Vertical, 12);
        details.set_margin_top(12);
        details.set_margin_bottom(12);
        details.set_margin_start(12);
        details.set_margin_end(12);
        details.set_hexpand(true);
        details.set_vexpand(true);

        let details_title = Label::new(Some("Select a tool"));
        details_title.set_xalign(0.0);
        details_title.add_css_class("title-2");

        let details_desc = Label::new(Some("Choose a tool from the list to see details."));
        details_desc.set_xalign(0.0);
        details_desc.set_wrap(true);

        let open_btn = Button::with_label("Open");
        open_btn.add_css_class("suggested-action");
        open_btn.set_sensitive(false);

        details.append(&details_title);
        details.append(&details_desc);
        details.append(&open_btn);

        paned.set_start_child(Some(&list_scroller));
        paned.set_end_child(Some(&details));

        // Initial ratio only; user resizing should persist for the session.
        set_paned_initial_fraction(&paned, 0.45);

        // Selection -> details
        {
            let details_title = details_title.clone();
            let details_desc = details_desc.clone();
            let open_btn = open_btn.clone();
            list.connect_row_selected(move |_, row| {
                if let Some(row) = row {
                    let idx = unsafe {
                        row.data::<u32>("camtool-index")
                            .map(|p| *p.as_ref() as usize)
                    };
                    if let Some(idx) = idx {
                        if let Some(tool) = TOOLS.get(idx) {
                            details_title.set_text(tool.title);
                            details_desc.set_text(tool.desc);
                        }
                    }
                    open_btn.set_sensitive(true);
                } else {
                    details_title.set_text("Select a tool");
                    details_desc.set_text("Choose a tool from the list to see details.");
                    open_btn.set_sensitive(false);
                }
            });
        }

        // Open selected tool
        {
            let stack_for_click = stack.clone();
            let list_for_click = list.clone();
            open_btn.connect_clicked(move |_| {
                if let Some(row) = list_for_click.selected_row() {
                    let idx = unsafe {
                        row.data::<u32>("camtool-index")
                            .map(|p| *p.as_ref() as usize)
                    };
                    if let Some(idx) = idx {
                        if let Some(tool) = TOOLS.get(idx) {
                            stack_for_click.set_visible_child_name(tool.page);
                        }
                    }
                }
            });

            let stack_for_activate = stack.clone();
            list.connect_row_activated(move |_, row| {
                let idx = unsafe {
                    row.data::<u32>("camtool-index")
                        .map(|p| *p.as_ref() as usize)
                };
                if let Some(idx) = idx {
                    if let Some(tool) = TOOLS.get(idx) {
                        stack_for_activate.set_visible_child_name(tool.page);
                    }
                }
            });
        }

        // Filtering
        {
            let list = list.clone();
            let search = search.clone();
            let category = category.clone();
            search.connect_search_changed(move |s| {
                apply_filters(
                    &list,
                    &s.text(),
                    &category.active_id().unwrap_or_else(|| "all".into()),
                );
            });
        }
        {
            let list = list.clone();
            let search = search.clone();
            category.connect_changed(move |c| {
                apply_filters(
                    &list,
                    &search.text(),
                    &c.active_id().unwrap_or_else(|| "all".into()),
                );
            });
        }

        container.append(&paned);

        let root = Box::new(Orientation::Vertical, 0);
        root.set_hexpand(true);
        root.set_vexpand(true);
        root.append(&container);
        root
    }

    #[allow(dead_code)]
    fn create_tool_card(
        title: &str,
        desc: &str,
        icon: &str,
        target_page: &str,
        stack: &Stack,
    ) -> Button {
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

        // Support loading themed icon names and local resource icons.
        // If `icon` is a resource path (e.g., `/com/gcodekit5/icons/whatever.svg`),
        // load it via `Image::from_resource`, otherwise via `Image::from_icon_name`.
        let icon_img = if icon.starts_with('/') {
            Image::from_resource(icon)
        } else {
            Image::from_icon_name(icon)
        };
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
}

struct JigsawWidgets {
    width: Entry,
    height: Entry,
    pieces_across: Entry,
    pieces_down: Entry,
    kerf: Entry,
    seed: Entry,
    tab_size: Entry,
    jitter: Entry,
    corner_radius: Entry,
    passes: Entry,
    power: Entry,
    feed_rate: Entry,
    offset_x: Entry,
    offset_y: Entry,
    home_before: CheckButton,
}

pub struct JigsawTool {
    content: Box,
}

impl JigsawTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        on_generate: Rc<F>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });
        header.append(&back_btn);

        let title = Label::builder()
            .label("Jigsaw Puzzle Generator")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);

        header.append(&help_browser::make_help_button("jigsaw_puzzle"));

        content_box.append(&header);

        // Paned Layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);
        content_box.append(&paned);

        // Sidebar (40%)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_label = Label::builder()
            .label("Jigsaw Puzzle Generator")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Create custom jigsaw puzzle patterns from images or blank material. Features Draradech's algorithm for unique pieces.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Content Area
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Widgets
        let (width_row, width, width_unit) = create_dimension_row("Width:", 200.0, &settings);
        let (height_row, height, height_unit) = create_dimension_row("Height:", 150.0, &settings);
        let pieces_across = Entry::builder().text("4").valign(Align::Center).build();
        let pieces_down = Entry::builder().text("3").valign(Align::Center).build();
        let (kerf_row, kerf, kerf_unit) = create_dimension_row("Kerf:", 0.5, &settings);
        let seed = Entry::builder().text("42").valign(Align::Center).build();
        let tab_size = Entry::builder().text("20").valign(Align::Center).build();
        let jitter = Entry::builder().text("4").valign(Align::Center).build();
        let (corner_radius_row, corner_radius, corner_radius_unit) =
            create_dimension_row("Corner Radius:", 2.0, &settings);
        let passes = Entry::builder().text("3").valign(Align::Center).build();
        let power = Entry::builder().text("1000").valign(Align::Center).build();
        let feed_rate = Entry::builder().text("500").valign(Align::Center).build();
        let (offset_x_row, offset_x, offset_x_unit) =
            create_dimension_row("Offset X:", 10.0, &settings);
        let (offset_y_row, offset_y, offset_y_unit) =
            create_dimension_row("Offset Y:", 10.0, &settings);
        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let dim_group = PreferencesGroup::builder()
            .title("Puzzle Dimensions")
            .build();
        dim_group.add(&width_row);
        dim_group.add(&height_row);
        dim_group.add(&corner_radius_row);
        scroll_content.append(&dim_group);

        let grid_group = PreferencesGroup::builder()
            .title("Grid Configuration")
            .build();
        grid_group.add(&Self::create_row("Pieces Across:", &pieces_across));
        grid_group.add(&Self::create_row("Pieces Down:", &pieces_down));
        scroll_content.append(&grid_group);

        let param_group = PreferencesGroup::builder()
            .title("Puzzle Parameters")
            .build();
        param_group.add(&kerf_row);
        param_group.add(&Self::create_row("Tab Size (%):", &tab_size));
        param_group.add(&Self::create_row("Jitter (%):", &jitter));

        let seed_row = ActionRow::builder().title("Random Seed:").build();
        let seed_box = Box::new(Orientation::Horizontal, 6);
        seed_box.append(&seed);
        let rand_btn = Button::builder()
            .icon_name("media-playlist-shuffle-symbolic")
            .build();
        seed_box.append(&rand_btn);
        seed_row.add_suffix(&seed_box);
        param_group.add(&seed_row);

        scroll_content.append(&param_group);

        let laser_group = PreferencesGroup::builder().title("Laser Settings").build();
        laser_group.add(&Self::create_row("Passes:", &passes));
        laser_group.add(&Self::create_row("Power (S):", &power));
        laser_group.add(&Self::create_row("Feed Rate:", &feed_rate));
        scroll_content.append(&laser_group);

        let offset_group = PreferencesGroup::builder().title("Work Offsets").build();
        offset_group.add(&offset_x_row);
        offset_group.add(&offset_y_row);

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        offset_group.add(&home_row);

        scroll_content.append(&offset_group);

        right_panel.append(&scrolled);

        // Actions
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label("Load");
        let save_btn = Button::with_label("Save");
        let cancel_btn = Button::with_label("Cancel");
        let generate_btn = Button::with_label("Generate");
        generate_btn.add_css_class("suggested-action");
        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_btn);

        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        let widgets = Rc::new(JigsawWidgets {
            width,
            height,
            pieces_across,
            pieces_down,
            kerf,
            seed,
            tab_size,
            jitter,
            corner_radius,
            passes,
            power,
            feed_rate,
            offset_x,
            offset_y,
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let width_unit = width_unit.clone();
            let height_unit = height_unit.clone();
            let kerf_unit = kerf_unit.clone();
            let corner_radius_unit = corner_radius_unit.clone();
            let offset_x_unit = offset_x_unit.clone();
            let offset_y_unit = offset_y_unit.clone();

            let last_system = Rc::new(Cell::new(
                settings.persistence.borrow().config().ui.measurement_system,
            ));

            settings.on_setting_changed(move |key, _| {
                if key == "measurement_system" {
                    let new_system = settings_clone
                        .persistence
                        .borrow()
                        .config()
                        .ui
                        .measurement_system;
                    let old_system = last_system.get();

                    if new_system != old_system {
                        let unit_label = units::get_unit_label(new_system);

                        let update_entry = |entry: &Entry, label: &Label| {
                            if let Ok(val_mm) = units::parse_length(&entry.text(), old_system) {
                                entry.set_text(&units::format_length(val_mm, new_system));
                            }
                            label.set_text(unit_label);
                        };

                        update_entry(&w.width, &width_unit);
                        update_entry(&w.height, &height_unit);
                        update_entry(&w.kerf, &kerf_unit);
                        update_entry(&w.corner_radius, &corner_radius_unit);
                        update_entry(&w.offset_x, &offset_x_unit);
                        update_entry(&w.offset_y, &offset_y_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Connect Generate
        let w_gen = widgets.clone();
        let on_gen = on_generate.clone();
        let settings_gen = settings.clone();
        generate_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&w_gen, &settings_gen);
            let home_before = w_gen.home_before.is_active();

            // Create progress dialog
            let progress_window = gtk4::Window::builder()
                .title("Generating Puzzle")
                .modal(true)
                .default_width(400)
                .default_height(150)
                .resizable(false)
                .build();

            let vbox = Box::new(Orientation::Vertical, 12);
            vbox.set_margin_top(24);
            vbox.set_margin_bottom(24);
            vbox.set_margin_start(24);
            vbox.set_margin_end(24);

            let status_label = Label::new(Some("Generating puzzle pieces..."));
            vbox.append(&status_label);

            let progress_bar = gtk4::ProgressBar::new();
            progress_bar.set_show_text(true);
            progress_bar.set_fraction(0.0);
            vbox.append(&progress_bar);

            let button_box = Box::new(Orientation::Horizontal, 6);
            button_box.set_halign(Align::End);
            let cancel_button = Button::with_label("Cancel");
            button_box.append(&cancel_button);
            vbox.append(&button_box);

            progress_window.set_child(Some(&vbox));
            progress_window.show();

            let on_gen_clone = on_gen.clone();
            let progress_window_clone = progress_window.clone();
            let progress_bar_clone = progress_bar.clone();

            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();

            let cancel_tx_clone = cancel_tx.clone();
            cancel_button.connect_clicked(move |_| {
                let _ = cancel_tx_clone.send(());
            });

            // Spawn background thread
            std::thread::spawn(move || {
                let result = (|| -> Result<String, String> {
                    if cancel_rx.try_recv().is_ok() {
                        return Err("Cancelled by user".to_string());
                    }
                    let mut maker = JigsawPuzzleMaker::new(params)?;
                    maker.generate()?;
                    let mut gcode = maker.to_gcode(500.0, 1.0);

                    // Handle homing
                    gcode = gcode.replace("$H\n", "").replace("$H", "");
                    if home_before {
                        gcode = format!("$H\n{}", gcode);
                    }

                    Ok(gcode)
                })();

                let _ = result_tx.send(result);
            });

            // Simulate progress since JigsawPuzzleMaker doesn't have progress callback yet
            let mut progress = 0.0;
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                // Check for result
                if let Ok(result) = result_rx.try_recv() {
                    progress_window_clone.close();

                    match result {
                        Ok(gcode) => {
                            on_gen_clone(gcode);
                        }
                        Err(e) => {
                            CamToolsView::show_error_dialog(
                                "Puzzle Generation Failed",
                                &format!("Failed to generate puzzle: {}", e),
                            );
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    // Simulate progress
                    progress += 0.05;
                    if progress > 0.95 {
                        progress = 0.95;
                    }
                    progress_bar_clone.set_fraction(progress);
                    progress_bar_clone.set_text(Some(&format!("{:.0}%", progress * 100.0)));
                    glib::ControlFlow::Continue
                }
            });
        });

        // Seed Randomizer
        let s_gen = widgets.clone();
        rand_btn.connect_clicked(move |_| {
            let now = std::time::SystemTime::now();
            let seed = now
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            let new_seed = seed % 100000;
            s_gen.seed.set_text(&new_seed.to_string());
        });

        // Save
        let w_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&w_save, &settings_save);
            Self::save_params(&params);
        });

        // Load
        let w_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            Self::load_params(&w_load, &settings_load);
        });

        // Cancel
        let stack_clone_cancel = stack.clone();
        cancel_btn.connect_clicked(move |_| {
            stack_clone_cancel.set_visible_child_name("dashboard");
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn create_row(title: &str, widget: &impl IsA<gtk4::Widget>) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        row.add_suffix(widget);
        row
    }

    fn collect_params(w: &JigsawWidgets, settings: &Rc<SettingsController>) -> PuzzleParameters {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        PuzzleParameters {
            width: units::parse_length(&w.width.text(), system).unwrap_or(200.0),
            height: units::parse_length(&w.height.text(), system).unwrap_or(150.0),
            pieces_across: w.pieces_across.text().parse().unwrap_or(4),
            pieces_down: w.pieces_down.text().parse().unwrap_or(3),
            kerf: units::parse_length(&w.kerf.text(), system).unwrap_or(0.5),
            seed: w.seed.text().parse::<u32>().unwrap_or(42), // Handles empty or invalid
            tab_size_percent: w.tab_size.text().parse().unwrap_or(20.0),
            jitter_percent: w.jitter.text().parse().unwrap_or(4.0),
            corner_radius: units::parse_length(&w.corner_radius.text(), system).unwrap_or(2.0),
            laser_passes: w.passes.text().parse().unwrap_or(3),
            laser_power: w.power.text().parse().unwrap_or(1000),
            feed_rate: w.feed_rate.text().parse().unwrap_or(500.0),
            offset_x: units::parse_length(&w.offset_x.text(), system).unwrap_or(10.0),
            offset_y: units::parse_length(&w.offset_y.text(), system).unwrap_or(10.0),
        }
    }

    fn save_params(params: &PuzzleParameters) {
        let dialog = FileChooserDialog::new(
            Some("Save Puzzle Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        dialog.set_current_name("puzzle_params.json");

        let params_clone = params.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(json) = serde_json::to_string_pretty(&params_clone) {
                            let _ = fs::write(path, json);
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn load_params(w: &Rc<JigsawWidgets>, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Load Puzzle Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Open,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        let w_clone = w.clone();
        let settings_clone = settings.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(content) = fs::read_to_string(path) {
                            if let Ok(params) = serde_json::from_str::<PuzzleParameters>(&content) {
                                Self::apply_params(&w_clone, &params, &settings_clone);
                            }
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn apply_params(w: &JigsawWidgets, p: &PuzzleParameters, settings: &Rc<SettingsController>) {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        w.width
            .set_text(&units::format_length(p.width as f32, system));
        w.height
            .set_text(&units::format_length(p.height as f32, system));
        w.pieces_across.set_text(&p.pieces_across.to_string());
        w.pieces_down.set_text(&p.pieces_down.to_string());
        w.kerf
            .set_text(&units::format_length(p.kerf as f32, system));
        w.seed.set_text(&p.seed.to_string());
        w.tab_size.set_text(&p.tab_size_percent.to_string());
        w.jitter.set_text(&p.jitter_percent.to_string());
        w.corner_radius
            .set_text(&units::format_length(p.corner_radius as f32, system));
        w.passes.set_text(&p.laser_passes.to_string());
        w.power.set_text(&p.laser_power.to_string());
        w.feed_rate.set_text(&p.feed_rate.to_string());
        w.offset_x
            .set_text(&units::format_length(p.offset_x as f32, system));
        w.offset_y
            .set_text(&units::format_length(p.offset_y as f32, system));
    }
}

// Bitmap Engraving Tool
struct BitmapEngravingWidgets {
    width_mm: Entry,
    feed_rate: Entry,
    travel_rate: Entry,
    min_power: Entry,
    max_power: Entry,
    pixels_per_mm: Entry,
    line_spacing: Entry,
    power_scale: Entry,
    offset_x: Entry,
    offset_y: Entry,
    scan_direction: ComboBoxText,
    bidirectional: CheckButton,
    invert: CheckButton,
    mirror_x: CheckButton,
    mirror_y: CheckButton,
    rotation: ComboBoxText,
    halftone: ComboBoxText,
    halftone_dot_size: Entry,
    halftone_threshold: Entry,
    image_path: Entry,
    preview_image: gtk4::Picture,
    preview_spinner: gtk4::Spinner,
    home_before: CheckButton,
}

pub struct BitmapEngravingTool {
    content: Box,
}

impl BitmapEngravingTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        on_generate: Rc<F>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });
        header.append(&back_btn);

        let title = Label::builder()
            .label("Laser Image Engraver")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);

        header.append(&help_browser::make_help_button("laser_image_engraver"));

        content_box.append(&header);

        // Paned Layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);
        content_box.append(&paned);

        // Sidebar with Preview (40%)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_label = Label::builder()
            .label("Bitmap Engraving")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Convert bitmap images to G-code for laser engraving. Supports various halftoning methods and image transformations.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Preview Image with spinner overlay
        let preview_overlay = Overlay::new();
        let preview_image = gtk4::Picture::new();
        preview_image.set_can_shrink(true);
        preview_image.set_vexpand(true);
        preview_image.set_hexpand(true);
        preview_overlay.set_child(Some(&preview_image));

        // Loading spinner
        let preview_spinner = gtk4::Spinner::new();
        preview_spinner.set_halign(Align::Center);
        preview_spinner.set_valign(Align::Center);
        preview_spinner.set_size_request(48, 48);
        preview_spinner.set_visible(false);
        preview_overlay.add_overlay(&preview_spinner);

        sidebar.append(&preview_overlay);

        // Content Area
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Create Widgets
        let image_path = Entry::builder()
            .placeholder_text("No image selected")
            .valign(Align::Center)
            .build();
        let (width_mm_row, width_mm, width_mm_unit) =
            create_dimension_row("Width:", 100.0, &settings);
        let feed_rate = Entry::builder().text("1000").valign(Align::Center).build();
        let travel_rate = Entry::builder().text("3000").valign(Align::Center).build();
        let min_power = Entry::builder().text("0").valign(Align::Center).build();
        let max_power = Entry::builder().text("100").valign(Align::Center).build();
        let pixels_per_mm = Entry::builder().text("10").valign(Align::Center).build();
        let line_spacing = Entry::builder().text("1.0").valign(Align::Center).build();
        let power_scale = Entry::builder().text("1000").valign(Align::Center).build();
        let (offset_x_row, offset_x, offset_x_unit) =
            create_dimension_row("Offset X:", 10.0, &settings);
        let (offset_y_row, offset_y, offset_y_unit) =
            create_dimension_row("Offset Y:", 10.0, &settings);

        let scan_direction = ComboBoxText::new();
        scan_direction.append(Some("0"), "Horizontal");
        scan_direction.append(Some("1"), "Vertical");
        scan_direction.set_active_id(Some("0"));
        scan_direction.set_valign(Align::Center);

        let bidirectional = CheckButton::builder()
            .active(true)
            .valign(Align::Center)
            .build();
        let invert = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let mirror_x = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let mirror_y = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        let rotation = ComboBoxText::new();
        rotation.append(Some("0"), "0°");
        rotation.append(Some("90"), "90°");
        rotation.append(Some("180"), "180°");
        rotation.append(Some("270"), "270°");
        rotation.set_active_id(Some("0"));
        rotation.set_valign(Align::Center);

        let halftone = ComboBoxText::new();
        halftone.append(Some("none"), "None");
        halftone.append(Some("threshold"), "Threshold");
        halftone.append(Some("bayer"), "Bayer 4x4");
        halftone.append(Some("floyd"), "Floyd-Steinberg");
        halftone.append(Some("atkinson"), "Atkinson");
        halftone.set_active_id(Some("none"));
        halftone.set_valign(Align::Center);

        let halftone_dot_size = Entry::builder().text("4").valign(Align::Center).build();
        let halftone_threshold = Entry::builder().text("127").valign(Align::Center).build();
        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let image_group = PreferencesGroup::builder().title("Image File").build();
        let image_row = ActionRow::builder().title("Image Path:").build();
        let image_box = Box::new(Orientation::Horizontal, 6);
        image_box.append(&image_path);
        let load_image_btn = Button::builder().label("Browse...").build();
        image_box.append(&load_image_btn);
        image_row.add_suffix(&image_box);
        image_group.add(&image_row);
        scroll_content.append(&image_group);

        let output_group = PreferencesGroup::builder().title("Output Settings").build();
        output_group.add(&width_mm_row);
        output_group.add(&Self::create_row("Feed Rate:", &feed_rate));
        output_group.add(&Self::create_row("Travel Rate:", &travel_rate));
        scroll_content.append(&output_group);

        let power_group = PreferencesGroup::builder().title("Laser Power").build();
        power_group.add(&Self::create_row("Min Power (%):", &min_power));
        power_group.add(&Self::create_row("Max Power (%):", &max_power));
        power_group.add(&Self::create_row("Power Scale (S):", &power_scale));
        scroll_content.append(&power_group);

        let scan_group = PreferencesGroup::builder().title("Scanning").build();
        scan_group.add(&Self::create_row("Scan Direction:", &scan_direction));
        scan_group.add(&Self::create_row("Pixels per mm:", &pixels_per_mm));
        scan_group.add(&Self::create_row("Line Spacing:", &line_spacing));
        let bid_row = ActionRow::builder().title("Bidirectional:").build();
        bid_row.add_suffix(&bidirectional);
        scan_group.add(&bid_row);
        scroll_content.append(&scan_group);

        let transform_group = PreferencesGroup::builder()
            .title("Image Transformations")
            .build();
        let invert_row = ActionRow::builder().title("Invert:").build();
        invert_row.add_suffix(&invert);
        transform_group.add(&invert_row);
        let mirror_x_row = ActionRow::builder().title("Mirror X:").build();
        mirror_x_row.add_suffix(&mirror_x);
        transform_group.add(&mirror_x_row);
        let mirror_y_row = ActionRow::builder().title("Mirror Y:").build();
        mirror_y_row.add_suffix(&mirror_y);
        transform_group.add(&mirror_y_row);
        transform_group.add(&Self::create_row("Rotation:", &rotation));
        scroll_content.append(&transform_group);

        let halftone_group = PreferencesGroup::builder().title("Halftoning").build();
        halftone_group.add(&Self::create_row("Method:", &halftone));
        halftone_group.add(&Self::create_row("Dot Size:", &halftone_dot_size));
        halftone_group.add(&Self::create_row("Threshold:", &halftone_threshold));
        scroll_content.append(&halftone_group);

        let offset_group = PreferencesGroup::builder().title("Work Offsets").build();
        offset_group.add(&offset_x_row);
        offset_group.add(&offset_y_row);

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        offset_group.add(&home_row);

        scroll_content.append(&offset_group);

        right_panel.append(&scrolled);

        // Actions
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label("Load");
        let save_btn = Button::with_label("Save");
        let cancel_btn = Button::with_label("Cancel");
        let generate_btn = Button::with_label("Generate");
        generate_btn.add_css_class("suggested-action");
        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_btn);

        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        let widgets = Rc::new(BitmapEngravingWidgets {
            width_mm,
            feed_rate,
            travel_rate,
            min_power,
            max_power,
            pixels_per_mm,
            line_spacing,
            power_scale,
            offset_x,
            offset_y,
            scan_direction,
            bidirectional,
            invert,
            mirror_x,
            mirror_y,
            rotation,
            halftone,
            halftone_dot_size,
            halftone_threshold,
            image_path,
            preview_image: preview_image.clone(),
            preview_spinner: preview_spinner.clone(),
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let width_mm_unit = width_mm_unit.clone();
            let offset_x_unit = offset_x_unit.clone();
            let offset_y_unit = offset_y_unit.clone();

            let last_system = Rc::new(Cell::new(
                settings.persistence.borrow().config().ui.measurement_system,
            ));

            settings.on_setting_changed(move |key, _| {
                if key == "measurement_system" {
                    let new_system = settings_clone
                        .persistence
                        .borrow()
                        .config()
                        .ui
                        .measurement_system;
                    let old_system = last_system.get();

                    if new_system != old_system {
                        let unit_label = units::get_unit_label(new_system);

                        let update_entry = |entry: &Entry, label: &Label| {
                            if let Ok(val_mm) = units::parse_length(&entry.text(), old_system) {
                                entry.set_text(&units::format_length(val_mm, new_system));
                            }
                            label.set_text(unit_label);
                        };

                        update_entry(&w.width_mm, &width_mm_unit);
                        update_entry(&w.offset_x, &offset_x_unit);
                        update_entry(&w.offset_y, &offset_y_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Load Image Button
        let w_load_image = widgets.clone();
        load_image_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Select Image"),
                None::<&gtk4::Window>,
                FileChooserAction::Open,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Open", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(900, 700);

            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Image Files"));
            filter.add_mime_type("image/png");
            filter.add_mime_type("image/jpeg");
            filter.add_mime_type("image/bmp");
            filter.add_mime_type("image/gif");
            filter.add_mime_type("image/tiff");
            dialog.add_filter(&filter);

            let w_clone = w_load_image.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            w_clone.image_path.set_text(&path.display().to_string());

                            // Show spinner and load preview in background
                            w_clone.preview_spinner.set_visible(true);
                            w_clone.preview_spinner.start();

                            let preview_img = w_clone.preview_image.clone();
                            let spinner = w_clone.preview_spinner.clone();
                            let path_clone = path.clone();

                            let (tx, rx) = std::sync::mpsc::channel();

                            std::thread::spawn(move || {
                                let file = gtk4::gio::File::for_path(&path_clone);
                                let texture_result = gtk4::gdk::Texture::from_file(&file);
                                let _ = tx.send(texture_result);
                            });

                            glib::timeout_add_local(
                                std::time::Duration::from_millis(50),
                                move || {
                                    if let Ok(texture_result) = rx.try_recv() {
                                        spinner.stop();
                                        spinner.set_visible(false);

                                        if let Ok(texture) = texture_result {
                                            preview_img.set_paintable(Some(&texture));
                                        }
                                        glib::ControlFlow::Break
                                    } else {
                                        glib::ControlFlow::Continue
                                    }
                                },
                            );
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // Connect Generate
        let w_gen = widgets.clone();
        let on_gen = on_generate.clone();
        generate_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&w_gen);
            let image_path = w_gen.image_path.text().to_string();
            let home_before = w_gen.home_before.is_active();

            if image_path.is_empty() {
                CamToolsView::show_error_dialog(
                    "No Image Selected",
                    "Please select an image file first.",
                );
                return;
            }

            // Create progress dialog with progress bar and cancel button
            let progress_window = gtk4::Window::builder()
                .title("Generating Engraving")
                .modal(true)
                .default_width(400)
                .default_height(150)
                .resizable(false)
                .build();

            let vbox = Box::new(Orientation::Vertical, 12);
            vbox.set_margin_top(24);
            vbox.set_margin_bottom(24);
            vbox.set_margin_start(24);
            vbox.set_margin_end(24);

            let status_label = Label::new(Some("Processing image..."));
            vbox.append(&status_label);

            let progress_bar = gtk4::ProgressBar::new();
            progress_bar.set_show_text(true);
            vbox.append(&progress_bar);

            let button_box = Box::new(Orientation::Horizontal, 6);
            button_box.set_halign(Align::End);
            let cancel_button = Button::with_label("Cancel");
            button_box.append(&cancel_button);
            vbox.append(&button_box);

            progress_window.set_child(Some(&vbox));
            progress_window.show();

            // Clone what we need
            let image_path_thread = image_path.clone();
            let on_gen_clone = on_gen.clone();
            let progress_window_clone = progress_window.clone();
            let progress_bar_clone = progress_bar.clone();
            let status_label_clone = status_label.clone();

            // Create channels for progress and result
            let (progress_tx, progress_rx) = std::sync::mpsc::channel();
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();

            // Cancel button handler
            let cancel_tx_clone = cancel_tx.clone();
            cancel_button.connect_clicked(move |_| {
                let _ = cancel_tx_clone.send(());
            });

            // Spawn background thread for generation
            std::thread::spawn(move || {
                let result = BitmapImageEngraver::from_file(&image_path_thread, params)
                    .and_then(|engraver| {
                        engraver.generate_gcode_with_progress(|progress| {
                            // Check for cancellation
                            if cancel_rx.try_recv().is_ok() {
                                return; // Abort
                            }
                            // Send progress update
                            let _ = progress_tx.send(progress);
                        })
                    })
                    .map(|mut gcode| {
                        gcode = gcode.replace("$H\n", "").replace("$H", "");
                        if home_before {
                            format!("$H\n{}", gcode)
                        } else {
                            gcode
                        }
                    });

                // Send result back
                let _ = result_tx.send(result);
            });

            // Poll for progress and result on main thread
            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                // Check for progress updates
                while let Ok(progress) = progress_rx.try_recv() {
                    progress_bar_clone.set_fraction(progress as f64);
                    progress_bar_clone.set_text(Some(&format!("{:.0}%", progress * 100.0)));

                    // Update status label based on progress
                    if progress < 0.1 {
                        status_label_clone.set_text("Loading image...");
                    } else if progress < 0.9 {
                        status_label_clone.set_text("Generating G-code...");
                    } else {
                        status_label_clone.set_text("Finalizing...");
                    }
                }

                // Check for result
                if let Ok(result) = result_rx.try_recv() {
                    progress_window_clone.close();

                    match result {
                        Ok(gcode) => {
                            on_gen_clone(gcode);
                        }
                        Err(e) => {
                            CamToolsView::show_error_dialog(
                                "Engraving Generation Failed",
                                &format!("Failed to generate engraving: {}", e),
                            );
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        });

        // Save params
        let w_save = widgets.clone();
        save_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Save Parameters"),
                None::<&gtk4::Window>,
                FileChooserAction::Save,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Save", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(900, 700);
            dialog.set_current_name("bitmap_params.json");

            let w_clone = w_save.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let params = Self::collect_params_for_save(&w_clone);
                            if let Ok(json) = serde_json::to_string_pretty(&params) {
                                let _ = fs::write(path, json);
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // Load params
        let w_load = widgets.clone();
        load_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Load Parameters"),
                None::<&gtk4::Window>,
                FileChooserAction::Open,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Open", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(900, 700);

            let w_clone = w_load.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            if let Ok(content) = fs::read_to_string(path) {
                                if let Ok(params) =
                                    serde_json::from_str::<serde_json::Value>(&content)
                                {
                                    Self::apply_params(&w_clone, &params);
                                }
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // Cancel
        let stack_clone_cancel = stack.clone();
        cancel_btn.connect_clicked(move |_| {
            stack_clone_cancel.set_visible_child_name("dashboard");
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn create_row(title: &str, widget: &impl IsA<gtk4::Widget>) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        row.add_suffix(widget);
        row
    }

    fn collect_params(w: &BitmapEngravingWidgets) -> EngravingParameters {
        let rotation = match w.rotation.active_id().as_ref().map(|s| s.as_str()) {
            Some("90") => RotationAngle::Degrees90,
            Some("180") => RotationAngle::Degrees180,
            Some("270") => RotationAngle::Degrees270,
            _ => RotationAngle::Degrees0,
        };

        let halftone = match w.halftone.active_id().as_ref().map(|s| s.as_str()) {
            Some("threshold") => HalftoneMethod::Threshold,
            Some("bayer") => HalftoneMethod::Bayer4x4,
            Some("floyd") => HalftoneMethod::FloydSteinberg,
            Some("atkinson") => HalftoneMethod::Atkinson,
            _ => HalftoneMethod::None,
        };

        let scan_direction =
            if w.scan_direction.active_id().as_ref().map(|s| s.as_str()) == Some("1") {
                ScanDirection::Vertical
            } else {
                ScanDirection::Horizontal
            };

        EngravingParameters {
            width_mm: w.width_mm.text().parse().unwrap_or(100.0),
            height_mm: None,
            feed_rate: w.feed_rate.text().parse().unwrap_or(1000.0),
            travel_rate: w.travel_rate.text().parse().unwrap_or(3000.0),
            min_power: w.min_power.text().parse().unwrap_or(0.0),
            max_power: w.max_power.text().parse().unwrap_or(100.0),
            pixels_per_mm: w.pixels_per_mm.text().parse().unwrap_or(10.0),
            scan_direction,
            bidirectional: w.bidirectional.is_active(),
            line_spacing: w.line_spacing.text().parse().unwrap_or(1.0),
            power_scale: w.power_scale.text().parse().unwrap_or(1000.0),
            transformations: ImageTransformations {
                mirror_x: w.mirror_x.is_active(),
                mirror_y: w.mirror_y.is_active(),
                rotation,
                halftone,
                halftone_dot_size: w.halftone_dot_size.text().parse().unwrap_or(4),
                halftone_threshold: w.halftone_threshold.text().parse().unwrap_or(127),
                invert: w.invert.is_active(),
            },
            offset_x: w.offset_x.text().parse().unwrap_or(10.0),
            offset_y: w.offset_y.text().parse().unwrap_or(10.0),
        }
    }

    fn collect_params_for_save(w: &BitmapEngravingWidgets) -> serde_json::Value {
        serde_json::json!({
            "image_path": w.image_path.text().to_string(),
            "width_mm": w.width_mm.text().to_string(),
            "feed_rate": w.feed_rate.text().to_string(),
            "travel_rate": w.travel_rate.text().to_string(),
            "min_power": w.min_power.text().to_string(),
            "max_power": w.max_power.text().to_string(),
            "pixels_per_mm": w.pixels_per_mm.text().to_string(),
            "line_spacing": w.line_spacing.text().to_string(),
            "power_scale": w.power_scale.text().to_string(),
            "offset_x": w.offset_x.text().to_string(),
            "offset_y": w.offset_y.text().to_string(),
            "scan_direction": w.scan_direction.active_id().unwrap_or_default().to_string(),
            "bidirectional": w.bidirectional.is_active(),
            "invert": w.invert.is_active(),
            "mirror_x": w.mirror_x.is_active(),
            "mirror_y": w.mirror_y.is_active(),
            "rotation": w.rotation.active_id().unwrap_or_default().to_string(),
            "halftone": w.halftone.active_id().unwrap_or_default().to_string(),
            "halftone_dot_size": w.halftone_dot_size.text().to_string(),
            "halftone_threshold": w.halftone_threshold.text().to_string(),
        })
    }

    fn apply_params(w: &BitmapEngravingWidgets, params: &serde_json::Value) {
        if let Some(image_path) = params.get("image_path").and_then(|v| v.as_str()) {
            w.image_path.set_text(image_path);

            // Show spinner and load preview in background
            w.preview_spinner.set_visible(true);
            w.preview_spinner.start();

            let preview_img = w.preview_image.clone();
            let spinner = w.preview_spinner.clone();
            let path = image_path.to_string();

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let file = gtk4::gio::File::for_path(&path);
                let texture_result = gtk4::gdk::Texture::from_file(&file);
                let _ = tx.send(texture_result);
            });

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                if let Ok(texture_result) = rx.try_recv() {
                    spinner.stop();
                    spinner.set_visible(false);

                    if let Ok(texture) = texture_result {
                        preview_img.set_paintable(Some(&texture));
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        }
        if let Some(v) = params.get("width_mm").and_then(|v| v.as_str()) {
            w.width_mm.set_text(v);
        }
        if let Some(v) = params.get("feed_rate").and_then(|v| v.as_str()) {
            w.feed_rate.set_text(v);
        }
        if let Some(v) = params.get("travel_rate").and_then(|v| v.as_str()) {
            w.travel_rate.set_text(v);
        }
        if let Some(v) = params.get("min_power").and_then(|v| v.as_str()) {
            w.min_power.set_text(v);
        }
        if let Some(v) = params.get("max_power").and_then(|v| v.as_str()) {
            w.max_power.set_text(v);
        }
        if let Some(v) = params.get("pixels_per_mm").and_then(|v| v.as_str()) {
            w.pixels_per_mm.set_text(v);
        }
        if let Some(v) = params.get("line_spacing").and_then(|v| v.as_str()) {
            w.line_spacing.set_text(v);
        }
        if let Some(v) = params.get("power_scale").and_then(|v| v.as_str()) {
            w.power_scale.set_text(v);
        }
        if let Some(v) = params.get("offset_x").and_then(|v| v.as_str()) {
            w.offset_x.set_text(v);
        }
        if let Some(v) = params.get("offset_y").and_then(|v| v.as_str()) {
            w.offset_y.set_text(v);
        }
        if let Some(v) = params.get("scan_direction").and_then(|v| v.as_str()) {
            w.scan_direction.set_active_id(Some(v));
        }
        if let Some(v) = params.get("bidirectional").and_then(|v| v.as_bool()) {
            w.bidirectional.set_active(v);
        }
        if let Some(v) = params.get("invert").and_then(|v| v.as_bool()) {
            w.invert.set_active(v);
        }
        if let Some(v) = params.get("mirror_x").and_then(|v| v.as_bool()) {
            w.mirror_x.set_active(v);
        }
        if let Some(v) = params.get("mirror_y").and_then(|v| v.as_bool()) {
            w.mirror_y.set_active(v);
        }
        if let Some(v) = params.get("rotation").and_then(|v| v.as_str()) {
            w.rotation.set_active_id(Some(v));
        }
        if let Some(v) = params.get("halftone").and_then(|v| v.as_str()) {
            w.halftone.set_active_id(Some(v));
        }
        if let Some(v) = params.get("halftone_dot_size").and_then(|v| v.as_str()) {
            w.halftone_dot_size.set_text(v);
        }
        if let Some(v) = params.get("halftone_threshold").and_then(|v| v.as_str()) {
            w.halftone_threshold.set_text(v);
        }
    }
}

// Vector Engraving Tool
struct VectorEngravingWidgets {
    feed_rate: Entry,
    travel_rate: Entry,
    cut_power: Entry,
    engrave_power: Entry,
    power_scale: Entry,
    multi_pass: CheckButton,
    num_passes: Entry,
    z_increment: Entry,
    invert_power: CheckButton,
    desired_width: Entry,
    offset_x: Entry,
    offset_y: Entry,
    enable_hatch: CheckButton,
    hatch_angle: Entry,
    hatch_spacing: Entry,
    hatch_tolerance: Entry,
    cross_hatch: CheckButton,
    enable_dwell: CheckButton,
    dwell_time: Entry,
    vector_path: Entry,
    preview_image: gtk4::Picture,
    preview_spinner: gtk4::Spinner,
    info_label: Label,
    home_before: CheckButton,
}

pub struct VectorEngravingTool {
    content: Box,
}

impl VectorEngravingTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        on_generate: Rc<F>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });
        header.append(&back_btn);

        let title = Label::builder()
            .label("Laser Vector Engraver")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);

        header.append(&help_browser::make_help_button("laser_vector_engraver"));

        content_box.append(&header);

        // Paned Layout (20% sidebar, 80% content)
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);
        content_box.append(&paned);

        // Sidebar with Preview (40%)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_label = Label::builder()
            .label("Vector Engraving")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Convert vector graphics (SVG, DXF) to G-code for laser cutting/engraving. Supports hatching, multi-pass, and path optimization.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Preview Area
        let preview_container = Box::new(Orientation::Vertical, 6);
        preview_container.set_vexpand(true);

        // Preview image with spinner overlay
        let preview_overlay = Overlay::new();

        // Add light gray background frame
        let preview_frame = gtk4::Frame::new(None::<&str>);
        preview_frame.add_css_class("vector-preview");
        preview_frame.set_vexpand(true);
        preview_frame.set_hexpand(true);

        let preview_image = gtk4::Picture::new();
        preview_image.set_can_shrink(true);
        preview_image.set_vexpand(true);
        preview_image.set_hexpand(true);
        preview_frame.set_child(Some(&preview_image));
        preview_overlay.set_child(Some(&preview_frame));

        // Loading spinner
        let preview_spinner = gtk4::Spinner::new();
        preview_spinner.set_halign(Align::Center);
        preview_spinner.set_valign(Align::Center);
        preview_spinner.set_size_request(48, 48);
        preview_overlay.add_overlay(&preview_spinner);

        preview_container.append(&preview_overlay);

        // Info overlay label
        let info_label = Label::builder()
            .label("No file selected")
            .css_classes(vec!["caption", "dim-label"])
            .halign(Align::Start)
            .wrap(true)
            .build();
        preview_container.append(&info_label);

        sidebar.append(&preview_container);

        // Content Area (80%)
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Create Widgets
        let vector_path = Entry::builder()
            .placeholder_text("No vector file selected")
            .valign(Align::Center)
            .build();
        let feed_rate = Entry::builder().text("600").valign(Align::Center).build();
        let travel_rate = Entry::builder().text("3000").valign(Align::Center).build();
        let cut_power = Entry::builder().text("100").valign(Align::Center).build();
        let engrave_power = Entry::builder().text("50").valign(Align::Center).build();
        let power_scale = Entry::builder().text("1000").valign(Align::Center).build();
        let multi_pass = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let num_passes = Entry::builder().text("1").valign(Align::Center).build();
        let (z_increment_row, z_increment, z_increment_unit) =
            create_dimension_row("Z Increment:", 0.5, &settings);
        let invert_power = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let (desired_width_row, desired_width, desired_width_unit) =
            create_dimension_row("Desired Width:", 100.0, &settings);
        let (offset_x_row, offset_x, offset_x_unit) =
            create_dimension_row("Offset X:", 10.0, &settings);
        let (offset_y_row, offset_y, offset_y_unit) =
            create_dimension_row("Offset Y:", 10.0, &settings);
        let enable_hatch = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let hatch_angle = Entry::builder().text("45").valign(Align::Center).build();
        let (hatch_spacing_row, hatch_spacing, hatch_spacing_unit) =
            create_dimension_row("Hatch Spacing:", 1.0, &settings);
        let (hatch_tolerance_row, hatch_tolerance, hatch_tolerance_unit) =
            create_dimension_row("Hatch Tolerance:", 0.1, &settings);
        let cross_hatch = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let enable_dwell = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let dwell_time = Entry::builder().text("0.1").valign(Align::Center).build();
        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let file_group = PreferencesGroup::builder().title("Vector File").build();
        let file_row = ActionRow::builder().title("File Path:").build();
        let file_box = Box::new(Orientation::Horizontal, 6);
        file_box.append(&vector_path);
        let load_file_btn = Button::builder().label("Browse...").build();
        file_box.append(&load_file_btn);
        file_row.add_suffix(&file_box);
        file_group.add(&file_row);
        scroll_content.append(&file_group);

        let output_group = PreferencesGroup::builder().title("Output Settings").build();
        output_group.add(&desired_width_row);
        output_group.add(&Self::create_row("Feed Rate:", &feed_rate));
        output_group.add(&Self::create_row("Travel Rate:", &travel_rate));
        scroll_content.append(&output_group);

        let power_group = PreferencesGroup::builder().title("Laser Power").build();
        power_group.add(&Self::create_row("Cut Power (%):", &cut_power));
        power_group.add(&Self::create_row("Engrave Power (%):", &engrave_power));
        power_group.add(&Self::create_row("Power Scale (S):", &power_scale));
        let invert_row = ActionRow::builder().title("Invert Power:").build();
        invert_row.add_suffix(&invert_power);
        power_group.add(&invert_row);
        scroll_content.append(&power_group);

        let multipass_group = PreferencesGroup::builder()
            .title("Multi-Pass Settings")
            .build();
        let multi_row = ActionRow::builder().title("Multi-Pass:").build();
        multi_row.add_suffix(&multi_pass);
        multipass_group.add(&multi_row);
        multipass_group.add(&Self::create_row("Number of Passes:", &num_passes));
        multipass_group.add(&z_increment_row);
        scroll_content.append(&multipass_group);

        let hatch_group = PreferencesGroup::builder().title("Hatching").build();
        let hatch_row = ActionRow::builder().title("Enable Hatch:").build();
        hatch_row.add_suffix(&enable_hatch);
        hatch_group.add(&hatch_row);
        hatch_group.add(&Self::create_row("Hatch Angle (°):", &hatch_angle));
        hatch_group.add(&hatch_spacing_row);
        hatch_group.add(&hatch_tolerance_row);
        let cross_row = ActionRow::builder().title("Cross Hatch:").build();
        cross_row.add_suffix(&cross_hatch);
        hatch_group.add(&cross_row);
        scroll_content.append(&hatch_group);

        let dwell_group = PreferencesGroup::builder().title("Dwell Settings").build();
        let dwell_row = ActionRow::builder().title("Enable Dwell:").build();
        dwell_row.add_suffix(&enable_dwell);
        dwell_group.add(&dwell_row);
        dwell_group.add(&Self::create_row("Dwell Time (s):", &dwell_time));
        scroll_content.append(&dwell_group);

        let offset_group = PreferencesGroup::builder().title("Work Offsets").build();
        offset_group.add(&offset_x_row);
        offset_group.add(&offset_y_row);

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        offset_group.add(&home_row);

        scroll_content.append(&offset_group);

        right_panel.append(&scrolled);

        // Actions
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label("Load");
        let save_btn = Button::with_label("Save");
        let cancel_btn = Button::with_label("Cancel");
        let generate_btn = Button::with_label("Generate");
        generate_btn.add_css_class("suggested-action");
        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_btn);

        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        let widgets = Rc::new(VectorEngravingWidgets {
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
            vector_path,
            preview_image: preview_image.clone(),
            preview_spinner: preview_spinner.clone(),
            info_label: info_label.clone(),
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let z_increment_unit = z_increment_unit.clone();
            let desired_width_unit = desired_width_unit.clone();
            let offset_x_unit = offset_x_unit.clone();
            let offset_y_unit = offset_y_unit.clone();
            let hatch_spacing_unit = hatch_spacing_unit.clone();
            let hatch_tolerance_unit = hatch_tolerance_unit.clone();

            let last_system = Rc::new(Cell::new(
                settings.persistence.borrow().config().ui.measurement_system,
            ));

            settings.on_setting_changed(move |key, _| {
                if key == "measurement_system" {
                    let new_system = settings_clone
                        .persistence
                        .borrow()
                        .config()
                        .ui
                        .measurement_system;
                    let old_system = last_system.get();

                    if new_system != old_system {
                        let unit_label = units::get_unit_label(new_system);

                        let update_entry = |entry: &Entry, label: &Label| {
                            if let Ok(val_mm) = units::parse_length(&entry.text(), old_system) {
                                entry.set_text(&units::format_length(val_mm, new_system));
                            }
                            label.set_text(unit_label);
                        };

                        update_entry(&w.z_increment, &z_increment_unit);
                        update_entry(&w.desired_width, &desired_width_unit);
                        update_entry(&w.offset_x, &offset_x_unit);
                        update_entry(&w.offset_y, &offset_y_unit);
                        update_entry(&w.hatch_spacing, &hatch_spacing_unit);
                        update_entry(&w.hatch_tolerance, &hatch_tolerance_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Load File Button
        let w_load_file = widgets.clone();
        load_file_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Select Vector File"),
                None::<&gtk4::Window>,
                FileChooserAction::Open,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Open", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(900, 700);

            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Vector Files"));
            filter.add_pattern("*.svg");
            filter.add_pattern("*.dxf");
            dialog.add_filter(&filter);

            let w_clone = w_load_file.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            w_clone.vector_path.set_text(&path.display().to_string());
                            Self::load_vector_preview(&w_clone, &path);
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // Connect Generate
        let w_gen = widgets.clone();
        let on_gen = on_generate.clone();
        let settings_gen = settings.clone();
        generate_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&w_gen, &settings_gen);
            let vector_path = w_gen.vector_path.text().to_string();
            let home_before = w_gen.home_before.is_active();

            if vector_path.is_empty() {
                CamToolsView::show_error_dialog(
                    "No Vector File Selected",
                    "Please select a vector file first.",
                );
                return;
            }

            // Create progress dialog
            let progress_window = gtk4::Window::builder()
                .title("Generating Vector Engraving")
                .modal(true)
                .default_width(400)
                .default_height(150)
                .resizable(false)
                .build();

            let vbox = Box::new(Orientation::Vertical, 12);
            vbox.set_margin_top(24);
            vbox.set_margin_bottom(24);
            vbox.set_margin_start(24);
            vbox.set_margin_end(24);

            let status_label = Label::new(Some("Loading vector file..."));
            vbox.append(&status_label);

            let progress_bar = gtk4::ProgressBar::new();
            progress_bar.set_show_text(true);
            vbox.append(&progress_bar);

            let button_box = Box::new(Orientation::Horizontal, 6);
            button_box.set_halign(Align::End);
            let cancel_button = Button::with_label("Cancel");
            button_box.append(&cancel_button);
            vbox.append(&button_box);

            progress_window.set_child(Some(&vbox));
            progress_window.show();

            let on_gen_clone = on_gen.clone();
            let progress_window_clone = progress_window.clone();
            let progress_bar_clone = progress_bar.clone();
            let status_label_clone = status_label.clone();

            let (progress_tx, progress_rx) = std::sync::mpsc::channel();
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();

            let cancel_tx_clone = cancel_tx.clone();
            cancel_button.connect_clicked(move |_| {
                let _ = cancel_tx_clone.send(());
            });

            // Spawn background thread
            std::thread::spawn(move || {
                let result = VectorEngraver::from_file(&vector_path, params)
                    .and_then(|engraver| {
                        engraver.generate_gcode_with_progress(|progress| {
                            if cancel_rx.try_recv().is_ok() {
                                return;
                            }
                            let _ = progress_tx.send(progress);
                        })
                    })
                    .map(|mut gcode| {
                        gcode = gcode.replace("$H\n", "").replace("$H", "");
                        if home_before {
                            format!("$H\n{}", gcode)
                        } else {
                            gcode
                        }
                    });

                let _ = result_tx.send(result);
            });

            // Poll for progress and result
            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                // Check for progress updates
                while let Ok(progress) = progress_rx.try_recv() {
                    progress_bar_clone.set_fraction(progress as f64);
                    progress_bar_clone.set_text(Some(&format!("{:.0}%", progress * 100.0)));

                    if progress < 0.1 {
                        status_label_clone.set_text("Loading vector file...");
                    } else if progress < 0.9 {
                        status_label_clone.set_text("Generating G-code...");
                    } else {
                        status_label_clone.set_text("Finalizing...");
                    }
                }

                // Check for result
                if let Ok(result) = result_rx.try_recv() {
                    progress_window_clone.close();

                    match result {
                        Ok(gcode) => {
                            on_gen_clone(gcode);
                        }
                        Err(e) => {
                            CamToolsView::show_error_dialog(
                                "Vector Engraving Generation Failed",
                                &format!("Failed to generate vector engraving: {}", e),
                            );
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        });

        // Save params
        let w_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Save Parameters"),
                None::<&gtk4::Window>,
                FileChooserAction::Save,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Save", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(900, 700);
            dialog.set_current_name("vector_params.json");

            let w_clone = w_save.clone();
            let settings_clone = settings_save.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let params = Self::collect_params_for_save(&w_clone, &settings_clone);
                            if let Ok(json) = serde_json::to_string_pretty(&params) {
                                let _ = fs::write(path, json);
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // Load params
        let w_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Load Parameters"),
                None::<&gtk4::Window>,
                FileChooserAction::Open,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Open", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(900, 700);

            let w_clone = w_load.clone();
            let settings_clone = settings_load.clone();
            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            if let Ok(content) = fs::read_to_string(path) {
                                if let Ok(params) =
                                    serde_json::from_str::<serde_json::Value>(&content)
                                {
                                    Self::apply_params(&w_clone, &params, &settings_clone);
                                }
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // Cancel
        let stack_clone_cancel = stack.clone();
        cancel_btn.connect_clicked(move |_| {
            stack_clone_cancel.set_visible_child_name("dashboard");
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn create_row(title: &str, widget: &impl IsA<gtk4::Widget>) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        row.add_suffix(widget);
        row
    }

    fn collect_params(
        w: &VectorEngravingWidgets,
        settings: &Rc<SettingsController>,
    ) -> VectorEngravingParameters {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        VectorEngravingParameters {
            feed_rate: w.feed_rate.text().parse().unwrap_or(600.0),
            travel_rate: w.travel_rate.text().parse().unwrap_or(3000.0),
            cut_power: w.cut_power.text().parse().unwrap_or(100.0),
            engrave_power: w.engrave_power.text().parse().unwrap_or(50.0),
            power_scale: w.power_scale.text().parse().unwrap_or(1000.0),
            multi_pass: w.multi_pass.is_active(),
            num_passes: w.num_passes.text().parse().unwrap_or(1),
            z_increment: units::parse_length(&w.z_increment.text(), system).unwrap_or(0.5),
            invert_power: w.invert_power.is_active(),
            desired_width: units::parse_length(&w.desired_width.text(), system).unwrap_or(100.0),
            offset_x: units::parse_length(&w.offset_x.text(), system).unwrap_or(10.0),
            offset_y: units::parse_length(&w.offset_y.text(), system).unwrap_or(10.0),
            enable_hatch: w.enable_hatch.is_active(),
            hatch_angle: w.hatch_angle.text().parse().unwrap_or(45.0),
            hatch_spacing: units::parse_length(&w.hatch_spacing.text(), system).unwrap_or(1.0),
            hatch_tolerance: units::parse_length(&w.hatch_tolerance.text(), system).unwrap_or(0.1),
            enable_dwell: w.enable_dwell.is_active(),
            dwell_time: w.dwell_time.text().parse().unwrap_or(0.1),
            cross_hatch: w.cross_hatch.is_active(),
        }
    }

    fn collect_params_for_save(
        w: &VectorEngravingWidgets,
        settings: &Rc<SettingsController>,
    ) -> serde_json::Value {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        serde_json::json!({
            "feed_rate": w.feed_rate.text().to_string(),
            "travel_rate": w.travel_rate.text().to_string(),
            "cut_power": w.cut_power.text().to_string(),
            "engrave_power": w.engrave_power.text().to_string(),
            "power_scale": w.power_scale.text().to_string(),
            "multi_pass": w.multi_pass.is_active(),
            "num_passes": w.num_passes.text().to_string(),
            "z_increment": units::parse_length(&w.z_increment.text(), system).unwrap_or(0.5),
            "invert_power": w.invert_power.is_active(),
            "desired_width": units::parse_length(&w.desired_width.text(), system).unwrap_or(100.0),
            "offset_x": units::parse_length(&w.offset_x.text(), system).unwrap_or(10.0),
            "offset_y": units::parse_length(&w.offset_y.text(), system).unwrap_or(10.0),
            "enable_hatch": w.enable_hatch.is_active(),
            "hatch_angle": w.hatch_angle.text().to_string(),
            "hatch_spacing": units::parse_length(&w.hatch_spacing.text(), system).unwrap_or(1.0),
            "hatch_tolerance": units::parse_length(&w.hatch_tolerance.text(), system).unwrap_or(0.1),
            "enable_dwell": w.enable_dwell.is_active(),
            "dwell_time": w.dwell_time.text().to_string(),
            "cross_hatch": w.cross_hatch.is_active(),
            "vector_path": w.vector_path.text().to_string(),
        })
    }

    fn apply_params(
        w: &VectorEngravingWidgets,
        params: &serde_json::Value,
        settings: &Rc<SettingsController>,
    ) {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        if let Some(v) = params.get("feed_rate").and_then(|v| v.as_str()) {
            w.feed_rate.set_text(v);
        }
        if let Some(v) = params.get("travel_rate").and_then(|v| v.as_str()) {
            w.travel_rate.set_text(v);
        }
        if let Some(v) = params.get("cut_power").and_then(|v| v.as_str()) {
            w.cut_power.set_text(v);
        }
        if let Some(v) = params.get("engrave_power").and_then(|v| v.as_str()) {
            w.engrave_power.set_text(v);
        }
        if let Some(v) = params.get("power_scale").and_then(|v| v.as_str()) {
            w.power_scale.set_text(v);
        }
        if let Some(v) = params.get("multi_pass").and_then(|v| v.as_bool()) {
            w.multi_pass.set_active(v);
        }
        if let Some(v) = params.get("num_passes").and_then(|v| v.as_str()) {
            w.num_passes.set_text(v);
        }
        if let Some(v) = params.get("z_increment").and_then(|v| v.as_f64()) {
            w.z_increment
                .set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("invert_power").and_then(|v| v.as_bool()) {
            w.invert_power.set_active(v);
        }
        if let Some(v) = params.get("desired_width").and_then(|v| v.as_f64()) {
            w.desired_width
                .set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("offset_x").and_then(|v| v.as_f64()) {
            w.offset_x.set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("offset_y").and_then(|v| v.as_f64()) {
            w.offset_y.set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("enable_hatch").and_then(|v| v.as_bool()) {
            w.enable_hatch.set_active(v);
        }
        if let Some(v) = params.get("hatch_angle").and_then(|v| v.as_str()) {
            w.hatch_angle.set_text(v);
        }
        if let Some(v) = params.get("hatch_spacing").and_then(|v| v.as_f64()) {
            w.hatch_spacing
                .set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("hatch_tolerance").and_then(|v| v.as_f64()) {
            w.hatch_tolerance
                .set_text(&units::format_length(v as f32, system));
        }
        if let Some(v) = params.get("enable_dwell").and_then(|v| v.as_bool()) {
            w.enable_dwell.set_active(v);
        }
        if let Some(v) = params.get("dwell_time").and_then(|v| v.as_str()) {
            w.dwell_time.set_text(v);
        }
        if let Some(v) = params.get("cross_hatch").and_then(|v| v.as_bool()) {
            w.cross_hatch.set_active(v);
        }
        if let Some(v) = params.get("vector_path").and_then(|v| v.as_str()) {
            w.vector_path.set_text(v);
            if !v.is_empty() {
                Self::load_vector_preview(w, std::path::Path::new(v));
            }
        }
    }

    fn load_vector_preview(w: &VectorEngravingWidgets, path: &std::path::Path) {
        // Show spinner
        w.preview_spinner.start();
        w.preview_spinner.set_visible(true);

        let path_clone = path.to_path_buf();
        let preview_image = w.preview_image.clone();
        let spinner = w.preview_spinner.clone();
        let info_label = w.info_label.clone();

        // Use channel to communicate with main thread
        let (tx, rx) = std::sync::mpsc::channel();

        // Load in background thread
        std::thread::spawn(move || {
            let result = Self::render_vector_file(&path_clone);
            let _ = tx.send(result);
        });

        // Poll for result on main thread
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            if let Ok(result) = rx.try_recv() {
                spinner.stop();
                spinner.set_visible(false);

                match result {
                    Ok((texture, info)) => {
                        preview_image.set_paintable(Some(&texture));
                        info_label.set_text(&info);
                    }
                    Err(e) => {
                        preview_image.set_paintable(None::<&gtk4::gdk::Texture>);
                        info_label.set_text(&format!("Error: {}", e));
                    }
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    fn render_vector_file(path: &std::path::Path) -> Result<(gtk4::gdk::Texture, String), String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or("Unknown file extension")?;

        match ext.to_lowercase().as_str() {
            "svg" => Self::render_svg(path),
            "dxf" => Self::render_dxf(path),
            _ => Err(format!("Unsupported file format: {}", ext)),
        }
    }

    fn render_svg(path: &std::path::Path) -> Result<(gtk4::gdk::Texture, String), String> {
        let file = gtk4::gio::File::for_path(path);
        let texture = gtk4::gdk::Texture::from_file(&file)
            .map_err(|e| format!("Failed to load SVG: {}", e))?;

        let width = texture.intrinsic_width();
        let height = texture.intrinsic_height();

        let info = format!("SVG: {}x{} pixels", width, height);
        Ok((texture, info))
    }

    fn render_dxf(path: &std::path::Path) -> Result<(gtk4::gdk::Texture, String), String> {
        // Load DXF using the vector engraver
        let params = VectorEngravingParameters::default();
        let engraver = VectorEngraver::from_file(path, params)
            .map_err(|e| format!("Failed to load DXF: {}", e))?;

        // Render paths to a raster image
        let (width, height) = (400, 400);
        let mut img = image::RgbImage::new(width, height);

        // Fill with white background
        for pixel in img.pixels_mut() {
            *pixel = image::Rgb([255, 255, 255]);
        }

        // Calculate bounds
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut path_count = 0;

        for path in &engraver.paths {
            path_count += 1;
            for event in path.iter() {
                use lyon::path::Event;
                match event {
                    Event::Begin { at }
                    | Event::Line { to: at, .. }
                    | Event::End { last: at, .. } => {
                        min_x = min_x.min(at.x);
                        min_y = min_y.min(at.y);
                        max_x = max_x.max(at.x);
                        max_y = max_y.max(at.y);
                    }
                    Event::Quadratic { to, .. } | Event::Cubic { to, .. } => {
                        min_x = min_x.min(to.x);
                        min_y = min_y.min(to.y);
                        max_x = max_x.max(to.x);
                        max_y = max_y.max(to.y);
                    }
                }
            }
        }

        let bounds_width = max_x - min_x;
        let bounds_height = max_y - min_y;

        if bounds_width > 0.0 && bounds_height > 0.0 {
            let scale = (width as f32 / bounds_width).min(height as f32 / bounds_height) * 0.9;
            let offset_x = (width as f32 - bounds_width * scale) / 2.0;
            let offset_y = (height as f32 - bounds_height * scale) / 2.0;

            // Draw paths
            for path in &engraver.paths {
                let mut prev_x = 0;
                let mut prev_y = 0;

                for event in path.iter() {
                    use lyon::path::Event;
                    match event {
                        Event::Begin { at } => {
                            let x = ((at.x - min_x) * scale + offset_x) as i32;
                            let y = ((at.y - min_y) * scale + offset_y) as i32;
                            prev_x = x.clamp(0, width as i32 - 1);
                            prev_y = y.clamp(0, height as i32 - 1);
                        }
                        Event::Line { to, .. } => {
                            let x = ((to.x - min_x) * scale + offset_x) as i32;
                            let y = ((to.y - min_y) * scale + offset_y) as i32;
                            let x = x.clamp(0, width as i32 - 1);
                            let y = y.clamp(0, height as i32 - 1);

                            // Draw line using Bresenham
                            Self::draw_line(&mut img, prev_x, prev_y, x, y);
                            prev_x = x;
                            prev_y = y;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Convert to texture
        let buffer = glib::Bytes::from(&img.into_raw());
        let texture = gtk4::gdk::MemoryTexture::new(
            width as i32,
            height as i32,
            gtk4::gdk::MemoryFormat::R8g8b8,
            &buffer,
            width as usize * 3,
        );

        let info = format!(
            "DXF: {:.1}x{:.1} mm, {} paths",
            bounds_width, bounds_height, path_count
        );
        Ok((texture.upcast(), info))
    }

    fn draw_line(img: &mut image::RgbImage, x0: i32, y0: i32, x1: i32, y1: i32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && x < img.width() as i32 && y >= 0 && y < img.height() as i32 {
                img.put_pixel(x as u32, y as u32, image::Rgb([0, 0, 0]));
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
}

struct TabbedBoxWidgets {
    width: Entry,
    depth: Entry,
    height: Entry,
    outside: CheckButton,
    thickness: Entry,
    burn: Entry,
    finger_width: Entry,
    space_width: Entry,
    surrounding_spaces: Entry,
    play: Entry,
    extra_length: Entry,
    // New controls
    box_type: ComboBoxText,
    dividers_x: Entry,
    dividers_y: Entry,
    divider_keying: ComboBoxText,
    optimize_layout: CheckButton,
    passes: Entry,
    power: Entry,
    feed_rate: Entry,
    offset_x: Entry,
    offset_y: Entry,
    home_before: CheckButton,
}

pub struct TabbedBoxMaker {
    pub content: Box,
}

impl TabbedBoxMaker {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        on_generate: Rc<F>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header with Back Button
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();

        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });

        header.append(&back_btn);
        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);
        header.append(&help_browser::make_help_button("tabbed_box_maker"));
        content_box.append(&header);

        // Split View
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar (40% width)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_lbl = Label::builder()
            .label("Tabbed Box Maker")
            .css_classes(vec!["title-3"])
            .wrap(true)
            .halign(Align::Start)
            .build();

        let desc_lbl = Label::builder()
            .label("Generate G-code for laser/CNC cut boxes with finger joints based on the boxes.py algorithm.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();

        sidebar.append(&title_lbl);
        sidebar.append(&desc_lbl);

        // Right Panel
        let right_panel = Box::new(Orientation::Vertical, 0);

        // Scrollable Content
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Widgets
        let (width_row, width, width_unit) = create_dimension_row("X (Width):", 100.0, &settings);
        let (depth_row, depth, depth_unit) = create_dimension_row("Y (Depth):", 100.0, &settings);
        let (height_row, height, height_unit) =
            create_dimension_row("H (Height):", 100.0, &settings);
        let outside = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let (thickness_row, thickness, thickness_unit) =
            create_dimension_row("Thickness:", 3.0, &settings);
        let (burn_row, burn, burn_unit) = create_dimension_row("Burn / Tool Dia:", 0.1, &settings);
        let finger_width = Entry::builder().text("2").valign(Align::Center).build();
        let space_width = Entry::builder().text("2").valign(Align::Center).build();
        let surrounding_spaces = Entry::builder().text("2").valign(Align::Center).build();
        let (play_row, play, play_unit) =
            create_dimension_row("Play (fit tolerance):", 0.0, &settings);
        let (extra_length_row, extra_length, extra_length_unit) =
            create_dimension_row("Extra Length:", 0.0, &settings);

        // New Widgets
        let box_type = ComboBoxText::new();
        box_type.append(Some("0"), "Full Box");
        box_type.append(Some("1"), "No Top");
        box_type.append(Some("2"), "No Bottom");
        box_type.append(Some("3"), "No Sides");
        box_type.append(Some("4"), "No Front/Back");
        box_type.append(Some("5"), "No Left/Right");
        box_type.set_active_id(Some("0"));
        box_type.set_valign(Align::Center);

        let dividers_x = Entry::builder().text("0").valign(Align::Center).build();
        let dividers_y = Entry::builder().text("0").valign(Align::Center).build();

        let divider_keying = ComboBoxText::new();
        divider_keying.append(Some("0"), "Walls & Floor");
        divider_keying.append(Some("1"), "Walls Only");
        divider_keying.append(Some("2"), "Floor Only");
        divider_keying.append(Some("3"), "None");
        divider_keying.set_active_id(Some("0"));
        divider_keying.set_valign(Align::Center);

        let optimize_layout = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        let passes = Entry::builder().text("3").valign(Align::Center).build();
        let power = Entry::builder().text("1000").valign(Align::Center).build();
        let feed_rate = Entry::builder().text("500").valign(Align::Center).build();

        let (offset_x_row, offset_x, offset_x_unit) =
            create_dimension_row("Offset X:", 10.0, &settings);
        let (offset_y_row, offset_y, offset_y_unit) =
            create_dimension_row("Offset Y:", 10.0, &settings);
        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Box Dimensions
        let dim_group = PreferencesGroup::builder().title("Box Dimensions").build();
        dim_group.add(&width_row);
        dim_group.add(&depth_row);
        dim_group.add(&height_row);

        let outside_row = ActionRow::builder().title("Outside Dims:").build();
        outside_row.add_suffix(&outside);
        dim_group.add(&outside_row);

        scroll_content.append(&dim_group);

        // Box Configuration
        let config_group = PreferencesGroup::builder()
            .title("Box Configuration")
            .build();
        config_group.add(&Self::create_row("Box Type:", &box_type));
        config_group.add(&Self::create_row("Dividers X:", &dividers_x));
        config_group.add(&Self::create_row("Dividers Y:", &dividers_y));
        config_group.add(&Self::create_row("Divider Keying:", &divider_keying));

        let optimize_row = ActionRow::builder().title("Optimize Layout:").build();
        optimize_row.add_suffix(&optimize_layout);
        config_group.add(&optimize_row);

        scroll_content.append(&config_group);

        // Material Settings
        let mat_group = PreferencesGroup::builder()
            .title("Material Settings")
            .build();
        mat_group.add(&thickness_row);
        mat_group.add(&burn_row);
        scroll_content.append(&mat_group);

        // Finger Joint Settings
        let finger_group = PreferencesGroup::builder()
            .title("Finger Joint Settings (multiples of thickness)")
            .build();
        finger_group.add(&Self::create_row("Finger Width:", &finger_width));
        finger_group.add(&Self::create_row("Space Width:", &space_width));
        finger_group.add(&Self::create_row(
            "Surrounding Spaces:",
            &surrounding_spaces,
        ));
        finger_group.add(&play_row);
        finger_group.add(&extra_length_row);
        scroll_content.append(&finger_group);

        // Laser Settings
        let laser_group = PreferencesGroup::builder().title("Laser Settings").build();
        laser_group.add(&Self::create_row("Passes:", &passes));
        laser_group.add(&Self::create_row("Power (S):", &power));
        laser_group.add(&Self::create_row("Feed Rate:", &feed_rate));
        scroll_content.append(&laser_group);

        // Work Origin Offsets
        let offset_group = PreferencesGroup::builder()
            .title("Work Origin Offsets")
            .build();
        offset_group.add(&offset_x_row);
        offset_group.add(&offset_y_row);

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        offset_group.add(&home_row);

        scroll_content.append(&offset_group);

        right_panel.append(&scrolled);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label("Load");
        let save_btn = Button::with_label("Save");
        let cancel_btn = Button::with_label("Cancel");
        let generate_btn = Button::with_label("Generate");
        generate_btn.add_css_class("suggested-action");

        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));

        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        content_box.append(&paned);

        let widgets = Rc::new(TabbedBoxWidgets {
            width,
            depth,
            height,
            outside,
            thickness,
            burn,
            finger_width,
            space_width,
            surrounding_spaces,
            play,
            extra_length,
            box_type,
            dividers_x,
            dividers_y,
            divider_keying,
            optimize_layout,
            passes,
            power,
            feed_rate,
            offset_x,
            offset_y,
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let width_unit = width_unit.clone();
            let depth_unit = depth_unit.clone();
            let height_unit = height_unit.clone();
            let thickness_unit = thickness_unit.clone();
            let burn_unit = burn_unit.clone();
            let play_unit = play_unit.clone();
            let extra_length_unit = extra_length_unit.clone();
            let offset_x_unit = offset_x_unit.clone();
            let offset_y_unit = offset_y_unit.clone();

            let last_system = Rc::new(Cell::new(
                settings.persistence.borrow().config().ui.measurement_system,
            ));

            settings.on_setting_changed(move |key, _| {
                if key == "measurement_system" {
                    let new_system = settings_clone
                        .persistence
                        .borrow()
                        .config()
                        .ui
                        .measurement_system;
                    let old_system = last_system.get();

                    if new_system != old_system {
                        let unit_label = units::get_unit_label(new_system);

                        let update_entry = |entry: &Entry, label: &Label| {
                            if let Ok(val_mm) = units::parse_length(&entry.text(), old_system) {
                                entry.set_text(&units::format_length(val_mm, new_system));
                            }
                            label.set_text(unit_label);
                        };

                        update_entry(&w.width, &width_unit);
                        update_entry(&w.depth, &depth_unit);
                        update_entry(&w.height, &height_unit);
                        update_entry(&w.thickness, &thickness_unit);
                        update_entry(&w.burn, &burn_unit);
                        update_entry(&w.play, &play_unit);
                        update_entry(&w.extra_length, &extra_length_unit);
                        update_entry(&w.offset_x, &offset_x_unit);
                        update_entry(&w.offset_y, &offset_y_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Connect Signals
        let widgets_gen = widgets.clone();
        let on_generate = on_generate.clone();
        let settings_gen = settings.clone();
        generate_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&widgets_gen, &settings_gen);
            let home_before = widgets_gen.home_before.is_active();

            // Create progress dialog
            let progress_window = gtk4::Window::builder()
                .title("Generating Box")
                .modal(true)
                .default_width(400)
                .default_height(150)
                .resizable(false)
                .build();

            let vbox = Box::new(Orientation::Vertical, 12);
            vbox.set_margin_top(24);
            vbox.set_margin_bottom(24);
            vbox.set_margin_start(24);
            vbox.set_margin_end(24);

            let status_label = Label::new(Some("Generating box panels..."));
            vbox.append(&status_label);

            let progress_bar = gtk4::ProgressBar::new();
            progress_bar.set_show_text(true);
            progress_bar.set_fraction(0.0);
            vbox.append(&progress_bar);

            let button_box = Box::new(Orientation::Horizontal, 6);
            button_box.set_halign(Align::End);
            let cancel_button = Button::with_label("Cancel");
            button_box.append(&cancel_button);
            vbox.append(&button_box);

            progress_window.set_child(Some(&vbox));
            progress_window.show();

            let on_generate_clone = on_generate.clone();
            let progress_window_clone = progress_window.clone();
            let progress_bar_clone = progress_bar.clone();

            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();

            let cancel_tx_clone = cancel_tx.clone();
            cancel_button.connect_clicked(move |_| {
                let _ = cancel_tx_clone.send(());
            });

            // Spawn background thread
            std::thread::spawn(move || {
                let result = (|| -> Result<String, String> {
                    if cancel_rx.try_recv().is_ok() {
                        return Err("Cancelled by user".to_string());
                    }
                    let mut generator = Generator::new(params)?;
                    generator.generate()?;
                    let mut gcode = generator.to_gcode();

                    gcode = gcode.replace("$H\n", "").replace("$H", "");
                    if home_before {
                        gcode = format!("$H\n{}", gcode);
                    }
                    Ok(gcode)
                })();

                let _ = result_tx.send(result);
            });

            // Simulate progress
            let mut progress = 0.0;
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                // Check for result
                if let Ok(result) = result_rx.try_recv() {
                    progress_window_clone.close();

                    match result {
                        Ok(gcode) => {
                            on_generate_clone(gcode);
                        }
                        Err(e) => {
                            CamToolsView::show_error_dialog(
                                "Box Generation Failed",
                                &format!("Failed to generate box: {}", e),
                            );
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    // Simulate progress
                    progress += 0.05;
                    if progress > 0.95 {
                        progress = 0.95;
                    }
                    progress_bar_clone.set_fraction(progress);
                    progress_bar_clone.set_text(Some(&format!("{:.0}%", progress * 100.0)));
                    glib::ControlFlow::Continue
                }
            });
        });

        let widgets_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&widgets_save, &settings_save);
            Self::save_params(&params);
        });

        let widgets_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            Self::load_params(&widgets_load, &settings_load);
        });

        let stack_clone_cancel = stack.clone();
        cancel_btn.connect_clicked(move |_| {
            stack_clone_cancel.set_visible_child_name("dashboard");
        });

        Self {
            content: content_box,
        }
    }
    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn create_row(title: &str, widget: &impl IsA<gtk4::Widget>) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        row.add_suffix(widget);
        row
    }

    fn collect_params(w: &TabbedBoxWidgets, settings: &Rc<SettingsController>) -> BoxParameters {
        let mut params = BoxParameters::default();
        let system = settings.persistence.borrow().config().ui.measurement_system;

        params.x = units::parse_length(&w.width.text(), system).unwrap_or(100.0);
        params.y = units::parse_length(&w.depth.text(), system).unwrap_or(100.0);
        params.h = units::parse_length(&w.height.text(), system).unwrap_or(100.0);
        params.outside = w.outside.is_active();
        params.thickness = units::parse_length(&w.thickness.text(), system).unwrap_or(3.0);
        params.burn = units::parse_length(&w.burn.text(), system).unwrap_or(0.1);

        params.finger_joint.finger = w.finger_width.text().parse().unwrap_or(2.0);
        params.finger_joint.space = w.space_width.text().parse().unwrap_or(2.0);
        params.finger_joint.surrounding_spaces = w.surrounding_spaces.text().parse().unwrap_or(2.0);
        params.finger_joint.play = units::parse_length(&w.play.text(), system).unwrap_or(0.0);
        params.finger_joint.extra_length =
            units::parse_length(&w.extra_length.text(), system).unwrap_or(0.0);

        // New params
        if let Some(id) = w.box_type.active_id() {
            params.box_type = BoxType::from(id.parse::<i32>().unwrap_or(0));
        }
        params.dividers_x = w.dividers_x.text().parse().unwrap_or(0);
        params.dividers_y = w.dividers_y.text().parse().unwrap_or(0);
        if let Some(id) = w.divider_keying.active_id() {
            params.key_divider_type = KeyDividerType::from(id.parse::<i32>().unwrap_or(0));
        }
        params.optimize_layout = w.optimize_layout.is_active();

        params.laser_passes = w.passes.text().parse().unwrap_or(3);
        params.laser_power = w.power.text().parse().unwrap_or(1000);
        params.feed_rate = w.feed_rate.text().parse().unwrap_or(500.0);

        params.offset_x = units::parse_length(&w.offset_x.text(), system).unwrap_or(10.0);
        params.offset_y = units::parse_length(&w.offset_y.text(), system).unwrap_or(10.0);

        params
    }

    fn save_params(params: &BoxParameters) {
        let dialog = FileChooserDialog::new(
            Some("Save Box Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        dialog.set_current_name("box_params.json");

        let params_clone = params.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(json) = serde_json::to_string_pretty(&params_clone) {
                            let _ = fs::write(path, json);
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn load_params(w: &Rc<TabbedBoxWidgets>, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Load Box Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Open,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        let w_clone = w.clone();
        let settings_clone = settings.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(content) = fs::read_to_string(path) {
                            if let Ok(params) = serde_json::from_str::<BoxParameters>(&content) {
                                Self::apply_params(&w_clone, &params, &settings_clone);
                            }
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn apply_params(w: &TabbedBoxWidgets, p: &BoxParameters, settings: &Rc<SettingsController>) {
        let system = settings.persistence.borrow().config().ui.measurement_system;
        w.width.set_text(&units::format_length(p.x as f32, system));
        w.depth.set_text(&units::format_length(p.y as f32, system));
        w.height.set_text(&units::format_length(p.h as f32, system));
        w.outside.set_active(p.outside);
        w.thickness
            .set_text(&units::format_length(p.thickness as f32, system));
        w.burn
            .set_text(&units::format_length(p.burn as f32, system));

        w.finger_width.set_text(&p.finger_joint.finger.to_string());
        w.space_width.set_text(&p.finger_joint.space.to_string());
        w.surrounding_spaces
            .set_text(&p.finger_joint.surrounding_spaces.to_string());
        w.play
            .set_text(&units::format_length(p.finger_joint.play as f32, system));
        w.extra_length.set_text(&units::format_length(
            p.finger_joint.extra_length as f32,
            system,
        ));

        // New params
        w.box_type
            .set_active_id(Some(&(p.box_type as i32).to_string()));
        w.dividers_x.set_text(&p.dividers_x.to_string());
        w.dividers_y.set_text(&p.dividers_y.to_string());
        w.divider_keying
            .set_active_id(Some(&(p.key_divider_type as i32).to_string()));
        w.optimize_layout.set_active(p.optimize_layout);

        w.passes.set_text(&p.laser_passes.to_string());
        w.power.set_text(&p.laser_power.to_string());
        w.feed_rate.set_text(&p.feed_rate.to_string());

        w.offset_x
            .set_text(&units::format_length(p.offset_x as f32, system));
        w.offset_y
            .set_text(&units::format_length(p.offset_y as f32, system));
    }
}

// Speeds & Feeds Calculator
pub struct SpeedsFeedsTool {
    content: Box,
}

impl SpeedsFeedsTool {
    pub fn new(stack: &Stack, _settings: Rc<SettingsController>) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });
        header.append(&back_btn);

        let title = Label::builder()
            .label("Speeds & Feeds Calculator")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);
        header.append(&help_browser::make_help_button("speeds_feeds_calculator"));
        content_box.append(&header);

        // Paned Layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar (40%)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_label = Label::builder()
            .label("Speeds & Feeds")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Calculate optimal cutting speeds and feed rates based on material properties and tool specifications. Uses standard machining formulas.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Results display area
        let results_box = Box::new(Orientation::Vertical, 6);
        results_box.set_vexpand(true);

        let results_frame = gtk4::Frame::new(Some("Calculated Results"));
        results_frame.set_margin_top(12);

        let results_content = Box::new(Orientation::Vertical, 6);
        results_content.set_margin_top(12);
        results_content.set_margin_bottom(12);
        results_content.set_margin_start(12);
        results_content.set_margin_end(12);

        let rpm_label = Label::builder()
            .label("RPM: --")
            .halign(Align::Start)
            .build();
        let feed_label = Label::builder()
            .label("Feed Rate: --")
            .halign(Align::Start)
            .build();
        let source_label = Label::builder()
            .label("")
            .css_classes(vec!["caption", "dim-label"])
            .halign(Align::Start)
            .wrap(true)
            .build();
        let warnings_label = Label::builder()
            .label("")
            .css_classes(vec!["caption", "warning"])
            .halign(Align::Start)
            .wrap(true)
            .build();

        results_content.append(&rpm_label);
        results_content.append(&feed_label);
        results_content.append(&source_label);
        results_content.append(&warnings_label);
        results_frame.set_child(Some(&results_content));
        results_box.append(&results_frame);
        sidebar.append(&results_box);

        // Content Area
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Material Selection
        let material_group = PreferencesGroup::builder().title("Material").build();
        let material_combo = ComboBoxText::new();
        material_combo.append(Some("aluminum"), "Aluminum");
        material_combo.append(Some("wood"), "Wood (Softwood)");
        material_combo.append(Some("acrylic"), "Acrylic");
        material_combo.append(Some("steel"), "Steel (Mild)");
        material_combo.set_active_id(Some("aluminum"));
        let material_row = ActionRow::builder().title("Material Type:").build();
        material_row.add_suffix(&material_combo);
        material_group.add(&material_row);
        scroll_content.append(&material_group);

        // Tool Selection
        let tool_group = PreferencesGroup::builder().title("Tool").build();
        let tool_combo = ComboBoxText::new();
        tool_combo.append(Some("endmill_6mm"), "End Mill - 6mm");
        tool_combo.append(Some("endmill_3mm"), "End Mill - 3mm");
        tool_combo.append(Some("vbit_30deg"), "V-Bit - 30°");
        tool_combo.set_active_id(Some("endmill_6mm"));
        let tool_row = ActionRow::builder().title("Tool Type:").build();
        tool_row.add_suffix(&tool_combo);
        tool_group.add(&tool_row);
        scroll_content.append(&tool_group);

        right_panel.append(&scrolled);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let calculate_btn = Button::with_label("Calculate");
        calculate_btn.add_css_class("suggested-action");
        action_box.append(&calculate_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        content_box.append(&paned);

        // Calculate button handler - simplified placeholder
        let rpm_label_calc = rpm_label.clone();
        let feed_label_calc = feed_label.clone();
        let source_label_calc = source_label.clone();
        let warnings_label_calc = warnings_label.clone();

        calculate_btn.connect_clicked(move |_| {
            // Placeholder calculation
            rpm_label_calc.set_text("RPM: 12,000");
            feed_label_calc.set_text("Feed Rate: 1,500 mm/min");
            source_label_calc.set_text("Source: Material defaults + Tool specifications");
            warnings_label_calc.set_text("");
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }
}

// Spoilboard Surfacing Tool
struct SpoilboardSurfacingWidgets {
    width: Entry,
    height: Entry,
    tool_diameter: Entry,
    feed_rate: Entry,
    spindle_speed: Entry,
    cut_depth: Entry,
    stepover_percent: Entry,
    safe_z: Entry,
    home_before: CheckButton,
}

pub struct SpoilboardSurfacingTool {
    content: Box,
}

impl SpoilboardSurfacingTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        on_generate: Rc<F>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });
        header.append(&back_btn);

        let title = Label::builder()
            .label("Spoilboard Surfacing")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);
        header.append(&help_browser::make_help_button("spoilboard_surfacing"));
        content_box.append(&header);

        // Paned Layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar (40%)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_label = Label::builder()
            .label("Spoilboard Surfacing")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Generate G-code for surfacing your CNC spoilboard to ensure a flat, level work surface.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Content Area
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Create widgets
        let (width_row, width, width_unit) = create_dimension_row("Width:", 400.0, &settings);
        let (height_row, height, height_unit) = create_dimension_row("Height:", 300.0, &settings);
        let (tool_diameter_row, tool_diameter, tool_diameter_unit) =
            create_dimension_row("Tool Diameter:", 25.0, &settings);
        let feed_rate = Entry::builder().text("1000").valign(Align::Center).build();
        let spindle_speed = Entry::builder().text("18000").valign(Align::Center).build();
        let (cut_depth_row, cut_depth, cut_depth_unit) =
            create_dimension_row("Cut Depth:", 0.5, &settings);
        let stepover_percent = Entry::builder().text("40").valign(Align::Center).build();
        let (safe_z_row, safe_z, safe_z_unit) =
            create_dimension_row("Safe Z Height:", 5.0, &settings);
        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let dim_group = PreferencesGroup::builder()
            .title("Spoilboard Dimensions")
            .build();
        dim_group.add(&width_row);
        dim_group.add(&height_row);
        scroll_content.append(&dim_group);

        let tool_group = PreferencesGroup::builder().title("Tool Settings").build();
        tool_group.add(&tool_diameter_row);
        tool_group.add(&cut_depth_row);
        tool_group.add(&Self::create_row("Stepover (%):", &stepover_percent));
        scroll_content.append(&tool_group);

        let machine_group = PreferencesGroup::builder()
            .title("Machine Settings")
            .build();
        machine_group.add(&Self::create_row("Feed Rate (mm/min):", &feed_rate));
        machine_group.add(&Self::create_row("Spindle Speed (RPM):", &spindle_speed));
        machine_group.add(&safe_z_row);

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        machine_group.add(&home_row);

        scroll_content.append(&machine_group);

        right_panel.append(&scrolled);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label("Load");
        let save_btn = Button::with_label("Save");
        let cancel_btn = Button::with_label("Cancel");
        let generate_btn = Button::with_label("Generate");
        generate_btn.add_css_class("suggested-action");
        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        content_box.append(&paned);

        let widgets = Rc::new(SpoilboardSurfacingWidgets {
            width,
            height,
            tool_diameter,
            feed_rate,
            spindle_speed,
            cut_depth,
            stepover_percent,
            safe_z,
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let width_unit = width_unit.clone();
            let height_unit = height_unit.clone();
            let tool_diameter_unit = tool_diameter_unit.clone();
            let cut_depth_unit = cut_depth_unit.clone();
            let safe_z_unit = safe_z_unit.clone();

            let last_system = Rc::new(Cell::new(
                settings.persistence.borrow().config().ui.measurement_system,
            ));

            settings.on_setting_changed(move |key, _| {
                if key == "measurement_system" {
                    let new_system = settings_clone
                        .persistence
                        .borrow()
                        .config()
                        .ui
                        .measurement_system;
                    let old_system = last_system.get();

                    if new_system != old_system {
                        let unit_label = units::get_unit_label(new_system);

                        let update_entry = |entry: &Entry, label: &Label| {
                            if let Ok(val_mm) = units::parse_length(&entry.text(), old_system) {
                                entry.set_text(&units::format_length(val_mm, new_system));
                            }
                            label.set_text(unit_label);
                        };

                        update_entry(&w.width, &width_unit);
                        update_entry(&w.height, &height_unit);
                        update_entry(&w.tool_diameter, &tool_diameter_unit);
                        update_entry(&w.cut_depth, &cut_depth_unit);
                        update_entry(&w.safe_z, &safe_z_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Generate button
        let w_gen = widgets.clone();
        let settings_gen = settings.clone();
        generate_btn.connect_clicked(move |_| {
            let home_before = w_gen.home_before.is_active();
            let system = settings_gen
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system;

            let params = SpoilboardSurfacingParameters {
                width: units::parse_length(&w_gen.width.text(), system).unwrap_or(400.0) as f64,
                height: units::parse_length(&w_gen.height.text(), system).unwrap_or(300.0) as f64,
                tool_diameter: units::parse_length(&w_gen.tool_diameter.text(), system)
                    .unwrap_or(25.0) as f64,
                feed_rate: w_gen.feed_rate.text().parse().unwrap_or(1000.0),
                spindle_speed: w_gen.spindle_speed.text().parse().unwrap_or(18000.0),
                cut_depth: units::parse_length(&w_gen.cut_depth.text(), system).unwrap_or(0.5)
                    as f64,
                stepover_percent: w_gen.stepover_percent.text().parse().unwrap_or(40.0),
                safe_z: units::parse_length(&w_gen.safe_z.text(), system).unwrap_or(5.0) as f64,
            };

            let generator = SpoilboardSurfacingGenerator::new(params);
            match generator.generate() {
                Ok(mut gcode) => {
                    gcode = gcode.replace("$H\n", "").replace("$H", "");
                    if home_before {
                        gcode = format!("$H\n{}", gcode);
                    }
                    on_generate(gcode);
                }
                Err(e) => {
                    CamToolsView::show_error_dialog(
                        "Generation Failed",
                        &format!("Failed to generate surfacing toolpath: {}", e),
                    );
                }
            }
        });

        // Save/Load/Cancel
        let w_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            Self::save_params(&w_save, &settings_save);
        });

        let w_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            Self::load_params(&w_load, &settings_load);
        });

        let stack_clone_cancel = stack.clone();
        cancel_btn.connect_clicked(move |_| {
            stack_clone_cancel.set_visible_child_name("dashboard");
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn create_row(title: &str, widget: &impl IsA<gtk4::Widget>) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        row.add_suffix(widget);
        row
    }

    fn save_params(w: &SpoilboardSurfacingWidgets, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Save Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);
        dialog.set_current_name("surfacing_params.json");

        let system = settings.persistence.borrow().config().ui.measurement_system;
        let w_clone = Rc::new((
            units::parse_length(&w.width.text(), system).unwrap_or(400.0),
            units::parse_length(&w.height.text(), system).unwrap_or(300.0),
            units::parse_length(&w.tool_diameter.text(), system).unwrap_or(25.0),
            w.feed_rate.text().to_string(),
            w.spindle_speed.text().to_string(),
            units::parse_length(&w.cut_depth.text(), system).unwrap_or(1.0),
            w.stepover_percent.text().to_string(),
            units::parse_length(&w.safe_z.text(), system).unwrap_or(5.0),
        ));

        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        let json = serde_json::json!({
                            "width": w_clone.0,
                            "height": w_clone.1,
                            "tool_diameter": w_clone.2,
                            "feed_rate": w_clone.3,
                            "spindle_speed": w_clone.4,
                            "cut_depth": w_clone.5,
                            "stepover_percent": w_clone.6,
                            "safe_z": w_clone.7,
                        });
                        let _ = fs::write(path, serde_json::to_string_pretty(&json).unwrap());
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn load_params(w: &SpoilboardSurfacingWidgets, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Load Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Open,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        let w_clone = Rc::new((
            w.width.clone(),
            w.height.clone(),
            w.tool_diameter.clone(),
            w.feed_rate.clone(),
            w.spindle_speed.clone(),
            w.cut_depth.clone(),
            w.stepover_percent.clone(),
            w.safe_z.clone(),
        ));
        let settings_clone = settings.clone();

        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(content) = fs::read_to_string(path) {
                            if let Ok(params) = serde_json::from_str::<serde_json::Value>(&content)
                            {
                                let system = settings_clone
                                    .persistence
                                    .borrow()
                                    .config()
                                    .ui
                                    .measurement_system;
                                if let Some(v) = params.get("width").and_then(|v| v.as_f64()) {
                                    w_clone.0.set_text(&units::format_length(v as f32, system));
                                }
                                if let Some(v) = params.get("height").and_then(|v| v.as_f64()) {
                                    w_clone.1.set_text(&units::format_length(v as f32, system));
                                }
                                if let Some(v) =
                                    params.get("tool_diameter").and_then(|v| v.as_f64())
                                {
                                    w_clone.2.set_text(&units::format_length(v as f32, system));
                                }
                                if let Some(v) = params.get("feed_rate").and_then(|v| v.as_str()) {
                                    w_clone.3.set_text(v);
                                }
                                if let Some(v) =
                                    params.get("spindle_speed").and_then(|v| v.as_str())
                                {
                                    w_clone.4.set_text(v);
                                }
                                if let Some(v) = params.get("cut_depth").and_then(|v| v.as_f64()) {
                                    w_clone.5.set_text(&units::format_length(v as f32, system));
                                }
                                if let Some(v) =
                                    params.get("stepover_percent").and_then(|v| v.as_str())
                                {
                                    w_clone.6.set_text(v);
                                }
                                if let Some(v) = params.get("safe_z").and_then(|v| v.as_f64()) {
                                    w_clone.7.set_text(&units::format_length(v as f32, system));
                                }
                            }
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }
}

// Spoilboard Grid Tool
struct SpoilboardGridWidgets {
    width: Entry,
    height: Entry,
    grid_spacing: Entry,
    feed_rate: Entry,
    laser_power: Entry,
    laser_mode: ComboBoxText,
    home_before: CheckButton,
}

pub struct SpoilboardGridTool {
    content: Box,
}

impl SpoilboardGridTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        on_generate: Rc<F>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);

        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });
        header.append(&back_btn);

        let title = Label::builder()
            .label("Spoilboard Grid")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);
        header.append(&help_browser::make_help_button("spoilboard_grid"));
        content_box.append(&header);

        // Paned Layout
        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar (40%)
        let sidebar = Box::new(Orientation::Vertical, 12);
        sidebar.add_css_class("sidebar");
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(24);
        sidebar.set_margin_start(24);
        sidebar.set_margin_end(24);

        let title_label = Label::builder()
            .label("Spoilboard Grid")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        sidebar.append(&title_label);

        let desc = Label::builder()
            .label("Create a grid pattern on your spoilboard for easy workpiece alignment and fixturing.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        sidebar.append(&desc);

        // Content Area
        let right_panel = Box::new(Orientation::Vertical, 0);
        let scroll_content = Box::new(Orientation::Vertical, 0);
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&scroll_content)
            .build();

        // Create widgets
        let (width_row, width, width_unit) = create_dimension_row("Width:", 400.0, &settings);
        let (height_row, height, height_unit) = create_dimension_row("Height:", 300.0, &settings);
        let (grid_spacing_row, grid_spacing, grid_spacing_unit) =
            create_dimension_row("Grid Spacing:", 10.0, &settings);
        let feed_rate = Entry::builder().text("1000").valign(Align::Center).build();
        let laser_power = Entry::builder().text("1000").valign(Align::Center).build();

        let laser_mode = ComboBoxText::new();
        laser_mode.append(Some("M3"), "M3 (Constant Power)");
        laser_mode.append(Some("M4"), "M4 (Dynamic Power)");
        laser_mode.set_active_id(Some("M4"));
        laser_mode.set_valign(Align::Center);

        let home_before = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();

        // Groups
        let dim_group = PreferencesGroup::builder()
            .title("Spoilboard Dimensions")
            .build();
        dim_group.add(&width_row);
        dim_group.add(&height_row);
        dim_group.add(&grid_spacing_row);
        scroll_content.append(&dim_group);

        let laser_group = PreferencesGroup::builder().title("Laser Settings").build();
        laser_group.add(&Self::create_row("Laser Power (S):", &laser_power));
        laser_group.add(&Self::create_row("Feed Rate (mm/min):", &feed_rate));
        laser_group.add(&Self::create_row("Laser Mode:", &laser_mode));

        let home_row = ActionRow::builder()
            .title("Home Device Before Start")
            .build();
        home_row.add_suffix(&home_before);
        laser_group.add(&home_row);

        scroll_content.append(&laser_group);

        right_panel.append(&scrolled);

        // Action Buttons
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label("Load");
        let save_btn = Button::with_label("Save");
        let cancel_btn = Button::with_label("Cancel");
        let generate_btn = Button::with_label("Generate");
        generate_btn.add_css_class("suggested-action");
        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&right_panel));
        // Initial ratio only; do not fight user resizing.
        set_paned_initial_fraction(&paned, 0.40);

        content_box.append(&paned);

        let widgets = Rc::new(SpoilboardGridWidgets {
            width,
            height,
            grid_spacing,
            feed_rate,
            laser_power,
            laser_mode,
            home_before,
        });

        // Unit update listener
        {
            let settings_clone = settings.clone();
            let w = widgets.clone();
            let width_unit = width_unit.clone();
            let height_unit = height_unit.clone();
            let grid_spacing_unit = grid_spacing_unit.clone();

            let last_system = Rc::new(Cell::new(
                settings.persistence.borrow().config().ui.measurement_system,
            ));

            settings.on_setting_changed(move |key, _| {
                if key == "measurement_system" {
                    let new_system = settings_clone
                        .persistence
                        .borrow()
                        .config()
                        .ui
                        .measurement_system;
                    let old_system = last_system.get();

                    if new_system != old_system {
                        let unit_label = units::get_unit_label(new_system);

                        let update_entry = |entry: &Entry, label: &Label| {
                            if let Ok(val_mm) = units::parse_length(&entry.text(), old_system) {
                                entry.set_text(&units::format_length(val_mm, new_system));
                            }
                            label.set_text(unit_label);
                        };

                        update_entry(&w.width, &width_unit);
                        update_entry(&w.height, &height_unit);
                        update_entry(&w.grid_spacing, &grid_spacing_unit);

                        last_system.set(new_system);
                    }
                }
            });
        }

        // Generate button
        let w_gen = widgets.clone();
        let settings_gen = settings.clone();
        generate_btn.connect_clicked(move |_| {
            let home_before = w_gen.home_before.is_active();
            let laser_mode_str = w_gen
                .laser_mode
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "M4".to_string());

            let system = settings_gen
                .persistence
                .borrow()
                .config()
                .ui
                .measurement_system;

            let params = SpoilboardGridParameters {
                width: units::parse_length(&w_gen.width.text(), system).unwrap_or(400.0) as f64,
                height: units::parse_length(&w_gen.height.text(), system).unwrap_or(300.0) as f64,
                grid_spacing: units::parse_length(&w_gen.grid_spacing.text(), system)
                    .unwrap_or(10.0) as f64,
                feed_rate: w_gen.feed_rate.text().parse().unwrap_or(1000.0),
                laser_power: w_gen.laser_power.text().parse().unwrap_or(1000.0),
                laser_mode: laser_mode_str,
            };

            let generator = SpoilboardGridGenerator::new(params);
            match generator.generate() {
                Ok(mut gcode) => {
                    gcode = gcode.replace("$H\n", "").replace("$H", "");
                    if home_before {
                        gcode = format!("$H\n{}", gcode);
                    }
                    on_generate(gcode);
                }
                Err(e) => {
                    CamToolsView::show_error_dialog(
                        "Generation Failed",
                        &format!("Failed to generate grid pattern: {}", e),
                    );
                }
            }
        });

        // Save/Load/Cancel
        let w_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            Self::save_params(&w_save, &settings_save);
        });

        let w_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            Self::load_params(&w_load, &settings_load);
        });

        let stack_clone_cancel = stack.clone();
        cancel_btn.connect_clicked(move |_| {
            stack_clone_cancel.set_visible_child_name("dashboard");
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn create_row(title: &str, widget: &impl IsA<gtk4::Widget>) -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        row.add_suffix(widget);
        row
    }

    fn save_params(w: &SpoilboardGridWidgets, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Save Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);
        dialog.set_current_name("grid_params.json");

        let system = settings.persistence.borrow().config().ui.measurement_system;
        let w_clone = Rc::new((
            units::parse_length(&w.width.text(), system).unwrap_or(400.0),
            units::parse_length(&w.height.text(), system).unwrap_or(300.0),
            units::parse_length(&w.grid_spacing.text(), system).unwrap_or(10.0),
            w.feed_rate.text().to_string(),
            w.laser_power.text().to_string(),
            w.laser_mode
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "M4".to_string()),
        ));

        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        let json = serde_json::json!({
                            "width": w_clone.0,
                            "height": w_clone.1,
                            "grid_spacing": w_clone.2,
                            "feed_rate": w_clone.3,
                            "laser_power": w_clone.4,
                            "laser_mode": w_clone.5,
                        });
                        let _ = fs::write(path, serde_json::to_string_pretty(&json).unwrap());
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn load_params(w: &SpoilboardGridWidgets, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Load Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Open,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        let w_clone = Rc::new((
            w.width.clone(),
            w.height.clone(),
            w.grid_spacing.clone(),
            w.feed_rate.clone(),
            w.laser_power.clone(),
            w.laser_mode.clone(),
        ));
        let settings_clone = settings.clone();

        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(content) = fs::read_to_string(path) {
                            if let Ok(params) = serde_json::from_str::<serde_json::Value>(&content)
                            {
                                let system = settings_clone
                                    .persistence
                                    .borrow()
                                    .config()
                                    .ui
                                    .measurement_system;
                                if let Some(v) = params.get("width").and_then(|v| v.as_f64()) {
                                    w_clone.0.set_text(&units::format_length(v as f32, system));
                                }
                                if let Some(v) = params.get("height").and_then(|v| v.as_f64()) {
                                    w_clone.1.set_text(&units::format_length(v as f32, system));
                                }
                                if let Some(v) = params.get("grid_spacing").and_then(|v| v.as_f64())
                                {
                                    w_clone.2.set_text(&units::format_length(v as f32, system));
                                }
                                if let Some(v) = params.get("feed_rate").and_then(|v| v.as_str()) {
                                    w_clone.3.set_text(v);
                                }
                                if let Some(v) = params.get("laser_power").and_then(|v| v.as_str())
                                {
                                    w_clone.4.set_text(v);
                                }
                                if let Some(v) = params.get("laser_mode").and_then(|v| v.as_str()) {
                                    w_clone.5.set_active_id(Some(v));
                                }
                            }
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }
}

pub struct GerberTool {
    content: Box,
}

struct GerberWidgets {
    layer_type: ComboBoxText,
    feed_rate: Entry,
    spindle_speed: Entry,
    board_width: Entry,
    board_height: Entry,
    offset_x: Entry,
    offset_y: Entry,
    generate_alignment_holes: CheckButton,
    alignment_hole_diameter: Entry,
    alignment_hole_margin: Entry,
    cut_depth: Entry,
    safe_z: Entry,
    tool_diameter: Entry,
    isolation_width: Entry,
    rubout: CheckButton,
    use_board_outline: CheckButton,
    layer_files: Rc<RefCell<HashMap<GerberLayerType, PathBuf>>>,
    file_label: Label,
}

impl GerberTool {
    pub fn new<F: Fn(String) + 'static>(
        stack: &Stack,
        settings: Rc<SettingsController>,
        on_generate: Rc<F>,
    ) -> Self {
        let content_box = Box::new(Orientation::Vertical, 0);
        // content_box.set_hexpand(true);
        // content_box.set_vexpand(true);

        // Header
        let header = Box::new(Orientation::Horizontal, 12);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        header.set_margin_start(12);
        header.set_margin_end(12);

        let back_btn = Button::builder().icon_name("go-previous-symbolic").build();
        let stack_clone = stack.clone();
        back_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });
        header.append(&back_btn);

        let title = Label::builder()
            .label("Gerber to G-Code")
            .css_classes(vec!["title-2"])
            .build();
        title.set_hexpand(true);
        title.set_halign(Align::Start);
        header.append(&title);

        header.append(&help_browser::make_help_button("gerber"));

        content_box.append(&header);

        let paned = Paned::new(Orientation::Horizontal);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Sidebar
        let sidebar = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .min_content_width(300)
            .build();
        let sidebar_box = Box::new(Orientation::Vertical, 12);
        sidebar_box.set_margin_top(12);
        sidebar_box.set_margin_bottom(12);
        sidebar_box.set_margin_start(12);
        sidebar_box.set_margin_end(12);
        sidebar.set_child(Some(&sidebar_box));

        // File Selection
        let file_group = PreferencesGroup::new();
        file_group.set_title("Gerber Project");

        let file_row = ActionRow::builder().title("Select Directory").build();
        let file_btn = Button::with_label("Browse...");
        let file_label = Label::new(Some("No directory selected"));
        file_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        file_label.set_width_chars(20);

        let file_box = Box::new(Orientation::Horizontal, 6);
        file_box.append(&file_label);
        file_box.append(&file_btn);
        file_row.add_suffix(&file_box);
        file_group.add(&file_row);
        sidebar_box.append(&file_group);

        let layer_files = Rc::new(RefCell::new(HashMap::new()));
        let layer_files_clone = layer_files.clone();
        let file_label_clone = file_label.clone();

        // Helper to detect layers
        let detect_layers = |path: &std::path::Path| -> HashMap<GerberLayerType, PathBuf> {
            let mut map = HashMap::new();
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    let name = path.file_name().unwrap().to_string_lossy().to_lowercase();
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();

                    // Simple heuristic matching
                    if ext == "gtl" || name.contains("f.cu") || name.contains("top.gbr") {
                        map.insert(GerberLayerType::TopCopper, path.clone());
                    } else if ext == "gbl" || name.contains("b.cu") || name.contains("bot.gbr") {
                        map.insert(GerberLayerType::BottomCopper, path.clone());
                    } else if ext == "gts" || name.contains("f.mask") || name.contains("smask_top")
                    {
                        map.insert(GerberLayerType::TopSolderMask, path.clone());
                    } else if ext == "gbs" || name.contains("b.mask") || name.contains("smask_bot")
                    {
                        map.insert(GerberLayerType::BottomSolderMask, path.clone());
                    } else if ext == "gto"
                        || name.contains("f.silks")
                        || name.contains("legend_top")
                    {
                        map.insert(GerberLayerType::TopScreenPrint, path.clone());
                    } else if ext == "gbo"
                        || name.contains("b.silks")
                        || name.contains("legend_bot")
                    {
                        map.insert(GerberLayerType::BottomScreenPrint, path.clone());
                    } else if ext == "drl" || ext == "txt" || name.contains("drill") {
                        map.insert(GerberLayerType::DrillHoles, path.clone());
                    } else if ext == "gko"
                        || ext == "gm1"
                        || name.contains("edge.cuts")
                        || name.contains("outline")
                    {
                        map.insert(GerberLayerType::BoardOutline, path.clone());
                    }
                }
            }
            map
        };

        file_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Open Gerber Directory"),
                None::<&gtk4::Window>,
                FileChooserAction::SelectFolder,
                &[
                    ("Cancel", ResponseType::Cancel),
                    ("Select", ResponseType::Accept),
                ],
            );
            dialog.set_default_size(800, 600);

            let lf = layer_files_clone.clone();
            let fl = file_label_clone.clone();

            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let map = detect_layers(&path);
                            *lf.borrow_mut() = map;
                            fl.set_text(path.file_name().unwrap().to_str().unwrap());
                        }
                    }
                }
                d.close();
            });
            dialog.show();
        });

        // Parameters
        let params_group = PreferencesGroup::new();
        params_group.set_title("Parameters");

        let layer_type = ComboBoxText::new();
        layer_type.append(Some("TopCopper"), "Top Copper");
        layer_type.append(Some("BottomCopper"), "Bottom Copper");
        layer_type.append(Some("TopSolderMask"), "Top Solder Mask");
        layer_type.append(Some("BottomSolderMask"), "Bottom Solder Mask");
        layer_type.append(Some("TopScreenPrint"), "Top Screen Print");
        layer_type.append(Some("BottomScreenPrint"), "Bottom Screen Print");
        layer_type.append(Some("DrillHoles"), "Drill Holes");
        layer_type.append(Some("BoardOutline"), "Board Outline");
        layer_type.set_active_id(Some("TopCopper"));

        let layer_row = ActionRow::builder().title("Layer Type").build();
        layer_row.add_suffix(&layer_type);
        params_group.add(&layer_row);

        let (w_row, board_width, _) = create_dimension_row("Board Width", 100.0, &settings);
        params_group.add(&w_row);
        let (h_row, board_height, _) = create_dimension_row("Board Height", 100.0, &settings);
        params_group.add(&h_row);

        let (ox_row, offset_x, _) = create_dimension_row("Offset X", 0.0, &settings);
        params_group.add(&ox_row);
        let (oy_row, offset_y, _) = create_dimension_row("Offset Y", 0.0, &settings);
        params_group.add(&oy_row);

        let feed_rate = Entry::builder()
            .text("100.0")
            .width_chars(8)
            .valign(Align::Center)
            .build();
        let fr_row = ActionRow::builder().title("Feed Rate").build();
        fr_row.add_suffix(&feed_rate);
        params_group.add(&fr_row);

        let spindle_speed = Entry::builder()
            .text("10000.0")
            .width_chars(8)
            .valign(Align::Center)
            .build();
        let ss_row = ActionRow::builder().title("Spindle Speed (RPM)").build();
        ss_row.add_suffix(&spindle_speed);
        params_group.add(&ss_row);

        let (cd_row, cut_depth, _) = create_dimension_row("Cut Depth", -0.1, &settings);
        params_group.add(&cd_row);

        let (sz_row, safe_z, _) = create_dimension_row("Safe Z", 5.0, &settings);
        params_group.add(&sz_row);

        let (td_row, tool_diameter, _) = create_dimension_row("Tool Diameter", 0.1, &settings);
        params_group.add(&td_row);

        let (iw_row, isolation_width, _) = create_dimension_row("Isolation Width", 0.0, &settings);
        params_group.add(&iw_row);

        let rubout = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let rubout_row = ActionRow::builder().title("Remove Excess Copper").build();
        rubout_row.add_suffix(&rubout);
        params_group.add(&rubout_row);

        let use_board_outline = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let ubo_row = ActionRow::builder()
            .title("Use Board Outline for Rubout")
            .build();
        ubo_row.add_suffix(&use_board_outline);
        params_group.add(&ubo_row);

        sidebar_box.append(&params_group);

        // Alignment Holes
        let align_group = PreferencesGroup::new();
        align_group.set_title("Alignment Holes");

        let generate_alignment_holes = CheckButton::builder()
            .active(false)
            .valign(Align::Center)
            .build();
        let ah_row = ActionRow::builder()
            .title("Generate Alignment Holes")
            .build();
        ah_row.add_suffix(&generate_alignment_holes);
        align_group.add(&ah_row);

        let (ahd_row, alignment_hole_diameter, _) =
            create_dimension_row("Hole Diameter", 3.175, &settings);
        align_group.add(&ahd_row);

        let (ahm_row, alignment_hole_margin, _) = create_dimension_row("Margin", 5.0, &settings);
        align_group.add(&ahm_row);

        sidebar_box.append(&align_group);

        // Left Panel (Description)
        let left_panel = Box::new(Orientation::Vertical, 12);
        left_panel.add_css_class("sidebar");
        left_panel.set_margin_top(24);
        left_panel.set_margin_bottom(24);
        left_panel.set_margin_start(24);
        left_panel.set_margin_end(24);

        let title_label = Label::builder()
            .label("Gerber to G-Code")
            .css_classes(vec!["title-3"])
            .halign(Align::Start)
            .build();
        left_panel.append(&title_label);

        let desc = Label::builder()
            .label("Convert standard Gerber files (PCB layers) into G-Code for CNC isolation routing. Supports copper clearing (rubout), alignment holes, and custom tool parameters.")
            .css_classes(vec!["body"])
            .wrap(true)
            .halign(Align::Start)
            .build();
        left_panel.append(&desc);

        // Right Panel (Controls + Actions)
        let right_panel = Box::new(Orientation::Vertical, 0);
        sidebar.set_vexpand(true);
        right_panel.append(&sidebar);

        // Actions
        let action_box = Box::new(Orientation::Horizontal, 12);
        action_box.set_margin_top(12);
        action_box.set_margin_bottom(12);
        action_box.set_margin_start(12);
        action_box.set_margin_end(12);
        action_box.set_halign(Align::End);

        let load_btn = Button::with_label("Load");
        let save_btn = Button::with_label("Save");
        let cancel_btn = Button::with_label("Cancel");
        let generate_btn = Button::with_label("Generate G-Code");
        generate_btn.add_css_class("suggested-action");

        action_box.append(&load_btn);
        action_box.append(&save_btn);
        action_box.append(&cancel_btn);
        action_box.append(&generate_btn);
        right_panel.append(&action_box);

        paned.set_start_child(Some(&left_panel));
        paned.set_end_child(Some(&right_panel));
        set_paned_initial_fraction(&paned, 0.40);
        content_box.append(&paned);

        let widgets = Rc::new(GerberWidgets {
            layer_type: layer_type.clone(),
            feed_rate,
            spindle_speed,
            board_width,
            board_height,
            offset_x,
            offset_y,
            generate_alignment_holes,
            alignment_hole_diameter,
            alignment_hole_margin,
            cut_depth,
            safe_z,
            tool_diameter,
            isolation_width,
            rubout,
            use_board_outline,
            layer_files,
            file_label,
        });

        // Update label when layer changes
        let w_layer = widgets.clone();
        layer_type.connect_changed(move |c| {
            if let Some(id) = c.active_id() {
                let layer_type = match id.as_str() {
                    "TopCopper" => GerberLayerType::TopCopper,
                    "BottomCopper" => GerberLayerType::BottomCopper,
                    "TopSolderMask" => GerberLayerType::TopSolderMask,
                    "BottomSolderMask" => GerberLayerType::BottomSolderMask,
                    "TopScreenPrint" => GerberLayerType::TopScreenPrint,
                    "BottomScreenPrint" => GerberLayerType::BottomScreenPrint,
                    "DrillHoles" => GerberLayerType::DrillHoles,
                    "BoardOutline" => GerberLayerType::BoardOutline,
                    _ => GerberLayerType::TopCopper,
                };

                let files = w_layer.layer_files.borrow();
                if let Some(path) = files.get(&layer_type) {
                    w_layer
                        .file_label
                        .set_text(path.file_name().unwrap().to_str().unwrap());
                } else {
                    w_layer.file_label.set_text("Layer not found in directory");
                }
            }
        });

        // Connect Generate
        let w_gen = widgets.clone();
        let settings_gen = settings.clone();
        let on_gen = on_generate.clone();

        generate_btn.connect_clicked(move |_| {
            let files = w_gen.layer_files.borrow();
            if files.is_empty() {
                CamToolsView::show_error_dialog("Error", "Please select a Gerber directory.");
                return;
            }

            let params = Self::collect_params(&w_gen, &settings_gen);

            let path = match files.get(&params.layer_type) {
                Some(p) => p,
                None => {
                    CamToolsView::show_error_dialog(
                        "Error",
                        "Selected layer not found in directory.",
                    );
                    return;
                }
            };

            warn!("Reading Gerber file: {:?}", path);

            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    CamToolsView::show_error_dialog(
                        "Error",
                        &format!("Failed to read file: {}", e),
                    );
                    return;
                }
            };

            warn!("Read {} bytes", content.len());
            warn!(
                "Generating G-Code for layer: {:?} with params: {:?}",
                params.layer_type, params
            );

            match GerberConverter::generate(&params, &content) {
                Ok(gcode) => {
                    warn!("Generated {} bytes of G-Code", gcode.len());
                    on_gen(gcode)
                }
                Err(e) => {
                    warn!("Generation failed: {}", e);
                    CamToolsView::show_error_dialog("Generation Failed", &e.to_string())
                }
            }
        });

        // Save
        let w_save = widgets.clone();
        let settings_save = settings.clone();
        save_btn.connect_clicked(move |_| {
            let params = Self::collect_params(&w_save, &settings_save);
            Self::save_params(&params);
        });

        // Load
        let w_load = widgets.clone();
        let settings_load = settings.clone();
        load_btn.connect_clicked(move |_| {
            Self::load_params(&w_load, &settings_load);
        });

        let stack_clone = stack.clone();
        cancel_btn.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("dashboard");
        });

        Self {
            content: content_box,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    fn collect_params(w: &GerberWidgets, settings: &Rc<SettingsController>) -> GerberParameters {
        let system = settings.persistence.borrow().config().ui.measurement_system;

        let layer_type = match w.layer_type.active_id().as_deref() {
            Some("TopCopper") => GerberLayerType::TopCopper,
            Some("BottomCopper") => GerberLayerType::BottomCopper,
            Some("TopSolderMask") => GerberLayerType::TopSolderMask,
            Some("BottomSolderMask") => GerberLayerType::BottomSolderMask,
            Some("TopScreenPrint") => GerberLayerType::TopScreenPrint,
            Some("BottomScreenPrint") => GerberLayerType::BottomScreenPrint,
            Some("DrillHoles") => GerberLayerType::DrillHoles,
            Some("BoardOutline") => GerberLayerType::BoardOutline,
            _ => GerberLayerType::TopCopper,
        };

        GerberParameters {
            layer_type,
            feed_rate: w.feed_rate.text().parse().unwrap_or(100.0),
            spindle_speed: w.spindle_speed.text().parse().unwrap_or(10000.0),
            board_width: units::parse_length(&w.board_width.text(), system).unwrap_or(100.0) as f32,
            board_height: units::parse_length(&w.board_height.text(), system).unwrap_or(100.0)
                as f32,
            offset_x: units::parse_length(&w.offset_x.text(), system).unwrap_or(0.0) as f32,
            offset_y: units::parse_length(&w.offset_y.text(), system).unwrap_or(0.0) as f32,
            generate_alignment_holes: w.generate_alignment_holes.is_active(),
            alignment_hole_diameter: units::parse_length(&w.alignment_hole_diameter.text(), system)
                .unwrap_or(3.175) as f32,
            alignment_hole_margin: units::parse_length(&w.alignment_hole_margin.text(), system)
                .unwrap_or(5.0) as f32,
            cut_depth: units::parse_length(&w.cut_depth.text(), system).unwrap_or(-0.1) as f32,
            safe_z: units::parse_length(&w.safe_z.text(), system).unwrap_or(5.0) as f32,
            tool_diameter: units::parse_length(&w.tool_diameter.text(), system).unwrap_or(0.1)
                as f32,
            isolation_width: units::parse_length(&w.isolation_width.text(), system).unwrap_or(0.0)
                as f32,
            rubout: w.rubout.is_active(),
            use_board_outline: w.use_board_outline.is_active(),
            directory: {
                let files = w.layer_files.borrow();
                // Just grab the parent dir of the first file found, or None
                files
                    .values()
                    .next()
                    .and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string()))
            },
        }
    }

    fn save_params(params: &GerberParameters) {
        let dialog = FileChooserDialog::new(
            Some("Save Gerber Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Save", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);
        dialog.set_current_name("gerber_params.json");

        let params_clone = params.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(json) = serde_json::to_string_pretty(&params_clone) {
                            let _ = fs::write(path, json);
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn load_params(w: &Rc<GerberWidgets>, settings: &Rc<SettingsController>) {
        let dialog = FileChooserDialog::new(
            Some("Load Gerber Parameters"),
            None::<&gtk4::Window>,
            FileChooserAction::Open,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );
        dialog.set_default_size(900, 700);

        let w_clone = w.clone();
        let settings_clone = settings.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        if let Ok(content) = fs::read_to_string(path) {
                            match serde_json::from_str::<GerberParameters>(&content) {
                                Ok(params) => {
                                    Self::apply_params(&w_clone, &params, &settings_clone)
                                }
                                Err(e) => warn!("Failed to load parameters: {}", e),
                            }
                        }
                    }
                }
            }
            d.close();
        });

        dialog.show();
    }

    fn apply_params(w: &GerberWidgets, p: &GerberParameters, settings: &Rc<SettingsController>) {
        let system = settings.persistence.borrow().config().ui.measurement_system;

        let layer_id = match p.layer_type {
            GerberLayerType::TopCopper => "TopCopper",
            GerberLayerType::BottomCopper => "BottomCopper",
            GerberLayerType::TopSolderMask => "TopSolderMask",
            GerberLayerType::BottomSolderMask => "BottomSolderMask",
            GerberLayerType::TopScreenPrint => "TopScreenPrint",
            GerberLayerType::BottomScreenPrint => "BottomScreenPrint",
            GerberLayerType::DrillHoles => "DrillHoles",
            GerberLayerType::BoardOutline => "BoardOutline",
        };
        w.layer_type.set_active_id(Some(layer_id));

        w.feed_rate.set_text(&p.feed_rate.to_string());
        w.spindle_speed.set_text(&p.spindle_speed.to_string());
        w.board_width
            .set_text(&units::format_length(p.board_width, system));
        w.board_height
            .set_text(&units::format_length(p.board_height, system));
        w.offset_x
            .set_text(&units::format_length(p.offset_x, system));
        w.offset_y
            .set_text(&units::format_length(p.offset_y, system));
        w.generate_alignment_holes
            .set_active(p.generate_alignment_holes);
        w.alignment_hole_diameter
            .set_text(&units::format_length(p.alignment_hole_diameter, system));
        w.alignment_hole_margin
            .set_text(&units::format_length(p.alignment_hole_margin, system));
        w.cut_depth
            .set_text(&units::format_length(p.cut_depth, system));
        w.safe_z.set_text(&units::format_length(p.safe_z, system));
        w.tool_diameter
            .set_text(&units::format_length(p.tool_diameter, system));
        w.isolation_width
            .set_text(&units::format_length(p.isolation_width, system));
        w.rubout.set_active(p.rubout);
        w.use_board_outline.set_active(p.use_board_outline);

        if let Some(dir) = &p.directory {
            let path = PathBuf::from(dir);
            if path.exists() {
                // Re-run detection
                // We need access to detect_layers logic here, but it's inside new().
                // We can duplicate the logic or refactor. Duplication is easier for now as it's small.
                let mut map = HashMap::new();
                if let Ok(entries) = fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if !path.is_file() {
                            continue;
                        }

                        let name = path.file_name().unwrap().to_string_lossy().to_lowercase();
                        let ext = path
                            .extension()
                            .map(|e| e.to_string_lossy().to_lowercase())
                            .unwrap_or_default();

                        if ext == "gtl" || name.contains("f.cu") || name.contains("top.gbr") {
                            map.insert(GerberLayerType::TopCopper, path.clone());
                        } else if ext == "gbl" || name.contains("b.cu") || name.contains("bot.gbr")
                        {
                            map.insert(GerberLayerType::BottomCopper, path.clone());
                        } else if ext == "gts"
                            || name.contains("f.mask")
                            || name.contains("smask_top")
                        {
                            map.insert(GerberLayerType::TopSolderMask, path.clone());
                        } else if ext == "gbs"
                            || name.contains("b.mask")
                            || name.contains("smask_bot")
                        {
                            map.insert(GerberLayerType::BottomSolderMask, path.clone());
                        } else if ext == "gto"
                            || name.contains("f.silks")
                            || name.contains("legend_top")
                        {
                            map.insert(GerberLayerType::TopScreenPrint, path.clone());
                        } else if ext == "gbo"
                            || name.contains("b.silks")
                            || name.contains("legend_bot")
                        {
                            map.insert(GerberLayerType::BottomScreenPrint, path.clone());
                        } else if ext == "drl" || ext == "txt" || name.contains("drill") {
                            map.insert(GerberLayerType::DrillHoles, path.clone());
                        } else if ext == "gko"
                            || ext == "gm1"
                            || name.contains("edge.cuts")
                            || name.contains("outline")
                        {
                            map.insert(GerberLayerType::BoardOutline, path.clone());
                        }
                    }
                }
                *w.layer_files.borrow_mut() = map;
                w.file_label
                    .set_text(path.file_name().unwrap().to_str().unwrap());

                // Update label for selected layer
                let files = w.layer_files.borrow();
                if let Some(path) = files.get(&p.layer_type) {
                    w.file_label
                        .set_text(path.file_name().unwrap().to_str().unwrap());
                } else {
                    w.file_label.set_text("Layer not found in directory");
                }
            }
        }
    }
}
