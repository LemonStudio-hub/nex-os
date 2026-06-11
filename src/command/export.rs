//! `export` - set or display environment variables
//!
//! Adds variables to the shell's environment so they are available to
//! subsequent commands (and can be referenced via `$VAR` expansion).
//! When called without arguments, lists all currently exported variables
//! in `declare -x KEY="VALUE"` format, matching bash's output style.
//!
//! # Usage
//!
//! ```text
//! export                  # list all exported variables
//! export KEY=VALUE        # set KEY to VALUE
//! export KEY              # set KEY to empty string
//! export K1=V1 K2=V2     # set multiple variables at once
//! ```
//!
//! # Behavior
//!
//! - Values may contain `=` characters; only the *first* `=` is treated
//!   as the key-value separator (e.g., `export PATH=/a:/b` works correctly).
//! - Without `=`, the variable is set to an empty string (mimicking bash).
//! - The environment is stored in a `HashMap<String, String>` owned by
//!   the `Shell` struct and passed in as `&mut`.

use std::collections::HashMap;

/// Execute the `export` command.
///
/// If `args` is empty, prints all environment variables in sorted
/// `declare -x KEY="VALUE"` format. Otherwise, parses each argument
/// as a `KEY=VALUE` pair (or a bare key) and inserts it into the map.
///
/// # Arguments
///
/// * `env_vars` - Mutable reference to the shell's environment map.
/// * `args` - Command-line arguments: either empty (list mode) or
///   one or more `KEY=VALUE` assignments.
///
/// # Returns
///
/// The formatted variable listing (in list mode), or an empty string
/// after setting variables.
pub fn execute(env_vars: &mut HashMap<String, String>, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        // No args: list all exported variables in sorted order.
        // We sort alphabetically so the output is deterministic and easy
        // to scan, unlike raw HashMap iteration which has random order.
        let mut output = String::new();
        let mut pairs: Vec<(&String, &String)> = env_vars.iter().collect();
        pairs.sort_by_key(|(k, _)| (*k).clone());
        for (key, value) in pairs {
            output.push_str(&format!("declare -x {}=\"{}\"\n", key, value));
        }
        return Ok(output);
    }

    // Process each argument as a variable assignment.
    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            // Split on the FIRST '=' only. Everything after it is the value,
            // even if the value itself contains '=' characters.
            let key = arg[..eq_pos].to_string();
            let value = arg[eq_pos + 1..].to_string();
            env_vars.insert(key, value);
        } else {
            // Bare key with no '=' -- set to empty string, matching bash
            // behavior where `export FOO` declares FOO without a value.
            env_vars.insert(arg.to_string(), String::new());
        }
    }

    Ok(String::new())
}

/// Unit struct implementing the [`super::Command`] trait for `export`.
pub struct ExportCommand;

/// Registers `export` with the command system.
impl super::Command for ExportCommand {
    fn name(&self) -> &'static str { "export" }
    fn description(&self) -> &'static str { "Set environment variables (export KEY=VALUE)" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.env_vars, ctx.args)
    }
    fn synopsis(&self) -> &'static str { "export KEY=VALUE" }
    fn man_description(&self) -> &'static str { "Set environment variables in the shell session. When called without arguments, lists all exported variables in declare -x format. Values may contain = characters; only the first = is treated as the key-value separator. A bare key without = sets the variable to an empty string." }
    fn examples(&self) -> &'static [&'static str] { &["export PATH=/usr/bin", "export EDITOR=vim"] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn set_variable() {
        let mut env = HashMap::new();
        execute(&mut env, &["FOO=bar"]).unwrap();
        assert_eq!(env.get("FOO").unwrap(), "bar");
    }

    #[test]
    fn value_with_equals() {
        let mut env = HashMap::new();
        execute(&mut env, &["KEY=val=ue"]).unwrap();
        assert_eq!(env.get("KEY").unwrap(), "val=ue");
    }

    #[test]
    fn key_without_value() {
        let mut env = HashMap::new();
        execute(&mut env, &["EMPTY"]).unwrap();
        assert_eq!(env.get("EMPTY").unwrap(), "");
    }

    #[test]
    fn no_args_lists_all() {
        let mut env = HashMap::new();
        env.insert("A".to_string(), "1".to_string());
        env.insert("B".to_string(), "2".to_string());
        let out = execute(&mut env, &[]).unwrap();
        assert!(out.contains("A"));
        assert!(out.contains("B"));
        assert!(out.contains("declare -x"));
    }

    #[test]
    fn list_is_sorted() {
        let mut env = HashMap::new();
        env.insert("Z".to_string(), "1".to_string());
        env.insert("A".to_string(), "2".to_string());
        let out = execute(&mut env, &[]).unwrap();
        let pos_a = out.find("A").unwrap();
        let pos_z = out.find("Z").unwrap();
        assert!(pos_a < pos_z);
    }

    #[test]
    fn multiple_exports() {
        let mut env = HashMap::new();
        execute(&mut env, &["X=1", "Y=2"]).unwrap();
        assert_eq!(env.get("X").unwrap(), "1");
        assert_eq!(env.get("Y").unwrap(), "2");
    }
}
