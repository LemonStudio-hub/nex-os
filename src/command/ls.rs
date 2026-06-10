//! ls command - list directory contents

use crate::vfs::{FsNode, Vfs};

pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut long_format = false;
    let mut path = ".";
    let mut path_set = false;

    for arg in args {
        if *arg == "-l" {
            long_format = true;
        } else if !path_set {
            path = arg;
            path_set = true;
        }
    }

    let resolved = vfs.resolve_path(path)?;

    if !vfs.exists(&resolved) {
        return Err(format!(
            "ls: cannot access '{}': No such file or directory",
            path
        ));
    }

    // If target is a file, just print its name
    if !vfs.is_dir(&resolved) {
        let name = resolved
            .rfind('/')
            .map(|i| &resolved[i + 1..])
            .unwrap_or(&resolved);
        return if long_format {
            Ok(format!("- {}\n", name))
        } else {
            Ok(format!("{}\n", name))
        };
    }

    let entries = vfs.list_dir(&resolved)?;

    // Sort entries by name
    let mut sorted = entries;
    sorted.sort_by(|a, b| {
        let name_a = match a {
            FsNode::File(f) => &f.name,
            FsNode::Directory(d) => &d.name,
        };
        let name_b = match b {
            FsNode::File(f) => &f.name,
            FsNode::Directory(d) => &d.name,
        };
        name_a.cmp(name_b)
    });

    if long_format {
        let mut output = String::new();
        for entry in &sorted {
            match entry {
                FsNode::File(f) => output.push_str(&format!("- {}\n", f.name)),
                FsNode::Directory(d) => output.push_str(&format!("d {}/\n", d.name)),
            }
        }
        Ok(output)
    } else {
        let names: Vec<String> = sorted
            .iter()
            .map(|entry| match entry {
                FsNode::File(f) => f.name.clone(),
                FsNode::Directory(d) => format!("{}/", d.name),
            })
            .collect();
        if names.is_empty() {
            Ok("\n".to_string())
        } else {
            Ok(format!("{}\n", names.join("  ")))
        }
    }
}

pub struct LsCommand;

impl super::Command for LsCommand {
    fn name(&self) -> &'static str { "ls" }
    fn description(&self) -> &'static str { "List directory contents (-l for long format)" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_directory() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "").unwrap();
        vfs.mkdir("/tmp/sub").unwrap();
        let out = execute(&vfs, &["/tmp"]).unwrap();
        assert!(out.contains("a.txt"));
        assert!(out.contains("sub/"));
    }

    #[test]
    fn list_file_shows_name() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "").unwrap();
        let out = execute(&vfs, &["/tmp/f.txt"]).unwrap();
        assert!(out.contains("f.txt"));
    }

    #[test]
    fn long_format() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "").unwrap();
        vfs.mkdir("/tmp/d").unwrap();
        let out = execute(&vfs, &["-l", "/tmp"]).unwrap();
        assert!(out.contains("- f.txt"));
        assert!(out.contains("d d/"));
    }

    #[test]
    fn empty_directory() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/tmp/empty").unwrap();
        let out = execute(&vfs, &["/tmp/empty"]).unwrap();
        assert!(!out.is_empty()); // still outputs a newline
    }

    #[test]
    fn nonexistent_path() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/nonexistent"]).is_err());
    }

    #[test]
    fn sorted_output() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/b.txt", "").unwrap();
        vfs.write_file("/tmp/a.txt", "").unwrap();
        vfs.write_file("/tmp/c.txt", "").unwrap();
        let out = execute(&vfs, &["/tmp"]).unwrap();
        let a = out.find("a.txt").unwrap();
        let b = out.find("b.txt").unwrap();
        let c = out.find("c.txt").unwrap();
        assert!(a < b);
        assert!(b < c);
    }
}
