//! clear command - clear the terminal screen

pub struct ClearCommand;

impl super::Command for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn description(&self) -> &'static str {
        "Clear the terminal screen"
    }

    fn execute(&self, _ctx: &mut super::CommandContext) -> Result<String, String> {
        Ok("\x1b[2J\x1b[H".to_string())
    }
}
