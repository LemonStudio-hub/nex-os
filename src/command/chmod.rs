//! `chmod` -- change file permissions.
//!
//! # Usage
//!
//! ```text
//! chmod <mode> <file> [file2 ...]
//! ```
//!
//! Changes the permission mode bits on one or more files or directories.
//! Only the file owner or root can change permissions.
//!
//! # Supported modes
//!
//! - **Octal**: 3 or 4 digits, each `0`-`7` (e.g. `755`, `0644`).
//! - **Symbolic without prefix**: `+x`, `-w`, `+rwx` (applies to all classes).
//! - **Symbolic with user prefix**: `u+r`, `g-w`, `a+x` where the prefix is
//!   one of `u` (user/owner), `g` (group), `o` (other), `a` (all).
//!
//! # Errors
//!
//! - Fewer than two arguments (mode + at least one file).
//! - Invalid mode string format.
//! - Any target path does not exist.
//! - Caller is not the file owner and not root.

use crate::vfs::permissions::{apply_symbolic_mode, parse_octal_mode};
use crate::vfs::{HostFs, Vfs};

/// Execute the `chmod` command.
///
/// Parses the mode string, then iterates over each file path.  For each path,
/// verifies the caller has permission (must be owner or root), then applies
/// the mode change.
pub fn execute(
    vfs: &mut Vfs,
    args: &[&str],
    euid: u32,
    _host_fs: Option<&dyn HostFs>,
) -> Result<String, String> {
    if args.len() < 2 {
        return Err("chmod: missing operand".to_string());
    }

    let mode_str = args[0];
    let files = &args[1..];

    // Validate mode syntax early.
    let is_octal = mode_str.chars().all(|c| ('0'..='7').contains(&c))
        && (mode_str.len() == 3 || mode_str.len() == 4);

    let mut output = String::new();
    for path in files {
        let resolved = match vfs.resolve_path(path) {
            Ok(p) => p,
            Err(e) => {
                output.push_str(&format!("chmod: cannot access '{}': {}\n", path, e));
                continue;
            }
        };

        // Check the node exists
        let meta = match vfs.get_meta(&resolved) {
            Some(m) => m.clone(),
            None => {
                output.push_str(&format!(
                    "chmod: cannot access '{}': No such file or directory\n",
                    path
                ));
                continue;
            }
        };

        // Permission check: must be owner or root
        if euid != 0 && euid != meta.uid {
            output.push_str(&format!(
                "chmod: changing permissions of '{}': Operation not permitted\n",
                path
            ));
            continue;
        }

        // Parse and apply the new mode
        let new_mode = if is_octal {
            match parse_octal_mode(mode_str) {
                Ok(m) => m,
                Err(e) => {
                    output.push_str(&format!("chmod: {}\n", e));
                    continue;
                }
            }
        } else {
            match apply_symbolic_mode(meta.mode, mode_str) {
                Ok(m) => m,
                Err(e) => {
                    output.push_str(&format!("chmod: {}\n", e));
                    continue;
                }
            }
        };

        // Apply the mode change
        if let Some(meta_mut) = vfs.get_meta_mut(&resolved) {
            meta_mut.mode = new_mode;
            vfs.mark_dirty(&resolved);
        }
    }

    Ok(output)
}

/// Unit struct for command registration.
pub struct ChmodCommand;

impl super::Command for ChmodCommand {
    fn name(&self) -> &'static str {
        "chmod"
    }
    fn description(&self) -> &'static str {
        "Change file permissions (octal or symbolic)"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&mut ctx.state.vfs, ctx.args, ctx.state.euid, ctx.host_fs).into()
    }
    fn synopsis(&self) -> &'static str {
        "chmod mode file [file2 ...]"
    }
    fn man_description(&self) -> &'static str {
        "Change file permissions. Accepts octal modes (e.g. 755, 0644) or symbolic modes \
         (e.g. +x, -w, u+r, a+x). Only the file owner or root can change permissions."
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
    use crate::vfs::Vfs;

    #[test]
    fn chmod_octal_as_root() {
        let mut vfs = Vfs::new();
        vfs.touch("/tmp/f.txt").unwrap();
        let out = execute(&mut vfs, &["755", "/tmp/f.txt"], 0, None).unwrap();
        assert!(out.is_empty());
        assert_eq!(vfs.get_meta("/tmp/f.txt").unwrap().mode, 0o755);
    }

    #[test]
    fn chmod_symbolic_as_root() {
        let mut vfs = Vfs::new();
        vfs.touch("/tmp/f.txt").unwrap();
        let out = execute(&mut vfs, &["+x", "/tmp/f.txt"], 0, None).unwrap();
        assert!(out.is_empty());
        // 644 + x for all = 755
        assert_eq!(vfs.get_meta("/tmp/f.txt").unwrap().mode, 0o755);
    }

    #[test]
    fn chmod_denied_for_non_owner() {
        let mut vfs = Vfs::new();
        vfs.touch("/tmp/f.txt").unwrap();
        // Set owner to 1000
        vfs.get_meta_mut("/tmp/f.txt").unwrap().uid = 1000;
        // Try as uid 2000 (not owner, not root)
        let out = execute(&mut vfs, &["777", "/tmp/f.txt"], 2000, None).unwrap();
        assert!(out.contains("Operation not permitted"));
        // Mode unchanged
        assert_eq!(vfs.get_meta("/tmp/f.txt").unwrap().mode, 0o644);
    }

    #[test]
    fn chmod_nonexistent_file() {
        let mut vfs = Vfs::new();
        let out = execute(&mut vfs, &["755", "/nonexistent"], 0, None).unwrap();
        assert!(out.contains("No such file or directory"));
    }

    #[test]
    fn chmod_missing_operand() {
        let mut vfs = Vfs::new();
        assert!(execute(&mut vfs, &[], 0, None).is_err());
    }
}
