use crate::workspace::workspace::Workspace;
use gtk::gdk;
use gtk::glib;
use gtk::pango;
use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Button, DragSource, DropTarget, Entry, EventControllerFocus,
    EventControllerKey, Frame, GestureClick, Image, Label, ListBox, ListBoxRow, Orientation,
    Popover, ScrolledWindow, Separator, Window,
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
    // Cut/Copy clipboard: the path, and whether it was a Cut (vs Copy).
    clipboard: Rc<RefCell<Option<(PathBuf, bool)>>>,
    // Decoded once and shared as *paintable data*, not as widgets: a GTK
    // widget can only ever have one parent, so reusing the same Image
    // widget instance across rows silently reparents it away from whichever
    // row last claimed it, leaving every other row blank. Each row instead
    // builds its own Image from the shared paintable, which is cheap and
    // avoids re-decoding the SVG per row.
    folder_icon: Option<gdk::Paintable>,
    file_icon: Option<gdk::Paintable>,
}

impl FileTree {
    pub fn new() -> Self {
        // Create a ListBox to display the workspace files
        let file_list = ListBox::builder()
            .css_classes(vec!["file-list"])
            .selection_mode(gtk::SelectionMode::Single)
            .build();

        // Keep the list bounded to the pane's height instead of growing
        // forever. Horizontal scrolling is disabled outright: without it,
        // a long name or deep nesting level would otherwise let the whole
        // tree shift sideways, misaligning every row's icons from the left
        // edge — wide content just clips instead.
        let scrolled_list = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
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
            clipboard: Rc::new(RefCell::new(None)),
            folder_icon: Image::from_resource("/org/gtk_rs/rhymr/icons/folder.svg").paintable(),
            file_icon: Image::from_resource("/org/gtk_rs/rhymr/icons/note-active.svg").paintable(),
        }
    }

    pub fn set_workspace(&mut self, workspace: Rc<Workspace>) {
        self.workspace.replace(Some(workspace.clone()));

        // Note: opening a file is handled by an explicit left-click gesture
        // on the row in build_row(), not by GTK's row-selected signal.
        // row-selected also fires for programmatic `select_row` calls (e.g.
        // highlighting a file after a move) and, apparently, for a
        // right-click — neither of those should open anything.

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
        self.clipboard.replace(None);
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
        // Extract into an owned value first: `select_row` below fires the
        // `row-selected` signal synchronously, whose handler can re-enter
        // `refresh()` (e.g. selecting a freshly moved, not-yet-open file
        // triggers `open_path` -> `add_new_tab` -> `refresh`). Holding this
        // borrow across that call (as an `if let` scrutinee would, via
        // temporary lifetime extension) panics with "already borrowed".
        let index = self.entries.borrow().iter().position(|(p, _)| p == path);
        if let Some(index) = index {
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
            // Files have no expand/collapse chevron, but the currently open
            // file gets one anyway (as an "active" indicator) so its slot
            // stays reserved either way and icons line up in a column with
            // their sibling folders' icons.
            let is_open = self
                .workspace
                .borrow()
                .as_ref()
                .map(|w| w.open_files.borrow().iter().any(|p| p == path))
                .unwrap_or(false);

            let chevron = Label::new(if is_open { Some("\u{25CA}") } else { None });
            chevron.set_css_classes(if is_open {
                &["dir-chevron", "active-chevron"]
            } else {
                &["dir-chevron"]
            });
            hbox.append(&chevron);

            let icon = Image::from_paintable(self.file_icon.as_ref());
            icon.set_pixel_size(14);
            icon.set_css_classes(&["file-icon"]);
            hbox.append(&icon);

            // .txt is the default/implied extension for a lyrics workspace,
            // so hide it in the tree — commit_rename() adds it back
            // automatically if the user doesn't type their own extension.
            let label = Label::new(Some(strip_txt_extension(&name)));
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
        } else {
            // Left click on a file opens it. This is deliberately its own
            // gesture rather than piggybacking on the ListBox's
            // `row-selected` signal — that signal also fires for
            // programmatic `select_row` calls (e.g. highlighting a file
            // after a drag/paste move) and, apparently, on right-click too,
            // both of which would otherwise open the file as a side effect.
            let open_click = GestureClick::new();
            open_click.set_button(1);
            let file_tree_ref = self.clone();
            let path_owned = path.to_path_buf();
            open_click.connect_released(move |_, _, _, _| {
                if let Some(workspace) = file_tree_ref.workspace.borrow().as_ref() {
                    workspace.open_path(path_owned.clone());
                }
            });
            hbox.add_controller(open_click);
        }

        // Right click opens a full context menu (New is offered everywhere,
        // including the workspace root; Cut/Copy/Rename/Delete are not,
        // since they don't make sense for the whole open workspace).
        let context_click = GestureClick::new();
        context_click.set_button(3);
        let file_tree_ref = self.clone();
        let path_owned = path.to_path_buf();
        let hbox_ref = hbox.clone();
        let label_ref = name_label.clone();
        context_click.connect_pressed(move |_, _, x, y| {
            file_tree_ref.show_context_menu(&hbox_ref, &label_ref, path_owned.clone(), is_dir, is_root, x, y);
        });
        hbox.add_controller(context_click);

        // Drag to move files/folders into a different folder (not the
        // workspace root itself — there's nowhere else to drop that).
        if !is_root {
            let drag_source = DragSource::new();
            drag_source.set_actions(gdk::DragAction::MOVE);
            let path_for_drag = path.to_path_buf();
            let hbox_for_drag = hbox.clone();
            drag_source.connect_prepare(move |_, _, _| {
                let value = path_for_drag.to_string_lossy().to_string().to_value();
                Some(gdk::ContentProvider::for_value(&value))
            });
            // Dim the row while it's being dragged, so it's clear what's moving
            drag_source.connect_drag_begin(move |_, _| {
                hbox_for_drag.add_css_class("dragging");
            });
            let hbox_for_drag_end = hbox.clone();
            drag_source.connect_drag_end(move |_, _, _| {
                hbox_for_drag_end.remove_css_class("dragging");
            });
            hbox.add_controller(drag_source);
        }

        // Directories (including the root) accept drops to move items into them
        if is_dir {
            let drop_target = DropTarget::new(glib::types::Type::STRING, gdk::DragAction::MOVE);

            // Highlight the folder currently under the pointer during a drag,
            // so it's obvious where a drop will land.
            let hbox_for_enter = hbox.clone();
            drop_target.connect_enter(move |_, _, _| {
                hbox_for_enter.add_css_class("drop-target-active");
                gdk::DragAction::MOVE
            });
            let hbox_for_leave = hbox.clone();
            drop_target.connect_leave(move |_| {
                hbox_for_leave.remove_css_class("drop-target-active");
            });

            let file_tree_ref = self.clone();
            let target_dir = path.to_path_buf();
            let hbox_for_drop = hbox.clone();
            drop_target.connect_drop(move |_, value, _, _| {
                hbox_for_drop.remove_css_class("drop-target-active");
                if let Ok(src) = value.get::<String>() {
                    file_tree_ref.move_into(PathBuf::from(src), target_dir.clone());
                    true
                } else {
                    false
                }
            });
            hbox.add_controller(drop_target);
        }

        let row = ListBoxRow::new();
        row.set_selectable(!is_dir);
        row.set_child(Some(&hbox));
        row
    }

    fn show_context_menu(
        &self,
        hbox: &GtkBox,
        label: &Label,
        path: PathBuf,
        is_dir: bool,
        is_root: bool,
        x: f64,
        y: f64,
    ) {
        let popover = Popover::new();
        popover.set_parent(hbox);
        popover.set_has_arrow(false);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));

        let menu_box = GtkBox::new(Orientation::Vertical, 0);
        menu_box.set_css_classes(&["context-menu"]);

        let target_dir = if is_dir {
            path.clone()
        } else {
            path.parent().map(Path::to_path_buf).unwrap_or_else(|| path.clone())
        };

        // New (opens a File/Folder submenu)
        let new_item = submenu_item_button("New");
        menu_box.append(&new_item);

        let popover_ref = popover.clone();
        let file_tree_ref = self.clone();
        let hbox_ref = hbox.clone();
        let target_dir_for_new = target_dir.clone();
        new_item.connect_clicked(move |_| {
            popover_ref.popdown();
            file_tree_ref.show_new_submenu(&hbox_ref, target_dir_for_new.clone());
        });

        if !is_root {
            menu_box.append(&Separator::new(Orientation::Horizontal));

            let cut_item = menu_item_button("Cut", Some("\u{2318}X"), None);
            let copy_item = menu_item_button("Copy", Some("\u{2318}C"), None);
            let copy_path_item = menu_item_button("Copy Path", Some("\u{21E7}\u{2318}C"), None);
            menu_box.append(&cut_item);
            menu_box.append(&copy_item);
            menu_box.append(&copy_path_item);

            let popover_ref = popover.clone();
            let clipboard_ref = self.clipboard.clone();
            let path_for_cut = path.clone();
            cut_item.connect_clicked(move |_| {
                popover_ref.popdown();
                clipboard_ref.replace(Some((path_for_cut.clone(), true)));
            });

            let popover_ref = popover.clone();
            let clipboard_ref = self.clipboard.clone();
            let path_for_copy = path.clone();
            copy_item.connect_clicked(move |_| {
                popover_ref.popdown();
                clipboard_ref.replace(Some((path_for_copy.clone(), false)));
            });

            let popover_ref = popover.clone();
            let path_for_copy_path = path.clone();
            copy_path_item.connect_clicked(move |_| {
                popover_ref.popdown();
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&path_for_copy_path.to_string_lossy());
                }
            });
        }

        if is_dir && self.clipboard.borrow().is_some() {
            let paste_item = menu_item_button("Paste", Some("\u{2318}V"), None);
            menu_box.append(&paste_item);

            let popover_ref = popover.clone();
            let file_tree_ref = self.clone();
            let target_dir_for_paste = path.clone();
            paste_item.connect_clicked(move |_| {
                popover_ref.popdown();
                file_tree_ref.paste_into(target_dir_for_paste.clone());
            });
        }

        if !is_root {
            let rename_item = menu_item_button("Rename", None, None);
            menu_box.append(&rename_item);

            let file_tree_ref = self.clone();
            let hbox_ref = hbox.clone();
            let label_ref = label.clone();
            let path_for_rename = path.clone();
            rename_item.connect_clicked(move |_| {
                // No popdown()-then-refresh() race here: renaming edits the
                // row's existing children in place rather than destroying
                // the row, so the popover's parent widget stays alive.
                file_tree_ref.start_rename(&hbox_ref, &label_ref, path_for_rename.clone());
            });

            menu_box.append(&Separator::new(Orientation::Horizontal));

            let delete_item = menu_item_button("Delete\u{2026}", Some("\u{232B}"), Some("destructive-menu-item"));
            menu_box.append(&delete_item);

            let popover_ref = popover.clone();
            let file_tree_ref = self.clone();
            let hbox_ref = hbox.clone();
            let path_for_delete = path.clone();
            delete_item.connect_clicked(move |_| {
                popover_ref.popdown();
                file_tree_ref.confirm_delete(&hbox_ref, path_for_delete.clone(), is_dir);
            });
        }

        popover.set_child(Some(&menu_box));

        let popover_for_close = popover.clone();
        popover.connect_closed(move |_| {
            popover_for_close.unparent();
        });

        popover.popup();
    }

    fn show_new_submenu(&self, hbox: &GtkBox, parent_dir: PathBuf) {
        let popover = Popover::new();
        popover.set_parent(hbox);
        popover.set_has_arrow(false);

        let menu_box = GtkBox::new(Orientation::Vertical, 0);
        menu_box.set_css_classes(&["context-menu"]);
        let file_item = menu_item_button("New File", None, None);
        let folder_item = menu_item_button("New Folder", None, None);
        menu_box.append(&file_item);
        menu_box.append(&folder_item);
        popover.set_child(Some(&menu_box));

        let popover_ref = popover.clone();
        let file_tree_ref = self.clone();
        let dir_for_file = parent_dir.clone();
        file_item.connect_clicked(move |_| {
            popover_ref.popdown();
            file_tree_ref.create_new_entry(dir_for_file.clone(), false);
        });

        let popover_ref = popover.clone();
        let file_tree_ref = self.clone();
        folder_item.connect_clicked(move |_| {
            popover_ref.popdown();
            file_tree_ref.create_new_entry(parent_dir.clone(), true);
        });

        let popover_for_close = popover.clone();
        popover.connect_closed(move |_| {
            popover_for_close.unparent();
        });

        popover.popup();
    }

    /// Create a new file or folder inside `parent_dir` with a default,
    /// de-duplicated name, then immediately start renaming it in place so
    /// the user can type the real name right away.
    fn create_new_entry(&self, parent_dir: PathBuf, is_dir: bool) {
        let mut candidate = parent_dir.join(if is_dir { "New Folder" } else { "Untitled.txt" });
        let mut n = 1;
        while candidate.exists() {
            n += 1;
            candidate = parent_dir.join(if is_dir {
                format!("New Folder {n}")
            } else {
                format!("Untitled {n}.txt")
            });
        }

        let result = if is_dir { fs::create_dir(&candidate) } else { fs::write(&candidate, "") };
        if let Err(e) = result {
            eprintln!("Failed to create {candidate:?}: {e}");
            return;
        }

        self.collapsed.borrow_mut().remove(&parent_dir);

        // Deferred: this handler fires from a Popover menu-item click, and
        // refresh() destroys the row that popover is parented to — doing
        // that synchronously, before the popover has finished closing,
        // crashes GTK. Let the close finish first, then rebuild and start
        // the rename.
        let file_tree_ref = self.clone();
        glib::idle_add_local_once(move || {
            file_tree_ref.refresh();
            file_tree_ref.select_path(&candidate);
            file_tree_ref.begin_rename_at(&candidate);
        });
    }

    /// Locate the currently displayed row for `path` and start inline
    /// renaming on it, reusing the same flow as the context menu's Rename.
    fn begin_rename_at(&self, path: &Path) {
        let Some(index) = self.entries.borrow().iter().position(|(p, _)| p == path) else {
            return;
        };
        let Some(row) = self.file_list.row_at_index(index as i32) else {
            return;
        };
        let Some(hbox) = row.child().and_then(|w| w.downcast::<GtkBox>().ok()) else {
            return;
        };
        let Some(label) = hbox.last_child().and_then(|w| w.downcast::<Label>().ok()) else {
            return;
        };
        self.start_rename(&hbox, &label, path.to_path_buf());
    }

    fn start_rename(&self, hbox: &GtkBox, label: &Label, path: PathBuf) {
        let current_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        // Same .txt-hiding as the tree display — commit_rename() restores
        // it if the user doesn't type a different extension.
        let prefill = if path.is_dir() { current_name } else { strip_txt_extension(&current_name).to_string() };

        let entry = Entry::new();
        entry.set_text(&prefill);
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
                // No extension typed for a file → assume .txt, matching what
                // the tree display and rename prefill both hide.
                let final_name = if !old_path.is_dir() && !new_name.contains('.') {
                    format!("{new_name}.txt")
                } else {
                    new_name.to_string()
                };
                let new_path = parent.join(&final_name);
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

    /// Move `src` into `target_dir` — the single move implementation shared
    /// by drag-and-drop and Cut+Paste.
    fn move_into(&self, src: PathBuf, target_dir: PathBuf) {
        if !src.exists() || src.parent() == Some(target_dir.as_path()) {
            return;
        }
        // Refuse to move a directory into itself or one of its own descendants
        if target_dir == src || target_dir.starts_with(&src) {
            return;
        }
        let Some(name) = src.file_name() else {
            return;
        };
        let dest = target_dir.join(name);
        if dest.exists() {
            return;
        }

        match fs::rename(&src, &dest) {
            Ok(()) => {
                if let Some(workspace) = self.workspace.borrow().as_ref() {
                    workspace.rename_path(&src, &dest);
                }
                self.collapsed.borrow_mut().remove(&target_dir);

                // Deferred for the same reason as create_new_entry: this can
                // run from a drag-and-drop completing on a Popover-adjacent
                // widget or from Paste (a Popover menu-item click), and
                // refresh() would otherwise destroy widgets mid-transition.
                let file_tree_ref = self.clone();
                glib::idle_add_local_once(move || {
                    file_tree_ref.refresh();
                    file_tree_ref.select_path(&dest);
                });
            }
            Err(e) => eprintln!("Failed to move {src:?} to {dest:?}: {e}"),
        }
    }

    fn paste_into(&self, target_dir: PathBuf) {
        let Some((src, is_cut)) = self.clipboard.borrow().clone() else {
            return;
        };
        if !src.exists() {
            return;
        }
        if target_dir == src || target_dir.starts_with(&src) {
            return;
        }

        if is_cut {
            self.clipboard.replace(None);
            self.move_into(src, target_dir);
            return;
        }

        let Some(name) = src.file_name() else {
            return;
        };
        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("item");
        let ext = src.extension().and_then(|s| s.to_str());

        let mut dest = target_dir.join(name);
        let mut n = 1;
        while dest.exists() {
            n += 1;
            let candidate_name = match ext {
                Some(ext) => format!("{stem} {n}.{ext}"),
                None => format!("{stem} {n}"),
            };
            dest = target_dir.join(candidate_name);
        }

        match copy_recursive(&src, &dest) {
            Ok(()) => {
                self.collapsed.borrow_mut().remove(&target_dir);

                // Deferred: Paste is invoked from a Popover menu-item click.
                let file_tree_ref = self.clone();
                glib::idle_add_local_once(move || {
                    file_tree_ref.refresh();
                    file_tree_ref.select_path(&dest);
                });
            }
            Err(e) => eprintln!("Failed to copy {src:?} to {dest:?}: {e}"),
        }
    }

    fn confirm_delete(&self, hbox: &GtkBox, path: PathBuf, is_dir: bool) {
        let Some(window) = hbox.root().and_then(|r| r.downcast::<Window>().ok()) else {
            return;
        };
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("this item").to_string();
        let kind = if is_dir { "folder" } else { "file" };

        let dialog = Window::builder()
            .title("Delete")
            .transient_for(&window)
            .modal(true)
            .resizable(false)
            .default_width(360)
            .css_classes(vec!["new-project-content"])
            .build();

        let content = GtkBox::new(Orientation::Vertical, 16);
        content.set_margin_top(20);
        content.set_margin_bottom(20);
        content.set_margin_start(20);
        content.set_margin_end(20);

        let message = Label::new(Some(&format!("Delete {kind} \u{201C}{name}\u{201D}? This can\u{2019}t be undone.")));
        message.set_wrap(true);
        message.set_halign(Align::Start);

        let action_box = GtkBox::new(Orientation::Horizontal, 12);
        action_box.set_halign(Align::End);
        action_box.set_margin_top(8);

        let cancel_btn = Button::builder().label("Cancel").build();
        let delete_btn = Button::builder().label("Delete").css_classes(vec!["destructive-action"]).build();
        action_box.append(&cancel_btn);
        action_box.append(&delete_btn);

        content.append(&message);
        content.append(&action_box);
        dialog.set_child(Some(&content));

        let dialog_for_cancel = dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            dialog_for_cancel.close();
        });

        let dialog_for_delete = dialog.clone();
        let file_tree_ref = self.clone();
        delete_btn.connect_clicked(move |_| {
            dialog_for_delete.close();
            file_tree_ref.delete_path(path.clone());
        });

        dialog.present();
    }

    fn delete_path(&self, path: PathBuf) {
        let result = if path.is_dir() { fs::remove_dir_all(&path) } else { fs::remove_file(&path) };
        match result {
            Ok(()) => {
                if let Some(workspace) = self.workspace.borrow().as_ref() {
                    workspace.close_paths_under(&path);
                }
                self.collapsed.borrow_mut().remove(&path);
                self.refresh_deferred();
            }
            Err(e) => eprintln!("Failed to delete {path:?}: {e}"),
        }
    }

    /// Same as `refresh`, but deferred to the next main-loop iteration.
    /// Needed whenever the call originates from a Popover menu-item click:
    /// the popover is parented to the very row `refresh()` is about to
    /// destroy, and tearing that row down before the popover has finished
    /// its own close transition crashes GTK.
    fn refresh_deferred(&self) {
        let file_tree_ref = self.clone();
        glib::idle_add_local_once(move || {
            file_tree_ref.refresh();
        });
    }
}

