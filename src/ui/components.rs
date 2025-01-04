use gtk::{prelude::*, Label, Frame, ScrolledWindow, TextBuffer, TextView};

pub fn create_section(label: &str, text_content: &str, editable: bool) -> Frame {
    // Create a TextBuffer
    let text_buffer = TextBuffer::builder().text(text_content).build();

    // Create a TextView, attach TextBuffer
    let text_view = TextView::builder()
        .editable(editable)
        .cursor_visible(editable)
        .wrap_mode(gtk::WrapMode::Word)
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

pub fn create_label_section(frame_label: &str, label_text: &str) -> Frame {
    // Create a simple label
    let label = Label::builder()
        .label(label_text)
        .halign(gtk::Align::Start)
        .build();

    // Wrap the label in a Frame
    Frame::builder()
        .label(frame_label)
        .child(&label)
        .hexpand(true)
        .vexpand(true)
        .build()
}

pub fn create_main_layout() -> gtk::Paned {
    // Left Section
    let left_frame = create_label_section("Navigation", "Left Section: Navigation");

    // Right Section
    let right_frame = create_label_section("Inspector / Details", "Right Section: Inspector/Details");

    // Vertical Center (Top and Bottom)
    let top_frame = create_section("Top Section", "This is the top section (3/5 height).", false);
    let bottom_frame =
        create_section("Bottom Section", "This is the bottom section (2/5 height).", false);
    let vertical_pane = crate::ui::layout::create_vertical_split(&top_frame, &bottom_frame, 432);

    // Horizontal Split between Left and Center
    let left_and_center_pane =
        crate::ui::layout::create_horizontal_split(&left_frame, &vertical_pane, 320);

    // Main Horizontal Split (Left/Center + Right)
    crate::ui::layout::create_horizontal_split(&left_and_center_pane, &right_frame, 960)
}