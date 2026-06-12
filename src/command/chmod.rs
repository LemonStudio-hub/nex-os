//! `chmod` -- change file permissions (simulated).
//!
//! # Usage
//!
//! ```text
//! chmod <mode> <file> [file2 ...]
//! ```
//!
//! Simulates changing file permissions in the virtual filesystem.  Because
//! NexOS's VFS has no real permission enforcement layer, this command validates
//! the mode string and checks that each target path exists, but does not
//! persist any permission bits.
//!
//! # Supported modes
//!
//! - **Octal**: 3 or 4 digits, each `0`-`7` (e.g. `755`, `0644`).
//! - **Symbolic without prefix**: `+x`, `-w`, `+rwx` (applies to all classes).
//! - **Symbolic with user prefix**: `u+r`, `g-w`, `a+x` where the prefix is
//!   one of `u` (user), `g` (group), `o` (other), `a` (all).
//!
//! # Errors
//!
//! - Fewer than two arguments (mode + at least one file).
//! - Invalid mode string format.
//! - Any target path does not exist.

use crate::vfs::Vfs;

/// Execute the `chmod` command.
///
/// Validates the mode string, then iterates over each file path to verify it
/// exists in the VFS.  Non-existent paths produce error lines in the output
/// rather than aborting the entire command, matching how real `chmod` reports
/// per-file failures.
///
/// # Returns
///
/// Empty string on full success, or a newline-delimited list of per-file error
/// messages for paths that could not be accessed.
///
/// # Errors
///
/// Returns immediately (no partial output) if the mode is invalid or the
/// argument count is too low.
pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("chmod: missing operand".to_string());
    }

    let mode = args[0];
    let files = &args[1..];

    // Reject syntactically invalid modes early, before touching any files.
    if !is_valid_mode(mode) {
        return Err(format!("chmod: invalid mode: '{}'", mode));
    }

    let mut output = String::new();
    for path in files {
        let resolved = vfs.resolve_path(path)?;
        // Probe the path by attempting both a file read and a directory list.
        // If both fail the path does not exist in the VFS.
        // In a full implementation we would store permissions in node metadata;
        // here we just verify the path exists and report an error if not.
        if vfs.read_file(&resolved).is_err() && vfs.list_dir(&resolved).is_err() {
            output.push_str(&format!(
                "chmod: cannot access '{}': No such file or directory\n",
                path
            ));
        }
        // Successful paths produce no output -- the change is simulated.
    }

    Ok(output)
}

/// Validate a mode string.
///
/// Accepts three forms:
///
/// 1. **Octal** -- 3 or 4 digits, each in `'0'..='7'` (e.g. `"755"`, `"0644"`).
/// 2. **Symbolic without prefix** -- starts with `+` or `-`, followed by any
///    combination of `r`, `w`, `x` (e.g. `"+x"`, `"-w"`, `"+rwx"`).
/// 3. **Symbolic with user prefix** -- starts with one of `u`, `g`, `o`, `a`,
///    then `+` or `-`, then `r`/`w`/`x` characters (e.g. `"u+r"`, `"a-x"`).
///
/// Returns `false` for anything else (e.g. `"999"`, `"invalid"`, `"12345"`).
fn is_valid_mode(mode: &str) -> bool {
    // Octal: exactly 3 or 4 digits, each 0-7.
    if mode.chars().all(|c| ('0'..='7').contains(&c)) && (mode.len() == 3 || mode.len() == 4) {
        return true;
    }
    // Symbolic without prefix: +x, -w, +rw, etc.
    if mode.starts_with('+') || mode.starts_with('-') {
        return mode[1..].chars().all(|c| "rwx".contains(c));
    }
    // Reject modes that end with +/- but have no permission chars after.
    if mode.len() >= 2 && (mode.ends_with('+') || mode.ends_with('-')) {
        return false; // Not a valid position
    }
    // Symbolic with user prefix: u+x, g-w, o+r, a+x
    // First char must be a user-class letter, second must be +/-, rest must be rwx.
    if mode.len() >= 3 {
        let chars: Vec<char> = mode.chars().collect();
        if "ugoa".contains(chars[0]) && (chars[1] == '+' || chars[1] == '-') {
            return chars[2..].iter().all(|c| "rwx".contains(*c));
        }
    }
    false
}

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct ChmodCommand;

/// Delegates to the standalone [`execute`] function.  Needs mutable VFS access
/// because `resolve_path` may normalise paths, though the VFS itself is not
/// mutated in this simulated implementation.
impl super::Command for ChmodCommand {
    fn name(&self) -> &'static str {
        "chmod"
    }
    fn description(&self) -> &'static str {
        "Change file permissions (octal or symbolic)"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&mut ctx.state.vfs, ctx.args)
    }
    fn synopsis(&self) -> &'static str {
        "chmod mode file [file2 ...]"
    }
    fn man_description(&self) -> &'static str {
        "Change file permissions (simulated). Accepts octal modes (e.g. 755, 0644) or symbolic modes (e.g. +x, -w, u+r, a+x). Since the VFS has no real permission enforcement, the command validates the mode and checks that files exist but does not persist permission bits."
    }
    fn examples(&self) -> &'static [&'static str] {
        &[
            "chmod 755 script.sh",
            "chmod +x script.sh",
            "chmod u+r file.txt",
        ]
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
