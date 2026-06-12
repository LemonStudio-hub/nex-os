//! `dirname` - strip the last component from a file path
//!
//! Returns the directory portion of a given path, effectively removing the
//! filename. This mirrors the POSIX `dirname` utility behavior:
//!
//! # Usage
//!
//! ```text
//! dirname <path>
//! ```
//!
//! # Behavior
//!
//! - `/home/user/file.txt` produces `/home/user`
//! - `/file.txt` produces `/` (root is the parent)
//! - `file.txt` (no slash) produces `.`
//! - Trailing slashes are stripped before processing (e.g., `/a/b/c/` => `/a/b`)
//!
//! # Notes
//!
//! Does not validate that the path exists in the VFS -- it operates purely
//! on string manipulation, matching the real `dirname` behavior.

/// Execute the `dirname` command.
///
/// Takes a single path argument and prints its directory component.
/// Returns an error if no arguments are provided.
pub fn execute(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("dirname: missing operand".to_string());
    }

    let path = args[0];

    // Strip trailing slashes so that "/a/b/c/" becomes "/a/b/c" before
    // we search for the last '/'. This prevents "/a/b/c/" from yielding
    // an empty basename at the trailing position.
    let trimmed = path.trim_end_matches('/');

    match trimmed.rfind('/') {
        // The only '/' is at position 0, meaning the path is like "/file" --
        // the parent directory is the filesystem root.
        Some(0) => Ok("/\n".to_string()),
        // Found a '/' somewhere in the middle -- everything before it is the
        // directory portion.
        Some(i) => Ok(format!("{}\n", &trimmed[..i])),
        // No '/' at all (e.g., "file.txt") -- POSIX dirname returns ".".
        None => Ok(".\n".to_string()),
    }
}

/// Unit struct that implements the [`super::Command`] trait for `dirname`.
///
/// This struct carries no state; it exists solely to participate in the
/// command registry and provide metadata (name, description) alongside
/// the execution logic.
pub struct DirnameCommand;

/// Registers `dirname` with the command system via the `Command` trait.
///
/// Delegates directly to the standalone `execute()` function, passing
/// through the raw argument slice from the shell context.
impl super::Command for DirnameCommand {
    fn name(&self) -> &'static str {
        "dirname"
    }
    fn description(&self) -> &'static str {
        "Strip filename from path"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.args)
    }
    fn synopsis(&self) -> &'static str {
        "dirname path"
    }
    fn man_description(&self) -> &'static str {
        "Strip the filename component from a path, returning only the directory portion. Trailing slashes are stripped before processing. Operates purely on string manipulation without filesystem lookups."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["dirname /home/user/file.txt", "dirname file.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_path() {
        let out = execute(&["/home/user/file.txt"]).unwrap();
        assert_eq!(out.trim(), "/home/user");
    }

    #[test]
    fn no_slash_returns_dot() {
        let out = execute(&["file.txt"]).unwrap();
        assert_eq!(out.trim(), ".");
    }

    #[test]
    fn root_file() {
        let out = execute(&["/file.txt"]).unwrap();
        assert_eq!(out.trim(), "/");
    }

    #[test]
    fn trailing_slashes() {
        let out = execute(&["/home/user/dir/"]).unwrap();
        assert_eq!(out.trim(), "/home/user");
    }

    #[test]
    fn deeply_nested() {
        let out = execute(&["/a/b/c/d/file"]).unwrap();
        assert_eq!(out.trim(), "/a/b/c/d");
    }

    #[test]
    fn missing_operand() {
        assert!(execute(&[]).is_err());
    }
}
