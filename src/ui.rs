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
const STARTUP_BANNER: &str = 
r#" 
██╗    ██╗ ██████╗ ███╗   ███╗██████╗  █████╗ ████████╗
██║    ██║██╔═══██╗████╗ ████║██╔══██╗██╔══██╗╚══██╔══╝
██║ █╗ ██║██║   ██║██╔████╔██║██████╔╝███████║   ██║   
██║███╗██║██║   ██║██║╚██╔╝██║██╔══██╗██╔══██║   ██║   
╚███╔███╔╝╚██████╔╝██║ ╚═╝ ██║██████╔╝██║  ██║   ██║   
 ╚══╝╚══╝  ╚═════╝ ╚═╝     ╚═╝╚═════╝ ╚═╝  ╚═╝   ╚═╝"#;

pub fn build_window(app: &adw::Application) -> adw::ApplicationWindow {
    let session = Rc::new(RefCell::new(NumbatSession::new()));
    let command_history: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let history_cursor: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let draft_input: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(900)
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
    overlay_split_view.set_min_sidebar_width(240.0);
    overlay_split_view.set_max_sidebar_width(320.0);
    let toast_overlay = adw::ToastOverlay::new();

    let sidebar_toggle_button = gtk::Button::with_label("Options");
    {
        let overlay_split_view = overlay_split_view.clone();
        sidebar_toggle_button.connect_clicked(move |_| {
            overlay_split_view.set_show_sidebar(!overlay_split_view.shows_sidebar());
        });
    }
    header.pack_end(&sidebar_toggle_button);

    let update_sidebar_toggle_label = {
        let sidebar_toggle_button = sidebar_toggle_button.clone();
        let overlay_split_view = overlay_split_view.clone();
        Rc::new(move || {
            if overlay_split_view.shows_sidebar() {
                sidebar_toggle_button.set_label("Hide Options");
            } else {
                sidebar_toggle_button.set_label("Options");
            }
        })
    };
    update_sidebar_toggle_label();
    {
        let update_sidebar_toggle_label = Rc::clone(&update_sidebar_toggle_label);
        overlay_split_view.connect_show_sidebar_notify(move |_| {
            update_sidebar_toggle_label();
        });
    }

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
            let dialog = gtk::Dialog::builder()
                .title("Reset Session?")
                .modal(true)
                .transient_for(&window)
                .build();
            
            let content_area = dialog.content_area();
            let label = gtk::Label::new(Some("This will clear all variables and start fresh."));
            label.set_wrap(true);
            label.set_margin_top(12);
            label.set_margin_bottom(12);
            label.set_margin_start(12);
            label.set_margin_end(12);
            content_area.append(&label);

            dialog.add_button("No", gtk::ResponseType::Cancel);
            dialog.add_button("Yes", gtk::ResponseType::Accept);
            dialog.set_default_response(gtk::ResponseType::Cancel);

            let session = Rc::clone(&session);
            let history_buffer = history_buffer.clone();
            dialog.connect_response(move |dialog, response_id| {
                if response_id == gtk::ResponseType::Accept {
                    // Reset session by creating a new one
                    *session.borrow_mut() = NumbatSession::new();
                    // Clear history display
                    history_buffer.set_text("");
                }
                dialog.close();
            });

            dialog.present();
        });
    }
    window.add_action(&reset_session_action);

    let clear_history_action = gio::SimpleAction::new("clear-history", None);
    {
        let window = window.clone();
        let history_buffer = history_buffer.clone();
        let command_history = Rc::clone(&command_history);
        clear_history_action.connect_activate(move |_, _| {
            let dialog = gtk::Dialog::builder()
                .title("Clear all inputs?")
                .modal(true)
                .transient_for(&window)
                .build();
            
            let content_area = dialog.content_area();
            let label = gtk::Label::new(Some("This will erase all inputs."));
            label.set_wrap(true);
            label.set_margin_top(12);
            label.set_margin_bottom(12);
            label.set_margin_start(12);
            label.set_margin_end(12);
            content_area.append(&label);

            dialog.add_button("No", gtk::ResponseType::Cancel);
            dialog.add_button("Yes", gtk::ResponseType::Accept);
            dialog.set_default_response(gtk::ResponseType::Cancel);

            let history_buffer = history_buffer.clone();
            let command_history = Rc::clone(&command_history);
            dialog.connect_response(move |dialog, response_id| {
                if response_id == gtk::ResponseType::Accept {
                    history_buffer.set_text("");
                    command_history.borrow_mut().clear();
                }
                dialog.close();
            });

            dialog.present();
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

    let make_sidebar_button = |label: &str, action: gio::SimpleAction| {
        let button = gtk::Button::with_label(label);
        button.set_hexpand(true);
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");

        let overlay_split_view = overlay_split_view.clone();
        button.connect_clicked(move |_| {
            action.activate(None);
            if overlay_split_view.is_collapsed() {
                overlay_split_view.set_show_sidebar(false);
            }
        });

        button
    };

    let sidebar_panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    sidebar_panel.set_margin_top(12);
    sidebar_panel.set_margin_bottom(12);
    sidebar_panel.set_margin_start(12);
    sidebar_panel.set_margin_end(12);

    let sidebar_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let sidebar_title = gtk::Label::new(Some("Actions"));
    sidebar_title.add_css_class("title-4");
    sidebar_title.set_halign(gtk::Align::Start);
    sidebar_title.set_hexpand(true);
    sidebar_header.append(&sidebar_title);

    let close_sidebar_button = gtk::Button::with_label("Close");
    close_sidebar_button.add_css_class("flat");
    {
        let overlay_split_view = overlay_split_view.clone();
        close_sidebar_button.connect_clicked(move |_| {
            overlay_split_view.set_show_sidebar(false);
        });
    }
    sidebar_header.append(&close_sidebar_button);
    sidebar_panel.append(&sidebar_header);

    let quick_help_label = gtk::Label::new(Some("Quick help"));
    quick_help_label.add_css_class("heading");
    quick_help_label.set_halign(gtk::Align::Start);
    sidebar_panel.append(&quick_help_label);
    sidebar_panel.append(&make_sidebar_button("Quick Syntax Help", open_numbat_help_action.clone()));
    sidebar_panel.append(&make_sidebar_button(
        "List of Constants and Functions",
        open_numbat_list_action.clone(),
    ));

    let docs_separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    sidebar_panel.append(&docs_separator);

    let docs_label = gtk::Label::new(Some("Online docs"));
    docs_label.add_css_class("heading");
    docs_label.set_halign(gtk::Align::Start);
    sidebar_panel.append(&docs_label);
    sidebar_panel.append(&make_sidebar_button(
        "Detailed Numbat Syntax",
        open_numbat_syntax_action.clone(),
    ));
    sidebar_panel.append(&make_sidebar_button("Online Examples", open_examples_action.clone()));

    let session_separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    sidebar_panel.append(&session_separator);

    let session_label = gtk::Label::new(Some("Session"));
    session_label.add_css_class("heading");
    session_label.set_halign(gtk::Align::Start);
    sidebar_panel.append(&session_label);
    sidebar_panel.append(&make_sidebar_button("Reset Session", reset_session_action.clone()));
    sidebar_panel.append(&make_sidebar_button("Clear Inputs", clear_history_action.clone()));

    let app_separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    sidebar_panel.append(&app_separator);

    let app_label = gtk::Label::new(Some("App"));
    app_label.add_css_class("heading");
    app_label.set_halign(gtk::Align::Start);
    sidebar_panel.append(&app_label);
    sidebar_panel.append(&make_sidebar_button("About", show_credits_action.clone()));
    {
        let button = gtk::Button::with_label("Fullscreen");
        button.set_hexpand(true);
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");

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
        let button = gtk::Button::with_label("Quit");
        button.set_hexpand(true);
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");

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
    sidebar_scroller.add_css_class("card");

    overlay_split_view.set_sidebar(Some(&sidebar_scroller));

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

    set_startup_message(&history_buffer, &history_view, STARTUP_BANNER);

    overlay_split_view.set_content(Some(&calculator_clamp));
    overlay_split_view.set_show_sidebar(false);

    let shell = gtk::Box::new(gtk::Orientation::Vertical, 0);
    shell.append(&header);
    shell.append(&overlay_split_view);

    toast_overlay.set_child(Some(&shell));
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
