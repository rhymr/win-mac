pub mod completion;
pub mod stat;

use crate::rhyme::highlight::RhymeHighlight;
use crate::setting::Settings;
use completion::WordCompletionProvider;
use gtk::prelude::*;
use gtk::{Frame, ScrolledWindow};
use sourceview5::GutterRendererText;
use sourceview5::prelude::{BufferExt, GutterRendererExt, GutterRendererTextExt, ViewExt};
use sourceview5::{Buffer as SourceBuffer, Completion, Gutter, View as SourceView};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

/// How long to wait after the last keystroke before writing to disk.
const AUTOSAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(600);

pub struct TextEditor {
    frame: Frame,
    source_view: SourceView,
    buffer: SourceBuffer,
    current_path: Rc<RefCell<Option<PathBuf>>>,
    gutter: Gutter,
    syllable_renderer: RefCell<Option<GutterRendererText>>,
    completion: Completion,
    word_provider: RefCell<Option<WordCompletionProvider>>,
    rhyme_highlight: RefCell<Option<RhymeHighlight>>,
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEditor {
    pub fn new() -> Self {
        let settings = crate::setting::Settings::load();

        // Create the source buffer and view
        let buffer = SourceBuffer::new(None);
        let source_view = SourceView::builder()
            .buffer(&buffer)
            .monospace(true)
            .show_line_numbers(true)
            .show_line_marks(true)
            .tab_width(settings.tab_width)
            .auto_indent(settings.auto_indent)
            .indent_width(settings.tab_width as i32)
            .highlight_current_line(true)
            .background_pattern(sourceview5::BackgroundPatternType::None)
            .smart_backspace(true)
            .smart_home_end(sourceview5::SmartHomeEndType::After)
            .hexpand(true)
            .vexpand(true)
            .build();

        // Save `AUTOSAVE_DEBOUNCE` after the last keystroke, so typing
        // doesn't hit the disk on every character. `current_path` is unset
        // until `set_path()` is called (after the tab's initial content is
        // loaded), so the programmatic `set_text()` calls below don't
        // trigger a spurious save.
        let current_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

        // Guards against a save that was already in flight when a newer
        // keystroke reschedules another one — only the latest write should
        // actually land.
        let save_generation = Rc::new(Cell::new(0u64));

        let buffer_clone = buffer.clone();
        let path_ref = current_path.clone();
        let generation_ref = save_generation.clone();
        buffer_clone.connect_changed(move |buf| {
            let Some(path) = path_ref.borrow().clone() else {
                return;
            };
            let text = buf
                .text(&buf.start_iter(), &buf.end_iter(), false)
                .to_string();

            let this_generation = generation_ref.get() + 1;
            generation_ref.set(this_generation);

            let generation_for_timeout = generation_ref.clone();
            glib::timeout_add_local_once(AUTOSAVE_DEBOUNCE, move || {
                // A newer edit came in while this was waiting — let that one win.
                if generation_for_timeout.get() != this_generation {
                    return;
                }
                if let Err(e) = std::fs::write(&path, text) {
                    eprintln!("Auto-save failed for {path:?}: {e}");
                    return;
                }
                if crate::setting::Settings::load().git_autostage
                    && let Some(root) = find_git_root(&path)
                {
                    crate::git::ops::stage_all_changes(&root);
                }
            });
        });

        source_view.set_css_classes(&["rhyme-editor-view"]);

        // Disable bracket matching
        buffer.set_highlight_matching_brackets(false);

        let scheme_manager = sourceview5::StyleSchemeManager::default();
        scheme_manager.append_search_path("assets/styles");
        let style_scheme = scheme_manager
            .scheme("rhymr")
            .expect("Failed to load rhymr style scheme");
        buffer.set_style_scheme(Some(&style_scheme));

        // Configure the gutter
        let gutter = ViewExt::gutter(&source_view, gtk::TextWindowType::Left);
        gutter.set_css_classes(&["rhyme-editor-gutter"]);

        let completion = ViewExt::completion(&source_view);

        // Add custom CSS classes for the editor
        source_view.set_css_classes(&["rhyme-editor"]);

        // Add scrolling support
        let scroll = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&source_view)
            .build();

        let frame = Frame::builder().child(&scroll).build();

        let editor = Self {
            frame,
            source_view,
            buffer,
            current_path,
            gutter,
            syllable_renderer: RefCell::new(None),
            completion,
            word_provider: RefCell::new(None),
            rhyme_highlight: RefCell::new(None),
        };

        // Set initial empty state
        editor.set_text("");
        editor.apply_settings(&settings);

        editor
    }

