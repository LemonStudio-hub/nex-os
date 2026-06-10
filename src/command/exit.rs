//! exit command - exit the terminal

pub struct ExitCommand;

impl super::Command for ExitCommand {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn description(&self) -> &'static str {
        "Exit the terminal"
    }

    fn execute(&self, _ctx: &mut super::CommandContext) -> Result<String, String> {
        Ok(String::new())
    }
}
