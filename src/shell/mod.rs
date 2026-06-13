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
pub mod pipeline;

use crate::command::{CommandOutput, Registry};
use crate::vfs::{HostFs, Vfs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// StateDirtyFlags — tracks non-VFS state changes for incremental persistence
// ---------------------------------------------------------------------------

/// Tracks which non-VFS state fields have changed since the last save.
///
/// Serialized with the state JSON so flags survive the round-trip between
/// the frontend and WASM.  Uses `#[serde(default)]` for backward
/// compatibility with state JSONs that predate this field.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StateDirtyFlags {
    #[serde(default)]
    pub history: bool,
    #[serde(default)]
    pub env_vars: bool,
}

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
    /// Exit code of the last executed command (`$?` in shell).
    #[serde(default)]
    pub last_exit_code: i32,
    /// Tracks which non-VFS fields have changed since the last save.
    #[serde(default)]
    pub dirty_state: StateDirtyFlags,
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
            last_exit_code: 0,
            dirty_state: StateDirtyFlags::default(),
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

    /// Serialize non-VFS state (history, env_vars, hostname) to JSON for
    /// separate OPFS persistence.  Returns a JSON object with those three fields.
    pub fn to_state_json(&self) -> String {
        let state_data = serde_json::json!({
            "history": self.history,
            "env_vars": self.env_vars,
            "hostname": self.hostname,
        });
        serde_json::to_string(&state_data).unwrap_or_else(|_| "{}".to_string())
    }

    /// Restore a full ShellState by combining an existing VFS with saved
    /// non-VFS state (from `nexos_state.json`) and the current username.
    ///
    /// Falls back to defaults for any missing fields.  Returns `None` only
    /// if the VFS itself is invalid.
    pub fn from_state_json_with_vfs(
        vfs: Vfs,
        saved_state_json: Option<&str>,
        username: &str,
    ) -> Option<Self> {
        let mut state = Self::new(vfs);
        state.username = username.to_string();
        state
            .env_vars
            .insert("USER".to_string(), username.to_string());

        if let Some(json) = saved_state_json {
            if !json.is_empty() {
                let saved: serde_json::Value = match serde_json::from_str(json) {
                    Ok(v) => v,
                    Err(_) => return Some(state), // malformed -> use defaults
                };

                // Restore history
                if let Some(history) = saved.get("history").and_then(|v| v.as_array()) {
                    state.history = history
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }

                // Restore env_vars (merge with defaults; saved values override)
                if let Some(env) = saved.get("env_vars").and_then(|v| v.as_object()) {
                    for (key, value) in env {
                        if let Some(val_str) = value.as_str() {
                            state.env_vars.insert(key.clone(), val_str.to_string());
                        }
                    }
                    // Re-apply USER to ensure it matches the current login
                    state
                        .env_vars
                        .insert("USER".to_string(), username.to_string());
                }

                // Restore hostname
                if let Some(hostname) = saved.get("hostname").and_then(|v| v.as_str()) {
                    state.hostname = hostname.to_string();
                    state
                        .env_vars
                        .insert("HOSTNAME".to_string(), hostname.to_string());
                }
            }
        }

        Some(state)
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
    /// Returns a [`CommandOutput`] with separate stdout, stderr, and exit
    /// code.  The `&&` chaining mechanism stops when `exit_code != 0`.
    pub fn execute_command(
        &self,
        state: &mut ShellState,
        input: &str,
        host_fs: Option<&dyn HostFs>,
    ) -> CommandOutput {
        let input = input.trim();
        if input.is_empty() {
            return CommandOutput::empty();
        }
        state.history.push(input.to_string());
        state.dirty_state.history = true;

        // Update PWD env var to match current directory, only if changed.
        let new_pwd = state.vfs.cwd.clone();
        if state.env_vars.get("PWD").map(|s| s.as_str()) != Some(&new_pwd) {
            state.env_vars.insert("PWD".to_string(), new_pwd);
            state.dirty_state.env_vars = true;
        }

        // Step 1: Split by `&&` (sequential execution, stop on error)
        let segments: Vec<&str> = input
            .split("&&")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut exit_code: i32 = 0;

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
            let result = self.run_pipeline(state, &pipe, host_fs);

            // Propagate special actions (e.g. mount requests)
            if result.action.is_some() {
                state.last_exit_code = result.exit_code;
                return result;
            }

            stdout.push_str(&result.stdout);
            stderr.push_str(&result.stderr);
            exit_code = result.exit_code;

            // `&&` semantics: stop on first non-zero exit code
            if exit_code != 0 {
                break;
            }
        }

        state.last_exit_code = exit_code;

        CommandOutput {
            stdout,
            stderr,
            exit_code,
            action: None,
        }
    }

    /// Execute a pipeline of commands, passing the stdout of each stage as
    /// stdin to the next.
    ///
    /// Only the **last** stage's redirection (if any) is applied.  If the
    /// last stage redirects to a file (`>` or `>>`), the terminal stdout
    /// is empty — the content is written to the VFS file instead.
    /// Stderr is accumulated across all stages and never piped.
    fn run_pipeline(
        &self,
        state: &mut ShellState,
        pipeline: &[(String, Option<(String, bool)>)],
        host_fs: Option<&dyn HostFs>,
    ) -> CommandOutput {
        let mut current_stdout = String::new();
        let mut stderr = String::new();
        let mut exit_code: i32 = 0;

        for (i, (cmd_part, redirect)) in pipeline.iter().enumerate() {
            let is_last = i == pipeline.len() - 1;
            let result = self.execute_with_stdin(state, cmd_part, &current_stdout, host_fs);

            // Propagate special actions (e.g. mount requests)
            if result.action.is_some() {
                return result;
            }

            stderr.push_str(&result.stderr);
            exit_code = result.exit_code;

            if is_last {
                // Last stage: handle redirection if present
                if let Some((target, append)) = redirect {
                    let content = if *append {
                        let existing = state
                            .vfs
                            .read_file_with_host(target, host_fs)
                            .unwrap_or_default();
                        format!("{}{}", existing, result.stdout)
                    } else {
                        result.stdout
                    };
                    if let Err(e) = state.vfs.write_file_with_host(target, &content, host_fs) {
                        return CommandOutput::error("nexsh", &e);
                    }
                    // When redirecting to a file, produce no terminal stdout
                    return CommandOutput {
                        stdout: String::new(),
                        stderr,
                        exit_code,
                        action: None,
                    };
                }
            }

            current_stdout = result.stdout;
        }

        CommandOutput {
            stdout: current_stdout,
            stderr,
            exit_code,
            action: None,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_has_clean_flags() {
        let state = ShellState::new(Vfs::new());
        assert!(!state.dirty_state.history);
        assert!(!state.dirty_state.env_vars);
    }

    #[test]
    fn to_state_json_roundtrip() {
        let mut state = ShellState::new(Vfs::new());
        state.history.push("ls".to_string());
        state.history.push("cd /tmp".to_string());
        state.env_vars.insert("FOO".to_string(), "bar".to_string());

        let json = state.to_state_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["history"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["env_vars"]["FOO"].as_str().unwrap(), "bar");
        assert_eq!(parsed["hostname"].as_str().unwrap(), "nexos");
    }

    #[test]
    fn from_state_json_with_vfs_restores_history() {
        let vfs = Vfs::new();
        let saved = r#"{"history":["ls","pwd"],"env_vars":{},"hostname":"nexos"}"#;
        let state = ShellState::from_state_json_with_vfs(vfs, Some(saved), "user").unwrap();
        assert_eq!(state.history, vec!["ls", "pwd"]);
    }

    #[test]
    fn from_state_json_with_vfs_merges_env_vars() {
        let vfs = Vfs::new();
        let saved = r#"{"history":[],"env_vars":{"FOO":"bar","EDITOR":"vim"},"hostname":"nexos"}"#;
        let state = ShellState::from_state_json_with_vfs(vfs, Some(saved), "user").unwrap();
        // User-set var is present
        assert_eq!(state.env_vars.get("FOO").unwrap(), "bar");
        // Default vars are still present
        assert_eq!(state.env_vars.get("HOME").unwrap(), "/home/user");
        // USER is re-applied
        assert_eq!(state.env_vars.get("USER").unwrap(), "user");
    }

    #[test]
    fn from_state_json_with_vfs_handles_empty() {
        let vfs = Vfs::new();
        let state = ShellState::from_state_json_with_vfs(vfs, None, "alice").unwrap();
        assert!(state.history.is_empty());
        assert_eq!(state.username, "alice");
    }

    #[test]
    fn from_state_json_with_vfs_handles_malformed() {
        let vfs = Vfs::new();
        let state = ShellState::from_state_json_with_vfs(vfs, Some("not json"), "user");
        assert!(state.is_some()); // degrades to defaults
    }

    #[test]
    fn dirty_state_serializes_and_deserializes() {
        let mut state = ShellState::new(Vfs::new());
        state.dirty_state.history = true;
        state.dirty_state.env_vars = false;

        let json = serde_json::to_string(&state).unwrap();
        let restored: ShellState = serde_json::from_str(&json).unwrap();
        assert!(restored.dirty_state.history);
        assert!(!restored.dirty_state.env_vars);
    }

    #[test]
    fn old_state_json_without_dirty_state_defaults() {
        // Simulate an old state JSON that doesn't have dirty_state
        let state = ShellState::new(Vfs::new());
        let json = serde_json::to_string(&state).unwrap();
        // Remove dirty_state if present (simulate old format)
        let mut map: serde_json::Value = serde_json::from_str(&json).unwrap();
        map.as_object_mut().unwrap().remove("dirty_state");
        let old_json = serde_json::to_string(&map).unwrap();

        let restored: ShellState = serde_json::from_str(&old_json).unwrap();
        assert!(!restored.dirty_state.history);
        assert!(!restored.dirty_state.env_vars);
    }
}
