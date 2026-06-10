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

pub struct GrepCommand;

impl super::Command for GrepCommand {
    fn name(&self) -> &'static str { "grep" }
    fn description(&self) -> &'static str { "Search for patterns in files (-i case-insensitive, -n line numbers)" }
    fn accepts_stdin(&self) -> bool { true }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs_with_lines(lines: &[&str]) -> Vfs {
        let mut vfs = Vfs::new();
        let content = lines.join("\n");
        vfs.write_file("/tmp/f.txt", &content).unwrap();
        vfs
    }

    #[test]
    fn basic_match() {
        let vfs = vfs_with_lines(&["hello", "world", "hello again"]);
        let out = execute(&vfs, &["hello", "/tmp/f.txt"]).unwrap();
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn no_match_returns_empty() {
        let vfs = vfs_with_lines(&["hello", "world"]);
        let out = execute(&vfs, &["xyz", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn case_insensitive() {
        let vfs = vfs_with_lines(&["Hello", "WORLD", "hello"]);
        let out = execute(&vfs, &["-i", "hello", "/tmp/f.txt"]).unwrap();
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn line_numbers() {
        let vfs = vfs_with_lines(&["aaa", "bbb", "aaa"]);
        let out = execute(&vfs, &["-n", "aaa", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("1:"));
        assert!(out.contains("3:"));
    }

    #[test]
    fn combined_in_flags() {
        let vfs = vfs_with_lines(&["Hello", "world"]);
        let out = execute(&vfs, &["-in", "hello", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("1:"));
    }

    #[test]
    fn combined_ni_flags() {
        let vfs = vfs_with_lines(&["Hello", "world"]);
        let out = execute(&vfs, &["-ni", "hello", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("1:"));
    }

    #[test]
    fn multiple_files_shows_filename() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "hello").unwrap();
        vfs.write_file("/tmp/b.txt", "hello").unwrap();
        let out = execute(&vfs, &["hello", "/tmp/a.txt", "/tmp/b.txt"]).unwrap();
        assert!(out.contains("/tmp/a.txt:"));
        assert!(out.contains("/tmp/b.txt:"));
    }

    #[test]
    fn missing_pattern() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[]).is_err());
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["pattern"]).is_err());
    }
}
