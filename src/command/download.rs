//! `download` -- download a VFS file to the host machine.
//!
//! # Usage
//!
//! ```text
//! download <file_path>
//! ```
//!
//! Triggers a browser download of the specified file from the VFS to the
//! host machine.  The file must exist and must not be a directory.
//!
//! Like `mount`, this command returns a special action that the frontend
//! intercepts to trigger the browser's save/download UI.

use crate::command::{CommandContext, CommandOutput};

/// Execute the `download` command.
pub fn execute(ctx: &mut CommandContext) -> CommandOutput {
    if ctx.args.is_empty() {
        return CommandOutput::error("download", "missing file operand");
    }

    let path = match ctx.state.vfs.resolve_path(ctx.args[0]) {
        Ok(p) => p,
        Err(e) => return CommandOutput::error("download", &e),
    };

    // Verify the path exists.
    if !ctx
        .state
        .vfs
        .exists_with_host(&path, ctx.host_fs)
        .unwrap_or(false)
    {
        return CommandOutput::error(
            "download",
            &format!("{}: No such file or directory", ctx.args[0]),
        );
    }

    // Reject directories — only single files can be downloaded.
    if ctx
        .state
        .vfs
        .is_dir_with_host(&path, ctx.host_fs)
        .unwrap_or(false)
    {
        return CommandOutput::error("download", &format!("{}: Is a directory", ctx.args[0]));
    }

    // Extract the filename for the browser's save dialog suggestion.
    let filename = path
        .rsplit('/')
        .next()
        .unwrap_or("download")
        .to_string();

    // Return an action that the frontend intercepts to trigger the download.
    // The action format is: download_request:<filename>:<vfs_path>
    CommandOutput::download_request(filename, path)
}

/// Unit struct for command registration.
pub struct DownloadCommand;

impl super::Command for DownloadCommand {
    fn name(&self) -> &'static str {
        "download"
    }
    fn description(&self) -> &'static str {
        "Download a VFS file to the host machine"
    }
    fn execute(&self, ctx: &mut CommandContext) -> CommandOutput {
        execute(ctx)
    }
    fn synopsis(&self) -> &'static str {
        "download <file>"
    }
    fn man_description(&self) -> &'static str {
        "Download a file from the NexOS virtual filesystem to the host machine.\n\n\
Opens the browser's save dialog to let you choose where to save the file. \
If the File System Access API is not available, the file is downloaded to \
the browser's default download location.\n\n\
The file must exist and must not be a directory."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["download file.txt", "download /home/user/report.md"]
    }
}

#[cfg(test)]
mod tests {
    use crate::shell::{Service, ShellState};
    use crate::vfs::Vfs;

    #[test]
    fn download_existing_file_returns_action() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        state.vfs.write_file("/tmp/test.txt", "hello").unwrap();
        let output = service.execute_command(&mut state, "download /tmp/test.txt", None);
        assert!(output.action.is_some());
        let action = output.action.unwrap();
        assert!(action.starts_with("download_request:"));
        assert!(action.contains("test.txt"));
        assert!(action.contains("/tmp/test.txt"));
    }

    #[test]
    fn download_missing_operand_errors() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        let output = service.execute_command(&mut state, "download", None);
        assert_ne!(output.exit_code, 0);
        assert!(output.stderr.contains("missing file operand"));
    }

    #[test]
    fn download_nonexistent_file_errors() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        let output = service.execute_command(&mut state, "download /nope.txt", None);
        assert_ne!(output.exit_code, 0);
        assert!(output.stderr.contains("No such file or directory"));
    }

    #[test]
    fn download_directory_errors() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        let output = service.execute_command(&mut state, "download /home", None);
        assert_ne!(output.exit_code, 0);
        assert!(output.stderr.contains("Is a directory"));
    }

    #[test]
    fn download_preserves_filename() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        state.vfs.write_file("/a/b/report.md", "content").unwrap();
        let output = service.execute_command(&mut state, "download /a/b/report.md", None);
        let action = output.action.unwrap();
        assert!(action.starts_with("download_request:"));
        assert!(action.contains("report.md"));
    }
}
