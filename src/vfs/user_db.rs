//! User and group database parsed from VFS files.
//!
//! Provides [`UserDatabase`] which parses `/etc/passwd`, `/etc/group`, and
//! `/etc/sudoers` from the VFS into lookup-optimised structures.  The
//! database is cached in `ShellState` (skipped from serde) and rebuilt
//! whenever the source files change.

use super::tree::Vfs;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A parsed entry from `/etc/passwd`.
///
/// Format: `username:x:uid:gid:gecos:home_dir:shell`
#[derive(Debug, Clone)]
pub struct PasswdEntry {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
    pub _gecos: String,
    pub home_dir: String,
    pub shell: String,
}

/// A parsed entry from `/etc/group`.
///
/// Format: `groupname:x:gid:member1,member2,...`
#[derive(Debug, Clone)]
pub struct GroupEntry {
    pub groupname: String,
    pub gid: u32,
    pub members: Vec<String>,
}

/// A parsed entry from `/etc/sudoers`.
///
/// Format: `user ALL=(ALL) NOPASSWD: ALL` or `user ALL=(ALL) ALL`
#[derive(Debug, Clone)]
pub struct SudoersEntry {
    pub user: String,
    pub nopasswd: bool,
    pub commands: Vec<String>,
}

// ---------------------------------------------------------------------------
// UserDatabase
// ---------------------------------------------------------------------------

/// Cached, lookup-optimised user database parsed from VFS `/etc/` files.
///
/// The database is `#[serde(skip)]` in `ShellState` — it is rebuilt from
/// the VFS on every state deserialization.  This is fast (parsing a few
/// text lines) and avoids dual-write problems.
#[derive(Debug, Clone, Default)]
pub struct UserDatabase {
    pub passwd: Vec<PasswdEntry>,
    pub group: Vec<GroupEntry>,
    pub sudoers: Vec<SudoersEntry>,
}

impl UserDatabase {
    /// Parse the user database from VFS `/etc/` files.
    ///
    /// Silently returns empty entries for files that don't exist (e.g. on
    /// first boot before bootstrap creates them).
    pub fn from_vfs(vfs: &Vfs) -> Self {
        let passwd = vfs
            .read_file("/etc/passwd")
            .ok()
            .map(|content| parse_passwd(&content))
            .unwrap_or_default();

        let group = vfs
            .read_file("/etc/group")
            .ok()
            .map(|content| parse_group(&content))
            .unwrap_or_default();

        let sudoers = vfs
            .read_file("/etc/sudoers")
            .ok()
            .map(|content| parse_sudoers(&content))
            .unwrap_or_default();

        Self {
            passwd,
            group,
            sudoers,
        }
    }

    /// Find a user entry by numeric UID.
    pub fn find_user_by_uid(&self, uid: u32) -> Option<&PasswdEntry> {
        self.passwd.iter().find(|e| e.uid == uid)
    }

    /// Find a user entry by username.
    pub fn find_user_by_name(&self, name: &str) -> Option<&PasswdEntry> {
        self.passwd.iter().find(|e| e.username == name)
    }

    /// Find a group entry by numeric GID.
    pub fn find_group_by_gid(&self, gid: u32) -> Option<&GroupEntry> {
        self.group.iter().find(|e| e.gid == gid)
    }

    /// Find a group entry by group name.
    pub fn find_group_by_name(&self, name: &str) -> Option<&GroupEntry> {
        self.group.iter().find(|e| e.groupname == name)
    }

    /// Check whether a user has NOPASSWD sudo for the given command.
    pub fn has_nopasswd_sudo(&self, username: &str, command: &str) -> bool {
        self.sudoers.iter().any(|entry| {
            (entry.user == username || entry.user == "ALL")
                && entry.nopasswd
                && (entry.commands.contains(&"ALL".to_string())
                    || entry.commands.contains(&command.to_string()))
        })
    }

    /// Check whether a user has any sudo entry (with or without password).
    pub fn has_sudo_entry(&self, username: &str, command: &str) -> bool {
        self.sudoers.iter().any(|entry| {
            (entry.user == username || entry.user == "ALL")
                && (entry.commands.contains(&"ALL".to_string())
                    || entry.commands.contains(&command.to_string()))
        })
    }

    /// Get the next available UID (max existing uid + 1, starting from 1000).
    pub fn next_uid(&self) -> u32 {
        let max_uid = self.passwd.iter().map(|e| e.uid).max().unwrap_or(999);
        max_uid.max(999) + 1
    }

    /// Get the next available GID (max existing gid + 1, starting from 1000).
    pub fn next_gid(&self) -> u32 {
        let max_gid = self.group.iter().map(|e| e.gid).max().unwrap_or(999);
        max_gid.max(999) + 1
    }

    /// Get all groups a user belongs to (by username).
    pub fn user_groups(&self, username: &str) -> Vec<&GroupEntry> {
        self.group
            .iter()
            .filter(|g| g.members.contains(&username.to_string()))
            .collect()
    }

    /// Format the `groups` output for a user: space-separated group names.
    pub fn format_groups(&self, username: &str) -> String {
        let groups: Vec<&str> = self
            .group
            .iter()
            .filter(|g| g.members.contains(&username.to_string()))
            .map(|g| g.groupname.as_str())
            .collect();
        groups.join(" ")
    }

