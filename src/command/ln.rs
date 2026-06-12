//! `ln` command -- create links to files in the VFS.
//!
//! # Usage
//!
//! ```text
//! ln [-s] <target> <link_name>
//! ```
//!
//! # Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `-s` | Create a **symbolic link** (a small file containing the target path). |
//!
//! # Description
//!
//! The VFS does not support real inodes or hard links, so link creation is
//! simulated:
//!
//! * **Symbolic link (`-s`)** -- Creates a regular file at `<link_name>` whose
//!   content is `-> <target>\n`.  This is a convention only; the shell does
//!   **not** dereference the link when reading.
//! * **Hard link (default)** -- Copies the target file's content to
//!   `<link_name>`.  The two files are independent; modifying one does not
//!   affect the other.
//!
//! # Examples
//!
//! ```text
//! $ ln -s /etc/config /tmp/cfg_link
//! $ ln /tmp/data.txt /tmp/data_copy.txt
//! ```
//!
//! # Errors
//!
//! * Missing operands (`ln` or `ln <target>` with no link name).
//! * Target does not exist.
//! * Hard-linking a directory is not supported.

use crate::vfs::Vfs;

/// Execute the `ln` command.
///
/// Parses flags and positional arguments, verifies the target exists, then
/// creates either a symbolic link file or a content copy depending on whether
/// `-s` was supplied.
///
/// # Arguments
///
/// * `vfs` -- Mutable reference to the virtual file system.
/// * `args` -- Command-line arguments (flags + positional tokens).
///
/// # Returns
///
/// `Ok(String::new())` on success (ln produces no output), or an
/// `Err(message)` describing what went wrong.
pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    let mut symbolic = false;
    let mut positional: Vec<&str> = Vec::new();

    // Separate flags from positional arguments.
    for arg in args {
        match *arg {
            "-s" => symbolic = true,
            _ => positional.push(arg),
        }
    }

    // Both <target> and <link_name> are required.
    if positional.len() < 2 {
        return Err("ln: missing file operand".to_string());
    }

    let target = positional[0];
    let link_name = positional[1];

    // Resolve the target to an absolute path inside the VFS.
    let resolved_target = vfs.resolve_path(target)?;

    // Verify the target actually exists -- check both file and directory
    // because the VFS exposes separate read_file / list_dir APIs.
    if vfs.read_file(&resolved_target).is_err() && vfs.list_dir(&resolved_target).is_err() {
        return Err(format!("ln: '{}': No such file or directory", target));
    }

    let resolved_link = vfs.resolve_path(link_name)?;

    if symbolic {
        // Symbolic link: store a human-readable arrow notation in the file.
        // The content is purely informational and not followed automatically.
        vfs.write_file(&resolved_link, &format!("-> {}\n", target))
            .map_err(|e| format!("ln: {}", e))?;
    } else {
        // Hard link simulation: read the target's content and write a copy.
        // Reading a directory fails here, which mirrors the real `ln` error
        // when attempting to hard-link a directory without `-r`.
        let content = vfs
            .read_file(&resolved_target)
            .map_err(|_| format!("ln: '{}': Cannot link directory", target))?;
        vfs.write_file(&resolved_link, &content)
            .map_err(|e| format!("ln: {}", e))?;
    }

    // ln produces no stdout on success.
    Ok(String::new())
}

/// Unit struct representing the `ln` command.
pub struct LnCommand;

/// Bridges the registry's [`Command`](super::Command) interface to the
/// module-level `execute` function.
impl super::Command for LnCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str { "ln" }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str { "Create links (-s for symbolic)" }

    /// Execute the command, forwarding the VFS and arguments from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&mut ctx.state.vfs, ctx.args)
    }

    fn synopsis(&self) -> &'static str { "ln [-s] target link_name" }
    fn man_description(&self) -> &'static str {
        "Create links to files in the virtual filesystem. By default, a hard link is created by copying the target file's \
content to the link name (the two files are independent). With -s, a symbolic link is created as a small file \
containing '-> target' notation. Symbolic links are not automatically dereferenced when reading."
    }
    fn examples(&self) -> &'static [&'static str] { &["ln file.txt link.txt", "ln -s /path/to/file symlink"] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Verify that `-s` creates a file containing the arrow-notation path.
    fn symbolic_link() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/target.txt", "content").unwrap();
        execute(&mut vfs, &["-s", "/tmp/target.txt", "/tmp/link.txt"]).unwrap();
        let out = vfs.read_file("/tmp/link.txt").unwrap();
        assert!(out.contains("-> /tmp/target.txt"));
    }

    #[test]
    /// Verify that the default (hard link) mode copies the target's content.
    fn hard_link_copies_content() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/src.txt", "data").unwrap();
        execute(&mut vfs, &["/tmp/src.txt", "/tmp/copy.txt"]).unwrap();
        assert_eq!(vfs.read_file("/tmp/copy.txt").unwrap(), "data");
    }

    #[test]
    /// Verify that fewer than 2 positional args returns an error.
    fn missing_operand() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &[]).is_err());
        assert!(execute(&mut vfs, &["/tmp/a"]).is_err());
    }

    #[test]
    /// Verify that a non-existent target path produces an error.
    fn nonexistent_target() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/nope", "/tmp/link"]).is_err());
    }
}
