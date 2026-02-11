//! Cross-desktop file dialog helpers.
//!
//! `FileChooserNative` relies on XDG Desktop Portal which is unreliable on
//! KDE/Kubuntu (and some Wayland compositors). This module provides thin
//! wrappers that use `FileChooserDialog` instead, matching the pattern already
//! used by the CAM-tools panels.

use gtk4::prelude::*;
use gtk4::{
    ButtonsType, FileChooserAction, FileChooserDialog, MessageDialog, MessageType, ResponseType,
    Widget,
};

/// Create a `FileChooserDialog` for **opening** a file.
///
/// The dialog is modal, uses the GTK built-in file chooser (works on all
/// desktops including KDE), and is sized to 900×700.
pub fn open_dialog(title: &str, parent: Option<&impl IsA<gtk4::Window>>) -> FileChooserDialog {
    let dlg = FileChooserDialog::new(
        Some(title),
        parent,
        FileChooserAction::Open,
        &[
            ("Cancel", ResponseType::Cancel),
            ("Open", ResponseType::Accept),
        ],
    );
    dlg.set_default_size(900, 700);
    dlg.set_modal(true);
    dlg
}

/// Create a `FileChooserDialog` for **saving** a file.
pub fn save_dialog(title: &str, parent: Option<&impl IsA<gtk4::Window>>) -> FileChooserDialog {
    let dlg = FileChooserDialog::new(
        Some(title),
        parent,
        FileChooserAction::Save,
        &[
            ("Cancel", ResponseType::Cancel),
            ("Save", ResponseType::Accept),
        ],
    );
    dlg.set_default_size(900, 700);
    dlg.set_modal(true);
    dlg
}

/// Create a `FileChooserDialog` for **selecting a folder**.
pub fn folder_dialog(title: &str, parent: Option<&impl IsA<gtk4::Window>>) -> FileChooserDialog {
    let dlg = FileChooserDialog::new(
        Some(title),
        parent,
        FileChooserAction::SelectFolder,
        &[
            ("Cancel", ResponseType::Cancel),
            ("Select", ResponseType::Accept),
        ],
    );
    dlg.set_default_size(900, 700);
    dlg.set_modal(true);
    dlg
}

/// Try to obtain the parent `gtk4::Window` from any widget.
pub fn parent_window(widget: &impl IsA<Widget>) -> Option<gtk4::Window> {
    widget
        .root()
        .and_then(|r| r.downcast::<gtk4::Window>().ok())
}

/// Show a modal error dialog with a title and detail message.
///
/// If a parent window is provided the dialog is set as transient and modal.
/// The dialog auto-destroys when the user clicks OK.
pub fn show_error_dialog(title: &str, message: &str, parent: Option<&gtk4::Window>) {
    let mut builder = MessageDialog::builder()
        .message_type(MessageType::Error)
        .buttons(ButtonsType::Ok)
        .text(title)
        .secondary_text(message);

    if let Some(win) = parent {
        builder = builder.transient_for(win).modal(true);
    }

    let dialog = builder.build();
    dialog.connect_response(|d, _| d.destroy());
    dialog.show();
}
