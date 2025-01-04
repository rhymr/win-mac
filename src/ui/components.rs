use gtk::{prelude::*, Frame, Label, ScrolledWindow, TextBuffer, TextView, WrapMode};

pub fn create_label_section(label: &str, text_content: &str) -> Frame {
    // Create a TextBuffer
    let text_buffer = TextBuffer::builder().text(text_content).build();

    // Create a TextView, attach TextBuffer
    let text_view = TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .wrap_mode(WrapMode::Word)
        .build();
    text_view.set_buffer(Some(&text_buffer));

    // Wrap the TextView in a ScrolledWindow
    let scroll_window = ScrolledWindow::new();
    scroll_window.set_child(Some(&text_view));

    // Add the ScrolledWindow to a Frame
    let frame = Frame::new(Some(label));
    frame.set_child(Some(&scroll_window));
    frame.set_hexpand(true);
    frame.set_vexpand(true);

    frame
}