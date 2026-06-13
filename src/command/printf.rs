//! `printf` - format and print text
//!
//! Formats and prints text according to a format string. Supports common
//! printf-style format specifiers and escape sequences.
//!
//! # Usage
//!
//! ```text
//! printf FORMAT [ARGS...]
//! ```
//!
//! # Format Specifiers
//!
//! - `%s` -- String
//! - `%d` -- Integer (decimal)
//! - `%f` -- Floating point (6 decimal places)
//! - `%x` -- Integer (lowercase hex)
//! - `%o` -- Integer (octal)
//! - `%%` -- Literal `%`
//!
//! # Escape Sequences
//!
//! - `\n` -- Newline
//! - `\t` -- Tab
//! - `\\` -- Literal backslash
//!
//! # Examples
//!
//! ```text
//! printf '%s %d' hello 42
//! printf '%x' 255          # ff
//! printf 'hello\n'
//! ```

/// Execute the `printf` command.
///
/// Processes the format string character by character, expanding format
/// specifiers with the provided arguments and escape sequences.
///
/// # Arguments
///
/// * `args` - First element is the format string; remaining are values.
///
/// # Returns
///
/// `Ok(String)` with the formatted output, or `Err` if no format is given.
pub fn execute(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("printf: missing format operand".to_string());
    }

    let format = args[0];
    let values = &args[1..];
    let mut output = String::new();
    let mut chars = format.chars().peekable();
    let mut val_idx = 0;

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.next() {
                Some('n') => output.push('\n'),
                Some('t') => output.push('\t'),
                Some('\\') => output.push('\\'),
                Some(other) => {
                    output.push('\\');
                    output.push(other);
                }
                None => output.push('\\'),
            },
            '%' => match chars.next() {
                Some('%') => output.push('%'),
                Some('s') => {
                    let val = values.get(val_idx).unwrap_or(&"");
                    output.push_str(val);
                    val_idx += 1;
                }
                Some('d') => {
                    let val = values.get(val_idx).unwrap_or(&"0");
                    let n: i64 = val.parse().unwrap_or(0);
                    output.push_str(&format!("{}", n));
                    val_idx += 1;
                }
                Some('f') => {
                    let val = values.get(val_idx).unwrap_or(&"0");
                    let n: f64 = val.parse().unwrap_or(0.0);
                    output.push_str(&format!("{}", n));
                    val_idx += 1;
                }
                Some('x') => {
                    let val = values.get(val_idx).unwrap_or(&"0");
                    let n: i64 = val.parse().unwrap_or(0);
                    output.push_str(&format!("{:x}", n));
                    val_idx += 1;
                }
                Some('o') => {
                    let val = values.get(val_idx).unwrap_or(&"0");
                    let n: i64 = val.parse().unwrap_or(0);
                    output.push_str(&format!("{:o}", n));
                    val_idx += 1;
                }
                Some(other) => {
                    output.push('%');
                    output.push(other);
                }
                None => output.push('%'),
            },
            _ => output.push(ch),
        }
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `printf`.
pub struct PrintfCommand;

impl super::Command for PrintfCommand {
    fn name(&self) -> &'static str {
        "printf"
    }

    fn description(&self) -> &'static str {
        "Format and print text"
    }

    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(ctx.args).into()
    }

    fn synopsis(&self) -> &'static str {
        "printf FORMAT [ARGS...]"
    }

    fn man_description(&self) -> &'static str {
        "Format and print text according to FORMAT. Supports format specifiers: \
%s (string), %d (integer), %f (floating point), %x (hex), %o (octal), %% (literal percent). \
Supports escape sequences: \\n (newline), \\t (tab), \\\\ (backslash). Missing arguments use \
empty string for %s or zero for numeric specifiers."
    }

    fn examples(&self) -> &'static [&'static str] {
        &[
            "printf '%s %d' hello 42",
            "printf '%x' 255",
            "printf 'hello\\n'",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        let out = execute(&["hello"]).unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn string_specifier() {
        let out = execute(&["%s %s", "a", "b"]).unwrap();
        assert_eq!(out, "a b");
    }

    #[test]
    fn integer_specifier() {
        let out = execute(&["%d", "42"]).unwrap();
        assert_eq!(out, "42");
    }

    #[test]
    fn hex_specifier() {
        let out = execute(&["%x", "255"]).unwrap();
        assert_eq!(out, "ff");
    }

    #[test]
    fn octal_specifier() {
        let out = execute(&["%o", "8"]).unwrap();
        assert_eq!(out, "10");
    }

    #[test]
    fn literal_percent() {
        let out = execute(&["100%%"]).unwrap();
        assert_eq!(out, "100%");
    }

    #[test]
    fn escape_newline() {
        let out = execute(&["hello\\n"]).unwrap();
        assert_eq!(out, "hello\n");
    }

    #[test]
    fn escape_tab() {
        let out = execute(&["a\\tb"]).unwrap();
        assert_eq!(out, "a\tb");
    }

    #[test]
    fn escape_backslash() {
        let out = execute(&["a\\\\b"]).unwrap();
        assert_eq!(out, "a\\b");
    }

    #[test]
    fn missing_string_arg() {
        let out = execute(&["[%s]"]).unwrap();
        assert_eq!(out, "[]");
    }

    #[test]
    fn missing_int_arg() {
        let out = execute(&["[%d]"]).unwrap();
        assert_eq!(out, "[0]");
    }

    #[test]
    fn invalid_int_arg() {
        let out = execute(&["%d", "abc"]).unwrap();
        assert_eq!(out, "0");
    }

    #[test]
    fn no_args() {
        assert!(execute(&[]).is_err());
    }

    #[test]
    fn mixed_specifiers() {
        let out = execute(&["%s=%d", "count", "7"]).unwrap();
        assert_eq!(out, "count=7");
    }

    #[test]
    fn float_specifier() {
        let out = execute(&["%f", "3.14"]).unwrap();
        assert_eq!(out, "3.14");
    }
}
