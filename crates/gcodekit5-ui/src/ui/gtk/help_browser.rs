//! GTK help browser
//!
//! Loads markdown help topics from GResources and displays them in a small in-app browser.
//! Navigation uses markdown links of the form `(help:topic_id)`.

use anyhow::{anyhow, Context, Result};
use gio::prelude::*;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Application, Box, Button, HeaderBar, Label, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;

fn running_app() -> Result<Application> {
    gio::Application::default()
        .and_then(|a| a.downcast::<Application>().ok())
        .ok_or_else(|| anyhow!("No running GtkApplication"))
}

fn topic_resource_path(topic: &str) -> String {
    let topic = topic.trim();
    let topic = if topic.is_empty() { "index" } else { topic };
    format!("/com/gcodekit5/help/{}.md", topic)
}

fn load_topic_markdown(topic: &str) -> Result<String> {
    let path = topic_resource_path(topic);
    let bytes = gio::resources_lookup_data(&path, gio::ResourceLookupFlags::NONE)
        .with_context(|| format!("Missing help topic resource: {}", path))?;

    let s = std::str::from_utf8(bytes.as_ref())
        .map_err(|e| anyhow!("Invalid UTF-8 in {}: {}", path, e))?;
    Ok(s.to_string())
}

fn escape_pango(s: &str) -> String {
    glib::markup_escape_text(s).to_string()
}

fn md_line_to_markup(line: &str) -> String {
    let mut line = line.trim_end_matches('\r');

    // Headings
    let (prefix, size) = if let Some(rest) = line.strip_prefix("### ") {
        line = rest;
        ("", "large")
    } else if let Some(rest) = line.strip_prefix("## ") {
        line = rest;
        ("", "x-large")
    } else if let Some(rest) = line.strip_prefix("# ") {
        line = rest;
        ("", "xx-large")
    } else {
        ("", "")
    };

    let mut out = String::new();
    if !prefix.is_empty() {
        out.push_str(prefix);
    }

    // Very small markdown subset: [text](href)
    // Everything else is rendered as plain escaped text.
    let mut i = 0usize;
    while let Some(open) = line[i..].find('[') {
        let open = i + open;
        // emit leading text
        out.push_str(&escape_pango(&line[i..open]));

        let Some(close) = line[open + 1..].find(']') else {
            out.push_str(&escape_pango(&line[open..]));
            return wrap_heading(&out, size);
        };
        let close = open + 1 + close;

        let link_text = &line[open + 1..close];
        let after = &line[close + 1..];
        if let Some(after) = after.strip_prefix('(') {
            if let Some(end) = after.find(')') {
                let href = &after[..end];
                let safe_text = escape_pango(link_text);
                let safe_href = escape_pango(href);
                out.push_str(&format!("<a href=\"{}\">{}</a>", safe_href, safe_text));

                i = close + 2 + end + 1; // ](href)
                continue;
            }
        }

        // Not a link; render literally
        out.push_str(&escape_pango(&line[open..=close]));
        i = close + 1;
    }

    out.push_str(&escape_pango(&line[i..]));
    wrap_heading(&out, size)
}

fn wrap_heading(s: &str, size: &str) -> String {
    if size.is_empty() {
        return s.to_string();
    }
    format!("<span weight=\"bold\" size=\"{}\">{}</span>", size, s)
}

