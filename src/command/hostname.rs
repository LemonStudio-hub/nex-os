//! hostname command - display the system hostname

/// Execute the `hostname` command.
///
/// Usage: `hostname`
///
/// Returns the hostname of the virtual system.
pub fn execute(hostname: &str) -> Result<String, String> {
    Ok(format!("{}\n", hostname))
}

pub struct HostnameCommand;

impl super::Command for HostnameCommand {
    fn name(&self) -> &'static str { "hostname" }
    fn description(&self) -> &'static str { "Display the system hostname" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.hostname)
    }
}
