//! tail command - display the last N lines of a file

use crate::vfs::Vfs;

/// Execute the `tail` command.
///
/// Usage: `tail [-n COUNT] <file>`
///
/// Displays the last 10 lines of a file by default. Use `-n` to specify
/// a different number of lines.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut count: usize = 10;
    let mut file_path: Option<&str> = None;

    let mut i = 0;
    while i < args.len() {
        if args[i] == "-n" && i + 1 < args.len() {
            count = args[i + 1]
                .parse::<usize>()
                .map_err(|_| format!("tail: invalid line count: '{}'", args[i + 1]))?;
            i += 2;
        } else if args[i].starts_with("-n") && args[i].len() > 2 {
            count = args[i][2..]
                .parse::<usize>()
                .map_err(|_| format!("tail: invalid line count: '{}'", &args[i][2..]))?;
            i += 1;
        } else if file_path.is_none() {
            file_path = Some(args[i]);
            i += 1;
        } else {
            return Err("tail: too many arguments".to_string());
        }
    }

    let path = file_path.ok_or("tail: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;
    let content = vfs.read_file(&resolved)?;

    let all_lines: Vec<&str> = content.lines().collect();
    let skip = if all_lines.len() > count {
        all_lines.len() - count
    } else {
        0
    };
    let lines: Vec<&str> = all_lines.into_iter().skip(skip).collect();
    Ok(format!("{}\n", lines.join("\n")))
}
