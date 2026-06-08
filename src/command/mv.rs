//! mv command - move or rename files and directories

use crate::vfs::Vfs;

pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("mv: missing destination operand".to_string());
    }

    let src = args[0];
    let dst = args[1];

    let src_resolved = vfs.resolve_path(src)?;
    let dst_resolved = vfs.resolve_path(dst)?;

    if !vfs.exists(&src_resolved) {
        return Err(format!(
            "mv: cannot stat '{}': No such file or directory",
            src
        ));
    }

    vfs.mv(&src_resolved, &dst_resolved)?;
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/old.txt", "data").unwrap();
        execute(&mut vfs, &["/tmp/old.txt", "/tmp/new.txt"]).unwrap();
        assert!(!vfs.exists("/tmp/old.txt"));
        assert_eq!(vfs.read_file("/tmp/new.txt").unwrap(), "data");
    }

    #[test]
    fn move_nonexistent_errors() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/nope", "/tmp/dst"]).is_err());
    }

    #[test]
    fn missing_destination() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/tmp/src"]).is_err());
    }
}
