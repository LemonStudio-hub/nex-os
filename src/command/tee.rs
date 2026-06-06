//! tee command - read from stdin and write to both stdout and files

use crate::vfs::Vfs;

/// Execute the `tee` command.
///
/// Usage: `tee [-a] <file> [file2 ...]`
///
/// Reads `input` (stdin from a pipe) and writes it to both stdout and the
/// specified files. `-a` appends instead of overwriting.
pub fn execute(vfs: &mut Vfs, input: &str, args: &[&str]) -> Result<String, String> {
    let mut append = false;
    let mut files: Vec<&str> = Vec::new();

    for arg in args {
        match *arg {
            "-a" => append = true,
            _ => files.push(arg),
        }
    }

    if files.is_empty() {
        return Err("tee: missing file operand".to_string());
    }

    for path in &files {
        let resolved = vfs.resolve_path(path)?;
        let write_result = if append {
            let existing = vfs.read_file(&resolved).unwrap_or_default();
            vfs.write_file(&resolved, &format!("{}{}", existing, input))
        } else {
            vfs.write_file(&resolved, input)
        };
        if let Err(e) = write_result {
            return Err(format!("tee: {}: {}", path, e));
        }
    }

    // Also output to stdout
    Ok(input.to_string())
}
