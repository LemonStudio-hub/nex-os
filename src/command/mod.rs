//! Command trait, execution context, and command registry.
//!
//! Every built-in command implements the [`Command`] trait and is registered
//! in the [`Registry`].  The shell dispatches commands through the registry
//! instead of a monolithic `match` statement, making it trivial to add new
//! commands: create a file, implement the trait, and register.
//!
//! # Adding a new command
//!
//! 1. Create `src/command/<name>.rs` with a struct implementing [`Command`].
//! 2. Add `pub mod <name>;` to this file.
//! 3. Register the struct in [`register_all`].
//!
//! The trait's [`Command::name`] and [`Command::accepts_stdin`] methods
//! automatically handle tab completion and pipe stdin routing.

use crate::vfs::Vfs;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Command trait
// ---------------------------------------------------------------------------

/// Execution context passed to every command.
///
/// Commands borrow only the fields they need — unused fields are silently
/// ignored by the compiler.  The lifetime `'a` ties all references to the
/// shell's own lifetime, preventing dangling pointers.
pub struct CommandContext<'a> {
    /// Mutable reference to the virtual file system.
    pub vfs: &'a mut Vfs,
    /// Stdin content from the preceding pipe stage (empty string if none).
    pub stdin: &'a str,
    /// Command-line arguments (tokens after the command name).
    pub args: &'a [&'a str],
    /// The logged-in username.
    pub username: &'a str,
    /// The machine hostname.
    pub hostname: &'a str,
    /// The full command history (read-only).
    pub history: &'a [String],
    /// Mutable reference to the shell's environment variables.
    pub env_vars: &'a mut HashMap<String, String>,
    /// Reference to the command registry (for introspection by `man`, `help`).
    pub registry: &'a Registry,
}

/// A built-in shell command.
///
/// Each command is a unit struct that implements this trait.  The trait
/// provides metadata (name, description, stdin acceptance) and the
/// execution logic.
pub trait Command {
    /// The command name as typed by the user (e.g. `"cat"`, `"ls"`).
    fn name(&self) -> &'static str;

    /// One-line description shown in the `help` output.
    fn description(&self) -> &'static str;

    /// Whether this command can consume stdin from a pipe.
    ///
    /// When `true`, the pipeline writes stdin to `/tmp/.pipe_input` and
    /// appends that path as a trailing argument.  Commands that access
    /// `ctx.stdin` directly (e.g. `tr`, `tee`) should return `false` and
    /// read from `ctx.stdin` themselves.
    fn accepts_stdin(&self) -> bool {
        false
    }

    /// Execute the command with the given context.
    ///
    /// Return `Ok(output)` on success or `Err(message)` on failure.
    /// The `&&` chaining mechanism stops on the first `Err`.
    fn execute(&self, ctx: &mut CommandContext) -> Result<String, String>;

    // ---- Man page metadata (all have defaults) ----------------------------

