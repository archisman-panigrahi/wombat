use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
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
const OPERATOR_BUTTONS_DESKTOP_PREF_FILE: &str = "operator-buttons-desktop.conf";
const CUSTOM_DEFINITIONS_PREF_FILE: &str = "custom-definitions.nbt";
const COMPLETION_ROW_HEIGHT: i32 = 40;
const COMPLETION_VISIBLE_ROWS: i32 = 3;
const COMPLETION_MIN_CHIP_WIDTH: i32 = 96;
const READY_STATUS: &str = "Ready when you are!";
const SUGGESTIONS_STATUS: &str = "Suggested (press Tab for quick access).";
const CUSTOM_DEFINITIONS_PLACEHOLDER: &str = "# Examples:
let my_const = 42
unit foot_inch = foot + inch
fn double(x) = 2 * x";
const STARTUP_BANNER_LARGE: &str = r#"
 ██╗    ██╗ ██████╗ ███╗   ███╗██████╗  █████╗ ████████╗
 ██║    ██║██╔═══██╗████╗ ████║██╔══██╗██╔══██╗╚══██╔══╝
 ██║ █╗ ██║██║   ██║██╔████╔██║██████╔╝███████║   ██║
 ██║███╗██║██║   ██║██║╚██╔╝██║██╔══██╗██╔══██║   ██║
 ╚███╔███╔╝╚██████╔╝██║ ╚═╝ ██║██████╔╝██║  ██║   ██║
  ╚══╝╚══╝  ╚═════╝ ╚═╝     ╚═╝╚═════╝ ╚═╝  ╚═╝   ╚═╝"#;

const STARTUP_BANNER_SMALL: &str = r#"
░▒█░░▒█░▒█▀▀▀█░▒█▀▄▀█░▒█▀▀▄░█▀▀▄░▀▀█▀▀
░▒█▒█▒█░▒█░░▒█░▒█▒█▒█░▒█▀▀▄▒█▄▄█░░▒█░░
░▒▀▄▀▄▀░▒█▄▄▄█░▒█░░▒█░▒█▄▄█▒█░▒█░░▒█░░
"#;

const BANNER_SWITCH_WIDTH: i32 = 500;

