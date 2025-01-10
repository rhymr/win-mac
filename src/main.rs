use gio::Resource;
use gtk::prelude::*;
use gtk::{gio, glib};
use rhymr_rs::layout;

pub const APP_ID: &str = "Rhymr";

fn main() -> glib::ExitCode {
    // Register the resource bundle from the compiled resource file
    let resource_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/compiled.gresource"));
    let resource_data = glib::Bytes::from(&resource_bytes[..]);
    gio::resources_register(
        &Resource::from_data(&resource_data)
            .expect("Failed to load resources")
    );

    let app = gtk::Application::builder()
        .application_id("org.gtk_rs.Rhymr")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    // Connect activate signal
    app.connect_activate(|app| {
        let (_window, _controller) = layout::build_ui(app);
    });

    app.run()
}