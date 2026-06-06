//! cat command - display file contents

use crate::vfs::Vfs;

pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("cat: missing file operand".to_string());
    }

    let mut output = String::new();

    for path in args {
        let resolved = vfs.resolve_path(path)?;

        if !vfs.exists(&resolved) {
            return Err(format!("cat: {}: No such file or directory", path));
        }

        if vfs.is_dir(&resolved) {
            return Err(format!("cat: {}: Is a directory", path));
        }

        let content = vfs.read_file(&resolved)?;
        output.push_str(&content);
        // Ensure content ends with a newline
        if !output.ends_with('\n') {
            output.push('\n');
        }
    }

    Ok(output)
}
