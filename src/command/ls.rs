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
//! | `-l` | Long format: show permissions, owner, group, size, and mtime. |
//!
//! # Description
//!
//! Lists the entries in the given directory.  If `path` is omitted, the
//! current working directory (`.`) is used.  If `path` points to a regular
//! file instead of a directory, just that file's name is printed.
//!
//! Entries are sorted alphabetically by name.  Directories are shown with
//! a trailing `/` to distinguish them from regular files.

use crate::vfs::permissions::format_mode;
use crate::vfs::{UserDatabase, Vfs};

/// Execute the `ls` command.
pub fn execute(
    vfs: &Vfs,
    args: &[&str],
    user_db: &UserDatabase,
    host_fs: Option<&dyn crate::vfs::HostFs>,
) -> Result<String, String> {
    let mut long_format = false;
    let mut path = ".";
    let mut path_set = false;

    for arg in args {
        if *arg == "-l" {
            long_format = true;
        } else if !path_set {
            path = arg;
            path_set = true;
        }
    }

    let resolved = vfs.resolve_path(path)?;

    if !vfs.exists_with_host(&resolved, host_fs).unwrap_or(false) {
        return Err(format!(
            "ls: cannot access '{}': No such file or directory",
            path
        ));
    }

    // If the target is a file, just print it.
    if !vfs.is_dir_with_host(&resolved, host_fs).unwrap_or(false) {
        let name = resolved
            .rfind('/')
            .map(|i| &resolved[i + 1..])
            .unwrap_or(&resolved);
        return if long_format {
            let meta = vfs.get_meta(&resolved).unwrap();
            let mode_str = format_mode(meta.mode, false);
            let owner = user_db
                .find_user_by_uid(meta.uid)
                .map(|e| e.username.as_str())
                .unwrap_or("unknown");
            let group = user_db
                .find_group_by_gid(meta.gid)
                .map(|e| e.groupname.as_str())
                .unwrap_or("unknown");
            let size = vfs.file_size(&resolved).unwrap_or(0);
            let mtime_str = format_timestamp(meta.mtime);
            Ok(format!(
                "{}  1 {:<8} {:<8} {:>8} {} {}\n",
                mode_str, owner, group, size, mtime_str, name
            ))
        } else {
            Ok(format!("{}\n", name))
        };
    }

    let entries = vfs.list_dir_with_host(&resolved, host_fs)?;

    let mut sorted = entries;
    sorted.sort_by(|a, b| a.name().cmp(b.name()));

    if long_format {
        let mut output = String::new();
        for entry in &sorted {
            let meta = entry.meta();
            let mode_str = format_mode(meta.mode, entry.is_dir());
            let owner = user_db
                .find_user_by_uid(meta.uid)
                .map(|e| e.username.as_str())
                .unwrap_or("unknown");
            let group = user_db
                .find_group_by_gid(meta.gid)
                .map(|e| e.groupname.as_str())
                .unwrap_or("unknown");
            // For directories, show child count; for files, show content size.
            let (size, suffix) = if entry.is_dir() {
                let child_count = match vfs.list_dir(&format!(
                    "{}/{}",
                    resolved.trim_end_matches('/'),
                    entry.name()
                )) {
                    Ok(children) => children.len(),
                    Err(_) => 0,
                };
                (child_count, "/")
            } else {
                let file_path = format!("{}/{}", resolved.trim_end_matches('/'), entry.name());
                let sz = vfs.file_size(&file_path).unwrap_or(0);
                (sz, "")
            };
            let mtime_str = format_timestamp(meta.mtime);
            output.push_str(&format!(
                "{}  1 {:<8} {:<8} {:>8} {} {}{}\n",
                mode_str,
                owner,
                group,
                size,
                mtime_str,
                entry.name(),
                suffix
            ));
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
            Ok("\n".to_string())
        } else {
            Ok(format!("{}\n", names.join("  ")))
        }
    }
}

