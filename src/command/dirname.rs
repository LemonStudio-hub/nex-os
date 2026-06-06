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
