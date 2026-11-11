//! Tests for `DialogWindow`, the `gtk4::Dialog` replacement: response handlers
//! must keep working after the caller drops its handle, `close()` from a response
//! handler must not report a second response, hidden dialogs must survive the
//! title-bar close, and a destroyed dialog must release its handlers.
//!
//! These need a display. Without one (`gtk4::init()` fails) the test returns early.
//! GTK must be driven from one thread, so every case runs inside a single test.

use std::cell::RefCell;
use std::rc::Rc;

use gcodekit5_ui::ui::gtk::common::dialog_window::DialogWindow;
use gtk4::ResponseType;
use gtk4::prelude::*;

/// Runs pending main-loop work so signals and redraws are processed.
fn drain_main_loop() {
    let context = glib::MainContext::default();
    while context.iteration(false) {}
}

#[test]
fn test_dialog_window_lifecycle() {
    if gtk4::init().is_err() {
        eprintln!("skipping test_dialog_window_lifecycle: no display available");
        return;
    }

    // Buttons keep working after the last handle is dropped (Preferences pattern).
    let responses: Rc<RefCell<Vec<ResponseType>>> = Rc::new(RefCell::new(Vec::new()));
    let ok_button = {
        let dialog = DialogWindow::new("dropped handle", false);
        let button = dialog.add_button("OK", ResponseType::Ok);
        let seen = responses.clone();
        dialog.connect_response(move |_, response| seen.borrow_mut().push(response));
        dialog.present();
        button
    };
    drain_main_loop();
    ok_button.emit_clicked();
    assert_eq!(*responses.borrow(), vec![ResponseType::Ok]);

    // close() inside a response handler destroys the window and reports nothing
    // further: exactly one response, no recursion (Tool/Material picker pattern).
    let responses: Rc<RefCell<Vec<ResponseType>>> = Rc::new(RefCell::new(Vec::new()));
    let dialog = DialogWindow::new("close in handler", false);
    let select = dialog.add_button("Select", ResponseType::Accept);
    let seen = responses.clone();
    dialog.connect_response(move |d, response| {
        seen.borrow_mut().push(response);
        d.close();
    });
    dialog.present();
    drain_main_loop();
    select.emit_clicked();
    drain_main_loop();
    assert_eq!(*responses.borrow(), vec![ResponseType::Accept]);
    assert!(!dialog.window().is_visible());

    // Title-bar close reports DeleteEvent once, even if the handler calls close().
    let responses: Rc<RefCell<Vec<ResponseType>>> = Rc::new(RefCell::new(Vec::new()));
    let dialog = DialogWindow::new("title-bar close", false);
    dialog.add_button("Cancel", ResponseType::Cancel);
    let seen = responses.clone();
    dialog.connect_response(move |d, response| {
        seen.borrow_mut().push(response);
        d.close();
    });
    dialog.present();
    drain_main_loop();
    dialog.window().close();
    drain_main_loop();
    assert_eq!(*responses.borrow(), vec![ResponseType::DeleteEvent]);

    // A dialog that hides itself survives the title-bar close and can be reused.
    let responses: Rc<RefCell<Vec<ResponseType>>> = Rc::new(RefCell::new(Vec::new()));
    let dialog = DialogWindow::new("reusable", false);
    let close_button = dialog.add_button("Close", ResponseType::Close);
    let seen = responses.clone();
    dialog.connect_response(move |d, response| {
        seen.borrow_mut().push(response);
        d.hide();
    });
    dialog.present();
    drain_main_loop();
    dialog.window().close();
    drain_main_loop();
    assert!(
        dialog.window().is_realized(),
        "hidden dialog must not be destroyed"
    );
    dialog.present();
    drain_main_loop();
    close_button.emit_clicked();
    assert_eq!(
        *responses.borrow(),
        vec![ResponseType::DeleteEvent, ResponseType::Close]
    );

    // A destroyed dialog releases its response handlers (no leak).
    let token = Rc::new(());
    {
        let dialog = DialogWindow::new("released", false);
        let held = token.clone();
        dialog.connect_response(move |d, _| {
            let _ = &held;
            d.close();
        });
        dialog.present();
        drain_main_loop();
        dialog.response(ResponseType::Ok);
    }
    drain_main_loop();
    assert_eq!(
        Rc::strong_count(&token),
        1,
        "handlers of a destroyed dialog leaked"
    );
}
