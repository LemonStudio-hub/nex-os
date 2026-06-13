//! `paste` - merge lines of files side by side
//!
//! Reads multiple files and merges their lines side by side, separated
//! by a tab character (or a custom delimiter with `-d`). If one file is
//! shorter than the others, empty strings are used for its missing lines.
//!
//! # Usage
//!
//! ```text
//! paste [-d DELIM] FILE1 FILE2 [FILE3 ...]
//! ```
//!
//! # Examples
//!
//! ```text
//! paste file1 file2              # tab-separated merge
//! paste -d , file1 file2         # comma-separated merge
//! paste -d '|' file1 file2 file3 # pipe-separated, three files
//! ```

use crate::vfs::Vfs;

/// Execute the `paste` command against the virtual filesystem.
///
/// Reads two or more files and merges their lines side by side with the
/// specified delimiter.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual filesystem to read from.
/// * `args` -- Optional `-d DELIM` flag followed by two or more file paths.
///
/// # Returns
///
/// `Ok(String)` with merged output, or `Err` for invalid arguments.
pub fn execute(
    vfs: &Vfs,
    args: &[&str],
    host_fs: Option<&dyn crate::vfs::HostFs>,
) -> Result<String, String> {
    let mut delimiter = '\t';
    let mut files: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        if args[i] == "-d" {
            if i + 1 >= args.len() {
                return Err("paste: option requires an argument -- 'd'".to_string());
            }
            delimiter = args[i + 1].chars().next().ok_or("paste: empty delimiter")?;
            i += 2;
        } else {
            files.push(args[i]);
            i += 1;
        }
    }

    if files.len() < 2 {
        return Err("paste: missing file operand".to_string());
    }

    // Read all files and split into lines.
    let contents: Vec<String> = files
        .iter()
        .map(|path| {
            let resolved = vfs.resolve_path(path)?;
            vfs.read_file_with_host(&resolved, host_fs)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let all_lines: Vec<Vec<&str>> = contents.iter().map(|c| c.lines().collect()).collect();

    // Find the maximum number of lines across all files.
    let max_lines = all_lines.iter().map(|l| l.len()).max().unwrap_or(0);

    let mut output = String::new();
    for line_idx in 0..max_lines {
        let mut fields: Vec<String> = Vec::new();
        for file_lines in &all_lines {
            fields.push(file_lines.get(line_idx).unwrap_or(&"").to_string());
        }
        output.push_str(&fields.join(&delimiter.to_string()));
        output.push('\n');
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `paste`.
pub struct PasteCommand;

impl super::Command for PasteCommand {
    fn name(&self) -> &'static str {
        "paste"
    }

    fn description(&self) -> &'static str {
        "Merge lines of files side by side"
    }

    fn accepts_stdin(&self) -> bool {
        true
    }

    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }

    fn synopsis(&self) -> &'static str {
        "paste [-d DELIM] FILE1 FILE2 [FILE3 ...]"
    }

    fn man_description(&self) -> &'static str {
        "Merge lines from multiple files side by side, separated by tab characters. \
Each line from FILE1 is joined with the corresponding line from FILE2 (and FILE3, etc.) \
using the delimiter. If one file is shorter, empty strings are used for its missing lines. \
Use -d to specify a custom single-character delimiter instead of tab."
    }

    fn examples(&self) -> &'static [&'static str] {
        &[
            "paste file1 file2",
            "paste -d , file1 file2",
            "paste -d '|' file1 file2 file3",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs_with_two(a: &str, b: &str) -> Vfs {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", a).unwrap();
        vfs.write_file("/tmp/b.txt", b).unwrap();
        vfs
    }

    fn vfs_with_three(a: &str, b: &str, c: &str) -> Vfs {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", a).unwrap();
        vfs.write_file("/tmp/b.txt", b).unwrap();
        vfs.write_file("/tmp/c.txt", c).unwrap();
        vfs
    }

    #[test]
    fn basic_merge() {
        let vfs = vfs_with_two("a\nb", "c\nd");
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        assert_eq!(out, "a\tc\nb\td\n");
    }

    #[test]
    fn custom_delimiter() {
        let vfs = vfs_with_two("1\n2", "3\n4");
        let out = execute(&vfs, &["-d", ",", "/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        assert_eq!(out, "1,3\n2,4\n");
    }

    #[test]
    fn different_lengths() {
        let vfs = vfs_with_two("a\nb\nc", "x");
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        assert_eq!(out, "a\tx\nb\t\nc\t\n");
    }

    #[test]
    fn three_files() {
        let vfs = vfs_with_three("a\nb", "c\nd", "e\nf");
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt", "/tmp/c.txt"], None).unwrap();
        assert_eq!(out, "a\tc\te\nb\td\tf\n");
    }

    #[test]
    fn empty_files() {
        let vfs = vfs_with_two("", "");
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn missing_args() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[], None).is_err());
    }

    #[test]
    fn single_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/tmp/a.txt"], None).is_err());
    }

    #[test]
    fn missing_d_value() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["-d"], None).is_err());
    }
}
