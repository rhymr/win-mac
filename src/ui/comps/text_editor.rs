use gtk::prelude::*;
use gtk::{Frame, ScrolledWindow, TextBuffer, TextView, WrapMode};

pub struct TextEditor {
    frame: Frame,
    text_buffer: TextBuffer,
    text_view: TextView,
}

impl TextEditor {
    pub fn new() -> Self {
        // Create a new text buffer
        let text_buffer = TextBuffer::builder()
            .text("Hello from Rust in the Text Editor!") // Initial content
            .build();

        // Create the text view and bind the text buffer to it
        let text_view = TextView::builder()
            .editable(true)
            .cursor_visible(true)
            .wrap_mode(WrapMode::Char)
            .monospace(true)
            .build();

        text_view.set_buffer(Some(&text_buffer));

        // Create a scrollable widget for the text view
        let text_scroll = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&text_view)
            .build();

        // Create the frame to hold the scrollable text view
        let frame = Frame::builder()
            .label("Text Editor")
            .child(&text_scroll)
            .build();

        Self {
            frame,
            text_buffer,
            text_view,
        }
    }

    pub fn get_widget(&self) -> &Frame {
        &self.frame
    }

    pub fn get_text_buffer(&self) -> &TextBuffer {
        &self.text_buffer
    }

    pub fn get_text_view(&self) -> &TextView {
        &self.text_view
    }
}