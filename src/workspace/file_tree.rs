use crate::workspace::workspace::Workspace;
use gtk::gdk;
use gtk::glib;
use gtk::pango;
use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Entry, EventControllerFocus, EventControllerKey, Frame, GestureClick,
    Image, Label, ListBox, ListBoxRow, Orientation, Popover, ScrolledWindow,
};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const IGNORED_ENTRIES: [&str; 2] = ["target", "node_modules"];
const INDENT_PX: i32 = 20;

#[derive(Clone)]
pub struct FileTree {
    frame: Frame,
    file_list: ListBox,
    // Shared via Rc<RefCell<..>> (rather than a plain Option) so that every
    // clone of FileTree — including the one Workspace holds, made before
    // set_workspace() ever runs — observes the same value once it's set.
    workspace: Rc<RefCell<Option<Rc<Workspace>>>>,
    root_path: Rc<RefCell<Option<PathBuf>>>,
    entries: Rc<RefCell<Vec<(PathBuf, bool)>>>,
    collapsed: Rc<RefCell<HashSet<PathBuf>>>,
    // Decoded once and shared as *paintable data*, not as widgets: a GTK
    // widget can only ever have one parent, so reusing the same Image
    // widget instance across rows silently reparents it away from whichever
    // row last claimed it, leaving every other row blank. Each row instead
    // builds its own Image from the shared paintable, which is cheap and
    // avoids re-decoding the SVG per row.
    folder_icon: Option<gdk::Paintable>,
    note_active_icon: Option<gdk::Paintable>,
    note_inactive_icon: Option<gdk::Paintable>,
}

impl FileTree {
    pub fn new() -> Self {
        // Create a ListBox to display the workspace files
        let file_list = ListBox::builder()
            .css_classes(vec!["file-list"])
            .selection_mode(gtk::SelectionMode::Single)
            .build();

        // Keep the list bounded to the pane's height instead of growing forever
        let scrolled_list = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&file_list)
            .build();

        // Create a box to hold both the label and list with consistent background
        let inner_container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .css_classes(vec!["file-tree-inner"])
            .build();

        inner_container.append(&scrolled_list);

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

