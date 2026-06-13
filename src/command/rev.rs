//! `rev` - reverse lines of text character by character
//!
//! Reads input and reverses each line's characters while preserving the
//! order of lines. Supports reading from a file or from stdin via pipe.
//!
//! # Usage
//!
//! ```text
//! rev [file]
//! echo hello | rev
//! ```
//!
//! # Examples
//!
//! ```text
//! rev file.txt             # reverse each line
//! echo hello | rev         # "olleh"
//! ```

use crate::vfs::Vfs;

/// Execute the `rev` command against the virtual filesystem.
///
/// Reads the specified file and reverses each line's characters.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual filesystem to read from.
/// * `args` -- Slice containing the file path.
///
/// # Returns
///
/// `Ok(String)` with reversed lines, or `Err` if the file operand is missing.
pub fn execute(
    vfs: &Vfs,
    args: &[&str],
    host_fs: Option<&dyn crate::vfs::HostFs>,
) -> Result<String, String> {
    if args.is_empty() {
        return Err("rev: missing file operand".to_string());
    }
    if args.len() > 1 {
        return Err("rev: too many arguments".to_string());
    }

    let resolved = vfs.resolve_path(args[0])?;
    let content = vfs.read_file_with_host(&resolved, host_fs)?;

    let mut output = String::new();
    for line in content.lines() {
        let reversed: String = line.chars().rev().collect();
        output.push_str(&reversed);
        output.push('\n');
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `rev`.
pub struct RevCommand;

impl super::Command for RevCommand {
    fn name(&self) -> &'static str {
        "rev"
    }

    fn description(&self) -> &'static str {
        "Reverse lines of text character by character"
    }

    fn accepts_stdin(&self) -> bool {
        true
    }

    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }

    fn synopsis(&self) -> &'static str {
        "rev [file]"
    }

    fn man_description(&self) -> &'static str {
        "Reverse each line of the input character by character and write the result to standard \
output. The order of lines is preserved; only the characters within each line are reversed. \
If no file is given, reads from stdin."
    }

    fn examples(&self) -> &'static [&'static str] {
        &["rev file.txt", "echo hello | rev"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs_with_content(content: &str) -> Vfs {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", content).unwrap();
        vfs
    }

    #[test]
    fn reverse_lines() {
        let vfs = vfs_with_content("hello\nworld");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "olleh\ndlrow\n");
    }

    #[test]
    fn empty_file() {
        let vfs = vfs_with_content("");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn single_line() {
        let vfs = vfs_with_content("abc");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "cba\n");
    }

    #[test]
    fn single_chars() {
        let vfs = vfs_with_content("a\nb");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "a\nb\n");
    }

    #[test]
    fn unicode() {
        let vfs = vfs_with_content("abc");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "cba\n");
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[], None).is_err());
    }

    #[test]
    fn too_many_args() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["a", "b"], None).is_err());
    }
}
