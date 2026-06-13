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

use crate::vfs::{HostFs, Vfs};

/// Execute the `cp` command.
///
/// Resolves both source and destination paths, verifies the source exists, then
/// performs the copy. Uses `_with_host` variants so that mounted host
/// directories are transparently supported.
///
/// # Returns
///
/// `Ok(String::new())` -- successful `cp` produces no output, matching POSIX.
///
/// # Errors
///
/// Returns an error if the source does not exist or the underlying VFS copy
/// operation fails.
pub fn execute(
    vfs: &mut Vfs,
    args: &[&str],
    host_fs: Option<&dyn HostFs>,
) -> Result<String, String> {
    if args.len() < 2 {
        return Err("cp: missing destination operand".to_string());
    }

    let src = args[0];
    let dst = args[1];

    // Resolve both paths to absolute VFS paths before checking existence or
    // copying.  This handles relative paths, `..`, and `~` uniformly.
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

    // Determine the actual destination: if dst is an existing directory,
    // copy into it preserving the source basename.
    let actual_dst = if vfs
        .is_dir_with_host(&dst_resolved, host_fs)
        .unwrap_or(false)
    {
        let basename = src_resolved.rsplit('/').next().unwrap_or(&src_resolved);
        format!("{}/{}", dst_resolved.trim_end_matches('/'), basename)
    } else {
        dst_resolved.clone()
    };

    if vfs
        .is_dir_with_host(&src_resolved, host_fs)
        .unwrap_or(false)
    {
        // Directory copy: recursively copy contents using list_dir_with_host
        copy_dir_recursive(vfs, &src_resolved, &actual_dst, host_fs)?;
    } else {
        // File copy: read source, write to destination
        let content = vfs
            .read_file_with_host(&src_resolved, host_fs)
            .map_err(|e| format!("cp: {}", e))?;
        vfs.write_file_with_host(&actual_dst, &content, host_fs)
            .map_err(|e| format!("cp: {}", e))?;
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
    // Create the destination directory
    vfs.mkdir_with_host(dst, host_fs)?;

    // List source directory contents
    let entries = vfs
        .list_dir_with_host(src, host_fs)
        .map_err(|e| format!("cp: {}", e))?;

    for entry in entries {
        let name = entry.name();
        let child_src = format!("{}/{}", src.trim_end_matches('/'), name);
        let child_dst = format!("{}/{}", dst.trim_end_matches('/'), name);

        if entry.is_dir() {
            copy_dir_recursive(vfs, &child_src, &child_dst, host_fs)?;
        } else {
            let content = vfs
                .read_file_with_host(&child_src, host_fs)
                .map_err(|e| format!("cp: {}", e))?;
            vfs.write_file_with_host(&child_dst, &content, host_fs)
                .map_err(|e| format!("cp: {}", e))?;
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
        execute(&mut ctx.state.vfs, ctx.args, ctx.host_fs).into()
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
        execute(&mut vfs, &["/tmp/src.txt", "/tmp/dst.txt"], None).unwrap();
        assert_eq!(vfs.read_file("/tmp/dst.txt").unwrap(), "data");
        assert_eq!(vfs.read_file("/tmp/src.txt").unwrap(), "data"); // original intact
    }

    #[test]
    fn copy_into_directory() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "data").unwrap();
        vfs.mkdir("/tmp/dest").unwrap();
        execute(&mut vfs, &["/tmp/f.txt", "/tmp/dest"], None).unwrap();
        assert_eq!(vfs.read_file("/tmp/dest/f.txt").unwrap(), "data");
    }

    #[test]
    fn copy_nonexistent_errors() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/nope", "/tmp/dst"], None).is_err());
    }

    #[test]
    fn missing_destination() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/tmp/src"], None).is_err());
    }
}
