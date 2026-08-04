use gtk::prelude::*;
use gtk::{Frame, ScrolledWindow};
use sourceview5::{Buffer as SourceBuffer, View as SourceView};
use sourceview5::prelude::{BufferExt, ViewExt, GutterRendererExt, GutterRendererTextExt};
use sourceview5::GutterRendererText;

pub struct TextEditor {
    frame: Frame,
    source_view: SourceView,
    buffer: SourceBuffer
}

impl TextEditor {
    pub fn new() -> Self {
        // Create the source buffer and view
        let buffer = SourceBuffer::new(None);
        let source_view = SourceView::builder()
            .buffer(&buffer)
            .monospace(true)
            .show_line_numbers(true)
            .show_line_marks(true)
            .tab_width(4)
            .auto_indent(true)
            .indent_width(4)
            .highlight_current_line(true)
            .background_pattern(sourceview5::BackgroundPatternType::None)
            .smart_backspace(true)
            .smart_home_end(sourceview5::SmartHomeEndType::After)
            .hexpand(true)
            .vexpand(true)
            .build();

        source_view.set_css_classes(&["rhyme-editor-view"]);

        // Create the syllable count renderer once
        let syllable_renderer = GutterRendererText::new();
        syllable_renderer.set_css_classes(&["syllable-count"]);
        syllable_renderer.set_xalign(0.5);
        syllable_renderer.set_yalign(0.5);

        // Connect the query-data handler to update syllable counts
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
                    let syllables = count_syllables(&line);
                    renderer.set_text(&syllables.to_string());
                } else {
                    renderer.set_text("");
                }
            }
        });

        // Get the gutter and add the renderer right after line numbers
        let gutter = ViewExt::gutter(&source_view, gtk::TextWindowType::Left);
        gutter.insert(&syllable_renderer, -20); // Position right after line numbers (-30)

        // Disable bracket matching
        buffer.set_highlight_matching_brackets(false);

        let scheme_manager = sourceview5::StyleSchemeManager::default();
        scheme_manager.append_search_path("assets/styles");
        let style_scheme = scheme_manager.scheme("rhymr").expect("Failed to load rhymr style scheme");
        buffer.set_style_scheme(Some(&style_scheme));

        // Configure the gutter
        let gutter = ViewExt::gutter(&source_view, gtk::TextWindowType::Left);
        gutter.set_css_classes(&["rhyme-editor-gutter"]);

        // Add custom CSS classes for the editor
        source_view.set_css_classes(&["rhyme-editor"]);

        // Add scrolling support
        let scroll = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&source_view)
            .build();

        let frame = Frame::builder()
            .child(&scroll)
            .build();

        let editor = Self {
            frame,
            source_view,
            buffer
        };

        // Set initial empty state
        editor.set_text("");

        editor
    }

    pub fn set_text(&self, text: &str) {
        self.buffer.set_text(text);
        self.source_view.grab_focus();
    }

    pub fn get_text(&self) -> String {
        self.buffer.text(
            &self.buffer.start_iter(),
            &self.buffer.end_iter(),
            false
        ).to_string()
    }

    pub fn get_widget(&self) -> &Frame {
        &self.frame
    }

}

impl Clone for TextEditor {
    fn clone(&self) -> Self {
        let editor = TextEditor::new();
        editor.set_text(&self.get_text());
        editor
    }
}

fn count_syllables(line: &str) -> u32 {
    // Basic syllable counting - you might want to use a more sophisticated algorithm
    // or integrate with a pronunciation dictionary
    let mut count = 0;
    let vowels = ['a', 'e', 'i', 'o', 'u', 'y', 'A', 'E', 'I', 'O', 'U', 'Y'];
    let mut prev_was_vowel = false;

    for c in line.chars() {
        let is_vowel = vowels.contains(&c);
        if is_vowel && !prev_was_vowel {
            count += 1;
        }
        prev_was_vowel = is_vowel;
    }

    count
}