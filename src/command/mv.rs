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
