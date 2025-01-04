use crate::ui::comps::workspace::Workspace;
use crate::utils::file_ops::FileOps;
use gtk::prelude::*;
use gtk::Window;
use std::rc::Rc;

pub struct WorkspaceController {
    pub workspace: Rc<Workspace>,
}

impl WorkspaceController {
    pub fn new(workspace: Workspace) -> Self {
        Self {
            workspace: Rc::new(workspace),
        }
    }

    pub fn handle_new_file(&self, window: &Window) {
        let (path, content) = FileOps::new_file();
        self.workspace.add_new_tab(&path, &content);
    }

    pub fn handle_open_file(&self, window: &Window) {
        if let Some((path, content)) = FileOps::open_file(Some(window.clone())) {
            self.workspace.add_new_tab(&path, &content);
        }
    }

    pub fn handle_save_file(&self, window: &Window) {
        if let Some((buffer, path)) = self.workspace.get_current_buffer() {
            let content = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            if let Some(existing_path) = path {
                FileOps::save_file(content.to_string(), Some(existing_path), Some(window.clone()));
            } else {
                if let Some(new_path) = FileOps::save_file(content.to_string(), None, Some(window.clone())) {
                    self.workspace.update_current_tab_path(new_path);
                }
            }
        }
    }

    pub fn handle_save_as_file(&self, window: &Window) {
        if let Some((buffer, _)) = self.workspace.get_current_buffer() {
            let content = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            if let Some(new_path) = FileOps::save_file(content.to_string(), None, Some(window.clone())) {
                self.workspace.update_current_tab_path(new_path);
            }
        }
    }
} 