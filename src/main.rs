use adw::prelude::*;
use gtk::gio;
use std::env;

mod about;
mod output;
mod session;
mod ui;

const APP_ID: &str = "io.github.archisman_panigrahi.wombat";
fn main() {
    configure_low_memory_runtime();

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
        trim_allocations_after_startup();
    });
    app.run();
}

fn configure_low_memory_runtime() {
    set_default_env("GSK_RENDERER", "cairo");
}

fn set_default_env(key: &str, value: &str) {
    if env::var_os(key).is_none() {
        env::set_var(key, value);
    }
}

fn trim_allocations_after_startup() {
    gtk::glib::idle_add_local_once(trim_allocations);
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn trim_allocations() {
    unsafe {
        malloc_trim(0);
    }
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn trim_allocations() {}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
extern "C" {
    fn malloc_trim(pad: usize) -> i32;
}
