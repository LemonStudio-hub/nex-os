//! export command - set environment variables

use std::collections::HashMap;

/// Execute the `export` command.
///
/// Usage: `export KEY=VALUE`
///
/// Sets an environment variable. If no `=` is present, the variable is set
/// to an empty string.
pub fn execute(env_vars: &mut HashMap<String, String>, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        // No args: list all exported variables
        let mut output = String::new();
        let mut pairs: Vec<(&String, &String)> = env_vars.iter().collect();
        pairs.sort_by_key(|(k, _)| (*k).clone());
        for (key, value) in pairs {
            output.push_str(&format!("declare -x {}=\"{}\"\n", key, value));
        }
        return Ok(output);
    }

    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            let key = arg[..eq_pos].to_string();
            let value = arg[eq_pos + 1..].to_string();
            env_vars.insert(key, value);
        } else {
            env_vars.insert(arg.to_string(), String::new());
        }
    }

    Ok(String::new())
}
