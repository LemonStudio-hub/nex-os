//! `tac` - concatenate files in reverse line order
//!
//! Reads a file and prints its lines from last to first. This is the
//! reverse of `cat` for line ordering (not to be confused with `rev`
//! which reverses characters within each line).
//!
//! # Usage
//!
//! ```text
//! tac [file]
//! echo -e 'a\nb\nc' | tac
//! ```
//!
//! # Examples
//!
//! ```text
//! tac file.txt             # print lines in reverse order
//! ```

use crate::vfs::Vfs;

/// Execute the `tac` command against the virtual filesystem.
///
/// Reads the specified file and outputs its lines in reverse order.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual filesystem to read from.
/// * `args` -- Slice containing the file path.
///
/// # Returns
///
/// `Ok(String)` with lines in reverse order, or `Err` if the file operand is missing.
pub fn execute(
    vfs: &Vfs,
    args: &[&str],
    host_fs: Option<&dyn crate::vfs::HostFs>,
) -> Result<String, String> {
    if args.is_empty() {
        return Err("tac: missing file operand".to_string());
    }
    if args.len() > 1 {
        return Err("tac: too many arguments".to_string());
    }

    let resolved = vfs.resolve_path(args[0])?;
    let content = vfs.read_file_with_host(&resolved, host_fs)?;

    if content.is_empty() {
        return Ok(String::new());
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut output = String::new();

    for line in lines.iter().rev() {
        output.push_str(line);
        output.push('\n');
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `tac`.
pub struct TacCommand;

impl super::Command for TacCommand {
    fn name(&self) -> &'static str {
        "tac"
    }

    fn description(&self) -> &'static str {
        "Concatenate files in reverse line order"
    }

    fn accepts_stdin(&self) -> bool {
        true
    }

    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }

    fn synopsis(&self) -> &'static str {
        "tac [file]"
    }

    fn man_description(&self) -> &'static str {
        "Concatenate and print files in reverse line order. Each file is read and its \
lines are printed from last to first. If no file is given, reads from stdin. \
This is the reverse of cat for line ordering."
    }

    fn examples(&self) -> &'static [&'static str] {
        &["tac file.txt", "cat file.txt | tac"]
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
    fn reverse_line_order() {
        let vfs = vfs_with_content("a\nb\nc");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "c\nb\na\n");
    }

    #[test]
    fn empty_file() {
        let vfs = vfs_with_content("");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn single_line() {
        let vfs = vfs_with_content("only");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "only\n");
    }

    #[test]
    fn two_lines() {
        let vfs = vfs_with_content("first\nsecond");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        assert_eq!(out, "second\nfirst\n");
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
