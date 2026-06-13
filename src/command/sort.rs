//! `sort` command -- sort lines of a file alphabetically.
//!
//! # Usage
//!
//! ```text
//! sort [-r] <file>
//! ```
//!
//! # Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `-r` | Reverse the sort order (Z before A). |
//!
//! # Description
//!
//! Reads the contents of `<file>`, splits into lines, sorts them
//! lexicographically, and prints the result.  With `-r`, the sort order
//! is reversed.
//!
//! This command also accepts piped stdin: when `accepts_stdin()` returns
//! `true`, the pipeline writes stdin to a temporary file and appends it
//! as a trailing argument if no explicit file was given.
//!
//! # Examples
//!
//! ```text
//! $ sort names.txt
//! $ sort -r names.txt
//! $ echo -e "banana\napple\ncherry" | sort
//! ```
//!
//! # Notes
//!
//! Sorting is case-sensitive (uppercase letters sort before lowercase,
//! per Unicode codepoint order).  There is no numeric sort (`-n`) or
//! locale-aware sort (`-k`).

use crate::vfs::Vfs;

/// Execute the `sort` command.
///
/// Parses the `-r` flag and the file path argument, reads the file from
/// the VFS, sorts its lines, and returns the sorted output.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual file system (read-only access).
/// * `args` -- Command-line arguments: optional `-r` flag and a file path.
///
/// # Returns
///
/// `Ok(sorted_output)` with lines joined by newlines, or `Err` if the
/// file argument is missing, the file does not exist, or too many
/// arguments are provided.
pub fn execute(
    vfs: &Vfs,
    args: &[&str],
    host_fs: Option<&dyn crate::vfs::HostFs>,
) -> Result<String, String> {
    let mut reverse = false;
    let mut file_path: Option<&str> = None;

    // Parse arguments: at most one flag and one file path.
    // Extra positional arguments are an error.
    for arg in args {
        match *arg {
            "-r" => reverse = true,
            // Capture the first non-flag arg as the file path.
            _ if file_path.is_none() => file_path = Some(arg),
            // A second positional arg is not allowed.
            _ => return Err("sort: too many arguments".to_string()),
        }
    }

    let path = file_path.ok_or("sort: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;
    let content = vfs.read_file_with_host(&resolved, host_fs)?;

    // Split into lines, sort in-place, then rejoin.
    // `sort()` uses the default lexicographic (Unicode codepoint) ordering.
    // For reverse, `sort_by` with `b.cmp(a)` inverts the comparison.
    let mut lines: Vec<&str> = content.lines().collect();
    if reverse {
        lines.sort_by(|a, b| b.cmp(a));
    } else {
        lines.sort();
    }

    Ok(format!("{}\n", lines.join("\n")))
}

/// Unit struct representing the `sort` command.
pub struct SortCommand;

/// Bridges the registry's [`Command`](super::Command) interface to the
/// module-level `execute` function.
impl super::Command for SortCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str {
        "sort"
    }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str {
        "Sort lines of a file (-r for reverse)"
    }

    /// Declares that this command can consume stdin from a pipe.
    ///
    /// When `true`, the pipeline stage writes stdin to a temp file and
    /// appends it as a trailing argument if the user didn't specify one.
    fn accepts_stdin(&self) -> bool {
        true
    }

    /// Execute the command, forwarding VFS and arguments from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }
    fn synopsis(&self) -> &'static str {
        "sort [-r] file"
    }
    fn man_description(&self) -> &'static str {
        "Sort lines of a file in lexicographic (Unicode codepoint) order and print the result. \
With -r, the sort order is reversed so that lines are printed from Z to A. Sorting is \
case-sensitive by default (uppercase letters sort before lowercase)."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["sort file.txt", "sort -r file.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a VFS with a single file containing the given lines.
    fn vfs_with_lines(lines: &[&str]) -> Vfs {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", &lines.join("\n")).unwrap();
        vfs
    }

    #[test]
    /// Lines should be sorted in ascending alphabetical order by default.
    fn basic_sort() {
        let vfs = vfs_with_lines(&["banana", "apple", "cherry"]);
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "apple");
        assert_eq!(lines[1], "banana");
        assert_eq!(lines[2], "cherry");
    }

    #[test]
    /// With `-r`, lines should be in descending (reverse) order.
    fn reverse_sort() {
        let vfs = vfs_with_lines(&["banana", "apple"]);
        let out = execute(&vfs, &["-r", "/tmp/f.txt"], None).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "banana");
        assert_eq!(lines[1], "apple");
    }

    #[test]
    /// A single-line file should return that line unchanged.
    fn single_line() {
        let vfs = vfs_with_lines(&["only"]);
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out.trim(), "only");
    }

    #[test]
    /// An empty file should not crash; output may be empty or a single newline.
    fn empty_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/empty.txt", "").unwrap();
        let out = execute(&vfs, &["/tmp/empty.txt"], None).unwrap();
        // Should not crash; output may be empty or a single newline
        let _ = out;
    }

    #[test]
    /// Omitting the file argument entirely should return an error.
    fn missing_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[], None).is_err());
    }

    #[test]
    /// Providing two file arguments should return an error.
    fn too_many_args() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["a.txt", "b.txt"], None).is_err());
    }
}
