//! basename command - strip directory and suffix from filenames

/// Execute the `basename` command.
///
/// Usage: `basename <path> [suffix]`
///
/// Print the last component of a path. If `suffix` is given and matches
/// the end of the component, it is removed.
pub fn execute(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("basename: missing operand".to_string());
    }

    let path = args[0];
    let suffix = if args.len() > 1 { Some(args[1]) } else { None };

    // Get the last component
    let trimmed = path.trim_end_matches('/');
    let name = match trimmed.rfind('/') {
        Some(i) => &trimmed[i + 1..],
        None => trimmed,
    };

    // Strip suffix if provided
    let result = if let Some(suf) = suffix {
        name.strip_suffix(suf).unwrap_or(name)
    } else {
        name
    };

    Ok(format!("{}\n", result))
}

pub struct BasenameCommand;

impl super::Command for BasenameCommand {
    fn name(&self) -> &'static str { "basename" }
    fn description(&self) -> &'static str { "Strip directory from filename" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_path() {
        let out = execute(&["/home/user/file.txt"]).unwrap();
        assert_eq!(out.trim(), "file.txt");
    }

    #[test]
    fn with_suffix() {
        let out = execute(&["/home/user/file.txt", ".txt"]).unwrap();
        assert_eq!(out.trim(), "file");
    }

    #[test]
    fn trailing_slashes() {
        let out = execute(&["/home/user/dir/"]).unwrap();
        assert_eq!(out.trim(), "dir");
    }

    #[test]
    fn single_component() {
        let out = execute(&["file.txt"]).unwrap();
        assert_eq!(out.trim(), "file.txt");
    }

    #[test]
    fn root_path() {
        let out = execute(&["/"]).unwrap();
        // root has no component name
        assert!(!out.is_empty());
    }

    #[test]
    fn suffix_that_doesnt_match() {
        let out = execute(&["/path/file.txt", ".log"]).unwrap();
        assert_eq!(out.trim(), "file.txt");
    }

    #[test]
    fn missing_operand() {
        assert!(execute(&[]).is_err());
    }
}
