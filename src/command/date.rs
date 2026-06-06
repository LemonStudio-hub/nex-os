//! date command - display the current date and time

/// Execute the `date` command.
///
/// Usage: `date`
///
/// Returns the current date and time in ISO-8601 format (UTC), derived from
/// the system clock at the time of execution.
pub fn execute() -> Result<String, String> {
    // Use js-sys to get the current date/time in the browser environment.
    // This avoids pulling in chrono or time crates.
    let now = js_sys::Date::new_0();
    let iso = now.to_iso_string().as_string().unwrap_or_default();
    Ok(format!("{}\n", iso))
}
