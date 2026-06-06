//! du command - estimate disk usage of files and directories

use crate::vfs::{FsNode, Vfs};

/// Execute the `du` command.
///
/// Usage: `du [-h] [-s] [path]`
///
/// Estimates disk usage (byte count based on content length) for each
/// directory entry. `-s` shows only the total for the given path.
/// `-h` displays sizes in human-readable format (KB).
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut human = false;
    let mut summary = false;
    let mut path = ".";

    for arg in args {
        match *arg {
            "-h" => human = true,
            "-s" => summary = true,
            _ if !arg.starts_with('-') => path = arg,
            _ => return Err(format!("du: unknown option: {}", arg)),
        }
    }

    let resolved = vfs.resolve_path(path)?;

    if summary {
        let total = dir_size(vfs, &resolved);
        return Ok(format!("{}\t{}\n", format_size(total, human), path));
    }

    let mut output = String::new();
    collect_sizes(vfs, &resolved, path, human, &mut output);
    let total = dir_size(vfs, &resolved);
    output.push_str(&format!("{}\t{}\n", format_size(total, human), path));
    Ok(output)
}

/// Recursively compute the byte size of a directory.
fn dir_size(vfs: &Vfs, path: &str) -> usize {
    let entries = match vfs.list_dir(path) {
        Ok(e) => e,
        Err(_) => {
            // Might be a file
            return vfs.read_file(path).map(|c| c.len()).unwrap_or(0);
        }
    };

    let mut total = 0;
    for entry in entries {
        let name = match &entry {
            FsNode::File(f) => f.name.clone(),
            FsNode::Directory(d) => d.name.clone(),
        };
        let entry_path = if path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", path, name)
        };

        match entry {
            FsNode::File(f) => total += f.content.len(),
            FsNode::Directory(_) => total += dir_size(vfs, &entry_path),
        }
    }
    total
}

/// Recursively collect sizes for each subdirectory.
fn collect_sizes(vfs: &Vfs, abs_path: &str, display_path: &str, human: bool, output: &mut String) {
    let entries = match vfs.list_dir(abs_path) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries {
        let name = match &entry {
            FsNode::File(f) => f.name.clone(),
            FsNode::Directory(d) => d.name.clone(),
        };
        let entry_abs = if abs_path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", abs_path, name)
        };
        let entry_display = if display_path == "." {
            name.clone()
        } else {
            format!("{}/{}", display_path, name)
        };

        if matches!(entry, FsNode::Directory(_)) {
            let size = dir_size(vfs, &entry_abs);
            output.push_str(&format!(
                "{}\t{}\n",
                format_size(size, human),
                entry_display
            ));
            collect_sizes(vfs, &entry_abs, &entry_display, human, output);
        }
    }
}

/// Format a byte count, optionally in human-readable form.
fn format_size(bytes: usize, human: bool) -> String {
    if human {
        if bytes >= 1024 * 1024 {
            format!("{:.1}M", bytes as f64 / 1024.0 / 1024.0)
        } else if bytes >= 1024 {
            format!("{:.1}K", bytes as f64 / 1024.0)
        } else {
            format!("{}B", bytes)
        }
    } else {
        // Display in KB (like real `du`)
        let kb = (bytes + 1023) / 1024;
        format!("{}K", kb)
    }
}
