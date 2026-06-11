//! `tee` - read stdin and write to both stdout and files simultaneously
//!
//! Acts as a "T-splitter" in a pipeline: it reads its input and writes it
//! verbatim to one or more files while also passing it through to stdout.
//! This is useful for logging intermediate pipeline output without consuming it.
//!
//! # Usage
//!
//! ```text
//! tee [-a] <file> [file2 ...]
//! ```
//!
//! # Flags
//!
//! - `-a` -- Append to each file instead of overwriting. Without this flag,
//!   existing file contents are replaced.
//!
//! # Examples
//!
//! ```text
//! echo hello | tee /tmp/out.txt          # write "hello" to file and stdout
//! echo log | tee -a /tmp/log.txt         # append to log file
//! echo x | tee /tmp/a.txt /tmp/b.txt     # write to multiple files
//! ```

use crate::vfs::Vfs;

/// Execute the `tee` command against the virtual filesystem.
///
/// Writes the provided `input` string to every file listed in `args`, then
/// returns the same `input` as stdout output. When `-a` is present the
/// existing file contents are preserved and the input is appended.
///
/// # Arguments
///
/// * `vfs` -- Mutable reference to the virtual filesystem (writes are needed).
/// * `input` -- The stdin data to tee (typically piped from a prior command).
/// * `args` -- Slice of argument strings: optional `-a` flag followed by one
///   or more file paths.
///
/// # Returns
///
/// `Ok(input)` -- the input is passed through unchanged to stdout, or `Err`
/// if no file operand is given or a write fails.
pub fn execute(vfs: &mut Vfs, input: &str, args: &[&str]) -> Result<String, String> {
    let mut append = false;
    let mut files: Vec<&str> = Vec::new();

    // Separate flags from positional file arguments.
    for arg in args {
        match *arg {
            "-a" => append = true,
            _ => files.push(arg),
        }
    }

    if files.is_empty() {
        return Err("tee: missing file operand".to_string());
    }

    // Write the input to every specified file.
    for path in &files {
        let resolved = vfs.resolve_path(path)?;
        let write_result = if append {
            // In append mode, read whatever is already there (defaulting to
            // empty for new files) and concatenate the new input.
            let existing = vfs.read_file(&resolved).unwrap_or_default();
            vfs.write_file(&resolved, &format!("{}{}", existing, input))
        } else {
            // In overwrite mode, simply replace the entire file.
            vfs.write_file(&resolved, input)
        };
        if let Err(e) = write_result {
            return Err(format!("tee: {}: {}", path, e));
        }
    }

    // Pass the input through to stdout -- this is the "tee" behavior.
    Ok(input.to_string())
}

/// Command struct implementing the [`super::Command`] trait for `tee`.
pub struct TeeCommand;

/// Trait implementation that wires `TeeCommand` into the shell's command
/// registry. Unlike most commands, `tee` does not declare `accepts_stdin`
/// because it receives stdin directly via the `CommandContext.stdin` field
/// rather than through the implicit stdin-to-argument injection.
impl super::Command for TeeCommand {
    /// Returns the command name used for dispatch and tab completion.
    fn name(&self) -> &'static str { "tee" }

    /// Short description shown in `help` output.
    fn description(&self) -> &'static str { "Write stdin to stdout and files (-a for append)" }

    /// Entry point called by the shell dispatcher. Passes the mutable VFS,
    /// the raw stdin string, and args through to the standalone [`execute`].
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.stdin, ctx.args)
    }
    fn synopsis(&self) -> &'static str { "echo text | tee [-a] file [file2 ...]" }
    fn man_description(&self) -> &'static str {
        "Read from stdin and write the input to both stdout and one or more files simultaneously, \
acting as a T-splitter in a pipeline. By default each file is overwritten. With -a, the input \
is appended to each file instead of replacing its contents."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["echo hello | tee output.txt", "echo data | tee -a log.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_to_single_file() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, "hello world", &["/tmp/out.txt"]).unwrap();
        // Stdout should be identical to the input.
        assert_eq!(out, "hello world");
        // The file should contain the written content.
        assert_eq!(vfs.read_file("/tmp/out.txt").unwrap(), "hello world");
    }

    #[test]
    fn write_to_multiple_files() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, "data", &["/tmp/a.txt", "/tmp/b.txt"]).unwrap();
        assert_eq!(out, "data");
        // Both files should receive the same content.
        assert_eq!(vfs.read_file("/tmp/a.txt").unwrap(), "data");
        assert_eq!(vfs.read_file("/tmp/b.txt").unwrap(), "data");
    }

    #[test]
    fn append_mode() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/log.txt", "first\n").unwrap();
        let out = execute(&mut vfs, "second\n", &["-a", "/tmp/log.txt"]).unwrap();
        assert_eq!(out, "second\n");
        // New content should be appended after existing content.
        assert_eq!(vfs.read_file("/tmp/log.txt").unwrap(), "first\nsecond\n");
    }

    #[test]
    fn overwrite_mode() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/log.txt", "old").unwrap();
        let out = execute(&mut vfs, "new", &["/tmp/log.txt"]).unwrap();
        assert_eq!(out, "new");
        // Without -a, the old content is replaced entirely.
        assert_eq!(vfs.read_file("/tmp/log.txt").unwrap(), "new");
    }

    #[test]
    fn missing_file_operand() {
        let mut vfs = Vfs::new();
        // Calling tee with zero file arguments should fail.
        assert!(execute(&mut vfs, "data", &[]).is_err());
    }

    #[test]
    fn empty_input() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, "", &["/tmp/out.txt"]).unwrap();
        // Even empty input should work and produce an empty file.
        assert_eq!(out, "");
        assert_eq!(vfs.read_file("/tmp/out.txt").unwrap(), "");
    }
}
