//! `mkdir` command -- create new directories in the VFS.
//!
//! # Usage
//!
//! ```text
//! mkdir [-p] <directory> [directory2 ...]
//! ```
//!
//! # Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `-p` | Create parent directories as needed.  Does not error if the directory already exists. |
//!
//! # Description
//!
//! Creates one or more directories at the specified paths.
//!
//! Without `-p`, the parent directory must already exist and the target
//! must not already exist; otherwise an error is returned.
//!
//! With `-p`, each path component is created on demand, walking from the
//! root down to the leaf.  Already-existing components are silently
//! skipped, which is why `-p` is commonly used in scripts to avoid race
//! conditions and to create deep trees in one call.
//!
//! # Examples
//!
//! ```text
//! $ mkdir projects
//! $ mkdir -p src/utils/helpers
//! $ mkdir a b c
//! ```
//!
//! # Errors
//!
//! * No path arguments provided.
//! * Without `-p`: target already exists, or parent directory is missing.
//! * Invalid path (no leading `/` after resolution).

use crate::vfs::{HostFs, Vfs};

/// Execute the `mkdir` command.
///
/// Separates flags from path arguments, then creates directories either
/// non-recursively (single level, with existence checks) or recursively
/// (walking path components from root to leaf).
///
/// # Arguments
///
/// * `vfs` -- Mutable reference to the virtual file system.
/// * `args` -- Command-line arguments (flags + path tokens).
/// * `host_fs` -- Optional host filesystem adapter for mounted directories.
///
/// # Returns
///
/// `Ok(String::new())` on success (mkdir produces no output), or
/// `Err(message)` describing the failure.
pub fn execute(
    vfs: &mut Vfs,
    args: &[&str],
    uid: u32,
    gid: u32,
    host_fs: Option<&dyn HostFs>,
) -> Result<String, String> {
    let mut recursive = false;
    let mut paths: Vec<&str> = Vec::new();

    // Separate the `-p` flag from positional path arguments.
    for arg in args {
        if *arg == "-p" {
            recursive = true;
        } else {
            paths.push(arg);
        }
    }

    if paths.is_empty() {
        return Err("mkdir: missing operand".to_string());
    }

    for path in paths {
        let resolved = vfs.resolve_path(path)?;

        if recursive {
            // Walk each path component from root to leaf, creating any that
            // don't exist yet.  This mirrors `mkdir -p` which never fails
            // on an already-existing intermediate directory.
            let components: Vec<&str> = resolved
                .split('/')
                .filter(|s: &&str| !s.is_empty())
                .collect();
            let mut current = String::new();
            for component in components {
                current.push('/');
                current.push_str(component);
                if !vfs.exists_with_host(&current, host_fs).unwrap_or(false) {
                    vfs.mkdir_with_host_and_owner(&current, host_fs, uid, gid)?;
                }
            }
        } else {
            // Non-recursive mode: the directory must not already exist.
            if vfs.exists_with_host(&resolved, host_fs).unwrap_or(false) {
                return Err(format!(
                    "mkdir: cannot create directory '{}': File exists",
                    path
                ));
            }

            // Extract the parent path.  For "/foo/bar" the parent is "/foo";
            // for "/foo" the parent is "/".  A path with no '/' is invalid
            // because resolve_path always produces absolute paths.
            let parent = match resolved.rfind('/') {
                Some(0) => "/".to_string(),
                Some(i) => resolved[..i].to_string(),
                None => return Err("mkdir: invalid path".to_string()),
            };

            // The parent must exist and must be a directory.
            if !vfs.exists_with_host(&parent, host_fs).unwrap_or(false)
                || !vfs.is_dir_with_host(&parent, host_fs).unwrap_or(false)
            {
                return Err(format!(
                    "mkdir: cannot create directory '{}': No such file or directory",
                    path
                ));
            }

            vfs.mkdir_with_host_and_owner(&resolved, host_fs, uid, gid)?;
        }
    }

    Ok(String::new())
}

/// Unit struct representing the `mkdir` command.
pub struct MkdirCommand;

/// Bridges the registry's [`Command`](super::Command) interface to the
/// module-level `execute` function.
impl super::Command for MkdirCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str {
        "mkdir"
    }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str {
        "Create directories (-p for recursive)"
    }

    /// Execute the command, forwarding VFS and arguments from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(
            &mut ctx.state.vfs,
            ctx.args,
            ctx.state.uid,
            ctx.state.gid,
            ctx.host_fs,
        )
        .into()
    }

    fn synopsis(&self) -> &'static str {
        "mkdir [-p] directory [directory2 ...]"
    }
    fn man_description(&self) -> &'static str {
        "Create one or more new directories at the specified paths. Without -p, the parent directory must already exist \
and the target must not already exist. With -p, parent directories are created as needed and already-existing \
directories are silently skipped, making it safe for creating deep directory trees in a single call."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["mkdir test", "mkdir -p path/to/dir"]
    }
}
