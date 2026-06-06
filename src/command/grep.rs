//! grep command - search for patterns in files

use crate::vfs::Vfs;

/// Execute the `grep` command.
///
/// Usage: `grep [-i] [-n] <pattern> <file> [file2 ...]`
///
/// Searches for lines matching `pattern` in the given files.
/// - `-i` case-insensitive matching
/// - `-n` prefix each matching line with its line number
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut case_insensitive = false;
    let mut show_line_numbers = false;
    let mut positional: Vec<&str> = Vec::new();

    for arg in args {
        match *arg {
            "-i" => case_insensitive = true,
            "-n" => show_line_numbers = true,
            "-in" | "-ni" => {
                case_insensitive = true;
                show_line_numbers = true;
            }
            _ => positional.push(arg),
        }
    }

    if positional.is_empty() {
        return Err("grep: missing pattern".to_string());
    }
    if positional.len() < 2 {
        return Err("grep: missing file operand".to_string());
    }

    let pattern = positional[0];
    let files = &positional[1..];
    let pattern_lower = pattern.to_lowercase();

    let mut output = String::new();

    for path in files {
        let resolved = vfs.resolve_path(path)?;
        let content = vfs.read_file(&resolved)?;
        let show_filename = files.len() > 1;

        for (idx, line) in content.lines().enumerate() {
            let matched = if case_insensitive {
                line.to_lowercase().contains(&pattern_lower)
            } else {
                line.contains(pattern)
            };

            if matched {
                let mut prefix = String::new();
                if show_filename {
                    prefix.push_str(path);
                    prefix.push(':');
                }
                if show_line_numbers {
                    prefix.push_str(&format!("{}:", idx + 1));
                }
                output.push_str(&format!("{}{}\n", prefix, line));
            }
        }
    }

    Ok(output)
}