/// Format a Unix timestamp as a human-readable date string.
///
/// Returns `"Mon DD HH:MM"` format (simplified — no year).
fn timestamp_to_datetime(ts: u64) -> String {
    // Simple conversion: days since epoch to date
    let secs = ts as i64;
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;

    // Compute year/month/day from days since epoch (1970-01-01)
    let (_year, month, day) = days_to_ymd(days);

    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month_str = month_names.get((month - 1) as usize).unwrap_or(&"???");

    format!("{} {:>2} {:02}:{:02}", month_str, day, hour, minute)
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    let mut y = 1970;
    let mut remaining = days;

    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }

    let leap = is_leap(y);
    let days_in_month = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut m = 1;
    for &dim in &days_in_month {
        if remaining < dim {
            break;
        }
        remaining -= dim;
        m += 1;
    }

    (y, m, remaining + 1)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Format a timestamp for display, or return a placeholder for epoch 0.
fn format_timestamp(ts: u64) -> String {
    if ts == 0 {
        // Legacy data with no timestamp
        "Jan  1 00:00".to_string()
    } else {
        timestamp_to_datetime(ts)
    }
}

/// Unit struct representing the `ls` command.
pub struct LsCommand;

impl super::Command for LsCommand {
    fn name(&self) -> &'static str {
        "ls"
    }

    fn description(&self) -> &'static str {
        "List directory contents (-l for long format)"
    }

    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, &ctx.state.user_db, ctx.host_fs).into()
    }

    fn synopsis(&self) -> &'static str {
        "ls [-l] [path]"
    }
    fn man_description(&self) -> &'static str {
        "List the contents of a directory. If no path is given, the current working directory is used. \
The -l flag enables long format output showing permissions, owner, group, size, and modification time."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["ls", "ls -l /home", "ls -l .."]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_directory() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "").unwrap();
        vfs.mkdir("/tmp/sub").unwrap();
        let user_db = UserDatabase::from_vfs(&vfs);
        let out = execute(&vfs, &["/tmp"], &user_db, None).unwrap();
        assert!(out.contains("a.txt"));
        assert!(out.contains("sub/"));
    }

    #[test]
    fn list_file_shows_name() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "").unwrap();
        let user_db = UserDatabase::from_vfs(&vfs);
        let out = execute(&vfs, &["/tmp/f.txt"], &user_db, None).unwrap();
        assert!(out.contains("f.txt"));
    }

    #[test]
    fn long_format() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "hello").unwrap();
        vfs.mkdir("/tmp/d").unwrap();
        let user_db = UserDatabase::from_vfs(&vfs);
        let out = execute(&vfs, &["-l", "/tmp"], &user_db, None).unwrap();
        // Should contain permission strings
        assert!(out.contains("rw"));
        // Should contain owner/group (unknown in test VFS without /etc/passwd)
        assert!(out.contains("unknown"));
        // Should contain file size
        assert!(out.contains("5")); // "hello" is 5 bytes
                                    // Should contain the file name
        assert!(out.contains("f.txt"));
    }

    #[test]
    fn empty_directory() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/tmp/empty").unwrap();
        let user_db = UserDatabase::from_vfs(&vfs);
        let out = execute(&vfs, &["/tmp/empty"], &user_db, None).unwrap();
        assert!(!out.is_empty());
    }

    #[test]
    fn nonexistent_path() {
        let vfs = Vfs::new();
        let user_db = UserDatabase::from_vfs(&vfs);
        assert!(execute(&vfs, &["/nonexistent"], &user_db, None).is_err());
    }

    #[test]
    fn sorted_output() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/b.txt", "").unwrap();
        vfs.write_file("/tmp/a.txt", "").unwrap();
        vfs.write_file("/tmp/c.txt", "").unwrap();
        let user_db = UserDatabase::from_vfs(&vfs);
        let out = execute(&vfs, &["/tmp"], &user_db, None).unwrap();
        let a = out.find("a.txt").unwrap();
        let b = out.find("b.txt").unwrap();
        let c = out.find("c.txt").unwrap();
        assert!(a < b);
        assert!(b < c);
    }
}