/// A left-aligned, full-width flat menu row (icon-menu look, not a centered
/// button) with an optional right-aligned shortcut hint and an optional
/// extra CSS class (e.g. for Delete).
fn menu_item_button(text: &str, shortcut: Option<&str>, extra_class: Option<&str>) -> Button {
    let hbox = GtkBox::new(Orientation::Horizontal, 0);

    let label = Label::new(Some(text));
    label.set_halign(Align::Start);
    label.set_hexpand(true);
    hbox.append(&label);

    if let Some(shortcut) = shortcut {
        let hint = Label::new(Some(shortcut));
        hint.set_halign(Align::End);
        hint.set_css_classes(&["menu-shortcut"]);
        hbox.append(&hint);
    }

    let button = Button::new();
    button.set_child(Some(&hbox));
    button.set_halign(Align::Fill);

    let mut classes = vec!["flat", "context-menu-item"];
    if let Some(extra) = extra_class {
        classes.push(extra);
    }
    button.set_css_classes(&classes);
    button
}

/// Same as `menu_item_button`, but with a trailing "opens a submenu" arrow.
fn submenu_item_button(text: &str) -> Button {
    let hbox = GtkBox::new(Orientation::Horizontal, 0);

    let label = Label::new(Some(text));
    label.set_halign(Align::Start);
    label.set_hexpand(true);

    let arrow = Label::new(Some("\u{203A}"));
    arrow.set_halign(Align::End);
    arrow.set_css_classes(&["dim-label"]);

    hbox.append(&label);
    hbox.append(&arrow);

    let button = Button::new();
    button.set_child(Some(&hbox));
    button.set_halign(Align::Fill);
    button.set_css_classes(&["flat", "context-menu-item"]);
    button
}

/// Strip a trailing ".txt" (case-insensitively) — the implied default
/// extension in a plain-text lyrics workspace, hidden everywhere a file
/// name is shown or edited.
fn strip_txt_extension(name: &str) -> &str {
    if name.len() > 4 && name[name.len() - 4..].eq_ignore_ascii_case(".txt") {
        &name[..name.len() - 4]
    } else {
        name
    }
}

fn copy_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dest.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(src, dest).map(|_| ())
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
