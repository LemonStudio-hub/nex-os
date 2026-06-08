//! cp command - copy files or directories

use crate::vfs::Vfs;

pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("cp: missing destination operand".to_string());
    }

    let src = args[0];
    let dst = args[1];

    let src_resolved = vfs.resolve_path(src)?;
    let dst_resolved = vfs.resolve_path(dst)?;

    if !vfs.exists(&src_resolved) {
        return Err(format!(
            "cp: cannot stat '{}': No such file or directory",
            src
        ));
    }

    vfs.cp(&src_resolved, &dst_resolved)?;
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/src.txt", "data").unwrap();
        execute(&mut vfs, &["/tmp/src.txt", "/tmp/dst.txt"]).unwrap();
        assert_eq!(vfs.read_file("/tmp/dst.txt").unwrap(), "data");
        assert_eq!(vfs.read_file("/tmp/src.txt").unwrap(), "data"); // original intact
    }

    #[test]
    fn copy_into_directory() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", "data").unwrap();
        vfs.mkdir("/tmp/dest").unwrap();
        execute(&mut vfs, &["/tmp/f.txt", "/tmp/dest"]).unwrap();
        assert_eq!(vfs.read_file("/tmp/dest/f.txt").unwrap(), "data");
    }

    #[test]
    fn copy_nonexistent_errors() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/nope", "/tmp/dst"]).is_err());
    }

    #[test]
    fn missing_destination() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &["/tmp/src"]).is_err());
    }
}
