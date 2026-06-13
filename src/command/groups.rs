//! `groups` -- display group memberships.
//!
//! # Usage
//!
//! ```text
//! groups [username]
//! ```
//!
//! Displays the groups the current or specified user belongs to.
//!
//! # Examples
//!
//! ```text
//! $ groups
//! user
//! $ groups root
//! root
//! ```

/// Execute the `groups` command.
pub fn execute(
    username: &str,
    user_db: &crate::vfs::UserDatabase,
    args: &[&str],
) -> Result<String, String> {
    let target = if args.is_empty() { username } else { args[0] };

    // Verify user exists
    if user_db.find_user_by_name(target).is_none() {
        return Err(format!("groups: '{}': no such user", target));
    }

    let groups_str = user_db.format_groups(target);
    if groups_str.is_empty() {
        // Every user should at least be in their own group
        Ok(format!("{}\n", target))
    } else {
        Ok(format!("{}\n", groups_str))
    }
}

pub struct GroupsCommand;

impl super::Command for GroupsCommand {
    fn name(&self) -> &'static str {
        "groups"
    }
    fn description(&self) -> &'static str {
        "Display group memberships"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.username, &ctx.state.user_db, ctx.args).into()
    }
    fn synopsis(&self) -> &'static str {
        "groups [username]"
    }
    fn man_description(&self) -> &'static str {
        "Display the groups the current user or a specified user belongs to."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["groups", "groups root"]
    }
}
