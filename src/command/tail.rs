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

pub struct TailCommand;

impl super::Command for TailCommand {
    fn name(&self) -> &'static str { "tail" }
    fn description(&self) -> &'static str { "Display last N lines of a file (-n COUNT)" }
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
        assert!(out.contains("line11"));
        assert!(out.contains("line20"));
        assert!(!out.contains("line10"));
    }

    #[test]
    fn custom_count() {
        let vfs = vfs_with_lines(20);
        let out = execute(&vfs, &["-n", "3", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("line18"));
        assert!(out.contains("line20"));
    }

    #[test]
    fn compact_n_flag() {
        let vfs = vfs_with_lines(10);
        let out = execute(&vfs, &["-n2", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("line9"));
        assert!(out.contains("line10"));
    }

    #[test]
    fn file_shorter_than_count() {
        let vfs = vfs_with_lines(3);
        let out = execute(&vfs, &["-n", "10", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("line1"));
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
