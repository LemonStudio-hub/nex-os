//! dirname command - strip last component from a path

/// Execute the `dirname` command.
///
/// Usage: `dirname <path>`
///
/// Print the directory portion of a path. If the path contains no `/`,
/// outputs `.`.
pub fn execute(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("dirname: missing operand".to_string());
    }

    let path = args[0];

    // Remove trailing slashes
    let trimmed = path.trim_end_matches('/');

    match trimmed.rfind('/') {
        Some(0) => Ok("/\n".to_string()), // Path is like "/file"
        Some(i) => Ok(format!("{}\n", &trimmed[..i])),
        None => Ok(".\n".to_string()), // No directory component
    }
}

pub struct DirnameCommand;

impl super::Command for DirnameCommand {
    fn name(&self) -> &'static str { "dirname" }
    fn description(&self) -> &'static str { "Strip filename from path" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_path() {
        let out = execute(&["/home/user/file.txt"]).unwrap();
        assert_eq!(out.trim(), "/home/user");
    }

    #[test]
    fn no_slash_returns_dot() {
        let out = execute(&["file.txt"]).unwrap();
        assert_eq!(out.trim(), ".");
    }

    #[test]
    fn root_file() {
        let out = execute(&["/file.txt"]).unwrap();
        assert_eq!(out.trim(), "/");
    }

    #[test]
    fn trailing_slashes() {
        let out = execute(&["/home/user/dir/"]).unwrap();
        assert_eq!(out.trim(), "/home/user");
    }

    #[test]
    fn deeply_nested() {
        let out = execute(&["/a/b/c/d/file"]).unwrap();
        assert_eq!(out.trim(), "/a/b/c/d");
    }

    #[test]
    fn missing_operand() {
        assert!(execute(&[]).is_err());
    }
}