    /// Usage synopsis shown in `man` pages (e.g. `"ls [-l] [path]"`).
    fn synopsis(&self) -> &'static str {
        ""
    }

    /// Detailed description for `man` pages.
    ///
    /// Falls back to [`description()`] when empty.  Override this to
    /// provide richer multi-line documentation.
    fn man_description(&self) -> &'static str {
        ""
    }

    /// Example command lines shown in `man` pages.
    ///
    /// Each entry is one example line (e.g. `"ls -l /home"`).
    fn examples(&self) -> &'static [&'static str] {
        &[]
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Central registry of all available commands.
///
/// The registry is built once during shell initialisation and lives for the
/// lifetime of the shell.  Commands are stored as trait objects (`Box<dyn
/// Command>`) so that heterogeneous command structs can coexist in a single
/// collection.
pub struct Registry {
    /// Dynamically-dispatched list of all registered commands.
    commands: Vec<Box<dyn Command>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// Create a new registry and register every built-in command.
    ///
    /// This is called once during [`Shell::new`](crate::shell::Shell::new).
    pub fn new() -> Self {
        let mut commands: Vec<Box<dyn Command>> = Vec::new();
        register_all(&mut commands);
        Registry { commands }
    }

    /// Look up a command by its name.
    ///
    /// Returns `None` if no command with that name is registered.
    pub fn get(&self, name: &str) -> Option<&dyn Command> {
        self.commands.iter().find(|c| c.name() == name).map(|c| &**c)
    }

    /// Return a slice of all registered commands.
    ///
    /// Used by the `help` command to enumerate every available command.
    pub fn all_commands(&self) -> &[Box<dyn Command>] {
        &self.commands
    }

    /// Return tab-completion candidates whose names start with `partial`.
    ///
    /// Used by the frontend's tab-completion logic.
    pub fn completions(&self, partial: &str) -> Vec<String> {
        self.commands
            .iter()
            .filter(|c| c.name().starts_with(partial))
            .map(|c| c.name().to_string())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Command module declarations
// ---------------------------------------------------------------------------

pub mod basename;
pub mod cat;
pub mod cd;
pub mod chmod;
pub mod chown;
pub mod clear;
pub mod cp;
pub mod cut;
pub mod date;
pub mod diff;
pub mod dirname;
pub mod du;
pub mod echo;
pub mod env;
pub mod exit;
pub mod export;
pub mod find;
pub mod grep;
pub mod head;
pub mod help;
pub mod history;
pub mod hostname;
pub mod ln;
pub mod ls;
pub mod man;
pub mod mkdir;
pub mod mv;
pub mod pwd;
pub mod rm;
pub mod sort;
pub mod tail;
pub mod tee;
pub mod touch;
pub mod tr;
pub mod tree;
pub mod uniq;
pub mod wc;
pub mod whoami;

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register every built-in command into the provided vector.
///
/// Commands are grouped by category for readability.  The order here does
/// **not** affect execution — only the order in which `help` lists them
/// and tab-completion suggests them.
///
/// To add a new command, insert a `commands.push(Box::new(...))` line in
/// the appropriate category section below.
fn register_all(commands: &mut Vec<Box<dyn Command>>) {
    // Filesystem navigation
    commands.push(Box::new(ls::LsCommand));
    commands.push(Box::new(cd::CdCommand));
    commands.push(Box::new(pwd::PwdCommand));
    commands.push(Box::new(mkdir::MkdirCommand));
    commands.push(Box::new(touch::TouchCommand));
    commands.push(Box::new(rm::RmCommand));
    commands.push(Box::new(cp::CpCommand));
    commands.push(Box::new(mv::MvCommand));
    commands.push(Box::new(tree::TreeCommand));
    commands.push(Box::new(ln::LnCommand));

    // File content
    commands.push(Box::new(cat::CatCommand));
    commands.push(Box::new(echo::EchoCommand));
    commands.push(Box::new(head::HeadCommand));
    commands.push(Box::new(tail::TailCommand));

    // Text processing
    commands.push(Box::new(grep::GrepCommand));
    commands.push(Box::new(sort::SortCommand));
    commands.push(Box::new(uniq::UniqCommand));
    commands.push(Box::new(wc::WcCommand));
    commands.push(Box::new(cut::CutCommand));
    commands.push(Box::new(tr::TrCommand));
    commands.push(Box::new(tee::TeeCommand));

    // Diff
    commands.push(Box::new(diff::DiffCommand));

    // Search
    commands.push(Box::new(find::FindCommand));

    // Disk usage
    commands.push(Box::new(du::DuCommand));

    // Permissions & ownership
    commands.push(Box::new(chmod::ChmodCommand));
    commands.push(Box::new(chown::ChownCommand));

    // System info
    commands.push(Box::new(whoami::WhoamiCommand));
    commands.push(Box::new(hostname::HostnameCommand));
    commands.push(Box::new(date::DateCommand));
    commands.push(Box::new(history::HistoryCommand));

    // Environment
    commands.push(Box::new(env::EnvCommand));
    commands.push(Box::new(export::ExportCommand));

    // Path utilities
    commands.push(Box::new(basename::BasenameCommand));
    commands.push(Box::new(dirname::DirnameCommand));

    // Documentation
    commands.push(Box::new(man::ManCommand));

    // Terminal
    commands.push(Box::new(clear::ClearCommand));
    commands.push(Box::new(help::HelpCommand));
    commands.push(Box::new(exit::ExitCommand));
}