pub fn build_window(app: &adw::Application) -> adw::ApplicationWindow {
    let custom_definitions_text = load_custom_definitions();
    let session = Rc::new(RefCell::new(NumbatSession::new_with_custom_code(
        &custom_definitions_text,
    )));
    let custom_definitions = Rc::new(RefCell::new(custom_definitions_text));
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
    sidebar_toggle_button.set_tooltip_text(Some("Toggle Options Panel (F10)"));
    header.pack_end(&sidebar_toggle_button);

    let show_credits_action = gio::SimpleAction::new("show-credits", None);
    {
        let window = window.clone();
        show_credits_action.connect_activate(move |_, _| {
            let about_dialog = build_about_dialog();
            about_dialog.present(Some(&window));
        });
    }
    window.add_action(&show_credits_action);

    let open_numbat_syntax_action = gio::SimpleAction::new("open-numbat-syntax", None);
    open_numbat_syntax_action.connect_activate(move |_, _| {
        open_uri(NUMBAT_SYNTAX_URL, "Numbat syntax docs");
    });
    window.add_action(&open_numbat_syntax_action);

    let open_examples_action = gio::SimpleAction::new("open-examples", None);
    open_examples_action.connect_activate(move |_, _| {
        open_uri(NUMBAT_EXAMPLES_URL, "Numbat examples docs");
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

    let show_keyboard_shortcuts_action = gio::SimpleAction::new("show-keyboard-shortcuts", None);
    {
        let window = window.clone();
        show_keyboard_shortcuts_action.connect_activate(move |_, _| {
            let shortcuts_dialog = build_shortcuts_dialog();
            shortcuts_dialog.present(Some(&window));
        });
    }
    window.add_action(&show_keyboard_shortcuts_action);
    app.set_accels_for_action("win.show-keyboard-shortcuts", &["<Control>question"]);

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
    let completion_buttons: Rc<RefCell<Vec<gtk::Button>>> = Rc::new(RefCell::new(Vec::new()));
    let completion_list = gtk::FlowBox::new();
    completion_list.set_margin_top(6);
    completion_list.set_margin_bottom(6);
    completion_list.set_margin_start(6);
    completion_list.set_margin_end(6);
    completion_list.set_column_spacing(6);
    completion_list.set_row_spacing(6);
    completion_list.set_halign(gtk::Align::Fill);
    completion_list.set_valign(gtk::Align::Start);
    completion_list.set_hexpand(true);
    completion_list.set_selection_mode(gtk::SelectionMode::None);
    completion_list.set_min_children_per_line(1);
    completion_list.set_max_children_per_line(12);
    let completion_scroller = gtk::ScrolledWindow::builder()
        .min_content_height(COMPLETION_ROW_HEIGHT)
        .max_content_height(COMPLETION_ROW_HEIGHT * COMPLETION_VISIBLE_ROWS)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&completion_list)
        .build();
    completion_scroller.set_propagate_natural_height(true);
    completion_scroller.add_css_class("card");
    completion_panel.set_child(Some(&completion_scroller));

    let run_button = gtk::Button::with_label("Run");
    run_button.add_css_class("suggested-action");

    let input_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    input_row.set_margin_top(INPUT_MARGIN);
    input_row.append(&input_entry);
    input_row.append(&run_button);

    // Operator buttons are always shown on mobile; desktop has an explicit toggle.
    let operators_revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .reveal_child(false)
        .build();
    let operators_row = gtk::FlowBox::new();
    operators_row.set_margin_top(6);
    operators_row.set_margin_bottom(2);
    operators_row.set_halign(gtk::Align::Center);
    operators_row.set_valign(gtk::Align::Start);
    operators_row.set_max_children_per_line(8);
    operators_row.set_selection_mode(gtk::SelectionMode::None);

    for operator in ["+", "-", "*", "/", "^"] {
        let btn = gtk::Button::with_label(operator);
        let entry_clone = input_entry.clone();
        btn.connect_clicked(move |_| {
            insert_entry_text(&entry_clone, operator);
        });
        operators_row.insert(&btn, -1);
    }
    operators_revealer.set_child(Some(&operators_row));

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
    root.append(&completion_panel);
    root.append(&input_row);
    root.append(&operators_revealer);
    let browser_shortcuts = gtk::FlowBox::new();
    browser_shortcuts.set_margin_top(4);
    browser_shortcuts.set_margin_bottom(4);
    browser_shortcuts.set_halign(gtk::Align::Center);
    browser_shortcuts.set_valign(gtk::Align::Start);
    browser_shortcuts.set_selection_mode(gtk::SelectionMode::None);
    browser_shortcuts.set_max_children_per_line(3);
    root.append(&browser_shortcuts);
    root.append(&constants_row);

    let desktop_operator_buttons_visible = Rc::new(Cell::new(load_operator_buttons_pref()));
    let operator_visibility_switch = gtk::Switch::builder()
        .active(desktop_operator_buttons_visible.get())
        .valign(gtk::Align::Center)
        .build();
    let sync_operator_buttons = Rc::new({
        let overlay_split_view = overlay_split_view.clone();
        let operator_visibility_switch = operator_visibility_switch.clone();
        let operators_revealer = operators_revealer.clone();
        let desktop_operator_buttons_visible = Rc::clone(&desktop_operator_buttons_visible);
        move || {
            let mobile = overlay_split_view.is_collapsed();
            let show = mobile || desktop_operator_buttons_visible.get();
            operator_visibility_switch.set_sensitive(!mobile);
            operator_visibility_switch.set_active(show);
            operators_revealer.set_reveal_child(show);
        }
    });
    {
        let overlay_split_view = overlay_split_view.clone();
        let desktop_operator_buttons_visible = Rc::clone(&desktop_operator_buttons_visible);
        let sync_operator_buttons = Rc::clone(&sync_operator_buttons);
        operator_visibility_switch.connect_active_notify(move |switch| {
            if !overlay_split_view.is_collapsed() {
                desktop_operator_buttons_visible.set(switch.is_active());
                save_operator_buttons_pref(switch.is_active());
            }
            sync_operator_buttons();
        });
    }

    let sync_layout = Rc::new({
        let history_buffer = history_buffer.clone();
        let history_view_ref = history_view.clone();
        let showing_startup = Rc::clone(&showing_startup);
        let overlay_split_view = overlay_split_view.clone();
        let sync_operator_buttons = Rc::clone(&sync_operator_buttons);
        move |collapsed: bool, max_sidebar_width: f64, startup_banner: &str| {
            overlay_split_view.set_collapsed(collapsed);
            overlay_split_view.set_max_sidebar_width(max_sidebar_width);
            sync_operator_buttons();
            if *showing_startup.borrow() {
                set_startup_message(&history_buffer, &history_view_ref, startup_banner);
            }
        }
    });
    {
        let sync_layout = Rc::clone(&sync_layout);
        banner_breakpoint.connect_apply(move |_| {
            sync_layout(true, SIDEBAR_MOBILE_MAX_WIDTH, STARTUP_BANNER_SMALL);
        });
    }
    {
        let sync_layout = Rc::clone(&sync_layout);
        banner_breakpoint.connect_unapply(move |_| {
            sync_layout(false, SIDEBAR_DESKTOP_MAX_WIDTH, STARTUP_BANNER_LARGE);
        });
    }
    window.add_breakpoint(banner_breakpoint);

    sync_operator_buttons();

    // Now add the reset and clear actions after history_buffer is created
    let reset_session_action = gio::SimpleAction::new("reset-session", None);
    {
        let window = window.clone();
        let session = Rc::clone(&session);
        let custom_definitions = Rc::clone(&custom_definitions);
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
            let custom_definitions = Rc::clone(&custom_definitions);
            let history_buffer = history_buffer.clone();
            dialog.connect_response(None, move |_, response| {
                if response == "reset" {
                    *session.borrow_mut() =
                        NumbatSession::new_with_custom_code(&custom_definitions.borrow());
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
        let history_scroller = history_scroller.clone();
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
            scroll_to_bottom_after_layout(&history_scroller);

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
                status_label.set_text(READY_STATUS);
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

    let edit_custom_definitions_action = gio::SimpleAction::new("edit-custom-definitions", None);
    {
        let window = window.clone();
        let session = Rc::clone(&session);
        let custom_definitions = Rc::clone(&custom_definitions);
        let status_label = status_label.clone();
        let toast_overlay = toast_overlay.clone();
        edit_custom_definitions_action.connect_activate(move |_, _| {
            show_custom_definitions_dialog(
                &window,
                Rc::clone(&session),
                Rc::clone(&custom_definitions),
                &status_label,
                &toast_overlay,
            );
        });
    }
    window.add_action(&edit_custom_definitions_action);

    let open_units_browser_action = gio::SimpleAction::new("open-units-browser", None);
    {
        let window = window.clone();
        let session = Rc::clone(&session);
        let input_entry = input_entry.clone();
        open_units_browser_action.connect_activate(move |_, _| {
            show_units_browser(&window, Rc::clone(&session), &input_entry);
        });
    }
    window.add_action(&open_units_browser_action);

    let open_constants_browser_action = gio::SimpleAction::new("open-constants-browser", None);
    {
        let window = window.clone();
        let session = Rc::clone(&session);
        let input_entry = input_entry.clone();
        open_constants_browser_action.connect_activate(move |_, _| {
            show_constants_browser(&window, Rc::clone(&session), &input_entry);
        });
    }
    window.add_action(&open_constants_browser_action);

    let open_functions_browser_action = gio::SimpleAction::new("open-functions-browser", None);
    {
        let window = window.clone();
        let session = Rc::clone(&session);
        let input_entry = input_entry.clone();
        open_functions_browser_action.connect_activate(move |_, _| {
            show_functions_browser(&window, Rc::clone(&session), &input_entry);
        });
    }
    window.add_action(&open_functions_browser_action);

    for (icon_name, label, action) in [
        (
            "applications-science-symbolic",
            "Units",
            open_units_browser_action.clone(),
        ),
        (
            "view-list-symbolic",
            "Constants",
            open_constants_browser_action.clone(),
        ),
        (
            "accessories-calculator-symbolic",
            "Functions",
            open_functions_browser_action.clone(),
        ),
    ] {
        let button = make_browser_shortcut_button(icon_name, label);
        button.connect_clicked(move |_| {
            action.activate(None);
        });
        browser_shortcuts.insert(&button, -1);
    }

    let sidebar_buttons: Rc<RefCell<Vec<gtk::Button>>> = Rc::new(RefCell::new(Vec::new()));

    let make_sidebar_button = {
        let overlay_split_view = overlay_split_view.clone();
        let sidebar_buttons = Rc::clone(&sidebar_buttons);
        move |icon_name: &str, label: &str, action: gio::SimpleAction| {
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

            sidebar_buttons.borrow_mut().push(button.clone());
            button
        }
    };

    let sidebar_panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
    sidebar_panel.set_margin_top(6);
    sidebar_panel.set_margin_bottom(6);
    sidebar_panel.set_margin_start(6);
    sidebar_panel.set_margin_end(6);

    sidebar_panel.append(&make_sidebar_button(
        "help-faq-symbolic",
        "Quick Syntax Help",
        open_numbat_help_action.clone(),
    ));
    sidebar_panel.append(&make_sidebar_button(
        "applications-science-symbolic",
        "Units Browser",
        open_units_browser_action.clone(),
    ));
    sidebar_panel.append(&make_sidebar_button(
        "view-list-symbolic",
        "Constants Browser",
        open_constants_browser_action.clone(),
    ));
    sidebar_panel.append(&make_sidebar_button(
        "accessories-calculator-symbolic",
        "Functions Browser",
        open_functions_browser_action.clone(),
    ));
    sidebar_panel.append(&make_sidebar_button(
        "document-edit-symbolic",
        "Custom Definitions",
        edit_custom_definitions_action.clone(),
    ));

    sidebar_panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    sidebar_panel.append(&make_sidebar_button(
        "web-browser-symbolic",
        "Detailed Numbat Syntax",
        open_numbat_syntax_action.clone(),
    ));
    sidebar_panel.append(&make_sidebar_button(
        "web-browser-symbolic",
        "Online Examples",
        open_examples_action.clone(),
    ));

    sidebar_panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    sidebar_panel.append(&make_sidebar_button(
        "view-refresh-symbolic",
        "Reset Session",
        reset_session_action.clone(),
    ));
    sidebar_panel.append(&make_sidebar_button(
        "edit-clear-all-symbolic",
        "Clear output",
        clear_history_action.clone(),
    ));

    let operator_row = adw::ActionRow::new();
    operator_row.set_title("Show Operator Buttons");
    operator_row.set_subtitle(
        "Desktop: toggle +, -, *, /, and ^ quick-insert buttons (Ctrl+Shift+O). Mobile: always shown.",
    );
    operator_row.add_suffix(&operator_visibility_switch);
    operator_row.set_activatable_widget(Some(&operator_visibility_switch));
    sidebar_panel.append(&operator_row);

    sidebar_panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    sidebar_panel.append(&make_sidebar_button(
        "help-about-symbolic",
        "About",
        show_credits_action.clone(),
    ));
    sidebar_panel.append(&make_sidebar_button(
        "preferences-desktop-keyboard-shortcuts-symbolic",
        "Keyboard Shortcuts",
        show_keyboard_shortcuts_action.clone(),
    ));
    {
        let button = make_sidebar_shortcut_button("view-fullscreen-symbolic", "Fullscreen", "F11");
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
        sidebar_buttons.borrow_mut().push(button.clone());
        sidebar_panel.append(&button);
    }
    {
        let button = make_sidebar_shortcut_button("application-exit-symbolic", "Quit", "Ctrl+Q");
        let app = app.clone();
        let overlay_split_view = overlay_split_view.clone();
        button.connect_clicked(move |_| {
            app.quit();
            if overlay_split_view.is_collapsed() {
                overlay_split_view.set_show_sidebar(false);
            }
        });
        sidebar_buttons.borrow_mut().push(button.clone());
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
    let sidebar_title_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    sidebar_title_box.set_halign(gtk::Align::Center);
    let sidebar_title_widget = gtk::Label::new(Some("Options"));
    let sidebar_shortcut_label = gtk::Label::new(Some("F10"));
    sidebar_shortcut_label.add_css_class("dim-label");
    sidebar_shortcut_label.add_css_class("caption");
    sidebar_title_box.append(&sidebar_title_widget);
    sidebar_title_box.append(&sidebar_shortcut_label);
    sidebar_header_bar.set_title_widget(Some(&sidebar_title_box));
    let close_sidebar_button = gtk::Button::new();
    close_sidebar_button.set_icon_name("window-close-symbolic");
    {
        let overlay_split_view = overlay_split_view.clone();
        let input_entry = input_entry.clone();
        close_sidebar_button.connect_clicked(move |_| {
            overlay_split_view.set_show_sidebar(false);
            input_entry.grab_focus();
        });
    }
    sidebar_header_bar.pack_end(&close_sidebar_button);

    let sidebar_toolbar_view = adw::ToolbarView::new();
    sidebar_toolbar_view.add_top_bar(&sidebar_header_bar);
    sidebar_toolbar_view.set_content(Some(&sidebar_scroller));
    overlay_split_view.set_sidebar(Some(&sidebar_toolbar_view));

    {
        let overlay_split_view = overlay_split_view.clone();
        let input_entry = input_entry.clone();
        let sidebar_buttons = Rc::clone(&sidebar_buttons);
        let toggle_options_action = gio::SimpleAction::new("toggle-options", None);
        toggle_options_action.connect_activate(move |_, _| {
            let show_sidebar = !overlay_split_view.shows_sidebar();
            overlay_split_view.set_show_sidebar(show_sidebar);

            if show_sidebar {
                focus_first_button(&sidebar_buttons);
            } else {
                input_entry.grab_focus();
            }
        });
        window.add_action(&toggle_options_action);
        app.set_accels_for_action("win.toggle-options", &["F10"]);
    }

    {
        let overlay_split_view = overlay_split_view.clone();
        let operator_visibility_switch = operator_visibility_switch.clone();
        let toggle_operator_buttons_action =
            gio::SimpleAction::new("toggle-operator-buttons", None);
        toggle_operator_buttons_action.connect_activate(move |_, _| {
            if overlay_split_view.is_collapsed() {
                return;
            }
            operator_visibility_switch.set_active(!operator_visibility_switch.is_active());
        });
        window.add_action(&toggle_operator_buttons_action);
        app.set_accels_for_action("win.toggle-operator-buttons", &["<Control><Shift>O"]);
    }

    {
        let overlay_split_view = overlay_split_view.clone();
        sidebar_toggle_button.connect_clicked(move |_| {
            overlay_split_view
                .activate_action("win.toggle-options", None)
                .unwrap_or_else(|err| eprintln!("Failed to toggle options panel: {err}"));
        });
    }

    {
        let sidebar_buttons_snapshot = sidebar_buttons.borrow().clone();
        for (index, button) in sidebar_buttons_snapshot.iter().enumerate() {
            let sidebar_buttons = Rc::clone(&sidebar_buttons);
            let key_controller = gtk::EventControllerKey::new();
            key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
            key_controller.connect_key_pressed(move |_, key, _, _| match key {
                gtk::gdk::Key::Up => {
                    let button_count = sidebar_buttons.borrow().len();
                    focus_button(&sidebar_buttons, previous_index(index, button_count));
                    gtk::glib::Propagation::Stop
                }
                gtk::gdk::Key::Down => {
                    let button_count = sidebar_buttons.borrow().len();
                    focus_button(&sidebar_buttons, next_index(index, button_count));
                    gtk::glib::Propagation::Stop
                }
                _ => gtk::glib::Propagation::Proceed,
            });
            button.add_controller(key_controller);
        }
    }

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

    let show_completions = {
        let session = Rc::clone(&session);
        let input_entry = input_entry.clone();
        let completion_panel = completion_panel.clone();
        let completion_list = completion_list.clone();
        let status_label = status_label.clone();
        let completion_buttons = Rc::clone(&completion_buttons);
        Rc::new(move || {
            while let Some(child) = completion_list.first_child() {
                completion_list.remove(&child);
            }
            completion_buttons.borrow_mut().clear();

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
                status_label.set_text(READY_STATUS);
                return;
            }

            let prefix = input[prefix_start..cursor].to_string();
            let suggestions = session.borrow().completions_for(&prefix);

            if suggestions.is_empty() {
                completion_panel.set_reveal_child(false);
                status_label.set_text(READY_STATUS);
                return;
            }

            for suggestion in suggestions.into_iter() {
                let button = gtk::Button::with_label(&suggestion);
                button.set_halign(gtk::Align::Start);
                button.set_hexpand(false);
                button.set_size_request(COMPLETION_MIN_CHIP_WIDTH, -1);
                button.add_css_class("flat");

                let suggestion_index = completion_buttons.borrow().len();
                let completion_buttons_for_keys = Rc::clone(&completion_buttons);
                let input_entry_for_keys = input_entry.clone();
                let completion_panel_for_keys = completion_panel.clone();
                let status_label_for_keys = status_label.clone();
                let key_controller = gtk::EventControllerKey::new();
                key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
                key_controller.connect_key_pressed(move |_, key, _, _| match key {
                    gtk::gdk::Key::Up => {
                        let button_count = completion_buttons_for_keys.borrow().len();
                        focus_button(
                            &completion_buttons_for_keys,
                            previous_index(suggestion_index, button_count),
                        );
                        gtk::glib::Propagation::Stop
                    }
                    gtk::gdk::Key::Down => {
                        let button_count = completion_buttons_for_keys.borrow().len();
                        focus_button(
                            &completion_buttons_for_keys,
                            next_index(suggestion_index, button_count),
                        );
                        gtk::glib::Propagation::Stop
                    }
                    gtk::gdk::Key::Escape => {
                        completion_panel_for_keys.set_reveal_child(false);
                        status_label_for_keys.set_text(READY_STATUS);
                        input_entry_for_keys.grab_focus();
                        gtk::glib::Propagation::Stop
                    }
                    _ => gtk::glib::Propagation::Proceed,
                });
                button.add_controller(key_controller);

                let input_entry = input_entry.clone();
                let completion_panel = completion_panel.clone();
                let status_label = status_label.clone();
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
                    let new_cursor = text[..prefix_start + suggestion.len()].chars().count() as i32;
                    input_entry.set_text(&text);
                    completion_panel.set_reveal_child(false);
                    status_label.set_text(READY_STATUS);
                    input_entry.grab_focus();
                    input_entry.set_position(new_cursor);
                    input_entry.select_region(new_cursor, new_cursor);
                });

                completion_buttons.borrow_mut().push(button.clone());
                completion_list.insert(&button, -1);
            }

            completion_panel.set_reveal_child(true);
            status_label.set_text(SUGGESTIONS_STATUS);
        })
    };

    {
        let completion_panel = completion_panel.clone();
        let completion_list = completion_list.clone();
        let status_label = status_label.clone();
        let completion_buttons = Rc::clone(&completion_buttons);
        let show_completions = Rc::clone(&show_completions);
        input_entry.connect_changed(move |entry| {
            let has_input = !entry.text().trim().is_empty();

            if has_input {
                show_completions();
            } else {
                while let Some(child) = completion_list.first_child() {
                    completion_list.remove(&child);
                }
                completion_buttons.borrow_mut().clear();
                completion_panel.set_reveal_child(false);
                status_label.set_text(READY_STATUS);
            }
        });
    }

    {
        let command_history = Rc::clone(&command_history);
        let history_cursor = Rc::clone(&history_cursor);
        let draft_input = Rc::clone(&draft_input);
        let input_entry_for_keys = input_entry.clone();
        let completion_panel = completion_panel.clone();
        let completion_buttons = Rc::clone(&completion_buttons);
        let status_label = status_label.clone();
        let show_completions = Rc::clone(&show_completions);
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |_, key, _, _| match key {
            gtk::gdk::Key::Tab => {
                if completion_panel.reveals_child() && !completion_buttons.borrow().is_empty() {
                    focus_first_button(&completion_buttons);
                } else {
                    show_completions();
                }
                gtk::glib::Propagation::Stop
            }
            gtk::gdk::Key::Up => {
                if completion_panel.reveals_child() && !completion_buttons.borrow().is_empty() {
                    let button_count = completion_buttons.borrow().len();
                    focus_button(&completion_buttons, button_count.saturating_sub(1));
                    return gtk::glib::Propagation::Stop;
                }

                completion_panel.set_reveal_child(false);
                status_label.set_text(READY_STATUS);
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
                if completion_panel.reveals_child() && !completion_buttons.borrow().is_empty() {
                    focus_first_button(&completion_buttons);
                    return gtk::glib::Propagation::Stop;
                }

                completion_panel.set_reveal_child(false);
                status_label.set_text(READY_STATUS);
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
                status_label.set_text(READY_STATUS);
                gtk::glib::Propagation::Proceed
            }
        });
        input_entry.add_controller(key_controller);
    }

    set_startup_message(&history_buffer, &history_view, STARTUP_BANNER_LARGE);

    let content_toolbar_view = adw::ToolbarView::new();
    content_toolbar_view.add_top_bar(&header);
    content_toolbar_view.set_content(Some(&root));

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

fn show_constants_browser(
    window: &adw::ApplicationWindow,
    session: Rc<RefCell<NumbatSession>>,
    input_entry: &gtk::Entry,
) {
    let constants = session.borrow().constants();
    let content = gtk::FlowBox::new();
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);
    content.set_column_spacing(6);
    content.set_row_spacing(6);
    content.set_selection_mode(gtk::SelectionMode::None);
    content.set_max_children_per_line(6);

    let dialog = browser_dialog("Constants Browser", &content);
    for constant in constants {
        let button = gtk::Button::with_label(&constant);
        button.add_css_class("flat");
        button.set_tooltip_text(Some(&constant));
        let input_entry = input_entry.clone();
        let dialog = dialog.clone();
        button.connect_clicked(move |_| {
            insert_entry_text(&input_entry, &constant);
            dialog.force_close();
        });
        content.insert(&button, -1);
    }
    dialog.present(Some(window));
}

fn show_units_browser(
    window: &adw::ApplicationWindow,
    session: Rc<RefCell<NumbatSession>>,
    input_entry: &gtk::Entry,
) {
    let groups = session.borrow().unit_groups();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);

    let dialog = browser_dialog("Units Browser", &content);
    for group in groups {
        let expander = gtk::Expander::new(Some(&format!(
            "{} ({})",
            group.dimension,
            group.units.len()
        )));
        expander.set_expanded(matches!(
            group.dimension.as_str(),
            "Length" | "Mass" | "Time" | "Temperature"
        ));

        let unit_grid = gtk::FlowBox::new();
        unit_grid.set_margin_top(6);
        unit_grid.set_margin_bottom(6);
        unit_grid.set_column_spacing(6);
        unit_grid.set_row_spacing(6);
        unit_grid.set_selection_mode(gtk::SelectionMode::None);
        unit_grid.set_max_children_per_line(5);

        for unit in group.units {
            let button = gtk::Button::with_label(&unit.display_name);
            button.add_css_class("flat");
            button.set_tooltip_text(Some(&unit.canonical_name));
            let input_entry = input_entry.clone();
            let dialog = dialog.clone();
            button.connect_clicked(move |_| {
                insert_entry_text(&input_entry, &unit.canonical_name);
                dialog.force_close();
            });
            unit_grid.insert(&button, -1);
        }

        expander.set_child(Some(&unit_grid));
        content.append(&expander);
    }
    dialog.present(Some(window));
}

fn show_functions_browser(
    window: &adw::ApplicationWindow,
    session: Rc<RefCell<NumbatSession>>,
    input_entry: &gtk::Entry,
) {
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for function in session.borrow().functions() {
        groups
            .entry(function.module.clone())
            .or_default()
            .push(function);
    }

    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);

    let dialog = browser_dialog("Functions Browser", &content);
    for (module, functions) in groups {
        let expander = gtk::Expander::new(Some(&format!("{} ({})", module, functions.len())));
        expander.set_expanded(module == "User-defined" || module == "Internal");

        let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        list.set_margin_top(6);
        list.set_margin_bottom(6);

        for function in functions {
            let button = gtk::Button::new();
            button.add_css_class("flat");
            button.set_halign(gtk::Align::Fill);
            button.set_hexpand(true);
            if let Some(description) = &function.description {
                button.set_tooltip_text(Some(description));
            }

            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let name = gtk::Label::new(Some(&function.fn_name));
            name.set_halign(gtk::Align::Start);
            name.set_width_chars(16);
            let signature = gtk::Label::new(Some(&function.signature));
            signature.add_css_class("dim-label");
            signature.set_halign(gtk::Align::Start);
            signature.set_ellipsize(gtk::pango::EllipsizeMode::End);
            signature.set_hexpand(true);
            row.append(&name);
            row.append(&signature);
            button.set_child(Some(&row));

            let input_entry = input_entry.clone();
            let dialog = dialog.clone();
            button.connect_clicked(move |_| {
                insert_entry_text(&input_entry, &format!("{}(", function.fn_name));
                dialog.force_close();
            });
            list.append(&button);
        }

        expander.set_child(Some(&list));
        content.append(&expander);
    }
    dialog.present(Some(window));
}

