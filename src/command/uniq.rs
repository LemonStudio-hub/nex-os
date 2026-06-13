//! `uniq` - filter adjacent duplicate lines from a file
//!
//! Reads a file and outputs it with consecutive duplicate lines collapsed
//! into a single line. Only *adjacent* duplicates are removed -- identical
//! lines that are separated by a different line are both kept.
//!
//! The `-c` flag prefixes each output line with its occurrence count,
//! right-aligned in a 6-character field.
//!
//! # Usage
//!
//! ```text
//! uniq [-c] <file>
//! ```
//!
//! # Flags
//!
//! - `-c` -- Prepend each line with the number of times it appeared
//!   consecutively in the input.
//!
//! # Examples
//!
//! ```text
//! uniq /tmp/sorted.txt             # remove adjacent duplicates
//! uniq -c /tmp/access.log          # show occurrence counts
//! ```

use crate::vfs::Vfs;

/// Execute the `uniq` command against the virtual filesystem.
///
/// Reads the specified file, scans line by line, and emits each unique
/// adjacent run either once (default) or with a count prefix (`-c`).
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual filesystem to read from.
/// * `args` -- Slice of argument strings: optional `-c` flag followed by
///   exactly one file path.
///
/// # Returns
///
/// `Ok(String)` with the filtered output, or `Err` if the file operand
/// is missing or arguments are invalid.
pub fn execute(vfs: &Vfs, args: &[&str], host_fs: Option<&dyn crate::vfs::HostFs>) -> Result<String, String> {
    let mut show_count = false;
    let mut file_path: Option<&str> = None;

    // Parse flags and the single positional file argument.
    for arg in args {
        match *arg {
            "-c" => show_count = true,
            _ if file_path.is_none() => file_path = Some(arg),
            _ => return Err("uniq: too many arguments".to_string()),
        }
    }

    let path = file_path.ok_or("uniq: missing file operand")?;
    let resolved = vfs.resolve_path(path)?;
    let content = vfs.read_file_with_host(&resolved, host_fs)?;

    let mut output = String::new();
    let mut prev: Option<&str> = None;
    let mut count: usize = 0;

    // Scan lines one at a time, tracking the previous line. When a new
    // non-matching line is encountered, flush the previous run.
    for line in content.lines() {
        if Some(line) == prev {
            // Same as the previous line -- just increment the counter.
            count += 1;
        } else {
            // A different line (or the first line). Flush the previous run
            // before starting a new one.
            if let Some(p) = prev {
                if show_count {
                    output.push_str(&format!("{:>6} {}\n", count, p));
                } else {
                    output.push_str(&format!("{}\n", p));
                }
            }
            prev = Some(line);
            count = 1;
        }
    }

    // Flush the last group. The loop above only flushes when it sees a
    // *different* line, so the final run is still pending here.
    if let Some(p) = prev {
        if show_count {
            output.push_str(&format!("{:>6} {}\n", count, p));
        } else {
            output.push_str(&format!("{}\n", p));
        }
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `uniq`.
pub struct UniqCommand;

/// Trait implementation that wires `UniqCommand` into the shell's command
/// registry. Although `accepts_stdin` is true (the shell knows this command
/// can receive piped input), the actual `execute` implementation reads from
/// the VFS file rather than directly from `ctx.stdin`. The shell handles
/// injecting stdin content as a pseudo-file argument when needed.
impl super::Command for UniqCommand {
    /// Returns the command name used for dispatch and tab completion.
    fn name(&self) -> &'static str {
        "uniq"
    }

    /// Short description shown in `help` output.
    fn description(&self) -> &'static str {
        "Filter adjacent duplicate lines (-c for counts)"
    }

    /// Declares that this command can accept piped stdin. The shell uses this
    /// to route stdin content as a file argument when no explicit path is given.
    fn accepts_stdin(&self) -> bool {
        true
    }

    /// Entry point called by the shell dispatcher. Delegates to the
    /// standalone [`execute`] function with VFS and args from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }
    fn synopsis(&self) -> &'static str {
        "uniq [-c] file"
    }
    fn man_description(&self) -> &'static str {
        "Filter adjacent duplicate lines from a file, collapsing consecutive identical lines \
into a single line. Only truly adjacent duplicates are removed; identical lines separated by \
a different line are both kept. With -c, each output line is prefixed with its occurrence \
count, right-aligned in a 6-character field."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["uniq file.txt", "sort file.txt | uniq -c"]
    }
}
