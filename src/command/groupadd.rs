//! `groupadd` -- create a new group.
//!
//! # Usage
//!
//! ```text
//! groupadd [-g gid] groupname
//! ```
//!
//! Creates a new group by adding an entry to `/etc/group`.
//!
//! # Examples
//!
//! ```text
//! $ groupadd developers
//! $ groupadd -g 500 admins
//! ```

/// Execute the `groupadd` command.
pub fn execute(
    vfs: &mut crate::vfs::Vfs,
    args: &[&str],
    user_db: &crate::vfs::UserDatabase,
) -> Result<String, String> {
    let mut specified_gid: Option<u32> = None;
    let mut group_name: Option<&str> = None;

    let mut i = 0;
    while i < args.len() {
        if args[i] == "-g" {
            i += 1;
            if i < args.len() {
                specified_gid = Some(
                    args[i]
                        .parse::<u32>()
                        .map_err(|_| format!("groupadd: invalid GID: '{}'", args[i]))?,
                );
            }
        } else if group_name.is_none() {
            group_name = Some(args[i]);
        }
        i += 1;
    }

    let group_name = group_name.ok_or("groupadd: missing group name".to_string())?;

    // Validate group name
    if group_name.is_empty() || group_name.len() > 32 {
        return Err("groupadd: invalid group name".to_string());
    }
    if group_name.contains(':') || group_name.contains('\n') {
        return Err("groupadd: invalid group name".to_string());
    }

    // Check if group already exists
    if user_db.find_group_by_name(group_name).is_some() {
        return Err(format!("groupadd: group '{}' already exists", group_name));
    }

    let new_gid = specified_gid.unwrap_or_else(|| user_db.next_gid());

    // Check if GID is already in use
    if user_db.find_group_by_gid(new_gid).is_some() {
        return Err(format!("groupadd: GID '{}' already in use", new_gid));
    }

    // Append to /etc/group
    let group_line = format!("{}:x:{}:\n", group_name, new_gid);
    let mut group_content = vfs.read_file("/etc/group").unwrap_or_default();
    group_content.push_str(&group_line);
    vfs.write_file("/etc/group", &group_content)?;
    vfs.mark_dirty("/etc/group");

    Ok(String::new())
}

pub struct GroupaddCommand;

impl super::Command for GroupaddCommand {
    fn name(&self) -> &'static str {
        "groupadd"
    }
    fn description(&self) -> &'static str {
        "Create a new group"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&mut ctx.state.vfs, ctx.args, &ctx.state.user_db).into()
    }
    fn synopsis(&self) -> &'static str {
        "groupadd [-g gid] groupname"
    }
    fn man_description(&self) -> &'static str {
        "Create a new group by adding an entry to /etc/group. With -g, specifies the GID."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["groupadd developers", "groupadd -g 500 admins"]
    }
}
