//! `rm` command -- remove files or directories from the VFS.
//!
//! # Usage
//!
//! ```text
//! rm [-r | -rf | -fr] <target> [target2 ...]
//! ```
//!
//! # Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `-r` | Remove directories and their contents recursively. |
//! | `-rf` | Same as `-r` (accepted for convenience). |
//! | `-fr` | Same as `-r` (accepted for convenience). |
//!
//! # Description
//!
//! Removes one or more files or directories.  Without the `-r` flag, only
//! regular files can be removed -- attempting to remove a directory without
//! `-r` produces an error ("Is a directory"), matching real `rm` behaviour.
//!
//! With `-r`, directories are removed recursively via
//! [`Vfs::rm_recursive`](crate::vfs::Vfs::rm_recursive).
//!
//! # Examples
//!
//! ```text
//! $ rm notes.txt
//! $ rm -r old_project/
//! $ rm -rf build dist
//! ```
//!
//! # Errors
//!
//! * No target arguments provided.
//! * Target does not exist.
//! * Target is a directory and `-r` was not specified.

use crate::vfs::{HostFs, Vfs};

/// Execute the `rm` command.
///
/// Separates flags from path arguments, then removes each target.  Files
/// are removed with [`Vfs::rm`]; directories require the recursive flag
/// and use [`Vfs::rm_recursive`].
///
/// # Arguments
///
/// * `vfs` -- Mutable reference to the virtual file system.
/// * `args` -- Command-line arguments (flags + target paths).
/// * `host_fs` -- Optional host filesystem adapter for mounted directories.
///
/// # Returns
///
/// `Ok(String::new())` on success (rm produces no output), or
/// `Err(message)` describing the failure.
pub fn execute(vfs: &mut Vfs, args: &[&str], host_fs: Option<&dyn HostFs>) -> Result<String, String> {
    let mut recursive = false;
    let mut paths: Vec<&str> = Vec::new();

    // Recognise all three recursive flag variants.  In real `rm`, `-rf`
    // and `-fr` are both common patterns, so we accept them for
    // compatibility with muscle memory.
    for arg in args {
        match *arg {
            "-r" | "-rf" | "-fr" => recursive = true,
            _ => paths.push(arg),
        }
    }

    if paths.is_empty() {
        return Err("rm: missing operand".to_string());
    }

    for path in paths {
        let resolved = vfs.resolve_path(path)?;

        if !vfs.exists_with_host(&resolved, host_fs).unwrap_or(false) {
            return Err(format!(
                "rm: cannot remove '{}': No such file or directory",
                path
            ));
        }

        // Directories cannot be removed without the recursive flag,
        // matching real `rm` which prints "Is a directory" in this case.
        if vfs.is_dir_with_host(&resolved, host_fs).unwrap_or(false) && !recursive {
            return Err(format!("rm: cannot remove '{}': Is a directory", path));
        }

        if recursive {
            vfs.rm_recursive_with_host(&resolved, host_fs)?;
        } else {
            vfs.rm_with_host(&resolved, host_fs)?;
        }
    }

    Ok(String::new())
}

/// Unit struct representing the `rm` command.
pub struct RmCommand;

/// Bridges the registry's [`Command`](super::Command) interface to the
/// module-level `execute` function.
impl super::Command for RmCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str {
        "rm"
    }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str {
        "Remove files or directories (-r for recursive)"
    }

    /// Execute the command, forwarding VFS and arguments from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&mut ctx.state.vfs, ctx.args, ctx.host_fs)
    }

    fn synopsis(&self) -> &'static str {
        "rm [-r] target [target2 ...]"
    }
    fn man_description(&self) -> &'static str {
        "Remove one or more files or directories. Without -r, only regular files can be removed; \
attempting to remove a directory without -r produces an 'Is a directory' error. The -r flag enables \
recursive removal of directories and all their contents. The variants -rf and -fr are also accepted \
for convenience."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["rm file.txt", "rm -r directory"]
    }
}
