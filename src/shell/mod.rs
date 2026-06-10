//! Shell state management

mod dispatch;
mod pipeline;

use crate::command::Registry;
use crate::vfs::Vfs;
use std::collections::HashMap;

/// Shell state
pub struct Shell {
    pub vfs: Vfs,
    pub username: String,
    pub hostname: String,
    pub history: Vec<String>,
    pub env_vars: HashMap<String, String>,
    registry: Registry,
}

impl Shell {
    /// Create a new shell with default user and hostname
    pub fn new(vfs: Vfs) -> Self {
        let username = "user".to_string();
        let hostname = "nexos".to_string();

        // Populate default environment variables
        let mut env_vars = HashMap::new();
        env_vars.insert("USER".to_string(), username.clone());
        env_vars.insert("HOSTNAME".to_string(), hostname.clone());
        env_vars.insert("HOME".to_string(), "/home/user".to_string());
        env_vars.insert("SHELL".to_string(), "/bin/nexsh".to_string());
        env_vars.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        env_vars.insert("PWD".to_string(), "/".to_string());
        env_vars.insert("TERM".to_string(), "xterm-256color".to_string());

        Shell {
            vfs,
            username,
            hostname,
            history: Vec::new(),
            env_vars,
            registry: Registry::new(),
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
            let stages: Vec<String> = pipeline::split_pipe_stages(segment);

            // Step 3: For each stage, extract redirection, then execute the pipeline
            let mut pipe: Vec<(String, Option<(String, bool)>)> = Vec::new();
            for stage in &stages {
                let (cmd_part, redirect) = pipeline::extract_redirect(stage);
                pipe.push((cmd_part, redirect));
            }

            // Run the pipeline; only the LAST stage honours redirection
            match self.run_pipeline(&pipe) {
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

    /// Get tab completion candidates for a partial input
    pub fn get_completions(&self, partial: &str) -> Vec<String> {
        self.registry.completions(partial)
    }

    /// Get the command history
    pub fn get_history(&self) -> &Vec<String> {
        &self.history
    }
}
