use crate::utils::file_ops::FileOps;
use gtk::prelude::*;
use gtk::Window;
use std::cell::RefCell;
use std::rc::Rc;
use crate::workspace::workspace::Workspace;

pub struct WorkspaceController {
    pub(crate) workspace: RefCell<Option<Rc<Workspace>>>,
}

impl WorkspaceController {
    pub fn new() -> Self {
        Self {
            workspace: RefCell::new(None),
        }
    }

    pub fn set_workspace(&self, workspace: Rc<Workspace>) {
        // Store the controller for use in tab operations
        self.workspace.replace(Some(workspace));
    }

    pub fn get_workspace(&self) -> Option<Rc<Workspace>> {
        self.workspace.borrow().clone()
    }

    pub fn handle_new_file(&self) {
        if let Some(workspace) = self.get_workspace() {
            let (path, content) = FileOps::new_file();
            workspace.add_new_tab(&path, &content);
            
            // Update the file tree after adding new file
            let open_files = workspace.get_open_files();
            if let Some(ref file_tree) = workspace.file_tree {
                file_tree.update_file_list(open_files);
            }
        }
    }

    pub fn handle_open_file(&self, window: &Window) {
        if let Some((path, content)) = FileOps::open_file(Some(window.clone())) {
            if let Some(workspace) = self.workspace.borrow().as_ref() {
                workspace.add_new_tab(&path, &content);
                
                // Update the file tree after opening the file
                let open_files = workspace.get_open_files();
                if let Some(ref file_tree) = workspace.file_tree {
                    file_tree.update_file_list(open_files);
                }
            }
        }
    }

    pub fn handle_save_file(&self, window: &Window) {
        if let Some(workspace) = self.get_workspace() {
            if let Some((buffer, path)) = workspace.get_current_buffer() {
                let content = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                if let Some(existing_path) = path {
                    FileOps::save_file(content.to_string(), Some(existing_path), Some(window.clone()));
                } else {
                    if let Some(new_path) = FileOps::save_file(content.to_string(), None, Some(window.clone())) {
                        workspace.update_current_tab_path(new_path);
                    }
                }
            }
        }
    }

    pub fn handle_save_as_file(&self, window: &Window) {
        if let Some(workspace) = self.get_workspace() {
            if let Some((buffer, _)) = workspace.get_current_buffer() {
                let content = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                if let Some(new_path) = FileOps::save_file(content.to_string(), None, Some(window.clone())) {
                    workspace.update_current_tab_path(new_path);
                }
            }
        }
    }

    pub fn handle_close_tab(&self, _window: &Window) {
        if let Some(workspace) = self.get_workspace() {
            if let Some(current_page) = workspace.notebook.current_page() {
                workspace.remove_tab(current_page as usize);
            }
        }
    }
} 