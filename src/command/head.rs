//! `head` - display the first N lines of a file
//!
//! Outputs the beginning of a file, defaulting to the first 10 lines.
//! The line count can be customized with the `-n` flag.
//!
//! # Usage
//!
//! ```text
//! head [-n COUNT] <file>
//! head [-nCOUNT] <file>       # compact flag form
//! ```
//!
//! # Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `-n COUNT` | Number of lines to display (default: 10) |
//! | `-nCOUNT` | Compact form, e.g., `-n5` |
//!
//! # Behavior
//!
//! - If the file has fewer lines than the requested count, all lines
//!   are shown without error.
//! - Accepts stdin: when used in a pipeline, piped input is passed as
//!   a file argument by the shell's dispatch logic.
//! - Returns an error if no file is specified or if the count is not
//!   a valid positive integer.

use crate::vfs::Vfs;

/// Execute the `head` command.
///
/// Parses the optional `-n` flag (in either spaced or compact form) and
/// the required file path, then reads the file and outputs the requested
/// number of leading lines.
///
/// # Arguments
///
/// * `vfs` - The virtual filesystem to read from.
/// * `args` - Command-line arguments: optional `-n COUNT` flag and a file path.
///
/// # Returns
///
/// The first N lines of the file joined by newlines, or an error.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut count: usize = 10; // Default matches POSIX head behavior.
    let mut file_path: Option<&str> = None;

    // Index-based loop to allow consuming two tokens ("-n" and "5") as one.
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-n" && i + 1 < args.len() {
            // Spaced form: `-n 5`
            count = args[i + 1]
                .parse::<usize>()
                .map_err(|_| format!("head: invalid line count: '{}'", args[i + 1]))?;
            i += 2;
        } else if args[i].starts_with("-n") && args[i].len() > 2 {
            // Compact form: `-n5` -- everything after "-n" is the count.
            count = args[i][2..]
                .parse::<usize>()
                .map_err(|_| format!("head: invalid line count: '{}'", &args[i][2..]))?;
            i += 1;
        } else if file_path.is_none() {
            // First non-flag argument is the file path.
            file_path = Some(args[i]);
            i += 1;
        } else {
            return Err("head: too many arguments".to_string());
        }
    }

    let path = file_path.ok_or("head: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;

    // Use the efficient partial-read API — only the first `count` lines
    // are extracted from the chunked content, avoiding a full read.
    let output = vfs.read_file_lines(&resolved, 0, count)?;
    Ok(format!("{}\n", output))
}

/// Unit struct implementing the [`super::Command`] trait for `head`.
pub struct HeadCommand;

/// Registers `head` with the command system.
///
/// `accepts_stdin()` returns `true` so the shell automatically feeds
/// piped input as a file argument.
impl super::Command for HeadCommand {
    fn name(&self) -> &'static str {
        "head"
    }
    fn description(&self) -> &'static str {
        "Display first N lines of a file (-n COUNT)"
    }
    fn accepts_stdin(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.vfs, ctx.args)
    }
    fn synopsis(&self) -> &'static str {
        "head [-n COUNT] file"
    }
    fn man_description(&self) -> &'static str {
        "Display the first N lines of a file to standard output. By default, the first 10 lines \
are shown. Use the -n flag to specify a different line count; both spaced (-n 5) and compact \
(-n5) forms are accepted. If the file has fewer lines than requested, all lines are shown."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["head file.txt", "head -n 5 file.txt", "head -n20 log.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(out.contains("line1"));
        assert!(out.contains("line10"));
        assert!(!out.contains("line11"));
    }

    #[test]
    fn custom_count() {
        let vfs = vfs_with_lines(20);
        let out = execute(&vfs, &["-n", "3", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("line1"));
        assert!(out.contains("line3"));
        assert!(!out.contains("line4"));
    }

    #[test]
    fn compact_n_flag() {
        let vfs = vfs_with_lines(20);
        let out = execute(&vfs, &["-n5", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("line5"));
        assert!(!out.contains("line6"));
    }

    #[test]
    fn file_shorter_than_count() {
        let vfs = vfs_with_lines(3);
        let out = execute(&vfs, &["-n", "10", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("line3"));
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[]).is_err());
    }

    #[test]
    fn invalid_count() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["-n", "abc", "/tmp/f.txt"]).is_err());
    }
}
