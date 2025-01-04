use gtk::prelude::*;
use gtk::{Application, Orientation, Paned};

pub fn build_ui(app: &Application) {
    let css_provider = crate::ui::css::load_css("assets/css/dark.css");
    crate::ui::css::apply_css_to_app(&css_provider);

    let main_window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Rhymr")
        .default_width(1280)
        .default_height(720)
        .build();

    let main_layout = crate::ui::components::create_main_layout();
    main_window.set_child(Some(&main_layout));

    main_window.present();
}

pub fn create_horizontal_split(
    left: &impl IsA<gtk::Widget>,
    right: &impl IsA<gtk::Widget>,
    position: i32,
) -> Paned {
    let horizontal_pane = Paned::new(Orientation::Horizontal);
    horizontal_pane.set_start_child(Some(left));
    horizontal_pane.set_end_child(Some(right));
    horizontal_pane.set_position(position);

    horizontal_pane.set_resize_start_child(true);
    horizontal_pane.set_resize_end_child(true);
    horizontal_pane.set_shrink_start_child(false);
    horizontal_pane.set_shrink_end_child(false);

    horizontal_pane
}

pub fn create_vertical_split(
    top: &impl IsA<gtk::Widget>,
    bottom: &impl IsA<gtk::Widget>,
    position: i32,
) -> Paned {
    let vertical_pane = Paned::new(Orientation::Vertical);
    vertical_pane.set_start_child(Some(top));
    vertical_pane.set_end_child(Some(bottom));
    vertical_pane.set_position(position);

    vertical_pane.set_resize_start_child(true);
    vertical_pane.set_resize_end_child(true);
    vertical_pane.set_shrink_start_child(false);
    vertical_pane.set_shrink_end_child(false);

    vertical_pane
}