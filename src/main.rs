use adw::prelude::*;

mod about;
mod output;
mod session;
mod ui;

const APP_ID: &str = "io.github.archisman_panigrahi.wombat";
fn main() {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        let window = ui::build_window(app);
        window.present();
    });
    app.run();
}
