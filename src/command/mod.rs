//! Command trait, context, and registry.
//!
//! Every command implements the `Command` trait and is registered in the
//! `Registry`. The shell dispatches commands through the registry instead
//! of a monolithic match statement.

use crate::vfs::Vfs;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Command trait
// ---------------------------------------------------------------------------

/// Context passed to every command execution. Commands borrow only the
/// fields they need — unused fields are silently ignored.
pub struct CommandContext<'a> {
    pub vfs: &'a mut Vfs,
    pub stdin: &'a str,
    pub args: &'a [&'a str],
    pub username: &'a str,
    pub hostname: &'a str,
    pub history: &'a [String],
    pub env_vars: &'a mut HashMap<String, String>,
}

/// A shell command with metadata and execution logic.
pub trait Command {
    /// The command name as typed by the user (e.g. "cat", "ls").
    fn name(&self) -> &'static str;

    /// One-line description shown in `help`.
    fn description(&self) -> &'static str;

    /// Whether this command consumes stdin via the pipe mechanism.
    /// When `true`, the pipeline writes stdin to `/tmp/.pipe_input` and
    /// appends that path to the args.
    fn accepts_stdin(&self) -> bool {
        false
    }

    /// Execute the command with the given context.
    fn execute(&self, ctx: &mut CommandContext) -> Result<String, String>;
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Central registry of all available commands. Built once at shell init.
pub struct Registry {
    commands: Vec<Box<dyn Command>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// Create a new registry with all built-in commands registered.
    pub fn new() -> Self {
        let mut commands: Vec<Box<dyn Command>> = Vec::new();
        register_all(&mut commands);
        Registry { commands }
    }

    /// Look up a command by name.
    pub fn get(&self, name: &str) -> Option<&dyn Command> {
        self.commands.iter().find(|c| c.name() == name).map(|c| &**c)
    }

    /// All registered commands.
    pub fn all_commands(&self) -> &[Box<dyn Command>] {
        &self.commands
    }

    /// Tab-completion candidates matching a prefix.
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

/// Register all built-in commands into the provided vector.
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
