use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::gio;

use crate::about::build_about_dialog;
use crate::output::{append_history, ensure_numbat_tags, set_startup_message};
use crate::session::NumbatSession;

const HISTORY_MARGIN: i32 = 8;
const INPUT_MARGIN: i32 = 12;
const NUMBAT_SYNTAX_URL: &str = "https://numbat.dev/docs/examples/example-numbat_syntax/";
const NUMBAT_EXAMPLES_URL: &str = "https://numbat.dev/docs/basics/conversions/";
const SIDEBAR_DESKTOP_MAX_WIDTH: f64 = 280.0;
const SIDEBAR_MOBILE_MAX_WIDTH: f64 = 280.0;
const STARTUP_BANNER_LARGE: &str =
r#"
██╗    ██╗ ██████╗ ███╗   ███╗██████╗  █████╗ ████████╗
██║    ██║██╔═══██╗████╗ ████║██╔══██╗██╔══██╗╚══██╔══╝
██║ █╗ ██║██║   ██║██╔████╔██║██████╔╝███████║   ██║
██║███╗██║██║   ██║██║╚██╔╝██║██╔══██╗██╔══██║   ██║
╚███╔███╔╝╚██████╔╝██║ ╚═╝ ██║██████╔╝██║  ██║   ██║
 ╚══╝╚══╝  ╚═════╝ ╚═╝     ╚═╝╚═════╝ ╚═╝  ╚═╝   ╚═╝"#;

const STARTUP_BANNER_SMALL: &str =
r#"
░▒█░░▒█░▒█▀▀▀█░▒█▀▄▀█░▒█▀▀▄░█▀▀▄░▀▀█▀▀
░▒█▒█▒█░▒█░░▒█░▒█▒█▒█░▒█▀▀▄▒█▄▄█░░▒█░░
░▒▀▄▀▄▀░▒█▄▄▄█░▒█░░▒█░▒█▄▄█▒█░▒█░░▒█░░
"#;

const BANNER_SWITCH_WIDTH: i32 = 500;

