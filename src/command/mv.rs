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

use crate::vfs::Vfs;

/// Execute the `mv` command.
///
/// Resolves both source and destination paths, verifies the source exists,
/// then delegates the actual move to [`Vfs::mv`].
///
/// # Arguments
///
/// * `vfs` -- Mutable reference to the virtual file system.
/// * `args` -- Command-line arguments: `[source, destination]`.
///
/// # Returns
///
/// `Ok(String::new())` on success (mv produces no output), or
/// `Err(message)` if the source is missing or the move fails.
pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
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

    if !vfs.exists(&src_resolved) {
        return Err(format!(
            "mv: cannot stat '{}': No such file or directory",
            src
        ));
    }

    // Delegate to the VFS move implementation which handles both files
    // and directories, and the case where dst is an existing directory.
    vfs.mv(&src_resolved, &dst_resolved)?;
    Ok(String::new())
}

/// Unit struct representing the `mv` command.
pub struct MvCommand;

/// Bridges the registry's [`Command`](super::Command) interface to the
/// module-level `execute` function.
impl super::Command for MvCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str { "mv" }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str { "Move or rename files and directories" }

    /// Execute the command, forwarding VFS and arguments from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }

    fn synopsis(&self) -> &'static str { "mv source destination" }
    fn man_description(&self) -> &'static str {
        "Move or rename a file or directory from source to destination. The source is removed after the move. \
If the destination is an existing directory, the source is moved into that directory preserving its basename."
    }
    fn examples(&self) -> &'static [&'static str] { &["mv old.txt new.txt", "mv file.txt /tmp/"] }
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
        execute(&mut vfs, &["/tmp/old.txt", "/tmp/new.txt"]).unwrap();
        assert!(!vfs.exists("/tmp/old.txt"));
        assert_eq!(vfs.read_file("/tmp/new.txt").unwrap(), "data");
    }

    #[test]
    /// Moving a non-existent source should produce an error.
    fn move_nonexistent_errors() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/nope", "/tmp/dst"]).is_err());
    }

    #[test]
    /// Omitting the destination argument should produce an error.
    fn missing_destination() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/tmp/src"]).is_err());
    }
}
