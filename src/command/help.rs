//! `help` - display a summary of all available commands
//!
//! Prints a formatted list of every command registered in the shell,
//! sorted alphabetically, with a short description of each. This is
//! the user's quick-reference guide within the terminal.
//!
//! # Usage
//!
//! ```text
//! help
//! ```
//!
//! # Implementation Notes
//!
//! The command list is generated dynamically from the
//! [`Registry`](super::Registry) — each command's
//! [`Command::name()`](super::Command::name) and
//! [`Command::description()`](super::Command::description) methods
//! provide the content.  Adding a new command automatically includes
//! it in the help output.

use crate::command::Registry;

/// Execute the `help` command.
///
/// Builds and returns a formatted help text listing all available commands
/// with their descriptions, sorted alphabetically by command name.
///
/// # Arguments
///
/// * `registry` — The command registry to enumerate.
///
/// # Returns
///
/// A multi-line string with aligned command names and descriptions.
pub fn execute(registry: &Registry) -> String {
    let mut entries: Vec<(&str, &str)> = registry
        .all_commands()
        .iter()
        .map(|c| (c.name(), c.description()))
        .collect();
    entries.sort_by_key(|(name, _)| *name);

    let mut output = String::from("Available commands:\n");
    for (name, desc) in &entries {
        output.push_str(&format!("  {:12} {}\n", name, desc));
    }
    output
}

/// Unit struct implementing the [`super::Command`] trait for `help`.
pub struct HelpCommand;

impl super::Command for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }
    fn description(&self) -> &'static str {
        "Display this help message"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        Ok(execute(ctx.registry))
    }
    fn synopsis(&self) -> &'static str {
        "help"
    }
    fn man_description(&self) -> &'static str {
        "Display a formatted list of all available commands sorted alphabetically, each with a brief description. This serves as the user's quick-reference guide within the terminal."
    }
    fn examples(&self) -> &'static [&'static str] {
        &[]
    }
}
