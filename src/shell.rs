//! Shell state management

use crate::command;
use crate::vfs::Vfs;
use std::collections::HashMap;

/// Shell state
pub struct Shell {
    pub vfs: Vfs,
    pub username: String,
    pub hostname: String,
    pub history: Vec<String>,
    pub env_vars: HashMap<String, String>,
}

impl Shell {
    /// Create a new shell with default user and hostname
    pub fn new(vfs: Vfs) -> Self {
        let username = "user".to_string();
        let hostname = "web-code".to_string();

        // Populate default environment variables
        let mut env_vars = HashMap::new();
        env_vars.insert("USER".to_string(), username.clone());
        env_vars.insert("HOSTNAME".to_string(), hostname.clone());
        env_vars.insert("HOME".to_string(), "/home/user".to_string());
        env_vars.insert("SHELL".to_string(), "/bin/web-sh".to_string());
        env_vars.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        env_vars.insert("PWD".to_string(), "/".to_string());
        env_vars.insert("TERM".to_string(), "xterm-256color".to_string());

        Shell {
            vfs,
            username,
            hostname,
            history: Vec::new(),
            env_vars,
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

    /// Parse and execute a command string.
    ///
    /// Precedence (highest to lowest):
    /// 1. `|`  (pipe) — within a `&&` segment
    /// 2. `>`, `>>` (redirection) — within a single pipe stage
    /// 3. `&&` (sequential chain — stop on first error)
    pub fn execute(&mut self, input: &str) -> String {
        let input = input.trim();
        if input.is_empty() {
            return String::new();
        }
        self.history.push(input.to_string());

        // Update PWD env var to match current directory
        self.env_vars
            .insert("PWD".to_string(), self.vfs.cwd.clone());

        // Step 1: Split by `&&` (sequential execution, stop on error)
        let segments: Vec<&str> = input
            .split("&&")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut output = String::new();

        for segment in segments {
            // Step 2: Within each `&&` segment, split by top-level `|`
            let stages: Vec<String> = Self::split_pipe_stages(segment);

            // Step 3: For each stage, extract redirection, then execute the pipeline
            let mut pipeline: Vec<(String, Option<(String, bool)>)> = Vec::new();
            for stage in &stages {
                let (cmd_part, redirect) = Self::extract_redirect(stage);
                pipeline.push((cmd_part, redirect));
            }

            // Run the pipeline; only the LAST stage honours redirection
            match self.run_pipeline(&pipeline) {
                Ok(result) => {
                    output.push_str(&result);
                }
                Err(e) => {
                    output.push_str(&e);
                    if !e.ends_with('\n') {
                        output.push('\n');
                    }
                    break; // `&&` semantics: stop on first error
                }
            }
        }

        output
    }

    /// Split a command string by top-level `|` tokens, respecting quoted strings.
    ///
    /// Example: `"cat file | grep hello | wc -l"` → `["cat file", "grep hello", "wc -l"]`
    fn split_pipe_stages(input: &str) -> Vec<String> {
        let mut stages = Vec::new();
        let mut current = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                    current.push(chars[i]);
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                    current.push(chars[i]);
                }
                '|' if !in_single_quote && !in_double_quote => {
                    stages.push(current.trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(chars[i]);
                }
            }
            i += 1;
        }

        let last = current.trim().to_string();
        if !last.is_empty() {
            stages.push(last);
        }

        // Ensure at least one stage
        if stages.is_empty() {
            stages.push(input.trim().to_string());
        }

        stages
    }

    /// Extract `>` / `>>` redirection from a single pipeline stage.
    ///
    /// Returns `(command_part, Some((target_file, is_append)))` if redirection is found,
    /// otherwise `(original, None)`.
    ///
    /// Handles both `cmd > file` (operator as separate token) and `cmd>file` (no spaces).
    fn extract_redirect(cmd: &str) -> (String, Option<(String, bool)>) {
        // First try token-based parsing (handles `cmd > file`)
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        for i in 0..tokens.len() {
            if tokens[i] == ">>" && i + 1 < tokens.len() {
                let cmd_part = tokens[..i].join(" ");
                let target = tokens[i + 1]
                    .trim_matches('\'')
                    .trim_matches('"')
                    .to_string();
                return (cmd_part, Some((target, true)));
            }
            if tokens[i] == ">" && i + 1 < tokens.len() {
                let cmd_part = tokens[..i].join(" ");
                let target = tokens[i + 1]
                    .trim_matches('\'')
                    .trim_matches('"')
                    .to_string();
                return (cmd_part, Some((target, false)));
            }
        }

        // Fallback: check for `cmd>>file` or `cmd>file` (no spaces)
        if let Some(idx) = cmd.find(">>") {
            let cmd_part = cmd[..idx].trim().to_string();
            let target = cmd[idx + 2..]
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();
            if !cmd_part.is_empty() && !target.is_empty() {
                return (cmd_part, Some((target, true)));
            }
        } else if let Some(idx) = cmd.find('>') {
            let cmd_part = cmd[..idx].trim().to_string();
            let target = cmd[idx + 1..]
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();
            if !cmd_part.is_empty() && !target.is_empty() {
                return (cmd_part, Some((target, false)));
            }
        }

        (cmd.to_string(), None)
    }

