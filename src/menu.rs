use gtk::prelude::*;
use gtk::{gio, Application};
use std::rc::Rc;
use gio::Menu;
use crate::workspace::workspace_controller::WorkspaceController;

pub fn setup_menu(app: &Application, workspace_controller: Rc<WorkspaceController>) {
    let file = file(app, workspace_controller.clone());
    // edit(app, workspace_controller);
    // other(app, workspace_controller);
    let help = help(app);

    // Create the menu bar and add menus
    let menu_bar = Menu::new();
    menu_bar.append_submenu(Some("File"), &file);
    menu_bar.append_submenu(Some("Help"), &help);

    // Set menu bar in app
    app.set_menubar(Some(&menu_bar));
}

fn help(app: &Application) -> Menu {
    let help_menu = Menu::new();
    
    // Documentation section
    let docs_section = gio::Menu::new();
    docs_section.append(Some("Documentation"), Some("app.docs"));
    docs_section.append(Some("Report Issue"), Some("app.report-issue")); 
    help_menu.insert_section(0, None, &docs_section);
    
    // About section
    let about_section = gio::Menu::new();
    about_section.append(Some("About"), Some("app.about"));
    help_menu.insert_section(1, None, &about_section);

    // Documentation action
    let docs_action = gio::SimpleAction::new("docs", None);
    docs_action.connect_activate(|_, _| {
        if let Err(err) = gio::AppInfo::launch_default_for_uri(
            "https://github.com/rhymr-rs/rhymr",
            None::<&gio::AppLaunchContext>
        ) {
            eprintln!("Failed to open docs: {}", err);
        }
    });
    app.add_action(&docs_action);

    // Report issue action 
    let report_action = gio::SimpleAction::new("report-issue", None);
    report_action.connect_activate(|_, _| {
        if let Err(err) = gtk::gio::AppInfo::launch_default_for_uri(
            "https://github.com/rhymr-rs/rhymr/issues",
            None::<&gio::AppLaunchContext>
        ) {
            eprintln!("Failed to open issue tracker: {}", err);
        }
    });
    app.add_action(&report_action);

    // About action
    let about_action = gio::SimpleAction::new("about", None);
    about_action.connect_activate(move |_, _| {
        /*if let Some(window) = app.active_window() {
            let dialog = gtk::AboutDialog::builder()
                .program_name("Rhymr")
                .version("0.1.0")
                .website("https://rhymr.app")
                .website_label("Visit Website")
                // .license_type(gtk::License::Custom)
                .authors(vec!["Rhymr Team".to_string()])
                .logo_icon_name("text-editor")
                .modal(true)
                .transient_for(&window)
                .build();

            dialog.present();
        }*/
    });
    app.add_action(&about_action);

    help_menu
}

pub fn file(app: &Application, workspace_controller: Rc<WorkspaceController>) -> Menu {
    let file_menu = gio::Menu::new();
    
    // File operations section
    let file_ops_section = gio::Menu::new();
    file_ops_section.append(Some("New"), Some("app.new"));
    file_ops_section.append(Some("Open..."), Some("app.open"));
    file_menu.insert_section(0, None, &file_ops_section);
    
    // Save operations section
    let save_ops_section = gio::Menu::new();
    save_ops_section.append(Some("Save"), Some("app.save"));
    save_ops_section.append(Some("Save As..."), Some("app.save-as"));
    file_menu.insert_section(1, None, &save_ops_section);
    
    // Tab operations section
    let tab_ops_section = gio::Menu::new();
    tab_ops_section.append(Some("Close Tab"), Some("app.close-tab"));
    file_menu.insert_section(2, None, &tab_ops_section);

    // New file action
    let controller = workspace_controller.clone();
    let new_action = gio::SimpleAction::new("new", None);
    new_action.connect_activate(move |_, _| {
        controller.handle_new_file();
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

    file_menu
} 