    /// Format the `id` output for a user: `uid=X(user) gid=X(group) groups=X(name),...`
    pub fn format_id(&self, username: &str) -> Option<String> {
        let entry = self.find_user_by_name(username)?;
        let uid = entry.uid;
        let gid = entry.gid;

        let group_name = self
            .find_group_by_gid(gid)
            .map(|g| g.groupname.as_str())
            .unwrap_or("unknown");

        let groups: Vec<String> = self
            .group
            .iter()
            .filter(|g| g.members.contains(&username.to_string()))
            .map(|g| format!("{}({})", g.gid, g.groupname))
            .collect();

        let groups_str = if groups.is_empty() {
            format!("{}({})", gid, group_name)
        } else {
            groups.join(",")
        };

        Some(format!(
            "uid={}({}) gid={}({}) groups={}",
            uid, username, gid, group_name, groups_str
        ))
    }
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

/// Parse `/etc/passwd` content into a list of entries.
///
/// Format per line: `username:x:uid:gid:gecos:home_dir:shell`
/// Lines starting with `#` or empty lines are skipped.
fn parse_passwd(content: &str) -> Vec<PasswdEntry> {
    content
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 7 {
                return None;
            }
            Some(PasswdEntry {
                username: parts[0].to_string(),
                uid: parts[2].parse().ok()?,
                gid: parts[3].parse().ok()?,
                _gecos: parts[4].to_string(),
                home_dir: parts[5].to_string(),
                shell: parts[6].to_string(),
            })
        })
        .collect()
}

/// Parse `/etc/group` content into a list of entries.
///
/// Format per line: `groupname:x:gid:member1,member2,...`
fn parse_group(content: &str) -> Vec<GroupEntry> {
    content
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 4 {
                return None;
            }
            let members = if parts[3].is_empty() {
                Vec::new()
            } else {
                parts[3].split(',').map(|s| s.to_string()).collect()
            };
            Some(GroupEntry {
                groupname: parts[0].to_string(),
                gid: parts[2].parse().ok()?,
                members,
            })
        })
        .collect()
}

/// Parse `/etc/sudoers` content into a list of entries.
///
/// Supports simplified format: `user ALL=(ALL) NOPASSWD: ALL` or `user ALL=(ALL) ALL`
fn parse_sudoers(content: &str) -> Vec<SudoersEntry> {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                return None;
            }
            let user = parts[0].to_string();
            let nopasswd = line.contains("NOPASSWD");
            // Everything after the (ALL) part is commands
            let commands = if nopasswd {
                // Find "NOPASSWD:" and take everything after it
                if let Some(idx) = line.find("NOPASSWD:") {
                    let after = &line[idx + "NOPASSWD:".len()..];
                    after
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                } else {
                    vec!["ALL".to_string()]
                }
            } else {
                // Take everything after the 3rd token (user, ALL=(ALL), commands...)
                if parts.len() > 3 {
                    parts[3..]
                        .iter()
                        .map(|s| s.trim_matches(',').to_string())
                        .collect()
                } else {
                    vec!["ALL".to_string()]
                }
            };
            Some(SudoersEntry {
                user,
                nopasswd,
                commands,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_passwd_basic() {
        let content =
            "root:x:0:0:root:/root:/bin/bash\nuser:x:1000:1000:user:/home/user:/bin/nexsh\n";
        let entries = parse_passwd(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].username, "root");
        assert_eq!(entries[0].uid, 0);
        assert_eq!(entries[1].username, "user");
        assert_eq!(entries[1].uid, 1000);
        assert_eq!(entries[1].home_dir, "/home/user");
    }

    #[test]
    fn parse_passwd_skips_comments() {
        let content = "# comment\nroot:x:0:0:root:/root:/bin/bash\n\n# another\n";
        let entries = parse_passwd(content);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn parse_group_basic() {
        let content = "root:x:0:\nuser:x:1000:alice,bob\n";
        let entries = parse_group(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].groupname, "root");
        assert_eq!(entries[0].members.len(), 0);
        assert_eq!(entries[1].groupname, "user");
        assert_eq!(entries[1].members, vec!["alice", "bob"]);
    }

    #[test]
    fn parse_sudoers_nopasswd() {
        let content = "user ALL=(ALL) NOPASSWD: ALL\n";
        let entries = parse_sudoers(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].user, "user");
        assert!(entries[0].nopasswd);
        assert_eq!(entries[0].commands, vec!["ALL"]);
    }

    #[test]
    fn parse_sudoers_with_password() {
        let content = "admin ALL=(ALL) ALL\n";
        let entries = parse_sudoers(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].user, "admin");
        assert!(!entries[0].nopasswd);
    }

    #[test]
    fn user_db_lookups() {
        let vfs = Vfs::new();
        // The default VFS doesn't have /etc files yet, so database is empty.
        let db = UserDatabase::from_vfs(&vfs);
        assert!(db.find_user_by_uid(0).is_none());
        assert!(db.next_uid() >= 1000);
    }

    #[test]
    fn user_db_format_id() {
        let db = UserDatabase {
            passwd: vec![PasswdEntry {
                username: "alice".to_string(),
                uid: 1000,
                gid: 1000,
                _gecos: String::new(),
                home_dir: "/home/alice".to_string(),
                shell: "/bin/nexsh".to_string(),
            }],
            group: vec![GroupEntry {
                groupname: "alice".to_string(),
                gid: 1000,
                members: vec!["alice".to_string()],
            }],
            sudoers: vec![],
        };
        let id_str = db.format_id("alice").unwrap();
        assert!(id_str.contains("uid=1000(alice)"));
        assert!(id_str.contains("gid=1000(alice)"));
    }
}
