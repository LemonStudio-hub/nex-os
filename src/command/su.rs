//! `su` -- switch user identity.
//!
//! # Usage
//!
//! ```text
//! su [-] [username]
//! ```
//!
//! Switches the effective user identity.  Without a username, switches to
//! root.  The `-` flag simulates a login shell (resets HOME, USER, SHELL).
//!
//! In this simulated environment, no password is required.
//!
//! # Examples
//!
//! ```text
//! $ su root
//! # whoami
//! root
//! $ su - alice
//! $ echo $HOME
//! /home/alice
//! ```

/// Execute the `su` command.
pub fn execute(state: &mut crate::shell::ShellState, args: &[&str]) -> Result<String, String> {
    let mut login_shell = false;
    let mut target_user: Option<&str> = None;

    for arg in args {
        if *arg == "-" {
            login_shell = true;
        } else if target_user.is_none() {
            target_user = Some(arg);
        }
    }

    let target = target_user.unwrap_or("root");

    // Look up the target user
    let entry = state
        .user_db
        .find_user_by_name(target)
        .ok_or_else(|| format!("su: user '{}' does not exist", target))?
        .clone();

    // Switch identity
    state.username = entry.username.clone();
    state.uid = entry.uid;
    state.gid = entry.gid;
    state.euid = entry.uid;

    // Update env vars
    state
        .env_vars
        .insert("USER".to_string(), entry.username.clone());

    if login_shell {
        state
            .env_vars
            .insert("HOME".to_string(), entry.home_dir.clone());
        state
            .env_vars
            .insert("SHELL".to_string(), entry.shell.clone());
        state.vfs.cwd = entry.home_dir.clone();
        state
            .env_vars
            .insert("PWD".to_string(), entry.home_dir.clone());
    }

    Ok(String::new())
}

pub struct SuCommand;

impl super::Command for SuCommand {
    fn name(&self) -> &'static str {
        "su"
    }
    fn description(&self) -> &'static str {
        "Switch user identity"
    }
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(ctx.state, ctx.args).into()
    }
    fn synopsis(&self) -> &'static str {
        "su [-] [username]"
    }
    fn man_description(&self) -> &'static str {
        "Switch user identity. Without a username, switches to root. The - flag resets HOME, \
         USER, SHELL and changes to the target user's home directory (login shell simulation). \
         No password is required in this simulated environment."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["su root", "su - alice", "su"]
    }
}
