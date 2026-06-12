//! `env` - display all environment variables
//!
//! Prints every environment variable currently defined in the shell session
//! as `KEY=VALUE` pairs, one per line. Variables are sorted alphabetically
//! for deterministic, readable output.
//!
//! # Usage
//!
//! ```text
//! env
//! ```
//!
//! # Notes
//!
//! This is a read-only command -- it does not modify the environment.
//! To set variables, use `export`. The environment is stored in the
//! shell's `env_vars` `HashMap` and persists across commands within a
//! session (and is serialized as part of VFS persistence).

use std::collections::HashMap;

/// Execute the `env` command.
///
/// Collects all key-value pairs from the environment map, sorts them
/// alphabetically by key, and formats each as `KEY=VALUE\n`.
///
/// # Arguments
///
/// * `env_vars` - Reference to the shell's environment variable map.
///
/// # Returns
///
/// A sorted, newline-separated list of `KEY=VALUE` pairs.
pub fn execute(env_vars: &HashMap<String, String>) -> Result<String, String> {
    let mut output = String::new();
    // Collect into a Vec so we can sort; HashMap iteration order is
    // non-deterministic, so sorting ensures stable output.
    let mut pairs: Vec<(&String, &String)> = env_vars.iter().collect();
    pairs.sort_by_key(|(k, _)| (*k).clone());
    for (key, value) in pairs {
        output.push_str(&format!("{}={}\n", key, value));
    }
    Ok(output)
}

/// Unit struct implementing the [`super::Command`] trait for `env`.
pub struct EnvCommand;

/// Registers `env` with the command system.
///
/// Passes a shared (immutable) reference to the environment map.
impl super::Command for EnvCommand {
    fn name(&self) -> &'static str { "env" }
    fn description(&self) -> &'static str { "Display environment variables" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        // Dereference the mutable borrow to get an immutable reference,
        // since `env` only reads the environment and never modifies it.
        execute(&ctx.state.env_vars)
    }
    fn synopsis(&self) -> &'static str { "env" }
    fn man_description(&self) -> &'static str { "Display all environment variables currently defined in the shell session as KEY=VALUE pairs, one per line. Variables are sorted alphabetically for deterministic output. This is a read-only command; use export to set variables." }
    fn examples(&self) -> &'static [&'static str] { &[] }
}