fn browser_dialog(title: &str, content: &impl IsA<gtk::Widget>) -> adw::AlertDialog {
    let scroller = gtk::ScrolledWindow::builder()
        .min_content_width(420)
        .min_content_height(320)
        .max_content_height(520)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(content)
        .build();

    let dialog = adw::AlertDialog::new(Some(title), None);
    dialog.add_response("close", "Close");
    dialog.set_close_response("close");
    dialog.set_default_response(Some("close"));
    dialog.set_extra_child(Some(&scroller));
    dialog
}

fn insert_entry_text(entry: &gtk::Entry, insertion: &str) {
    let current = entry.text().to_string();
    let cursor = entry.position().max(0) as usize;
    let cursor = current
        .char_indices()
        .nth(cursor)
        .map(|(index, _)| index)
        .unwrap_or_else(|| current.len());

    let needs_leading_space = cursor > 0
        && current[..cursor]
            .chars()
            .next_back()
            .is_some_and(|ch| !ch.is_whitespace());
    let needs_trailing_space = cursor < current.len()
        && current[cursor..]
            .chars()
            .next()
            .is_some_and(|ch| !ch.is_whitespace());

    let mut inserted = String::new();
    if needs_leading_space {
        inserted.push(' ');
    }
    inserted.push_str(insertion);
    if needs_trailing_space {
        inserted.push(' ');
    }

    let mut next = current;
    next.insert_str(cursor, &inserted);
    entry.set_text(&next);
    entry.set_position((cursor + inserted.len()) as i32);
    entry.grab_focus();
}