    /// Toggle the syllable gutter, completion provider, and rhyme
    /// highlighting to match `settings` (adding/removing each live rather
    /// than requiring the tab to be reopened), and update tab
    /// width/auto-indent, which are plain `SourceView` properties.
    pub fn apply_settings(&self, settings: &Settings) {
        self.source_view.set_tab_width(settings.tab_width);
        self.source_view.set_indent_width(settings.tab_width as i32);
        self.source_view.set_auto_indent(settings.auto_indent);

        let mut renderer_slot = self.syllable_renderer.borrow_mut();
        match (renderer_slot.is_some(), settings.show_syllable_gutter) {
            (false, true) => {
                let renderer = create_syllable_renderer(&self.buffer);
                self.gutter.insert(&renderer, -20); // Position right after line numbers (-30)
                *renderer_slot = Some(renderer);
            }
            (true, false) => {
                if let Some(renderer) = renderer_slot.take() {
                    self.gutter.remove(&renderer);
                }
            }
            _ => {}
        }
        drop(renderer_slot);

        let mut provider_slot = self.word_provider.borrow_mut();
        match (provider_slot.is_some(), settings.word_completion) {
            (false, true) => {
                let provider = WordCompletionProvider::new();
                self.completion.add_provider(&provider);
                *provider_slot = Some(provider);
            }
            (true, false) => {
                if let Some(provider) = provider_slot.take() {
                    self.completion.remove_provider(&provider);
                }
            }
            _ => {}
        }
        drop(provider_slot);

        let mut rhyme_slot = self.rhyme_highlight.borrow_mut();
        match (rhyme_slot.is_some(), settings.rhyme_highlighting) {
            (false, true) => {
                *rhyme_slot = Some(crate::rhyme::highlight::attach(&self.buffer));
            }
            (true, false) => {
                if let Some(handle) = rhyme_slot.take() {
                    handle.detach();
                }
            }
            _ => {}
        }
    }

    pub fn set_text(&self, text: &str) {
        self.buffer.set_text(text);
        self.source_view.grab_focus();
    }

    pub fn get_text(&self) -> String {
        self.buffer
            .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false)
            .to_string()
    }

    /// Associate this editor with the file it should auto-save to.
    /// Deliberately separate from construction/`set_text()`, so loading a
    /// tab's initial content never triggers a spurious save.
    pub fn set_path(&self, path: PathBuf) {
        self.current_path.replace(Some(path));
    }

    pub fn get_widget(&self) -> &Frame {
        &self.frame
    }

    /// Notify `f` on every buffer edit — used by the status bar's live
    /// word-count listener, independent of the autosave debounce above.
    pub fn connect_changed(&self, f: impl Fn() + 'static) {
        self.buffer.connect_changed(move |_| f());
    }
}

impl Clone for TextEditor {
    fn clone(&self) -> Self {
        let editor = TextEditor::new();
        editor.set_text(&self.get_text());
        if let Some(path) = self.current_path.borrow().clone() {
            editor.set_path(path);
        }
        editor
    }
}

/// Builds a gutter renderer that shows each line's syllable count, live —
/// separate from `TextEditor::new()` so `apply_settings` can add this back
/// after the setting was toggled off and back on again.
fn create_syllable_renderer(buffer: &SourceBuffer) -> GutterRendererText {
    let syllable_renderer = GutterRendererText::new();
    syllable_renderer.set_css_classes(&["syllable-count"]);
    syllable_renderer.set_xalign(0.5);
    syllable_renderer.set_yalign(0.5);

    let buffer_clone = buffer.clone();
    syllable_renderer.connect_query_data(move |renderer, _line_obj, line_num| {
        if let Some(iter) = buffer_clone.iter_at_line(line_num as i32) {
            let end_iter = if let Some(next_iter) = buffer_clone.iter_at_line(line_num as i32 + 1) {
                next_iter
            } else {
                buffer_clone.end_iter()
            };

            let line = buffer_clone.text(&iter, &end_iter, false);
            if !line.trim().is_empty() {
                let syllables = crate::editor::stat::count_syllables(&line);
                renderer.set_text(&syllables.to_string());
            } else {
                renderer.set_text("");
            }
        }
    });

    syllable_renderer
}

/// Walk up from a file's directory looking for the workspace's `.git` —
/// TextEditor doesn't otherwise know the workspace root.
fn find_git_root(file_path: &std::path::Path) -> Option<PathBuf> {
    let mut dir = file_path.parent();
    while let Some(current) = dir {
        if current.join(".git").is_dir() {
            return Some(current.to_path_buf());
        }
        dir = current.parent();
    }
    None
}
