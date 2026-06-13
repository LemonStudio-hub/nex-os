//! `sudo` -- execute a command with elevated privileges.
//!
//! # Usage
//!
//! ```text
//! sudo [-u user] command [args...]
//! ```
//!
//! Temporarily elevates the effective UID to 0 (root) or to the specified
//! `-u user` for one command execution.  Checks `/etc/sudoers` for
//! authorization.
//!
//! v1 limitation: only NOPASSWD sudoers entries are supported.  If a
//! password is required, the command returns an error.
//!
//! # Examples
//!
//! ```text
//! $ sudo whoami
//! root
//! $ sudo -u alice whoami
//! alice
//! ```

/// Execute the `sudo` command.
///
/// Parses flags, checks sudoers, temporarily elevates euid, dispatches
/// the inner command via the registry, then restores euid.
pub fn execute(ctx: &mut super::CommandContext) -> super::CommandOutput {
    let mut target_user: Option<&str> = None;
    let mut cmd_start = 0;

    // Parse -u flag
    let mut i = 0;
    while i < ctx.args.len() {
        if ctx.args[i] == "-u" {
            i += 1;
            if i < ctx.args.len() {
                target_user = Some(ctx.args[i]);
            } else {
                return super::CommandOutput::error("sudo", "option '-u' requires an argument");
            }
        } else {
            cmd_start = i;
            break;
        }
        i += 1;
    }

    if cmd_start >= ctx.args.len() {
        return super::CommandOutput::error("sudo", "missing command");
    }

    let cmd_name = ctx.args[cmd_start];
    let cmd_args = &ctx.args[cmd_start + 1..];

    // Determine target UID
    let target_uid = if let Some(user) = target_user {
        match ctx.state.user_db.find_user_by_name(user) {
            Some(entry) => entry.uid,
            None => {
                return super::CommandOutput::error("sudo", &format!("unknown user: '{}'", user));
            }
        }
    } else {
        0 // Default: root
    };

    // Check sudoers authorization
    let username = ctx.state.username.clone();
    if !ctx.state.user_db.has_nopasswd_sudo(&username, cmd_name) {
        // Check if any sudo entry exists (password required)
        if ctx.state.user_db.has_sudo_entry(&username, cmd_name) {
            return super::CommandOutput::error(
                "sudo",
                "password required (NOPASSWD sudoers entry not found)",
            );
        }
        return super::CommandOutput::error(
            "sudo",
            &format!("{} is not in the sudoers file", username),
        );
    }

    // Save old euid and elevate
    let old_euid = ctx.state.euid;
    ctx.state.euid = target_uid;

    // Look up and execute the inner command
    let result = match ctx.registry.get(cmd_name) {
        Some(cmd) => {
            let mut inner_ctx = super::CommandContext {
                state: ctx.state,
                stdin: ctx.stdin,
                args: cmd_args,
                registry: ctx.registry,
                host_fs: ctx.host_fs,
            };
            cmd.execute(&mut inner_ctx)
        }
        None => {
            ctx.state.euid = old_euid;
            return super::CommandOutput::error(
                "sudo",
                &format!("command not found: {}", cmd_name),
            );
        }
    };

    // Restore euid
    ctx.state.euid = old_euid;

    result
}

pub struct SudoCommand;

impl super::Command for SudoCommand {
    fn name(&self) -> &'static str {
        "sudo"
    }
    fn description(&self) -> &'static str {
        "Execute a command with elevated privileges"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(ctx)
    }
    fn synopsis(&self) -> &'static str {
        "sudo [-u user] command [args...]"
    }
    fn man_description(&self) -> &'static str {
        "Execute a command with elevated privileges (euid 0 by default). Checks /etc/sudoers \
         for NOPASSWD authorization. Only NOPASSWD entries are supported in v1."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["sudo whoami", "sudo -u alice id", "sudo cat /etc/shadow"]
    }
}
