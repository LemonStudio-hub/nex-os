//! `wc` - word, line, character, and byte count
//!
//! Counts lines, words, and characters in one or more files. By default all
//! three counts are displayed. Individual counts can be selected with flags.
//! When multiple files are given, a `total` summary line is appended.
//!
//! # Usage
//!
//! ```text
//! wc [-l] [-w] [-c] <file> [file2 ...]
//! ```
//!
//! # Flags
//!
//! - `-l` -- Show only the line count.
//! - `-w` -- Show only the word count.
//! - `-c` -- Show only the character count.
//!
//! If none of the flags are specified, all three counts are shown.
//!
//! # Output Format
//!
//! Each count is right-aligned in a 6-character field, separated by spaces,
//! followed by the file path. The optional `total` line uses the same format.
//!
//! # Examples
//!
//! ```text
//! wc file.txt                 # lines  words  chars  file.txt
//! wc -l file.txt              # lines only
//! wc -w -c file.txt           # words and chars
//! wc a.txt b.txt              # per-file lines + total
//! ```

use crate::vfs::Vfs;

/// Execute the `wc` command against the virtual filesystem.
///
/// Reads each specified file, computes the requested statistics, formats
/// them into aligned columns, and optionally appends a summary `total` line
/// when more than one file is given.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual filesystem to read from.
/// * `args` -- Slice of argument strings: optional `-l`, `-w`, `-c` flags
///   followed by one or more file paths.
///
/// # Returns
///
/// `Ok(String)` with formatted count output, or `Err` if no file operand
/// is given or a path cannot be resolved.
pub fn execute(vfs: &Vfs, args: &[&str], host_fs: Option<&dyn crate::vfs::HostFs>) -> Result<String, String> {
    let mut show_lines = false;
    let mut show_words = false;
    let mut show_chars = false;
    let mut files: Vec<&str> = Vec::new();

    // Separate flags from positional file arguments.
    for arg in args {
        match *arg {
            "-l" => show_lines = true,
            "-w" => show_words = true,
            "-c" => show_chars = true,
            _ => files.push(arg),
        }
    }

    // If no flags were specified, show all three statistics (POSIX default).
    if !show_lines && !show_words && !show_chars {
        show_lines = true;
        show_words = true;
        show_chars = true;
    }

    if files.is_empty() {
        return Err("wc: missing file operand".to_string());
    }

    let mut output = String::new();
    // Accumulators for the multi-file summary line.
    let mut total_lines: usize = 0;
    let mut total_words: usize = 0;
    let mut total_chars: usize = 0;

    for path in &files {
        let resolved = vfs.resolve_path(path)?;

        // Use file_line_count for efficient line counting from chunked
        // content.  Word and char counts still need the full content.
        let line_count = vfs.file_line_count_with_host(&resolved, host_fs)?;
        let content = vfs.read_file_with_host(&resolved, host_fs)?;
        let word_count = content.split_whitespace().count();
        let char_count = content.chars().count();

        total_lines += line_count;
        total_words += word_count;
        total_chars += char_count;

        // Build the output line dynamically based on which flags are active.
        let mut parts = Vec::new();
        if show_lines {
            parts.push(format!("{:>6}", line_count));
        }
        if show_words {
            parts.push(format!("{:>6}", word_count));
        }
        if show_chars {
            parts.push(format!("{:>6}", char_count));
        }
        output.push_str(&format!("{} {}\n", parts.join(" "), path));
    }

    // Only show the total line when there are multiple files, since a
    // single-file total would be redundant.
    if files.len() > 1 {
        let mut parts = Vec::new();
        if show_lines {
            parts.push(format!("{:>6}", total_lines));
        }
        if show_words {
            parts.push(format!("{:>6}", total_words));
        }
        if show_chars {
            parts.push(format!("{:>6}", total_chars));
        }
        output.push_str(&format!("{} total\n", parts.join(" ")));
    }

    Ok(output)
}

/// Command struct implementing the [`super::Command`] trait for `wc`.
pub struct WcCommand;

/// Trait implementation that wires `WcCommand` into the shell's command
/// registry. `accepts_stdin` is true so the shell can pipe data into wc
/// when no file argument is provided.
impl super::Command for WcCommand {
    /// Returns the command name used for dispatch and tab completion.
    fn name(&self) -> &'static str {
        "wc"
    }

    /// Short description shown in `help` output.
    fn description(&self) -> &'static str {
        "Count lines, words, and characters (-l -w -c)"
    }

    /// Declares that this command can accept piped stdin. The shell uses
    /// this to route stdin content as a file argument when no explicit
    /// path is given on the command line.
    fn accepts_stdin(&self) -> bool {
        true
    }

    /// Entry point called by the shell dispatcher. Delegates to the
    /// standalone [`execute`] function with VFS and args from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> super::CommandOutput {
        execute(&ctx.state.vfs, ctx.args, ctx.host_fs).into()
    }
    fn synopsis(&self) -> &'static str {
        "wc [-l] [-w] [-c] file [file2 ...]"
    }
    fn man_description(&self) -> &'static str {
        "Count lines, words, and characters in one or more files. By default all three counts \
are displayed. Use -l to show only lines, -w to show only words, or -c to show only \
characters. When multiple files are given, a summary total line is appended."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["wc file.txt", "wc -l *.txt"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a VFS with a single file containing the given content.
    fn vfs_with_content(content: &str) -> Vfs {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/f.txt", content).unwrap();
        vfs
    }

    #[test]
    fn default_shows_all_counts() {
        let vfs = vfs_with_content("hello world\nfoo bar");
        let out = execute(&vfs, &["/tmp/f.txt"], None).unwrap();
        // "hello world\nfoo bar" has 2 lines and 4 words.
        assert!(out.contains("2")); // 2 lines
        assert!(out.contains("4")); // 4 words
    }

    #[test]
    fn lines_only() {
        let vfs = vfs_with_content("a\nb\nc");
        let out = execute(&vfs, &["-l", "/tmp/f.txt"], None).unwrap();
        // Three newline-separated lines.
        assert!(out.contains("3"));
    }

    #[test]
    fn words_only() {
        let vfs = vfs_with_content("one two three");
        let out = execute(&vfs, &["-w", "/tmp/f.txt"], None).unwrap();
        // Three whitespace-separated words.
        assert!(out.contains("3"));
    }

    #[test]
    fn chars_only() {
        let vfs = vfs_with_content("abc");
        let out = execute(&vfs, &["-c", "/tmp/f.txt"], None).unwrap();
        // Three Unicode characters.
        assert!(out.contains("3"));
    }

    #[test]
    fn empty_file() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/empty.txt", "").unwrap();
        let out = execute(&vfs, &["/tmp/empty.txt"], None).unwrap();
        // An empty file should report zero for all counts.
        assert!(out.contains("0"));
    }

    #[test]
    fn multiple_files_shows_total() {
        let mut vfs = Vfs::new();
        vfs.write_file("/tmp/a.txt", "hello").unwrap();
        vfs.write_file("/tmp/b.txt", "world").unwrap();
        let out = execute(&vfs, &["/tmp/a.txt", "/tmp/b.txt"], None).unwrap();
        // Multiple files should produce a summary "total" line.
        assert!(out.contains("total"));
    }

    #[test]
    fn missing_file() {
        let vfs = Vfs::new();
        // No arguments at all should produce an error.
        assert!(execute(&vfs, &[], None).is_err());
    }
}
