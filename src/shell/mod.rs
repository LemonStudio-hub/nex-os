//! Shell state management and top-level command execution.
//!
//! This module defines two core types:
//!
//! - [`ShellState`] — serializable, owns all mutable data (VFS, history,
//!   environment variables, identity).  Passed into and returned from every
//!   operation, enabling stateless service design.
//! - [`Service`] — stateless, holds only the immutable command [`Registry`].
//!   All methods accept [`ShellState`] as input and return results alongside
//!   the (potentially modified) state.
//!
//! # Execution flow
//!
//! 1. Receive raw input from the frontend.
//! 2. Split by `&&` (sequential chaining, stop on first error).
//! 3. Split each segment by `|` (pipeline stages).
//! 4. Extract `>` / `>>` redirections.
//! 5. Execute the pipeline, passing stdin between stages.
//!
//! Submodules:
//! - [`dispatch`] — single-command execution via the registry.
//! - [`pipeline`] — pipe splitting and redirect extraction.

mod dispatch;
mod pipeline;

use crate::command::Registry;
use crate::vfs::Vfs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ShellState — all mutable data, serializable for cross-Worker transfer
// ---------------------------------------------------------------------------

/// Serializable shell state.
///
/// Contains every piece of mutable data: the virtual file system, identity
/// metadata, command history, and environment variables.  This struct is
/// passed into and returned from every [`Service`] method, enabling
/// stateless operation and parallel invocation by multiple Workers.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShellState {
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
}

impl ShellState {
    /// Create a fresh state with default identity (`user@nexos`) and
    /// standard environment variables (`HOME`, `PATH`, `PWD`, etc.).
    pub fn new(vfs: Vfs) -> Self {
        let username = "user".to_string();
        let hostname = "nexos".to_string();

        let mut env_vars = HashMap::new();
        env_vars.insert("USER".to_string(), username.clone());
        env_vars.insert("HOSTNAME".to_string(), hostname.clone());
        env_vars.insert("HOME".to_string(), "/home/user".to_string());
        env_vars.insert("SHELL".to_string(), "/bin/nexsh".to_string());
        env_vars.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        env_vars.insert("PWD".to_string(), "/".to_string());
        env_vars.insert("TERM".to_string(), "xterm-256color".to_string());

        ShellState {
            vfs,
            username,
            hostname,
            history: Vec::new(),
            env_vars,
        }
    }

    /// Restore state from a persisted JSON snapshot, applying the given
    /// username.  Returns `None` if deserialization fails.
    pub fn from_state_json(json: &str, username: &str) -> Option<Self> {
        Vfs::from_json(json).ok().map(|vfs| {
            let mut state = Self::new(vfs);
            state.username = username.to_string();
            state
                .env_vars
                .insert("USER".to_string(), username.to_string());
            state
        })
    }

    /// Build the formatted prompt string with ANSI colour codes.
    ///
    /// Displayed as `user@hostname:/cwd$ ` with green for the identity,
    /// blue for the path, and a reset sequence at the end.
    pub fn get_prompt(&self) -> String {
        format!(
            "\x1b[1;32m{}@{}:\x1b[1;34m{}\x1b[0m$ ",
            self.username, self.hostname, self.vfs.cwd
        )
    }

    /// Serialize the VFS to JSON for OPFS persistence.
    pub fn to_json(&self) -> String {
        self.vfs.to_json()
    }
}

// ---------------------------------------------------------------------------
// Service — stateless command executor
// ---------------------------------------------------------------------------

/// Stateless shell service.
///
/// Holds only the immutable command [`Registry`].  All operations accept a
/// [`ShellState`] as input and return results alongside the (potentially
/// modified) state.  Because the service carries no mutable data, it is safe
/// to share across Workers — each Worker holds its own state independently.
pub struct Service {
    /// Central registry of all available commands, built once at init.
    registry: Registry,
}

impl Service {
    /// Create a new service with all built-in commands registered.
    pub fn new() -> Self {
        Service {
            registry: Registry::new(),
        }
    }

    /// Execute a full command string against the given state.
    ///
    /// Returns `(output, new_state)` — the command output and the modified
    /// state.  See [`ShellState`] for details on what changes between the
    /// input and output state.
    pub fn execute_command(&self, state: &mut ShellState, input: &str) -> String {
        let input = input.trim();
        if input.is_empty() {
            return String::new();
        }
        state.history.push(input.to_string());

        // Update PWD env var to match current directory
        state
            .env_vars
            .insert("PWD".to_string(), state.vfs.cwd.clone());

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
            match self.run_pipeline(state, &pipe) {
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
        &self,
        state: &mut ShellState,
        pipeline: &[(String, Option<(String, bool)>)],
    ) -> Result<String, String> {
        let mut current_input = String::new();

        for (i, (cmd_part, redirect)) in pipeline.iter().enumerate() {
            let is_last = i == pipeline.len() - 1;
            let result = self.execute_with_stdin(state, cmd_part, &current_input)?;

            if is_last {
                // Last stage: handle redirection if present
                if let Some((target, append)) = redirect {
                    let write_result = if *append {
                        let existing = state.vfs.read_file(target).unwrap_or_default();
                        state
                            .vfs
                            .write_file(target, &format!("{}{}", existing, result))
                    } else {
                        state.vfs.write_file(target, &result)
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
    pub fn get_completions(&self, _state: &ShellState, partial: &str) -> Vec<String> {
        self.registry.completions(partial)
    }

    /// Return the command history from the given state.
    pub fn get_history(&self, state: &ShellState) -> Vec<String> {
        state.history.clone()
    }
}

impl Default for Service {
    fn default() -> Self {
        Self::new()
    }
}
