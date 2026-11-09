//! Cross-desktop file dialog helpers built on `gtk4::FileDialog`.
//!
//! `FileChooserDialog` is deprecated since GTK 4.10. `FileDialog` replaces it and,
//! when an XDG Desktop Portal is available, shows the desktop's native picker.
//! [`FileDialogHelpers`] keeps the old call-site shape: filters are added one at a
//! time, and the result arrives in a callback that runs only when the user picks a
//! file (cancel and dismiss are silent; other errors are logged).
//!
//! Dependencies: `gtk4` (feature `v4_10`+), `gio`, `gcodekit5_settings`.

use gcodekit5_settings::controller::SettingsController;
use gtk4::prelude::*;
use gtk4::{FileDialog, FileFilter, Widget, Window};

/// Creates a modal `FileDialog` with `title`, the replacement for
/// `FileChooserDialog::new`.
pub fn new_file_dialog(title: &str) -> FileDialog {
    FileDialog::builder().title(title).modal(true).build()
}

/// Creates a dialog for **opening** a file; show it with [`FileDialogHelpers::pick_open`].
pub fn open_dialog(title: &str) -> FileDialog {
    new_file_dialog(title)
}

/// Creates a dialog for **saving** a file; show it with [`FileDialogHelpers::pick_save`].
pub fn save_dialog(title: &str) -> FileDialog {
    new_file_dialog(title)
}

/// Creates a dialog for **selecting a folder**; show it with
/// [`FileDialogHelpers::pick_folder`].
pub fn folder_dialog(title: &str) -> FileDialog {
    new_file_dialog(title)
}

/// Starts `dialog` in the persisted working directory.
///
/// # Arguments
/// * `dialog` - The dialog to configure.
/// * `settings_controller` - Source of the last output directory; `None` leaves the
///   dialog's default folder.
pub fn set_last_working_directory(
    dialog: &FileDialog,
    settings_controller: Option<&SettingsController>,
) {
    let Some(controller) = settings_controller else {
        return;
    };

    let settings_ref = controller.persistence.borrow();
    let last_path = &settings_ref.config().file_processing.output_directory;
    let folder_path = if last_path.is_file() {
        last_path.parent().unwrap_or(last_path).to_path_buf()
    } else {
        last_path.clone()
    };

    dialog.set_initial_folder(Some(&gtk4::gio::File::for_path(folder_path)));
}

/// Hands a successful pick to `on_file`; logs real failures, ignores cancel/dismiss.
fn deliver(result: Result<gtk4::gio::File, glib::Error>, on_file: impl FnOnce(gtk4::gio::File)) {
    match result {
        Ok(file) => on_file(file),
        Err(err)
            if err.matches(gtk4::DialogError::Dismissed)
                || err.matches(gtk4::DialogError::Cancelled) => {}
        Err(err) => tracing::warn!("File dialog failed: {err}"),
    }
}

/// `FileChooserDialog`-style conveniences for `gtk4::FileDialog`.
pub trait FileDialogHelpers {
    /// Adds `filter` to the dialog's filter list; the first filter added is the default.
    fn add_filter(&self, filter: &FileFilter);
    /// Shows an Open dialog and calls `on_file` with the chosen file.
    fn pick_open(
        &self,
        parent: Option<&impl IsA<Window>>,
        on_file: impl FnOnce(gtk4::gio::File) + 'static,
    );
    /// Shows a Save dialog and calls `on_file` with the chosen destination.
    fn pick_save(
        &self,
        parent: Option<&impl IsA<Window>>,
        on_file: impl FnOnce(gtk4::gio::File) + 'static,
    );
    /// Shows a folder picker and calls `on_file` with the chosen folder.
    fn pick_folder(
        &self,
        parent: Option<&impl IsA<Window>>,
        on_file: impl FnOnce(gtk4::gio::File) + 'static,
    );
}

impl FileDialogHelpers for FileDialog {
    fn add_filter(&self, filter: &FileFilter) {
        let store = match self.filters() {
            Some(model) => model.downcast::<gtk4::gio::ListStore>().ok(),
            None => None,
        }
        .unwrap_or_else(|| {
            let store = gtk4::gio::ListStore::new::<FileFilter>();
            self.set_filters(Some(&store));
            store
        });
        store.append(filter);
        if self.default_filter().is_none() {
            self.set_default_filter(Some(filter));
        }
    }

    fn pick_open(
        &self,
        parent: Option<&impl IsA<Window>>,
        on_file: impl FnOnce(gtk4::gio::File) + 'static,
    ) {
        self.open(parent, gtk4::gio::Cancellable::NONE, move |r| {
            deliver(r, on_file)
        });
    }

    fn pick_save(
        &self,
        parent: Option<&impl IsA<Window>>,
        on_file: impl FnOnce(gtk4::gio::File) + 'static,
    ) {
        self.save(parent, gtk4::gio::Cancellable::NONE, move |r| {
            deliver(r, on_file)
        });
    }

    fn pick_folder(
        &self,
        parent: Option<&impl IsA<Window>>,
        on_file: impl FnOnce(gtk4::gio::File) + 'static,
    ) {
        FileDialog::select_folder(self, parent, gtk4::gio::Cancellable::NONE, move |r| {
            deliver(r, on_file)
        });
    }
}

/// Try to obtain the parent `gtk4::Window` from any widget.
pub fn parent_window(widget: &impl IsA<Widget>) -> Option<gtk4::Window> {
    widget
        .root()
        .and_then(|r| r.downcast::<gtk4::Window>().ok())
}

/// Show a modal error dialog with a title and detail message.
///
/// If a parent window is provided the dialog is modal to it.
pub fn show_error_dialog(title: &str, message: &str, parent: Option<&gtk4::Window>) {
    crate::ui::gtk::common::dialog::show_error(title, message, parent);
}
