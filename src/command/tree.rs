//! `tree` - display directory structure as a visual tree
//!
//! Recursively lists the contents of a directory in a tree-like format,
//! using box-drawing characters (`├──`, `└──`, `│`) to show the hierarchy.
//! A summary line at the end reports the total number of directories and files.
//!
//! # Usage
//!
//! ```text
//! tree [directory]
//! ```
//!
//! When called without arguments, `tree` displays the tree rooted at the
//! current working directory (`.`). Entries within each directory are sorted
//! alphabetically.
//!
//! # Examples
//!
//! ```text
//! tree               # tree from current directory
//! tree /home/user    # tree from a specific directory
//! ```

use crate::vfs::{FsNode, Vfs};

/// Execute the `tree` command against the virtual filesystem.
///
/// Resolves the target path (defaulting to `.`), verifies it is a directory,
/// then recursively builds a tree representation. Each directory entry is
/// annotated with a trailing `/` to distinguish it from files.
///
/// # Arguments
///
/// * `vfs` -- Reference to the virtual filesystem to traverse.
/// * `args` -- Optional single argument: the root directory path. Defaults
///   to `.` (current working directory) when empty.
///
/// # Returns
///
/// `Ok(String)` containing the formatted tree with a summary line, or `Err`
/// if the path does not exist.
pub fn execute(vfs: &Vfs, args: &[&str]) -> Result<String, String> {
    let path = if args.is_empty() { "." } else { args[0] };
    let resolved = vfs.resolve_path(path)?;

    if !vfs.exists(&resolved) {
        return Err(format!("tree: '{}': No such file or directory", path));
    }

    // If the target is a regular file rather than a directory, just print
    // its name with a zero-count summary (matching the behavior of `tree`
    // on a non-directory argument).
    if !vfs.is_dir(&resolved) {
        return Ok(format!("{}\n0 directories, 0 files\n", path));
    }

    let mut dir_count: usize = 0;
    let mut file_count: usize = 0;
    let mut output = String::new();

    // Print the root node name. For "/" just print the slash; otherwise
    // extract the last path component.
    if resolved == "/" {
        output.push('/');
    } else {
        let root_name = resolved
            .rfind('/')
            .map(|i| &resolved[i + 1..])
            .unwrap_or(&resolved);
        output.push_str(root_name);
    }
    output.push('\n');

    // Recursively build the tree from the root directory.
    build_tree(
        vfs,
        &resolved,
        "",         // no prefix for the root level
        &mut output,
        &mut dir_count,
        &mut file_count,
    );

    output.push_str(&format!(
        "\n{} directories, {} files\n",
        dir_count, file_count
    ));
    Ok(output)
}

/// Recursively build the tree output for a single directory level.
///
/// For each entry in the directory, this function:
/// 1. Draws the appropriate connector (`├── ` for non-last entries, `└── `
///    for the last entry in each group).
/// 2. Appends a `/` suffix to directory names.
/// 3. Recurses into subdirectories with an updated prefix that carries the
///    vertical line (`│   `) or blank (`    `) for proper alignment.
///
/// # Arguments
///
/// * `vfs` -- The virtual filesystem to read directory listings from.
/// * `path` -- Absolute path of the directory being listed.
/// * `prefix` -- String prefix prepended to each line for indentation
///   (carries the vertical bars from parent levels).
/// * `output` -- Mutable string accumulator for the formatted output.
/// * `dir_count` -- Running total of directories encountered.
/// * `file_count` -- Running total of files encountered.
fn build_tree(
    vfs: &Vfs,
    path: &str,
    prefix: &str,
    output: &mut String,
    dir_count: &mut usize,
    file_count: &mut usize,
) {
    let entries: Vec<FsNode> = match vfs.list_dir(path) {
        Ok(e) => e,
        // If we can't list the directory (e.g., permission issue), silently
        // skip it rather than aborting the entire tree.
        Err(_) => return,
    };

    // Sort entries alphabetically so the output is deterministic and matches
    // the behavior of the standard `tree` command.
    let mut sorted = entries;
    sorted.sort_by(|a, b| a.name().cmp(b.name()));

    for (i, entry) in sorted.iter().enumerate() {
        // Determine if this is the last entry at this level, which changes
        // the connector from a T-junction (├──) to an L-corner (└──).
        let is_last = i == sorted.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        // The prefix for children differs based on whether this entry is the
        // last: last entries use blank padding, others keep the vertical bar.
        let next_prefix = if is_last { "    " } else { "│   " };

        let suffix = if entry.is_dir() { "/" } else { "" };
        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(entry.name());
        output.push_str(suffix);
        output.push('\n');

        if entry.is_dir() {
            *dir_count += 1;

            // Build the child path and recurse. The child_prefix concatenates
            // the current prefix with the connector line so nested entries
            // maintain proper vertical alignment.
            let child_path = Vfs::child_path(path, entry.name());
            let child_prefix = format!("{}{}", prefix, next_prefix);
            build_tree(
                vfs,
                &child_path,
                &child_prefix,
                output,
                dir_count,
                file_count,
            );
        } else {
            *file_count += 1;
        }
    }
}

/// Command struct implementing the [`super::Command`] trait for `tree`.
pub struct TreeCommand;

/// Trait implementation that wires `TreeCommand` into the shell's command
/// registry. Tree is a read-only traversal command that does not accept
/// piped input.
impl super::Command for TreeCommand {
    /// Returns the command name used for dispatch and tab completion.
    fn name(&self) -> &'static str { "tree" }

    /// Short description shown in `help` output.
    fn description(&self) -> &'static str { "Display directory tree structure" }

    /// Entry point called by the shell dispatcher. Delegates to the
    /// standalone [`execute`] function with VFS and args from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(&ctx.state.vfs, ctx.args)
    }

    fn synopsis(&self) -> &'static str { "tree [path]" }
    fn man_description(&self) -> &'static str {
        "Recursively display the contents of a directory in a tree-like format using box-drawing characters. \
When called without arguments, displays the tree rooted at the current working directory. \
Entries within each directory are sorted alphabetically. A summary line at the end reports the total number of directories and files."
    }
    fn examples(&self) -> &'static [&'static str] { &["tree", "tree /home"] }
}
