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

use crate::vfs::Vfs;

/// Execute the `cat` command.
///
/// Iterates over every path in `args`, resolves it against the VFS, validates
/// that it exists and is not a directory, then appends its contents to the
/// output buffer.  A trailing newline is guaranteed even if the file content
/// does not end with one.
///
/// # Errors
///
/// Propagates VFS resolution/read errors, or returns a descriptive error for
/// missing files, directories, and empty argument lists.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("cat: missing file operand".to_string());
    }

    let mut output = String::new();

    for path in args {
        // Resolve relative paths, `..`, `~`, etc. into absolute VFS paths.
        let resolved = vfs.resolve_path(path)?;

        if !vfs.exists(&resolved) {
            return Err(format!("cat: {}: No such file or directory", path));
        }

        // Directories are rejected -- they have no single "content" to display.
        if vfs.is_dir(&resolved) {
            return Err(format!("cat: {}: Is a directory", path));
        }

        let content = vfs.read_file(&resolved)?;
        output.push_str(&content);
        // Guarantee a trailing newline so consecutive file contents don't merge
        // on the same line and piped output stays well-formed.
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
    fn name(&self) -> &'static str { "cat" }
    fn description(&self) -> &'static str { "Display file contents" }
    fn accepts_stdin(&self) -> bool { true }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.vfs, ctx.args)
    }
    fn synopsis(&self) -> &'static str { "cat file [file2 ...]" }
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
        let out = execute(&vfs, &["/tmp/f.txt"]).unwrap();
        assert!(out.contains("hello"));
    }

    #[test]
    fn read_multiple_files() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "AAA").unwrap();
        vfs.write_file("/tmp/b.txt", "BBB").unwrap();
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"]).unwrap();
        assert!(out.contains("AAA"));
        assert!(out.contains("BBB"));
    }

    #[test]
    fn nonexistent_file_errors() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/nope"]).is_err());
    }

    #[test]
    fn directory_errors() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/home"]).is_err());
    }

    #[test]
    fn missing_operand() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[]).is_err());
    }
}
