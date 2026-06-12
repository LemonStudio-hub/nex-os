//! `ls` command -- list directory contents.
//!
//! # Usage
//!
//! ```text
//! ls [-l] [path]
//! ```
//!
//! # Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `-l` | Long format: prepend each entry with `d ` (directory) or `- ` (file). |
//!
//! # Description
//!
//! Lists the entries in the given directory.  If `path` is omitted, the
//! current working directory (`.`) is used.  If `path` points to a regular
//! file instead of a directory, just that file's name is printed.
//!
//! Entries are sorted alphabetically by name.  Directories are shown with
//! a trailing `/` to distinguish them from regular files.
//!
//! # Examples
//!
//! ```text
//! $ ls
//! a.txt  subdir/
//! $ ls -l /tmp
//! d subdir/
//! - a.txt
//! ```
//!
//! # Notes
//!
//! Unlike real `ls`, there is no support for hidden files (dotfiles are
//! not special), no colour output, and no flags beyond `-l`.

use crate::vfs::Vfs;

/// Execute the `ls` command.
///
/// Resolves the target path, then lists its contents (or prints the file
/// name if the path points to a single file).  Entries are sorted
/// alphabetically.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual file system.
/// * `args` -- Command-line arguments (flags and optional path).
///
/// # Returns
///
/// `Ok(output)` with the formatted listing, or `Err` if the path does not
/// exist or cannot be read.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut long_format = false;
    let mut path = ".";
    let mut path_set = false;

    // Parse flags and capture the first non-flag argument as the path.
    // Only one path is supported; additional args are silently ignored.
    for arg in args {
        if *arg == "-l" {
            long_format = true;
        } else if !path_set {
            path = arg;
            path_set = true;
        }
    }

    let resolved = vfs.resolve_path(path)?;

    if !vfs.exists(&resolved) {
        return Err(format!(
            "ls: cannot access '{}': No such file or directory",
            path
        ));
    }

    // If the target is a file (not a directory), just print its basename.
    // Extract the last component after the final '/' for display.
    if !vfs.is_dir(&resolved) {
        let name = resolved
            .rfind('/')
            .map(|i| &resolved[i + 1..])
            .unwrap_or(&resolved);
        return if long_format {
            // Long format for a single file uses the "-" (regular file) prefix.
            Ok(format!("- {}\n", name))
        } else {
            Ok(format!("{}\n", name))
        };
    }

    let entries = vfs.list_dir(&resolved)?;

    // Sort entries alphabetically so the output is deterministic and
    // matches the typical `ls` behaviour users expect.
    let mut sorted = entries;
    sorted.sort_by(|a, b| a.name().cmp(b.name()));

    if long_format {
        let mut output = String::new();
        for entry in &sorted {
            // Prefix with "d" for directories or "-" for files, mirroring
            // the first character of `ls -l` permission strings on real systems.
            let prefix = if entry.is_dir() { "d " } else { "- " };
            // Append a trailing "/" to directory names for visual distinction.
            let suffix = if entry.is_dir() { "/" } else { "" };
            output.push_str(&format!("{}{}{}\n", prefix, entry.name(), suffix));
        }
        Ok(output)
    } else {
        let names: Vec<String> = sorted
            .iter()
            .map(|entry| {
                if entry.is_dir() {
                    format!("{}/", entry.name())
                } else {
                    entry.name().to_string()
                }
            })
            .collect();
        if names.is_empty() {
            // An empty directory still produces a newline so the user gets
            // visual feedback that the command ran successfully.
            Ok("\n".to_string())
        } else {
            // Join names with two-space separators for readability.
            Ok(format!("{}\n", names.join("  ")))
        }
    }
}

/// Unit struct representing the `ls` command.
pub struct LsCommand;

/// Bridges the registry's [`Command`](super::Command) interface to the
/// module-level `execute` function.
impl super::Command for LsCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str { "ls" }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str { "List directory contents (-l for long format)" }

    /// Execute the command, forwarding VFS and arguments from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.vfs, ctx.args)
    }

    fn synopsis(&self) -> &'static str { "ls [-l] [path]" }
    fn man_description(&self) -> &'static str {
        "List the contents of a directory. If no path is given, the current working directory is used. \
If the path points to a regular file rather than a directory, just that file's name is printed. \
Entries are sorted alphabetically and directories are shown with a trailing /. \
The -l flag enables long format output, which prefixes each entry with a type indicator: 'd' for directories and '-' for regular files."
    }
    fn examples(&self) -> &'static [&'static str] { &["ls", "ls -l /home", "ls -l .."] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// A directory listing should include both files and subdirectories.
    fn list_directory() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "").unwrap();
        vfs.mkdir("/tmp/sub").unwrap();
        let out = execute(&vfs, &["/tmp"]).unwrap();
        assert!(out.contains("a.txt"));
        assert!(out.contains("sub/"));
    }

    #[test]
    /// Listing a file path should print just that file's name, not error.
    fn list_file_shows_name() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "").unwrap();
        let out = execute(&vfs, &["/tmp/f.txt"]).unwrap();
        assert!(out.contains("f.txt"));
    }

    #[test]
    /// Long format should prepend type indicators ("d" or "-") to each entry.
    fn long_format() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "").unwrap();
        vfs.mkdir("/tmp/d").unwrap();
        let out = execute(&vfs, &["-l", "/tmp"]).unwrap();
        assert!(out.contains("- f.txt"));
        assert!(out.contains("d d/"));
    }

    #[test]
    /// An empty directory should still produce output (at minimum a newline).
    fn empty_directory() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/tmp/empty").unwrap();
        let out = execute(&vfs, &["/tmp/empty"]).unwrap();
        assert!(!out.is_empty()); // still outputs a newline
    }

    #[test]
    /// A non-existent path should return an error.
    fn nonexistent_path() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/nonexistent"]).is_err());
    }

    #[test]
    /// Output should list entries in alphabetical order.
    fn sorted_output() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/b.txt", "").unwrap();
        vfs.write_file("/tmp/a.txt", "").unwrap();
        vfs.write_file("/tmp/c.txt", "").unwrap();
        let out = execute(&vfs, &["/tmp"]).unwrap();
        let a = out.find("a.txt").unwrap();
        let b = out.find("b.txt").unwrap();
        let c = out.find("c.txt").unwrap();
        assert!(a < b);
        assert!(b < c);
    }
}
