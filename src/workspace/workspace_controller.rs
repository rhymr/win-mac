use crate::utils::file_ops::FileOps;
use gtk::prelude::*;
use gtk::Window;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use crate::workspace::workspace::Workspace;

pub struct WorkspaceController {
    pub(crate) workspace: RefCell<Option<Rc<Workspace>>>,
    root_path: RefCell<Option<PathBuf>>,
}

impl WorkspaceController {
    pub fn new() -> Self {
        Self {
            workspace: RefCell::new(None),
            root_path: RefCell::new(None),
        }
    }

    pub fn set_workspace(&self, workspace: Rc<Workspace>) {
        // Store the controller for use in tab operations
        self.workspace.replace(Some(workspace));
    }

    pub fn get_workspace(&self) -> Option<Rc<Workspace>> {
        self.workspace.borrow().clone()
    }

    /// Point the file tree at the loaded workspace folder.
    pub fn set_root_path(&self, path: PathBuf) {
        self.root_path.replace(Some(path.clone()));
        if let Some(workspace) = self.get_workspace() {
            if let Some(ref file_tree) = workspace.file_tree {
                file_tree.set_root_path(path);
            }
        }
    }

    pub fn get_root_path(&self) -> Option<PathBuf> {
        self.root_path.borrow().clone()
    }

    pub fn handle_new_file(&self) {
        let Some(workspace) = self.get_workspace() else {
            return;
        };
        let Some(root) = self.get_root_path() else {
            return;
        };

        // Create the file directly on disk in the workspace root — same
        // de-duplicated naming as the file tree's own "New File" — instead
        // of a disconnected "Untitled" tab that isn't a real file (and
        // doesn't show up in the tree) until an eventual Save As.
        let mut candidate = root.join("Untitled.txt");
        let mut n = 1;
        while candidate.exists() {
            n += 1;
            candidate = root.join(format!("Untitled {n}.txt"));
        }

        if let Err(e) = fs::write(&candidate, "") {
            eprintln!("Failed to create {candidate:?}: {e}");
            return;
        }

        workspace.add_new_tab(&candidate, "");
        if let Some(ref file_tree) = workspace.file_tree {
            file_tree.refresh();
            file_tree.select_path(&candidate);
        }
    }

    pub fn handle_new_folder(&self) {
        let Some(workspace) = self.get_workspace() else {
            return;
        };
        let Some(root) = self.get_root_path() else {
            return;
        };

        let mut candidate = root.join("New Folder");
        let mut n = 1;
        while candidate.exists() {
            n += 1;
            candidate = root.join(format!("New Folder {n}"));
        }

        if let Err(e) = fs::create_dir(&candidate) {
            eprintln!("Failed to create {candidate:?}: {e}");
            return;
        }

        if let Some(ref file_tree) = workspace.file_tree {
            file_tree.refresh();
            file_tree.select_path(&candidate);
        }
    }

    /// Close every open tab and re-point the workspace at a different
    /// folder — used by the Recent Projects menu.
    pub fn switch_workspace(&self, path: PathBuf) {
        if let Some(workspace) = self.get_workspace() {
            while !workspace.open_files.borrow().is_empty() {
                workspace.remove_tab(0);
            }
        }
        crate::utils::recent_workspaces::record_recent_workspace(&path);
        self.set_root_path(path);
    }

    pub fn handle_open_file(&self, window: &Window) {
        if let Some((path, content)) = FileOps::open_file(Some(window.clone())) {
            if let Some(workspace) = self.workspace.borrow().as_ref() {
                workspace.add_new_tab(&path, &content);
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