pub fn build_window(app: &adw::Application) -> adw::ApplicationWindow {
    let session = Rc::new(RefCell::new(NumbatSession::new()));
    let command_history: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let history_cursor: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let draft_input: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let showing_startup: Rc<RefCell<bool>> = Rc::new(RefCell::new(true));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(650)
        .default_height(640)
        .title("Wombat")
        .build();

    let header = adw::HeaderBar::new();
    let title = gtk::Label::new(Some("Wombat"));
    title.add_css_class("title-3");
    header.set_title_widget(Some(&title));

    let overlay_split_view = adw::OverlaySplitView::new();
    overlay_split_view.set_enable_show_gesture(true);
    overlay_split_view.set_enable_hide_gesture(true);
    overlay_split_view.set_pin_sidebar(true);
    overlay_split_view.set_sidebar_position(gtk::PackType::End);
    overlay_split_view.set_sidebar_width_unit(adw::LengthUnit::Px);
    overlay_split_view.set_min_sidebar_width(240.0);
    overlay_split_view.set_max_sidebar_width(SIDEBAR_DESKTOP_MAX_WIDTH);
    let toast_overlay = adw::ToastOverlay::new();

    let sidebar_toggle_button = gtk::Button::new();
    sidebar_toggle_button.set_icon_name("sidebar-show-right-symbolic");
    sidebar_toggle_button.set_tooltip_text(Some("Toggle Options Panel"));
    {
        let overlay_split_view = overlay_split_view.clone();
        sidebar_toggle_button.connect_clicked(move |_| {
            overlay_split_view.set_show_sidebar(!overlay_split_view.shows_sidebar());
        });
    }
    header.pack_end(&sidebar_toggle_button);

    let show_credits_action = gio::SimpleAction::new("show-credits", None);
    {
        let window = window.clone();
        show_credits_action.connect_activate(move |_, _| {
            let about_dialog = build_about_dialog(&window);
            about_dialog.present(Some(&window));
        });
    }
    window.add_action(&show_credits_action);

    let open_numbat_syntax_action = gio::SimpleAction::new("open-numbat-syntax", None);
    open_numbat_syntax_action.connect_activate(move |_, _| {
        if let Err(err) = gio::AppInfo::launch_default_for_uri(
            NUMBAT_SYNTAX_URL,
            None::<&gio::AppLaunchContext>,
        ) {
            eprintln!("Failed to open Numbat syntax docs: {err}");
        }
    });
    window.add_action(&open_numbat_syntax_action);

    let open_examples_action = gio::SimpleAction::new("open-examples", None);
    open_examples_action.connect_activate(move |_, _| {
        if let Err(err) = gio::AppInfo::launch_default_for_uri(
            NUMBAT_EXAMPLES_URL,
            None::<&gio::AppLaunchContext>,
        ) {
            eprintln!("Failed to open Numbat examples docs: {err}");
        }
    });
    window.add_action(&open_examples_action);

    let toggle_fullscreen_action = gio::SimpleAction::new("toggle-fullscreen", None);
    {
        let window = window.clone();
        toggle_fullscreen_action.connect_activate(move |_, _| {
            if window.is_fullscreen() {
                window.unfullscreen();
            } else {
                window.fullscreen();
            }
        });
    }
    window.add_action(&toggle_fullscreen_action);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_margin_top(HISTORY_MARGIN);
    root.set_margin_bottom(HISTORY_MARGIN);
    root.set_margin_start(HISTORY_MARGIN);
    root.set_margin_end(HISTORY_MARGIN);
    root.set_spacing(8);

    let history_view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .vexpand(true)
        .build();
    let history_buffer = gtk::TextBuffer::new(None);
    history_view.set_buffer(Some(&history_buffer));
    ensure_numbat_tags(&history_buffer);

    let banner_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        BANNER_SWITCH_WIDTH as f64,
        adw::LengthUnit::Px,
    ));
    {
        let history_buffer = history_buffer.clone();
        let history_view_ref = history_view.clone();
        let showing_startup = Rc::clone(&showing_startup);
        let overlay_split_view = overlay_split_view.clone();
        banner_breakpoint.connect_apply(move |_| {
            overlay_split_view.set_collapsed(true);
            overlay_split_view.set_max_sidebar_width(SIDEBAR_MOBILE_MAX_WIDTH);
            if *showing_startup.borrow() {
                set_startup_message(&history_buffer, &history_view_ref, STARTUP_BANNER_SMALL);
            }
        });
    }
    {
        let history_buffer = history_buffer.clone();
        let history_view_ref = history_view.clone();
        let showing_startup = Rc::clone(&showing_startup);
        let overlay_split_view = overlay_split_view.clone();
        banner_breakpoint.connect_unapply(move |_| {
            overlay_split_view.set_collapsed(false);
            overlay_split_view.set_max_sidebar_width(SIDEBAR_DESKTOP_MAX_WIDTH);
            if *showing_startup.borrow() {
                set_startup_message(&history_buffer, &history_view_ref, STARTUP_BANNER_LARGE);
            }
        });
    }
    window.add_breakpoint(banner_breakpoint);

    let history_scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&history_view)
        .build();
    history_scroller.set_min_content_height(120);
    history_scroller.add_css_class("card");

    let input_entry = gtk::Entry::builder()
        .hexpand(true)
        .placeholder_text("Type Numbat code, then press Enter")
        .build();

    let completion_panel = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .reveal_child(false)
        .build();
    let completion_list = gtk::Box::new(gtk::Orientation::Vertical, 4);
    completion_list.set_margin_top(6);
    completion_list.set_margin_bottom(6);
    completion_list.set_margin_start(6);
    completion_list.set_margin_end(6);
    let completion_scroller = gtk::ScrolledWindow::builder()
        .min_content_height(96)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&completion_list)
        .build();
    completion_scroller.add_css_class("card");
    completion_panel.set_child(Some(&completion_scroller));

    let run_button = gtk::Button::with_label("Run");
    run_button.add_css_class("suggested-action");

    let suggestions_button = gtk::Button::with_label("Suggestions");
    suggestions_button.set_sensitive(false);

    let input_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    input_row.set_margin_top(INPUT_MARGIN);
    input_row.append(&input_entry);
    input_row.append(&suggestions_button);
    input_row.append(&run_button);

    // Physics constants buttons
    let constants_row = gtk::FlowBox::new();
    constants_row.set_margin_top(8);
    constants_row.set_margin_bottom(8);
    constants_row.set_halign(gtk::Align::Center);
    constants_row.set_valign(gtk::Align::Start);
    constants_row.set_max_children_per_line(8);
    constants_row.set_selection_mode(gtk::SelectionMode::None);
    
    let constants: &[(&str, &str)] = &[
        ("ℏ", "h_bar"),
        ("k_B", "boltzmann_constant"),
        ("𝜋", "pi"),
        ("m_e", "electron_mass"),
        ("𝛼", "fine_structure_constant"),
        ("ε₀", "eps0"),
        ("μ₀", "magnetic_constant"),
        ("μ_B", "bohr_magneton"),
    ];

    for (label, constant_name) in constants {
        let btn = gtk::Button::with_label(label);
        btn.set_tooltip_text(Some(constant_name));
        let entry_clone = input_entry.clone();
        let const_name = constant_name.to_string();
        btn.connect_clicked(move |_| {
            let current = entry_clone.text().to_string();
            let separator = if current.is_empty() { "" } else { " " };
            entry_clone.set_text(&format!("{}{}{}", current, separator, const_name));
        });
        constants_row.insert(&btn, -1);
    }

    let status_label = gtk::Label::new(Some(
        "Ready. Commands like \"help\", \"list\", \"clear\", \"reset\", and \"quit\" work here too.",
    ));
    status_label.set_halign(gtk::Align::Start);
    status_label.set_wrap(true);

    root.append(&history_scroller);
    root.append(&status_label);
    root.append(&input_row);
    root.append(&completion_panel);
    root.append(&constants_row);

    let calculator_clamp = adw::Clamp::new();
    calculator_clamp.set_maximum_size(860);
    calculator_clamp.set_tightening_threshold(520);
    calculator_clamp.set_child(Some(&root));

    // Now add the reset and clear actions after history_buffer is created
    let reset_session_action = gio::SimpleAction::new("reset-session", None);
    {
        let window = window.clone();
        let session = Rc::clone(&session);
        let history_buffer = history_buffer.clone();
        reset_session_action.connect_activate(move |_, _| {
            let dialog = adw::AlertDialog::new(
                Some("Reset Session?"),
                Some("This will clear all variables and start fresh."),
            );
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("reset", "Reset");
            dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");

            let session = Rc::clone(&session);
            let history_buffer = history_buffer.clone();
            dialog.connect_response(None, move |_, response| {
                if response == "reset" {
                    *session.borrow_mut() = NumbatSession::new();
                    history_buffer.set_text("");
                }
            });

            dialog.present(Some(&window));
        });
    }
    window.add_action(&reset_session_action);

    let clear_history_action = gio::SimpleAction::new("clear-history", None);
    {
        let window = window.clone();
        let history_buffer = history_buffer.clone();
        let command_history = Rc::clone(&command_history);
        clear_history_action.connect_activate(move |_, _| {
            let dialog = adw::AlertDialog::new(
                Some("Clear All Inputs?"),
                Some("This will erase all inputs."),
            );
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("clear", "Clear");
            dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");

            let history_buffer = history_buffer.clone();
            let command_history = Rc::clone(&command_history);
            dialog.connect_response(None, move |_, response| {
                if response == "clear" {
                    history_buffer.set_text("");
                    command_history.borrow_mut().clear();
                }
            });

            dialog.present(Some(&window));
        });
    }
    window.add_action(&clear_history_action);

    let submit_input = Rc::new({
        let session = Rc::clone(&session);
        let command_history = Rc::clone(&command_history);
        let history_cursor = Rc::clone(&history_cursor);
        let draft_input = Rc::clone(&draft_input);
        let history_buffer = history_buffer.clone();
        let history_view = history_view.clone();
        let status_label = status_label.clone();
        let toast_overlay = toast_overlay.clone();
        let app = app.clone();
        let showing_startup = Rc::clone(&showing_startup);
        move |input: String| {
            let trimmed = input.trim().to_string();
            if trimmed.is_empty() {
                status_label.set_text("Type a Numbat expression or command first.");
                return;
            }

            {
                let mut history = command_history.borrow_mut();
                if history.last().map(|s| s.as_str()) != Some(trimmed.as_str()) {
                    history.push(trimmed.clone());
                }
            }
            *showing_startup.borrow_mut() = false;
            *history_cursor.borrow_mut() = None;
            draft_input.borrow_mut().clear();

            let outcome = session.borrow_mut().handle_input(&trimmed);
            append_history(&history_buffer, &history_view, &trimmed, &outcome.output);

            if outcome.clear_history {
                history_buffer.set_text("");
            }

            if outcome.quit {
                app.quit();
                return;
            }

            if let Some(status) = outcome.status {
                status_label.set_text(status);
                toast_overlay.add_toast(adw::Toast::new(status));
            } else {
                status_label.set_text("Ready when you are!");
            }

            if outcome.reset_session {
                status_label.set_text("Session reset.");
            }
        }
    });

    let open_numbat_help_action = gio::SimpleAction::new("open-numbat-help", None);
    {
        let submit_input = Rc::clone(&submit_input);
        open_numbat_help_action.connect_activate(move |_, _| {
            submit_input(String::from("help"));
        });
    }
    window.add_action(&open_numbat_help_action);

    let open_numbat_list_action = gio::SimpleAction::new("open-numbat-list", None);
    {
        let submit_input = Rc::clone(&submit_input);
        open_numbat_list_action.connect_activate(move |_, _| {
            submit_input(String::from("list"));
        });
    }
    window.add_action(&open_numbat_list_action);

    let make_sidebar_button = |icon_name: &str, label: &str, action: gio::SimpleAction| {
        let button = gtk::Button::new();
        button.set_hexpand(true);
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let icon = gtk::Image::from_icon_name(icon_name);
        let text = gtk::Label::new(Some(label));
        text.set_ellipsize(gtk::pango::EllipsizeMode::End);
        text.set_hexpand(true);
        text.set_halign(gtk::Align::Start);
        row.append(&icon);
        row.append(&text);
        button.set_child(Some(&row));

        let overlay_split_view = overlay_split_view.clone();
        button.connect_clicked(move |_| {
            action.activate(None);
            if overlay_split_view.is_collapsed() {
                overlay_split_view.set_show_sidebar(false);
            }
        });

        button
    };

    let sidebar_panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
    sidebar_panel.set_margin_top(6);
    sidebar_panel.set_margin_bottom(6);
    sidebar_panel.set_margin_start(6);
    sidebar_panel.set_margin_end(6);

    sidebar_panel.append(&make_sidebar_button("help-faq-symbolic", "Quick Syntax Help", open_numbat_help_action.clone()));
    sidebar_panel.append(&make_sidebar_button(
        "view-list-symbolic",
        "List of Constants",
        open_numbat_list_action.clone(),
    ));

    sidebar_panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    sidebar_panel.append(&make_sidebar_button(
        "emblem-documents-symbolic",
        "Detailed Numbat Syntax",
        open_numbat_syntax_action.clone(),
    ));
    sidebar_panel.append(&make_sidebar_button("globe-symbolic", "Online Examples", open_examples_action.clone()));

    sidebar_panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    sidebar_panel.append(&make_sidebar_button("view-refresh-symbolic", "Reset Session", reset_session_action.clone()));
    sidebar_panel.append(&make_sidebar_button("edit-clear-all-symbolic", "Clear Inputs", clear_history_action.clone()));

    sidebar_panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    sidebar_panel.append(&make_sidebar_button("help-about-symbolic", "About", show_credits_action.clone()));
    {
        let button = gtk::Button::new();
        button.set_hexpand(true);
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.append(&gtk::Image::from_icon_name("view-fullscreen-symbolic"));
        let text = gtk::Label::new(Some("Fullscreen"));
        text.set_halign(gtk::Align::Start);
        row.append(&text);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        row.append(&spacer);
        let shortcut = gtk::Label::new(Some("F11"));
        shortcut.add_css_class("dim-label");
        shortcut.add_css_class("caption");
        row.append(&shortcut);
        button.set_child(Some(&row));

        let window = window.clone();
        let overlay_split_view = overlay_split_view.clone();
        button.connect_clicked(move |_| {
            if window.is_fullscreen() {
                window.unfullscreen();
            } else {
                window.fullscreen();
            }
            if overlay_split_view.is_collapsed() {
                overlay_split_view.set_show_sidebar(false);
            }
        });
        sidebar_panel.append(&button);
    }
    {
        let button = gtk::Button::new();
        button.set_hexpand(true);
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.append(&gtk::Image::from_icon_name("application-exit-symbolic"));
        let text = gtk::Label::new(Some("Quit"));
        text.set_halign(gtk::Align::Start);
        row.append(&text);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        row.append(&spacer);
        let shortcut = gtk::Label::new(Some("Ctrl+Q"));
        shortcut.add_css_class("dim-label");
        shortcut.add_css_class("caption");
        row.append(&shortcut);
        button.set_child(Some(&row));

        let app = app.clone();
        let overlay_split_view = overlay_split_view.clone();
        button.connect_clicked(move |_| {
            app.quit();
            if overlay_split_view.is_collapsed() {
                overlay_split_view.set_show_sidebar(false);
            }
        });
        sidebar_panel.append(&button);
    }

    let sidebar_scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(240)
        .child(&sidebar_panel)
        .build();

    let sidebar_header_bar = adw::HeaderBar::new();
    sidebar_header_bar.set_show_end_title_buttons(false);
    sidebar_header_bar.set_show_start_title_buttons(false);
    let sidebar_title_widget = gtk::Label::new(Some("Options"));
    sidebar_header_bar.set_title_widget(Some(&sidebar_title_widget));
    let close_sidebar_button = gtk::Button::new();
    close_sidebar_button.set_icon_name("window-close-symbolic");
    {
        let overlay_split_view = overlay_split_view.clone();
        close_sidebar_button.connect_clicked(move |_| {
            overlay_split_view.set_show_sidebar(false);
        });
    }
    sidebar_header_bar.pack_end(&close_sidebar_button);

    let sidebar_toolbar_view = adw::ToolbarView::new();
    sidebar_toolbar_view.add_top_bar(&sidebar_header_bar);
    sidebar_toolbar_view.set_content(Some(&sidebar_scroller));
    overlay_split_view.set_sidebar(Some(&sidebar_toolbar_view));

    let submit = {
        let submit_input = Rc::clone(&submit_input);
        let input_entry = input_entry.clone();
        let completion_panel = completion_panel.clone();
        Rc::new(move || {
            completion_panel.set_reveal_child(false);
            let input = input_entry.text().to_string();
            submit_input(input);
            input_entry.set_text("");
        })
    };

    {
        let submit = Rc::clone(&submit);
        run_button.connect_clicked(move |_| submit());
    }

    {
        let submit = Rc::clone(&submit);
        input_entry.connect_activate(move |_| submit());
    }

    {
        let suggestions_button = suggestions_button.clone();
        let completion_panel = completion_panel.clone();
        let completion_list = completion_list.clone();
        input_entry.connect_changed(move |entry| {
            let has_input = !entry.text().trim().is_empty();
            suggestions_button.set_sensitive(has_input);

            if !has_input {
                while let Some(child) = completion_list.first_child() {
                    completion_list.remove(&child);
                }
                completion_panel.set_reveal_child(false);
            }
        });
    }

    let show_completions = {
        let session = Rc::clone(&session);
        let input_entry = input_entry.clone();
        let completion_panel = completion_panel.clone();
        let completion_list = completion_list.clone();
        Rc::new(move || {
            while let Some(child) = completion_list.first_child() {
                completion_list.remove(&child);
            }

            let input = input_entry.text();
            let cursor = input_entry.position().max(0) as usize;
            let cursor = input
                .char_indices()
                .nth(cursor)
                .map(|(index, _)| index)
                .unwrap_or_else(|| input.len());

            let prefix_start = completion_prefix_start(&input, cursor);

            if prefix_start >= cursor {
                completion_panel.set_reveal_child(false);
                return;
            }

            let prefix = input[prefix_start..cursor].to_string();
            let suggestions = session.borrow().completions_for(&prefix);

            if suggestions.is_empty() {
                completion_panel.set_reveal_child(false);
                return;
            }

            for suggestion in suggestions.into_iter() {
                let button = gtk::Button::with_label(&suggestion);
                button.set_halign(gtk::Align::Fill);
                button.set_hexpand(true);
                button.add_css_class("flat");

                let input_entry = input_entry.clone();
                let completion_panel = completion_panel.clone();
                let suggestion = suggestion.clone();
                button.connect_clicked(move |_| {
                    let mut text = input_entry.text().to_string();
                    let cursor = input_entry.position().max(0) as usize;
                    let cursor = text
                        .char_indices()
                        .nth(cursor)
                        .map(|(index, _)| index)
                        .unwrap_or_else(|| text.len());

                    let prefix_start = completion_prefix_start(&text, cursor);

                    text.replace_range(prefix_start..cursor, &suggestion);
                    input_entry.set_text(&text);
                    input_entry.set_position(-1);
                    completion_panel.set_reveal_child(false);
                });

                completion_list.append(&button);
            }

            completion_panel.set_reveal_child(true);
        })
    };

    {
        let show_completions = Rc::clone(&show_completions);
        suggestions_button.connect_clicked(move |_| show_completions());
    }

    {
        let command_history = Rc::clone(&command_history);
        let history_cursor = Rc::clone(&history_cursor);
        let draft_input = Rc::clone(&draft_input);
        let input_entry_for_keys = input_entry.clone();
        let completion_panel = completion_panel.clone();
        let show_completions = Rc::clone(&show_completions);
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |_, key, _, _| match key {
            gtk::gdk::Key::Tab => {
                show_completions();
                gtk::glib::Propagation::Stop
            }
            gtk::gdk::Key::Up => {
                completion_panel.set_reveal_child(false);
                let history = command_history.borrow();
                if history.is_empty() {
                    return gtk::glib::Propagation::Stop;
                }

                let next_index = match *history_cursor.borrow() {
                    Some(idx) => idx.saturating_sub(1),
                    None => {
                        *draft_input.borrow_mut() = input_entry_for_keys.text().to_string();
                        history.len().saturating_sub(1)
                    }
                };

                *history_cursor.borrow_mut() = Some(next_index);
                input_entry_for_keys.set_text(&history[next_index]);
                input_entry_for_keys.set_position(-1);
                gtk::glib::Propagation::Stop
            }
            gtk::gdk::Key::Down => {
                completion_panel.set_reveal_child(false);
                let history = command_history.borrow();
                if history.is_empty() {
                    return gtk::glib::Propagation::Stop;
                }

                let Some(idx) = *history_cursor.borrow() else {
                    return gtk::glib::Propagation::Stop;
                };

                if idx + 1 < history.len() {
                    let next_index = idx + 1;
                    *history_cursor.borrow_mut() = Some(next_index);
                    input_entry_for_keys.set_text(&history[next_index]);
                    input_entry_for_keys.set_position(-1);
                } else {
                    *history_cursor.borrow_mut() = None;
                    let restored = draft_input.borrow().clone();
                    input_entry_for_keys.set_text(&restored);
                    input_entry_for_keys.set_position(-1);
                }

                gtk::glib::Propagation::Stop
            }
            _ => {
                completion_panel.set_reveal_child(false);
                gtk::glib::Propagation::Proceed
            }
        });
        input_entry.add_controller(key_controller);
    }

    set_startup_message(&history_buffer, &history_view, STARTUP_BANNER_LARGE);

    let content_toolbar_view = adw::ToolbarView::new();
    content_toolbar_view.add_top_bar(&header);
    content_toolbar_view.set_content(Some(&calculator_clamp));

    overlay_split_view.set_content(Some(&content_toolbar_view));
    overlay_split_view.set_show_sidebar(false);

    toast_overlay.set_child(Some(&overlay_split_view));
    window.set_content(Some(&toast_overlay));

    window
}

fn completion_prefix_start(text: &str, cursor: usize) -> usize {
    let mut prefix_start = cursor;

    for (index, ch) in text[..cursor].char_indices().rev() {
        if ch.is_alphanumeric() || ch == '_' {
            prefix_start = index;
        } else {
            break;
        }
    }

    prefix_start
}
