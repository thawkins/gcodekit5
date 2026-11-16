//! # Standardized Dialog Creation
//!
//! Provides consistent dialog creation patterns across the UI.
//! Built on `gtk4::AlertDialog` (GTK 4.10+), which replaces the deprecated
//! `MessageDialog`. `MessageType` is kept in [`DialogConfig`] for API stability, but
//! `AlertDialog` shows no type icon, so it does not change what is displayed.

use crate::t;
use gtk4::prelude::*;
use gtk4::{AlertDialog, ButtonsType, MessageType, ResponseType, Widget, Window};

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

// ---
/// Result for the three-button unsaved changes dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedChangesResponse {
    Accept,
    Discard,
    Cancel,
}

/// Show a three-button dialog for unsaved changes: Cancel, Discard, Accept
pub fn show_unsaved_changes_dialog(
    title: &str,
    message: &str,
    parent: Option<&Window>,
    callback: impl FnOnce(UnsavedChangesResponse) + 'static,
) {
    if DIALOG_OPEN.swap(true, Ordering::SeqCst) {
        return;
    }

    let buttons = [
        (t!("Cancel"), UnsavedChangesResponse::Cancel),
        (t!("Discard"), UnsavedChangesResponse::Discard),
        (t!("Accept"), UnsavedChangesResponse::Accept),
    ];

    let labels: Vec<&str> = buttons.iter().map(|(label, _)| label.as_str()).collect();
    let dialog = AlertDialog::builder()
        .message(title)
        .detail(message)
        .modal(true)
        .build();

    dialog.set_buttons(&labels);
    dialog.set_cancel_button(0);
    dialog.set_default_button(2);

    let responses: Vec<UnsavedChangesResponse> = buttons.into_iter().map(|(_, r)| r).collect();
    let parent_owned = parent.cloned();

    gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
        dialog.choose(
            parent_owned.as_ref(),
            gtk4::gio::Cancellable::NONE,
            move |result| {
                DIALOG_OPEN.store(false, Ordering::SeqCst);

                let response = result
                    .ok()
                    .and_then(|index| usize::try_from(index).ok())
                    .and_then(|index| responses.get(index).copied())
                    .unwrap_or(UnsavedChangesResponse::Cancel);

                callback(response);
            },
        );
    });
}

// ---

/// Show a standardized message dialog
///
/// # Arguments
/// * `config` - Dialog configuration
/// * `parent` - Optional parent window
/// * `callback` - Optional callback to handle response
use std::sync::atomic::{AtomicBool, Ordering};

static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

pub fn show_message_dialog(
    config: DialogConfig,
    parent: Option<&Window>,
    callback: Option<Box<dyn FnOnce(ResponseType)>>,
) {
    // Evita que se apilen dos diálogos idénticos uno encima del otro
    if DIALOG_OPEN.swap(true, Ordering::SeqCst) {
        return;
    }

    let detail = match &config.secondary_text {
        Some(secondary) => format!("{}\n\n{}", config.message, secondary),
        None => config.message.clone(),
    };

    let buttons: Vec<(String, ResponseType)> = match config.buttons {
        ButtonsType::None => Vec::new(),
        ButtonsType::Close => vec![(t!("Close"), ResponseType::Close)],
        ButtonsType::Cancel => vec![(t!("Cancel"), ResponseType::Cancel)],
        ButtonsType::YesNo => vec![(t!("No"), ResponseType::No), (t!("Yes"), ResponseType::Yes)],
        ButtonsType::OkCancel => vec![
            (t!("Cancel"), ResponseType::Cancel),
            (t!("OK"), ResponseType::Ok),
        ],
        _ => vec![(t!("OK"), ResponseType::Ok)],
    };

    let labels: Vec<&str> = buttons.iter().map(|(label, _)| label.as_str()).collect();
    let dialog = AlertDialog::builder()
        .message(config.title.as_str())
        .detail(detail.as_str())
        .modal(config.modal)
        .build();
    if !labels.is_empty() {
        dialog.set_buttons(&labels);
        let last = labels.len() as i32 - 1;
        dialog.set_default_button(last);
        if labels.len() > 1 {
            dialog.set_cancel_button(0);
        }
    }

    let responses: Vec<ResponseType> = buttons.into_iter().map(|(_, r)| r).collect();
    let parent_owned = parent.cloned();

    // Esperar 100 ms para que el compositor (Wayland/X11) libere el foco del DropDown/Popover anterior
    gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
        dialog.choose(
            parent_owned.as_ref(),
            gtk4::gio::Cancellable::NONE,
            move |result| {
                DIALOG_OPEN.store(false, Ordering::SeqCst);

                let response = result
                    .ok()
                    .and_then(|index| usize::try_from(index).ok())
                    .and_then(|index| responses.get(index).copied())
                    .unwrap_or(ResponseType::DeleteEvent);
                if let Some(cb) = callback {
                    cb(response);
                }
            },
        );
    });
}

/// Show an error dialog
pub fn show_error(title: &str, message: &str, parent: Option<&Window>) {
    show_message_dialog(
        DialogConfig::new(title, message).message_type(MessageType::Error),
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
