//! head command - display the first N lines of a file

use crate::vfs::Vfs;

/// Execute the `head` command.
///
/// Usage: `head [-n COUNT] <file>`
///
/// Displays the first 10 lines of a file by default. Use `-n` to specify
/// a different number of lines.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut count: usize = 10;
    let mut file_path: Option<&str> = None;

    let mut i = 0;
    while i < args.len() {
        if args[i] == "-n" && i + 1 < args.len() {
            count = args[i + 1]
                .parse::<usize>()
                .map_err(|_| format!("head: invalid line count: '{}'", args[i + 1]))?;
            i += 2;
        } else if args[i].starts_with("-n") && args[i].len() > 2 {
            // Handle `-n5` style
            count = args[i][2..]
                .parse::<usize>()
                .map_err(|_| format!("head: invalid line count: '{}'", &args[i][2..]))?;
            i += 1;
        } else if file_path.is_none() {
            file_path = Some(args[i]);
            i += 1;
        } else {
            return Err("head: too many arguments".to_string());
        }
    }

    let path = file_path.ok_or("head: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;
    let content = vfs.read_file(&resolved)?;

    let lines: Vec<&str> = content.lines().take(count).collect();
    Ok(format!("{}\n", lines.join("\n")))
}
