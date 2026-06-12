//! `cut` -- extract selected fields from each line of a file.
//!
//! # Usage
//!
//! ```text
//! cut -f FIELDS [-d DELIM] [file]
//! ```
//!
//! Reads the file (or stdin via pipeline) line by line, splits each line into
//! fields using the delimiter, and prints only the requested fields.
//!
//! # Flags
//!
//! - `-f FIELDS` -- Comma-separated list of 1-indexed field numbers to extract
//!   (e.g. `-f 1,3`).  **Required.**
//! - `-d DELIM` -- Single-character field delimiter.  Defaults to tab (`\t`).
//!
//! # Examples
//!
//! ```text
//! cut -f 1,3 -d "," data.csv
//! echo "a:b:c" | cut -f 2 -d ":"
//! ```
//!
//! # Stdin
//!
//! Declares `accepts_stdin() = true` so the pipeline layer can feed preceding
//! output as a temporary file argument.
//!
//! # Notes
//!
//! Fields are 1-indexed to match POSIX `cut`.  Out-of-range field numbers are
//! silently skipped rather than producing errors.

use crate::vfs::Vfs;

/// Execute the `cut` command.
///
/// Parses flags manually (no external crate), reads the file, and for each
/// line splits on the delimiter and collects the requested fields.  Fields
/// that exceed the number of columns in a given line are silently omitted.
///
/// # Returns
///
/// The selected fields for each input line, re-joined with the delimiter.
///
/// # Errors
///
/// - Missing `-f` flag.
/// - Missing file operand.
/// - Unknown option flag.
/// - VFS resolution or read errors.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut fields: Vec<usize> = Vec::new();
    // Default delimiter is tab, matching POSIX `cut` behaviour.
    let mut delimiter = '\t';
    let mut file_path: Option<&str> = None;

    // Manual argument parsing loop -- each branch advances `i` by the number
    // of tokens it consumed (flag + value = 2, positional = 1).
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-f" if i + 1 < args.len() => {
                // Parse comma-separated field numbers.  Non-numeric tokens are
                // silently dropped via `filter_map` so "-f 1,abc" yields [1].
                fields = args[i + 1]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<usize>().ok())
                    .collect();
                i += 2;
            }
            "-d" if i + 1 < args.len() => {
                // Take only the first character of the delimiter argument.
                // If the argument is empty, fall back to tab.
                delimiter = args[i + 1].chars().next().unwrap_or('\t');
                i += 2;
            }
            _ if !args[i].starts_with('-') && file_path.is_none() => {
                // First non-flag argument is treated as the file path.
                file_path = Some(args[i]);
                i += 1;
            }
            _ => return Err(format!("cut: unknown option: {}", args[i])),
        }
    }

    if fields.is_empty() {
        return Err("cut: missing -f argument".to_string());
    }

    let path = file_path.ok_or("cut: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;
    let content = vfs.read_file(&resolved)?;

    let mut output = String::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split(delimiter).collect();
        // Select only the requested fields.  Fields are 1-indexed, so we
        // subtract 1 for the 0-indexed Vec.  Out-of-range indices are
        // filtered out rather than causing an error.
        let selected: Vec<&str> = fields
            .iter()
            .filter_map(|&f| {
                if f >= 1 && f <= parts.len() {
                    Some(parts[f - 1])
                } else {
                    None
                }
            })
            .collect();
        output.push_str(&format!("{}\n", selected.join(&delimiter.to_string())));
    }

    Ok(output)
}

/// Unit struct that implements the [`Command`](super::Command) trait for
/// registration in the command [`Registry`](super::Registry).
pub struct CutCommand;

/// Implements the `Command` trait.  `accepts_stdin` is `true` so that the
/// pipeline layer can feed stdin from a preceding stage as a temporary file
/// argument when no explicit file is given.
impl super::Command for CutCommand {
    fn name(&self) -> &'static str { "cut" }
    fn description(&self) -> &'static str { "Extract fields from each line (-f FIELDS -d DELIM)" }
    fn accepts_stdin(&self) -> bool { true }
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.vfs, ctx.args)
    }
    fn synopsis(&self) -> &'static str { "cut -f FIELDS [-d DELIM] file" }
    fn man_description(&self) -> &'static str {
        "Extract selected fields from each line of a file. The -f flag (required) specifies \
a comma-separated list of 1-indexed field numbers to extract. The -d flag sets the \
single-character field delimiter, which defaults to tab. Out-of-range field numbers are \
silently skipped."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["cut -f 1,3 data.csv", "cut -f 2 -d , data.csv"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs_with_content(content: &str) -> Vfs {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", content).unwrap();
        vfs
    }

    #[test]
    fn extract_single_field() {
        let vfs = vfs_with_content("a\tb\tc");
        let out = execute(&vfs, &["-f", "2", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("b"));
    }

    #[test]
    fn extract_multiple_fields() {
        let vfs = vfs_with_content("a,b,c\nd,e,f");
        let out = execute(&vfs, &["-f", "1,3", "-d", ",", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("a,c"));
        assert!(out.contains("d,f"));
    }

    #[test]
    fn out_of_range_field_skipped() {
        let vfs = vfs_with_content("a,b");
        let out = execute(&vfs, &["-f", "1,5", "-d", ",", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("a"));
        assert!(!out.contains("5"));
    }

    #[test]
    fn missing_f_flag() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["/tmp/f.txt"]).is_err());
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        assert!(execute(&vfs, &["-f", "1"]).is_err());
    }

    #[test]
    fn default_delimiter_is_tab() {
        let vfs = vfs_with_content("x\ty\tz");
        let out = execute(&vfs, &["-f", "1,3", "/tmp/f.txt"]).unwrap();
        assert!(out.contains("x"));
        assert!(out.contains("z"));
    }
}
