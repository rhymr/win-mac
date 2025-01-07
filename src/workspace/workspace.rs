use crate::utils::file_ops::FileOps;
use gtk::prelude::*;
use gtk::{Box, Button, Frame, Label, Notebook, ScrolledWindow, TextBuffer, TextView, Window};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use crate::workspace::file_tree::FileTree;
use crate::workspace::workspace_controller::WorkspaceController;

pub struct Workspace {
    frame: Frame,
    pub(crate) notebook: Notebook,
    pub(crate) open_files: Rc<RefCell<Vec<PathBuf>>>,
    controller: Rc<WorkspaceController>,
    pub(crate) file_tree: Option<FileTree>,
}

impl Workspace {
    pub fn new(controller: Rc<WorkspaceController>, file_tree: Option<FileTree>) -> Self {
        // Create notebook (tabbed interface)
        let notebook = Notebook::builder()
            .scrollable(true)
            .show_border(false)
            .css_classes(vec!["workspace-notebook"])
            .build();

        let open_files = Rc::new(RefCell::new(Vec::new()));

        let workspace = Self {
            frame: Frame::builder()
                .css_classes(vec!["workspace-frame"])
                .child(&notebook)
                .build(),
            notebook: notebook.clone(),
            open_files: open_files.clone(),
            controller,
            file_tree,
        };

        // Check if there are no open files and display the empty state
        if workspace.open_files.borrow().is_empty() {
            let empty_state = workspace.create_empty_state();
            notebook.append_page(&empty_state, Option::<&gtk::Widget>::None);
            notebook.set_show_tabs(false);
        }

        workspace
    }

    pub fn set_controller(&mut self, controller: Rc<WorkspaceController>) {
        // Store the controller for use in tab operations
        self.controller = controller;
    }

    pub fn get_controller(&self) -> &Rc<WorkspaceController> {
        &self.controller
    }

    pub fn add_new_tab(&self, path: &Path, content: &str) -> u32 {
        // Remove empty state if it exists
        if self.notebook.n_pages() == 1 && self.open_files.borrow().is_empty() {
            self.notebook.remove_page(Some(0));
        }
        
        // Ensure the controller is still referenced
        let controller = self.controller.clone();
        let page_num = add_new_tab(&self.notebook, path, content, Some(controller));
        self.open_files.borrow_mut().push(path.to_path_buf());

        // Update the file tree and select the new tab
        if let Some(ref file_tree) = self.file_tree {
            let open_files = self.get_open_files();
            file_tree.update_file_list(open_files);
            file_tree.select_row(page_num as i32);
        }
        
        // Ensure notebook shows tabs and has proper styling
        self.notebook.set_show_tabs(true);
        self.notebook.add_css_class("has-open-files");

        page_num
    }

    pub fn get_current_buffer(&self) -> Option<(TextBuffer, Option<PathBuf>)> {
        let current_page = self.notebook.current_page()?;
        let scrolled_window = self.notebook.nth_page(Some(current_page))?;
        let text_view = scrolled_window.first_child()?.downcast::<TextView>().ok()?;
        
        // Get the buffer and path
        let buffer = text_view.buffer();
        let path = self.open_files.borrow().get(current_page as usize).cloned();
        
        // Return the tuple directly since we already handled the Option
        Some((buffer, path))
    }

    pub fn update_current_tab_path(&self, new_path: PathBuf) {
        if let Some(current_page) = self.notebook.current_page() {
            if let Some(tab_label) = new_path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
            {
                if let Some(page) = self.notebook.nth_page(Some(current_page)) {
                    let tab_box = Box::builder()
                        .orientation(gtk::Orientation::Horizontal)
                        .spacing(4)  // Reduced spacing
                        .build();

                    let label = Label::new(Some(&tab_label));
                    let close_button = Button::builder()
                        .icon_name("window-close-symbolic")
                        .css_classes(vec!["flat", "tab-close-button"])
                        .build();

                    tab_box.append(&label);
                    tab_box.append(&close_button);

                    // Connect close button signal using controller directly
                    let controller = self.controller.clone();
                    close_button.connect_clicked(move |button| {
                        if let Some(window) = button.root().and_downcast::<Window>() {
                            controller.handle_close_tab(&window);
                        }
                    });

                    self.notebook.set_tab_label(&page, Some(&tab_box));
                    
                    if let Some(path) = self.open_files.borrow_mut().get_mut(current_page as usize) {
                        *path = new_path;
                    }
                }
            }
        }
    }

    pub fn get_widget(&self) -> &Frame {
        &self.frame
    }

