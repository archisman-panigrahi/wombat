use std::cell::{Cell, RefCell};
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use adw::prelude::*;
use numbat::command::{CommandControlFlow, CommandRunner};
use numbat::markup::{FormatType as NumbatFormatType, FormattedString};
use numbat::module_importer::{BuiltinModuleImporter, ChainedImporter, FileSystemImporter};
use numbat::resolver::CodeSource;
use numbat::session_history::SessionHistory;
use numbat::{Context, FormatOptions, InterpreterSettings};

const APP_ID: &str = "io.github.archisman_panigrahi.wombat";
const HISTORY_MARGIN: i32 = 16;
const INPUT_MARGIN: i32 = 16;

#[derive(Clone)]
enum OutputEvent {
    Markup(numbat::markup::Markup),
    Plain(String),
}

#[derive(Clone)]
struct SharedOutput {
    events: Arc<Mutex<Vec<OutputEvent>>>,
}

impl SharedOutput {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    fn push_markup(&self, markup: &numbat::markup::Markup) {
        self.events
            .lock()
            .unwrap()
            .push(OutputEvent::Markup(markup.clone()));
    }

    fn push_text(&self, text: &str) {
        self.events
            .lock()
            .unwrap()
            .push(OutputEvent::Plain(text.to_string()));
    }

    fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }

    fn take_events(&self) -> Vec<OutputEvent> {
        self.events.lock().unwrap().clone()
    }
}

struct NumbatSession {
    module_paths: Vec<PathBuf>,
    context: Context,
    command_runner: CommandRunner<'static, ()>,
    output: SharedOutput,
    clear_requested: Rc<Cell<bool>>,
}

impl NumbatSession {
    fn new() -> Self {
        let module_paths = configured_module_paths();
        let context = make_context(&module_paths);
        let output = SharedOutput::new();
        let clear_requested = Rc::new(Cell::new(false));

        let output_for_commands = output.clone();
        let clear_for_commands = Rc::clone(&clear_requested);
        let command_runner = CommandRunner::new()
            .print_with(move |markup| output_for_commands.push_markup(markup))
            .enable_clear(move |_| {
                clear_for_commands.set(true);
                CommandControlFlow::Continue
            })
            .enable_save(SessionHistory::default())
            .enable_reset()
            .enable_quit();

        Self {
            module_paths,
            context,
            command_runner,
            output,
            clear_requested,
        }
    }

    fn reset_context(&mut self) {
        self.context = make_context(&self.module_paths);
    }

    fn handle_input(&mut self, input: &str) -> SubmissionOutcome {
        self.output.clear();
        self.clear_requested.set(false);

        let trimmed = input.trim();
        if trimmed.is_empty() {
            return SubmissionOutcome {
                output: Vec::new(),
                clear_history: false,
                reset_session: false,
                quit: false,
                status: None,
            };
        }

        let mut is_error = false;
        let mut status = None;
        let control_flow = match self
            .command_runner
            .try_run_command(trimmed, &mut self.context, &mut ())
        {
            Ok(control_flow) => control_flow,
            Err(error) => {
                self.output.push_text(&format!("{error:?}"));
                is_error = true;
                status = Some("Command error");
                CommandControlFlow::Continue
            }
        };

        let mut reset_session = false;
        let mut quit = false;

        match control_flow {
            CommandControlFlow::NotACommand => {
                let evaluation_result = self.evaluate_expression(trimmed);
                if let Err(error_text) = evaluation_result {
                    self.output.push_text(&error_text);
                    is_error = true;
                    status = Some("Evaluation failed");
                }
            }
            CommandControlFlow::Continue => {
                if self.clear_requested.get() {
                    status = Some("History cleared");
                } else if !self.output.is_empty() {
                    status = Some("Command executed");
                }
            }
            CommandControlFlow::Reset => {
                reset_session = true;
                status = Some("Session reset");
            }
            CommandControlFlow::Return => {
                quit = true;
                status = Some("Closing window");
            }
        }

        self.command_runner
            .push_to_history(trimmed, if is_error { Err(()) } else { Ok(()) });

        if reset_session {
            self.reset_context();
        }

        SubmissionOutcome {
            output: self.output.take_events(),
            clear_history: self.clear_requested.get(),
            reset_session,
            quit,
            status,
        }
    }

    fn evaluate_expression(&mut self, input: &str) -> Result<(), String> {
        let output = self.output.clone();
        let mut settings = InterpreterSettings {
            print_fn: Box::new(move |markup| output.push_markup(markup)),
        };

        let (statements, result) = self
            .context
            .interpret_with_settings(&mut settings, input, CodeSource::Text)
            .map_err(|error| error.to_string())?;

        let result_markup = result.to_markup(
            statements.last(),
            self.context.dimension_registry(),
            true,
            true,
            &FormatOptions::default(),
        );
        self.output.push_markup(&result_markup);
        Ok(())
    }
}

struct SubmissionOutcome {
    output: Vec<OutputEvent>,
    clear_history: bool,
    reset_session: bool,
    quit: bool,
    status: Option<&'static str>,
}

