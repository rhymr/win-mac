use gtk::prelude::*;
use gtk::{Application, Orientation, Paned};
use std::rc::Rc;
use crate::ui::rhyme_search::RhymeSearch;
use crate::workspace::file_tree::FileTree;
use crate::workspace::workspace::Workspace;
use crate::workspace::workspace_controller::WorkspaceController;

pub fn build_ui(app: &Application) -> (gtk::ApplicationWindow, Rc<WorkspaceController>) {
    let css_provider = crate::css::load_css();
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

    crate::ui::menu::setup_menu(app, workspace_controller.clone());

    main_window.present();

    (main_window, workspace_controller)
}

pub fn create_main_layout() -> (Paned, Rc<WorkspaceController>) {
    // Create the workspace controller
    let workspace_controller = Rc::new(WorkspaceController::new());

    // Create the main content area: file tree on the left, editor on the right
    let (content_pane, _file_tree, _workspace) = create_content_layout(&workspace_controller);

    (content_pane, workspace_controller)
}

// Create the file tree / rhyme search / editor split
fn create_content_layout(workspace_controller: &Rc<WorkspaceController>) -> (Paned, FileTree, Rc<Workspace>) {
    // Create the FileTree component
    let mut file_tree = FileTree::new();

    // Create the Workspace instance with the FileTree
    let workspace = Rc::new(Workspace::new(
        Rc::clone(workspace_controller),
        Some(file_tree.clone())
    ));

    // Set the workspace reference in the file tree
    file_tree.set_workspace(workspace.clone());

    workspace_controller.set_workspace(workspace.clone());

    // Rhyme search sits below the file tree on the left
    let rhyme_search = RhymeSearch::new();
    let rhyme_search_widget = rhyme_search.get_widget();
    rhyme_search_widget.add_css_class("bottom-section");

    let file_tree_widget = file_tree.get_widget();
    file_tree_widget.add_css_class("left-edge");
    let left_split = create_vertical_split(file_tree_widget, rhyme_search_widget, 360);

    // Horizontal split between the left column and the editor
    let main_pane = create_horizontal_split(&left_split, workspace.get_widget(), 320);

    (main_pane, file_tree, workspace)
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
