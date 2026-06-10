//! echo command - display text or write to files

use crate::vfs::Vfs;

pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    // Check for >> or > redirection operators within args
    for i in 0..args.len() {
        if args[i] == ">>" && i + 1 < args.len() {
            let content = args[..i].join(" ");
            let file = args[i + 1];
            let resolved = vfs.resolve_path(file)?;
            let existing = vfs.read_file(&resolved).unwrap_or_default();
            vfs.write_file(&resolved, &format!("{}{}\n", existing, content))?;
            return Ok(String::new());
        }
        if args[i] == ">" && i + 1 < args.len() {
            let content = args[..i].join(" ");
            let file = args[i + 1];
            let resolved = vfs.resolve_path(file)?;
            vfs.write_file(&resolved, &format!("{}\n", content))?;
            return Ok(String::new());
        }
    }

    Ok(format!("{}\n", args.join(" ")))
}

pub struct EchoCommand;

impl super::Command for EchoCommand {
    fn name(&self) -> &'static str { "echo" }
    fn description(&self) -> &'static str { "Display a line of text (supports > and >> redirection)" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_args_prints_empty_line() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, &[]).unwrap();
        assert_eq!(out, "\n");
    }

    #[test]
    fn single_word() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, &["hello"]).unwrap();
        assert_eq!(out, "hello\n");
    }

    #[test]
    fn multiple_words() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, &["hello", "world"]).unwrap();
        assert_eq!(out, "hello world\n");
    }

    #[test]
    fn redirect_overwrite() {
        let mut vfs = Vfs::new();
        execute(&mut vfs, &["first", ">", "/tmp/out.txt"]).unwrap();
        execute(&mut vfs, &["second", ">", "/tmp/out.txt"]).unwrap();
        assert_eq!(vfs.read_file("/tmp/out.txt").unwrap(), "second\n");
    }

    #[test]
    fn redirect_append() {
        let mut vfs = Vfs::new();
        execute(&mut vfs, &["first", ">", "/tmp/out.txt"]).unwrap();
        execute(&mut vfs, &["second", ">>", "/tmp/out.txt"]).unwrap();
        let content = vfs.read_file("/tmp/out.txt").unwrap();
        assert!(content.contains("first"));
        assert!(content.contains("second"));
    }
}
