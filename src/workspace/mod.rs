pub mod controller;
pub mod manager;
pub mod recent;

use crate::editor::TextEditor;
use crate::file::ops::FileOps;
use crate::file::tree::FileTree;
use controller::WorkspaceController;
use gtk::prelude::*;
use gtk::{Box, Button, Frame, Label, Notebook, TextBuffer, TextView, Window};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct Workspace {
    frame: Frame,
    pub(crate) notebook: Notebook,
    pub(crate) open_files: Rc<RefCell<Vec<PathBuf>>>,
    // Index-aligned with open_files (and with notebook pages, once the
    // empty-state placeholder page is accounted for) — lets path updates
    // (rename, Save As) retarget an already-open tab's auto-save.
    text_editors: Rc<RefCell<Vec<TextEditor>>>,
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
            text_editors: Rc::new(RefCell::new(Vec::new())),
            controller,
            file_tree,
        };

        // Check if there are no open files and display the empty state
        if workspace.open_files.borrow().is_empty() {
            let empty_state = workspace.create_empty_state();
            notebook.append_page(&empty_state, Option::<&gtk::Widget>::None);
            notebook.set_show_tabs(false);
        }

        // Keep the status bar's word count pointed at whichever tab is active.
        let controller_for_switch = workspace.controller.clone();
        notebook.connect_switch_page(move |_, _, _| {
            controller_for_switch.refresh_word_count();
        });

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
        let (page_num, text_editor) = add_new_tab(&self.notebook, path, content, Some(controller));
        self.open_files.borrow_mut().push(path.to_path_buf());
        self.text_editors.borrow_mut().push(text_editor);

        // Refresh so the file's row picks up the "open" icon, then highlight it
        if let Some(ref file_tree) = self.file_tree {
            file_tree.refresh();
            file_tree.select_path(path);
        }

        // Ensure notebook shows tabs and has proper styling
        self.notebook.set_show_tabs(true);
        self.notebook.add_css_class("has-open-files");

        self.controller.refresh_word_count();

        page_num
    }

    /// Open `path` in the editor: focus its existing tab if already open,
    /// otherwise read it from disk and create a new tab.
    pub fn open_path(&self, path: PathBuf) {
        if path.is_dir() {
            return;
        }

        if let Some(index) = self.open_files.borrow().iter().position(|p| p == &path) {
            self.switch_to_tab(index);
            return;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            self.add_new_tab(&path, &content);
        }
    }

    pub fn get_current_buffer(&self) -> Option<(TextBuffer, Option<PathBuf>)> {
        let current_page = self.notebook.current_page()?;
        let page = self.notebook.nth_page(Some(current_page))?;
        let text_view = text_view_for_page(&page)?;

        // Get the buffer and path
        let buffer = text_view.buffer();
        let path = self.open_files.borrow().get(current_page as usize).cloned();

        // Return the tuple directly since we already handled the Option
        Some((buffer, path))
    }

    /// Save every open tab to its (already-known) path on disk.
    pub fn save_all(&self) {
        let paths = self.open_files.borrow().clone();
        for (index, path) in paths.iter().enumerate() {
            let Some(page) = self.notebook.nth_page(Some(index as u32)) else {
                continue;
            };
            let Some(text_view) = text_view_for_page(&page) else {
                continue;
            };
            let buffer = text_view.buffer();
            let content = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            if let Err(e) = fs::write(path, content.as_str()) {
                eprintln!("Failed to save {path:?}: {e}");
            }
        }

        if let Some(root) = self.controller.get_root_path() {
            crate::git::ops::stage_all_changes(&root);
        }
        if let Some(ref file_tree) = self.file_tree {
            file_tree.refresh();
        }
    }

    /// Reload every open tab's contents from disk, discarding any unsaved
    /// in-editor changes — useful after files changed outside the app.
    pub fn reload_all(&self) {
        let paths = self.open_files.borrow().clone();
        for (index, path) in paths.iter().enumerate() {
            let Some(page) = self.notebook.nth_page(Some(index as u32)) else {
                continue;
            };
            let Some(text_view) = text_view_for_page(&page) else {
                continue;
            };
            match fs::read_to_string(path) {
                Ok(content) => text_view.buffer().set_text(&content),
                Err(e) => eprintln!("Failed to reload {path:?}: {e}"),
            }
        }
    }

    pub fn update_current_tab_path(&self, new_path: PathBuf) {
        if let Some(current_page) = self.notebook.current_page() {
            self.set_tab_label(current_page, &new_path);

            if let Some(path) = self.open_files.borrow_mut().get_mut(current_page as usize) {
                *path = new_path.clone();
            }
            // Retarget auto-save to the new path (Save As)
            if let Some(editor) = self.text_editors.borrow().get(current_page as usize) {
                editor.set_path(new_path.clone());
            }

            // The file may be new on disk (Save As) — rebuild the tree to show it
            if let Some(ref file_tree) = self.file_tree {
                file_tree.refresh();
                file_tree.select_path(&new_path);
            }
        }
    }

    /// Rewrite `old_path` (a file, or a directory whose contents moved) to
    /// `new_path` for every open tab affected, keeping tab titles and saves
    /// pointed at the right place after a file-tree rename.
    pub fn rename_path(&self, old_path: &Path, new_path: &Path) {
        let affected: Vec<(u32, PathBuf)> = {
            let mut open_files = self.open_files.borrow_mut();
            let mut affected = Vec::new();
            for (index, path) in open_files.iter_mut().enumerate() {
                let updated = if path == old_path {
                    Some(new_path.to_path_buf())
                } else if let Ok(rel) = path.strip_prefix(old_path) {
                    Some(new_path.join(rel))
                } else {
                    None
                };

                if let Some(updated) = updated {
                    *path = updated.clone();
                    affected.push((index as u32, updated));
                }
            }
            affected
        };

        for (page_num, path) in affected {
            self.set_tab_label(page_num, &path);
            // Retarget auto-save to the new path
            if let Some(editor) = self.text_editors.borrow().get(page_num as usize) {
                editor.set_path(path);
            }
        }
    }

    /// Close any open tab pointed at `path` itself, or nested under it —
    /// used after a file-tree delete removes something out from under an
    /// open editor.
    pub fn close_paths_under(&self, path: &Path) {
        let indices: Vec<usize> = self
            .open_files
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, p)| *p == path || p.starts_with(path))
            .map(|(index, _)| index)
            .collect();

        // Remove from the highest index down so earlier indices stay valid
        for index in indices.into_iter().rev() {
            self.remove_tab(index);
        }
    }

    fn set_tab_label(&self, page_num: u32, path: &Path) {
        let Some(page) = self.notebook.nth_page(Some(page_num)) else {
            return;
        };
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return;
        };

        let tab_box = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(4)
            .build();

        let label = Label::new(Some(name));
        let close_button = Button::builder()
            .icon_name("window-close-symbolic")
            .css_classes(vec!["flat", "tab-close-button"])
            .build();

        tab_box.append(&label);
        tab_box.append(&close_button);

        let controller = self.controller.clone();
        close_button.connect_clicked(move |button| {
            if let Some(window) = button.root().and_downcast::<Window>() {
                controller.handle_close_tab(&window);
            }
        });

        self.notebook.set_tab_label(&page, Some(&tab_box));
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

        let new_file_box = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .halign(gtk::Align::Center)
            .build();

        new_file_box.set_css_classes(&["empty-file-box"]);

        let new_file_btn = Button::builder()
            .label("New file")
            .css_classes(vec!["empty-file-btn"])
            .build();

        let new_file_ctrl = Label::builder()
            .label("^N")
            .css_classes(vec!["empty-file-ctrl"])
            .build();

        new_file_box.append(&new_file_btn);
        new_file_box.append(&new_file_ctrl);

        let open_file_box = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .halign(gtk::Align::Center)
            .build();

        open_file_box.set_css_classes(&["empty-file-box"]);

        let open_file_btn = Button::builder()
            .label("Open file")
            .css_classes(vec!["empty-file-btn"])
            .build();

        let open_file_ctrl = Label::builder()
            .label("^O")
            .css_classes(vec!["empty-file-ctrl"])
            .build();

        open_file_box.append(&open_file_btn);
        open_file_box.append(&open_file_ctrl);

        let save_file_box = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .halign(gtk::Align::Center)
            .build();

        save_file_box.set_css_classes(&["empty-file-box"]);

        let save_file = Label::builder()
            .label("Save file")
            .css_classes(vec!["empty-file-btn"])
            .build();

        let save_file_ctrl = Label::builder()
            .label("^S")
            .css_classes(vec!["empty-file-ctrl"])
            .build();

        save_file_box.append(&save_file);
        save_file_box.append(&save_file_ctrl);

        let close_tab_box = Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .halign(gtk::Align::Center)
            .build();

        close_tab_box.set_css_classes(&["empty-file-box"]);

        let close_tab = Label::builder()
            .label("Close tab")
            .css_classes(vec!["empty-file-btn"])
            .build();

        let close_tab_ctrl = Label::builder()
            .label("^W")
            .css_classes(vec!["empty-file-ctrl"])
            .build();

        close_tab_box.append(&close_tab);
        close_tab_box.append(&close_tab_ctrl);

        // Clone the Rc pointers to avoid borrowing issues
        let controller_ref = self.controller.clone();

        new_file_btn.connect_clicked(move |_button| {
            controller_ref.handle_new_file();
        });

        let controller_ref = self.controller.clone();

        open_file_btn.connect_clicked(move |button| {
            if let Some(window) = button.root().and_downcast::<Window>()
                && let Some((path, content)) = FileOps::open_file(Some(window.clone()))
                && let Some(workspace) = controller_ref.get_workspace()
            {
                workspace.add_new_tab(&path, &content);
            }
        });

        empty_state.append(&new_file_box);
        empty_state.append(&open_file_box);
        empty_state.append(&save_file_box);
        empty_state.append(&close_tab_box);

        empty_state
    }

    pub fn remove_tab(&self, index: usize) {
        // Remove the tab
        self.notebook.remove_page(Some(index as u32));
        self.open_files.borrow_mut().remove(index);
        self.text_editors.borrow_mut().remove(index);

        // Show empty state if no more tabs
        if self.notebook.n_pages() == 0 {
            self.notebook.remove_css_class("has-open-files");
            let empty_state = self.create_empty_state();
            self.notebook
                .append_page(&empty_state, Option::<&gtk::Widget>::None);
            self.notebook.set_show_tabs(false);
        }

        // The closed file's row should drop back to the "not open" icon
        if let Some(ref file_tree) = self.file_tree {
            file_tree.refresh();
            if let Some((_, Some(path))) = self.get_current_buffer() {
                file_tree.select_path(&path);
            }
        }

        self.controller.refresh_word_count();
    }

    pub fn switch_to_tab(&self, index: usize) {
        if index < self.notebook.n_pages() as usize {
            self.notebook.set_current_page(Some(index as u32));
        }
    }

    /// Live-apply `settings` to every currently open tab — called after the
    /// settings dialog saves, so a toggle takes effect immediately instead
    /// of only for tabs opened afterward.
    pub fn apply_settings_to_open_tabs(&self, settings: &crate::setting::Settings) {
        for editor in self.text_editors.borrow().iter() {
            editor.apply_settings(settings);
        }
    }
}

