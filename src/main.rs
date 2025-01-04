use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Button};

const APP_ID: &str = "Rhymr";

fn main() -> glib::ExitCode {
    // Construct GTK_ID dynamically at runtime
    let gtk_id = format!("org.gtk_rs.{}", APP_ID.to_lowercase());

    // Create a new application
    let app = Application::builder().application_id(&gtk_id).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

fn build_ui(app: &Application) {
    // Create a button with label and margins
    let button = Button::builder()
        .label("Press me!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Connect to "clicked" signal of `button`
    button.connect_clicked(|button| {
        // Set the label to "Hello World!" after the button has been clicked on
        button.set_label("Hello World!");
        println!("Hello World!");
    });


    // Create a window and set the title
    let window = ApplicationWindow::builder()
        .application(app)
        .title(APP_ID)
        .child(&button)
        .build();

    // Present window
    window.present();
}