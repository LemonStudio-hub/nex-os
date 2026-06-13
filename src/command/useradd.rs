//! `useradd` / `adduser` -- create a new user.
//!
//! # Usage
//!
//! ```text
//! useradd [-m] [-s shell] [-g group] username
//! adduser username
//! ```
//!
//! Creates a new user by adding entries to `/etc/passwd` and `/etc/group`.
//! With `-m`, creates a home directory at `/home/username`.
//!
//! # Examples
//!
//! ```text
//! $ useradd -m alice
//! $ adduser bob
//! ```

use crate::vfs::Vfs;

/// Execute the `useradd` command.
pub fn execute(
    vfs: &mut Vfs,
    args: &[&str],
    _uid: u32,
    _gid: u32,
    user_db: &crate::vfs::UserDatabase,
) -> Result<String, String> {
    let mut create_home = false;
    let mut shell = "/bin/nexsh";
    let mut group_name: Option<&str> = None;
    let mut username: Option<&str> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-m" => create_home = true,
            "-s" => {
                i += 1;
                if i < args.len() {
                    shell = args[i];
                }
            }
            "-g" => {
                i += 1;
                if i < args.len() {
                    group_name = Some(args[i]);
                }
            }
            _ => {
                if username.is_none() {
                    username = Some(args[i]);
                }
            }
        }
        i += 1;
    }

    let username = username.ok_or("useradd: missing username".to_string())?;

    // Validate username
    if username.is_empty() || username.len() > 32 {
        return Err("useradd: invalid username".to_string());
    }
    if username.contains(':') || username.contains('\n') {
        return Err("useradd: invalid username".to_string());
    }

    // Check if user already exists
    if user_db.find_user_by_name(username).is_some() {
        return Err(format!("useradd: user '{}' already exists", username));
    }

    // Allocate UID and GID
    let new_uid = user_db.next_uid();
    let new_gid = if let Some(gname) = group_name {
        match user_db.find_group_by_name(gname) {
            Some(g) => g.gid,
            None => return Err(format!("useradd: group '{}' does not exist", gname)),
        }
    } else {
        new_uid // Default: create a group with same name and GID = UID
    };

    // Append to /etc/passwd
    let passwd_line = format!(
        "{}:x:{}:{}:{}:/home/{}:{}\n",
        username, new_uid, new_gid, username, username, shell
    );
    let mut passwd_content = vfs.read_file("/etc/passwd").unwrap_or_default();
    passwd_content.push_str(&passwd_line);
    vfs.write_file("/etc/passwd", &passwd_content)?;
    vfs.mark_dirty("/etc/passwd");

    // Create group entry if no explicit group was specified
    if group_name.is_none() {
        let group_line = format!("{}:x:{}:{}\n", username, new_gid, username);
        let mut group_content = vfs.read_file("/etc/group").unwrap_or_default();
        group_content.push_str(&group_line);
        vfs.write_file("/etc/group", &group_content)?;
        vfs.mark_dirty("/etc/group");
    } else {
        // Add user to the specified group
        let group_content = vfs.read_file("/etc/group").unwrap_or_default();
        let mut new_group_content = String::new();
        let target_gid = new_gid;
        for line in group_content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                if let Ok(gid) = parts[2].parse::<u32>() {
                    if gid == target_gid {
                        // Add user to this group's member list
                        let mut members: Vec<&str> = if parts[3].is_empty() {
                            Vec::new()
                        } else {
                            parts[3].split(',').collect()
                        };
                        members.push(username);
                        new_group_content.push_str(&format!(
                            "{}:{}:{}:{}\n",
                            parts[0],
                            parts[1],
                            parts[2],
                            members.join(",")
                        ));
                        continue;
                    }
                }
            }
            new_group_content.push_str(line);
            new_group_content.push('\n');
        }
        vfs.write_file("/etc/group", &new_group_content)?;
        vfs.mark_dirty("/etc/group");
    }

    // Create home directory if -m was specified
    if create_home {
        let home_path = format!("/home/{}", username);
        if !vfs.exists(&home_path) {
            vfs.mkdir_with_owner(&home_path, new_uid, new_gid)?;
        }
    }

    Ok(String::new())
}

pub struct UseraddCommand;

impl super::Command for UseraddCommand {
    fn name(&self) -> &'static str {
        "useradd"
    }
    fn description(&self) -> &'static str {
        "Create a new user"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(
            &mut ctx.state.vfs,
            ctx.args,
            ctx.state.uid,
            ctx.state.gid,
            &ctx.state.user_db,
        )
        .into()
    }
    fn synopsis(&self) -> &'static str {
        "useradd [-m] [-s shell] [-g group] username"
    }
    fn man_description(&self) -> &'static str {
        "Create a new user. With -m, creates a home directory. With -s, sets the login shell. \
         With -g, assigns the user to an existing group."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["useradd -m alice", "adduser bob"]
    }
}

/// `adduser` -- higher-level alias for `useradd -m`.
pub struct AdduserCommand;

impl super::Command for AdduserCommand {
    fn name(&self) -> &'static str {
        "adduser"
    }
    fn description(&self) -> &'static str {
        "Create a new user (with home directory)"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        // adduser always creates a home directory
        let mut args_with_m = vec!["-m"];
        args_with_m.extend(ctx.args.iter().copied());
        execute(
            &mut ctx.state.vfs,
            &args_with_m,
            ctx.state.uid,
            ctx.state.gid,
            &ctx.state.user_db,
        )
        .into()
    }
    fn synopsis(&self) -> &'static str {
        "adduser username"
    }
    fn man_description(&self) -> &'static str {
        "Create a new user with a home directory. This is a convenience wrapper around useradd -m."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["adduser alice"]
    }
}
