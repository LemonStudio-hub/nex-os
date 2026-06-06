//! uniq command - filter adjacent duplicate lines

use crate::vfs::Vfs;

/// Execute the `uniq` command.
///
/// Usage: `uniq [-c] <file>`
///
/// Filters out adjacent duplicate lines. Use `-c` to prefix each line with
/// its occurrence count.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut show_count = false;
    let mut file_path: Option<&str> = None;

    for arg in args {
        match *arg {
            "-c" => show_count = true,
            _ if file_path.is_none() => file_path = Some(arg),
            _ => return Err("uniq: too many arguments".to_string()),
        }
    }

    let path = file_path.ok_or("uniq: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;
    let content = vfs.read_file(&resolved)?;

    let mut output = String::new();
    let mut prev: Option<&str> = None;
    let mut count: usize = 0;

    for line in content.lines() {
        if Some(line) == prev {
            count += 1;
        } else {
            if let Some(p) = prev {
                if show_count {
                    output.push_str(&format!("{:>6} {}\n", count, p));
                } else {
                    output.push_str(&format!("{}\n", p));
                }
            }
            prev = Some(line);
            count = 1;
        }
    }

    // Flush the last group
    if let Some(p) = prev {
        if show_count {
            output.push_str(&format!("{:>6} {}\n", count, p));
        } else {
            output.push_str(&format!("{}\n", p));
        }
    }

    Ok(output)
}
