use adw::prelude::*;
use numbat::markup::{FormatType as NumbatFormatType, FormattedString};

use crate::session::OutputEvent;

pub fn ensure_numbat_tags(history_buffer: &gtk::TextBuffer) {
    let table = history_buffer.tag_table();

    let tags = [
        ("nb-prompt", "#64748b"),
        ("nb-value", "#0f766e"),
        ("nb-unit", "#2563eb"),
        ("nb-operator", "#b45309"),
        ("nb-identifier", "#334155"),
        ("nb-type", "#7c2d12"),
        ("nb-string", "#166534"),
        ("nb-keyword", "#a16207"),
        ("nb-dimmed", "#6b7280"),
        ("nb-decorator", "#be123c"),
        ("nb-emphasized", "#111827"),
    ];

    for (name, color) in tags {
        if table.lookup(name).is_none() {
            let tag = gtk::TextTag::builder().name(name).foreground(color).build();
            table.add(&tag);
        }
    }

    if table.lookup("nb-banner").is_none() {
        let tag = gtk::TextTag::builder()
            .name("nb-banner")
            .family("monospace")
            .family_set(true)
            .wrap_mode(gtk::WrapMode::None)
            .build();
        table.add(&tag);
    }
}

pub fn append_history(
    history_buffer: &gtk::TextBuffer,
    history_view: &gtk::TextView,
    input: &str,
    output: &[OutputEvent],
) {
    let mut end_iter = history_buffer.end_iter();
    history_buffer.insert_with_tags_by_name(&mut end_iter, ">>> ", &["nb-prompt"]);
    insert_colored_input(history_buffer, &mut end_iter, input);
    history_buffer.insert(&mut end_iter, "\n");

    for event in output {
        insert_output_event(history_buffer, &mut end_iter, event);
    }

    history_buffer.insert(&mut end_iter, "\n");

    let mut end_iter = history_buffer.end_iter();
    history_view.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 1.0);
}

pub fn set_startup_message(
    history_buffer: &gtk::TextBuffer,
    history_view: &gtk::TextView,
    banner: &str,
) {
    history_buffer.set_text("");

    let mut end_iter = history_buffer.end_iter();
    history_buffer.insert_with_tags_by_name(&mut end_iter, banner, &["nb-banner"]);
    history_buffer.insert_with_tags_by_name(&mut end_iter, "\n", &["nb-banner"]);
    history_buffer.insert(&mut end_iter, "  Wombat v");
    history_buffer.insert(&mut end_iter, env!("CARGO_PKG_VERSION"));
    history_buffer.insert(&mut end_iter, ", powered by Numbat v");
    history_buffer.insert(&mut end_iter, env!("NUMBAT_VERSION"));
    history_buffer.insert(&mut end_iter, "\n  ........................................");
    history_buffer.insert(&mut end_iter, "\n\n  Type something like ");
    insert_colored_input(history_buffer, &mut end_iter, "\"2 m + 30 inch to cm\"");
    history_buffer.insert(&mut end_iter, " to get started.");
    history_buffer.insert(&mut end_iter, "\n  Or check out \"Quick Syntax Help\" in the menu\n");

    let mut end_iter = history_buffer.end_iter();
    history_view.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 1.0);
}

fn tag_for_format(format: NumbatFormatType) -> Option<&'static str> {
    match format {
        NumbatFormatType::Whitespace | NumbatFormatType::Text => None,
        NumbatFormatType::Emphasized => Some("nb-emphasized"),
        NumbatFormatType::Dimmed => Some("nb-dimmed"),
        NumbatFormatType::String => Some("nb-string"),
        NumbatFormatType::Keyword => Some("nb-keyword"),
        NumbatFormatType::Value => Some("nb-value"),
        NumbatFormatType::Unit => Some("nb-unit"),
        NumbatFormatType::Identifier => Some("nb-identifier"),
        NumbatFormatType::TypeIdentifier => Some("nb-type"),
        NumbatFormatType::Operator => Some("nb-operator"),
        NumbatFormatType::Decorator => Some("nb-decorator"),
    }
}

fn insert_output_event(
    history_buffer: &gtk::TextBuffer,
    end_iter: &mut gtk::TextIter,
    event: &OutputEvent,
) {
    match event {
        OutputEvent::Plain(text) => {
            history_buffer.insert(end_iter, text);
            if !text.ends_with('\n') {
                history_buffer.insert(end_iter, "\n");
            }
        }
        OutputEvent::Markup(markup) => {
            for FormattedString(_, format, text) in &markup.0 {
                let content = text.to_string();
                if let Some(tag_name) = tag_for_format(*format) {
                    history_buffer.insert_with_tags_by_name(end_iter, &content, &[tag_name]);
                } else {
                    history_buffer.insert(end_iter, &content);
                }
            }
            if !markup.to_string().ends_with('\n') {
                history_buffer.insert(end_iter, "\n");
            }
        }
    }
}

fn insert_colored_input(history_buffer: &gtk::TextBuffer, end_iter: &mut gtk::TextIter, input: &str) {
    let keywords = [
        "use", "let", "fn", "where", "dimension", "unit", "struct", "if", "then", "else",
        "true", "false", "per", "to", "print", "assert", "assert_eq", "type",
    ];

    let mut chars = input.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch.is_ascii_whitespace() {
            history_buffer.insert(end_iter, &ch.to_string());
            continue;
        }

        if ch == '"' {
            let mut end = start + ch.len_utf8();
            let mut escaped = false;
            for (idx, c) in chars.by_ref() {
                end = idx + c.len_utf8();
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if c == '"' {
                    break;
                }
            }
            history_buffer.insert_with_tags_by_name(end_iter, &input[start..end], &["nb-string"]);
            continue;
        }

        if ch.is_ascii_digit() || (ch == '.' && chars.peek().is_some_and(|(_, c)| c.is_ascii_digit())) {
            let mut end = start + ch.len_utf8();
            while let Some((idx, c)) = chars.peek() {
                if c.is_ascii_digit() || matches!(*c, '.' | '_' | 'e' | 'E' | '+' | '-') {
                    end = *idx + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            history_buffer.insert_with_tags_by_name(end_iter, &input[start..end], &["nb-value"]);
            continue;
        }

        if ch.is_ascii_alphabetic() || ch == '_' {
            let mut end = start + ch.len_utf8();
            while let Some((idx, c)) = chars.peek() {
                if c.is_ascii_alphanumeric() || *c == '_' {
                    end = *idx + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }

            let token = &input[start..end];
            let tag = if keywords.contains(&token) {
                "nb-keyword"
            } else {
                "nb-identifier"
            };
            history_buffer.insert_with_tags_by_name(end_iter, token, &[tag]);
            continue;
        }

        history_buffer.insert_with_tags_by_name(end_iter, &ch.to_string(), &["nb-operator"]);
    }
}
