//! `tail` - display the last N lines of a file
//!
//! Prints the last portion of a file to stdout. By default, the last 10 lines
//! are shown. The number of lines can be controlled with the `-n` flag.
//!
//! # Usage
//!
//! ```text
//! tail [-n COUNT] <file>
//! tail [-nCOUNT] <file>
//! ```
//!
//! # Flags
//!
//! - `-n COUNT` -- Show the last `COUNT` lines instead of the default 10.
//!   Can also be written in compact form `-nCOUNT` (no space).
//!
//! # Examples
//!
//! ```text
//! tail /var/log/syslog           # last 10 lines
//! tail -n 20 /var/log/syslog     # last 20 lines
//! tail -n5 /var/log/syslog       # last 5 lines (compact flag)
//! ```

use crate::vfs::Vfs;

/// Execute the `tail` command against the virtual filesystem.
///
/// Parses arguments to extract an optional `-n` line count and a required file
/// path, then reads the file and returns only the trailing lines. If the file
/// has fewer lines than the requested count, all lines are returned.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual filesystem to read from.
/// * `args` -- Slice of argument strings. Accepts `-n COUNT` or `-nCOUNT`
///   followed by a file path.
///
/// # Returns
///
/// `Ok(String)` containing the trailing lines joined by newlines (with a
/// trailing newline), or `Err` if the file is missing or arguments are invalid.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    // Default to the POSIX-standard 10 lines when -n is not specified.
    let mut count: usize = 10;
    let mut file_path: Option<&str> = None;

    let mut i = 0;
    while i < args.len() {
        // Handle "-n COUNT" (space-separated form).
        if args[i] == "-n" && i + 1 < args.len() {
            count = args[i + 1]
                .parse::<usize>()
                .map_err(|_| format!("tail: invalid line count: '{}'", args[i + 1]))?;
            i += 2;
        // Handle "-nCOUNT" (compact form with no space).
        } else if args[i].starts_with("-n") && args[i].len() > 2 {
            count = args[i][2..]
                .parse::<usize>()
                .map_err(|_| format!("tail: invalid line count: '{}'", &args[i][2..]))?;
            i += 1;
        } else if file_path.is_none() {
            // First non-flag argument is the file path.
            file_path = Some(args[i]);
            i += 1;
        } else {
            // Reject extra positional arguments -- tail only operates on one file.
            return Err("tail: too many arguments".to_string());
        }
    }

    let path = file_path.ok_or("tail: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;

    // Use file_line_count to determine the skip offset, then read only the
    // trailing lines via the efficient partial-read API.
    let total = vfs.file_line_count(&resolved)?;
    let start = if total > count { total - count } else { 0 };
    let output = vfs.read_file_lines(&resolved, start, count)?;
    Ok(format!("{}\n", output))
}

/// Command struct implementing the [`super::Command`] trait for `tail`.
pub struct TailCommand;

/// Trait implementation that wires `TailCommand` into the shell's command
/// registry. Delegates to the standalone [`execute`] function.
impl super::Command for TailCommand {
    /// Returns the command name used for dispatch and tab completion.
    fn name(&self) -> &'static str { "tail" }

    /// Short description shown in `help` output.
    fn description(&self) -> &'static str { "Display last N lines of a file (-n COUNT)" }

    /// Indicates that `tail` can accept piped stdin, which the shell routes
    /// as a file argument when no explicit path is given.
    fn accepts_stdin(&self) -> bool { true }

    /// Entry point called by the shell dispatcher. Extracts the VFS and args
    /// from the shared [`super::CommandContext`].
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.vfs, ctx.args)
    }
    fn synopsis(&self) -> &'static str { "tail [-n COUNT] file" }
    fn man_description(&self) -> &'static str {
        "Display the last N lines of a file to standard output. By default, the last 10 lines \
are shown. Use the -n flag to specify a different line count; both spaced (-n 5) and compact \
(-n5) forms are accepted. If the file has fewer lines than requested, all lines are shown."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["tail file.txt", "tail -n 5 file.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a VFS containing a file with `n` numbered lines.
    fn vfs_with_lines(n: usize) -> Vfs {
        let mut vfs = Vfs::new();
        let content: String = (1..=n)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        vfs.write_file("/tmp/f.txt", &content).unwrap();
        vfs
    }

    #[test]
    fn default_ten_lines() {
        let vfs = vfs_with_lines(20);
        let out = execute(&vfs, &["/tmp/f.txt"]).unwrap();
        // Should contain lines 11-20 but not line 10.
        assert!(out.contains("line11"));
        assert!(out.contains("line20"));
        assert!(!out.contains("line10"));
    }

    #[test]
    fn custom_count() {
        let vfs = vfs_with_lines(20);
        let out = execute(&vfs, &["-n", "3", "/tmp/f.txt"]).unwrap();
        // Last 3 lines of a 20-line file: lines 18, 19, 20.
        assert!(out.contains("line18"));
        assert!(out.contains("line20"));
    }

    #[test]
    fn compact_n_flag() {
        let vfs = vfs_with_lines(10);
        let out = execute(&vfs, &["-n2", "/tmp/f.txt"]).unwrap();
        // Compact flag "-n2" should behave the same as "-n 2".
        assert!(out.contains("line9"));
        assert!(out.contains("line10"));
    }

    #[test]
    fn file_shorter_than_count() {
        let vfs = vfs_with_lines(3);
        let out = execute(&vfs, &["-n", "10", "/tmp/f.txt"]).unwrap();
        // When the file has fewer lines than requested, all lines are returned.
        assert!(out.contains("line1"));
        assert!(out.contains("line3"));
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        // No arguments at all should produce an error.
        assert!(execute(&vfs, &[]).is_err());
    }

    #[test]
    fn invalid_count() {
        let vfs = Vfs::new();
        // A non-numeric value after -n should produce an error.
        assert!(execute(&vfs, &["-n", "abc", "/tmp/f.txt"]).is_err());
    }
}
