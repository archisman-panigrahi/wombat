use std::env;

// use gtk::prelude::DisplayExtManual;

const APP_ID: &str = "io.github.archisman_panigrahi.wombat";
const WEBSITE: &str = "https://github.com/archisman-panigrahi/wombat";
const ISSUE_TRACKER: &str = "https://github.com/archisman-panigrahi/wombat/issues";

pub fn build_about_dialog(_parent: &adw::ApplicationWindow) -> adw::AboutDialog {
    // let info = debug_info();

    let dialog = adw::AboutDialog::builder()
        .application_icon(APP_ID)
        .application_name("Wombat")
        .developer_name("Archisman Panigrahi")
        .developers(["Archisman Panigrahi https://github.com/archisman-panigrahi"])
        .artists(["Gemini AI and Archisman Panigrahi"])
        .designers(["Archisman Panigrahi https://github.com/archisman-panigrahi"])
        .comments("Graphical scientific calculator and unit converter based on Numbat programming language.")
        .version(env!("CARGO_PKG_VERSION"))
        .license_type(gtk::License::MitX11)
        .website(WEBSITE)
        .issue_url(ISSUE_TRACKER)
        .copyright("Copyright 2026 Archisman Panigrahi")
        // .debug_info(info)
        // .debug_info_filename("wombat-debug")
        .build();

    dialog.add_credit_section(
        Some("Thanks"),
        &[
            "David Peter for creating Numbat https://github.com/sharkdp",
            "GitHub Copilot for assisting in development",
        ],
    );

    dialog.add_link("Numbat", "https://github.com/numbat.dev");

    dialog
}

// fn debug_info() -> String {
//     let mut information = String::new();

//     information.push_str(&format!("{APP_ID}: {}\n", env!("CARGO_PKG_VERSION")));
//     information.push_str(&format!(
//         "Profile: {}\n",
//         env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string())
//     ));

//     if let Some(backend) = backend() {
//         information.push_str(&format!("Backend: {backend}\n"));
//     }

//     information.push_str("Libraries:\n");
//     information.push_str(&format!(
//         " - GTK: {}.{}.{}\n",
//         gtk::major_version(),
//         gtk::minor_version(),
//         gtk::micro_version()
//     ));
//     information.push_str(&format!(
//         " - Libadwaita: {}.{}.{}\n",
//         adw::major_version(),
//         adw::minor_version(),
//         adw::micro_version()
//     ));

//     information.push_str("Thanks:\n");
//     information.push_str(" - David Peter: https://github.com/sharkdp\n");
//     information.push_str(" - GitHub Copilot: https://github.com/features/copilot\n");
//     information.push_str(" - Numbat: https://github.com/numbat.dev\n");

//     information
// }

// fn backend() -> Option<&'static str> {
//     let display = gtk::gdk::Display::default()?;

//     Some(match display.backend() {
//         gtk::gdk::Backend::Wayland => "Wayland",
//         gtk::gdk::Backend::X11 => "X11",
//         gtk::gdk::Backend::Win32 => "Win32",
//         gtk::gdk::Backend::MacOS => "macOS",
//         gtk::gdk::Backend::Broadway => "Broadway",
//     })
// }
