//! Shell state management

use crate::command;
use crate::vfs::Vfs;

/// Shell state
pub struct Shell {
    pub vfs: Vfs,
    pub username: String,
    pub hostname: String,
    pub history: Vec<String>,
}

impl Shell {
    /// Create a new shell with default user and hostname
    pub fn new(vfs: Vfs) -> Self {
        Shell {
            vfs,
            username: "user".to_string(),
            hostname: "web-code".to_string(),
            history: Vec::new(),
        }
    }

    /// Get the formatted prompt string with ANSI colors
    /// Green for user@host, blue for cwd, reset after
    pub fn get_prompt(&self) -> String {
        format!(
            "\x1b[1;32m{}@{}:\x1b[1;34m{}\x1b[0m$ ",
            self.username, self.hostname, self.vfs.cwd
        )
    }

    /// Parse and execute a command string
    ///
    /// 1. Trim input
    /// 2. Add to history if non-empty
    /// 3. Split by `&&` and execute sequentially, stopping on error
    /// 4. Handle `>` and `>>` redirection for the last command in the chain
    /// 5. Parse command name and args
    /// 6. Dispatch to the appropriate handler
    pub fn execute(&mut self, input: &str) -> String {
        let input = input.trim();
        if input.is_empty() {
            return String::new();
        }
        self.history.push(input.to_string());

        // Split by && to support chained commands
        let segments: Vec<&str> = input
            .split("&&")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut output = String::new();

        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;

            // Handle redirection for the last command in the pipeline
            if is_last {
                if let Some((cmd_part, target, append)) = Self::parse_redirect(segment) {
                    match self.execute_single(&cmd_part) {
                        Ok(result) => {
                            let write_result = if append {
                                let existing = self.vfs.read_file(&target).unwrap_or_default();
                                self.vfs
                                    .write_file(&target, &format!("{}{}", existing, result))
                            } else {
                                self.vfs.write_file(&target, &result)
                            };
                            if let Err(e) = write_result {
                                output.push_str(&e);
                                output.push('\n');
                            }
                        }
                        Err(e) => {
                            output.push_str(&e);
                            if !e.ends_with('\n') {
                                output.push('\n');
                            }
                        }
                    }
                    continue;
                }
            }

            // Execute the command normally
            match self.execute_single(segment) {
                Ok(result) => {
                    output.push_str(&result);
                }
                Err(e) => {
                    output.push_str(&e);
                    if !e.ends_with('\n') {
                        output.push('\n');
                    }
                    // Stop execution on error (&& semantics)
                    break;
                }
            }
        }

        output
    }

    /// Get tab completion candidates for a partial input
    pub fn get_completions(&self, partial: &str) -> Vec<String> {
        let commands = [
            "ls", "cd", "pwd", "mkdir", "touch", "rm", "cat", "echo", "cp", "mv", "tree",
            "clear", "help", "exit",
        ];
        commands
            .iter()
            .filter(|cmd| cmd.starts_with(partial))
            .map(|cmd| cmd.to_string())
            .collect()
    }

    /// Get the command history
    pub fn get_history(&self) -> &Vec<String> {
        &self.history
    }

    // --- Private helpers ---

    /// Parse `>` and `>>` redirection from a command string.
    /// Returns (command_part, target_file, is_append) if found.
    fn parse_redirect(cmd: &str) -> Option<(String, String, bool)> {
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        for i in 0..tokens.len() {
            if tokens[i] == ">>" && i + 1 < tokens.len() {
                let cmd_part = tokens[..i].join(" ");
                let target = tokens[i + 1].to_string();
                return Some((cmd_part, target, true));
            }
            if tokens[i] == ">" && i + 1 < tokens.len() {
                let cmd_part = tokens[..i].join(" ");
                let target = tokens[i + 1].to_string();
                return Some((cmd_part, target, false));
            }
        }
        None
    }

    /// Execute a single command (no `&&` or redirection handling).
    fn execute_single(&mut self, input: &str) -> Result<String, String> {
        let tokens: Vec<&str> = input.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(String::new());
        }

        let cmd_name = tokens[0];
        let args = &tokens[1..];

        match cmd_name {
            "ls" => command::ls::execute(&self.vfs, args),
            "cd" => command::cd::execute(&mut self.vfs, args),
            "pwd" => command::pwd::execute(&self.vfs),
            "mkdir" => command::mkdir::execute(&mut self.vfs, args),
            "touch" => command::touch::execute(&mut self.vfs, args),
            "rm" => command::rm::execute(&mut self.vfs, args),
            "cat" => command::cat::execute(&self.vfs, args),
            "echo" => command::echo::execute(&mut self.vfs, args),
            "cp" => command::cp::execute(&mut self.vfs, args),
            "mv" => command::mv::execute(&mut self.vfs, args),
            "tree" => command::tree::execute(&self.vfs, args),
            "clear" => Ok("\x1b[2J\x1b[H".to_string()),
            "help" => Ok(command::help::execute()),
            "exit" => Ok(String::new()),
            _ => Err(format!("command not found: {}", cmd_name)),
        }
    }
}
