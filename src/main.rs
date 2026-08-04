use gio::Resource;
use gtk::prelude::*;
use gtk::{gio, glib};
use rhymr_rs::{css, ui::layout};

pub const APP_ID: &str = "org.gtk_rs.Rhymr";

fn main() -> glib::ExitCode {
    // Register the resource bundle from the compiled resource file
    let resource_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/compiled.gresource"));
    let resource_data = glib::Bytes::from(&resource_bytes[..]);
    gio::resources_register(
        &Resource::from_data(&resource_data)
            .expect("Failed to load resources")
    );

    println!(
        "Current dir = {:?}",
        std::env::current_dir().unwrap()
    );

    // Compile scss files into css files
    if let Err(e) = css::compile_sass() {
        eprintln!("compile_sass failed: {e}");
        panic!("{e}");
    }

    // Build the application
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    // Connect activate signal
    app.connect_activate(|app| {
        let app_for_workspace = app.clone();

        // Show the workspace picker first; the main editor layout is only
        // built once a workspace has actually been chosen.
        rhymr_rs::ui::welcome::show_welcome_dialog(app, move |workspace_path| {
            println!("Loaded workspace at: {:?}", workspace_path);
            let (_window, controller) = layout::build_ui(&app_for_workspace);
            controller.set_root_path(workspace_path);
        });
    });

    // Run application!
    app.run()
}