//! `touch` - create empty files or update existing file timestamps
//!
//! Creates one or more empty files in the virtual filesystem. If a file
//! already exists, its contents are left unchanged (the simulated timestamp
//! update is not meaningful in this VFS, so the effect is a no-op for
//! existing files).
//!
//! # Usage
//!
//! ```text
//! touch <file> [file2 ...]
//! ```
//!
//! # Flags
//!
//! None. This implementation does not support `-a` or `-t` flags.
//!
//! # Examples
//!
//! ```text
//! touch newfile.txt              # create a single empty file
//! touch a.txt b.txt c.txt        # create multiple files at once
//! ```

use crate::vfs::Vfs;

/// Execute the `touch` command against the virtual filesystem.
///
/// Iterates over all positional arguments, resolves each path, and calls
/// [`Vfs::touch`] which creates the file if it does not exist or is a
/// no-op if it already does.
///
/// # Arguments
///
/// * `vfs` -- Mutable reference to the virtual filesystem (files may be created).
/// * `args` -- Slice of file paths to touch.
///
/// # Returns
///
/// `Ok(String::new())` on success (touch produces no stdout), or `Err` if
/// no file operand is given or a path cannot be resolved.
pub fn execute(vfs: &mut Vfs, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("touch: missing file operand".to_string());
    }

    for path in args {
        let resolved = vfs.resolve_path(path)?;
        vfs.touch(&resolved)?;
    }

    // touch produces no output on success.
    Ok(String::new())
}

/// Command struct implementing the [`super::Command`] trait for `touch`.
pub struct TouchCommand;

/// Trait implementation that wires `TouchCommand` into the shell's command
/// registry. Since `touch` never reads from stdin, `accepts_stdin` defaults
/// to `false`.
impl super::Command for TouchCommand {
    /// Returns the command name used for dispatch and tab completion.
    fn name(&self) -> &'static str { "touch" }

    /// Short description shown in `help` output.
    fn description(&self) -> &'static str { "Create empty files" }

    /// Entry point called by the shell dispatcher. Delegates to the
    /// standalone [`execute`] function with VFS and args from the context.
    fn execute(&self, ctx: &mut super::CommandContext) -> Result<String, String> {
        execute(ctx.vfs, ctx.args)
    }

    fn synopsis(&self) -> &'static str { "touch file [file2 ...]" }
    fn man_description(&self) -> &'static str {
        "Create one or more empty files in the virtual filesystem. If a file already exists, its contents are left \
unchanged (the simulated timestamp update is a no-op for existing files). Multiple files can be created in a \
single invocation by listing them as separate arguments."
    }
    fn examples(&self) -> &'static [&'static str] { &["touch newfile.txt", "touch a.txt b.txt c.txt"] }
}
