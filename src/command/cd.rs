//! cd command - change directory

use crate::vfs::Vfs;

pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    let target = if args.is_empty() || args[0] == "~" {
        "/home/user".to_string()
    } else if args[0] == "/" {
        "/".to_string()
    } else {
        vfs.resolve_path(args[0])?
    };

    if !vfs.exists(&target) {
        let display = if args.is_empty() { "~" } else { args[0] };
        return Err(format!("cd: {}: No such file or directory", display));
    }

    if !vfs.is_dir(&target) {
        return Err(format!("cd: {}: Not a directory", args[0]));
    }

    vfs.cwd = target;
    Ok(String::new())
}

pub struct CdCommand;

impl super::Command for CdCommand {
    fn name(&self) -> &'static str { "cd" }
    fn description(&self) -> &'static str { "Change the current directory" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }
}
