//! touch command - create empty files

use crate::vfs::Vfs;

pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("touch: missing file operand".to_string());
    }

    for path in args {
        let resolved = vfs.resolve_path(path)?;
        vfs.touch(&resolved)?;
    }

    Ok(String::new())
}

pub struct TouchCommand;

impl super::Command for TouchCommand {
    fn name(&self) -> &'static str { "touch" }
    fn description(&self) -> &'static str { "Create empty files" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }
}
