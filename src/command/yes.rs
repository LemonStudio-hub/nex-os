//! `yes` - output a string repeatedly
//!
//! Repeatedly outputs a line with the specified STRING, or `y` by default.
//! Without `-n`, output is capped at 10000 lines (browser-safe limit).
//! With `-n COUNT`, outputs exactly COUNT lines.
//!
//! # Usage
//!
//! ```text
//! yes [-n COUNT] [STRING ...]
//! ```
//!
//! # Examples
//!
//! ```text
//! yes                    # y y y y ...
//! yes hello              # hello hello hello ...
//! yes -n 3 ok            # ok ok ok
//! ```

/// Default output cap to prevent browser hangs.
const DEFAULT_MAX_LINES: usize = 10_000;

/// Execute the `yes` command.
///
/// Generates repeated output of a string, optionally limited by `-n COUNT`.
///
/// # Arguments
///
/// * `args` - Optional `-n COUNT` flag and/or string tokens.
///
/// # Returns
///
/// `Ok(String)` with the repeated output, or `Err` for invalid arguments.
pub fn execute(args: &[&str]) -> Result<String, String> {
    let mut count: Option<usize> = None;
    let mut text_parts: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        if args[i] == "-n" {
            if i + 1 >= args.len() {
                return Err("yes: option requires an argument -- 'n'".to_string());
            }
            let n: usize = args[i + 1]
                .parse()
                .map_err(|_| format!("yes: invalid count: '{}'", args[i + 1]))?;
            count = Some(n);
            i += 2;
        } else {
            text_parts.push(args[i]);
            i += 1;
        }
    }

    let text = if text_parts.is_empty() {
        "y".to_string()
    } else {
        text_parts.join(" ")
    };

    let limit = count.unwrap_or(DEFAULT_MAX_LINES);
    let mut output = String::new();

    for _ in 0..limit {
        output.push_str(&text);
        output.push('\n');
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `yes`.
pub struct YesCommand;

impl super::Command for YesCommand {
    fn name(&self) -> &'static str {
        "yes"
    }

    fn description(&self) -> &'static str {
        "Output a string repeatedly"
    }

    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(ctx.args).into()
    }

    fn synopsis(&self) -> &'static str {
        "yes [-n COUNT] [STRING ...]"
    }

    fn man_description(&self) -> &'static str {
        "Repeatedly output a line with the specified STRING, or 'y' by default. \
Without -n, output is capped at 10000 lines (browser-safe limit). With -n COUNT, \
outputs exactly COUNT lines. Useful for piping into commands that need affirmative input."
    }

    fn examples(&self) -> &'static [&'static str] {
        &["yes", "yes hello", "yes -n 5 ok"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_y() {
        let out = execute(&["-n", "3"]).unwrap();
        assert_eq!(out, "y\ny\ny\n");
    }

    #[test]
    fn custom_string() {
        let out = execute(&["-n", "2", "hello"]).unwrap();
        assert_eq!(out, "hello\nhello\n");
    }

    #[test]
    fn multi_word_string() {
        let out = execute(&["-n", "1", "hello", "world"]).unwrap();
        assert_eq!(out, "hello world\n");
    }

    #[test]
    fn zero_count() {
        let out = execute(&["-n", "0"]).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn invalid_count() {
        assert!(execute(&["-n", "abc"]).is_err());
    }

    #[test]
    fn missing_n_value() {
        assert!(execute(&["-n"]).is_err());
    }

    #[test]
    fn without_n_flag_caps_at_default() {
        let out = execute(&[]).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), DEFAULT_MAX_LINES);
        assert!(lines.iter().all(|l| *l == "y"));
    }

    #[test]
    fn string_without_n() {
        let out = execute(&["ok"]).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), DEFAULT_MAX_LINES);
        assert!(lines.iter().all(|l| *l == "ok"));
    }
}
