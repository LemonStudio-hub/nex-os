//! history command - display command history

/// Execute the `history` command.
///
/// Usage: `history`
///
/// Returns the list of previously executed commands, numbered sequentially.
pub fn execute(history: &[String]) -> Result<String, String> {
    let mut output = String::new();
    for (i, cmd) in history.iter().enumerate() {
        output.push_str(&format!("{:>5}  {}\n", i + 1, cmd));
    }
    Ok(output)
}
