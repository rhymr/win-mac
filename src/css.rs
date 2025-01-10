use grass::from_path;
use gtk::{gdk, CssProvider};
use std::fs;

// TODO Load Dynamically
const CSS_FILES: [&str; 8] = [
    "assets/{1}/base.{1}",
    "assets/{1}/editor.{1}",
    "assets/{1}/empty_state.{1}",
    "assets/{1}/file_tree.{1}",
    "assets/{1}/layout.{1}",
    "assets/{1}/notebook.{1}",
    "assets/{1}/rhyme_search.{1}",
    "assets/{1}/status_bar.{1}",
];

pub fn compile_sass() -> Result<(), Box<dyn std::error::Error>> {
    for css_file in CSS_FILES {
        let scss_path = css_file.replace("{1}", "scss");
        let css_output = from_path(scss_path.clone(), &Default::default())?;
        fs::write(css_file.replace("{1}", "css"), css_output)?;
    }

    Ok(())
}

pub fn load_css() -> CssProvider {
    let css_provider = CssProvider::new();

    // Read and combine all CSS files
    let mut combined_css = String::new();
    for css_file in CSS_FILES {
        if let Ok(css_content) = fs::read_to_string(css_file.replace("{1}", "css")) {
            combined_css.push_str(&css_content);
            combined_css.push('\n');
        }
    }

    css_provider.load_from_data(&combined_css);
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