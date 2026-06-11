//! Pipeline parsing: pipe splitting and redirect extraction.
//!
//! This module provides two pure functions used by the shell during command
//! execution:
//!
//! - [`split_pipe_stages`] — splits a command string by top-level `|`,
//!   respecting single and double quotes.
//! - [`extract_redirect`] — extracts `>` / `>>` redirection operators from
//!   a single pipeline stage.

/// Split a command string by top-level `|` tokens, respecting quoted strings.
///
/// Pipe characters inside single (`'`) or double (`"`) quotes are treated as
/// literal characters and do **not** trigger a split.
///
/// # Examples
///
/// ```text
/// "cat file | grep hello | wc -l"  →  ["cat file", "grep hello", "wc -l"]
/// "echo 'a|b'"                     →  ["echo 'a|b'"]   (no split inside quotes)
/// ```
pub fn split_pipe_stages(input: &str) -> Vec<String> {
    let mut stages = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(chars[i]);
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(chars[i]);
            }
            '|' if !in_single_quote && !in_double_quote => {
                stages.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(chars[i]);
            }
        }
        i += 1;
    }

    let last = current.trim().to_string();
    if !last.is_empty() {
        stages.push(last);
    }

    // Ensure at least one stage
    if stages.is_empty() {
        stages.push(input.trim().to_string());
    }

    stages
}

/// Extract a `>` (overwrite) or `>>` (append) redirection from a single
/// pipeline stage.
///
/// Returns `(command_part, Some((target_file, is_append)))` when a redirect
/// is found, otherwise `(original, None)`.
///
/// # Parsing modes
///
/// The function tries two strategies in order:
///
/// 1. **Token-based** — handles `cmd > file` where the operator is a
///    separate whitespace-delimited token.
/// 2. **Character-based fallback** — handles `cmd>file` or `cmd>>file`
///    where the operator is glued to the surrounding text.
///
/// Quoted filenames have their surrounding quotes stripped.
pub fn extract_redirect(cmd: &str) -> (String, Option<(String, bool)>) {
    // First try token-based parsing (handles `cmd > file`)
    let tokens: Vec<&str> = cmd.split_whitespace().collect();
    for i in 0..tokens.len() {
        if tokens[i] == ">>" && i + 1 < tokens.len() {
            let cmd_part = tokens[..i].join(" ");
            let target = tokens[i + 1]
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();
            return (cmd_part, Some((target, true)));
        }
        if tokens[i] == ">" && i + 1 < tokens.len() {
            let cmd_part = tokens[..i].join(" ");
            let target = tokens[i + 1]
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();
            return (cmd_part, Some((target, false)));
        }
    }

    // Fallback: check for `cmd>>file` or `cmd>file` (no spaces)
    if let Some(idx) = cmd.find(">>") {
        let cmd_part = cmd[..idx].trim().to_string();
        let target = cmd[idx + 2..]
            .trim()
            .trim_matches('\'')
            .trim_matches('"')
            .to_string();
        if !cmd_part.is_empty() && !target.is_empty() {
            return (cmd_part, Some((target, true)));
        }
    } else if let Some(idx) = cmd.find('>') {
        let cmd_part = cmd[..idx].trim().to_string();
        let target = cmd[idx + 1..]
            .trim()
            .trim_matches('\'')
            .trim_matches('"')
            .to_string();
        if !cmd_part.is_empty() && !target.is_empty() {
            return (cmd_part, Some((target, false)));
        }
    }

    (cmd.to_string(), None)
}
