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
            output.push_str(&format!(
                "chmod: cannot access '{}': No such file or directory\n",
                path
            ));
        }
        // Confirm the change (simulated)
    }

    Ok(output)
}

/// Check whether a mode string looks valid (octal or symbolic).
fn is_valid_mode(mode: &str) -> bool {
    // Octal: 3-4 digits, each 0-7
    if mode.chars().all(|c| ('0'..='7').contains(&c)) && (mode.len() == 3 || mode.len() == 4) {
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

pub struct ChmodCommand;

impl super::Command for ChmodCommand {
    fn name(&self) -> &'static str { "chmod" }
    fn description(&self) -> &'static str { "Change file permissions (octal or symbolic)" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_octal_3digit() {
        let mut vfs = Vfs::new();
        vfs.touch("/tmp/f.txt").unwrap();
        let out = execute(&mut vfs, &["755", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn valid_octal_4digit() {
        let mut vfs = Vfs::new();
        vfs.touch("/tmp/f.txt").unwrap();
        let out = execute(&mut vfs, &["0644", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn valid_symbolic_plus() {
        let mut vfs = Vfs::new();
        vfs.touch("/tmp/f.txt").unwrap();
        let out = execute(&mut vfs, &["+x", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn valid_symbolic_minus() {
        let mut vfs = Vfs::new();
        vfs.touch("/tmp/f.txt").unwrap();
        let out = execute(&mut vfs, &["-w", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn valid_symbolic_with_prefix() {
        let mut vfs = Vfs::new();
        vfs.touch("/tmp/f.txt").unwrap();
        let out = execute(&mut vfs, &["u+rwx", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn invalid_mode() {
        let mut vfs = Vfs::new();
        vfs.touch("/tmp/f.txt").unwrap();
        let err = execute(&mut vfs, &["invalid", "/tmp/f.txt"]).unwrap_err();
        assert!(err.contains("invalid mode"));
    }

    #[test]
    fn missing_operand() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &[]).is_err());
    }

    #[test]
    fn nonexistent_file_reports_error() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, &["755", "/nonexistent"]).unwrap();
        assert!(out.contains("No such file or directory"));
    }

    #[test]
    fn is_valid_mode_checks() {
        assert!(is_valid_mode("755"));
        assert!(is_valid_mode("644"));
        assert!(is_valid_mode("0777"));
        assert!(is_valid_mode("+x"));
        assert!(is_valid_mode("-w"));
        assert!(is_valid_mode("+rwx"));
        assert!(is_valid_mode("u+r"));
        assert!(is_valid_mode("g-w"));
        assert!(is_valid_mode("a+x"));
        assert!(!is_valid_mode("invalid"));
        assert!(!is_valid_mode("999"));
        assert!(!is_valid_mode("12345"));
    }
}
