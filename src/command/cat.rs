//! `cat` -- concatenate and display file contents.
//!
//! # Usage
//!
//! ```text
//! cat <file> [file2 ...]
//! ```
//!
//! Reads each file argument in order and writes its contents to standard
//! output.  Multiple files are concatenated without any separator.  When a file
//! does not end with a newline, one is appended so that piped output remains
//! well-formed.
//!
//! # Errors
//!
//! - Missing file operand.
//! - Path does not exist or is a directory (directories cannot be read as
//!   files).
//!
//! # Stdin
//!
//! The `cat` command declares `accepts_stdin() = true`.  When used at the end
//! of a pipeline without an explicit file argument, the pipeline layer appends
//! the path of a temporary file containing the preceding stage's stdout, so
//! `cat` reads it like any other file.

use crate::vfs::permissions::{check_access, AccessMode};
use crate::vfs::Vfs;

/// Execute the `cat` command.
///
/// Iterates over every path in `args`, resolves it against the VFS, validates
/// that it exists and is not a directory, checks read permission, then appends
/// its contents to the output buffer.
pub fn execute(
    vfs: &Vfs,
    args: &[&str],
    uid: u32,
    gid: u32,
    host_fs: Option<&dyn crate::vfs::HostFs>,
) -> Result<String, String> {
    if args.is_empty() {
        return Err("cat: missing file operand".to_string());
    }

    let mut output = String::new();

    for path in args {
        let resolved = vfs.resolve_path(path)?;

        if !vfs.exists_with_host(&resolved, host_fs).unwrap_or(false) {
            return Err(format!("cat: {}: No such file or directory", path));
        }

        if vfs.is_dir_with_host(&resolved, host_fs).unwrap_or(false) {
            return Err(format!("cat: {}: Is a directory", path));
        }

        // Permission check: read access required (skip for host-mounted paths)
        if host_fs.is_none() || vfs.find_mount(&resolved).is_none() {
            if let Some(meta) = vfs.get_meta(&resolved) {
                check_access(meta, AccessMode::Read, uid, gid)?;
            }
        }

        let content = vfs.read_file_with_host(&resolved, host_fs)?;
        output.push_str(&content);
        if !output.ends_with('\n') {
            output.push('\n');
        }
    }

    Ok(output)
}

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct CatCommand;

/// Implements the `Command` trait.  `accepts_stdin` is `true` so that the
/// pipeline layer can feed stdin from a preceding stage as a temporary file
/// argument when no explicit file is given.
impl super::Command for CatCommand {
    fn name(&self) -> &'static str {
        "cat"
    }
    fn description(&self) -> &'static str {
        "Display file contents"
    }
    fn accepts_stdin(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(
            &ctx.state.vfs,
            ctx.args,
            ctx.state.euid,
            ctx.state.gid,
            ctx.host_fs,
        )
        .into()
    }
    fn synopsis(&self) -> &'static str {
        "cat file [file2 ...]"
    }
    fn man_description(&self) -> &'static str {
        "Concatenate and display the contents of one or more files to standard output. \
Multiple files are read in order and their contents are concatenated without any separator. \
A trailing newline is appended if the last file does not end with one."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["cat file.txt", "cat a.txt b.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_single_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "hello").unwrap();
        let out = execute(&vfs, &["/tmp/f.txt"], 0, 0, None).unwrap();
        assert!(out.contains("hello"));
    }

    #[test]
    fn read_multiple_files() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "AAA").unwrap();
        vfs.write_file("/tmp/b.txt", "BBB").unwrap();
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"], 0, 0, None).unwrap();
        assert!(out.contains("AAA"));
        assert!(out.contains("BBB"));
    }

    #[test]
    fn nonexistent_file_errors() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/nope"], 0, 0, None).is_err());
    }

    #[test]
    fn directory_errors() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/home"], 0, 0, None).is_err());
    }

    #[test]
    fn missing_operand() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[], 0, 0, None).is_err());
    }

    #[test]
    fn read_denied_for_non_owner() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "secret").unwrap();
        // Set file to owner-only read (0o400)
        vfs.get_meta_mut("/tmp/f.txt").unwrap().mode = 0o400;
        vfs.get_meta_mut("/tmp/f.txt").unwrap().uid = 1000;
        // Root can always read
        assert!(execute(&vfs, &["/tmp/f.txt"], 0, 0, None).is_ok());
        // Owner can read
        assert!(execute(&vfs, &["/tmp/f.txt"], 1000, 1000, None).is_ok());
        // Other user cannot
        assert!(execute(&vfs, &["/tmp/f.txt"], 2000, 2000, None).is_err());
    }
}
