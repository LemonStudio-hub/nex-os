//! `cp` -- copy files or directories within the virtual filesystem.
//!
//! # Usage
//!
//! ```text
//! cp <source> <destination>
//! ```
//!
//! Copies the file or directory at `source` to `destination`.  If
//! `destination` is an existing directory, the source is copied *into* that
//! directory (preserving its original name).  The source file remains intact.
//!
//! # Examples
//!
//! ```text
//! cp /tmp/a.txt /tmp/b.txt          # copy file to new name
//! cp /tmp/a.txt /tmp/backup/        # copy file into directory
//! ```
//!
//! # Errors
//!
//! - Fewer than two arguments.
//! - Source path does not exist.
//! - VFS-level copy errors (e.g. destination already exists as a file in some
//!   implementations).

use crate::vfs::permissions::{check_access, AccessMode};
use crate::vfs::{HostFs, Vfs};

/// Execute the `cp` command.
///
/// Resolves both source and destination paths, checks permissions, then
/// performs the copy.  The copy is owned by the current user.
pub fn execute(
    vfs: &mut Vfs,
    args: &[&str],
    uid: u32,
    gid: u32,
    host_fs: Option<&dyn HostFs>,
) -> Result<String, String> {
    if args.len() < 2 {
        return Err("cp: missing destination operand".to_string());
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
            "cp: cannot stat '{}': No such file or directory",
            src
        ));
    }

    // Permission checks (skip for host-mounted paths)
    if host_fs.is_none() || vfs.find_mount(&src_resolved).is_none() {
        // Read permission on source
        if let Some(meta) = vfs.get_meta(&src_resolved) {
            check_access(meta, AccessMode::Read, uid, gid)?;
        }
    }

    // Determine the actual destination
    let actual_dst = if vfs
        .is_dir_with_host(&dst_resolved, host_fs)
        .unwrap_or(false)
    {
        let basename = src_resolved.rsplit('/').next().unwrap_or(&src_resolved);
        format!("{}/{}", dst_resolved.trim_end_matches('/'), basename)
    } else {
        dst_resolved.clone()
    };

    // Write permission on destination parent
    let dst_parent = match actual_dst.rfind('/') {
        Some(0) => "/".to_string(),
        Some(i) => actual_dst[..i].to_string(),
        None => return Err("cp: invalid destination path".to_string()),
    };
    if host_fs.is_none() || vfs.find_mount(&dst_parent).is_none() {
        if let Some(meta) = vfs.get_meta(&dst_parent) {
            check_access(meta, AccessMode::Write, uid, gid)?;
        }
    }

    if vfs
        .is_dir_with_host(&src_resolved, host_fs)
        .unwrap_or(false)
    {
        copy_dir_recursive(vfs, &src_resolved, &actual_dst, uid, gid, host_fs)?;
    } else {
        let content = vfs
            .read_file_with_host(&src_resolved, host_fs)
            .map_err(|e| format!("cp: {}", e))?;
        vfs.write_file_with_host(&actual_dst, &content, host_fs)
            .map_err(|e| format!("cp: {}", e))?;
        // Set ownership of the copied file to the current user
        if let Some(meta) = vfs.get_meta_mut(&actual_dst) {
            meta.uid = uid;
            meta.gid = gid;
        }
    }

    Ok(String::new())
}

/// Recursively copy a directory from `src` to `dst`.
fn copy_dir_recursive(
    vfs: &mut Vfs,
    src: &str,
    dst: &str,
    uid: u32,
    gid: u32,
    host_fs: Option<&dyn HostFs>,
) -> Result<String, String> {
    vfs.mkdir_with_host(dst, host_fs)?;
    // Set ownership of copied directory
    if let Some(meta) = vfs.get_meta_mut(dst) {
        meta.uid = uid;
        meta.gid = gid;
    }

    let entries = vfs
        .list_dir_with_host(src, host_fs)
        .map_err(|e| format!("cp: {}", e))?;

    for entry in entries {
        let name = entry.name();
        let child_src = format!("{}/{}", src.trim_end_matches('/'), name);
        let child_dst = format!("{}/{}", dst.trim_end_matches('/'), name);

        if entry.is_dir() {
            copy_dir_recursive(vfs, &child_src, &child_dst, uid, gid, host_fs)?;
        } else {
            let content = vfs
                .read_file_with_host(&child_src, host_fs)
                .map_err(|e| format!("cp: {}", e))?;
            vfs.write_file_with_host(&child_dst, &content, host_fs)
                .map_err(|e| format!("cp: {}", e))?;
            // Set ownership of copied file
            if let Some(meta) = vfs.get_meta_mut(&child_dst) {
                meta.uid = uid;
                meta.gid = gid;
            }
        }
    }

    Ok(String::new())
}

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct CpCommand;

/// Delegates to the standalone [`execute`] function, forwarding the mutable
/// VFS reference needed for the copy operation.
impl super::Command for CpCommand {
    fn name(&self) -> &'static str {
        "cp"
    }
    fn description(&self) -> &'static str {
        "Copy files or directories"
    }
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
        "cp source destination"
    }
    fn man_description(&self) -> &'static str {
        "Copy a file or directory from source to destination. If the destination is an existing directory, \
the source is copied into that directory preserving its original name. The source file remains intact \
after the copy operation."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["cp file.txt backup.txt", "cp file.txt /tmp/"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/src.txt", "data").unwrap();
        execute(&mut vfs, &["/tmp/src.txt", "/tmp/dst.txt"], 0, 0, None).unwrap();
        assert_eq!(vfs.read_file("/tmp/dst.txt").unwrap(), "data");
        assert_eq!(vfs.read_file("/tmp/src.txt").unwrap(), "data");
    }

    #[test]
    fn copy_into_directory() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "data").unwrap();
        vfs.mkdir("/tmp/dest").unwrap();
        execute(&mut vfs, &["/tmp/f.txt", "/tmp/dest"], 0, 0, None).unwrap();
        assert_eq!(vfs.read_file("/tmp/dest/f.txt").unwrap(), "data");
    }

    #[test]
    fn copy_nonexistent_errors() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/nope", "/tmp/dst"], 0, 0, None).is_err());
    }

    #[test]
    fn missing_destination() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/tmp/src"], 0, 0, None).is_err());
    }
}
