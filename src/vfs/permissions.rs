//! Filesystem permission types and access-check logic.
//!
//! Provides [`NodeMeta`] (mode, uid, gid, mtime attached to every VFS node),
//! [`AccessMode`] (the kind of access being requested), and [`check_access`]
//! which enforces standard Unix rwx permission bits.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// NodeMeta — metadata attached to every VFS node
// ---------------------------------------------------------------------------

/// Metadata attached to every file and directory in the VFS.
///
/// All fields use `#[serde(default)]` so that old VFS JSON without metadata
/// deserialises cleanly (root:root, 0o644 for files, 0o755 for dirs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMeta {
    /// Permission mode (lower 12 bits: sticky + rwxrwxrwx).
    #[serde(default)]
    pub mode: u16,
    /// Numeric user ID of the owner.  0 = root.
    #[serde(default)]
    pub uid: u32,
    /// Numeric group ID.  0 = root.
    #[serde(default)]
    pub gid: u32,
    /// Modification time as seconds since Unix epoch.
    #[serde(default)]
    pub mtime: u64,
}

impl NodeMeta {
    /// Default metadata for a newly created file (mode 0o644).
    pub fn file_default(uid: u32, gid: u32, mtime: u64) -> Self {
        Self {
            mode: 0o644,
            uid,
            gid,
            mtime,
        }
    }

    /// Default metadata for a newly created directory (mode 0o755).
    pub fn dir_default(uid: u32, gid: u32, mtime: u64) -> Self {
        Self {
            mode: 0o755,
            uid,
            gid,
            mtime,
        }
    }

    /// Default metadata for the root directory (mode 0o755, root:root).
    pub fn root_default(mtime: u64) -> Self {
        Self {
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime,
        }
    }
}

/// Default `NodeMeta` for `FileNode` (used by `#[serde(default)]`).
pub fn default_file_meta() -> NodeMeta {
    NodeMeta {
        mode: 0o644,
        uid: 0,
        gid: 0,
        mtime: 0,
    }
}

/// Default `NodeMeta` for `DirNode` (used by `#[serde(default)]`).
pub fn default_dir_meta() -> NodeMeta {
    NodeMeta {
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
    }
}

// ---------------------------------------------------------------------------
// AccessMode — the kind of filesystem access being requested
// ---------------------------------------------------------------------------

/// The type of access being requested on a VFS node.
#[derive(Debug, Clone, Copy)]
pub enum AccessMode {
    /// Read file content or list directory entries.
    Read,
    /// Write file content or add/remove directory entries.
    Write,
    /// Execute a file or search (traverse) a directory.
    Execute,
}

// ---------------------------------------------------------------------------
// check_access — Unix permission enforcement
// ---------------------------------------------------------------------------

/// Check whether the given identity can perform `access` on the node.
///
/// Returns `Ok(())` if allowed, `Err(message)` if denied.
/// Root (`uid == 0`) always passes.
pub fn check_access(
    node_meta: &NodeMeta,
    access: AccessMode,
    uid: u32,
    gid: u32,
) -> Result<(), String> {
    // Root bypasses all permission checks.
    if uid == 0 {
        return Ok(());
    }

    let mode = node_meta.mode;
    // Select the 3-bit permission triplet based on identity match.
    let triplet = if uid == node_meta.uid {
        // Owner bits (bits 8-6)
        (mode >> 6) & 7
    } else if gid == node_meta.gid {
        // Group bits (bits 5-3)
        (mode >> 3) & 7
    } else {
        // Other bits (bits 2-0)
        mode & 7
    };

    let has = match access {
        AccessMode::Read => triplet & 4 != 0,
        AccessMode::Write => triplet & 2 != 0,
        AccessMode::Execute => triplet & 1 != 0,
    };

    if has {
        Ok(())
    } else {
        Err("Permission denied".to_string())
    }
}

