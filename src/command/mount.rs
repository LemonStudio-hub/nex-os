//! `mount` -- mount a host directory into the VFS.
//!
//! # Usage
//!
//! ```text
//! mount                  — list current mounts
//! mount <vfs_path>       — request mounting a host directory at <vfs_path>
//! mount -u <vfs_path>    — unmount (remove mount point)
//! ```
//!
//! When called with a path, the command creates the mount point in the VFS
//! metadata and returns a `CommandOutput` with a `mount_request` action that
//! the frontend intercepts to open the browser's directory picker.
//!
//! The actual `FileSystemDirectoryHandle` lives on the JavaScript side and
//! must be registered with WASM via `register_host_fs` after the user selects
//! a directory.

use crate::command::{CommandContext, CommandOutput};

/// Execute the `mount` command.
pub fn execute(ctx: &mut CommandContext) -> CommandOutput {
    let args = ctx.args;

    if args.is_empty() {
        // List current mounts
        let mounts = ctx.state.vfs.list_mounts();
        if mounts.is_empty() {
            return CommandOutput::success(
                "No directories mounted.\nUsage: mount <path>\n       mount -u <path>\n"
                    .to_string(),
            );
        }
        let mut output = String::from("Mounted directories:\n");
        for (vfs_path, host_path) in mounts {
            output.push_str(&format!("  {} -> {}\n", vfs_path, host_path));
        }
        return CommandOutput::success(output);
    }

    // Unmount
    if args[0] == "-u" || args[0] == "--unmount" {
        if args.len() < 2 {
            return CommandOutput::error("mount", "missing path for -u");
        }
        let path = match ctx.state.vfs.resolve_path(args[1]) {
            Ok(p) => p,
            Err(e) => return CommandOutput::error("mount", &e),
        };
        if ctx.state.vfs.remove_mount(&path) {
            CommandOutput::success(format!("Unmounted {}\n", path))
        } else {
            CommandOutput::error("mount", &format!("{} is not mounted", path))
        }
    } else {
        // Mount request
        let path = match ctx.state.vfs.resolve_path(args[0]) {
            Ok(p) => p,
            Err(e) => return CommandOutput::error("mount", &e),
        };
        // Create the mount point directory in VFS if it doesn't exist.
        // Use mkdir -p semantics: create intermediate directories as needed.
        if !ctx.state.vfs.exists(&path) {
            let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            let mut current = String::new();
            for comp in &components {
                current = if current.is_empty() {
                    format!("/{}", comp)
                } else {
                    format!("{}/{}", current, comp)
                };
                if !ctx.state.vfs.exists(&current) {
                    if let Err(e) = ctx.state.vfs.mkdir(&current) {
                        return CommandOutput::error("mount", &e);
                    }
                }
            }
        }
        // Register a placeholder mount entry so the frontend knows the path
        ctx.state
            .vfs
            .add_mount(path.clone(), String::new());
        // Return a mount request action for the frontend to intercept
        CommandOutput::mount_request(path)
    }
}

/// Unit struct for command registration.
pub struct MountCommand;

impl super::Command for MountCommand {
    fn name(&self) -> &'static str {
        "mount"
    }
    fn description(&self) -> &'static str {
        "Mount a host directory into the virtual filesystem"
    }
    fn execute(&self, ctx: &mut CommandContext) -> CommandOutput {
        execute(ctx)
    }
    fn synopsis(&self) -> &'static str {
        "mount [-u] [path]"
    }
    fn man_description(&self) -> &'static str {
        "Mount a real directory from the host machine into the NexOS virtual filesystem.\n\n\
When called without arguments, lists all current mounts.\n\
When called with a path, requests mounting a host directory at that VFS path.\n\
The browser will open a directory picker for you to select the directory.\n\n\
Use -u to unmount a previously mounted directory."
    }
    fn examples(&self) -> &'static [&'static str] {
        &["mount /mnt/project", "mount -u /mnt/project"]
    }
}

#[cfg(test)]
mod tests {
    use crate::shell::{Service, ShellState};
    use crate::vfs::Vfs;

    #[test]
    fn mount_list_empty() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        let output = service.execute_command(&mut state, "mount", None);
        assert!(output.stdout.contains("No directories mounted"));
    }

    #[test]
    fn mount_request_returns_marker() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        let output = service.execute_command(&mut state, "mount /mnt/test", None);
        // mount returns a mount_request action for the frontend to intercept
        assert!(output.action.is_some());
        assert!(output.action.unwrap().contains("mount_request:/mnt/test"));
        // The mount point directory should have been created
        assert!(state.vfs.exists("/mnt/test"));
        // The mount metadata should be registered
        assert!(state.vfs.mounts.contains_key("/mnt/test"));
    }

    #[test]
    fn mount_unmount() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        // Manually add a mount
        state.vfs.add_mount("/mnt/host".to_string(), "host".to_string());
        state.vfs.mkdir("/mnt").unwrap();
        state.vfs.mkdir("/mnt/host").unwrap();
        let output = service.execute_command(&mut state, "mount -u /mnt/host", None);
        assert!(output.stdout.contains("Unmounted"));
        assert!(!state.vfs.mounts.contains_key("/mnt/host"));
    }

    #[test]
    fn mount_unmount_not_mounted() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        let output = service.execute_command(&mut state, "mount -u /mnt/nope", None);
        assert!(output.stderr.contains("not mounted"));
    }

    #[test]
    fn mount_lists_after_mount() {
        let service = Service::new();
        let mut state = ShellState::new(Vfs::new());
        state.vfs.add_mount("/mnt/a".to_string(), "dir_a".to_string());
        state.vfs.add_mount("/mnt/b".to_string(), "dir_b".to_string());
        let output = service.execute_command(&mut state, "mount", None);
        assert!(output.stdout.contains("/mnt/a -> dir_a"));
        assert!(output.stdout.contains("/mnt/b -> dir_b"));
    }
}
