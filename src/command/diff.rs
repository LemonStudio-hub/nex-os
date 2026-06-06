//! diff command - compare two files line by line

use crate::vfs::Vfs;

/// Execute the `diff` command.
///
/// Usage: `diff <file1> <file2>`
///
/// Compares two files line by line and outputs the differences using a
/// simple unified-style format.
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

    // Simple LCS-based diff
    let hunks = compute_diff(&lines_a, &lines_b, args[0], args[1]);

    if hunks.is_empty() {
        Ok(String::new()) // Files are identical
    } else {
        Ok(format!("{}\n", hunks.join("\n")))
    }
}

/// Compute a simple diff between two slices of lines.
///
/// Returns a list of diff hunk strings. This is a simplified implementation
/// that finds the longest common subsequence to identify changed regions.
fn compute_diff(a: &[&str], b: &[&str], name_a: &str, name_b: &str) -> Vec<String> {
    let n = a.len();
    let m = b.len();

    // Build LCS table
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to produce diff hunks
    let mut ops: Vec<(char, usize, &str)> = Vec::new(); // (op, line_no, text)
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            ops.push((' ', i, a[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            ops.push(('+', j, b[j - 1]));
            j -= 1;
        } else {
            ops.push(('-', i, a[i - 1]));
            i -= 1;
        }
    }
    ops.reverse();

    // Group consecutive changes into hunks
    let mut output = vec![format!("--- {}", name_a), format!("+++ {}", name_b)];
    let mut idx = 0;
    while idx < ops.len() {
        if ops[idx].0 == ' ' {
            idx += 1;
            continue;
        }

        // Collect a hunk of changes
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
        output.push(format!("@@ line {} @@", start_a));
        output.extend(hunk_lines);
    }

    output
}