fn markdown_to_markup(md: &str) -> String {
    md.lines()
        .map(md_line_to_markup)
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_topic_link(href: &str) -> Option<String> {
    let href = href.trim();

    if let Some(rest) = href.strip_prefix("help:") {
        let topic = rest.trim().trim_start_matches('/');
        let topic = topic.split('#').next().unwrap_or(topic);
        return Some(topic.to_string());
    }

    None
}

fn is_help_image_uri(href: &str) -> bool {
    href.starts_with("resource:///com/gcodekit5/help/images/")
}

fn show_help_image(parent: Option<&gtk4::Window>, href: &str) {
    let Ok(app) = running_app() else {
        return;
    };

    let win = gtk4::Window::builder()
        .application(&app)
        .title("Help Image")
        .default_width(900)
        .default_height(600)
        .build();

    if let Some(parent) = parent {
        win.set_transient_for(Some(parent));
        win.set_modal(true);
    }

    let resource_path = href
        .strip_prefix("resource://")
        .unwrap_or(href)
        .to_string();

    let pic = gtk4::Picture::for_resource(&resource_path);
    pic.set_can_shrink(true);
    pic.set_keep_aspect_ratio(true);
    pic.set_margin_top(5);
    pic.set_margin_bottom(5);
    pic.set_margin_start(5);
    pic.set_margin_end(5);

    // GtkViewport does not reliably honor direct-child margins; put margins on the content widget.
    let pad = Box::new(Orientation::Vertical, 0);
    pad.append(&pic);

    let scroller = ScrolledWindow::builder().child(&pad).build();
    win.set_child(Some(&scroller));
    win.present();
}

/// Show the help browser at a topic ID.
pub fn present(topic: &str) {
    let Ok(app) = running_app() else {
        return;
    };
    let parent = app.active_window();
    present_for_parent(topic, parent.as_ref());
}

/// Show the help browser at a topic ID with an explicit parent.
pub fn present_for_parent(topic: &str, parent: Option<&gtk4::Window>) {
    let Ok(app) = running_app() else {
        return;
    };

    let window = gtk4::Window::builder()
        .application(&app)
        .title("Help")
        .default_width(900)
        .default_height(700)
        .build();

    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
        window.set_modal(false);
    }

    let header = HeaderBar::new();
    window.set_titlebar(Some(&header));

    let history: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![topic.to_string()]));
    let history_idx: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

    let back_btn = Button::builder()
        .icon_name("go-previous-symbolic")
        .tooltip_text("Back")
        .build();
    let fwd_btn = Button::builder()
        .icon_name("go-next-symbolic")
        .tooltip_text("Forward")
        .build();
    let home_btn = Button::builder()
        .icon_name("go-home-symbolic")
        .tooltip_text("Help index")
        .build();

    header.pack_start(&back_btn);
    header.pack_start(&fwd_btn);
    header.pack_start(&home_btn);

    let title = Label::new(Some("Help"));
    title.set_halign(Align::Center);
    header.set_title_widget(Some(&title));

    let content_label = Label::new(None);
    content_label.set_selectable(true);
    content_label.set_wrap(true);
    content_label.set_xalign(0.0);
    content_label.set_use_markup(true);
    content_label.set_margin_top(5);
    content_label.set_margin_bottom(5);
    content_label.set_margin_start(5);
    content_label.set_margin_end(5);

    // Apply theme-aware background using CSS
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(
        ".help-container {
            background-color: @theme_bg_color;
            color: @theme_fg_color;
        }
        .help-text {
            color: @theme_fg_color;
        }"
    );
    
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().unwrap(),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION
    );
    
    content_label.add_css_class("help-text");

    // GtkViewport does not reliably honor direct-child margins; put margins on the content widget.
    let content_pad = Box::new(Orientation::Vertical, 0);
    content_pad.add_css_class("help-container");
    content_pad.set_margin_top(10);
    content_pad.set_margin_bottom(10);
    content_pad.set_margin_start(10);
    content_pad.set_margin_end(10);
    content_pad.append(&content_label);

    let scroller = ScrolledWindow::builder()
        .child(&content_pad)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();
    scroller.add_css_class("help-scroller");
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);

    let root = Box::new(Orientation::Vertical, 0);
    root.set_hexpand(true);
    root.set_vexpand(true);
    root.append(&scroller);
    window.set_child(Some(&root));

    let load_and_render = {
        let window = window.clone();
        let title = title.clone();
        let content_label = content_label.clone();
        move |topic: &str| {
            match load_topic_markdown(topic) {
                Ok(md) => {
                    let markup = markdown_to_markup(&md);
                    title.set_text(topic);
                    content_label.set_markup(&markup);
                    let label_clone = content_label.clone();
                    glib::idle_add_local(move || {
                        label_clone.select_region(0, 0);
                        glib::ControlFlow::Break
                    });
                }
                Err(e) => {
                    title.set_text("Help (missing topic)");
                    content_label.set_text(&format!("Missing help topic '{topic}':\n{e}"));
                }
            }

            // Keep the window title friendly.
            window.set_title(Some("Help"));
        }
    };

    // Initial load
    {
        let topic = history.borrow()[0].clone();
        load_and_render(&topic);
    }

    // Navigation helpers
    let update_nav_sensitivity = {
        let back_btn = back_btn.clone();
        let fwd_btn = fwd_btn.clone();
        let history = history.clone();
        let history_idx = history_idx.clone();
        move || {
            let idx = *history_idx.borrow();
            let len = history.borrow().len();
            back_btn.set_sensitive(idx > 0);
            fwd_btn.set_sensitive(idx + 1 < len);
        }
    };

    update_nav_sensitivity();

    // Back
    {
        let history = history.clone();
        let history_idx = history_idx.clone();
        let load_and_render = load_and_render.clone();
        let update_nav_sensitivity = update_nav_sensitivity.clone();
        back_btn.connect_clicked(move |_| {
            let mut idx = history_idx.borrow_mut();
            if *idx == 0 {
                return;
            }
            *idx -= 1;
            if let Some(topic) = history.borrow().get(*idx).cloned() {
                load_and_render(&topic);
            }
            update_nav_sensitivity();
        });
    }

    // Forward
    {
        let history = history.clone();
        let history_idx = history_idx.clone();
        let load_and_render = load_and_render.clone();
        let update_nav_sensitivity = update_nav_sensitivity.clone();
        fwd_btn.connect_clicked(move |_| {
            let mut idx = history_idx.borrow_mut();
            let len = history.borrow().len();
            if *idx + 1 >= len {
                return;
            }
            *idx += 1;
            if let Some(topic) = history.borrow().get(*idx).cloned() {
                load_and_render(&topic);
            }
            update_nav_sensitivity();
        });
    }

    // Home
    {
        let history = history.clone();
        let history_idx = history_idx.clone();
        let load_and_render = load_and_render.clone();
        let update_nav_sensitivity = update_nav_sensitivity.clone();
        home_btn.connect_clicked(move |_| {
            history.borrow_mut().truncate(*history_idx.borrow() + 1);
            history.borrow_mut().push("index".to_string());
            *history_idx.borrow_mut() = history.borrow().len() - 1;
            load_and_render("index");
            update_nav_sensitivity();
        });
    }

    // Link navigation
    {
        let parent = parent.map(|p| p.downgrade());
        let history = history.clone();
        let history_idx = history_idx.clone();
        let load_and_render = load_and_render.clone();
        let update_nav_sensitivity = update_nav_sensitivity.clone();

        content_label.connect_activate_link(move |_, href| {
            if is_help_image_uri(href) {
                let parent = parent.as_ref().and_then(|w| w.upgrade());
                show_help_image(parent.as_ref(), href);
                return glib::Propagation::Stop;
            }

            if let Some(topic) = normalize_topic_link(href) {
                history.borrow_mut().truncate(*history_idx.borrow() + 1);
                history.borrow_mut().push(topic.clone());
                *history_idx.borrow_mut() = history.borrow().len() - 1;
                load_and_render(&topic);
                update_nav_sensitivity();
                return glib::Propagation::Stop;
            }

            // Non-help links: open using default handler.
            if let Some(parent) = parent.as_ref().and_then(|w| w.upgrade()) {
                let _ = gtk4::show_uri(Some(&parent), href, gtk4::gdk::CURRENT_TIME);
            } else {
                let _ = gtk4::show_uri(None::<&gtk4::Window>, href, gtk4::gdk::CURRENT_TIME);
            }

            glib::Propagation::Stop
        });
    }

    window.present();
}

/// Create a small icon-only help button that opens the help browser.
pub fn make_help_button(topic: &'static str) -> Button {
    let btn = Button::from_icon_name("help-browser-symbolic");
    btn.set_tooltip_text(Some("Help"));

    btn.connect_clicked(move |_| {
        present(topic);
    });

    btn
}
