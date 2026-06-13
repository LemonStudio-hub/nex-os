//! `pwd` command -- print the current working directory.
//!
//! # Usage
//!
//! ```text
//! pwd
//! ```
//!
//! # Description
//!
//! Outputs the absolute path of the shell's current working directory
//! followed by a newline.  The working directory is stored in
//! [`Vfs::cwd`](crate::vfs::Vfs::cwd) and is updated by the `cd` command.
//!
//! # Examples
//!
//! ```text
//! $ cd /tmp
//! $ pwd
//! /tmp
//! ```
//!
//! # Notes
//!
//! This implementation takes no flags.  Real `pwd` supports `-L` (logical,
//! use `$PWD`) and `-P` (physical, resolve symlinks), but those concepts
//! do not apply to the simulated VFS.

use crate::vfs::Vfs;

/// Execute the `pwd` command.
///
/// Reads the current working directory from the VFS and returns it with
/// a trailing newline.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual file system (read-only access).
///
/// # Returns
///
/// Always returns `Ok` with the absolute path of the working directory.
pub fn execute(vfs: &Vfs) -> Result<String, String> {
    Ok(format!("{}\n", vfs.cwd))
}

/// Unit struct representing the `pwd` command.
pub struct PwdCommand;

/// Bridges the registry's [`Command`](super::Command) interface to the
/// module-level `execute` function.
impl super::Command for PwdCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str {
        "pwd"
    }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str {
        "Print the current working directory"
    }

    /// Execute the command, forwarding the VFS from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs).into()
    }

    fn synopsis(&self) -> &'static str {
        "pwd"
    }
    fn man_description(&self) -> &'static str {
        "Print the absolute path of the shell's current working directory, followed by a newline. \
The working directory is stored in the VFS and is updated by the cd command. Takes no arguments or flags."
    }
}
