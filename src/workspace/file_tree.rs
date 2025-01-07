use gtk::prelude::*;
use gtk::{Align, Box as GtkBox, Frame, Image, Label, ListBox, Orientation};
use gtk::builders::LabelBuilder;

#[derive(Clone)]
pub struct FileTree {
    frame: Frame,
    file_list: ListBox
}

impl FileTree {
    pub fn new(open_files: Vec<String>) -> Self {
        // Create a ListBox to display the open files
        let file_list = ListBox::builder()
            .css_classes(vec!["file-list"])
            .build();

        let label = Label::new(Some("Workspace"));

        // Create a vertical box for the file tree
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.append(&label);
        container.append(&file_list);

        let frame = Frame::builder()
            .child(&container)
            .css_classes(vec!["file-tree-container"])
            .build();

        // Populate the ListBox with open files
        let file_tree = Self {
            frame,
            file_list,
            // header_label,
        };

        file_tree.update_file_list(open_files);

        file_tree
    }

    pub fn update_file_list(&self, open_files: Vec<String>) {
        println!("Updating file list with files: {:?}", open_files);

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
    }

    pub fn get_widget(&self) -> &Frame {
        &self.frame
    }
} 