use adw::prelude::*;
use gtk::gio;

mod about;
mod output;
mod session;
mod theme;
mod ui;

const APP_ID: &str = "io.github.archisman_panigrahi.wombat";
fn main() {
    let app = adw::Application::builder().application_id(APP_ID).build();

    let quit_action = gio::SimpleAction::new("quit", None);
    {
        let app = app.clone();
        quit_action.connect_activate(move |_, _| {
            app.quit();
        });
    }
    app.add_action(&quit_action);
    app.set_accels_for_action("app.quit", &["<Primary>q"]);
    app.set_accels_for_action("win.toggle-fullscreen", &["F11"]);

    app.connect_activate(|app| {
        let window = ui::build_window(app);
        window.present();
    });
    app.run();
}
