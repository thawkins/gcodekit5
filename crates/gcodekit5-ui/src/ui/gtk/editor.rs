use gtk4::prelude::*;
use gtk4::{ScrolledWindow, PolicyType};
use sourceview5::prelude::*;
use sourceview5::{View, Buffer, LanguageManager, StyleSchemeManager};

pub struct GcodeEditor {
    pub widget: ScrolledWindow,
    pub view: View,
    pub buffer: Buffer,
}

impl GcodeEditor {
    pub fn new() -> Self {
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
        if let Some(scheme) = scheme_manager.scheme("kate") { // 'kate' is often available and good for code
            buffer.set_style_scheme(Some(&scheme));
        } else if let Some(scheme) = scheme_manager.scheme("classic") {
            buffer.set_style_scheme(Some(&scheme));
        }

        // TODO: Load G-code language definition
        // let lm = LanguageManager::default();
        // if let Some(lang) = lm.language("gcode") {
        //     buffer.set_language(Some(&lang));
        // }

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .child(&view)
            .build();

        Self {
            widget: scrolled,
            view,
            buffer,
        }
    }

    pub fn set_text(&self, text: &str) {
        self.buffer.set_text(text);
    }

    pub fn get_text(&self) -> String {
        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        self.buffer.text(&start, &end, true).to_string()
    }
    
    pub fn connect_changed<F: Fn(&Buffer) + 'static>(&self, f: F) {
        self.buffer.connect_changed(f);
    }
}
