use gtk::prelude::*;
use gtk::{Frame, ScrolledWindow, TextView, WrapMode};

pub struct TextEditor {
    frame: Frame,
    text_view: TextView,
}

impl TextEditor {
    pub fn new() -> Self {
        // Create the text view and bind the text buffer to it
        let text_view = TextView::builder()
            .editable(true)
            .cursor_visible(true)
            .wrap_mode(WrapMode::Char)
            .monospace(true)
            .build();

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
            text_view,
        }
    }

    pub fn get_widget(&self) -> &Frame {
        &self.frame
    }

    pub fn get_text_view(&self) -> &TextView {
        &self.text_view
    }
}