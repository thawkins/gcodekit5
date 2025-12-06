use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use libadwaita::prelude::*;
use libadwaita::Application as AdwApplication;

pub fn main() {
    let app = AdwApplication::builder()
        .application_id("com.github.thawkins.gcodekit5")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("GCodeKit5")
            .default_width(1200)
            .default_height(800)
            .build();

        window.present();
    });

    app.run();
}
