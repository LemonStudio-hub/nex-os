//! `du` - estimate disk usage of files and directories
//!
//! Recursively walks the VFS tree and reports the byte size of each directory
//! entry. Sizes are based on in-memory content length (this is a simulated
//! filesystem, so there are no real disk blocks).
//!
//! # Usage
//!
//! ```text
//! du [-h] [-s] [path]
//! ```
//!
//! # Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `-h` | Human-readable output: sizes shown as `K` (kilobytes) or `M` (megabytes) |
//! | `-s` | Summary mode: only display the total for the specified path |
//!
//! # Behavior
//!
//! - Without a path argument, operates on the current working directory (`.`).
//! - Default (non-human) output displays sizes in whole kilobytes, matching
//!   the traditional `du` convention of showing 1K as the minimum unit.
//! - Files are sized by their content byte length; directories accumulate
//!   the sizes of all descendants.

use crate::vfs::{FsNode, Vfs};

/// Execute the `du` command.
///
/// Parses flags (`-h`, `-s`) and the optional path argument, then walks the
/// VFS tree to compute and format disk usage information.
///
/// # Arguments
///
/// * `vfs` - The virtual filesystem to inspect.
/// * `args` - Raw command-line arguments (flags and optional path).
///
/// # Returns
///
/// Formatted string of directory sizes, or an error for unknown options.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut human = false;
    let mut summary = false;
    // Default to current directory when no path is provided.
    let mut path = ".";

    // Parse each argument: known flags toggle their booleans; the first
    // non-flag argument is treated as the target path.
    for arg in args {
        match *arg {
            "-h" => human = true,
            "-s" => summary = true,
            _ if !arg.starts_with('-') => path = arg,
            _ => return Err(format!("du: unknown option: {}", arg)),
        }
    }

    let resolved = vfs.resolve_path(path)?;

    // In summary mode, skip per-entry output and print only the grand total.
    if summary {
        let total = dir_size(vfs, &resolved);
        return Ok(format!("{}\t{}\n", format_size(total, human), path));
    }

    // Collect per-subdirectory sizes, then append the overall total.
    let mut output = String::new();
    collect_sizes(vfs, &resolved, path, human, &mut output);
    let total = dir_size(vfs, &resolved);
    output.push_str(&format!("{}\t{}\n", format_size(total, human), path));
    Ok(output)
}

/// Recursively compute the total byte size of a directory (or file).
///
/// If `path` points to a file, returns its content length. If it points to
/// a directory, sums the sizes of all entries recursively. A `list_dir`
/// failure is silently treated as a file lookup -- this handles the edge
/// case where the path might be a file rather than a directory.
fn dir_size(vfs: &Vfs, path: &str) -> usize {
    let entries = match vfs.list_dir(path) {
        Ok(e) => e,
        Err(_) => {
            // Not a directory -- attempt to read as a file and return its
            // content length, or 0 if the read also fails.
            return vfs.read_file(path).map(|c| c.len()).unwrap_or(0);
        }
    };

    let mut total = 0;
    for entry in entries {
        let entry_path = Vfs::child_path(path, entry.name());

        match entry {
            FsNode::File(f) => total += f.content.len(),
            // Recurse into subdirectories to accumulate their total.
            FsNode::Directory(_) => total += dir_size(vfs, &entry_path),
        }
    }
    total
}

/// Recursively collect per-directory size entries into `output`.
///
/// For each subdirectory encountered, records its size and then recurses
/// deeper. Files themselves are not listed individually -- their sizes
/// contribute to their parent directory's total.
///
/// `display_path` tracks the user-visible path (relative or absolute),
/// while `abs_path` is the resolved VFS path used for lookups. The
/// distinction is needed so that output paths match what the user typed.
fn collect_sizes(vfs: &Vfs, abs_path: &str, display_path: &str, human: bool, output: &mut String) {
    let entries = match vfs.list_dir(abs_path) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries {
        let entry_abs = Vfs::child_path(abs_path, entry.name());
        // Build the display-friendly path: if the base is ".", don't
        // prefix with "./" -- just use the entry name directly.
        let entry_display = if display_path == "." {
            entry.name().to_string()
        } else {
            format!("{}/{}", display_path, entry.name())
        };

        // Only directories get their own line entry; files contribute
        // silently to their parent's total.
        if entry.is_dir() {
            let size = dir_size(vfs, &entry_abs);
            output.push_str(&format!(
                "{}\t{}\n",
                format_size(size, human),
                entry_display
            ));
            collect_sizes(vfs, &entry_abs, &entry_display, human, output);
        }
    }
}

/// Format a byte count into a display string.
///
/// In human-readable mode (`-h`), uses `M` for megabytes and `K` for
/// kilobytes, with one decimal place. In default mode, displays whole
/// kilobytes using `div_ceil` so that even a 1-byte file shows as `1K`
/// (matching traditional `du` behavior where the minimum unit is 1K).
fn format_size(bytes: usize, human: bool) -> String {
    if human {
        if bytes >= 1024 * 1024 {
            format!("{:.1}M", bytes as f64 / 1024.0 / 1024.0)
        } else if bytes >= 1024 {
            format!("{:.1}K", bytes as f64 / 1024.0)
        } else {
            format!("{}B", bytes)
        }
    } else {
        // Display in KB (like real `du`)
        let kb = bytes.div_ceil(1024);
        format!("{}K", kb)
    }
}

/// Unit struct implementing the [`super::Command`] trait for `du`.
pub struct DuCommand;

/// Registers `du` with the command system.
///
/// Delegates to the standalone `execute()` function, forwarding the
/// VFS reference and arguments from the shell context.
impl super::Command for DuCommand {
    fn name(&self) -> &'static str {
        "du"
    }
    fn description(&self) -> &'static str {
        "Estimate disk usage (-h human-readable, -s summary)"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.vfs, ctx.args)
    }
    fn synopsis(&self) -> &'static str {
        "du [-h] [-s] [path]"
    }
    fn man_description(&self) -> &'static str {
        "Estimate disk usage of files and directories by recursively walking the VFS tree. The -h flag displays sizes in human-readable format (K for kilobytes, M for megabytes). The -s flag shows only a summary total for the specified path rather than per-subdirectory breakdown."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["du", "du -h /home", "du -s ."]
    }
}
