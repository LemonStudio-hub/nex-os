//! `mv` command -- move or rename files and directories.
//!
//! # Usage
//!
//! ```text
//! mv <source> <destination>
//! ```
//!
//! # Description
//!
//! Moves (renames) a file or directory from `<source>` to `<destination>`.
//! The source is removed after the move.  If the destination is an existing
//! directory, the source is moved **into** that directory (preserving its
//! basename).
//!
//! The heavy lifting is delegated to [`Vfs::mv`](crate::vfs::Vfs::mv),
//! which handles both file and directory moves.
//!
//! # Examples
//!
//! ```text
//! $ mv old.txt new.txt
//! $ mv file.txt /tmp/
//! $ mv project/ archive/project_old/
//! ```
//!
//! # Errors
//!
//! * Fewer than two arguments (missing destination).
//! * Source path does not exist.

use crate::vfs::{HostFs, Vfs};

/// Execute the `mv` command.
///
/// Resolves both source and destination paths, verifies the source exists,
/// then performs a copy + remove using `_with_host` variants so that
/// mounted host directories are transparently supported.
///
/// # Arguments
///
/// * `vfs` -- Mutable reference to the virtual file system.
/// * `args` -- Command-line arguments: `[source, destination]`.
/// * `host_fs` -- Optional host filesystem adapter for mounted directories.
///
/// # Returns
///
/// `Ok(String::new())` on success (mv produces no output), or
/// `Err(message)` if the source is missing or the move fails.
pub fn execute(vfs: &mut Vfs, args: &[&str], host_fs: Option<&dyn HostFs>) -> Result<String, String> {
    // Both source and destination are required.
    if args.len() < 2 {
        return Err("mv: missing destination operand".to_string());
    }

    let src = args[0];
    let dst = args[1];

    // Resolve to absolute VFS paths so the Vfs layer doesn't need to
    // handle relative path logic.
    let src_resolved = vfs.resolve_path(src)?;
    let dst_resolved = vfs.resolve_path(dst)?;

    if !vfs.exists_with_host(&src_resolved, host_fs).unwrap_or(false) {
        return Err(format!(
            "mv: cannot stat '{}': No such file or directory",
            src
        ));
    }

    // Determine the actual destination: if dst is an existing directory,
    // move into it preserving the source basename.
    let actual_dst = if vfs.is_dir_with_host(&dst_resolved, host_fs).unwrap_or(false) {
        let basename = src_resolved.rsplit('/').next().unwrap_or(&src_resolved);
        format!("{}/{}", dst_resolved.trim_end_matches('/'), basename)
    } else {
        dst_resolved.clone()
    };

    // Copy the source to the destination using _with_host variants.
    if vfs.is_dir_with_host(&src_resolved, host_fs).unwrap_or(false) {
        copy_dir_recursive(vfs, &src_resolved, &actual_dst, host_fs)?;
        vfs.rm_recursive_with_host(&src_resolved, host_fs)?;
    } else {
        let content = vfs
            .read_file_with_host(&src_resolved, host_fs)
            .map_err(|e| format!("mv: {}", e))?;
        vfs.write_file_with_host(&actual_dst, &content, host_fs)
            .map_err(|e| format!("mv: {}", e))?;
        vfs.rm_with_host(&src_resolved, host_fs)?;
    }

    Ok(String::new())
}

/// Recursively copy a directory from `src` to `dst`.
fn copy_dir_recursive(
    vfs: &mut Vfs,
    src: &str,
    dst: &str,
    host_fs: Option<&dyn HostFs>,
) -> Result<String, String> {
    vfs.mkdir_with_host(dst, host_fs)?;

    let entries = vfs
        .list_dir_with_host(src, host_fs)
        .map_err(|e| format!("mv: {}", e))?;

    for entry in entries {
        let name = entry.name();
        let child_src = format!("{}/{}", src.trim_end_matches('/'), name);
        let child_dst = format!("{}/{}", dst.trim_end_matches('/'), name);

        if entry.is_dir() {
            copy_dir_recursive(vfs, &child_src, &child_dst, host_fs)?;
        } else {
            let content = vfs
                .read_file_with_host(&child_src, host_fs)
                .map_err(|e| format!("mv: {}", e))?;
            vfs.write_file_with_host(&child_dst, &content, host_fs)
                .map_err(|e| format!("mv: {}", e))?;
        }
    }

    Ok(String::new())
}

/// Unit struct representing the `mv` command.
pub struct MvCommand;

/// Bridges the registry's [`Command`](super::Command) interface to the
/// module-level `execute` function.
impl super::Command for MvCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str {
        "mv"
    }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str {
        "Move or rename files and directories"
    }

    /// Execute the command, forwarding VFS and arguments from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&mut ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }

    fn synopsis(&self) -> &'static str {
        "mv source destination"
    }
    fn man_description(&self) -> &'static str {
        "Move or rename a file or directory from source to destination. The source is removed after the move. \
If the destination is an existing directory, the source is moved into that directory preserving its basename."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["mv old.txt new.txt", "mv file.txt /tmp/"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Moving a file should remove the source and create the destination
    /// with identical content.
    fn move_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/old.txt", "data").unwrap();
        execute(&mut vfs, &["/tmp/old.txt", "/tmp/new.txt"], None).unwrap();
        assert!(!vfs.exists("/tmp/old.txt"));
        assert_eq!(vfs.read_file("/tmp/new.txt").unwrap(), "data");
    }

    #[test]
    /// Moving a non-existent source should produce an error.
    fn move_nonexistent_errors() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/nope", "/tmp/dst"], None).is_err());
    }

    #[test]
    /// Omitting the destination argument should produce an error.
    fn missing_destination() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/tmp/src"], None).is_err());
    }
}
