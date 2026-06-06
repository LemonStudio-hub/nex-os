//! env command - display environment variables

use std::collections::HashMap;

/// Execute the `env` command.
///
/// Usage: `env`
///
/// Displays all environment variables as `KEY=VALUE` pairs, one per line.
pub fn execute(env_vars: &HashMap<String, String>) -> Result<String, String> {
    let mut output = String::new();
    let mut pairs: Vec<(&String, &String)> = env_vars.iter().collect();
    pairs.sort_by_key(|(k, _)| (*k).clone());
    for (key, value) in pairs {
        output.push_str(&format!("{}={}\n", key, value));
    }
    Ok(output)
}
