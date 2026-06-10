//! Command dispatch via the registry.

use crate::command::CommandContext;
use crate::shell::Shell;

impl Shell {
    /// Execute a single command with optional stdin from a preceding pipe stage.
    ///
    /// Uses the command registry for dispatch. If the command accepts stdin,
    /// writes stdin to a temp file and passes that path as a trailing argument.
    pub fn execute_with_stdin(&mut self, input: &str, stdin: &str) -> Result<String, String> {
        let tokens: Vec<&str> = input.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(String::new());
        }

        let cmd_name = tokens[0];
        let args = &tokens[1..];

        let command = self
            .registry
            .get(cmd_name)
            .ok_or_else(|| format!("command not found: {}", cmd_name))?;

        // If stdin is non-empty and the command can consume it as file data,
        // write stdin content to a temp file and pass that file path as a trailing argument.
        let temp_path;
        let effective_args: Vec<&str> = if !stdin.is_empty() && command.accepts_stdin() {
            temp_path = "/tmp/.pipe_input".to_string();
            let _ = self.vfs.write_file(&temp_path, stdin);
            let mut new_args: Vec<&str> = args.to_vec();
            new_args.push(&temp_path);
            new_args
        } else {
            args.to_vec()
        };

        let mut ctx = CommandContext {
            vfs: &mut self.vfs,
            stdin,
            args: &effective_args,
            username: &self.username,
            hostname: &self.hostname,
            history: &self.history,
            env_vars: &mut self.env_vars,
        };

        command.execute(&mut ctx)
    }
}