fn add_new_tab(
    notebook: &Notebook,
    path: &Path,
    content: &str,
    controller: Option<Rc<WorkspaceController>>,
) -> (u32, TextEditor) {
    // Create text editor
    let text_editor = TextEditor::new();
    text_editor.set_text(content);
    // Set after set_text(): loading the initial content also fires the
    // buffer's "changed" signal, and we don't want that mistaken for a
    // real edit that needs auto-saving.
    text_editor.set_path(path.to_path_buf());

    // Create tab label box
    let tab_box = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .css_classes(vec!["tab-box"])
        .spacing(0)
        .build();

    // Create the label
    let label = Label::new(Some(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled"),
    ));

    // Create close button
    let close_button = Button::builder()
        .icon_name("window-close-symbolic")
        .css_classes(vec!["tab-close-button"])
        .build();

    tab_box.append(&label);
    tab_box.append(&close_button);

    // Add the page with our custom tab
    let page_num = notebook.append_page(text_editor.get_widget(), Some(&tab_box));

    // Connect close button signal
    if let Some(controller) = controller {
        // Only the active tab's edits should move the status bar's word
        // count — background tabs keep typing (autosave) without it.
        let notebook_for_word_count = notebook.clone();
        let controller_for_word_count = controller.clone();
        text_editor.connect_changed(move || {
            if notebook_for_word_count.current_page() == Some(page_num) {
                controller_for_word_count.refresh_word_count();
            }
        });

        close_button.connect_clicked(move |button| {
            if let Some(window) = button.root().and_downcast::<Window>() {
                controller.handle_close_tab(&window);
            }
        });
    }

    notebook.set_show_tabs(true);
    notebook.add_css_class("has-open-files");
    notebook.set_current_page(Some(page_num));

    (page_num, text_editor)
}

/// Descend from a notebook page's root widget to its actual TextView
/// (SourceView, which extends TextView). A tab page is
/// `Frame -> ScrolledWindow -> SourceView` (see TextEditor) — this needs
/// *two* `first_child()` hops, not one, since the ScrolledWindow itself
/// obviously isn't a TextView.
fn text_view_for_page(page: &gtk::Widget) -> Option<TextView> {
    page.first_child()?
        .first_child()?
        .downcast::<TextView>()
        .ok()
}
