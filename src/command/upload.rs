//! `upload` -- upload files from the host machine into the VFS.
//!
//! # Usage
//!
//! ```text
//! upload [destination_path]
//! ```
//!
//! Opens the browser's file picker to select one or more files from the host
//! machine, then copies them into the VFS at the specified destination
//! directory.  When no path is given, files are uploaded to the current
//! working directory.
//!
//! Like `mount`, this command returns a `CommandOutput` with an `upload_request`
//! action that the frontend intercepts to open the file picker and transfer
//! the file contents.

use crate::command::{CommandContext, CommandOutput};

/// Execute the `upload` command.
pub fn execute(ctx: &mut CommandContext) -> CommandOutput {
    let dest = if ctx.args.is_empty() {
        match ctx.state.vfs.resolve_path(".") {
            Ok(p) => p,
            Err(e) => return CommandOutput::error("upload", &e),
        }
    } else {
        match ctx.state.vfs.resolve_path(ctx.args[0]) {
            Ok(p) => p,
            Err(e) => return CommandOutput::error("upload", &e),
        }
    };

    // Verify the destination is an existing directory.
    if !ctx.state.vfs.exists(&dest) {
        return CommandOutput::error("upload", &format!("{}: No such file or directory", dest));
    }
    if !ctx.state.vfs.is_dir(&dest) {
        return CommandOutput::error("upload", &format!("{}: Not a directory", dest));
    }

    // Return an upload request action for the frontend.
    CommandOutput::upload_request(dest)
}

/// Unit struct for command registration.
pub struct UploadCommand;

impl super::Command for UploadCommand {
    fn name(&self) -> &'static str {
        "upload"
    }
    fn description(&self) -> &'static str {
        "Upload files from the host machine into the VFS"
    }
    fn execute(&self, ctx: &mut CommandContext) -> CommandOutput {
        execute(ctx)
    }
    fn synopsis(&self) -> &'static str {
        "upload [path]"
    }
    fn man_description(&self) -> &'static str {
        "Upload one or more files from the host machine into the NexOS virtual filesystem.\n\n\
Opens the browser's file picker for you to select files.  The selected files \
are copied into the destination directory in the VFS.\n\n\
If no destination path is given, files are uploaded to the current working directory.\n\
If a path is given, it must be an existing directory."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["upload", "upload /home/user/documents"]
    }
}

#[cfg(test)]
mod tests {
    use crate::shell::{Service, ShellState};
    use crate::vfs::Vfs;

    #[test]
    fn upload_current_dir_returns_action() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        let output = service.execute_command(&mut state, "upload", None);
        assert!(output.action.is_some());
        assert!(output.action.unwrap().starts_with("upload_request:"));
    }

    #[test]
    fn upload_specific_dir_returns_action() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        let output = service.execute_command(&mut state, "upload /home", None);
        let action = output.action.unwrap();
        assert!(action.starts_with("upload_request:"));
        assert!(action.contains("/home"));
    }

    #[test]
    fn upload_nonexistent_dir_errors() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        let output = service.execute_command(&mut state, "upload /nope", None);
        assert_ne!(output.exit_code, 0);
        assert!(output.stderr.contains("No such file or directory"));
    }

    #[test]
    fn upload_file_path_errors() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        state.vfs.write_file("/tmp/file.txt", "content").unwrap();
        let output = service.execute_command(&mut state, "upload /tmp/file.txt", None);
        assert_ne!(output.exit_code, 0);
        assert!(output.stderr.contains("Not a directory"));
    }
}
