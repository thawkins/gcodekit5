use gtk4::prelude::*;
use gtk4::{
    Box, Button, Entry, EventControllerKey, Label, Orientation, Paned, ScrolledWindow, TextView, WrapMode, gdk,
};
use std::{borrow::Cow, cell::RefCell, rc::Rc};

use crate::ui::gtk::command_history::CommandHistory;

pub struct DeviceConsoleView {
    pub widget: Paned,
    pub console_text: TextView,
    pub command_entry: Entry,
    pub send_btn: Button,
    history: Rc<RefCell<CommandHistory>>,
}

impl DeviceConsoleView {
    pub fn new() -> Rc<Self> {
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);

        // ═════════════════════════════════════════════
        // MAIN AREA
        // ═════════════════════════════════════════════
        let main_area = Box::new(Orientation::Vertical, 10);
        main_area.set_hexpand(true);
        main_area.set_vexpand(true);
        main_area.set_margin_top(10);
        main_area.set_margin_bottom(10);
        main_area.set_margin_start(10);
        main_area.set_margin_end(10);

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
        command_entry.set_placeholder_text(Some("Enter G-code command..."));
        command_entry.set_hexpand(true);
        input_box.append(&command_entry);

        let send_btn = Button::from_icon_name("mail-send-symbolic"); // or document-send-symbolic
        send_btn.set_tooltip_text(Some("Send Command"));
        input_box.append(&send_btn);

        main_area.append(&input_box);

        // Setup Paned

        widget.set_end_child(Some(&main_area));

        // Dynamic resizing for 20% sidebar width
        widget.add_tick_callback(|paned, _clock| {
            let width = paned.width();
            let target = (width as f64 * 0.2) as i32;
            if (paned.position() - target).abs() > 2 {
                paned.set_position(target);
            }
            gtk4::glib::ControlFlow::Continue
        });

        // Load command history
        let history = Rc::new(RefCell::new(CommandHistory::load()));

        let view = Rc::new(Self {
            widget,
            console_text,
            command_entry: command_entry.clone(),
            send_btn,
            history: history.clone(),
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
        
        // Auto-scroll to bottom after inserting
        let mark = buffer.create_mark(None, &buffer.end_iter(), false);
        self.console_text.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
        buffer.delete_mark(&mark);
    }

    pub fn get_log_text(&self) -> String {
        let buffer = self.console_text.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        buffer.text(&start, &end, true).to_string()
    }

    pub fn clear_log(&self) {
        let buffer = self.console_text.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        buffer.delete(&mut start.clone(), &mut end.clone());

        // Auto-scroll to bottom
        let mark = buffer.create_mark(None, &buffer.end_iter(), false);
        self.console_text.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
        buffer.delete_mark(&mark);
    }

    pub fn add_to_history(&self, command: String) {
        self.history.borrow_mut().add(command);
        self.history.borrow().save();
    }

    pub fn reset_history_navigation(&self) {
        self.history.borrow_mut().reset_navigation();
    }
}
