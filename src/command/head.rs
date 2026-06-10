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

pub struct HeadCommand;

impl super::Command for HeadCommand {
    fn name(&self) -> &'static str { "head" }
    fn description(&self) -> &'static str { "Display first N lines of a file (-n COUNT)" }
    fn accepts_stdin(&self) -> bool { true }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs_with_lines(n: usize) -> Vfs {
        let mut vfs = Vfs::new();
        let content: String = (1..=n)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        vfs.write_file("/tmp/f.txt", &content).unwrap();
        vfs
    }

    #[test]
    fn default_ten_lines() {
        let vfs = vfs_with_lines(20);
        let out = execute(&vfs, &["/tmp/f.txt"]).unwrap();
        assert!(out.contains("line1"));
        assert!(out.contains("line10"));
        assert!(!out.contains("line11"));
    }

    #[test]
    fn custom_count() {
        let vfs = vfs_with_lines(20);
        let out = execute(&vfs, &["-n", "3", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("line1"));
        assert!(out.contains("line3"));
        assert!(!out.contains("line4"));
    }

    #[test]
    fn compact_n_flag() {
        let vfs = vfs_with_lines(20);
        let out = execute(&vfs, &["-n5", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("line5"));
        assert!(!out.contains("line6"));
    }

    #[test]
    fn file_shorter_than_count() {
        let vfs = vfs_with_lines(3);
        let out = execute(&vfs, &["-n", "10", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("line3"));
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[]).is_err());
    }

    #[test]
    fn invalid_count() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["-n", "abc", "/tmp/f.txt"]).is_err());
    }
}
