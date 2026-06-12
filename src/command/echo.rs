//! `echo` - display a line of text
//!
//! Prints its arguments separated by spaces, followed by a trailing newline.
//! This is a simplified implementation of the POSIX `echo` command that
//! handles the most common use case.
//!
//! # Usage
//!
//! ```text
//! echo [text ...]
//! ```
//!
//! # Behavior
//!
//! - With no arguments, outputs a single empty line (`\n`).
//! - Multiple arguments are joined with spaces.
//! - Output redirection (`>` / `>>`) is not handled here; it is managed
//!   by the shell pipeline layer (`shell/dispatch.rs`) which intercepts
//!   the last stage's output and writes it to a file when redirection
//!   operators are present.

/// Execute the `echo` command.
///
/// Joins all arguments with spaces and appends a newline.
///
/// # Arguments
///
/// * `args` - The text tokens to echo.
///
/// # Returns
///
/// The concatenated text with a trailing newline.
pub fn execute(args: &[&str]) -> Result<String, String> {
    Ok(format!("{}\n", args.join(" ")))
}

/// Unit struct implementing the [`super::Command`] trait for `echo`.
pub struct EchoCommand;

/// Registers `echo` with the command system.
impl super::Command for EchoCommand {
    fn name(&self) -> &'static str {
        "echo"
    }
    fn description(&self) -> &'static str {
        "Display a line of text"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.args)
    }
    fn synopsis(&self) -> &'static str {
        "echo text"
    }
    fn man_description(&self) -> &'static str {
        "Display a line of text to standard output. Arguments are joined with spaces and a \
trailing newline is appended. With no arguments, prints an empty line. Shell redirection \
operators (> and >>) are handled by the pipeline layer, so echo output can be written to files."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["echo Hello World", "echo data > output.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_args_prints_empty_line() {
        let out = execute(&[]).unwrap();
        assert_eq!(out, "\n");
    }

    #[test]
    fn single_word() {
        let out = execute(&["hello"]).unwrap();
        assert_eq!(out, "hello\n");
    }

    #[test]
    fn multiple_words() {
        let out = execute(&["hello", "world"]).unwrap();
        assert_eq!(out, "hello world\n");
    }
}
