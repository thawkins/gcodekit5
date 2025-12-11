use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, Entry, Frame, Label, Orientation, Paned, ScrolledWindow, TextView,
    WrapMode,
};
use std::rc::Rc;

pub struct DeviceConsoleView {
    pub widget: Paned,
    pub console_text: TextView,
    pub command_entry: Entry,
    pub send_btn: Button,
    pub clear_btn: Button,
    pub copy_btn: Button,
}

impl DeviceConsoleView {
    pub fn new() -> Rc<Self> {
        let widget = Paned::new(Orientation::Horizontal);
        widget.set_hexpand(true);
        widget.set_vexpand(true);
    

        // We'll move 'Clear' and 'Copy' buttons into the main console area (above the output)
        let clear_btn = Button::from_icon_name("user-trash-symbolic");
        clear_btn.set_label("Clear");
        clear_btn.set_tooltip_text(Some("Clear Console Output"));

        let copy_btn = Button::from_icon_name("edit-copy-symbolic");
        copy_btn.set_label("Copy");
        copy_btn.set_tooltip_text(Some("Copy to Clipboard"));

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

        // Toolbar above Console Output
        let console_toolbar = Box::new(Orientation::Horizontal, 5);
        console_toolbar.set_halign(Align::End);
        console_toolbar.append(&clear_btn);
        console_toolbar.append(&copy_btn);

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
        main_area.append(&console_toolbar);
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

        let view = Rc::new(Self {
            widget,
            console_text,
            command_entry,
            send_btn,
            clear_btn,
            copy_btn,
        });

        // Connect signals
        let view_clone = view.clone();
        view.clear_btn.connect_clicked(move |_| {
            view_clone.console_text.buffer().set_text("");
        });

        let view_clone = view.clone();
        view.copy_btn.connect_clicked(move |_| {
            let buffer = view_clone.console_text.buffer();
            let (start, end) = buffer.bounds();
            let text = buffer.text(&start, &end, false);
            let clipboard = view_clone.widget.display().clipboard();
            clipboard.set_text(&text);
        });

        view
    }

    pub fn append_log(&self, message: &str) {
        let buffer = self.console_text.buffer();
        // Append to bottom and auto-scroll
        let mut iter = buffer.end_iter();
        buffer.insert(&mut iter, message);
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
    }
}
