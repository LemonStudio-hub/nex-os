//! pwd command - print working directory

use crate::vfs::Vfs;

pub fn execute(vfs: &Vfs) -> Result<String, String> {
    Ok(format!("{}\n", vfs.cwd))
}

pub struct PwdCommand;

impl super::Command for PwdCommand {
    fn name(&self) -> &'static str { "pwd" }
    fn description(&self) -> &'static str { "Print the current working directory" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs)
    }
}
