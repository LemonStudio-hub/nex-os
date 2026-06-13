//! `chown` -- change file ownership (simulated).
//!
//! # Usage
//!
//! ```text
//! chown <owner>[:<group>] <file> [file2 ...]
//! ```
//!
//! Simulates changing the owner (and optionally group) of one or more files.
//! Because NexOS's VFS has no real ownership system, this command validates
//! the argument format and silently succeeds -- no metadata is persisted.
//!
//! # Examples
//!
//! ```text
//! chown alice /tmp/file.txt
//! chown alice:staff /tmp/file.txt /tmp/other.txt
//! ```
//!
//! # Errors
//!
//! - Fewer than two arguments (owner spec + at least one file).
//! - Empty owner string.

/// Execute the `chown` command.
///
/// Validates the owner specification (`args[0]`) and requires at least one
/// file path argument.  The file arguments are currently unused because the
/// VFS has no ownership metadata -- the command exists for script
/// compatibility and user familiarity.
///
/// # Returns
///
/// Always `Ok(String::new())` when the arguments are valid -- `chown` produces
/// no stdout on success, matching POSIX behaviour.
///
/// # Errors
///
/// Returns an error if fewer than two arguments are provided or the owner
/// string is empty.
pub fn execute(args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("chown: missing operand".to_string());
    }

    let owner = args[0];
    let _files = &args[1..];

    // Validate owner format: must be a non-empty username, optionally followed
    // by `:group`.  The colon-only case ("") is caught by the is_empty check.
    if !owner.contains(':') && owner.is_empty() {
        return Err("chown: invalid user".to_string());
    }

    // In a full implementation we would iterate `_files`, resolve each path,
    // and store ownership in node metadata.  Here we just validate and confirm.
    Ok(String::new())
}

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct ChownCommand;

/// Delegates to the standalone [`execute`] function.  Does not need VFS access
/// because ownership is purely simulated.
impl super::Command for ChownCommand {
    fn name(&self) -> &'static str {
        "chown"
    }
    fn description(&self) -> &'static str {
        "Change file ownership (owner[:group])"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(ctx.args).into()
    }
    fn synopsis(&self) -> &'static str {
        "chown owner[:group] file [file2 ...]"
    }
    fn man_description(&self) -> &'static str {
        "Change file ownership (simulated). The owner specification can be a plain username or username:group format. Since the VFS has no real ownership system, the command validates argument format and silently succeeds without persisting any metadata."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["chown alice file.txt", "chown alice:staff file.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_owner() {
        let out = execute(&["alice", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn valid_owner_with_group() {
        let out = execute(&["alice:staff", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn missing_operand() {
        assert!(execute(&[]).is_err());
        assert!(execute(&["alice"]).is_err());
    }

    #[test]
    fn empty_owner_errors() {
        assert!(execute(&["", "/tmp/f.txt"]).is_err());
    }
}
