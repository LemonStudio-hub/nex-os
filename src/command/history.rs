//! `history` command -- display the numbered list of previously executed commands.
//!
//! # Usage
//!
//! ```text
//! history
//! ```
//!
//! # Description
//!
//! Prints every command the user has entered during the current session,
//! prefixed with a sequential number (starting at 1).  The history is
//! maintained by the [`Shell`](crate::shell::Shell) and passed into the
//! command via [`CommandContext::history`](crate::command::CommandContext).
//!
//! # Examples
//!
//! ```text
//! $ ls
//! $ echo hello
//! $ history
//!     1  ls
//!     2  echo hello
//!     3  history
//! ```
//!
//! # Notes
//!
//! Unlike real bash, this implementation does not support `history -c`
//! (clear), `history -d N` (delete entry), or `!N` (re-execute).
//! The entire session history is always displayed.

/// Execute the `history` command against the given history slice.
///
/// Each entry is printed right-aligned in a 5-character wide number column
/// followed by two spaces and the command text.  The output always ends with
/// a trailing newline per entry.
///
/// # Arguments
///
/// * `history` -- Slice of command strings in chronological order.
///
/// # Returns
///
/// `Ok(output)` with the formatted history, or an empty string if history
/// is empty (the for-loop simply produces nothing).
pub fn execute(history: &[String]) -> Result<String, String> {
    let mut output = String::new();
    for (i, cmd) in history.iter().enumerate() {
        // Right-align the 1-based index in a 5-char column so columns stay
        // aligned even when history grows past 9 or 99 entries.
        output.push_str(&format!("{:>5}  {}\n", i + 1, cmd));
    }
    Ok(output)
}

/// Unit struct representing the `history` command.
///
/// Implements [`Command`](super::Command) so the shell registry can
/// dispatch to it by name.
pub struct HistoryCommand;

/// Trait implementation that bridges the registry's generic dispatch
/// to the module-level `execute` function.
impl super::Command for HistoryCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str {
        "history"
    }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str {
        "Display command history"
    }

    /// Execute the command, forwarding the shell's history slice.
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.history).into()
    }
    fn synopsis(&self) -> &'static str {
        "history"
    }
    fn man_description(&self) -> &'static str {
        "Display a numbered list of all commands entered during the current session. Each entry is prefixed with a sequential number starting at 1. Unlike real bash, this implementation does not support history deletion or re-execution."
    }
    fn examples(&self) -> &'static [&'static str] {
        &[]
    }
}
