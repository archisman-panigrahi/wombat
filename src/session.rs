use std::cell::Cell;
use std::env;
use std::mem;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use numbat::command::{CommandControlFlow, CommandRunner};
use numbat::module_importer::{BuiltinModuleImporter, ChainedImporter, FileSystemImporter};
use numbat::resolver::CodeSource;
use numbat::session_history::SessionHistory;
use numbat::{Context, FormatOptions, InterpreterSettings};

#[derive(Clone)]
pub enum OutputEvent {
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
        mem::take(&mut *self.events.lock().unwrap())
    }
}

pub struct NumbatSession {
    module_paths: Vec<PathBuf>,
    context: Context,
    command_runner: CommandRunner<'static, ()>,
    output: SharedOutput,
    clear_requested: Rc<Cell<bool>>,
}

impl NumbatSession {
    pub fn new() -> Self {
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

    pub fn completions_for(&self, word_part: &str) -> Vec<String> {
        self.context
            .get_completions_for(word_part, true)
            .filter(|completion| completion != word_part)
            .take(64)
            .collect()
    }

    pub fn handle_input(&mut self, input: &str) -> SubmissionOutcome {
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
        let control_flow =
            match self
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
                    // status = Some("Evaluation failed");
                }
            }
            CommandControlFlow::Continue => {
                if self.clear_requested.get() {
                    status = Some("History cleared");
                } else if !self.output.is_empty() {
                    // status = Some("Command executed");
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

    fn reset_context(&mut self) {
        self.context = make_context(&self.module_paths);
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

pub struct SubmissionOutcome {
    pub output: Vec<OutputEvent>,
    pub clear_history: bool,
    pub reset_session: bool,
    pub quit: bool,
    pub status: Option<&'static str>,
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
