//! date command - display the current date and time

/// Execute the `date` command.
///
/// Usage: `date`
///
/// Returns the current date and time in ISO-8601 format (UTC), derived from
/// the system clock at the time of execution.
pub fn execute() -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let now = js_sys::Date::new_0();
        let iso = now.to_iso_string().as_string().unwrap_or_default();
        Ok(format!("{}\n", iso))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        // Format as a simple timestamp (seconds since epoch)
        Ok(format!("{}\n", secs))
    }
}
