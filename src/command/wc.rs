//! wc command - word, line, character, and byte count

use crate::vfs::Vfs;

/// Execute the `wc` command.
///
/// Usage: `wc [-l] [-w] [-c] <file> [file2 ...]`
///
/// By default displays lines, words, and characters. Flags restrict output:
/// - `-l` lines only
/// - `-w` words only
/// - `-c` characters only
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let mut show_lines = false;
    let mut show_words = false;
    let mut show_chars = false;
    let mut files: Vec<&str> = Vec::new();

    for arg in args {
        match *arg {
            "-l" => show_lines = true,
            "-w" => show_words = true,
            "-c" => show_chars = true,
            _ => files.push(arg),
        }
    }

    // If no flags specified, show all three
    if !show_lines && !show_words && !show_chars {
        show_lines = true;
        show_words = true;
        show_chars = true;
    }

    if files.is_empty() {
        return Err("wc: missing file operand".to_string());
    }

    let mut output = String::new();
    let mut total_lines: usize = 0;
    let mut total_words: usize = 0;
    let mut total_chars: usize = 0;

    for path in &files {
        let resolved = vfs.resolve_path(path)?;
        let content = vfs.read_file(&resolved)?;

        let line_count = content.lines().count();
        let word_count = content.split_whitespace().count();
        let char_count = content.chars().count();

        total_lines += line_count;
        total_words += word_count;
        total_chars += char_count;

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
