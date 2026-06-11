//! `basename` -- strip directory and trailing suffix from a file path.
//!
//! # Usage
//!
//! ```text
//! basename <path> [suffix]
//! ```
//!
//! Prints the last component of `path` after removing any leading directory
//! components (everything up to and including the final `/`).  If `suffix` is
//! provided and matches the end of the component, it is stripped as well.
//!
//! # Examples
//!
//! ```text
//! basename /home/user/file.txt        => file.txt
//! basename /home/user/file.txt .txt   => file
//! basename /home/user/dir/            => dir
//! ```
//!
//! # Notes
//!
//! Unlike the POSIX `basename`, this implementation does not accept multiple
//! paths or the `--` end-of-options sentinel.  It operates purely on string
//! manipulation -- no filesystem lookups are performed.

/// Execute the `basename` command.
///
/// Returns the final component of `args[0]`, optionally stripping `args[1]` as
/// a suffix.  The result is always terminated with a newline to match shell
/// convention.
///
/// # Errors
///
/// Returns an error if no arguments are provided.
pub fn execute(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("basename: missing operand".to_string());
    }

    let path = args[0];
    let suffix = if args.len() > 1 { Some(args[1]) } else { None };

    // Trim trailing slashes so that paths like "/home/user/dir/" yield "dir"
    // instead of an empty string.
    let trimmed = path.trim_end_matches('/');

    // Find the last '/' to isolate the final component.  If no '/' is found
    // the entire trimmed string is already the basename.
    let name = match trimmed.rfind('/') {
        Some(i) => &trimmed[i + 1..],
        None => trimmed,
    };

    // Strip the suffix only if it actually matches the tail of the name.
    // `strip_suffix` returns `None` on mismatch, so we fall back to the
    // original name -- matching POSIX `basename` behaviour.
    let result = if let Some(suf) = suffix {
        name.strip_suffix(suf).unwrap_or(name)
    } else {
        name
    };

    Ok(format!("{}\n", result))
}

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct BasenameCommand;

/// Delegates to the standalone [`execute`] function, forwarding only the
/// positional arguments (no VFS access needed).
impl super::Command for BasenameCommand {
    fn name(&self) -> &'static str { "basename" }
    fn description(&self) -> &'static str { "Strip directory from filename" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.args)
    }
    fn synopsis(&self) -> &'static str { "basename path [suffix]" }
    fn man_description(&self) -> &'static str { "Strip directory components and optional trailing suffix from a file path. Prints the last component of the path after removing everything up to and including the final slash. If a suffix argument is provided and matches the end of the component, it is also stripped." }
    fn examples(&self) -> &'static [&'static str] { &["basename /home/user/file.txt", "basename /home/user/file.txt .txt"] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_path() {
        let out = execute(&["/home/user/file.txt"]).unwrap();
        assert_eq!(out.trim(), "file.txt");
    }

    #[test]
    fn with_suffix() {
        let out = execute(&["/home/user/file.txt", ".txt"]).unwrap();
        assert_eq!(out.trim(), "file");
    }

    #[test]
    fn trailing_slashes() {
        let out = execute(&["/home/user/dir/"]).unwrap();
        assert_eq!(out.trim(), "dir");
    }

    #[test]
    fn single_component() {
        let out = execute(&["file.txt"]).unwrap();
        assert_eq!(out.trim(), "file.txt");
    }

    #[test]
    fn root_path() {
        let out = execute(&["/"]).unwrap();
        // root has no component name
        assert!(!out.is_empty());
    }

    #[test]
    fn suffix_that_doesnt_match() {
        let out = execute(&["/path/file.txt", ".log"]).unwrap();
        assert_eq!(out.trim(), "file.txt");
    }

    #[test]
    fn missing_operand() {
        assert!(execute(&[]).is_err());
    }
}
