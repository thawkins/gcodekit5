//! CAM Tools module - provides various CAM tool generators
//!
//! This module contains UI components for various Computer-Aided Manufacturing tools
//! including box makers, engravers, spoilboard tools, and more.

mod bitmap_engraving;
mod common;
mod drill_press;
mod gerber;
mod jigsaw;
mod speeds_feeds;
mod spoilboard_grid;
mod spoilboard_surfacing;
mod tabbed_box;
mod vector_engraving;

// Re-export all tool types
pub use bitmap_engraving::BitmapEngravingTool;
pub use drill_press::DrillPressTool;
pub use gerber::GerberTool;
pub use jigsaw::JigsawTool;
pub use speeds_feeds::SpeedsFeedsTool;
pub use spoilboard_grid::SpoilboardGridTool;
pub use spoilboard_surfacing::SpoilboardSurfacingTool;
pub use tabbed_box::TabbedBoxMaker;
pub use vector_engraving::VectorEngravingTool;

use common::set_paned_initial_fraction;

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, ComboBoxText, Image, Label, Orientation, Paned, ScrolledWindow, Stack,
};
use std::rc::Rc;

use crate::ui::gtk::machine_control::MachineControlView;
use gcodekit5_settings::SettingsController;

pub struct CamToolsView {
    pub content: Stack,
}

impl CamToolsView {
    fn show_error_dialog(title: &str, message: &str) {
        crate::ui::gtk::file_dialog::show_error_dialog(title, message, None);
    }

    pub fn new<F: Fn(String) + 'static>(
        settings: Rc<SettingsController>,
        machine_control: Option<MachineControlView>,
        on_generate: F,
    ) -> Self {
        Self::new_with_designer(settings, machine_control, on_generate, None)
    }

    pub fn new_with_designer<F: Fn(String) + 'static>(
        settings: Rc<SettingsController>,
        machine_control: Option<MachineControlView>,
        on_generate: F,
        designer_view: Option<Rc<crate::ui::gtk::designer::DesignerView>>,
    ) -> Self {
        let on_generate = Rc::new(on_generate);
        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);

        // Dashboard Page
        let dashboard = Self::create_dashboard(&stack);
        stack.add_named(&dashboard, Some("dashboard"));

        // Tool Pages
        let tabbed_box = TabbedBoxMaker::new(
            &stack,
            settings.clone(),
            on_generate.clone(),
            designer_view.clone(),
        );
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

        // Drill Press Tool
        let drill_tool = DrillPressTool::new(
            &stack,
            settings.clone(),
            machine_control.clone(),
            on_generate.clone(),
        );
        stack.add_named(drill_tool.widget(), Some("drill_press"));

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
                title: "Speeds and Feeds Calculator",
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
            Tool {
                page: "drill_press",
                title: "Drill Press",
                desc: "Emulate a drill press with peck drilling and helical cycles",
                icon: "input-mouse-symbolic",
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
