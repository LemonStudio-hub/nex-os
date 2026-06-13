//! `id` -- display user identity information.
//!
//! # Usage
//!
//! ```text
//! id [username]
//! ```
//!
//! Displays the UID, GID, and group memberships for the current or specified user.
//!
//! # Examples
//!
//! ```text
//! $ id
//! uid=1000(user) gid=1000(user) groups=1000(user)
//! $ id root
//! uid=0(root) gid=0(root) groups=0(root)
//! ```

/// Execute the `id` command.
pub fn execute(
    username: &str,
    user_db: &crate::vfs::UserDatabase,
    args: &[&str],
) -> Result<String, String> {
    let target = if args.is_empty() { username } else { args[0] };

    match user_db.format_id(target) {
        Some(id_str) => Ok(format!("{}\n", id_str)),
        None => Err(format!("id: '{}': no such user", target)),
    }
}

pub struct IdCommand;

impl super::Command for IdCommand {
    fn name(&self) -> &'static str {
        "id"
    }
    fn description(&self) -> &'static str {
        "Display user identity (uid, gid, groups)"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.username, &ctx.state.user_db, ctx.args).into()
    }
    fn synopsis(&self) -> &'static str {
        "id [username]"
    }
    fn man_description(&self) -> &'static str {
        "Display the UID, GID, and group memberships for the current user or a specified user."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["id", "id root"]
    }
}
