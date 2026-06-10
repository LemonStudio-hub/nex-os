//! history command - display command history

/// Execute the `history` command.
///
/// Usage: `history`
///
/// Returns the list of previously executed commands, numbered sequentially.
pub fn execute(history: &[String]) -> Result<String, String> {
    let mut output = String::new();
    for (i, cmd) in history.iter().enumerate() {
        output.push_str(&format!("{:>5}  {}\n", i + 1, cmd));
    }
    Ok(output)
}

pub struct HistoryCommand;

impl super::Command for HistoryCommand {
    fn name(&self) -> &'static str { "history" }
    fn description(&self) -> &'static str { "Display command history" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.history)
    }
}
