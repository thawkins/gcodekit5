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
    let detail = match &config.secondary_text {
        Some(secondary) => format!("{}\n\n{}", config.message, secondary),
        None => config.message.clone(),
    };

    // Buttons in display order, with the response each one reports. The first
    // entry is what Escape/close maps to when the dialog has a cancel choice.
    let buttons: Vec<(String, ResponseType)> = match config.buttons {
        ButtonsType::None => Vec::new(),
        ButtonsType::Close => vec![(t!("Close"), ResponseType::Close)],
        ButtonsType::Cancel => vec![(t!("Cancel"), ResponseType::Cancel)],
        ButtonsType::YesNo => vec![(t!("No"), ResponseType::No), (t!("Yes"), ResponseType::Yes)],
        ButtonsType::OkCancel => vec![
            (t!("Cancel"), ResponseType::Cancel),
            (t!("OK"), ResponseType::Ok),
        ],
        // `Ok` and any future variant of this non-exhaustive GTK enum.
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
    dialog.choose(parent, gtk4::gio::Cancellable::NONE, move |result| {
        // Escape or closing the window reports an error, which MessageDialog
        // reported as `DeleteEvent`.
        let response = result
            .ok()
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| responses.get(index).copied())
            .unwrap_or(ResponseType::DeleteEvent);
        if let Some(cb) = callback {
            cb(response);
        }
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

/// The user's answer to an unsaved-changes prompt
/// (see [`show_unsaved_changes`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedChangesChoice {
    /// Save the work, then carry on with the pending action.
    Save,
    /// Carry on with the pending action without saving.
    Discard,
    /// Abandon the pending action and keep the work as it is.
    Cancel,
}

impl UnsavedChangesChoice {
    /// Button order of [`show_unsaved_changes`], left to right.
    pub const BUTTONS: [Self; 3] = [Self::Cancel, Self::Discard, Self::Save];

    /// Choice for the index of the button pressed in [`show_unsaved_changes`].
    ///
    /// # Arguments
    /// * `index` - The pressed button's index, or `None` when the dialog was
    ///   dismissed with Escape or by closing it.
    ///
    /// # Returns
    /// The matching choice; dismissing the dialog or an unknown index is
    /// [`UnsavedChangesChoice::Cancel`], so nothing is lost by accident.
    pub fn from_button_index(index: Option<i32>) -> Self {
        index
            .and_then(|i| usize::try_from(i).ok())
            .and_then(|i| Self::BUTTONS.get(i).copied())
            .unwrap_or(Self::Cancel)
    }

    /// Translated button label.
    fn label(self) -> String {
        match self {
            Self::Save => t!("Save"),
            Self::Discard => t!("Discard"),
            Self::Cancel => t!("Cancel"),
        }
    }
}

/// Ask what to do with unsaved work before an action that would lose it
/// (quitting, starting a new document, opening another one).
///
/// Shows Cancel, Discard and Save buttons; Save is the default and Escape or
/// closing the dialog counts as Cancel.
///
/// # Arguments
/// * `title` - Dialog heading.
/// * `message` - Explanation of what is unsaved.
/// * `parent` - Optional parent window.
/// * `callback` - Receives the user's choice. The caller saves for
///   [`UnsavedChangesChoice::Save`] and runs the pending action after a
///   successful save or for [`UnsavedChangesChoice::Discard`].
pub fn show_unsaved_changes(
    title: &str,
    message: &str,
    parent: Option<&Window>,
    callback: impl FnOnce(UnsavedChangesChoice) + 'static,
) {
    let labels: Vec<String> = UnsavedChangesChoice::BUTTONS
        .iter()
        .map(|choice| choice.label())
        .collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let dialog = AlertDialog::builder()
        .message(title)
        .detail(message)
        .modal(true)
        .build();
    dialog.set_buttons(&labels);
    dialog.set_cancel_button(0);
    dialog.set_default_button(labels.len() as i32 - 1);
    dialog.choose(parent, gtk4::gio::Cancellable::NONE, move |result| {
        callback(UnsavedChangesChoice::from_button_index(result.ok()));
    });
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
