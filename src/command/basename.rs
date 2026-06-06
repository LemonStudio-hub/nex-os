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