fn configured_module_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(value) = env::var_os("NUMBAT_MODULES_PATH") {
        let value = value.to_string_lossy();
        paths.extend(
            value
                .split(':')
                .filter(|part| !part.trim().is_empty())
                .map(expand_tilde),
        );
    }

    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        paths.push(PathBuf::from(config_home).join("numbat/modules"));
    } else if let Some(home) = env::var_os("HOME") {
        paths.push(PathBuf::from(home).join(".config/numbat/modules"));
    }

    paths.push(PathBuf::from("/usr/share/numbat/modules"));
    paths.retain(|path| path.exists());
    paths.sort();
    paths.dedup();
    paths
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }

    PathBuf::from(path)
}

fn make_context(module_paths: &[PathBuf]) -> Context {
    let mut filesystem_importer = FileSystemImporter::default();
    for path in module_paths {
        filesystem_importer.add_path(path);
    }

    let importer = ChainedImporter::new(
        Box::new(filesystem_importer),
        Box::<BuiltinModuleImporter>::default(),
    );

    let mut context = Context::new(importer);
    context.load_currency_module_on_demand(true);
    context.set_terminal_width(Some(84));

    let _ = context.interpret("use prelude", CodeSource::Internal);
    context
}

fn build_window(app: &adw::Application) -> adw::ApplicationWindow {
    let session = Rc::new(RefCell::new(NumbatSession::new()));
    let command_history: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let history_cursor: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let draft_input: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(1100)
        .default_height(800)
        .title("Wombat")
        .build();

    let header = adw::HeaderBar::new();
    let title_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let title = gtk::Label::new(Some("Wombat"));
    title.add_css_class("title-1");
    title.set_halign(gtk::Align::Start);
    let subtitle = gtk::Label::new(Some("GTK/libadwaita front-end for Numbat"));
    subtitle.add_css_class("dim-label");
    subtitle.set_halign(gtk::Align::Start);
    title_box.append(&title);
    title_box.append(&subtitle);
    header.set_title_widget(Some(&title_box));

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&header);
    root.set_margin_top(HISTORY_MARGIN);
    root.set_margin_bottom(HISTORY_MARGIN);
    root.set_margin_start(HISTORY_MARGIN);
    root.set_margin_end(HISTORY_MARGIN);
    root.set_spacing(12);

    let history_view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    history_view.set_vexpand(true);
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

    let input_label = gtk::Label::new(Some("Expression or command"));
    input_label.set_halign(gtk::Align::Start);

    let input_entry = gtk::Entry::builder()
        .hexpand(true)
        .placeholder_text("Type Numbat code, then press Enter")
        .build();

    let run_button = gtk::Button::with_label("Run");
    run_button.add_css_class("suggested-action");

    let input_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    input_row.set_margin_top(INPUT_MARGIN);
    input_row.append(&input_entry);
    input_row.append(&run_button);

    let status_label = gtk::Label::new(Some("Ready. Commands like help, list, clear, save, reset, and quit work here too."));
    status_label.set_halign(gtk::Align::Start);
    status_label.set_wrap(true);

    root.append(&history_scroller);
    root.append(&input_label);
    root.append(&input_row);
    root.append(&status_label);

    window.set_content(Some(&root));

    let submit = Rc::new({
        let session = Rc::clone(&session);
        let command_history = Rc::clone(&command_history);
        let history_cursor = Rc::clone(&history_cursor);
        let draft_input = Rc::clone(&draft_input);
        let history_buffer = history_buffer.clone();
        let history_view = history_view.clone();
        let input_entry = input_entry.clone();
        let status_label = status_label.clone();
        let app = app.clone();
        move || {
            let input = input_entry.text().to_string();
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
            input_entry.set_text("");

            if outcome.clear_history {
                history_buffer.set_text("");
            }

            if outcome.quit {
                app.quit();
                return;
            }

            if let Some(status) = outcome.status {
                status_label.set_text(status);
            } else {
                status_label.set_text("Ready.");
            }

            if outcome.reset_session {
                status_label.set_text("Session reset.");
            }
        }
    });

    {
        let submit = Rc::clone(&submit);
        run_button.connect_clicked(move |_| submit());
    }

    {
        let submit = Rc::clone(&submit);
        input_entry.connect_activate(move |_| submit());
    }

    {
        let command_history = Rc::clone(&command_history);
        let history_cursor = Rc::clone(&history_cursor);
        let draft_input = Rc::clone(&draft_input);
        let input_entry_for_keys = input_entry.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |_, key, _, _| {
            match key {
                gtk::gdk::Key::Up => {
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
                _ => gtk::glib::Propagation::Proceed,
            }
        });
        input_entry.add_controller(key_controller);
    }

    append_history(
        &history_buffer,
        &history_view,
        "welcome",
        &[OutputEvent::Plain(
            "Wombat is ready. Try `help`, `list`, or `2 m + 30 cm`.".to_string(),
        )],
    );

    window
}

fn ensure_numbat_tags(history_buffer: &gtk::TextBuffer) {
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

fn append_history(
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

fn main() {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        let window = build_window(app);
        window.present();
    });
    app.run();
}
