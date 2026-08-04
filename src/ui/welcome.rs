use gio::prelude::FileExt;
use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Box, Button, CheckButton, FileDialog, Image, Label,
    ListBox, ListBoxRow, Orientation, SearchEntry,
};
use std::path::PathBuf;
use std::rc::Rc;
use crate::workspace::workspace_manager::WorkspaceManager;
use crate::tools::apple_notes::fetch_apple_notes;

/// Shows the workspace picker as the app's initial ApplicationWindow.
/// The main editor layout isn't built until a workspace is actually chosen.
pub fn show_welcome_dialog<F>(app: &Application, on_workspace_ready: F)
where
    F: Fn(PathBuf) + 'static,
{
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Welcome to Rhymr")
        .default_width(850)
        .default_height(540)
        .resizable(false)
        .build();

    let main_box = Box::new(Orientation::Horizontal, 0);

    // ==========================================
    // 1. LEFT SIDEBAR
    // ==========================================
    let sidebar = Box::new(Orientation::Vertical, 0);
    sidebar.set_width_request(200);
    sidebar.set_css_classes(&["welcome-sidebar"]);

    // App Branding Header
    let brand_box = Box::new(Orientation::Horizontal, 10);
    brand_box.set_margin_top(16);
    brand_box.set_margin_bottom(20);
    brand_box.set_margin_start(16);
    brand_box.set_margin_end(16);

    let logo_icon = Image::from_icon_name("edit-paste-symbolic");
    logo_icon.set_pixel_size(28);

    let title_vbox = Box::new(Orientation::Vertical, 0);
    let app_title = Label::builder()
        .label("Rhymr")
        .halign(Align::Start)
        .css_classes(vec!["title-3", "bold"])
        .build();
    let app_version = Label::builder()
        .label("2026.1")
        .halign(Align::Start)
        .css_classes(vec!["caption", "dim-label"])
        .build();

    title_vbox.append(&app_title);
    title_vbox.append(&app_version);
    brand_box.append(&logo_icon);
    brand_box.append(&title_vbox);

    // Sidebar Nav List
    let nav_list = ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(vec!["sidebar-nav"])
        .build();

    let nav_items = ["Projects", "Customize", "Plugins", "Learn"];
    for item in nav_items {
        let label = Label::builder()
            .label(item)
            .halign(Align::Start)
            .margin_start(16)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        nav_list.append(&label);
    }
    nav_list.select_row(nav_list.row_at_index(0).as_ref());

    // Settings Gear Button at Bottom
    let settings_btn = Button::builder()
        .icon_name("emblem-system-symbolic")
        .has_frame(false)
        .margin_start(12)
        .margin_bottom(12)
        .halign(Align::Start)
        .valign(Align::End)
        .vexpand(true)
        .build();

    sidebar.append(&brand_box);
    sidebar.append(&nav_list);
    sidebar.append(&settings_btn);

    // ==========================================
    // 2. MAIN CONTENT AREA (Projects View)
    // ==========================================
    let content_vbox = Box::new(Orientation::Vertical, 0);
    content_vbox.set_hexpand(true);
    content_vbox.set_css_classes(&["welcome-content"]);

    // Top Header Bar: Search + Actions
    let top_bar = Box::new(Orientation::Horizontal, 8);
    top_bar.set_margin_top(16);
    top_bar.set_margin_bottom(12);
    top_bar.set_margin_start(20);
    top_bar.set_margin_end(20);

    let search_entry = SearchEntry::builder()
        .placeholder_text("Search projects")
        .hexpand(true)
        .build();

    let new_btn = Button::builder()
        .label("New Workspace")
        .css_classes(vec!["suggested-action"])
        .build();

    let open_btn = Button::builder().label("Open").build();

    top_bar.append(&search_entry);
    top_bar.append(&new_btn);
    top_bar.append(&open_btn);

    // Toggle Bar for Apple Notes Import
    let options_bar = Box::new(Orientation::Horizontal, 0);
    options_bar.set_margin_start(20);
    options_bar.set_margin_end(20);
    options_bar.set_margin_bottom(12);

    let notes_toggle = CheckButton::builder()
        .label("Import Apple Notes on new workspace creation")
        .active(false)
        .build();

    options_bar.append(&notes_toggle);

    // Recent Projects List
    let projects_list = ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .margin_start(20)
        .margin_end(20)
        .margin_bottom(20)
        .vexpand(true)
        .css_classes(vec!["recent-projects-list"])
        .build();

    let default_path = dirs::home_dir()
        .map(|p| p.join("RhymrWorkspace"))
        .unwrap_or_else(|| PathBuf::from("./RhymrWorkspace"));

    let default_path_str = default_path.to_string_lossy().to_string();

    let sample_workspaces = vec![
        ("CYBERNETIC Workspace", "~/Lyrics/Cybernetic"),
        ("ZOMBIES Drafts", "~/Lyrics/Zombies"),
        ("Rhymr Workspace", default_path_str.as_str()),
    ];

    for (name, path) in sample_workspaces {
        let row = ListBoxRow::new();
        let hbox = Box::new(Orientation::Horizontal, 12);
        hbox.set_margin_top(10);
        hbox.set_margin_bottom(10);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);

        // Avatar icon with first letter
        let initial = name.chars().next().unwrap_or('W').to_string();
        let avatar_label = Label::builder()
            .label(&initial)
            .css_classes(vec!["project-avatar"])
            .width_request(32)
            .height_request(32)
            .build();

        let details_vbox = Box::new(Orientation::Vertical, 2);
        let name_label = Label::builder()
            .label(name)
            .halign(Align::Start)
            .css_classes(vec!["bold"])
            .build();
        let path_label = Label::builder()
            .label(path)
            .halign(Align::Start)
            .css_classes(vec!["dim-label", "caption"])
            .build();

        details_vbox.append(&name_label);
        details_vbox.append(&path_label);

        hbox.append(&avatar_label);
        hbox.append(&details_vbox);
        row.set_child(Some(&hbox));

        unsafe {
            row.set_data("workspace_path", PathBuf::from(path));
        }

        projects_list.append(&row);
    }

    content_vbox.append(&top_bar);
    content_vbox.append(&options_bar);
    content_vbox.append(&projects_list);

    main_box.append(&sidebar);
    main_box.append(&content_vbox);

    window.set_child(Some(&main_box));

    // ==========================================
    // 3. ACTION HANDLERS & LOGIC
    // ==========================================
    let dialog_clone = window.clone();
    let parent_clone = window.clone();
    let on_workspace_ready = Rc::new(on_workspace_ready);

    // Open Workspace (Choose existing folder via FileDialog)
    let callback_open = on_workspace_ready.clone();
    let dlg_open = dialog_clone.clone();
    open_btn.connect_clicked(move |_| {
        let file_dialog = FileDialog::new();
        file_dialog.set_title("Open Workspace Folder");

        let dlg_inner = dlg_open.clone();
        let cb_inner = callback_open.clone();

        file_dialog.select_folder(Some(&parent_clone), gio::Cancellable::NONE, move |result| {
            if let Ok(folder) = result {
                if let Some(path) = folder.path() {
                    dlg_inner.close();
                    cb_inner(path);
                }
            }
        });
    });

    // New Workspace Button Handler
    let callback_new = on_workspace_ready.clone();
    let dlg_new = dialog_clone.clone();
    let toggle_for_new = notes_toggle.clone();

    new_btn.connect_clicked(move |_| {
        let default_workspace = dirs::home_dir()
            .map(|p| p.join("RhymrWorkspace"))
            .unwrap_or_else(|| PathBuf::from("./RhymrWorkspace"));

        if let Ok(manager) = WorkspaceManager::init_workspace(&default_workspace, true) {
            if toggle_for_new.is_active() {
                if let Ok(notes) = fetch_apple_notes() {
                    for (title, body) in notes {
                        let note_path = manager.root_path.join(format!("{}.txt", title));
                        let _ = std::fs::write(note_path, body);
                    }
                    let git = crate::tools::git_ops::GitController::new(&manager.root_path);
                    let _ = git.commit_all("Initial import from Apple Notes");
                }
            }
            dlg_new.close();
            callback_new(manager.root_path);
        }
    });

    // Recent Projects Row Selection Handler
    let callback_row = on_workspace_ready.clone();
    let dlg_row = dialog_clone.clone();
    projects_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            unsafe {
                if let Some(path_ptr) = row.data::<PathBuf>("workspace_path") {
                    let path = path_ptr.as_ref().clone();
                    dlg_row.close();
                    callback_row(path);
                }
            }
        }
    });

    window.present();
}