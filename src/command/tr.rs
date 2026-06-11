//! `tr` - translate, squeeze, or delete characters from stdin
//!
//! Reads text from stdin and replaces each character found in `set1` with the
//! corresponding character at the same position in `set2`. If `set2` is shorter
//! than `set1`, the last character of `set2` is used for all remaining positions
//! (this matches POSIX `tr` behavior when sets differ in length).
//!
//! This command always reads from stdin -- it does not accept file arguments.
//! It is typically used at the end of a pipe (e.g., `echo hello | tr h H`).
//!
//! # Usage
//!
//! ```text
//! <input> | tr <set1> <set2>
//! ```
//!
//! # Escape Sequences
//!
//! Character sets support these backslash escapes:
//!
//! - `\n` -- newline
//! - `\t` -- tab
//! - `\\` -- literal backslash
//!
//! # Examples
//!
//! ```text
//! echo hello | tr h H          # "Hello"
//! echo "a b c" | tr " " "_"    # "a_b_c"
//! echo "abc" | tr a-z A-Z      # "ABC"
//! ```

/// Execute the `tr` command on the given stdin input.
///
/// Translates every character in `input` by looking it up in `set1` and
/// replacing it with the character at the same index in `set2`. Characters
/// not found in `set1` pass through unchanged.
///
/// # Arguments
///
/// * `input` -- The stdin text to translate.
/// * `args` -- Exactly two elements: `[set1, set2]`, each a string of
///   characters (with optional escape sequences).
///
/// # Returns
///
/// `Ok(String)` with the translated text, or `Err` if fewer than 2 args
/// are provided or `set1` is empty.
pub fn execute(input: &str, args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("tr: missing operand".to_string());
    }

    // Parse both character sets, expanding escape sequences into actual chars.
    let set1 = parse_char_set(args[0]);
    let set2 = parse_char_set(args[1]);

    if set1.is_empty() {
        return Err("tr: set1 must not be empty".to_string());
    }

    let mut output = String::new();
    for ch in input.chars() {
        if let Some(pos) = set1.iter().position(|&c| c == ch) {
            // Map to the corresponding character in set2. When set2 is shorter,
            // the last character of set2 acts as a catch-all -- this mirrors
            // POSIX tr behavior where trailing characters in set1 all map to
            // the final character of set2.
            let mapped = if pos < set2.len() {
                set2[pos]
            } else if !set2.is_empty() {
                set2[set2.len() - 1]
            } else {
                // set2 is empty (shouldn't happen given the validation above,
                // but be defensive) -- leave the character unchanged.
                ch
            };
            output.push(mapped);
        } else {
            // Character not in set1 -- pass it through unchanged.
            output.push(ch);
        }
    }

    Ok(output)
}

/// Parse a character set string into a vector of `char` values, expanding
/// backslash escape sequences along the way.
///
/// Recognized escapes: `\n` (newline), `\t` (tab), `\\` (literal backslash).
/// Any other `\X` sequence is kept as two separate characters `\` and `X`
/// to avoid silently losing information.
fn parse_char_set(s: &str) -> Vec<char> {
    let mut chars = Vec::new();
    let mut iter = s.chars().peekable();
    while let Some(ch) = iter.next() {
        if ch == '\\' {
            match iter.next() {
                Some('n') => chars.push('\n'),
                Some('t') => chars.push('\t'),
                Some('\\') => chars.push('\\'),
                Some(other) => {
                    // Unknown escape -- preserve both the backslash and the
                    // following character so the user can debug unexpected input.
                    chars.push('\\');
                    chars.push(other);
                }
                None => chars.push('\\'),
            }
        } else {
            chars.push(ch);
        }
    }
    chars
}

/// Command struct implementing the [`super::Command`] trait for `tr`.
pub struct TrCommand;

/// Trait implementation that wires `TrCommand` into the shell's command
/// registry. `tr` does not declare `accepts_stdin` because it accesses
/// stdin directly via `ctx.stdin` rather than through the implicit
/// stdin-to-argument injection mechanism.
impl super::Command for TrCommand {
    /// Returns the command name used for dispatch and tab completion.
    fn name(&self) -> &'static str { "tr" }

    /// Short description shown in `help` output.
    fn description(&self) -> &'static str { "Translate characters from stdin (echo text | tr a-z A-Z)" }

    /// Entry point called by the shell dispatcher. Passes the raw stdin
    /// string and args to the standalone [`execute`] function.
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.stdin, ctx.args)
    }
    fn synopsis(&self) -> &'static str { "echo text | tr set1 set2" }
    fn man_description(&self) -> &'static str {
        "Translate characters from stdin by replacing each character found in set1 with the \
character at the same position in set2. Characters not in set1 are passed through unchanged. \
If set2 is shorter than set1, the last character of set2 is used for all remaining positions."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["echo hello | tr a-z A-Z", "echo hello | tr eo OE"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_translation() {
        let out = execute("hello", &["h", "H"]).unwrap();
        assert_eq!(out, "Hello");
    }

    #[test]
    fn translate_multiple_chars() {
        let out = execute("abc", &["a", "x"]).unwrap();
        assert_eq!(out, "xbc");
    }

    #[test]
    fn translate_with_no_match() {
        // Characters not in set1 should pass through unchanged.
        let out = execute("hello", &["z", "X"]).unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn escape_newline_in_set() {
        // \n in set1 should match actual newline characters in the input.
        let out = execute("a\nb", &["\\n", "X"]).unwrap();
        assert_eq!(out, "aXb");
    }

    #[test]
    fn escape_tab_in_set() {
        let out = execute("a\tb", &["\\t", " "]).unwrap();
        assert_eq!(out, "a b");
    }

    #[test]
    fn escape_backslash_in_set() {
        // \\\\ in the Rust string literal produces \\, which parse_char_set
        // interprets as a single literal backslash character.
        let out = execute("a\\b", &["\\\\", "/"]).unwrap();
        assert_eq!(out, "a/b");
    }

    #[test]
    fn set1_longer_than_set2_maps_to_last() {
        // When set1 has more characters than set2, extra positions in set1
        // all map to the last character of set2.
        let out = execute("abcde", &["abcde", "XY"]).unwrap();
        assert_eq!(out, "XYYYY");
    }

    #[test]
    fn missing_args_returns_error() {
        // tr requires exactly 2 set arguments.
        assert!(execute("hello", &[]).is_err());
        assert!(execute("hello", &["a"]).is_err());
    }

    #[test]
    fn empty_set1_returns_error() {
        // set1 must contain at least one character to define the mapping.
        assert!(execute("hello", &["", "b"]).is_err());
    }

    #[test]
    fn empty_input_returns_empty() {
        // An empty input string should produce empty output regardless of sets.
        let out = execute("", &["a", "b"]).unwrap();
        assert_eq!(out, "");
    }
}
