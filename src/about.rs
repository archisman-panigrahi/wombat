use adw::prelude::*;

pub fn build_about_page() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.set_margin_top(24);
    page.set_margin_bottom(24);
    page.set_margin_start(24);
    page.set_margin_end(24);

    let heading = adw::StatusPage::builder()
        .icon_name("applications-science-symbolic")
        .title("About Wombat")
        .description("A mobile-friendly libadwaita front-end for Numbat.")
        .build();

    let credits_group = adw::PreferencesGroup::builder()
        .title("Credits")
        .description("Acknowledgements and upstream projects")
        .build();

    let numbat_row = adw::ActionRow::builder()
        .title("Numbat")
        .subtitle("Created by David Peter")
        .build();
    let numbat_link = gtk::LinkButton::with_label("https://github.com/sharkdp", "github.com/sharkdp");
    numbat_row.add_suffix(&numbat_link);
    numbat_row.set_activatable_widget(Some(&numbat_link));
    credits_group.add(&numbat_row);

    let wombat_row = adw::ActionRow::builder()
        .title("Wombat")
        .subtitle("GUI shell built with Rust, GTK4, and libadwaita")
        .build();
    credits_group.add(&wombat_row);

    let archisman_row = adw::ActionRow::builder()
        .title("Archisman Panigrahi")
        .subtitle("Creator of Wombat")
        .build();
    let archisman_link =
        gtk::LinkButton::with_label("https://github.com/archisman-panigrahi", "github.com/archisman");
    archisman_row.add_suffix(&archisman_link);
    archisman_row.set_activatable_widget(Some(&archisman_link));
    credits_group.add(&archisman_row);

    let copilot_row = adw::ActionRow::builder()
        .title("GitHub Copilot")
        .subtitle("Thanks for development assistance")
        .build();
    credits_group.add(&copilot_row);

    let docs_group = adw::PreferencesGroup::builder()
        .title("Resources")
        .description("Learn more")
        .build();

    let docs_row = adw::ActionRow::builder()
        .title("Numbat Documentation")
        .subtitle("numbat.dev/docs")
        .build();
    let docs_link = gtk::LinkButton::with_label("https://numbat.dev/docs/", "Open");
    docs_row.add_suffix(&docs_link);
    docs_row.set_activatable_widget(Some(&docs_link));
    docs_group.add(&docs_row);

    page.append(&heading);
    page.append(&credits_group);
    page.append(&docs_group);
    page
}
