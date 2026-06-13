//! `nl` - number lines of a file
//!
//! Reads a file and prefixes each line with its 1-indexed line number,
//! right-aligned in a 6-character field followed by a tab.
//!
//! # Usage
//!
//! ```text
//! nl [file]
//! cat file | nl
//! ```
//!
//! # Examples
//!
//! ```text
//! nl file.txt              # number all lines
//! cat file.txt | nl        # same via pipe
//! ```

use crate::vfs::Vfs;

/// Execute the `nl` command against the virtual filesystem.
///
/// Reads the specified file and prepends each line with its 1-indexed
/// line number.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual filesystem to read from.
/// * `args` -- Slice containing the file path.
///
/// # Returns
///
/// `Ok(String)` with numbered lines, or `Err` if the file operand is missing.
pub fn execute(
    vfs: &Vfs,
    args: &[&str],
    host_fs: Option<&dyn crate::vfs::HostFs>,
) -> Result<String, String> {
    if args.is_empty() {
        return Err("nl: missing file operand".to_string());
    }
    if args.len() > 1 {
        return Err("nl: too many arguments".to_string());
    }

    let resolved = vfs.resolve_path(args[0])?;
    let content = vfs.read_file_with_host(&resolved, host_fs)?;

    if content.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();
    for (i, line) in content.lines().enumerate() {
        output.push_str(&format!("{:>6}\t{}\n", i + 1, line));
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `nl`.
pub struct NlCommand;

impl super::Command for NlCommand {
    fn name(&self) -> &'static str {
        "nl"
    }

    fn description(&self) -> &'static str {
        "Number lines of a file"
    }

    fn accepts_stdin(&self) -> bool {
        true
    }

    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }

    fn synopsis(&self) -> &'static str {
        "nl [file]"
    }

    fn man_description(&self) -> &'static str {
        "Number each line of the input file and write the result to standard output. \
Each line is prefixed with its 1-indexed line number, right-aligned in a 6-character \
field, followed by a tab character. If no file is given, reads from stdin."
    }

    fn examples(&self) -> &'static [&'static str] {
        &["nl file.txt", "cat file.txt | nl"]
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
    fn number_lines() {
        let vfs = vfs_with_content("a\nb\nc");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "     1\ta\n     2\tb\n     3\tc\n");
    }

    #[test]
    fn empty_file() {
        let vfs = vfs_with_content("");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn single_line() {
        let vfs = vfs_with_content("hello");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "     1\thello\n");
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
