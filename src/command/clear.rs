//! `clear` -- clear the terminal screen.
//!
//! # Usage
//!
//! ```text
//! clear
//! ```
//!
//! Returns the ANSI escape sequence `\x1b[2J\x1b[H` which, when printed by
//! the xterm.js frontend, erases the entire visible screen and moves the
//! cursor to the top-left position (row 1, column 1).
//!
//! # Notes
//!
//! This command takes no arguments and ignores all context.  The actual screen
//! clearing is performed by the terminal emulator (xterm.js) interpreting the
//! returned escape codes -- the command itself does not manipulate any VFS or
//! shell state.

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct ClearCommand;

/// Returns the ANSI clear-screen sequence.  No VFS access, no arguments, no
/// side effects -- the terminal emulator handles the visual reset.
impl super::Command for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn description(&self) -> &'static str {
        "Clear the terminal screen"
    }

    fn execute(&self, _ctx: &mut super::CommandContext) -> Result<String, String> {
        // \x1b[2J  = Erase entire display
        // \x1b[H   = Move cursor to home position (row 1, col 1)
        Ok("\x1b[2J\x1b[H".to_string())
    }
    fn synopsis(&self) -> &'static str {
        "clear"
    }
    fn man_description(&self) -> &'static str {
        "Clear the terminal screen by returning ANSI escape sequences that erase the entire visible display and move the cursor to the top-left position. The actual clearing is performed by the terminal emulator (xterm.js)."
    }
    fn examples(&self) -> &'static [&'static str] {
        &[]
    }
}
