//! Command dispatch via the registry.
//!
//! This module implements [`Shell::execute_with_stdin`], which is the final
//! step of command execution: tokenise the input, look up the command in
//! the registry, build a [`CommandContext`], and call `execute`.

use crate::command::CommandContext;
use crate::shell::Shell;

impl Shell {
    /// Execute a single command string with optional stdin from a preceding
    /// pipe stage.
    ///
    /// # Flow
    ///
    /// 1. Tokenise the input by whitespace.
    /// 2. Look up the command name in the registry.
    /// 3. If stdin is non-empty **and** the command declares
    ///    `accepts_stdin()`, write stdin to `/tmp/.pipe_input` and append
    ///    that path to the argument list.  This lets file-reading commands
    ///    (cat, grep, wc, …) consume piped data transparently.
    /// 4. Build a [`CommandContext`] and call the command's `execute` method.
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
            registry: &self.registry,
        };

        command.execute(&mut ctx)
    }
}
