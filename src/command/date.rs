//! `date` -- display the current date and time.
//!
//! # Usage
//!
//! ```text
//! date
//! ```
//!
//! Prints the current date and time to stdout.  The output format depends on
//! the compilation target:
//!
//! - **WASM (`wasm32`)**: ISO-8601 string from `js_sys::Date` (e.g.
//!   `2026-06-11T14:30:00.000Z`).
//! - **Native (tests / CI)**: Seconds since the Unix epoch as a plain integer.
//!
//! The dual implementation exists because `std::time::SystemTime` is
//! unavailable or unreliable under `wasm32-unknown-unknown`, while `js_sys`
//! is not available in native builds.
//!
//! # Notes
//!
//! Takes no arguments and ignores all shell context.

/// Execute the `date` command.
///
/// Uses conditional compilation (`#[cfg]`) to select the appropriate clock
/// source for the current target architecture.
///
/// # Returns
///
/// A newline-terminated date/time string.
pub fn execute() -> Result<String, String> {
    // WASM target: use the browser's Date object via js_sys bindings.
    // This gives us a proper ISO-8601 timestamp including timezone info.
    #[cfg(target_arch = "wasm32")]
    {
        let now = js_sys::Date::new_0();
        let iso = now.to_iso_string().as_string().unwrap_or_default();
        Ok(format!("{}\n", iso))
    }

    // Native target (used in `cargo test` and CI): fall back to the standard
    // library's SystemTime.  We output raw epoch seconds because formatting a
    // human-readable string would require an external crate like `chrono`.
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        Ok(format!("{}\n", secs))
    }
}

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct DateCommand;

/// Delegates to the standalone [`execute`] function.  The `CommandContext` is
/// ignored entirely -- `date` needs no VFS, args, or other shell state.
impl super::Command for DateCommand {
    fn name(&self) -> &'static str {
        "date"
    }
    fn description(&self) -> &'static str {
        "Display the current date and time"
    }
    fn execute(&self, _ctx: &mut super::CommandContext) -> Result<String, String> {
        execute()
    }
    fn synopsis(&self) -> &'static str {
        "date"
    }
    fn man_description(&self) -> &'static str {
        "Display the current date and time in ISO-8601 format. On WASM targets, the output comes from the browser's Date object. On native targets, it falls back to seconds since the Unix epoch."
    }
    fn examples(&self) -> &'static [&'static str] {
        &[]
    }
}
