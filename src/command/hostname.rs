//! `hostname` command -- display the system hostname.
//!
//! # Usage
//!
//! ```text
//! hostname
//! ```
//!
//! # Description
//!
//! Prints the hostname of the virtual NexOS system followed by a newline.
//! The hostname is configured during shell initialisation (either
//! user-provided or a default) and stored in the
//! [`Shell`](crate::shell::Shell) struct.
//!
//! # Examples
//!
//! ```text
//! $ hostname
//! mymachine
//! ```
//!
//! # Notes
//!
//! Unlike real systems, this command takes no flags.  It does not support
//! setting the hostname (`hostname newname`) or FQDN display (`hostname -f`).

/// Execute the `hostname` command, returning the hostname string.
///
/// # Arguments
///
/// * `hostname` -- The current hostname string from the shell state.
///
/// # Returns
///
/// Always returns `Ok` with the hostname followed by a newline.
pub fn execute(hostname: &str) -> Result<String, String> {
    Ok(format!("{}\n", hostname))
}

/// Unit struct representing the `hostname` command.
///
/// Registered in the command registry so the shell can dispatch
/// `hostname` input to this implementation.
pub struct HostnameCommand;

/// Bridges the registry's generic [`Command`](super::Command) interface
/// to the module-level `execute` function.
impl super::Command for HostnameCommand {
    /// The command name as typed by the user.
    fn name(&self) -> &'static str { "hostname" }

    /// One-line summary shown in `help` output.
    fn description(&self) -> &'static str { "Display the system hostname" }

    /// Execute the command, forwarding the hostname from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.hostname)
    }
    fn synopsis(&self) -> &'static str { "hostname" }
    fn man_description(&self) -> &'static str { "Display the hostname of the virtual NexOS system. The hostname is configured during shell initialization and stored in the shell state." }
    fn examples(&self) -> &'static [&'static str] { &[] }
}
