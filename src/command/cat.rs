//! cat command - display file contents

use crate::vfs::Vfs;

pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("cat: missing file operand".to_string());
    }

    let mut output = String::new();

    for path in args {
        let resolved = vfs.resolve_path(path)?;

        if !vfs.exists(&resolved) {
            return Err(format!("cat: {}: No such file or directory", path));
        }

        if vfs.is_dir(&resolved) {
            return Err(format!("cat: {}: Is a directory", path));
        }

        let content = vfs.read_file(&resolved)?;
        output.push_str(&content);
        // Ensure content ends with a newline
        if !output.ends_with('\n') {
            output.push('\n');
        }
    }

    Ok(output)
}

pub struct CatCommand;

impl super::Command for CatCommand {
    fn name(&self) -> &'static str { "cat" }
    fn description(&self) -> &'static str { "Display file contents" }
    fn accepts_stdin(&self) -> bool { true }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_single_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "hello").unwrap();
        let out = execute(&vfs, &["/tmp/f.txt"]).unwrap();
        assert!(out.contains("hello"));
    }

    #[test]
    fn read_multiple_files() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "AAA").unwrap();
        vfs.write_file("/tmp/b.txt", "BBB").unwrap();
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"]).unwrap();
        assert!(out.contains("AAA"));
        assert!(out.contains("BBB"));
    }

    #[test]
    fn nonexistent_file_errors() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/nope"]).is_err());
    }

    #[test]
    fn directory_errors() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/home"]).is_err());
    }

    #[test]
    fn missing_operand() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &[]).is_err());
    }
}