    /// Execute a pipeline of commands, passing stdout of each as stdin to the next.
    ///
    /// Only the last stage's redirection (if any) is applied.
    fn run_pipeline(
        &mut self,
        pipeline: &[(String, Option<(String, bool)>)],
    ) -> Result<String, String> {
        let mut current_input = String::new();

        for (i, (cmd_part, redirect)) in pipeline.iter().enumerate() {
            let is_last = i == pipeline.len() - 1;
            let result = self.execute_with_stdin(cmd_part, &current_input)?;

            if is_last {
                // Last stage: handle redirection if present
                if let Some((target, append)) = redirect {
                    let write_result = if *append {
                        let existing = self.vfs.read_file(target).unwrap_or_default();
                        self.vfs
                            .write_file(target, &format!("{}{}", existing, result))
                    } else {
                        self.vfs.write_file(target, &result)
                    };
                    write_result?;
                    // When redirecting to a file, produce no terminal output
                    return Ok(String::new());
                }
            }

            current_input = result;
        }

        Ok(current_input)
    }

    /// Execute a single command with optional stdin from a preceding pipe stage.
    ///
    /// If the command has no file arguments, `stdin` is passed as a trailing argument
    /// so that file-reading commands (cat, grep, sort, etc.) can consume piped input.
    fn execute_with_stdin(&mut self, input: &str, stdin: &str) -> Result<String, String> {
        let tokens: Vec<&str> = input.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(String::new());
        }

        // Commands that read file contents — they accept stdin as a synthetic argument
        let file_reading_commands = ["cat", "head", "tail", "wc", "grep", "sort", "uniq", "cut"];

        let cmd_name = tokens[0];
        let args = &tokens[1..];

        // If stdin is non-empty and the command can consume it as file data,
        // write stdin to a temp file and pass that file path as a trailing argument.
        // This allows commands like `echo hello | grep hello` to work — grep's
        // first positional arg is the pattern, not a file, so stdin must still be passed.
        let temp_path;
        let effective_args: Vec<&str> =
            if !stdin.is_empty() && file_reading_commands.contains(&cmd_name) {
                // Write stdin content to a temp file so file-reading commands can consume it
                temp_path = "/tmp/.pipe_input".to_string();
                let _ = self.vfs.write_file(&temp_path, stdin);
                let mut new_args: Vec<&str> = args.to_vec();
                new_args.push(&temp_path);
                new_args
            } else {
                args.to_vec()
            };

        let args_slice = effective_args.as_slice();

        match cmd_name {
            // Filesystem navigation
            "ls" => command::ls::execute(&self.vfs, args_slice),
            "cd" => command::cd::execute(&mut self.vfs, args_slice),
            "pwd" => command::pwd::execute(&self.vfs),
            "mkdir" => command::mkdir::execute(&mut self.vfs, args_slice),
            "touch" => command::touch::execute(&mut self.vfs, args_slice),
            "rm" => command::rm::execute(&mut self.vfs, args_slice),
            "cp" => command::cp::execute(&mut self.vfs, args_slice),
            "mv" => command::mv::execute(&mut self.vfs, args_slice),
            "tree" => command::tree::execute(&self.vfs, args_slice),
            "ln" => command::ln::execute(&mut self.vfs, args_slice),

            // File content
            "cat" => command::cat::execute(&self.vfs, args_slice),
            "echo" => command::echo::execute(&mut self.vfs, args_slice),
            "head" => command::head::execute(&self.vfs, args_slice),
            "tail" => command::tail::execute(&self.vfs, args_slice),

            // Text processing
            "grep" => command::grep::execute(&self.vfs, args_slice),
            "sort" => command::sort::execute(&self.vfs, args_slice),
            "uniq" => command::uniq::execute(&self.vfs, args_slice),
            "wc" => command::wc::execute(&self.vfs, args_slice),
            "cut" => command::cut::execute(&self.vfs, args_slice),
            "tr" => command::tr::execute(stdin, args_slice),
            "tee" => command::tee::execute(&mut self.vfs, stdin, args_slice),

            // Diff
            "diff" => command::diff::execute(&self.vfs, args_slice),

            // Search
            "find" => command::find::execute(&self.vfs, args_slice),

            // Disk usage
            "du" => command::du::execute(&self.vfs, args_slice),

            // Permissions & ownership (simulated)
            "chmod" => command::chmod::execute(&mut self.vfs, args_slice),
            "chown" => command::chown::execute(args_slice),

            // System info
            "whoami" => command::whoami::execute(&self.username),
            "hostname" => command::hostname::execute(&self.hostname),
            "date" => command::date::execute(),
            "history" => command::history::execute(&self.history),

            // Environment
            "env" => command::env::execute(&self.env_vars),
            "export" => command::export::execute(&mut self.env_vars, args_slice),

            // Path utilities
            "basename" => command::basename::execute(args_slice),
            "dirname" => command::dirname::execute(args_slice),

            // Documentation
            "man" => command::man::execute(args_slice),

            // Terminal
            "clear" => Ok("\x1b[2J\x1b[H".to_string()),
            "help" => Ok(command::help::execute()),
            "exit" => Ok(String::new()),

            _ => Err(format!("command not found: {}", cmd_name)),
        }
    }

    /// Get tab completion candidates for a partial input
    pub fn get_completions(&self, partial: &str) -> Vec<String> {
        let commands = [
            "ls", "cd", "pwd", "mkdir", "touch", "rm", "cat", "echo", "cp", "mv", "tree", "clear",
            "help", "exit", "head", "tail", "wc", "grep", "find", "sort", "uniq", "whoami",
            "hostname", "date", "history", "diff", "du", "tr", "cut", "tee", "ln", "chmod",
            "chown", "man", "env", "export", "basename", "dirname",
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
}
