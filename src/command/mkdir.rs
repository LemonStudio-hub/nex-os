//! mkdir command - create directories

use crate::vfs::Vfs;

pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    let mut recursive = false;
    let mut paths: Vec<&str> = Vec::new();

    for arg in args {
        if *arg == "-p" {
            recursive = true;
        } else {
            paths.push(arg);
        }
    }

    if paths.is_empty() {
        return Err("mkdir: missing operand".to_string());
    }

    for path in paths {
        let resolved = vfs.resolve_path(path)?;

        if recursive {
            // Create each component along the path
            let components: Vec<&str> = resolved.split('/').filter(|s: &&str| !s.is_empty()).collect();
            let mut current = String::new();
            for component in components {
                current.push('/');
                current.push_str(component);
                if !vfs.exists(&current) {
                    vfs.mkdir(&current)?;
                }
            }
        } else {
            if vfs.exists(&resolved) {
                return Err(format!(
                    "mkdir: cannot create directory '{}': File exists",
                    path
                ));
            }

            // Check that the parent directory exists
            let parent = match resolved.rfind('/') {
                Some(0) => "/".to_string(),
                Some(i) => resolved[..i].to_string(),
                None => return Err("mkdir: invalid path".to_string()),
            };

            if !vfs.exists(&parent) || !vfs.is_dir(&parent) {
                return Err(format!(
                    "mkdir: cannot create directory '{}': No such file or directory",
                    path
                ));
            }

            vfs.mkdir(&resolved)?;
        }
    }

    Ok(String::new())
}
