//! `comm` - compare two sorted files line by line
//!
//! Reads two sorted files and outputs three tab-separated columns:
//!
//! - Column 1: lines only in FILE1
//! - Column 2: lines only in FILE2
//! - Column 3: lines in both files
//!
//! Columns can be suppressed with `-1`, `-2`, `-3` flags.
//!
//! # Usage
//!
//! ```text
//! comm [-1] [-2] [-3] FILE1 FILE2
//! ```
//!
//! # Examples
//!
//! ```text
//! comm file1 file2              # show all three columns
//! comm -1 -2 file1 file2        # show only common lines
//! comm -1 -3 file1 file2        # show only lines unique to file2
//! ```

use crate::vfs::Vfs;

/// Execute the `comm` command against the virtual filesystem.
///
/// Reads two sorted files and produces a three-column comparison output.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual filesystem to read from.
/// * `args` -- Optional `-1`, `-2`, `-3` flags followed by two file paths.
///
/// # Returns
///
/// `Ok(String)` with the comparison output, or `Err` for invalid arguments.
pub fn execute(
    vfs: &Vfs,
    args: &[&str],
    host_fs: Option<&dyn crate::vfs::HostFs>,
) -> Result<String, String> {
    let mut show_col1 = true;
    let mut show_col2 = true;
    let mut show_col3 = true;
    let mut files: Vec<&str> = Vec::new();

    for arg in args {
        match *arg {
            "-1" => show_col1 = false,
            "-2" => show_col2 = false,
            "-3" => show_col3 = false,
            _ => files.push(arg),
        }
    }

    if files.len() < 2 {
        return Err("comm: missing file operand".to_string());
    }
    if files.len() > 2 {
        return Err("comm: too many arguments".to_string());
    }

    let resolved_a = vfs.resolve_path(files[0])?;
    let resolved_b = vfs.resolve_path(files[1])?;
    let content_a = vfs.read_file_with_host(&resolved_a, host_fs)?;
    let content_b = vfs.read_file_with_host(&resolved_b, host_fs)?;

    let lines_a: Vec<&str> = content_a.lines().collect();
    let lines_b: Vec<&str> = content_b.lines().collect();

    let mut output = String::new();
    let mut i = 0;
    let mut j = 0;

    while i < lines_a.len() && j < lines_b.len() {
        match lines_a[i].cmp(lines_b[j]) {
            std::cmp::Ordering::Less => {
                if show_col1 {
                    output.push_str(lines_a[i]);
                    output.push('\n');
                }
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                if show_col2 {
                    output.push('\t');
                    output.push_str(lines_b[j]);
                    output.push('\n');
                }
                j += 1;
            }
            std::cmp::Ordering::Equal => {
                if show_col3 {
                    output.push('\t');
                    output.push('\t');
                    output.push_str(lines_a[i]);
                    output.push('\n');
                }
                i += 1;
                j += 1;
            }
        }
    }

    // Remaining lines in A (only in file 1).
    while i < lines_a.len() {
        if show_col1 {
            output.push_str(lines_a[i]);
            output.push('\n');
        }
        i += 1;
    }

    // Remaining lines in B (only in file 2).
    while j < lines_b.len() {
        if show_col2 {
            output.push('\t');
            output.push_str(lines_b[j]);
            output.push('\n');
        }
        j += 1;
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `comm`.
pub struct CommCommand;

impl super::Command for CommCommand {
    fn name(&self) -> &'static str {
        "comm"
    }

    fn description(&self) -> &'static str {
        "Compare two sorted files line by line"
    }

    fn accepts_stdin(&self) -> bool {
        true
    }

    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }

    fn synopsis(&self) -> &'static str {
        "comm [-1] [-2] [-3] FILE1 FILE2"
    }

    fn man_description(&self) -> &'static str {
        "Compare two sorted files line by line. Output three tab-separated columns: lines only \
in FILE1, lines only in FILE2, and lines in both. Columns can be suppressed with flags: \
-1 suppress column 1, -2 suppress column 2, -3 suppress column 3. Both input files must be sorted."
    }

    fn examples(&self) -> &'static [&'static str] {
        &["comm file1 file2", "comm -1 -3 file1 file2"]
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

    #[test]
    fn three_column_output() {
        let vfs = vfs_with_two("a\nb\nc", "b\nc\nd");
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        // col1: "a", col3: "b", col3: "c", col2: "d"
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "a"); // col1: no prefix
        assert_eq!(lines[1], "\t\tb"); // col3: two tabs
        assert_eq!(lines[2], "\t\tc"); // col3: two tabs
        assert_eq!(lines[3], "\td"); // col2: one tab
    }

    #[test]
    fn suppress_col1_and_col2() {
        let vfs = vfs_with_two("a\nb\nc", "b\nc\nd");
        let out = execute(&vfs, &["-1", "-2", "/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["\t\tb", "\t\tc"]);
    }

    #[test]
    fn suppress_col3() {
        let vfs = vfs_with_two("a\nb\nc", "b\nc\nd");
        let out = execute(&vfs, &["-3", "/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["a", "\td"]);
    }

    #[test]
    fn identical_files() {
        let vfs = vfs_with_two("a\nb", "a\nb");
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["\t\ta", "\t\tb"]);
    }

    #[test]
    fn disjoint_files() {
        let vfs = vfs_with_two("a\nb", "c\nd");
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["a", "b", "\tc", "\td"]);
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
    fn only_one_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/tmp/a.txt"], None).is_err());
    }

    #[test]
    fn suppress_all() {
        let vfs = vfs_with_two("a\nb", "a\nb");
        let out = execute(&vfs, &["-1", "-2", "-3", "/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        assert_eq!(out, "");
    }
}
