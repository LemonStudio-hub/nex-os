//! whoami command - display the current username

/// Execute the `whoami` command.
///
/// Usage: `whoami`
///
/// Returns the current username. The `username` is passed in from the Shell
/// so it reflects the logged-in user.
pub fn execute(username: &str) -> Result<String, String> {
    Ok(format!("{}\n", username))
}

pub struct WhoamiCommand;

impl super::Command for WhoamiCommand {
    fn name(&self) -> &'static str { "whoami" }
    fn description(&self) -> &'static str { "Display the current username" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.username)
    }
}
