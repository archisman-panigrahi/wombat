const WEBSITE: &str = "https://archisman-panigrahi.github.io/wombat/";
const ISSUE_TRACKER: &str = "https://github.com/archisman-panigrahi/wombat/issues";

pub fn build_about_dialog() -> adw::AboutDialog {
    let dialog = adw::AboutDialog::builder()
        .application_icon(crate::APP_ID)
        .application_name("Wombat")
        .developer_name("Archisman Panigrahi")
        .developers(["Archisman Panigrahi https://github.com/archisman-panigrahi"])
        .artists(["Gemini AI, Archisman Panigrahi and gnoman"])
        .designers(["Archisman Panigrahi https://github.com/archisman-panigrahi"])
        .comments("High-precision scientific calculator with full support for physical units, powered by Numbat programming language.\n")
        .version(env!("CARGO_PKG_VERSION"))
        .license_type(gtk::License::Gpl30)
        .website(WEBSITE)
        .issue_url(ISSUE_TRACKER)
        .copyright("Copyright \u{00a9} 2026 Archisman Panigrahi")
        .build();

    dialog.add_legal_section(
        "\nNumbat",
        Some("Copyright \u{00a9} 2022-2026 David Peter, and all Numbat contributors"),
        gtk::License::Custom,
        Some("Released jointly under the <a href=\"https://opensource.org/licenses/MIT\">MIT</a> and <a href=\"https://www.apache.org/licenses/LICENSE-2.0\">Apache-2.0</a> licenses."),
    );

    dialog.add_credit_section(
        Some("Thanks"),
        &[
            "David Peter for creating Numbat and Numbat-app https://github.com/sharkdp",
            "Codex, Copilot and Claude for assisting in development",
        ],
    );

    dialog.add_link("Numbat", "https://numbat.dev");
    dialog.add_link(
        "Numbat-app (for Android and iOS)",
        "https://github.com/sharkdp/numbat-app",
    );

    dialog
}
