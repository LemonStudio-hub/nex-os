//! `cd` -- change the current working directory.
//!
//! # Usage
//!
//! ```text
//! cd [directory]
//! ```
//!
//! Updates the VFS current working directory (`cwd`) to the given path.
//! With no arguments or with `~`, the working directory is set to the user's
//! home directory (`/home/user`).  The special path `/` navigates to the
//! filesystem root.
//!
//! # Errors
//!
//! - Target does not exist.
//! - Target exists but is not a directory.
//!
//! # Notes
//!
//! This command requires `&mut Vfs` because it writes to `vfs.cwd`.  It
//! produces no stdout -- successful `cd` is silent, matching POSIX behaviour.

use crate::vfs::Vfs;

/// Execute the `cd` command.
///
/// Resolves the target path (handling `~` and `/` as special cases), validates
/// that the path exists and is a directory, then sets `vfs.cwd` to the resolved
/// absolute path.
///
/// # Returns
///
/// Always returns `Ok(String::new())` on success -- `cd` produces no output.
///
/// # Errors
///
/// Returns an error if the target does not exist or is not a directory.
pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    // Determine the target directory.  No args or `~` both mean the home dir.
    // `/` is handled separately to avoid resolve_path overhead for a trivial case.
    let target = if args.is_empty() || args[0] == "~" {
        "/home/user".to_string()
    } else if args[0] == "/" {
        "/".to_string()
    } else {
        vfs.resolve_path(args[0])?
    };

    if !vfs.exists(&target) {
        // Show "~" in the error when no argument was given, so the user knows
        // what path was attempted.
        let display = if args.is_empty() { "~" } else { args[0] };
        return Err(format!("cd: {}: No such file or directory", display));
    }

    if !vfs.is_dir(&target) {
        return Err(format!("cd: {}: Not a directory", args[0]));
    }

    // Mutation: update the VFS current working directory.
    vfs.cwd = target;
    Ok(String::new())
}

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct CdCommand;

/// Delegates to the standalone [`execute`] function, passing the mutable VFS
/// reference from the context so that `cwd` can be updated.
impl super::Command for CdCommand {
    fn name(&self) -> &'static str {
        "cd"
    }
    fn description(&self) -> &'static str {
        "Change the current directory"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&mut ctx.state.vfs, ctx.args).into()
    }

    fn synopsis(&self) -> &'static str {
        "cd [path]"
    }
    fn man_description(&self) -> &'static str {
        "Change the shell's current working directory to the given path. The special path ~ navigates to the home directory (/home/user), \
.. moves to the parent directory, and / navigates to the filesystem root. \
With no arguments, cd returns to the home directory. Produces no output on success, matching POSIX behaviour."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["cd /tmp", "cd ..", "cd ~"]
    }
}
