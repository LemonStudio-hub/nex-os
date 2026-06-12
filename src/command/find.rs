//! `find` - search for files and directories by name pattern
//!
//! Recursively walks the VFS directory tree and returns paths of entries
//! whose names contain the given substring pattern. This is a simplified
//! version of the POSIX `find` command -- it supports substring matching
//! only (not glob patterns or regular expressions).
//!
//! # Usage
//!
//! ```text
//! find [path] -name <pattern>
//! ```
//!
//! # Behavior
//!
//! - If `path` is omitted, searches from the current working directory (`.`).
//! - The `-name` flag is required and must be followed by a pattern string.
//! - Matching is case-sensitive substring containment (`"foo"` matches
//!   `"foobar"`, `"afool"`, etc.).
//! - Both files and directories are matched; directories are also recursed
//!   into regardless of whether they match.
//! - Returns no output (empty string) when no matches are found.

use crate::vfs::Vfs;

/// Execute the `find` command.
///
/// Parses the search path and `-name` pattern from the arguments, then
/// initiates a recursive walk of the VFS tree to collect matching paths.
///
/// # Arguments
///
/// * `vfs` - The virtual filesystem to search.
/// * `args` - Command-line arguments: optional path and required `-name <pattern>`.
///
/// # Returns
///
/// Newline-separated list of matching paths, or an error if `-name` is missing.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut search_path = ".";
    let mut pattern: Option<&str> = None;

    // Manual index-based loop because we need to skip ahead by 2 when
    // consuming the `-name <pattern>` pair.
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-name" && i + 1 < args.len() {
            pattern = Some(args[i + 1]);
            i += 2; // Skip both the flag and its value
        } else if search_path == "." && !args[i].starts_with('-') {
            // The first non-flag argument is treated as the search path.
            // We only accept one path; subsequent non-flag args are ignored.
            search_path = args[i];
            i += 1;
        } else {
            i += 1;
        }
    }

    let pattern = pattern.ok_or("find: missing -name argument")?;
    let resolved = vfs.resolve_path(search_path)?;

    let mut results = Vec::new();
    collect_matches(vfs, &resolved, pattern, &mut results);

    if results.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", results.join("\n")))
    }
}

/// Recursively walk the directory tree and collect paths whose final
/// component (filename) contains `pattern` as a substring.
///
/// Uses `name.contains(pattern)` for matching -- this means "readme"
/// will match "readme.txt", "README.md", etc. The match is always
/// against the entry name only, not the full path.
fn collect_matches(vfs: &Vfs, dir_path: &str, pattern: &str, results: &mut Vec<String>) {
    let entries = match vfs.list_dir(dir_path) {
        Ok(e) => e,
        // If we can't list the directory (e.g., it's actually a file),
        // silently skip it rather than propagating an error mid-walk.
        Err(_) => return,
    };

    for entry in entries {
        let name = entry.name();
        let entry_path = Vfs::child_path(dir_path, name);

        // Check if the entry name contains the pattern substring.
        if name.contains(pattern) {
            results.push(entry_path.clone());
        }

        // Always recurse into subdirectories, even if the directory itself
        // matched -- its children may also match independently.
        if entry.is_dir() {
            collect_matches(vfs, &entry_path, pattern, results);
        }
    }
}

/// Unit struct implementing the [`super::Command`] trait for `find`.
pub struct FindCommand;

/// Registers `find` with the command system.
impl super::Command for FindCommand {
    fn name(&self) -> &'static str { "find" }
    fn description(&self) -> &'static str { "Find files by name (find [path] -name PATTERN)" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.vfs, ctx.args)
    }
    fn synopsis(&self) -> &'static str { "find [path] -name pattern" }
    fn man_description(&self) -> &'static str { "Recursively search for files and directories whose names contain the given substring pattern. If no path is specified, searches from the current working directory. The -name flag is required and must be followed by the pattern string." }
    fn examples(&self) -> &'static [&'static str] { &["find -name README", "find /home -name .txt", "find . -name config"] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_vfs() -> Vfs {
        let mut vfs = Vfs::new();
        vfs.mkdir("/tmp/search").unwrap();
        vfs.write_file("/tmp/search/readme.txt", "").unwrap();
        vfs.write_file("/tmp/search/data.csv", "").unwrap();
        vfs.write_file("/tmp/search/readme.md", "").unwrap();
        vfs.mkdir("/tmp/search/sub").unwrap();
        vfs.write_file("/tmp/search/sub/readme.log", "").unwrap();
        vfs
    }

    #[test]
    fn find_by_name() {
        let vfs = setup_vfs();
        let out = execute(&vfs, &["/tmp/search", "-name", "readme"]).unwrap();
        assert!(out.contains("readme.txt"));
        assert!(out.contains("readme.md"));
        assert!(out.contains("readme.log"));
        assert!(!out.contains("data.csv"));
    }

    #[test]
    fn find_no_results() {
        let vfs = setup_vfs();
        let out = execute(&vfs, &["/tmp/search", "-name", "nonexistent"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn find_from_current_dir() {
        let mut vfs = Vfs::new();
        vfs.cwd = "/tmp".to_string();
        vfs.write_file("/tmp/target.txt", "").unwrap();
        let out = execute(&vfs, &["-name", "target"]).unwrap();
        assert!(out.contains("target.txt"));
    }

    #[test]
    fn find_missing_name_arg() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/tmp"]).is_err());
    }
}
