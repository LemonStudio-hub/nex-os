//! ln command - create hard links (simulated as copies) in the VFS

use crate::vfs::Vfs;

/// Execute the `ln` command.
///
/// Usage: `ln [-s] <target> <link_name>`
///
/// Creates a link. Since the VFS does not support true hard links or inodes,
/// `-s` (symbolic link) creates a small text file containing the target path.
/// Without `-s`, the target file is copied to the link name.
pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    let mut symbolic = false;
    let mut positional: Vec<&str> = Vec::new();

    for arg in args {
        match *arg {
            "-s" => symbolic = true,
            _ => positional.push(arg),
        }
    }

    if positional.len() < 2 {
        return Err("ln: missing file operand".to_string());
    }

    let target = positional[0];
    let link_name = positional[1];

    let resolved_target = vfs.resolve_path(target)?;

    // Verify target exists
    if vfs.read_file(&resolved_target).is_err() && vfs.list_dir(&resolved_target).is_err() {
        return Err(format!("ln: '{}': No such file or directory", target));
    }

    let resolved_link = vfs.resolve_path(link_name)?;

    if symbolic {
        // Create a symlink file containing the target path
        vfs.write_file(&resolved_link, &format!("-> {}\n", target))
            .map_err(|e| format!("ln: {}", e))?;
    } else {
        // Hard link simulated as a copy
        let content = vfs
            .read_file(&resolved_target)
            .map_err(|_| format!("ln: '{}': Cannot link directory", target))?;
        vfs.write_file(&resolved_link, &content)
            .map_err(|e| format!("ln: {}", e))?;
    }

    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbolic_link() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/target.txt", "content").unwrap();
        execute(&mut vfs, &["-s", "/tmp/target.txt", "/tmp/link.txt"]).unwrap();
        let out = vfs.read_file("/tmp/link.txt").unwrap();
        assert!(out.contains("-> /tmp/target.txt"));
    }

    #[test]
    fn hard_link_copies_content() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/src.txt", "data").unwrap();
        execute(&mut vfs, &["/tmp/src.txt", "/tmp/copy.txt"]).unwrap();
        assert_eq!(vfs.read_file("/tmp/copy.txt").unwrap(), "data");
    }

    #[test]
    fn missing_operand() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &[]).is_err());
        assert!(execute(&mut vfs, &["/tmp/a"]).is_err());
    }

    #[test]
    fn nonexistent_target() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/nope", "/tmp/link"]).is_err());
    }
}
