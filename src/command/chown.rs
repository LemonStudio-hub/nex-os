//! `chown` -- change file ownership.
//!
//! # Usage
//!
//! ```text
//! chown <owner>[:<group>] <file> [file2 ...]
//! ```
//!
//! Changes the owner (and optionally group) of one or more files or
//! directories.  Only root can change file ownership.
//!
//! # Examples
//!
//! ```text
//! chown alice /tmp/file.txt
//! chown alice:staff /tmp/file.txt /tmp/other.txt
//! chown root:root /etc/config
//! ```
//!
//! # Errors
//!
//! - Fewer than two arguments (owner spec + at least one file).
//! - Empty owner string.
//! - Caller is not root.
//! - Owner or group name not found in `/etc/passwd` or `/etc/group`.

use crate::vfs::{HostFs, UserDatabase, Vfs};

/// Execute the `chown` command.
///
/// Parses the owner specification (`owner[:group]`), looks up numeric IDs
/// in the user database, then sets `meta.uid` and `meta.gid` on each target.
pub fn execute(
    vfs: &mut Vfs,
    args: &[&str],
    euid: u32,
    user_db: &UserDatabase,
    _host_fs: Option<&dyn HostFs>,
) -> Result<String, String> {
    if args.len() < 2 {
        return Err("chown: missing operand".to_string());
    }

    // Only root can change ownership
    if euid != 0 {
        return Err("chown: Operation not permitted".to_string());
    }

    let owner_spec = args[0];
    let files = &args[1..];

    // Parse owner[:group]
    let (owner_name, group_name) = if let Some((o, g)) = owner_spec.split_once(':') {
        (o, Some(g))
    } else {
        (owner_spec, None)
    };

    if owner_name.is_empty() {
        return Err("chown: invalid user".to_string());
    }

    // Resolve owner UID
    let new_uid = match user_db.find_user_by_name(owner_name) {
        Some(entry) => entry.uid,
        None => {
            // Try parsing as numeric UID
            match owner_name.parse::<u32>() {
                Ok(uid) => uid,
                Err(_) => return Err(format!("chown: invalid user: '{}'", owner_name)),
            }
        }
    };

    // Resolve group GID (if specified)
    let new_gid = if let Some(gname) = group_name {
        if gname.is_empty() {
            // "owner:" means keep current group — use a sentinel
            None
        } else {
            match user_db.find_group_by_name(gname) {
                Some(entry) => Some(entry.gid),
                None => {
                    // Try parsing as numeric GID
                    match gname.parse::<u32>() {
                        Ok(gid) => Some(gid),
                        Err(_) => return Err(format!("chown: invalid group: '{}'", gname)),
                    }
                }
            }
        }
    } else {
        None
    };

    let mut output = String::new();
    for path in files {
        let resolved = match vfs.resolve_path(path) {
            Ok(p) => p,
            Err(e) => {
                output.push_str(&format!("chown: cannot access '{}': {}\n", path, e));
                continue;
            }
        };

        // Check the node exists
        if vfs.get_meta(&resolved).is_none() {
            output.push_str(&format!(
                "chown: cannot access '{}': No such file or directory\n",
                path
            ));
            continue;
        }

        // Apply ownership change
        if let Some(meta) = vfs.get_meta_mut(&resolved) {
            meta.uid = new_uid;
            if let Some(gid) = new_gid {
                meta.gid = gid;
            }
            vfs.mark_dirty(&resolved);
        }
    }

    Ok(output)
}

/// Unit struct for command registration.
pub struct ChownCommand;

impl super::Command for ChownCommand {
    fn name(&self) -> &'static str {
        "chown"
    }
    fn description(&self) -> &'static str {
        "Change file ownership (owner[:group])"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(
            &mut ctx.state.vfs,
            ctx.args,
            ctx.state.euid,
            &ctx.state.user_db,
            ctx.host_fs,
        )
        .into()
    }
    fn synopsis(&self) -> &'static str {
        "chown owner[:group] file [file2 ...]"
    }
    fn man_description(&self) -> &'static str {
        "Change file ownership. Only root can change ownership. The owner can be a username or \
         numeric UID, optionally followed by :group or :GID."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["chown alice file.txt", "chown root:root /etc/config"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs::Vfs;

    #[test]
    fn chown_as_root_by_uid() {
        let mut vfs = Vfs::new();
        let user_db = UserDatabase::from_vfs(&vfs);
        vfs.touch("/tmp/f.txt").unwrap();
        // Root can always chown — use numeric UID since no /etc/passwd in test VFS
        let out = execute(&mut vfs, &["0", "/tmp/f.txt"], 0, &user_db, None).unwrap();
        assert!(out.is_empty());
        assert_eq!(vfs.get_meta("/tmp/f.txt").unwrap().uid, 0);
    }

    #[test]
    fn chown_denied_for_non_root() {
        let mut vfs = Vfs::new();
        let user_db = UserDatabase::from_vfs(&vfs);
        vfs.touch("/tmp/f.txt").unwrap();
        let err = execute(&mut vfs, &["0", "/tmp/f.txt"], 1000, &user_db, None).unwrap_err();
        assert!(err.contains("Operation not permitted"));
    }

    #[test]
    fn chown_nonexistent_file() {
        let mut vfs = Vfs::new();
        let user_db = UserDatabase::from_vfs(&vfs);
        let out = execute(&mut vfs, &["0", "/nonexistent"], 0, &user_db, None).unwrap();
        assert!(out.contains("No such file or directory"));
    }

    #[test]
    fn chown_missing_operand() {
        let mut vfs = Vfs::new();
        let user_db = UserDatabase::from_vfs(&vfs);
        assert!(execute(&mut vfs, &[], 0, &user_db, None).is_err());
    }

    #[test]
    fn chown_invalid_user() {
        let mut vfs = Vfs::new();
        let user_db = UserDatabase::from_vfs(&vfs);
        let err = execute(&mut vfs, &["nonexistent", "/tmp"], 0, &user_db, None).unwrap_err();
        assert!(err.contains("invalid user"));
    }
}
