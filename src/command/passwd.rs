//! `passwd` -- change user password.
//!
//! # Usage
//!
//! ```text
//! passwd [username]
//! ```
//!
//! In this simulated environment, password management is handled by the
//! frontend (Argon2id hashing in TypeScript).  This command emits a
//! `password_change` action that the frontend intercepts.
//!
//! For now, the command simply acknowledges the request.

/// Execute the `passwd` command.
pub fn execute(username: &str, args: &[&str]) -> Result<String, String> {
    let target = if args.is_empty() { username } else { args[0] };

    // In a real implementation, this would emit a password_change action
    // that the frontend intercepts.  For now, just acknowledge.
    Ok(format!(
        "passwd: password for '{}' updated successfully\n",
        target
    ))
}

pub struct PasswdCommand;

impl super::Command for PasswdCommand {
    fn name(&self) -> &'static str {
        "passwd"
    }
    fn description(&self) -> &'static str {
        "Change user password"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.username, ctx.args).into()
    }
    fn synopsis(&self) -> &'static str {
        "passwd [username]"
    }
    fn man_description(&self) -> &'static str {
        "Change the password for the current or specified user. In this simulated environment, \
         the command acknowledges the request but actual password management is handled by the \
         browser frontend."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["passwd", "passwd alice"]
    }
}
