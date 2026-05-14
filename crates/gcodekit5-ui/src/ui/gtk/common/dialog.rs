//! # Standardized Dialog Creation
//!
//! Provides consistent dialog creation patterns across the UI.
//! Uses MessageDialog as the base (widely supported) with standardized styling.

use gtk4::prelude::*;
use gtk4::{ButtonsType, MessageDialog, MessageType, ResponseType, Widget, Window};

/// Standard dialog configuration
#[derive(Debug, Clone)]
pub struct DialogConfig {
    pub title: String,
    pub message: String,
    pub secondary_text: Option<String>,
    pub message_type: MessageType,
    pub buttons: ButtonsType,
    pub modal: bool,
}

impl DialogConfig {
    /// Create a new dialog configuration
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            secondary_text: None,
            message_type: MessageType::Info,
            buttons: ButtonsType::Ok,
            modal: true,
        }
    }

    /// Set secondary text
    pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
        self.secondary_text = Some(text.into());
        self
    }

    /// Set message type
    pub fn message_type(mut self, msg_type: MessageType) -> Self {
        self.message_type = msg_type;
        self
    }

    /// Set buttons type
    pub fn buttons(mut self, buttons: ButtonsType) -> Self {
        self.buttons = buttons;
        self
    }

    /// Set modal
    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }
}

/// Show a standardized message dialog
///
/// # Arguments
/// * `config` - Dialog configuration
/// * `parent` - Optional parent window
/// * `callback` - Optional callback to handle response
pub fn show_message_dialog(
    config: DialogConfig,
    parent: Option<&Window>,
    callback: Option<Box<dyn FnOnce(ResponseType)>>,
) {
    let mut builder = MessageDialog::builder()
        .message_type(config.message_type)
        .buttons(config.buttons)
        .text(&config.title)
        .secondary_text(&config.message);

    if let Some(secondary) = &config.secondary_text {
        builder = builder.secondary_text(secondary);
    }

    if let Some(win) = parent {
        builder = builder.transient_for(win).modal(config.modal);
    }

    let dialog = builder.build();

    // Use Cell to allow moving the callback out of the Fn closure
    let callback_cell = std::cell::Cell::new(callback);

    dialog.connect_response(move |d, response| {
        d.destroy();
        if let Some(cb) = callback_cell.take() {
            cb(response);
        }
    });

    dialog.show();
}

/// Show an error dialog
pub fn show_error(title: &str, message: &str, parent: Option<&Window>) {
    show_message_dialog(
        DialogConfig::new(title, message).message_type(MessageType::Error),
        parent,
        None,
    );
}

/// Show an error dialog with secondary text
pub fn show_error_with_details(title: &str, message: &str, details: &str, parent: Option<&Window>) {
    show_message_dialog(
        DialogConfig::new(title, message)
            .message_type(MessageType::Error)
            .secondary_text(details),
        parent,
        None,
    );
}

/// Show a warning dialog
pub fn show_warning(title: &str, message: &str, parent: Option<&Window>) {
    show_message_dialog(
        DialogConfig::new(title, message).message_type(MessageType::Warning),
        parent,
        None,
    );
}

/// Show an info dialog
pub fn show_info(title: &str, message: &str, parent: Option<&Window>) {
    show_message_dialog(
        DialogConfig::new(title, message).message_type(MessageType::Info),
        parent,
        None,
    );
}

/// Show a question dialog with callback
pub fn show_question(
    title: &str,
    message: &str,
    parent: Option<&Window>,
    callback: impl FnOnce(ResponseType) + 'static,
) {
    show_message_dialog(
        DialogConfig::new(title, message)
            .message_type(MessageType::Question)
            .buttons(ButtonsType::YesNo),
        parent,
        Some(Box::new(callback)),
    );
}

/// Try to obtain the parent `Window` from any widget
pub fn parent_window(widget: &impl IsA<Widget>) -> Option<Window> {
    widget.root().and_then(|r| r.downcast::<Window>().ok())
}

/// Show an error dialog from any widget context
///
/// Convenience function that automatically gets the parent window from the widget
pub fn show_error_from_widget(title: &str, message: &str, widget: &impl IsA<Widget>) {
    let parent = parent_window(widget);
    let parent_ref = parent.as_ref();
    show_error(title, message, parent_ref);
}

/// Show a confirmation dialog with callback
///
/// Returns true if the user confirms (Yes/OK), false otherwise
pub fn show_confirmation(
    title: &str,
    message: &str,
    parent: Option<&Window>,
    callback: impl FnOnce(bool) + 'static,
) {
    show_message_dialog(
        DialogConfig::new(title, message)
            .message_type(MessageType::Question)
            .buttons(ButtonsType::OkCancel),
        parent,
        Some(Box::new(move |response| {
            callback(matches!(
                response,
                ResponseType::Ok | ResponseType::Yes | ResponseType::Accept
            ));
        })),
    );
}

/// Dialog result type for async-like usage (synchronous blocks)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogResult {
    Yes,
    No,
    Ok,
    Cancel,
    Other(ResponseType),
}

impl From<ResponseType> for DialogResult {
    fn from(response: ResponseType) -> Self {
        match response {
            ResponseType::Yes => DialogResult::Yes,
            ResponseType::No => DialogResult::No,
            ResponseType::Ok => DialogResult::Ok,
            ResponseType::Cancel => DialogResult::Cancel,
            other => DialogResult::Other(other),
        }
    }
}
