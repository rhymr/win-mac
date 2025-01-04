use gtk::prelude::*;
use gtk::{Application, Frame, Orientation, Paned, ScrolledWindow, TextBuffer, TextView, WrapMode};
use crate::ui::workspace_controller::WorkspaceController;
use std::rc::Rc;
use crate::ui::comps::rhyme_search::RhymeSearch;
use crate::ui::comps::workspace::Workspace;
use std::cell::RefCell;

pub fn build_ui(app: &Application) {
    let css_provider = crate::ui::css::load_css("assets/css/dark.css");

    crate::ui::css::apply_css_to_app(&css_provider);

    let main_window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Rhymr")
        .default_width(1280)
        .default_height(720)
        .build();

    let (main_layout, workspace_controller) = create_main_layout();

    main_window.set_child(Some(&main_layout));

    crate::ui::menu::setup_menu(app, workspace_controller);

    main_window.present();
}

pub fn create_main_layout() -> (gtk::Paned, Rc<WorkspaceController>) {
    // Left Section
    let left_frame = create_label_section("Navigation", "Left Section: Navigation");

    // Right Section
    let right_frame = create_label_section("Inspector / Details", "Right Section: Inspector/Details");

    // Create the workspace first
    let workspace_controller = Rc::new(WorkspaceController::new());

    let mut workspace = Rc::new(Workspace::new(Rc::clone(&workspace_controller)));

    // Set the controller in the workspace
    if let Some(workspace_mut) = Rc::get_mut(&mut workspace) {
        workspace_mut.set_controller(Rc::clone(&workspace_controller));
    }
    workspace_controller.set_workspace(Rc::clone(&workspace));

    // Vertical Center (Top and Bottom)
    let bottom_frame = RhymeSearch::new();
    let vertical_pane = create_vertical_split(workspace.get_widget(), bottom_frame.get_widget(), 432);

    // Horizontal Split between Left and Center
    let left_and_center_pane = create_horizontal_split(&left_frame, &vertical_pane, 320);

    // Create the final horizontal split
    let final_split = Rc::new(create_horizontal_split(&left_and_center_pane, &right_frame, 960));

    (final_split.as_ref().clone(), workspace_controller)
}

pub fn create_horizontal_split(
    left: &impl IsA<gtk::Widget>,
    right: &impl IsA<gtk::Widget>,
    position: i32,
) -> Paned {
    let horizontal_pane = Paned::new(Orientation::Horizontal);
    horizontal_pane.set_start_child(Some(left));
    horizontal_pane.set_end_child(Some(right));
    horizontal_pane.set_position(position);

    horizontal_pane.set_resize_start_child(true);
    horizontal_pane.set_resize_end_child(true);
    horizontal_pane.set_shrink_start_child(false);
    horizontal_pane.set_shrink_end_child(false);

    horizontal_pane
}

pub fn create_vertical_split(
    top: &impl IsA<gtk::Widget>,
    bottom: &impl IsA<gtk::Widget>,
    position: i32,
) -> Paned {
    let vertical_pane = Paned::new(Orientation::Vertical);
    vertical_pane.set_start_child(Some(top));
    vertical_pane.set_end_child(Some(bottom));
    vertical_pane.set_position(position);

    vertical_pane.set_resize_start_child(true);
    vertical_pane.set_resize_end_child(true);
    vertical_pane.set_shrink_start_child(false);
    vertical_pane.set_shrink_end_child(false);

    vertical_pane
}

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