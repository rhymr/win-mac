use gtk::prelude::*;
use gtk::{Box, Button, Frame, Notebook, ScrolledWindow, TextBuffer, TextView, Label};
use crate::utils::file_ops::FileOps;
use std::path::{Path, PathBuf};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Workspace {
    frame: Frame,
    notebook: Notebook,
    open_files: Rc<RefCell<Vec<PathBuf>>>,
}

impl Workspace {
    pub fn new() -> Self {
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
        };

        // Create empty state container
        let empty_state = Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(vec!["empty-state-box"])
            // .spacing(10)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .build();

        let empty_label = Label::builder()
            .label("No files open")
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

        // Show empty state if no tabs
        if workspace.open_files.borrow().is_empty() {
            workspace.notebook.append_page(&empty_state, Option::<&gtk::Widget>::None);
            workspace.notebook.set_show_tabs(false);
        }

        // Handle button clicks
        let notebook_ref = notebook.clone();
        let open_files_ref = open_files.clone();

        new_button.connect_clicked(move |_| {
            let (path, content) = FileOps::new_file();
            // Remove empty state if it exists (first tab when no files are open)
            if notebook_ref.n_pages() == 1 && open_files_ref.borrow().is_empty() {
                notebook_ref.remove_page(Some(0));
            }
            add_new_tab(&notebook_ref, &path, &content);
            open_files_ref.borrow_mut().push(path);
        });

        let notebook_ref = notebook.clone();
        let open_files_ref = open_files.clone();

        open_button.connect_clicked(move |_| {
            if let Some((path, content)) = FileOps::open_file(None) {
                // Remove empty state if it exists (first tab when no files are open)
                if notebook_ref.n_pages() == 1 && open_files_ref.borrow().is_empty() {
                    notebook_ref.remove_page(Some(0));
                }
                add_new_tab(&notebook_ref, &path, &content);
                open_files_ref.borrow_mut().push(path);
            }
        });

        button_box.append(&new_button);
        button_box.append(&open_button);
        empty_state.append(&empty_label);
        empty_state.append(&button_box);

        workspace
    }

    pub fn add_new_tab(&self, path: &Path, content: &str) {
        // Remove empty state if it exists
        if self.notebook.n_pages() == 1 && self.open_files.borrow().is_empty() {
            self.notebook.remove_page(Some(0));
        }
        
        add_new_tab(&self.notebook, path, content);
        self.open_files.borrow_mut().push(path.to_path_buf());
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
                    self.notebook.set_tab_label(
                        &page,
                        Some(&gtk::Label::new(Some(&tab_label)))
                    );
                    
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
}

fn add_new_tab(notebook: &Notebook, path: &Path, content: &str) {
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

    // Create tab label
    let tab_label = &Label::new(Some(&path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string()));

    // Add the page and switch to it
    let page_num = notebook.append_page(&scrolled_window, Some(tab_label));
    notebook.set_show_tabs(true);
    notebook.set_current_page(Some(page_num));
} 