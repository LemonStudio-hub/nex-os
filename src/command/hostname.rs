//! hostname command - display the system hostname

/// Execute the `hostname` command.
///
/// Usage: `hostname`
///
/// Returns the hostname of the virtual system.
pub fn execute(hostname: &str) -> Result<String, String> {
    Ok(format!("{}\n", hostname))
}
