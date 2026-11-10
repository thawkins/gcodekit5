//! # Text drop-down
//!
//! Replacement for the deprecated `gtk4::ComboBoxText` (deprecated since GTK 4.10).
//!
//! [`new_text_dropdown`] builds a [`gtk4::DropDown`] backed by a [`gio::ListStore`] of
//! `(id, text)` entries, and [`TextDropDownExt`] gives it the subset of the
//! `ComboBoxText` API this crate uses (`append`, `active_id`, `set_active_id`, ...), so
//! call sites keep their existing shape.
//!
//! Behavioural difference from `ComboBoxText`: a `DropDown` auto-selects the first
//! item once its model is non-empty, so `active()` is `Some(0)` after the first
//! `append` rather than `None`.
//!
//! Dependencies: `gtk4`, `gio`, `glib` (`BoxedAnyObject` holds each entry).

use gio::prelude::*;
use glib::{BoxedAnyObject, GString, SignalHandlerId};
use gtk4::prelude::*;
use gtk4::{DropDown, INVALID_LIST_POSITION, Label, ListItem, SignalListItemFactory};

/// One drop-down entry: an optional stable id and the displayed text.
#[derive(Debug, Clone)]
struct Entry {
    id: Option<String>,
    text: String,
}

/// Creates an empty text drop-down, the replacement for `ComboBoxText::new()`.
///
/// # Returns
/// A `DropDown` whose model is a `gio::ListStore` of text entries; populate it with
/// [`TextDropDownExt::append`] or [`TextDropDownExt::append_text`].
pub fn new_text_dropdown() -> DropDown {
    let store = gio::ListStore::new::<BoxedAnyObject>();

    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        if let Some(item) = item.downcast_ref::<ListItem>() {
            let label = Label::new(None);
            label.set_xalign(0.0);
            item.set_child(Some(&label));
        }
    });
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<ListItem>() else {
            return;
        };
        let text = item
            .item()
            .and_downcast::<BoxedAnyObject>()
            .map(|obj| obj.borrow::<Entry>().text.clone());
        if let (Some(text), Some(label)) = (text, item.child().and_downcast::<Label>()) {
            label.set_text(&text);
        }
    });

    DropDown::builder().model(&store).factory(&factory).build()
}

/// `ComboBoxText`-style accessors for a drop-down made by [`new_text_dropdown`].
///
/// Positions and return types match the `ComboBoxText` methods they replace. On a
/// `DropDown` with any other model the getters return `None` and the mutators do nothing.
pub trait TextDropDownExt {
    /// Appends an entry with an optional id.
    fn append(&self, id: Option<&str>, text: &str);
    /// Appends an entry with no id.
    fn append_text(&self, text: &str);
    /// Removes every entry.
    fn remove_all(&self);
    /// Returns the selected position, or `None` when nothing is selected.
    fn active(&self) -> Option<u32>;
    /// Selects the entry at `index`, or clears the selection with `None`.
    fn set_active(&self, index: Option<u32>);
    /// Returns the id of the selected entry.
    fn active_id(&self) -> Option<GString>;
    /// Selects the first entry whose id is `id` (`None` clears the selection).
    ///
    /// # Returns
    /// `true` if an entry matched (or the selection was cleared).
    fn set_active_id(&self, id: Option<&str>) -> bool;
    /// Returns the text of the selected entry.
    fn active_text(&self) -> Option<GString>;
    /// Connects `f` to selection changes (the `ComboBox::changed` equivalent).
    fn connect_changed<F: Fn(&DropDown) + 'static>(&self, f: F) -> SignalHandlerId;
}

/// Returns the drop-down's entry store, if it was made by [`new_text_dropdown`].
fn store(dropdown: &DropDown) -> Option<gio::ListStore> {
    dropdown.model().and_downcast::<gio::ListStore>()
}

/// Returns a copy of the entry at `position`.
fn entry_at(dropdown: &DropDown, position: u32) -> Option<Entry> {
    store(dropdown)?
        .item(position)
        .and_downcast::<BoxedAnyObject>()
        .map(|obj| obj.borrow::<Entry>().clone())
}

impl TextDropDownExt for DropDown {
    fn append(&self, id: Option<&str>, text: &str) {
        if let Some(store) = store(self) {
            store.append(&BoxedAnyObject::new(Entry {
                id: id.map(str::to_owned),
                text: text.to_owned(),
            }));
        }
    }

    fn append_text(&self, text: &str) {
        self.append(None, text);
    }

    fn remove_all(&self) {
        if let Some(store) = store(self) {
            store.remove_all();
        }
    }

    fn active(&self) -> Option<u32> {
        let selected = self.selected();
        (selected != INVALID_LIST_POSITION).then_some(selected)
    }

    fn set_active(&self, index: Option<u32>) {
        self.set_selected(index.unwrap_or(INVALID_LIST_POSITION));
    }

    fn active_id(&self) -> Option<GString> {
        entry_at(self, self.active()?)?.id.map(GString::from)
    }

    fn set_active_id(&self, id: Option<&str>) -> bool {
        let Some(id) = id else {
            self.set_selected(INVALID_LIST_POSITION);
            return true;
        };
        let Some(store) = store(self) else {
            return false;
        };
        let found = (0..store.n_items())
            .find(|&i| entry_at(self, i).is_some_and(|entry| entry.id.as_deref() == Some(id)));
        match found {
            Some(position) => {
                self.set_selected(position);
                true
            }
            None => false,
        }
    }

    fn active_text(&self) -> Option<GString> {
        entry_at(self, self.active()?).map(|entry| GString::from(entry.text))
    }

    fn connect_changed<F: Fn(&DropDown) + 'static>(&self, f: F) -> SignalHandlerId {
        self.connect_selected_notify(f)
    }
}
