use std::cell::Cell;
use std::env;
use std::mem;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use numbat::command::{CommandControlFlow, CommandRunner};
use numbat::markup::plain_text_format;
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
    context: Option<Context>,
    command_runner: CommandRunner<'static, ()>,
    output: SharedOutput,
    clear_requested: Rc<Cell<bool>>,
    custom_code: String,
}

impl NumbatSession {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::new_with_custom_code("")
    }

    pub fn new_with_custom_code(custom_code: &str) -> Self {
        let module_paths = configured_module_paths();
        let custom_code = custom_code.to_string();
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
            context: None,
            command_runner,
            output,
            clear_requested,
            custom_code,
        }
    }

    pub fn completions_for(&mut self, word_part: &str) -> Vec<String> {
        match self.ensure_context() {
            Ok(context) => context
                .get_completions_for(word_part, true)
                .filter(|completion| completion != word_part)
                .take(64)
                .collect(),
            Err(error) => {
                eprintln!("Failed to load custom definitions: {error}");
                Vec::new()
            }
        }
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
        let control_flow = match self.with_context_for_command(trimmed) {
            Ok(control_flow) => control_flow,
            Err(error) => {
                self.output.push_text(&error);
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
        match self.make_context_with_custom_code(&self.custom_code) {
            Ok(context) => self.context = Some(context),
            Err(error) => {
                self.context = None;
                self.output
                    .push_text(&format!("Failed to load custom definitions: {error}"));
            }
        }
    }

    fn ensure_context(&mut self) -> Result<&mut Context, String> {
        if self.context.is_none() {
            self.context = Some(self.make_context_with_custom_code(&self.custom_code)?);
        }

        Ok(self.context.as_mut().expect("context was just initialized"))
    }

    fn make_context_with_custom_code(&self, custom_code: &str) -> Result<Context, String> {
        let mut context = make_context(&self.module_paths);
        apply_custom_code_to_context(&mut context, custom_code)?;
        Ok(context)
    }

    fn with_context_for_command(&mut self, input: &str) -> Result<CommandControlFlow, String> {
        self.ensure_context()?;
        let context = self.context.as_mut().expect("context was just initialized");
        self.command_runner
            .try_run_command(input, context, &mut ())
            .map_err(|error| format!("{error:?}"))
    }

    pub fn set_custom_code(&mut self, custom_code: &str) -> Result<(), String> {
        let context = self.make_context_with_custom_code(custom_code)?;
        self.context = Some(context);
        self.custom_code = custom_code.to_string();
        Ok(())
    }

    pub fn constants(&mut self) -> Vec<String> {
        match self.ensure_context() {
            Ok(context) => {
                let mut constants: Vec<_> = context
                    .variable_names()
                    .map(|name| name.to_string())
                    .collect();
                constants.sort();
                constants.dedup();
                constants
            }
            Err(error) => {
                eprintln!("Failed to load custom definitions: {error}");
                Vec::new()
            }
        }
    }

    pub fn unit_groups(&mut self) -> Vec<UnitBrowserGroup> {
        use std::collections::BTreeMap;

        let mut groups: BTreeMap<String, Vec<UnitBrowserItem>> = BTreeMap::new();
        let Ok(context) = self.ensure_context() else {
            return Vec::new();
        };
        for (unit_name, (_base_representation, metadata)) in context.unit_representations() {
            let dimension = plain_text_format(&metadata.readable_type, false).to_string();
            if dimension == "Scalar" || dimension.is_empty() {
                continue;
            }

            let canonical_name = metadata
                .aliases
                .first()
                .map(|(name, _)| name.to_string())
                .unwrap_or_else(|| unit_name.to_string());
            if canonical_name.is_empty() {
                continue;
            }

            let display_name = metadata
                .name
                .as_ref()
                .map(|name| name.to_string())
                .unwrap_or_else(|| canonical_name.clone());

            groups.entry(dimension).or_default().push(UnitBrowserItem {
                display_name,
                canonical_name,
            });
        }

        let mut groups: Vec<_> = groups
            .into_iter()
            .map(|(dimension, mut units)| {
                units.sort_by(|a, b| a.display_name.cmp(&b.display_name));
                units.dedup_by(|a, b| {
                    a.display_name == b.display_name && a.canonical_name == b.canonical_name
                });
                UnitBrowserGroup { dimension, units }
            })
            .collect();
        groups.sort_by(|a, b| {
            dimension_priority(&a.dimension)
                .cmp(&dimension_priority(&b.dimension))
                .then_with(|| a.dimension.cmp(&b.dimension))
        });
        groups
    }

    pub fn functions(&mut self) -> Vec<FunctionBrowserItem> {
        let Ok(context) = self.ensure_context() else {
            return Vec::new();
        };
        let mut functions: Vec<_> = context
            .functions()
            .map(|function| FunctionBrowserItem {
                fn_name: function.fn_name.to_string(),
                signature: function.signature_str.to_string(),
                description: function
                    .description
                    .map(|description| description.to_string()),
                module: code_source_label(&function.code_source),
            })
            .collect();
        functions.sort_by(|a, b| {
            a.module
                .cmp(&b.module)
                .then_with(|| a.fn_name.cmp(&b.fn_name))
        });
        functions
    }

    fn evaluate_expression(&mut self, input: &str) -> Result<(), String> {
        let output = self.output.clone();
        let mut settings = InterpreterSettings {
            print_fn: Box::new(move |markup| output.push_markup(markup)),
        };

        let context = self.ensure_context()?;
        let (statements, result) = context
            .interpret_with_settings(&mut settings, input, CodeSource::Text)
            .map_err(|error| error.to_string())?;

        let result_markup = result.to_markup(
            statements.last(),
            context.dimension_registry(),
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

#[derive(Clone)]
pub struct UnitBrowserItem {
    pub display_name: String,
    pub canonical_name: String,
}

#[derive(Clone)]
pub struct UnitBrowserGroup {
    pub dimension: String,
    pub units: Vec<UnitBrowserItem>,
}

#[derive(Clone)]
pub struct FunctionBrowserItem {
    pub fn_name: String,
    pub signature: String,
    pub description: Option<String>,
    pub module: String,
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
    context.set_terminal_width(Some(84));

    let _ = context.interpret("use prelude", CodeSource::Internal);
    context
}

fn apply_custom_code_to_context(context: &mut Context, custom_code: &str) -> Result<(), String> {
    let custom_code = custom_code.trim();
    if custom_code.is_empty() {
        return Ok(());
    }

    context
        .interpret(custom_code, CodeSource::Text)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn dimension_priority(dimension: &str) -> usize {
    const PRIORITY_DIMENSIONS: &[&str] = &[
        "Length",
        "Mass",
        "Time",
        "ElectricCurrent",
        "Temperature",
        "AmountOfSubstance",
        "LuminousIntensity",
        "DigitalInformation",
        "Money",
    ];

    PRIORITY_DIMENSIONS
        .iter()
        .position(|&candidate| candidate == dimension)
        .unwrap_or(usize::MAX)
}

fn code_source_label(code_source: &CodeSource) -> String {
    match code_source {
        CodeSource::Module(path, _) => path.to_string(),
        CodeSource::Text => "User-defined".to_string(),
        CodeSource::Internal => "Internal".to_string(),
        CodeSource::File(path) => path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("File")
            .to_string(),
    }
}
