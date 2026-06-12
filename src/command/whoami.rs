//! `whoami` - display the current username
//!
//! Prints the username of the currently logged-in user. In NexOS the
//! username is set during the initial authentication flow and stored in
//! the shell's state. This command simply retrieves that value.
//!
//! # Usage
//!
//! ```text
//! whoami
//! ```
//!
//! # Flags
//!
//! None. This command takes no arguments.
//!
//! # Examples
//!
//! ```text
//! whoami          # prints "user" (or whatever username was configured)
//! ```

/// Execute the `whoami` command.
///
/// Returns the username string followed by a newline. The username is
/// passed in from the shell's `CommandContext`, which in turn comes from
/// the initial login/authentication flow.
///
/// # Arguments
///
/// * `username` -- The current user's username string.
///
/// # Returns
///
/// `Ok(String)` containing the username and a trailing newline.
pub fn execute(username: &str) -> Result<String, String> {
    Ok(format!("{}\n", username))
}

/// Command struct implementing the [`super::Command`] trait for `whoami`.
pub struct WhoamiCommand;

/// Trait implementation that wires `WhoamiCommand` into the shell's command
/// registry. This is one of the simplest commands -- it takes no arguments,
/// reads no files, and does not accept stdin.
impl super::Command for WhoamiCommand {
    /// Returns the command name used for dispatch and tab completion.
    fn name(&self) -> &'static str {
        "whoami"
    }

    /// Short description shown in `help` output.
    fn description(&self) -> &'static str {
        "Display the current username"
    }

    /// Entry point called by the shell dispatcher. Extracts the username
    /// from the [`super::CommandContext`] and delegates to the standalone
    /// [`execute`] function.
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.username)
    }
    fn synopsis(&self) -> &'static str {
        "whoami"
    }
    fn man_description(&self) -> &'static str {
        "Display the username of the currently logged-in user. The username is set during the initial authentication flow and stored in the shell state."
    }
    fn examples(&self) -> &'static [&'static str] {
        &[]
    }
}
