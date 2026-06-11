//! Shell state management and top-level command execution.
//!
//! The [`Shell`] struct owns the VFS, command registry, environment variables,
//! and command history.  It is the central coordinator that:
//!
//! 1. Receives raw input from the frontend.
//! 2. Splits it by `&&` (sequential chaining, stop on first error).
//! 3. Splits each segment by `|` (pipeline stages).
//! 4. Extracts `>` / `>>` redirections.
//! 5. Executes the pipeline, passing stdin between stages.
//!
//! Submodules:
//! - [`dispatch`] — single-command execution via the registry.
//! - [`pipeline`] — pipe splitting and redirect extraction.

mod dispatch;
mod pipeline;

use crate::command::Registry;
use crate::vfs::Vfs;
use std::collections::HashMap;

/// The top-level shell state.
///
/// Holds everything needed to execute commands: the virtual file system,
/// the command registry, environment variables, command history, and
/// identity metadata (username / hostname) used in the prompt.
pub struct Shell {
    /// The in-memory virtual file system.
    pub vfs: Vfs,
    /// The logged-in username (displayed in the prompt).
    pub username: String,
    /// The machine hostname (displayed in the prompt).
    pub hostname: String,
    /// Chronological list of previously executed command strings.
    pub history: Vec<String>,
    /// Shell environment variables (e.g. `HOME`, `PATH`, `PWD`).
    pub env_vars: HashMap<String, String>,
    /// Central registry of all available commands, built once at init.
    registry: Registry,
}

impl Shell {
    /// Create a new shell with default identity (`user@nexos`) and
    /// populate standard environment variables (`HOME`, `PATH`, `PWD`, etc.).
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

    /// Build the formatted prompt string with ANSI colour codes.
    ///
    /// The prompt is displayed as `user@hostname:/cwd$ ` with green for the
    /// identity, blue for the path, and a reset sequence at the end.
    pub fn get_prompt(&self) -> String {
        format!(
            "\x1b[1;32m{}@{}:\x1b[1;34m{}\x1b[0m$ ",
            self.username, self.hostname, self.vfs.cwd
        )
    }

    /// Parse and execute a full command string.
    ///
    /// # Operator precedence (highest → lowest)
    ///
    /// 1. `|`  — pipe: within a single `&&` segment.
    /// 2. `>`, `>>` — file redirection: within a single pipe stage.
    /// 3. `&&` — sequential chaining: stop on the first error.
    ///
    /// # Behaviour
    ///
    /// - The raw input is appended to the command history.
    /// - The `PWD` environment variable is updated to match `vfs.cwd`.
    /// - On success the combined stdout of the pipeline is returned.
    /// - On error the error message is returned and remaining `&&` segments
    ///   are skipped.
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

    /// Execute a pipeline of commands, passing the stdout of each stage as
    /// stdin to the next.
    ///
    /// Only the **last** stage's redirection (if any) is applied.  If the
    /// last stage redirects to a file (`>` or `>>`), the terminal output
    /// is empty — the content is written to the VFS file instead.
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

    /// Get tab-completion candidates for the given partial input string.
    ///
    /// Delegates to the command registry, which matches against registered
    /// command names.
    pub fn get_completions(&self, partial: &str) -> Vec<String> {
        self.registry.completions(partial)
    }

    /// Return a reference to the command history list.
    pub fn get_history(&self) -> &Vec<String> {
        &self.history
    }
}
