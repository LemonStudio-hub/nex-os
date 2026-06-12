//! `diff` -- compare two files line by line.
//!
//! # Usage
//!
//! ```text
//! diff <file1> <file2>
//! ```
//!
//! Reads both files, compares them line by line using a Longest Common
//! Subsequence (LCS) algorithm, and outputs the differences in a simplified
//! unified-style format:
//!
//! ```text
//! --- file1
//! +++ file2
//! @@ line 3 @@
//! - removed line
//! + added line
//! ```
//!
//! Identical files produce no output.
//!
//! # Errors
//!
//! - Fewer than two file arguments.
//! - More than two file arguments (only pairwise comparison is supported).
//! - VFS resolution or read errors for either file.

use crate::vfs::Vfs;

/// Execute the `diff` command.
///
/// Resolves and reads both files, splits their contents into lines, then
/// delegates to [`compute_diff`] to produce a list of unified-diff hunks.
///
/// # Returns
///
/// Empty string if the files are identical, or a newline-joined diff output
/// otherwise.
///
/// # Errors
///
/// Returns an error for wrong argument count or VFS read failures.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("diff: missing file operand".to_string());
    }
    if args.len() > 2 {
        return Err("diff: too many arguments".to_string());
    }

    let resolved_a = vfs.resolve_path(args[0])?;
    let resolved_b = vfs.resolve_path(args[1])?;
    let content_a = vfs.read_file(&resolved_a)?;
    let content_b = vfs.read_file(&resolved_b)?;

    let lines_a: Vec<&str> = content_a.lines().collect();
    let lines_b: Vec<&str> = content_b.lines().collect();

    // Compute the diff using LCS-based algorithm.
    let hunks = compute_diff(&lines_a, &lines_b, args[0], args[1]);

    if hunks.is_empty() {
        Ok(String::new()) // Files are identical -- no diff output.
    } else {
        Ok(format!("{}\n", hunks.join("\n")))
    }
}

/// Compute a line-level diff between two slices using the Longest Common
/// Subsequence (LCS) dynamic programming algorithm.
///
/// # Algorithm
///
/// 1. Build an `(n+1) x (m+1)` DP table where `dp[i][j]` is the length of
///    the LCS of `a[0..i]` and `b[0..j]`.
/// 2. Backtrack from `dp[n][m]` to produce a sequence of operations:
///    - `' '` (space) = line is common to both files (context).
///    - `'-'` = line exists only in `a` (removed).
///    - `'+'` = line exists only in `b` (added).
/// 3. Group consecutive non-context operations into hunks, each headed by a
///    `@@ line N @@` marker and prefixed with the original filenames.
///
/// # Returns
///
/// A `Vec<String>` of diff output lines.  Empty if the files are identical.
///
/// # Limitations
///
/// This is O(n*m) in time and space -- adequate for typical files in a
/// browser-based terminal but not suitable for very large files.
fn compute_diff(a: &[&str], b: &[&str], name_a: &str, name_b: &str) -> Vec<String> {
    let n = a.len();
    let m = b.len();

    // Step 1: Build the LCS dynamic programming table.
    // dp[i][j] = length of LCS of a[0..i] and b[0..j].
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if a[i - 1] == b[j - 1] {
                // Lines match: extend the diagonal (common subsequence).
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                // Lines differ: take the best of skipping a line from either side.
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Step 2: Backtrack through the DP table to recover the operation sequence.
    // Each entry is (operation_char, line_number_1indexed, line_text).
    let mut ops: Vec<(char, usize, &str)> = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            // Lines are equal -- context line (common to both).
            ops.push((' ', i, a[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            // Prefer consuming from `b` when scores tie -- this biases toward
            // showing additions before deletions, matching typical diff output.
            ops.push(('+', j, b[j - 1]));
            j -= 1;
        } else {
            // Line exists in `a` but not in `b` -- deletion.
            ops.push(('-', i, a[i - 1]));
            i -= 1;
        }
    }
    // Backtracking produces operations in reverse order; fix that.
    ops.reverse();

    // Step 3: Group consecutive change operations into hunks.
    // Context lines (' ') separate hunks; we skip them and only emit blocks
    // of contiguous '+'/'-' operations.
    let mut output: Vec<String> = Vec::new();
    let mut idx = 0;
    while idx < ops.len() {
        // Skip context lines -- they are only boundaries between hunks.
        if ops[idx].0 == ' ' {
            idx += 1;
            continue;
        }

        // Collect a contiguous block of changes (no context lines in between).
        let start_a = ops[idx].1;
        let mut hunk_lines = Vec::new();
        while idx < ops.len() && ops[idx].0 != ' ' {
            let prefix = match ops[idx].0 {
                '-' => "-",
                '+' => "+",
                _ => " ",
            };
            hunk_lines.push(format!("{} {}", prefix, ops[idx].2));
            idx += 1;
        }
        // Emit the unified-diff header only once (before the first hunk).
        if output.is_empty() {
            output.push(format!("--- {}", name_a));
            output.push(format!("+++ {}", name_b));
        }
        // Hunk header: indicates where in file_a the changes start.
        output.push(format!("@@ line {} @@", start_a));
        output.extend(hunk_lines);
    }

    output
}

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct DiffCommand;

/// Delegates to the standalone [`execute`] function, forwarding the VFS
/// reference needed to read both files.
impl super::Command for DiffCommand {
    fn name(&self) -> &'static str {
        "diff"
    }
    fn description(&self) -> &'static str {
        "Compare two files line by line"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.vfs, ctx.args)
    }
    fn synopsis(&self) -> &'static str {
        "diff file1 file2"
    }
    fn man_description(&self) -> &'static str {
        "Compare two files line by line using a Longest Common Subsequence (LCS) algorithm. Output is in a simplified unified-diff format showing added and removed lines. Identical files produce no output."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["diff old.txt new.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_files_empty_output() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "line1\nline2").unwrap();
        vfs.write_file("/tmp/b.txt", "line1\nline2").unwrap();
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn different_files_shows_diff() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "hello").unwrap();
        vfs.write_file("/tmp/b.txt", "world").unwrap();
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"]).unwrap();
        assert!(!out.is_empty());
        assert!(out.contains("-") || out.contains("+"));
    }

    #[test]
    fn missing_args() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[]).is_err());
        assert!(execute(&vfs, &["/tmp/a.txt"]).is_err());
    }

    #[test]
    fn too_many_args() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["a", "b", "c"]).is_err());
    }

    #[test]
    fn empty_files_are_identical() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "").unwrap();
        vfs.write_file("/tmp/b.txt", "").unwrap();
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn one_empty_one_nonempty() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "").unwrap();
        vfs.write_file("/tmp/b.txt", "content").unwrap();
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"]).unwrap();
        assert!(!out.is_empty());
        assert!(out.contains("+"));
    }
}
