use crate::ui::gtk::status_bar::StatusBar;
use glib;
use gtk4::prelude::*;
use gtk4::{
    Box, Button, Entry, Label, Orientation, Overlay, PolicyType, ResponseType, ScrolledWindow,
};
use sourceview5::prelude::*;
use sourceview5::{
    Buffer, LanguageManager, SearchContext, SearchSettings, StyleSchemeManager, View,
};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use tracing::error;

pub struct GcodeEditor {
    pub widget: Overlay,
    pub view: View,
    pub buffer: Buffer,
    _line_counter_label: Label,
    current_file: Rc<RefCell<Option<PathBuf>>>,
    _search_context: SearchContext,
    _search_settings: SearchSettings,
    _status_bar: Option<StatusBar>,
}

impl GcodeEditor {
    pub fn new(status_bar: Option<StatusBar>) -> Self {
        let buffer = Buffer::new(None);
        let view = View::with_buffer(&buffer);

        view.set_show_line_numbers(true);
        view.set_monospace(true);
        view.set_highlight_current_line(true);
        view.set_tab_width(4);
        view.set_insert_spaces_instead_of_tabs(true);
        view.set_show_right_margin(true);
        view.set_right_margin_position(80);

        // Try to set a dark style scheme if available, matching the app's dark theme
        let scheme_manager = StyleSchemeManager::default();

        // Add custom search path for styles
        let current_style_paths = scheme_manager.search_path();
        let mut new_style_paths: Vec<String> =
            current_style_paths.iter().map(|s| s.to_string()).collect();

        if let Ok(cwd) = std::env::current_dir() {
            let assets_path = cwd.join("assets");
            if assets_path.exists() {
                if let Some(path_str) = assets_path.to_str() {
                    new_style_paths.push(path_str.to_string());
                }
            }
            let styles_path = assets_path.join("styles");
            if styles_path.exists() {
                if let Some(path_str) = styles_path.to_str() {
                    new_style_paths.push(path_str.to_string());
                }
            }
        }

        let style_path_refs: Vec<&str> = new_style_paths.iter().map(|s| s.as_str()).collect();
        scheme_manager.set_search_path(&style_path_refs);

        // Try to load our custom bright scheme first
        if let Some(scheme) = scheme_manager.scheme("gcode-bright") {
            buffer.set_style_scheme(Some(&scheme));
        } else if let Some(scheme) = scheme_manager.scheme("kate") {
            buffer.set_style_scheme(Some(&scheme));
        } else if let Some(scheme) = scheme_manager.scheme("classic") {
            buffer.set_style_scheme(Some(&scheme));
        }

        // Load G-code language definition
        let lm = LanguageManager::default();

        // Add custom search path for language specs
        let current_paths = lm.search_path();
        let mut new_paths: Vec<String> = current_paths.iter().map(|s| s.to_string()).collect();

        // Check for assets/language-specs in current directory (dev mode)
        if let Ok(cwd) = std::env::current_dir() {
            let assets_path = cwd.join("assets");

            // Add assets directory (manager might look for language-specs subdir)
            if assets_path.exists() {
                if let Some(path_str) = assets_path.to_str() {
                    new_paths.push(path_str.to_string());
                }
            }

            // Also add direct path to language-specs just in case
            let specs_path = assets_path.join("language-specs");
            if specs_path.exists() {
                if let Some(path_str) = specs_path.to_str() {
                    new_paths.push(path_str.to_string());
                }
            }
        }

        let path_refs: Vec<&str> = new_paths.iter().map(|s| s.as_str()).collect();
        lm.set_search_path(&path_refs);

        if let Some(lang) = lm.language("gcode") {
            buffer.set_language(Some(&lang));
        } else {
            // Fallback or log if needed
            tracing::warn!(
                "G-code language definition not found in search path: {:?}",
                new_paths
            );
        }

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .child(&view)
            .build();

        // Create overlay to hold scrolled window and floating panel
        let overlay = Overlay::new();
        overlay.set_child(Some(&scrolled));

        // Create floating line counter panel (bottom right)
        let line_counter_box = Box::new(Orientation::Horizontal, 4);
        line_counter_box.add_css_class("visualizer-osd");
        line_counter_box.set_halign(gtk4::Align::End);
        line_counter_box.set_valign(gtk4::Align::End);
        line_counter_box.set_margin_bottom(20);
        line_counter_box.set_margin_end(20);

        let line_counter_label = Label::builder().label("Line 1 / 1").build();
        line_counter_box.append(&line_counter_label);

        overlay.add_overlay(&line_counter_box);

        // --- Search Panel ---
        let search_settings = SearchSettings::new();
        let search_context = SearchContext::new(&buffer, Some(&search_settings));
        search_context.set_highlight(true);

        let search_box = Box::new(Orientation::Vertical, 4);
        search_box.add_css_class("visualizer-osd");
        search_box.set_halign(gtk4::Align::End);
        search_box.set_valign(gtk4::Align::Start);
        search_box.set_margin_top(20);
        search_box.set_margin_end(20);

        // Search Row
        let search_row = Box::new(Orientation::Horizontal, 4);
        let search_entry = Entry::builder()
            .placeholder_text("Search...")
            .width_request(200)
            .build();
        let prev_btn = Button::builder()
            .icon_name("go-up-symbolic")
            .tooltip_text("Previous Match")
            .build();
        let next_btn = Button::builder()
            .icon_name("go-down-symbolic")
            .tooltip_text("Next Match")
            .build();

        search_row.append(&search_entry);
        search_row.append(&prev_btn);
        search_row.append(&next_btn);

        // Replace Row
        let replace_row = Box::new(Orientation::Horizontal, 4);
        let replace_entry = Entry::builder()
            .placeholder_text("Replace with...")
            .width_request(200)
            .build();
        let replace_btn = Button::builder()
            .icon_name("edit-find-replace-symbolic")
            .tooltip_text("Replace")
            .build();
        let replace_all_btn = Button::builder()
            .icon_name("mail-send-receive-symbolic")
            .tooltip_text("Replace All")
            .build();

        replace_row.append(&replace_entry);
        replace_row.append(&replace_btn);
        replace_row.append(&replace_all_btn);

        // Status Row
        let status_row = Box::new(Orientation::Horizontal, 4);
        let match_label = Label::builder()
            .label("0 matches")
            .halign(gtk4::Align::Start)
            .build();
        status_row.append(&match_label);

        search_box.append(&search_row);
        search_box.append(&replace_row);
        search_box.append(&status_row);

        overlay.add_overlay(&search_box);

        // Connect Search Entry
        let settings_clone = search_settings.clone();
        search_entry.connect_changed(move |entry| {
            let text = entry.text();
            if text.is_empty() {
                settings_clone.set_search_text(None);
            } else {
                settings_clone.set_search_text(Some(text.as_str()));
            }
        });

        // Shared UI Update Logic
        let update_ui = {
            let label = match_label.clone();
            let prev_btn = prev_btn.clone();
            let next_btn = next_btn.clone();
            let replace_btn = replace_btn.clone();
            let replace_all_btn = replace_all_btn.clone();
            let replace_entry = replace_entry.clone();
            let buffer = buffer.clone();
            let context = search_context.clone();

            Rc::new(move || {
                let count = context.occurrences_count();
                let replace_text = replace_entry.text();
                let has_replace_text = !replace_text.is_empty();
                let has_matches = count > 0;

                replace_btn.set_sensitive(has_matches && has_replace_text);
                replace_all_btn.set_sensitive(has_matches && has_replace_text);

                if count == -1 {
                    label.set_label("Calculating...");
                    prev_btn.set_sensitive(false);
                    next_btn.set_sensitive(false);
                    return;
                }
                if count == 0 {
                    label.set_label("0 matches");
                    prev_btn.set_sensitive(false);
                    next_btn.set_sensitive(false);
                    return;
                }

                // Calculate n (current match index)
                let mut current_idx = 0;
                let mut iter = buffer.start_iter();

                // Get current selection start/end
                let insert = buffer.iter_at_mark(&buffer.get_insert());
                let bound = buffer.iter_at_mark(&buffer.selection_bound());
                let selection_start = if bound.offset() < insert.offset() {
                    bound
                } else {
                    insert
                };
                let selection_end = if bound.offset() > insert.offset() {
                    bound
                } else {
                    insert
                };

                let mut found_current = false;
                let mut match_count = 0;

                // Iterate to find current match index
                while let Some((start, end, _)) = context.forward(&iter) {
                    match_count += 1;
                    // Check if this match corresponds to current selection
                    if start.offset() == selection_start.offset() {
                        current_idx = match_count;
                        found_current = true;
                    }

                    iter = end;
                    if start.offset() > selection_start.offset() {
                        break;
                    }
                }

                if found_current {
                    label.set_label(&format!("{} of {} matches", current_idx, count));
                } else {
                    label.set_label(&format!("{} matches", count));
                }

                // Update buttons
                // Prev: check if there is a match before selection start
                if context.backward(&selection_start).is_some() {
                    prev_btn.set_sensitive(true);
                } else {
                    prev_btn.set_sensitive(false);
                }

                // Next: check if there is a match after selection end
                if context.forward(&selection_end).is_some() {
                    next_btn.set_sensitive(true);
                } else {
                    next_btn.set_sensitive(false);
                }
            })
        };

        // Connect Match Count
        let update_ui_clone = update_ui.clone();
        search_context.connect_notify_local(Some("occurrences-count"), move |_, _| {
            update_ui_clone();
        });

        // Connect Replace Entry
        let update_ui_clone = update_ui.clone();
        replace_entry.connect_changed(move |_| {
            update_ui_clone();
        });

        // Connect Cursor Move (Mark Set)
        let update_ui_clone = update_ui.clone();
        buffer.connect_mark_set(move |_, _, mark| {
            if mark.name().as_deref() == Some("insert")
                || mark.name().as_deref() == Some("selection_bound")
            {
                update_ui_clone();
            }
        });

        // Connect Next Button
        let view_clone = view.clone();
        let buffer_clone = buffer.clone();
        let context_clone = search_context.clone();
        next_btn.connect_clicked(move |_| {
            let insert = buffer_clone.iter_at_mark(&buffer_clone.get_insert());
            let bound = buffer_clone.iter_at_mark(&buffer_clone.selection_bound());
            let selection_end = if bound.offset() > insert.offset() {
                bound
            } else {
                insert
            };

            if let Some((start, end, _)) = context_clone.forward(&selection_end) {
                buffer_clone.select_range(&start, &end);
                view_clone.scroll_to_mark(
                    &buffer_clone.create_mark(None, &start, false),
                    0.0,
                    true,
                    0.5,
                    0.5,
                );
                // UI update will happen via mark-set signal
            }
        });

        // Connect Previous Button
        let view_clone = view.clone();
        let buffer_clone = buffer.clone();
        let context_clone = search_context.clone();
        prev_btn.connect_clicked(move |_| {
            let insert = buffer_clone.iter_at_mark(&buffer_clone.get_insert());
            let bound = buffer_clone.iter_at_mark(&buffer_clone.selection_bound());
            let selection_start = if bound.offset() < insert.offset() {
                bound
            } else {
                insert
            };

            if let Some((start, end, _)) = context_clone.backward(&selection_start) {
                buffer_clone.select_range(&start, &end);
                view_clone.scroll_to_mark(
                    &buffer_clone.create_mark(None, &start, false),
                    0.0,
                    true,
                    0.5,
                    0.5,
                );
                // UI update will happen via mark-set signal
            }
        });

        // Connect Replace Button
        let buffer_clone = buffer.clone();
        let context_clone = search_context.clone();
        let replace_entry_clone = replace_entry.clone();
        let view_clone = view.clone();
        replace_btn.connect_clicked(move |_| {
            if let Some(insert) = buffer_clone.mark("insert") {
                if let Some(selection_bound) = buffer_clone.mark("selection_bound") {
                    let mut start = buffer_clone.iter_at_mark(&insert);
                    let mut end = buffer_clone.iter_at_mark(&selection_bound);

                    // Ensure start is before end
                    if start.offset() > end.offset() {
                        std::mem::swap(&mut start, &mut end);
                    }

                    let replace_text = replace_entry_clone.text();

                    // Check if current selection is a match
                    // Note: This is a simplified check. Ideally we check if the selection matches the search text.
                    // But context.replace() requires us to pass the match iterators.
                    // If we just want to replace the current match and move to next:

                    if context_clone
                        .replace(&mut start, &mut end, &replace_text)
                        .is_ok()
                    {
                        // Move to next match
                        let iter = end; // Continue from end of replacement
                        if let Some((next_start, next_end, _wrapped)) = context_clone.forward(&iter)
                        {
                            buffer_clone.select_range(&next_start, &next_end);
                            view_clone.scroll_to_mark(
                                &buffer_clone.create_mark(None, &next_start, false),
                                0.0,
                                true,
                                0.5,
                                0.5,
                            );
                        }
                    }
                }
            }
        });

        // Connect Replace All Button
        let buffer_clone = buffer.clone();
        let replace_entry_clone = replace_entry.clone();
        let search_settings_clone = search_settings.clone();
        let status_bar_clone = status_bar.clone();

        replace_all_btn.connect_clicked(move |_| {
            let replace_text = replace_entry_clone.text().to_string();
            let search_text = search_settings_clone
                .search_text()
                .map(|s| s.to_string())
                .unwrap_or_default();

            if search_text.is_empty() {
                return;
            }

            let buffer = buffer_clone.clone();
            let status_bar = status_bar_clone.clone();

            // Get text from buffer (must be on main thread)
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let text = buffer.text(&start, &end, true).to_string();

            if let Some(sb) = &status_bar {
                sb.set_progress(10.0, "Calculating...", "Please wait");
            }

            // Spawn thread
            let (sender, receiver) = mpsc::channel();

            thread::spawn(move || {
                // Perform replacement
                let new_text = text.replace(&search_text, &replace_text);
                let _ = sender.send(new_text);
            });

            let buffer = buffer_clone.clone();
            let status_bar = status_bar_clone.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                match receiver.try_recv() {
                    Ok(new_text) => {
                        buffer.set_text(&new_text);
                        if let Some(sb) = &status_bar {
                            sb.set_progress(0.0, "", ""); // Clear
                        }
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if let Some(sb) = &status_bar {
                            sb.set_progress(0.0, "", ""); // Clear
                        }
                        glib::ControlFlow::Break
                    }
                }
            });
        });

        let editor = Self {
            widget: overlay,
            view: view.clone(),
            buffer: buffer.clone(),
            _line_counter_label: line_counter_label.clone(),
            current_file: Rc::new(RefCell::new(None)),
            _search_context: search_context,
            _search_settings: search_settings,
            _status_bar: status_bar,
        };

        // Update line counter when cursor moves
        let label_clone = line_counter_label.clone();
        let buffer_clone = buffer.clone();
        view.connect_move_cursor(move |_, _, _, _| {
            Self::update_line_counter(&buffer_clone, &label_clone);
        });

        // Update line counter when buffer changes
        let label_clone = line_counter_label.clone();
        let buffer_clone = buffer.clone();
        buffer.connect_changed(move |_| {
            Self::update_line_counter(&buffer_clone, &label_clone);
        });

        // Update line counter when cursor position changes (mark-set signal)
        let label_clone = line_counter_label.clone();
        let buffer_clone = buffer.clone();
        buffer.connect_mark_set(move |_, _, mark| {
            // Only update for insert mark (cursor position)
            if mark.name().as_deref() == Some("insert") {
                Self::update_line_counter(&buffer_clone, &label_clone);
            }
        });

        // Initial update
        Self::update_line_counter(&buffer, &line_counter_label);

        editor
    }

    fn update_line_counter(buffer: &Buffer, label: &Label) {
        let total_lines = buffer.line_count();
        let insert_mark = buffer.get_insert();
        let cursor_iter = buffer.iter_at_mark(&insert_mark);
        let current_line = cursor_iter.line() + 1; // Lines are 0-indexed
        label.set_text(&format!("Line {} / {}", current_line, total_lines));
    }

    pub fn set_text(&self, text: &str) {
        self.buffer.set_text(text);
        // Move cursor to start (line 1, column 1)
        let mut start_iter = self.buffer.start_iter();
        self.buffer.place_cursor(&start_iter);
        // Scroll to top
        self.view
            .scroll_to_iter(&mut start_iter, 0.0, false, 0.0, 0.0);
    }

    pub fn get_text(&self) -> String {
        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        self.buffer.text(&start, &end, true).to_string()
    }

    pub fn grab_focus(&self) {
        self.view.grab_focus();
    }

    pub fn connect_changed<F: Fn(&Buffer) + 'static>(&self, f: F) {
        self.buffer.connect_changed(f);
    }

    pub fn undo(&self) {
        if self.buffer.can_undo() {
            self.buffer.undo();
        }
    }

    pub fn redo(&self) {
        if self.buffer.can_redo() {
            self.buffer.redo();
        }
    }

    pub fn cut(&self) {
        let clipboard = self.widget.clipboard();
        self.buffer.cut_clipboard(&clipboard, true);
    }

    pub fn copy(&self) {
        let clipboard = self.widget.clipboard();
        self.buffer.copy_clipboard(&clipboard);
    }

    pub fn paste(&self) {
        let clipboard = self.widget.clipboard();
        self.buffer.paste_clipboard(&clipboard, None, true);
    }

    pub fn new_file(&self) {
        self.set_text("");
        *self.current_file.borrow_mut() = None;
    }

    pub fn open_file(&self) {
        let parent = super::file_dialog::parent_window(&self.widget);
        let dialog = super::file_dialog::open_dialog("Open G-Code File", parent.as_ref());

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("G-Code Files"));
        filter.add_pattern("*.gcode");
        filter.add_pattern("*.nc");
        filter.add_pattern("*.gc");
        filter.add_pattern("*.tap");
        dialog.add_filter(&filter);

        let all_filter = gtk4::FileFilter::new();
        all_filter.set_name(Some("All Files"));
        all_filter.add_pattern("*");
        dialog.add_filter(&all_filter);

        let buffer = self.buffer.clone();
        let current_file = self.current_file.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        match fs::read_to_string(&path) {
                            Ok(content) => {
                                buffer.set_text(&content);
                                *current_file.borrow_mut() = Some(path);
                                // Move cursor to start
                                let start_iter = buffer.start_iter();
                                buffer.place_cursor(&start_iter);
                            }
                            Err(e) => {
                                error!("Error reading file {}: {}", path.display(), e);
                                let parent = super::file_dialog::parent_window(dialog);
                                super::file_dialog::show_error_dialog(
                                    "Error Reading File",
                                    &format!("Could not open '{}'.\n\n{}", path.display(), e),
                                    parent.as_ref(),
                                );
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }

    pub fn save_file(&self) {
        let current_path = self.current_file.borrow().clone();

        if let Some(path) = current_path {
            let start = self.buffer.start_iter();
            let end = self.buffer.end_iter();
            let content = self.buffer.text(&start, &end, true);

            if let Err(e) = fs::write(&path, content.as_str()) {
                error!("Error saving file {}: {}", path.display(), e);
                let parent = super::file_dialog::parent_window(&self.widget);
                super::file_dialog::show_error_dialog(
                    "Error Saving File",
                    &format!("Could not save '{}'.\n\n{}", path.display(), e),
                    parent.as_ref(),
                );
            }
        } else {
            self.save_as_file();
        }
    }

    pub fn save_as_file(&self) {
        let parent = super::file_dialog::parent_window(&self.widget);
        let dialog = super::file_dialog::save_dialog("Save G-Code File", parent.as_ref());

        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("G-Code Files"));
        filter.add_pattern("*.gcode");
        filter.add_pattern("*.nc");
        filter.add_pattern("*.gc");
        filter.add_pattern("*.tap");
        dialog.add_filter(&filter);

        let buffer = self.buffer.clone();
        let current_file = self.current_file.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(mut path) = file.path() {
                        // Ensure extension
                        if path.extension().is_none() {
                            path.set_extension("gcode");
                        }

                        let start = buffer.start_iter();
                        let end = buffer.end_iter();
                        let content = buffer.text(&start, &end, true);

                        match fs::write(&path, content.as_str()) {
                            Ok(_) => {
                                *current_file.borrow_mut() = Some(path);
                            }
                            Err(e) => {
                                error!("Error saving file {}: {}", path.display(), e);
                                let parent = super::file_dialog::parent_window(dialog);
                                super::file_dialog::show_error_dialog(
                                    "Error Saving File",
                                    &format!("Could not save '{}'.\n\n{}", path.display(), e),
                                    parent.as_ref(),
                                );
                            }
                        }
                    }
                }
            }
            dialog.destroy();
        });

        dialog.show();
    }
}
