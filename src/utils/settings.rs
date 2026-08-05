use std::fs;
use std::path::PathBuf;

/// User-configurable app behavior, persisted across launches. New
/// `TextEditor`s read this at construction time — changing a setting takes
/// effect for tabs opened afterward, not ones already open.
#[derive(Clone, Copy)]
pub struct Settings {
    pub show_syllable_gutter: bool,
    pub rhyme_highlighting: bool,
    pub word_completion: bool,
    pub auto_indent: bool,
    pub tab_width: u32,
    pub git_autostage: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_syllable_gutter: true,
            rhyme_highlighting: true,
            // Off by default: dictionary-only completion misses most
            // songwriting vocabulary (slang, informal spellings), so it's
            // opt-in rather than on by default.
            word_completion: false,
            auto_indent: true,
            tab_width: 4,
            git_autostage: true,
        }
    }
}

fn settings_file() -> Option<PathBuf> {
    let mut dir = dirs::config_dir()?;
    dir.push("rhymr");
    fs::create_dir_all(&dir).ok()?;
    dir.push("settings.txt");
    Some(dir)
}

impl Settings {
    pub fn load() -> Self {
        let mut settings = Settings::default();
        let Some(path) = settings_file() else {
            return settings;
        };
        let Ok(contents) = fs::read_to_string(&path) else {
            return settings;
        };

        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "show_syllable_gutter" => settings.show_syllable_gutter = value == "true",
                "rhyme_highlighting" => settings.rhyme_highlighting = value == "true",
                "word_completion" => settings.word_completion = value == "true",
                "auto_indent" => settings.auto_indent = value == "true",
                "tab_width" => settings.tab_width = value.parse().unwrap_or(settings.tab_width),
                "git_autostage" => settings.git_autostage = value == "true",
                _ => {}
            }
        }

        settings
    }

    pub fn save(&self) {
        let Some(path) = settings_file() else {
            return;
        };
        let contents = format!(
            "show_syllable_gutter={}\nrhyme_highlighting={}\nword_completion={}\nauto_indent={}\ntab_width={}\ngit_autostage={}\n",
            self.show_syllable_gutter,
            self.rhyme_highlighting,
            self.word_completion,
            self.auto_indent,
            self.tab_width,
            self.git_autostage,
        );
        let _ = fs::write(path, contents);
    }
}