/// Check whether the user can delete a file inside a (potentially sticky) directory.
///
/// In a sticky directory (mode bit `0o1000`), only the file owner, the
/// directory owner, or root can delete files.
pub fn check_delete_in_sticky(
    parent_meta: &NodeMeta,
    file_meta: &NodeMeta,
    uid: u32,
) -> Result<(), String> {
    if parent_meta.mode & 0o1000 != 0 && uid != 0 && uid != file_meta.uid && uid != parent_meta.uid
    {
        return Err("Permission denied: sticky directory".to_string());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Mode parsing — octal and symbolic
// ---------------------------------------------------------------------------

/// Parse an octal mode string (3 or 4 digits) into a `u16`.
///
/// # Examples
///
/// - `"755"` → `0o755`
/// - `"0644"` → `0o644`
/// - `"1777"` → `0o1777`
pub fn parse_octal_mode(mode_str: &str) -> Result<u16, String> {
    if mode_str.is_empty() || mode_str.len() > 4 {
        return Err(format!("invalid mode: '{}'", mode_str));
    }
    if !mode_str.chars().all(|c| ('0'..='7').contains(&c)) {
        return Err(format!("invalid mode: '{}'", mode_str));
    }
    u16::from_str_radix(mode_str, 8).map_err(|_| format!("invalid mode: '{}'", mode_str))
}

/// Parse a symbolic mode string (e.g. `+x`, `-w`, `u+r`, `a+x`, `g-rwx`)
/// and apply it to the current mode, returning the new mode.
pub fn apply_symbolic_mode(current_mode: u16, symbolic: &str) -> Result<u16, String> {
    if symbolic.is_empty() {
        return Err("invalid mode: empty string".to_string());
    }

    let chars: Vec<char> = symbolic.chars().collect();

    // Determine which classes to apply to and the operation position.
    let (class_mask, op_idx) = if chars.len() >= 2 && "ugoa".contains(chars[0]) {
        let mask = match chars[0] {
            'u' => 0o700, // owner
            'g' => 0o070, // group
            'o' => 0o007, // other
            'a' => 0o777, // all
            _ => return Err(format!("invalid mode: '{}'", symbolic)),
        };
        (mask, 1)
    } else {
        // No prefix: apply to all (a)
        (0o777, 0)
    };

    if op_idx >= chars.len() {
        return Err(format!("invalid mode: '{}'", symbolic));
    }

    let op = chars[op_idx];
    if op != '+' && op != '-' {
        return Err(format!("invalid mode: '{}'", symbolic));
    }

    // Parse permission bits after the operator.
    let mut bits: u16 = 0;
    for &c in &chars[op_idx + 1..] {
        match c {
            'r' => bits |= 4,
            'w' => bits |= 2,
            'x' => bits |= 1,
            _ => return Err(format!("invalid mode: '{}'", symbolic)),
        }
    }

    // Shift bits into the correct position for the class mask.
    let effective_bits = if class_mask == 0o700 {
        bits << 6
    } else if class_mask == 0o070 {
        bits << 3
    } else if class_mask == 0o007 {
        bits
    } else {
        // all: apply to all three positions
        (bits << 6) | (bits << 3) | bits
    };

    match op {
        '+' => Ok(current_mode | effective_bits),
        '-' => Ok(current_mode & !effective_bits),
        _ => Err(format!("invalid mode: '{}'", symbolic)),
    }
}

/// Format a mode as a 10-character `rwxrwxrwx` string for `ls -l`.
///
/// The first character is `d` for directories, `-` for files.
/// The sticky bit is shown as `t`/`T` on the execute bit of "other".
pub fn format_mode(mode: u16, is_dir: bool) -> String {
    let kind = if is_dir { 'd' } else { '-' };

    let rwx = |triplet: u16| -> (char, char, char) {
        (
            if triplet & 4 != 0 { 'r' } else { '-' },
            if triplet & 2 != 0 { 'w' } else { '-' },
            if triplet & 1 != 0 { 'x' } else { '-' },
        )
    };

    let owner = rwx((mode >> 6) & 7);
    let group = rwx((mode >> 3) & 7);
    let other = rwx(mode & 7);

    // Sticky bit on the "other" execute position.
    let other_x = if mode & 0o1000 != 0 {
        if other.2 == 'x' {
            't'
        } else {
            'T'
        }
    } else {
        other.2
    };

    format!(
        "{}{}{}{}{}{}{}{}{}{}",
        kind, owner.0, owner.1, owner.2, group.0, group.1, group.2, other.0, other.1, other_x,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_access_owner_read() {
        let meta = NodeMeta {
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            mtime: 0,
        };
        assert!(check_access(&meta, AccessMode::Read, 1000, 1000).is_ok());
        assert!(check_access(&meta, AccessMode::Write, 1000, 1000).is_ok());
        assert!(check_access(&meta, AccessMode::Execute, 1000, 1000).is_err());
    }

    #[test]
    fn check_access_other_read() {
        let meta = NodeMeta {
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            mtime: 0,
        };
        assert!(check_access(&meta, AccessMode::Read, 2000, 2000).is_ok());
        assert!(check_access(&meta, AccessMode::Write, 2000, 2000).is_err());
    }

    #[test]
    fn check_access_group() {
        let meta = NodeMeta {
            mode: 0o640,
            uid: 1000,
            gid: 1000,
            mtime: 0,
        };
        // Same group
        assert!(check_access(&meta, AccessMode::Read, 2000, 1000).is_ok());
        assert!(check_access(&meta, AccessMode::Write, 2000, 1000).is_err());
        // Different group
        assert!(check_access(&meta, AccessMode::Read, 2000, 2000).is_err());
    }

    #[test]
    fn check_access_root_bypass() {
        let meta = NodeMeta {
            mode: 0o000,
            uid: 1000,
            gid: 1000,
            mtime: 0,
        };
        assert!(check_access(&meta, AccessMode::Read, 0, 0).is_ok());
        assert!(check_access(&meta, AccessMode::Write, 0, 0).is_ok());
        assert!(check_access(&meta, AccessMode::Execute, 0, 0).is_ok());
    }

    #[test]
    fn parse_octal_valid() {
        assert_eq!(parse_octal_mode("755").unwrap(), 0o755);
        assert_eq!(parse_octal_mode("644").unwrap(), 0o644);
        assert_eq!(parse_octal_mode("0644").unwrap(), 0o644);
        assert_eq!(parse_octal_mode("1777").unwrap(), 0o1777);
        assert_eq!(parse_octal_mode("0").unwrap(), 0);
    }

    #[test]
    fn parse_octal_invalid() {
        assert!(parse_octal_mode("").is_err());
        assert!(parse_octal_mode("999").is_err());
        assert!(parse_octal_mode("12345").is_err());
        assert!(parse_octal_mode("abc").is_err());
    }

    #[test]
    fn symbolic_plus_x() {
        assert_eq!(apply_symbolic_mode(0o644, "+x").unwrap(), 0o755);
    }

    #[test]
    fn symbolic_minus_w() {
        assert_eq!(apply_symbolic_mode(0o777, "-w").unwrap(), 0o555);
    }

    #[test]
    fn symbolic_user_plus_r() {
        assert_eq!(apply_symbolic_mode(0o000, "u+r").unwrap(), 0o400);
    }

    #[test]
    fn symbolic_group_minus_rwx() {
        assert_eq!(apply_symbolic_mode(0o777, "g-rwx").unwrap(), 0o707);
    }

    #[test]
    fn format_mode_file() {
        assert_eq!(format_mode(0o644, false), "-rw-r--r--");
        assert_eq!(format_mode(0o755, false), "-rwxr-xr-x");
        assert_eq!(format_mode(0o600, false), "-rw-------");
    }

    #[test]
    fn format_mode_dir() {
        assert_eq!(format_mode(0o755, true), "drwxr-xr-x");
        assert_eq!(format_mode(0o700, true), "drwx------");
    }

    #[test]
    fn format_mode_sticky() {
        assert_eq!(format_mode(0o1777, true), "drwxrwxrwt");
        assert_eq!(format_mode(0o1770, true), "drwxrwx--T");
    }

    #[test]
    fn sticky_delete_check() {
        let parent = NodeMeta {
            mode: 0o1777,
            uid: 0,
            gid: 0,
            mtime: 0,
        };
        let file = NodeMeta {
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            mtime: 0,
        };
        // File owner can delete
        assert!(check_delete_in_sticky(&parent, &file, 1000).is_ok());
        // Root can delete
        assert!(check_delete_in_sticky(&parent, &file, 0).is_ok());
        // Dir owner can delete
        assert!(check_delete_in_sticky(&parent, &file, 0).is_ok());
        // Other user cannot
        assert!(check_delete_in_sticky(&parent, &file, 2000).is_err());
    }
}
