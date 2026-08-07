use grass::Options;
use gtk::{CssProvider, gdk};
use std::fs;

const CSS_FILES: [&str; 10] = [
    "assets/{1}/base.{1}",
    "assets/{1}/editor.{1}",
    "assets/{1}/empty_state.{1}",
    "assets/{1}/file_tree.{1}",
    "assets/{1}/layout.{1}",
    "assets/{1}/notebook.{1}",
    "assets/{1}/rhyme_search.{1}",
    "assets/{1}/settings.{1}",
    "assets/{1}/status_bar.{1}",
    "assets/{1}/welcome.{1}",
];

pub fn compile_sass() -> Result<(), Box<dyn std::error::Error>> {
    for css_file in CSS_FILES {
        let scss_path = css_file.replace("{1}", "scss");
        let css_path = css_file.replace("{1}", "css");

        println!("Compiling {scss_path}");
        let css_output = grass::from_path(&scss_path, &Options::default())?;
        fs::write(css_path, css_output)?;
    }

    Ok(())
}

pub fn load_css() -> CssProvider {
    let css_provider = CssProvider::new();
    let mut combined_css = String::new();

    for css_file in CSS_FILES {
        let scss_path = css_file.replace("{1}", "scss");
        match grass::from_path(&scss_path, &Options::default()) {
            Ok(css) => {
                combined_css.push_str(&css);
                combined_css.push('\n');
            }
            Err(err) => eprintln!("Failed to compile {scss_path}: {err}"),
        }
    }

    // `load_from_string` replaces the deprecated `load_from_data`
    css_provider.load_from_string(&combined_css);
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
