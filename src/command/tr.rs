//! tr command - translate, squeeze, or delete characters

/// Execute the `tr` command.
///
/// Usage: `tr <set1> <set2>`
///
/// Reads from `input` (stdin) and translates characters in `set1` to
/// corresponding characters in `set2`. This implementation is called by the
/// shell with piped stdin.
///
/// Special constructs in set1/set2:
/// - Literal characters
/// - `\n` newline, `\t` tab, `\\` backslash
pub fn execute(input: &str, args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("tr: missing operand".to_string());
    }

    let set1 = parse_char_set(args[0]);
    let set2 = parse_char_set(args[1]);

    if set1.is_empty() {
        return Err("tr: set1 must not be empty".to_string());
    }

    let mut output = String::new();
    for ch in input.chars() {
        if let Some(pos) = set1.iter().position(|&c| c == ch) {
            // Map to corresponding character in set2, or last character of set2
            let mapped = if pos < set2.len() {
                set2[pos]
            } else if !set2.is_empty() {
                set2[set2.len() - 1]
            } else {
                ch
            };
            output.push(mapped);
        } else {
            output.push(ch);
        }
    }

    Ok(output)
}

/// Parse a character set string, handling escape sequences.
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