    pub fn create_empty_state(&self) -> Box {
        let empty_state = Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(vec!["empty-state-box"])
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .build();

        let empty_label = Label::builder()
            .label("no files open")
            .css_classes(vec!["empty-state-label"])
            .build();

        let button_box = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .css_classes(vec!["empty-button-box"])
            .halign(gtk::Align::Center)
            .build();

        let new_button = Button::builder()
            .label("New File")
            .css_classes(vec!["suggested-action"])
            .build();

        let open_button = Button::builder()
            .label("Open File")
            .css_classes(vec!["suggested-action"])
            .build();

        // Clone the Rc pointers to avoid borrowing issues
        let controller_ref = self.controller.clone();

        new_button.connect_clicked(move |button| {
            controller_ref.handle_new_file();
        });

        let notebook_ref = self.notebook.clone();
        let open_files_ref = self.open_files.clone();
        let controller_ref = self.controller.clone();
        let file_tree_ref = self.file_tree.clone();

        open_button.connect_clicked(move |button| {
            if let Some(window) = button.root().and_downcast::<Window>() {
                if let Some((path, content)) = FileOps::open_file(Some(window.clone())) {
                    if notebook_ref.n_pages() == 1 && open_files_ref.borrow().is_empty() {
                        notebook_ref.remove_page(Some(0));
                    }
                    let page_num = add_new_tab(&notebook_ref, &*path, &*content, Some(controller_ref.clone()));
                    open_files_ref.borrow_mut().push(path);
                    
                    // Update file tree and select the new tab
                    if let Some(workspace) = controller_ref.get_workspace() {
                        if let Some(ref file_tree) = workspace.file_tree {
                            let open_files = workspace.get_open_files();
                            file_tree.update_file_list(open_files);
                            file_tree.select_row(page_num as i32);
                        }
                    }
                }
            }
        });

        button_box.append(&new_button);
        button_box.append(&open_button);
        empty_state.append(&empty_label);
        empty_state.append(&button_box);

        empty_state
    }

    pub fn get_open_files(&self) -> Vec<String> {
        self.open_files.borrow().iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()).map(|s| s.to_string()))
            .collect()
    }

    pub fn remove_tab(&self, index: usize) {
        // Remove the tab
        self.notebook.remove_page(Some(index as u32));
        self.open_files.borrow_mut().remove(index);
        
        // Show empty state if no more tabs
        if self.notebook.n_pages() <= 0 {
            self.notebook.remove_css_class("has-open-files");
            let empty_state = self.create_empty_state();
            self.notebook.append_page(&empty_state, Option::<&gtk::Widget>::None);
            self.notebook.set_show_tabs(false);
        }

        // Update the file tree
        if let Some(ref file_tree) = self.file_tree {
            let open_files = self.get_open_files();
            file_tree.update_file_list(open_files);
        }
    }

    pub fn switch_to_tab(&self, index: usize) {
        if index < self.notebook.n_pages() as usize {
            self.notebook.set_current_page(Some(index as u32));
        }
    }
}

fn add_new_tab(notebook: &Notebook, path: &Path, content: &str, controller: Option<Rc<WorkspaceController>>) -> u32 {
    // Create text view and buffer
    let text_view = TextView::builder()
        .editable(true)
        .wrap_mode(gtk::WrapMode::Word)
        .build();
    
    let buffer = TextBuffer::builder()
        .text(content)
        .build();
    
    text_view.set_buffer(Some(&buffer));

    // Add text view to a scrolled window
    let scrolled_window = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&text_view)
        .build();

    // Create tab label box
    let tab_box = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .css_classes(vec!["tab-box"])
        .spacing(0)  // Reduced spacing
        .build();

    // Create the label
    let label = Label::new(Some(&path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")));

    // Create close button
    let close_button = Button::builder()
        .icon_name("window-close-symbolic")
        .css_classes(vec!["tab-close-button"])
        .build();

    tab_box.append(&label);
    tab_box.append(&close_button);

    // Add the page with our custom tab
    let page_num = notebook.append_page(&scrolled_window, Some(&tab_box));

    // Connect close button signal
    if let Some(controller) = controller {
        close_button.connect_clicked(move |button| {
            // Get the application window from the button's toplevel
            if let Some(window) = button.root().and_downcast::<Window>() {
                controller.handle_close_tab(&window);
                println!("Close button clicked {}", &window.title().unwrap().to_lowercase());
            } else {
                println!("Error: Could not get window from button");
            }
        });
    } else {
        println!("No controller available");
    }

    notebook.set_show_tabs(true);
    notebook.add_css_class("has-open-files");
    notebook.set_current_page(Some(page_num));
    
    page_num
} 