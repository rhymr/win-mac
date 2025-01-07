
use gtk::prelude::*;
use gtk::{Application, Box as GtkBox, Frame, Label, Orientation, Paned, ScrolledWindow, TextBuffer, TextView, WrapMode};
use std::rc::Rc;
use crate::rhyme_search::RhymeSearch;
use crate::workspace::file_tree::FileTree;
use crate::workspace::workspace::Workspace;
use crate::workspace::workspace_controller::WorkspaceController;

pub fn build_ui(app: &Application) -> (gtk::ApplicationWindow, Rc<WorkspaceController>) {
    let css_provider = crate::css::load_css("assets/css/dark.css");
    crate::css::apply_css_to_app(&css_provider);

    let main_window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Rhymr")
        .default_width(1280)
        .default_height(720)
        .build();

    let (main_layout, workspace_controller) = create_main_layout();
    
    // Store the workspace controller in the window's data safely
    unsafe {
        main_window.set_data("workspace_controller", workspace_controller.clone());
    }

    main_window.set_child(Some(&main_layout));

    crate::menu::setup_menu(app, workspace_controller.clone());

    main_window.present();
    
    (main_window, workspace_controller)
}

pub fn create_main_layout() -> (GtkBox, Rc<WorkspaceController>) {
    // Create the workspace controller
    let workspace_controller = Rc::new(WorkspaceController::new());

    // Create main vertical box to hold everything
    let main_box = GtkBox::new(Orientation::Vertical, 0);

    // Create the main content area (existing paned layout)
    let (content_pane, _file_tree, _workspace) = create_content_layout(&workspace_controller);
    main_box.append(&content_pane);

    // Create the status bar
    let status_bar = create_status_bar();
    main_box.append(&status_bar);

    (main_box, workspace_controller)
}

fn create_status_bar() -> GtkBox {
    let status_bar = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(vec!["status-bar"])
        .build();
    
    // Add some sample content to the status bar
    let status_label = Label::new(Some("Rhymr"));
    status_label.set_css_classes(&["status-text"]);
    status_bar.append(&status_label);

    status_bar
}

// Move existing paned layout creation to a new function
fn create_content_layout(workspace_controller: &Rc<WorkspaceController>) -> (Paned, FileTree, Rc<Workspace>) {
    // Create the FileTree component with open files
    let open_files = Vec::new();
    let file_tree = FileTree::new(open_files.clone());

    // Create the Workspace instance with the FileTree
    let workspace = Rc::new(Workspace::new(
        Rc::clone(workspace_controller),
        Some(file_tree.clone())
    ));

    workspace_controller.set_workspace(workspace.clone());

    // Create the bottom frame
    let bottom_frame = RhymeSearch::new();
    let vertical_pane = create_vertical_split(workspace.get_widget(), bottom_frame.get_widget(), 432);

    // Horizontal Split between File Tree and Center
    let left_and_center_pane = create_horizontal_split(file_tree.get_widget(), &vertical_pane, 320);

    // Create the final horizontal split
    let right_frame = create_label_section("Right", "Description");
    let final_split = create_horizontal_split(&left_and_center_pane, &right_frame, 960);

    (final_split, file_tree, workspace)
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