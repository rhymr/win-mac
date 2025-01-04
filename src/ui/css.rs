use gtk::{CssProvider, gdk};

pub fn load_css(file_path: &str) -> CssProvider {
    let css_provider = CssProvider::new();

    if let Ok(data) = std::fs::read_to_string(file_path) {
        css_provider.load_from_data(&data);
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