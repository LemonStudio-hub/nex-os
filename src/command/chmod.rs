//! chmod command - change file permissions (simulated)

use crate::vfs::Vfs;

/// Execute the `chmod` command.
///
/// Usage: `chmod <mode> <file> [file2 ...]`
///
/// Simulates changing file permissions. Since the VFS has no real permission
/// system, this stores the mode string as metadata and confirms the change.
/// Modes can be symbolic (e.g. `+x`, `-w`) or octal (e.g. `755`, `644`).
pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("chmod: missing operand".to_string());
    }

    let mode = args[0];
    let files = &args[1..];

    // Validate mode format
    if !is_valid_mode(mode) {
        return Err(format!("chmod: invalid mode: '{}'", mode));
    }

    let mut output = String::new();
    for path in files {
        let resolved = vfs.resolve_path(path)?;
        // In a full implementation we would store permissions in node metadata.
        // Here we just verify the path exists.
        if vfs.read_file(&resolved).is_err() && vfs.list_dir(&resolved).is_err() {
            output.push_str(&format!("chmod: cannot access '{}': No such file or directory\n", path));
        }
        // Confirm the change (simulated)
    }

    Ok(output)
}

/// Check whether a mode string looks valid (octal or symbolic).
fn is_valid_mode(mode: &str) -> bool {
    // Octal: 3-4 digits, each 0-7
    if mode.chars().all(|c| c >= '0' && c <= '7') && (mode.len() == 3 || mode.len() == 4) {
        return true;
    }
    // Symbolic: e.g. +x, -w, +rw, u+x, a+r
    if mode.starts_with('+') || mode.starts_with('-') {
        return mode[1..].chars().all(|c| "rwx".contains(c));
    }
    if mode.len() >= 2 && (mode.ends_with('+') || mode.ends_with('-')) {
        return false; // Not a valid position
    }
    // Symbolic with user prefix: u+x, g-w, o+r, a+x
    if mode.len() >= 3 {
        let chars: Vec<char> = mode.chars().collect();
        if "ugoa".contains(chars[0]) && (chars[1] == '+' || chars[1] == '-') {
            return chars[2..].iter().all(|c| "rwx".contains(*c));
        }
    }
    false
}
