//! find command - find files and directories by name

use crate::vfs::{FsNode, Vfs};

/// Execute the `find` command.
///
/// Usage: `find [path] -name <pattern>`
///
/// Recursively searches for files and directories whose names contain `pattern`.
/// If no path is given, searches from the current directory.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut search_path = ".";
    let mut pattern: Option<&str> = None;

    let mut i = 0;
    while i < args.len() {
        if args[i] == "-name" && i + 1 < args.len() {
            pattern = Some(args[i + 1]);
            i += 2;
        } else if search_path == "." && !args[i].starts_with('-') {
            search_path = args[i];
            i += 1;
        } else {
            i += 1;
        }
    }

    let pattern = pattern.ok_or("find: missing -name argument")?;
    let resolved = vfs.resolve_path(search_path)?;

    let mut results = Vec::new();
    collect_matches(vfs, &resolved, pattern, &mut results);

    if results.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", results.join("\n")))
    }
}

/// Recursively walk the directory tree and collect paths whose final component
/// contains `pattern`.
fn collect_matches(vfs: &Vfs, dir_path: &str, pattern: &str, results: &mut Vec<String>) {
    let entries = match vfs.list_dir(dir_path) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries {
        let name = match &entry {
            FsNode::File(f) => f.name.clone(),
            FsNode::Directory(d) => d.name.clone(),
        };

        let entry_path = if dir_path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", dir_path, name)
        };

        if name.contains(pattern) {
            results.push(entry_path.clone());
        }

        if matches!(entry, FsNode::Directory(_)) {
            collect_matches(vfs, &entry_path, pattern, results);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_vfs() -> Vfs {
        let mut vfs = Vfs::new();
        vfs.mkdir("/tmp/search").unwrap();
        vfs.write_file("/tmp/search/readme.txt", "").unwrap();
        vfs.write_file("/tmp/search/data.csv", "").unwrap();
        vfs.write_file("/tmp/search/readme.md", "").unwrap();
        vfs.mkdir("/tmp/search/sub").unwrap();
        vfs.write_file("/tmp/search/sub/readme.log", "").unwrap();
        vfs
    }

    #[test]
    fn find_by_name() {
        let vfs = setup_vfs();
        let out = execute(&vfs, &["/tmp/search", "-name", "readme"]).unwrap();
        assert!(out.contains("readme.txt"));
        assert!(out.contains("readme.md"));
        assert!(out.contains("readme.log"));
        assert!(!out.contains("data.csv"));
    }

    #[test]
    fn find_no_results() {
        let vfs = setup_vfs();
        let out = execute(&vfs, &["/tmp/search", "-name", "nonexistent"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn find_from_current_dir() {
        let mut vfs = Vfs::new();
        vfs.cwd = "/tmp".to_string();
        vfs.write_file("/tmp/target.txt", "").unwrap();
        let out = execute(&vfs, &["-name", "target"]).unwrap();
        assert!(out.contains("target.txt"));
    }

    #[test]
    fn find_missing_name_arg() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/tmp"]).is_err());
    }
}
