//! `grep` - search for text patterns within files
//!
//! Scans one or more files for lines containing a given pattern string and
//! outputs the matching lines. Supports case-insensitive matching and line
//! number display, matching common `grep` usage patterns.
//!
//! # Usage
//!
//! ```text
//! grep [-i] [-n] <pattern> <file> [file2 ...]
//! ```
//!
//! # Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `-i` | Case-insensitive matching |
//! | `-n` | Prefix each matching line with its line number (1-based) |
//! | `-in` / `-ni` | Combined form of both flags |
//!
//! # Behavior
//!
//! - Pattern matching is substring containment (not regex).
//! - When multiple files are specified, each matching line is prefixed
//!   with the filename and a colon (e.g., `file.txt:3:hello`).
//! - A single file does not get the filename prefix.
//! - Accepts stdin: when piped input is provided, the shell appends it
//!   as a trailing file argument (via the `accepts_stdin()` trait method).

use crate::vfs::Vfs;

/// Execute the `grep` command.
///
/// Parses flags, extracts the pattern and file list, then scans each file
/// line by line. Matching lines are accumulated into an output string with
/// optional filename and line number prefixes.
///
/// # Arguments
///
/// * `vfs` - The virtual filesystem to read files from.
/// * `args` - Command-line arguments: flags, then pattern, then one or more files.
///
/// # Returns
///
/// Matching lines (with optional prefixes), or an error if pattern/file
/// arguments are missing.
pub fn execute(
    vfs: &Vfs,
    args: &[&str],
    host_fs: Option<&dyn crate::vfs::HostFs>,
) -> Result<String, String> {
    let mut case_insensitive = false;
    let mut show_line_numbers = false;
    let mut positional: Vec<&str> = Vec::new();

    // Separate flags from positional arguments. We handle combined flag
    // forms like `-in` and `-ni` so users don't have to type two flags.
    for arg in args {
        match *arg {
            "-i" => case_insensitive = true,
            "-n" => show_line_numbers = true,
            // Support combined flags: either order is valid.
            "-in" | "-ni" => {
                case_insensitive = true;
                show_line_numbers = true;
            }
            _ => positional.push(arg),
        }
    }

    // Validate that we have at least a pattern and one file.
    if positional.is_empty() {
        return Err("grep: missing pattern".to_string());
    }
    if positional.len() < 2 {
        return Err("grep: missing file operand".to_string());
    }

    let pattern = positional[0];
    let files = &positional[1..];
    // Pre-compute the lowercase pattern once for case-insensitive matching,
    // rather than lowercasing the pattern on every line comparison.
    let pattern_lower = pattern.to_lowercase();

    let mut output = String::new();

    for path in files {
        let resolved = vfs.resolve_path(path)?;
        let content = vfs.read_file_with_host(&resolved, host_fs)?;
        // Only show the filename prefix when searching multiple files,
        // matching real grep's behavior for unambiguous output.
        let show_filename = files.len() > 1;

        for (idx, line) in content.lines().enumerate() {
            // Choose the matching strategy based on the -i flag.
            let matched = if case_insensitive {
                line.to_lowercase().contains(&pattern_lower)
            } else {
                line.contains(pattern)
            };

            if matched {
                let mut prefix = String::new();
                if show_filename {
                    prefix.push_str(path);
                    prefix.push(':');
                }
                if show_line_numbers {
                    // Line numbers are 1-indexed (enumerate gives 0-indexed).
                    prefix.push_str(&format!("{}:", idx + 1));
                }
                output.push_str(&format!("{}{}\n", prefix, line));
            }
        }
    }

    Ok(output)
}

/// Unit struct implementing the [`super::Command`] trait for `grep`.
pub struct GrepCommand;

/// Registers `grep` with the command system.
///
/// `accepts_stdin()` returns `true` so that piped input from a prior
/// pipeline stage is automatically appended as a file argument by the
/// shell's dispatch logic.
impl super::Command for GrepCommand {
    fn name(&self) -> &'static str {
        "grep"
    }
    fn description(&self) -> &'static str {
        "Search for patterns in files (-i case-insensitive, -n line numbers)"
    }
    fn accepts_stdin(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }
    fn synopsis(&self) -> &'static str {
        "grep [-i] [-n] pattern file [file2 ...]"
    }
    fn man_description(&self) -> &'static str {
        "Search for lines matching a pattern in one or more files. The pattern is matched as a \
substring (not a regular expression). With -i, matching is case-insensitive. With -n, each \
matching line is prefixed with its 1-based line number. When multiple files are specified, \
output lines are prefixed with the filename."
    }
    fn examples(&self) -> &'static [&'static str] {
        &[
            "grep hello file.txt",
            "grep -i error log.txt",
            "grep -n TODO *.rs",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs_with_lines(lines: &[&str]) -> Vfs {
        let mut vfs = Vfs::new();
        let content = lines.join("\n");
        vfs.write_file("/tmp/f.txt", &content).unwrap();
        vfs
    }

    #[test]
    fn basic_match() {
        let vfs = vfs_with_lines(&["hello", "world", "hello again"]);
        let out = execute(&vfs, &["hello", "/tmp/f.txt"], None).unwrap();
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn no_match_returns_empty() {
        let vfs = vfs_with_lines(&["hello", "world"]);
        let out = execute(&vfs, &["xyz", "/tmp/f.txt"], None).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn case_insensitive() {
        let vfs = vfs_with_lines(&["Hello", "WORLD", "hello"]);
        let out = execute(&vfs, &["-i", "hello", "/tmp/f.txt"], None).unwrap();
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn line_numbers() {
        let vfs = vfs_with_lines(&["aaa", "bbb", "aaa"]);
        let out = execute(&vfs, &["-n", "aaa", "/tmp/f.txt"], None).unwrap();
        assert!(out.contains("1:"));
        assert!(out.contains("3:"));
    }

    #[test]
    fn combined_in_flags() {
        let vfs = vfs_with_lines(&["Hello", "world"]);
        let out = execute(&vfs, &["-in", "hello", "/tmp/f.txt"], None).unwrap();
        assert!(out.contains("1:"));
    }

    #[test]
    fn combined_ni_flags() {
        let vfs = vfs_with_lines(&["Hello", "world"]);
        let out = execute(&vfs, &["-ni", "hello", "/tmp/f.txt"], None).unwrap();
        assert!(out.contains("1:"));
    }

    #[test]
    fn multiple_files_shows_filename() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "hello").unwrap();
        vfs.write_file("/tmp/b.txt", "hello").unwrap();
        let out = execute(&vfs, &["hello", "/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        assert!(out.contains("/tmp/a.txt:"));
        assert!(out.contains("/tmp/b.txt:"));
    }

    #[test]
    fn missing_pattern() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[], None).is_err());
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["pattern"], None).is_err());
    }
}
