//! `exit` - terminate the terminal session
//!
//! Signals the shell to stop accepting input and shut down. In this
//! browser-based environment, "exit" doesn't kill a process -- it
//! triggers the frontend to display a shutdown message and disable
//! further input.
//!
//! # Usage
//!
//! ```text
//! exit
//! ```
//!
//! # Implementation Notes
//!
//! The command returns an empty string on success. The frontend checks
//! for the `"exit"` command name specifically and handles the shutdown
//! sequence (disabling the terminal, showing a farewell message).
//! The actual exit logic lives in the TypeScript `input.ts` handler.

/// Unit struct implementing the [`super::Command`] trait for `exit`.
///
/// Carries no state; exists solely to register with the command system.
pub struct ExitCommand;

/// Registers `exit` with the command system.
///
/// Returns an empty `Ok(String)` -- the frontend detects this command
/// by name and performs the shutdown sequence on its side.
impl super::Command for ExitCommand {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn description(&self) -> &'static str {
        "Exit the terminal"
    }

    /// Execute the exit command.
    ///
    /// Always returns `Ok("")`. The shell's top-level `execute()` method
    /// (in `lib.rs`) checks for the command name `"exit"` and sets a
    /// flag that the frontend reads to determine whether to shut down.
    fn execute(&self, _ctx: &mut super::CommandContext) -> super::CommandOutput {
        Ok(String::new()).into()
    }
    fn synopsis(&self) -> &'static str {
        "exit"
    }
    fn man_description(&self) -> &'static str {
        "Exit the terminal session. In this browser-based environment, exit signals the frontend to display a shutdown message and disable further input rather than killing a process."
    }
    fn examples(&self) -> &'static [&'static str] {
        &[]
    }
}
