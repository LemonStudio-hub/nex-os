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

pub struct ExportCommand;

impl super::Command for ExportCommand {
    fn name(&self) -> &'static str { "export" }
    fn description(&self) -> &'static str { "Set environment variables (export KEY=VALUE)" }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.env_vars, ctx.args)
    }
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
