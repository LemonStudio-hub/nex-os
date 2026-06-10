//! tee command - read from stdin and write to both stdout and files

use crate::vfs::Vfs;

/// Execute the `tee` command.
///
/// Usage: `tee [-a] <file> [file2 ...]`
///
/// Reads `input` (stdin from a pipe) and writes it to both stdout and the
/// specified files. `-a` appends instead of overwriting.
pub fn execute(vfs: &mut Vfs, input: &str, args: &[&str]) -> Result<String, String> {
    let mut append = false;
    let mut files: Vec<&str> = Vec::new();

    for arg in args {
        match *arg {
            "-a" => append = true,
            _ => files.push(arg),
        }
    }

    if files.is_empty() {
        return Err("tee: missing file operand".to_string());
    }

    for path in &files {
        let resolved = vfs.resolve_path(path)?;
        let write_result = if append {
            let existing = vfs.read_file(&resolved).unwrap_or_default();
            vfs.write_file(&resolved, &format!("{}{}", existing, input))
        } else {
            vfs.write_file(&resolved, input)
        };
        if let Err(e) = write_result {
            return Err(format!("tee: {}: {}", path, e));
        }
    }

    // Also output to stdout
    Ok(input.to_string())
}

pub struct TeeCommand;

impl super::Command for TeeCommand {
    fn name(&self) -> &'static str { "tee" }
    fn description(&self) -> &'static str { "Write stdin to stdout and files (-a for append)" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.stdin, ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_to_single_file() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, "hello world", &["/tmp/out.txt"]).unwrap();
        assert_eq!(out, "hello world");
        assert_eq!(vfs.read_file("/tmp/out.txt").unwrap(), "hello world");
    }

    #[test]
    fn write_to_multiple_files() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, "data", &["/tmp/a.txt", "/tmp/b.txt"]).unwrap();
        assert_eq!(out, "data");
        assert_eq!(vfs.read_file("/tmp/a.txt").unwrap(), "data");
        assert_eq!(vfs.read_file("/tmp/b.txt").unwrap(), "data");
    }

    #[test]
    fn append_mode() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/log.txt", "first\n").unwrap();
        let out = execute(&mut vfs, "second\n", &["-a", "/tmp/log.txt"]).unwrap();
        assert_eq!(out, "second\n");
        assert_eq!(vfs.read_file("/tmp/log.txt").unwrap(), "first\nsecond\n");
    }

    #[test]
    fn overwrite_mode() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/log.txt", "old").unwrap();
        let out = execute(&mut vfs, "new", &["/tmp/log.txt"]).unwrap();
        assert_eq!(out, "new");
        assert_eq!(vfs.read_file("/tmp/log.txt").unwrap(), "new");
    }

    #[test]
    fn missing_file_operand() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, "data", &[]).is_err());
    }

    #[test]
    fn empty_input() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, "", &["/tmp/out.txt"]).unwrap();
        assert_eq!(out, "");
        assert_eq!(vfs.read_file("/tmp/out.txt").unwrap(), "");
    }
}
