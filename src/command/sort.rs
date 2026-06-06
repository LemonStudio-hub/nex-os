//! sort command - sort lines of a file

use crate::vfs::Vfs;

/// Execute the `sort` command.
///
/// Usage: `sort [-r] <file>`
///
/// Sorts lines alphabetically. Use `-r` for reverse order.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut reverse = false;
    let mut file_path: Option<&str> = None;

    for arg in args {
        match *arg {
            "-r" => reverse = true,
            _ if file_path.is_none() => file_path = Some(arg),
            _ => return Err("sort: too many arguments".to_string()),
        }
    }

    let path = file_path.ok_or("sort: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;
    let content = vfs.read_file(&resolved)?;

    let mut lines: Vec<&str> = content.lines().collect();
    if reverse {
        lines.sort_by(|a, b| b.cmp(a));
    } else {
        lines.sort();
    }

    Ok(format!("{}\n", lines.join("\n")))
}