fn show_custom_definitions_dialog(
    window: &adw::ApplicationWindow,
    session: Rc<RefCell<NumbatSession>>,
    custom_definitions: Rc<RefCell<String>>,
    status_label: &gtk::Label,
    toast_overlay: &adw::ToastOverlay,
) {
    let dialog = adw::AlertDialog::new(
        Some("Custom Definitions"),
        Some("Define custom variables, units, and functions. This code runs when Wombat starts and after session resets."),
    );
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("save", "Save");
    dialog.set_default_response(Some("save"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let editor_buffer = gtk::TextBuffer::new(None);
    editor_buffer.set_text(&custom_definitions.borrow());

    let editor = gtk::TextView::builder()
        .buffer(&editor_buffer)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .vexpand(true)
        .hexpand(true)
        .build();

    let placeholder = gtk::Label::new(Some(CUSTOM_DEFINITIONS_PLACEHOLDER));
    placeholder.add_css_class("dim-label");
    placeholder.set_halign(gtk::Align::Start);
    placeholder.set_valign(gtk::Align::Start);
    placeholder.set_margin_top(10);
    placeholder.set_margin_start(10);
    placeholder.set_wrap(true);
    placeholder.set_selectable(false);
    placeholder.set_can_target(false);
    placeholder.set_visible(buffer_text(&editor_buffer).trim().is_empty());
    {
        let placeholder = placeholder.clone();
        editor_buffer.connect_changed(move |buffer| {
            placeholder.set_visible(buffer_text(buffer).trim().is_empty());
        });
    }

    let editor_scroller = gtk::ScrolledWindow::builder()
        .min_content_height(220)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&editor)
        .build();
    editor_scroller.add_css_class("card");

    let editor_overlay = gtk::Overlay::new();
    editor_overlay.set_child(Some(&editor_scroller));
    editor_overlay.add_overlay(&placeholder);
    content.append(&editor_overlay);

    let error_label = gtk::Label::new(None);
    error_label.add_css_class("error");
    error_label.set_halign(gtk::Align::Start);
    error_label.set_wrap(true);
    error_label.set_visible(false);
    content.append(&error_label);
    dialog.set_extra_child(Some(&content));

    {
        let session = Rc::clone(&session);
        let custom_definitions = Rc::clone(&custom_definitions);
        let status_label = status_label.clone();
        let toast_overlay = toast_overlay.clone();
        dialog.connect_response(None, move |dialog, response| {
            if response != "save" {
                return;
            }

            let new_code = buffer_text(&editor_buffer);
            match session.borrow_mut().set_custom_code(&new_code) {
                Ok(()) => {
                    save_custom_definitions(&new_code);
                    *custom_definitions.borrow_mut() = new_code;
                    status_label.set_text("Custom definitions saved.");
                    toast_overlay.add_toast(adw::Toast::new("Custom definitions saved."));
                    dialog.force_close();
                }
                Err(error) => {
                    error_label.set_text(&error);
                    error_label.set_visible(true);
                }
            }
        });
    }

    dialog.present(Some(window));
}

fn buffer_text(buffer: &gtk::TextBuffer) -> String {
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

fn scroll_to_bottom_after_layout(scroller: &gtk::ScrolledWindow) {
    let scroller = scroller.clone();
    gtk::glib::idle_add_local_once(move || {
        let adjustment = scroller.vadjustment();
        adjustment.set_value((adjustment.upper() - adjustment.page_size()).max(adjustment.lower()));
    });
}

fn build_shortcuts_dialog() -> adw::ShortcutsDialog {
    let dialog = adw::ShortcutsDialog::builder()
        .title("Keyboard Shortcuts")
        .build();

    let general_section = adw::ShortcutsSection::new(Some("General"));
    general_section.add(adw::ShortcutsItem::new("Toggle Options Panel", "F10"));
    general_section.add(adw::ShortcutsItem::new(
        "Show Keyboard Shortcuts",
        "<Control>question",
    ));
    general_section.add(adw::ShortcutsItem::new("Toggle Fullscreen", "F11"));
    general_section.add(adw::ShortcutsItem::new("Quit", "<Control>q"));
    dialog.add(general_section);

    let input_section = adw::ShortcutsSection::new(Some("Input"));
    input_section.add(adw::ShortcutsItem::new("Run Input", "Return"));
    input_section.add(adw::ShortcutsItem::new("Show Suggestions", "Tab"));
    input_section.add(adw::ShortcutsItem::new(
        "Toggle Operator Buttons",
        "<Control><Shift>O",
    ));
    input_section.add(adw::ShortcutsItem::new("Previous Input", "Up"));
    input_section.add(adw::ShortcutsItem::new("Next Input", "Down"));
    dialog.add(input_section);

    let options_section = adw::ShortcutsSection::new(Some("Options"));
    options_section.add(adw::ShortcutsItem::new("Move to Previous Option", "Up"));
    options_section.add(adw::ShortcutsItem::new("Move to Next Option", "Down"));
    options_section.add(adw::ShortcutsItem::new(
        "Activate Selected Option",
        "Return",
    ));
    dialog.add(options_section);

    dialog
}

fn open_uri(uri: &str, description: &str) {
    if let Err(err) = gio::AppInfo::launch_default_for_uri(uri, None::<&gio::AppLaunchContext>) {
        eprintln!("Failed to open {description}: {err}");
    }
}

fn focus_first_button(buttons: &Rc<RefCell<Vec<gtk::Button>>>) {
    focus_button(buttons, 0);
}

fn make_sidebar_shortcut_button(icon_name: &str, label: &str, shortcut_text: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_hexpand(true);
    button.set_halign(gtk::Align::Fill);
    button.add_css_class("flat");

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.append(&gtk::Image::from_icon_name(icon_name));

    let text = gtk::Label::new(Some(label));
    text.set_halign(gtk::Align::Start);
    row.append(&text);

    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    row.append(&spacer);

    let shortcut = gtk::Label::new(Some(shortcut_text));
    shortcut.add_css_class("dim-label");
    shortcut.add_css_class("caption");
    row.append(&shortcut);

    button.set_child(Some(&row));
    button
}

fn make_browser_shortcut_button(icon_name: &str, label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_tooltip_text(Some(label));

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.append(&gtk::Image::from_icon_name(icon_name));
    row.append(&gtk::Label::new(Some(label)));
    button.set_child(Some(&row));
    button
}

fn focus_button(buttons: &Rc<RefCell<Vec<gtk::Button>>>, index: usize) {
    if let Some(button) = buttons.borrow().get(index) {
        button.grab_focus();
    }
}

fn previous_index(index: usize, button_count: usize) -> usize {
    if button_count == 0 {
        0
    } else {
        index.checked_sub(1).unwrap_or(button_count - 1)
    }
}

fn load_operator_buttons_pref() -> bool {
    load_bool_pref(pref_path(), false)
}

fn save_operator_buttons_pref(value: bool) {
    save_bool_pref(pref_path(), value);
}

fn pref_path() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("wombat")
        .join(OPERATOR_BUTTONS_DESKTOP_PREF_FILE)
}

fn custom_definitions_path() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("wombat")
        .join(CUSTOM_DEFINITIONS_PREF_FILE)
}

fn load_custom_definitions() -> String {
    fs::read_to_string(custom_definitions_path()).unwrap_or_default()
}

fn save_custom_definitions(value: &str) {
    let path = custom_definitions_path();
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("Failed to create config directory for custom definitions: {err}");
            return;
        }
    }
    if let Err(err) = fs::write(path, value) {
        eprintln!("Failed to save custom definitions: {err}");
    }
}

fn load_bool_pref(path: PathBuf, default_value: bool) -> bool {
    match fs::read_to_string(path) {
        Ok(value) => value.trim().eq_ignore_ascii_case("true"),
        Err(_) => default_value,
    }
}

fn save_bool_pref(path: PathBuf, value: bool) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, if value { "true\n" } else { "false\n" });
}

fn next_index(index: usize, button_count: usize) -> usize {
    if button_count == 0 {
        0
    } else {
        (index + 1) % button_count
    }
}
