//! # Dialog window
//!
//! Replacement for the deprecated `gtk4::Dialog` (deprecated since GTK 4.10).
//!
//! [`DialogWindow`] is a plain `gtk4::Window` with a content area and a row of
//! response buttons, keeping the subset of the `Dialog` API this crate uses
//! (`add_button`, `content_area`, `connect_response`, `response`,
//! `set_default_response`). As with `Dialog`, closing the window (title-bar button
//! or Escape) reports [`ResponseType::DeleteEvent`] and then lets the window close.
//!
//! Dependencies: `gtk4`, `glib`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, CallbackAction, Orientation, ResponseType, Shortcut, ShortcutController,
    ShortcutTrigger, Window,
};

/// Response handler registered with [`DialogWindow::connect_response`].
type ResponseHandler = Rc<dyn Fn(&DialogWindow, ResponseType)>;

/// Shared state behind a [`DialogWindow`] handle.
struct Inner {
    window: Window,
    content: Box,
    actions: Box,
    buttons: RefCell<Vec<(Button, ResponseType)>>,
    handlers: RefCell<Vec<ResponseHandler>>,
}

/// A modal-capable window with a content area and response buttons.
///
/// Cloning gives another handle to the same window.
#[derive(Clone)]
pub struct DialogWindow {
    inner: Rc<Inner>,
}

impl std::fmt::Debug for DialogWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogWindow")
            .field("title", &self.inner.window.title())
            .finish()
    }
}

impl DialogWindow {
    /// Creates a hidden dialog window titled `title`.
    ///
    /// # Arguments
    /// * `title` - Window title.
    /// * `modal` - Whether the window blocks input to its transient parent.
    pub fn new(title: &str, modal: bool) -> Self {
        let window = Window::builder().title(title).modal(modal).build();

        let content = Box::new(Orientation::Vertical, 0);
        content.set_vexpand(true);
        let actions = Box::new(Orientation::Horizontal, 6);
        actions.set_halign(Align::End);
        actions.set_margin_top(6);
        actions.set_margin_bottom(6);
        actions.set_margin_start(6);
        actions.set_margin_end(6);
        actions.set_visible(false);

        let root = Box::new(Orientation::Vertical, 0);
        root.append(&content);
        root.append(&actions);
        window.set_child(Some(&root));

        let dialog = Self {
            inner: Rc::new(Inner {
                window,
                content,
                actions,
                buttons: RefCell::new(Vec::new()),
                handlers: RefCell::new(Vec::new()),
            }),
        };

        // Escape closes the window, as it did for `Dialog`.
        let escape = ShortcutController::new();
        escape.add_shortcut(Shortcut::new(
            ShortcutTrigger::parse_string("Escape"),
            Some(CallbackAction::new(|widget, _| {
                if let Some(window) = widget.downcast_ref::<Window>() {
                    window.close();
                }
                glib::Propagation::Stop
            })),
        ));
        dialog.inner.window.add_controller(escape);

        // Closing reports DeleteEvent, then lets the window close normally, unless a
        // handler hid the window to reuse it later, in which case it is kept. A weak
        // reference avoids a cycle between the window and its own handler.
        let weak = Rc::downgrade(&dialog.inner);
        dialog.inner.window.connect_close_request(move |window| {
            if let Some(inner) = weak.upgrade() {
                DialogWindow { inner }.response(ResponseType::DeleteEvent);
            }
            if window.is_visible() {
                glib::Propagation::Proceed
            } else {
                glib::Propagation::Stop
            }
        });

        dialog
    }

    /// Returns the underlying window (for sizing, parenting, presenting).
    pub fn window(&self) -> &Window {
        &self.inner.window
    }

    /// Returns the box that holds the dialog's content, above the buttons.
    pub fn content_area(&self) -> &Box {
        &self.inner.content
    }

    /// Adds a button to the action row that reports `response` when clicked.
    ///
    /// # Returns
    /// The new button.
    pub fn add_button(&self, label: &str, response: ResponseType) -> Button {
        let button = Button::with_label(label);
        let weak = Rc::downgrade(&self.inner);
        button.connect_clicked(move |_| {
            if let Some(inner) = weak.upgrade() {
                DialogWindow { inner }.response(response);
            }
        });
        self.inner.actions.append(&button);
        self.inner.actions.set_visible(true);
        self.inner
            .buttons
            .borrow_mut()
            .push((button.clone(), response));
        button
    }

    /// Makes the button for `response` the default (activated by Enter).
    pub fn set_default_response(&self, response: ResponseType) {
        let button = self
            .inner
            .buttons
            .borrow()
            .iter()
            .find(|(_, r)| *r == response)
            .map(|(b, _)| b.clone());
        if let Some(button) = button {
            button.add_css_class("suggested-action");
            self.inner.window.set_default_widget(Some(&button));
        }
    }

    /// Registers `f` to run whenever the dialog reports a response.
    pub fn connect_response<F: Fn(&DialogWindow, ResponseType) + 'static>(&self, f: F) {
        self.inner.handlers.borrow_mut().push(Rc::new(f));
    }

    /// Reports `response` to every handler, as `Dialog::response` did.
    pub fn response(&self, response: ResponseType) {
        // Clone the list so a handler may register another handler or close the
        // window without re-borrowing.
        let handlers: Vec<ResponseHandler> = self.inner.handlers.borrow().clone();
        for handler in handlers {
            handler(self, response);
        }
    }

    /// Sets the window that this dialog stays above.
    pub fn set_transient_for(&self, parent: Option<&impl IsA<Window>>) {
        self.inner.window.set_transient_for(parent);
    }

    /// Sets the initial window size.
    pub fn set_default_size(&self, width: i32, height: i32) {
        self.inner.window.set_default_size(width, height);
    }

    /// Shows the window and brings it to the front.
    pub fn present(&self) {
        self.inner.window.present();
    }

    /// Hides the window without destroying it, so it can be presented again.
    pub fn hide(&self) {
        self.inner.window.set_visible(false);
    }

    /// Closes the window (reports `DeleteEvent` first, like the title-bar button).
    pub fn close(&self) {
        self.inner.window.close();
    }
}
