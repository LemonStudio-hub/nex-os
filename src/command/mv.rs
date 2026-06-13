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

use crate::vfs::permissions::{check_access, AccessMode};
use crate::vfs::{HostFs, Vfs};

/// Execute the `mv` command.
///
/// Resolves both source and destination paths, checks permissions (write on
/// source parent for deletion, write on dest parent for creation), then
/// performs the move.  Ownership is preserved for intra-VFS moves.
pub fn execute(
    vfs: &mut Vfs,
    args: &[&str],
    uid: u32,
    gid: u32,
    host_fs: Option<&dyn HostFs>,
) -> Result<String, String> {
    if args.len() < 2 {
        return Err("mv: missing destination operand".to_string());
    }

    let src = args[0];
    let dst = args[1];

    let src_resolved = vfs.resolve_path(src)?;
    let dst_resolved = vfs.resolve_path(dst)?;

    if !vfs
        .exists_with_host(&src_resolved, host_fs)
        .unwrap_or(false)
    {
        return Err(format!(
            "mv: cannot stat '{}': No such file or directory",
            src
        ));
    }

    // Permission checks (skip for host-mounted paths)
    if host_fs.is_none() || vfs.find_mount(&src_resolved).is_none() {
        // Write on source parent (for deletion)
        let src_parent = match src_resolved.rfind('/') {
            Some(0) => "/".to_string(),
            Some(i) => src_resolved[..i].to_string(),
            None => return Err("mv: invalid source path".to_string()),
        };
        if let Some(meta) = vfs.get_meta(&src_parent) {
            check_access(meta, AccessMode::Write, uid, gid)?;
        }
    }

    // Determine actual destination
    let actual_dst = if vfs
        .is_dir_with_host(&dst_resolved, host_fs)
        .unwrap_or(false)
    {
        let basename = src_resolved.rsplit('/').next().unwrap_or(&src_resolved);
        format!("{}/{}", dst_resolved.trim_end_matches('/'), basename)
    } else {
        dst_resolved.clone()
    };

    // Write on dest parent (for creation)
    let dst_parent = match actual_dst.rfind('/') {
        Some(0) => "/".to_string(),
        Some(i) => actual_dst[..i].to_string(),
        None => return Err("mv: invalid destination path".to_string()),
    };
    if host_fs.is_none() || vfs.find_mount(&dst_parent).is_none() {
        if let Some(meta) = vfs.get_meta(&dst_parent) {
            check_access(meta, AccessMode::Write, uid, gid)?;
        }
    }

    // Perform the move (ownership is preserved — the node itself moves)
    if vfs
        .is_dir_with_host(&src_resolved, host_fs)
        .unwrap_or(false)
    {
        copy_dir_recursive(vfs, &src_resolved, &actual_dst, host_fs)?;
        vfs.rm_recursive_with_host(&src_resolved, host_fs)?;
    } else {
        let content = vfs
            .read_file_with_host(&src_resolved, host_fs)
            .map_err(|e| format!("mv: {}", e))?;
        vfs.write_file_with_host(&actual_dst, &content, host_fs)
            .map_err(|e| format!("mv: {}", e))?;
        // Preserve original ownership
        let src_meta = vfs.get_meta(&src_resolved).cloned();
        if let (Some(meta), Some(dst_meta)) = (src_meta, vfs.get_meta_mut(&actual_dst)) {
            dst_meta.uid = meta.uid;
            dst_meta.gid = meta.gid;
            dst_meta.mode = meta.mode;
        }
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
        execute(
            &mut ctx.state.vfs,
            ctx.args,
            ctx.state.euid,
            ctx.state.gid,
            ctx.host_fs,
        )
        .into()
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
    fn move_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/old.txt", "data").unwrap();
        execute(&mut vfs, &["/tmp/old.txt", "/tmp/new.txt"], 0, 0, None).unwrap();
        assert!(!vfs.exists("/tmp/old.txt"));
        assert_eq!(vfs.read_file("/tmp/new.txt").unwrap(), "data");
    }

    #[test]
    fn move_nonexistent_errors() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/nope", "/tmp/dst"], 0, 0, None).is_err());
    }

    #[test]
    fn missing_destination() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/tmp/src"], 0, 0, None).is_err());
    }
}
