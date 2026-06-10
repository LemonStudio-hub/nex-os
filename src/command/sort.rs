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

pub struct SortCommand;

impl super::Command for SortCommand {
    fn name(&self) -> &'static str { "sort" }
    fn description(&self) -> &'static str { "Sort lines of a file (-r for reverse)" }
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
        vfs.write_file("/tmp/f.txt", &lines.join("\n")).unwrap();
        vfs
    }

    #[test]
    fn basic_sort() {
        let vfs = vfs_with_lines(&["banana", "apple", "cherry"]);
        let out = execute(&vfs, &["/tmp/f.txt"]).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "apple");
        assert_eq!(lines[1], "banana");
        assert_eq!(lines[2], "cherry");
    }

    #[test]
    fn reverse_sort() {
        let vfs = vfs_with_lines(&["banana", "apple"]);
        let out = execute(&vfs, &["-r", "/tmp/f.txt"]).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "banana");
        assert_eq!(lines[1], "apple");
    }

    #[test]
    fn single_line() {
        let vfs = vfs_with_lines(&["only"]);
        let out = execute(&vfs, &["/tmp/f.txt"]).unwrap();
        assert_eq!(out.trim(), "only");
    }

    #[test]
    fn empty_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/empty.txt", "").unwrap();
        let out = execute(&vfs, &["/tmp/empty.txt"]).unwrap();
        // Should not crash; output may be empty or a single newline
        let _ = out;
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[]).is_err());
    }

    #[test]
    fn too_many_args() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["a.txt", "b.txt"]).is_err());
    }
}
