//! chown command - change file ownership (simulated)

/// Execute the `chown` command.
///
/// Usage: `chown <owner>[:<group>] <file> [file2 ...]`
///
/// Simulates changing file ownership. Since the VFS has no real ownership
/// system, this validates the arguments and confirms the change.
pub fn execute(args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("chown: missing operand".to_string());
    }

    let owner = args[0];
    let _files = &args[1..];

    // Validate owner format: username or username:group
    if !owner.contains(':') && owner.is_empty() {
        return Err("chown: invalid user".to_string());
    }

    // In a full implementation we would store ownership in node metadata.
    // Here we just validate and confirm.
    Ok(String::new())
}

pub struct ChownCommand;

impl super::Command for ChownCommand {
    fn name(&self) -> &'static str { "chown" }
    fn description(&self) -> &'static str { "Change file ownership (owner[:group])" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_owner() {
        let out = execute(&["alice", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn valid_owner_with_group() {
        let out = execute(&["alice:staff", "/tmp/f.txt"]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn missing_operand() {
        assert!(execute(&[]).is_err());
        assert!(execute(&["alice"]).is_err());
    }

    #[test]
    fn empty_owner_errors() {
        assert!(execute(&["", "/tmp/f.txt"]).is_err());
    }
}
