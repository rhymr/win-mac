use gtk::prelude::*;
use gtk::{gio, Application};

use crate::ui::workspace_controller::WorkspaceController;
use std::rc::Rc;

pub fn setup_menu(app: &Application, workspace_controller: Rc<WorkspaceController>) {
    let file_menu = gio::Menu::new();
    file_menu.append(Some("New"), Some("app.new"));
    file_menu.append(Some("Open..."), Some("app.open"));
    file_menu.append(Some("Save"), Some("app.save"));
    file_menu.append(Some("Save As..."), Some("app.save-as"));
    file_menu.append(Some("Close Tab"), Some("app.close-tab"));
    
    let menu_bar = gio::Menu::new();
    menu_bar.append_submenu(Some("File"), &file_menu);
    
    app.set_menubar(Some(&menu_bar));

    // New file action
    let controller = workspace_controller.clone();
    let app_weak = app.downgrade();
    let new_action = gio::SimpleAction::new("new", None);
    new_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            if let Some(window) = app.active_window() {
                controller.handle_new_file(Some(&window));
            }
        }
    });
    app.add_action(&new_action);

    // Open file action
    let controller = workspace_controller.clone();
    let app_weak = app.downgrade();
    let open_action = gio::SimpleAction::new("open", None);
    open_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            if let Some(window) = app.active_window() {
                controller.handle_open_file(&window);
            }
        }
    });
    app.add_action(&open_action);

    // Save action
    let controller = workspace_controller.clone();
    let app_weak = app.downgrade();
    let save_action = gio::SimpleAction::new("save", None);
    save_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            if let Some(window) = app.active_window() {
                controller.handle_save_file(&window);
            }
        }
    });
    app.add_action(&save_action);

    // Save As action
    let controller = workspace_controller.clone();
    let app_weak = app.downgrade();
    let save_as_action = gio::SimpleAction::new("save-as", None);
    save_as_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            if let Some(window) = app.active_window() {
                controller.handle_save_as_file(&window);
            }
        }
    });
    app.add_action(&save_as_action);

    // Add close tab action
    let controller = workspace_controller.clone();
    let app_weak = app.downgrade();
    let close_action = gio::SimpleAction::new("close-tab", None);
    close_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            if let Some(window) = app.active_window() {
                controller.handle_close_tab(&window);
            }
        }
    });
    app.add_action(&close_action);

    // Add keyboard accelerators
    app.set_accels_for_action("app.new", &["<Primary>n"]);
    app.set_accels_for_action("app.open", &["<Primary>o"]);
    app.set_accels_for_action("app.save", &["<Primary>s"]);
    app.set_accels_for_action("app.save-as", &["<Primary><Shift>s"]);
    app.set_accels_for_action("app.close-tab", &["<Primary>w"]);
} 