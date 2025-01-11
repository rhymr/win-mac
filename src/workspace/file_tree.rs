use crate::workspace::workspace::Workspace;
use gtk::prelude::*;
use gtk::{Align, Box as GtkBox, Frame, Image, Label, ListBox, Orientation};
use std::rc::Rc;

#[derive(Clone)]
pub struct FileTree {
    frame: Frame,
    file_list: ListBox,
    workspace: Option<Rc<Workspace>>,
}

impl FileTree {
    pub fn new(open_files: Vec<String>) -> Self {
        // Create a ListBox to display the open files
        let file_list = ListBox::builder()
            .css_classes(vec!["file-list"])
            .selection_mode(gtk::SelectionMode::Single)
            .build();

        let label = Label::builder()
            .label("Workspace Open Files")
            .css_classes(vec!["file-tree-header"])
            .height_request(28)
            .build();

        // Create a box to hold both the label and list with consistent background
        let inner_container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .css_classes(vec!["file-tree-inner"])
            .build();

        inner_container.append(&label);
        inner_container.append(&file_list);

        // Create the outer container with margin
        let outer_container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .css_classes(vec!["file-tree-outer"])
            .build();
        outer_container.append(&inner_container);

        let frame = Frame::builder()
            .child(&outer_container)
            .css_classes(vec!["file-tree-container"])
            .build();

        // Populate the ListBox with open files
        let file_tree = Self {
            frame,
            file_list,
            workspace: None,
        };

        file_tree.update_file_list(open_files);

        file_tree
    }

    pub fn set_workspace(&mut self, workspace: Rc<Workspace>) {
        self.workspace = Some(workspace.clone());
        let workspace_ref = self.workspace.clone();
        
        // Connect the row selection signal
        self.file_list.connect_row_selected(move |_, row| {
            if let Some(workspace) = &workspace_ref {
                if let Some(row) = row {
                    if let index = row.index() {
                        workspace.switch_to_tab(index as usize);
                    }
                }
            }
        });

        // Connect to notebook page changes to update selection
        let file_list = self.file_list.clone();
        workspace.notebook.connect_switch_page(move |_, _, page_num| {
            file_list.select_row(file_list.row_at_index(page_num as i32).as_ref());
        });
    }

    pub fn update_file_list(&self, open_files: Vec<String>) {
        println!("Updating file list with files: {:?}", open_files);

        // Store current page number before clearing
        let current_page = if let Some(workspace) = &self.workspace {
            workspace.notebook.current_page()
        } else {
            None
        };

        // Clear existing items
        let mut child = self.file_list.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            self.file_list.remove(&widget);
        }

        for file in open_files {
            // Create a horizontal box to hold the icon and label
            let hbox = GtkBox::new(Orientation::Horizontal, 0);
            hbox.set_valign(Align::Center);
            hbox.set_css_classes(&["file-list-row"]);
            
            // Create the icon from the resource
            let icon = Image::from_resource("/org/gtk_rs/rhymr/icons/text_dark.svg");
            icon.set_pixel_size(16);
            icon.set_css_classes(&["file-icon"]);
            
            // Create the label for the file
            let label = Label::new(Some(&file));
            
            // Add the icon and label to the box
            hbox.append(&icon);
            hbox.append(&label);

            // Add the horizontal box to the file list
            self.file_list.append(&hbox);
        }

        // Restore selection after updating
        if let Some(page_num) = current_page {
            self.file_list.select_row(self.file_list.row_at_index(page_num as i32).as_ref());
        }
    }

    pub fn get_widget(&self) -> &Frame {
        &self.frame
    }

    pub fn select_row(&self, index: i32) {
        if let Some(row) = self.file_list.row_at_index(index) {
            self.file_list.select_row(Some(&row));
        }
    }
} 