use gtk::prelude::*;
use gtk::glib;

const APP_ID: &str = "Rhymr";

mod ui;

fn main() -> glib::ExitCode {
    let gtk_id = format!("org.gtk_rs.{}", APP_ID.to_lowercase());
    let app = gtk::Application::builder()
        .application_id(&gtk_id)
        .flags(gtk::gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(ui::layout::build_ui);

    app.run()
}