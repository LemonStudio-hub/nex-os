//! rm command - remove files or directories

use crate::vfs::Vfs;

pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    let mut recursive = false;
    let mut paths: Vec<&str> = Vec::new();

    for arg in args {
        match *arg {
            "-r" | "-rf" | "-fr" => recursive = true,
            _ => paths.push(arg),
        }
    }

    if paths.is_empty() {
        return Err("rm: missing operand".to_string());
    }

    for path in paths {
        let resolved = vfs.resolve_path(path)?;

        if !vfs.exists(&resolved) {
            return Err(format!(
                "rm: cannot remove '{}': No such file or directory",
                path
            ));
        }

        if vfs.is_dir(&resolved) && !recursive {
            return Err(format!("rm: cannot remove '{}': Is a directory", path));
        }

        if recursive {
            vfs.rm_recursive(&resolved)?;
        } else {
            vfs.rm(&resolved)?;
        }
    }

    Ok(String::new())
}
