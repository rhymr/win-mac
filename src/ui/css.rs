use gtk::{CssProvider, gdk};
use gtk::glib::Bytes;

pub fn load_css(file_path: &str) -> CssProvider {
    let css_provider = CssProvider::new();

    // Read the file as raw bytes, NOT as a string
    if let Ok(data) = std::fs::read(file_path) {
        css_provider
            .load_from_bytes(&Bytes::from_owned(data));
    } else {
        eprintln!("CSS file not found: {}", file_path);
    }

    css_provider
}

pub fn apply_css_to_app(css_provider: &CssProvider) {
    if let Some(default_display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &default_display,
            css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}