        Self {
            frame,
            file_list,
            workspace: Rc::new(RefCell::new(None)),
            root_path: Rc::new(RefCell::new(None)),
            entries: Rc::new(RefCell::new(Vec::new())),
            collapsed: Rc::new(RefCell::new(HashSet::new())),
            folder_icon: Image::from_resource("/org/gtk_rs/rhymr/icons/folder.svg").paintable(),
            note_active_icon: Image::from_resource("/org/gtk_rs/rhymr/icons/note-active.svg").paintable(),
            note_inactive_icon: Image::from_resource("/org/gtk_rs/rhymr/icons/note-inactive.svg").paintable(),
        }
    }

    pub fn set_workspace(&mut self, workspace: Rc<Workspace>) {
        self.workspace.replace(Some(workspace.clone()));

        // Open the file backing a selected row (directories aren't selectable)
        let entries = self.entries.clone();
        let workspace_ref = workspace.clone();
        self.file_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let index = row.index();
                // Extract into an owned value first: `open_path` re-enters
                // FileTree::refresh(), which needs its own mutable borrow of
                // `entries` — holding this borrow across that call (as an
                // `if let` scrutinee would, via temporary lifetime extension)
                // panics with "already borrowed".
                let entry = entries.borrow().get(index as usize).cloned();
                if let Some((path, _)) = entry {
                    workspace_ref.open_path(path);
                }
            }
        });

        // Keep the tree selection in sync with the active editor tab
        let file_tree_for_sync = self.clone();
        workspace.notebook.connect_switch_page(move |_, _, page_num| {
            if let Some(workspace) = file_tree_for_sync.workspace.borrow().as_ref() {
                if let Some(path) = workspace.open_files.borrow().get(page_num as usize).cloned() {
                    file_tree_for_sync.select_path(&path);
                }
            }
        });
    }

    /// Point the tree at a workspace folder and populate it from disk.
    pub fn set_root_path(&self, path: PathBuf) {
        self.root_path.replace(Some(path));
        self.collapsed.borrow_mut().clear();
        self.refresh();
    }

    /// Re-walk the workspace folder and rebuild the displayed rows.
    pub fn refresh(&self) {
        // Clear existing items
        let mut child = self.file_list.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            self.file_list.remove(&widget);
        }
        self.entries.borrow_mut().clear();

        let Some(root) = self.root_path.borrow().clone() else {
            return;
        };

        // The workspace root itself is the first, always-visible row; its
        // children (the real entries) collapse/expand underneath it.
        let root_row = self.build_row(&root, true, 0, true);
        self.file_list.append(&root_row);
        self.entries.borrow_mut().push((root.clone(), true));

        let collapsed = self.collapsed.borrow();
        if !collapsed.contains(&root) {
            let mut collected = Vec::new();
            collect_entries(&root, 1, &collapsed, &mut collected);

            for (path, is_dir, depth) in collected {
                let row = self.build_row(&path, is_dir, depth, false);
                self.file_list.append(&row);
                self.entries.borrow_mut().push((path, is_dir));
            }
        }
    }

    /// Highlight the row backing `path`, if it is currently shown.
    pub fn select_path(&self, path: &Path) {
        if let Some(index) = self.entries.borrow().iter().position(|(p, _)| p == path) {
            if let Some(row) = self.file_list.row_at_index(index as i32) {
                self.file_list.select_row(Some(&row));
            }
        }
    }

    pub fn get_widget(&self) -> &Frame {
        &self.frame
    }

    fn build_row(&self, path: &Path, is_dir: bool, depth: usize, is_root: bool) -> ListBoxRow {
        let hbox = GtkBox::new(Orientation::Horizontal, 0);
        hbox.set_valign(Align::Center);
        hbox.set_css_classes(&["file-list-row"]);
        hbox.set_margin_start(INDENT_PX * depth as i32);
        if is_root {
            hbox.add_css_class("file-tree-header");
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Workspace")
            .to_string();

        let name_label = if is_dir {
            // Expand/collapse indicator, separate from the folder icon itself
            let expanded = !self.collapsed.borrow().contains(path);
            let glyph = if expanded { "\u{25BE}" } else { "\u{25B8}" };
            let chevron = Label::new(Some(glyph));
            chevron.set_css_classes(&["dir-chevron"]);
            hbox.append(&chevron);

            let icon = Image::from_paintable(self.folder_icon.as_ref());
            icon.set_pixel_size(14);
            icon.set_css_classes(&["file-icon"]);
            hbox.append(&icon);

            let label = Label::new(Some(&name));
            label.set_css_classes(&["dir-label"]);
            hbox.append(&label);
            label
        } else {
            // Files have no chevron, but reserve the same slot for one so
            // file icons line up in a column with their sibling folders'
            // icons instead of sitting flush against the indent.
            let chevron_spacer = Label::new(None);
            chevron_spacer.set_css_classes(&["dir-chevron"]);
            hbox.append(&chevron_spacer);

            let is_open = self
                .workspace
                .borrow()
                .as_ref()
                .map(|w| w.open_files.borrow().iter().any(|p| p == path))
                .unwrap_or(false);

            let paintable = if is_open { &self.note_active_icon } else { &self.note_inactive_icon };
            let icon = Image::from_paintable(paintable.as_ref());
            icon.set_pixel_size(14);
            icon.set_css_classes(&["file-icon"]);
            hbox.append(&icon);

            let label = Label::new(Some(&name));
            hbox.append(&label);
            label
        };

        if is_root {
            if let Some(path_str) = path.to_str() {
                let path_label = Label::new(Some(path_str));
                path_label.set_css_classes(&["dir-path"]);
                path_label.set_hexpand(true);
                path_label.set_halign(Align::End);
                path_label.set_ellipsize(pango::EllipsizeMode::Start);
                hbox.append(&path_label);
            }
        }

        // Left click on a directory (including the root) toggles it collapsed/expanded
        if is_dir {
            let toggle_click = GestureClick::new();
            toggle_click.set_button(1);
            let file_tree_ref = self.clone();
            let path_owned = path.to_path_buf();
            toggle_click.connect_released(move |_, _, _, _| {
                let mut collapsed = file_tree_ref.collapsed.borrow_mut();
                if !collapsed.remove(&path_owned) {
                    collapsed.insert(path_owned.clone());
                }
                drop(collapsed);
                file_tree_ref.refresh();
            });
            hbox.add_controller(toggle_click);
        }

        // Right click opens a context menu to rename (not offered for the workspace root)
        if !is_root {
            let context_click = GestureClick::new();
            context_click.set_button(3);
            let file_tree_ref = self.clone();
            let path_owned = path.to_path_buf();
            let hbox_ref = hbox.clone();
            let label_ref = name_label.clone();
            context_click.connect_pressed(move |_, _, x, y| {
                file_tree_ref.show_context_menu(&hbox_ref, &label_ref, path_owned.clone(), x, y);
            });
            hbox.add_controller(context_click);
        }

        let row = ListBoxRow::new();
        row.set_selectable(!is_dir);
        row.set_child(Some(&hbox));
        row
    }

    fn show_context_menu(&self, hbox: &GtkBox, label: &Label, path: PathBuf, x: f64, y: f64) {
        let popover = Popover::new();
        popover.set_parent(hbox);
        popover.set_has_arrow(false);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));

        let menu_box = GtkBox::new(Orientation::Vertical, 0);
        let rename_button = gtk::Button::builder()
            .label("Rename")
            .css_classes(vec!["flat", "context-menu-item"])
            .build();
        menu_box.append(&rename_button);
        popover.set_child(Some(&menu_box));

        let file_tree_ref = self.clone();
        let hbox_ref = hbox.clone();
        let label_ref = label.clone();
        let popover_ref = popover.clone();
        rename_button.connect_clicked(move |_| {
            popover_ref.popdown();
            file_tree_ref.start_rename(&hbox_ref, &label_ref, path.clone());
        });

        let popover_for_close = popover.clone();
        popover.connect_closed(move |_| {
            popover_for_close.unparent();
        });

        popover.popup();
    }

    fn start_rename(&self, hbox: &GtkBox, label: &Label, path: PathBuf) {
        let current_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

        let entry = Entry::new();
        entry.set_text(&current_name);
        entry.set_hexpand(true);
        entry.set_css_classes(&["rename-entry"]);

        hbox.insert_child_after(&entry, Some(label));
        hbox.remove(label);

        entry.grab_focus();
        entry.select_region(0, -1);

        // Guards against committing twice (e.g. Enter immediately followed by focus-out)
        let committed = Rc::new(Cell::new(false));

        let file_tree_ref = self.clone();
        let path_ref = path.clone();
        let committed_ref = committed.clone();
        entry.connect_activate(move |entry| {
            if committed_ref.replace(true) {
                return;
            }
            file_tree_ref.commit_rename(path_ref.clone(), entry.text().to_string());
        });

        let file_tree_ref = self.clone();
        let path_ref = path.clone();
        let committed_ref = committed.clone();
        let focus_controller = EventControllerFocus::new();
        focus_controller.connect_leave(move |controller| {
            if committed_ref.replace(true) {
                return;
            }
            if let Some(entry) = controller.widget().and_downcast::<Entry>() {
                file_tree_ref.commit_rename(path_ref.clone(), entry.text().to_string());
            }
        });
        entry.add_controller(focus_controller);

        let file_tree_ref = self.clone();
        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gdk::Key::Escape {
                committed.set(true);
                file_tree_ref.refresh();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        entry.add_controller(key_controller);
    }

    fn commit_rename(&self, old_path: PathBuf, new_name: String) {
        let new_name = new_name.trim();
        if !new_name.is_empty() && !new_name.contains('/') {
            if let Some(parent) = old_path.parent() {
                let new_path = parent.join(new_name);
                if new_path != old_path {
                    match fs::rename(&old_path, &new_path) {
                        Ok(()) => {
                            if let Some(workspace) = self.workspace.borrow().as_ref() {
                                workspace.rename_path(&old_path, &new_path);
                            }

                            let mut collapsed = self.collapsed.borrow_mut();
                            if collapsed.remove(&old_path) {
                                collapsed.insert(new_path);
                            }
                        }
                        Err(e) => eprintln!("Failed to rename {old_path:?} to {new_path:?}: {e}"),
                    }
                }
            }
        }

        self.refresh();
    }
}

fn collect_entries(
    dir: &Path,
    depth: usize,
    collapsed: &HashSet<PathBuf>,
    entries: &mut Vec<(PathBuf, bool, usize)>,
) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };

    let mut items: Vec<_> = read_dir.filter_map(|entry| entry.ok()).collect();
    items.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.file_name().to_ascii_lowercase().cmp(&b.file_name().to_ascii_lowercase()),
        }
    });

    for item in items {
        let name = item.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || IGNORED_ENTRIES.contains(&name_str.as_ref()) {
            continue;
        }

        let path = item.path();
        let is_dir = path.is_dir();
        entries.push((path.clone(), is_dir, depth));

        if is_dir && !collapsed.contains(&path) {
            collect_entries(&path, depth + 1, collapsed, entries);
        }
    }
}
