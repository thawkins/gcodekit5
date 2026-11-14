//! # Device Console Panel
//!
//! Provides a terminal-like console for direct G-code communication
//! with the connected CNC device. Displays sent commands and
//! received responses with syntax highlighting.

use gtk4::prelude::*;
use gtk4::{
    Box, Button, Entry, EventControllerKey, Label, Orientation, Paned, ScrolledWindow, TextView,
    WrapMode, gdk,
};
use std::cell::Cell;
use std::{borrow::Cow, rc::Rc};

use crate::t;
use crate::ui::gtk::command_history::CommandHistory;
use crate::ui::gtk::common::spacing;
use gcodekit5_core::{Shared, shared};

/// Maximum number of lines kept in the console log; older lines are discarded.
pub const MAX_CONSOLE_LINES: i32 = 5000;

#[derive(Clone)]
pub struct DeviceConsoleView {
    pub widget: Paned,
    pub console_text: TextView,
    pub command_entry: Entry,
    pub send_btn: Button,
    history: Shared<CommandHistory>,
    /// Bytes of log text discarded so far (trimmed or cleared), so log positions stay
    /// valid while old lines are dropped.
    discarded_bytes: Rc<Cell<usize>>,
}

impl DeviceConsoleView {
    /// Builds the console view (log, command entry and history navigation).
    pub fn new() -> Rc<Self> {
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        // ═════════════════════════════════════════════
        // MAIN AREA
        // ═════════════════════════════════════════════
        let main_area = Box::new(Orientation::Vertical, spacing::MEDIUM);
        main_area.set_hexpand(true);
        main_area.set_vexpand(true);
        main_area.set_margin_top(spacing::MEDIUM);
        main_area.set_margin_bottom(spacing::MEDIUM);
        main_area.set_margin_start(spacing::MEDIUM);
        main_area.set_margin_end(spacing::MEDIUM);

        // Console Output
        let scroll = ScrolledWindow::new();
        scroll.set_hexpand(true);
        scroll.set_vexpand(true);
        scroll.add_css_class("view"); // Standard frame look
        scroll.add_css_class("console-view"); // Custom styling

        let console_text = TextView::new();
        console_text.set_editable(false);
        console_text.set_monospace(true);
        console_text.set_wrap_mode(WrapMode::WordChar);
        console_text.set_cursor_visible(false);
        // console_text.set_bottom_margin(10);
        // console_text.set_top_margin(10);
        // console_text.set_left_margin(10);
        // console_text.set_right_margin(10);

        scroll.set_child(Some(&console_text));
        main_area.append(&scroll);

        // Command Input
        let input_box = Box::new(Orientation::Horizontal, 10);

        let prompt_label = Label::new(Some(">"));
        prompt_label.add_css_class("accent-color"); // Or similar
        prompt_label.add_css_class("title-2");
        input_box.append(&prompt_label);

        let command_entry = Entry::new();
        command_entry.set_placeholder_text(Some(&t!("Enter G-code command...")));
        command_entry.set_hexpand(true);
        input_box.append(&command_entry);

        let send_btn = Button::from_icon_name("mail-send-symbolic"); // or document-send-symbolic
        send_btn.set_tooltip_text(Some(&t!("Send Command")));
        input_box.append(&send_btn);

        main_area.append(&input_box);

        // Setup Paned

        widget.set_end_child(Some(&main_area));

        // Load command history
        let history = shared(CommandHistory::load());

        let view = Rc::new(Self {
            widget,
            console_text,
            command_entry: command_entry.clone(),
            send_btn,
            history: history.clone(),
            discarded_bytes: Rc::new(Cell::new(0)),
        });

        // Setup key event controller for command history navigation
        let key_controller = EventControllerKey::new();
        let history_clone = history.clone();
        let entry_clone = command_entry.clone();

        key_controller.connect_key_pressed(move |_, keyval, _, _| {
            match keyval {
                gdk::Key::Up => {
                    let current_text = entry_clone.text().to_string();
                    if let Some(prev_cmd) = history_clone.borrow_mut().previous(&current_text) {
                        entry_clone.set_text(&prev_cmd);
                        entry_clone.set_position(-1); // Move cursor to end
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Down => {
                    if let Some(next_cmd) = history_clone.borrow_mut().next() {
                        entry_clone.set_text(&next_cmd);
                        entry_clone.set_position(-1); // Move cursor to end
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });

        command_entry.add_controller(key_controller);

        view
    }

    /// Appends text to the log, keeps at most [`MAX_CONSOLE_LINES`] lines and scrolls to
    /// the end.
    ///
    /// # Arguments
    /// * `message` - Text to append (normally ending with a newline).
    pub fn append_log(&self, message: &str) {
        let buffer = self.console_text.buffer();
        // GTK/glib strings must not contain NUL bytes.
        let msg: Cow<'_, str> = if message.contains('\0') {
            Cow::Owned(message.replace('\0', ""))
        } else {
            Cow::Borrowed(message)
        };

        // Append to bottom and auto-scroll
        let mut iter = buffer.end_iter();
        buffer.insert(&mut iter, msg.as_ref());
        self.trim_to_max_lines();

        // Auto-scroll to bottom after inserting
        let mark = buffer.create_mark(None, &buffer.end_iter(), false);
        self.console_text.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
        buffer.delete_mark(&mark);
    }

    /// Drops the oldest lines so at most [`MAX_CONSOLE_LINES`] remain.
    fn trim_to_max_lines(&self) {
        let buffer = self.console_text.buffer();
        // A trailing newline leaves an empty last line, which is not counted.
        let lines = buffer.line_count() - i32::from(buffer.end_iter().starts_line());
        let excess = lines - MAX_CONSOLE_LINES;
        if excess <= 0 {
            return;
        }
        let mut start = buffer.start_iter();
        let Some(mut cut) = buffer.iter_at_line(excess) else {
            return;
        };
        let removed = buffer.text(&start, &cut, true).len();
        buffer.delete(&mut start, &mut cut);
        self.discarded_bytes
            .set(self.discarded_bytes.get().saturating_add(removed));
    }

    /// Returns a position marking the current end of the log.
    ///
    /// Positions count every byte ever appended, so they stay valid while old lines are
    /// trimmed. Pass one to [`Self::log_text_since`] to read what arrived afterwards.
    pub fn log_position(&self) -> usize {
        self.discarded_bytes.get() + self.get_log_text().len()
    }

    /// Returns the log text appended after `position` (from [`Self::log_position`]).
    ///
    /// If part of that text was already trimmed or cleared, the whole remaining log is
    /// returned.
    ///
    /// # Arguments
    /// * `position` - A value previously returned by `log_position`.
    pub fn log_text_since(&self, position: usize) -> String {
        let text = self.get_log_text();
        let discarded = self.discarded_bytes.get();
        match position.checked_sub(discarded) {
            Some(offset) => text.get(offset..).map(str::to_string).unwrap_or_default(),
            None => text,
        }
    }

    /// Returns the whole log text.
    pub fn get_log_text(&self) -> String {
        let buffer = self.console_text.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        buffer.text(&start, &end, true).to_string()
    }

    /// Clears the log.
    pub fn clear_log(&self) {
        let buffer = self.console_text.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let removed = buffer.text(&start, &end, true).len();
        self.discarded_bytes
            .set(self.discarded_bytes.get().saturating_add(removed));
        buffer.delete(&mut start.clone(), &mut end.clone());

        // Auto-scroll to bottom
        let mark = buffer.create_mark(None, &buffer.end_iter(), false);
        self.console_text.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
        buffer.delete_mark(&mark);
    }

    /// Adds a command to the persistent command history.
    pub fn add_to_history(&self, command: String) {
        self.history.borrow_mut().add(command);
        self.history.borrow().save();
    }

    /// Resets Up/Down history navigation to the newest entry.
    pub fn reset_history_navigation(&self) {
        self.history.borrow_mut().reset_navigation();
    }
}
