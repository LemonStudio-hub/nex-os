//! NexOS — WebAssembly entry point for the browser-based terminal.
//!
//! This crate is compiled to `wasm32-unknown-unknown` and loaded by the
//! TypeScript frontend via `wasm-bindgen`. All functions annotated with
//! `#[wasm_bindgen]` are exported to JavaScript as synchronous calls.
//!
//! # Architecture overview
//!
//! The immutable command [`Service`] is stored in a `thread_local!` and
//! shared across all calls.  Mutable shell state ([`ShellState`]) is
//! serialised to JSON and passed into every operation — the caller owns
//! the state and receives the updated version after each mutation.
//!
//! This stateless design enables:
//! - **Async execution** — the frontend can `postMessage` state to a Worker.
//! - **Parallel Workers** — each Worker holds its own state independently.
//! - **Deterministic replay** — state snapshots can be stored and restored.

use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Once;
use wasm_bindgen::prelude::*;

// Re-export the three core subsystems so downstream code can reference them
// via `crate::command`, `crate::shell`, and `crate::vfs`.
pub mod command;
pub mod shell;
pub mod vfs;

use command::CommandOutput;
use shell::{Service, ShellState, StateDirtyFlags};
use vfs::host_fs_wasm::WasmHostFs;
use vfs::{HostFs, Vfs};

// ---------------------------------------------------------------------------
// Panic recovery
// ---------------------------------------------------------------------------

/// One-time initialisation for the panic hook.  The hook itself doesn't need
/// to do anything special — `catch_unwind` extracts the payload.  This exists
/// so that panics are caught rather than aborting the WASM module.
static PANIC_HOOK: Once = Once::new();

fn ensure_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let _ = panic::take_hook(); // keep default behaviour (console error)
        panic::set_hook(Box::new(|_info| {
            // The message is extracted by catch_unwind below.
        }));
    });
}

/// Extract a human-readable string from a panic payload.
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown internal error".to_string()
    }
}

// ---------------------------------------------------------------------------
// Global service (immutable — no borrow conflicts)
// ---------------------------------------------------------------------------

// The single, process-wide service instance.  The service holds only the
// immutable command registry, so it is safe to borrow concurrently (though
// the WASM target is single-threaded anyway).  Starts as `None` and is
// populated by `init` or `init_with_username`.
thread_local! {
    static SERVICE: RefCell<Option<Service>> = const { RefCell::new(None) };
    static HOST_FS_REGISTRY: RefCell<HashMap<String, Box<dyn HostFs>>> = RefCell::new(HashMap::new());
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run a closure with a reference to the service.
///
/// Returns the provided fallback if the service has not been initialized.
fn with_service<R>(fallback: R, f: impl FnOnce(&Service) -> R) -> R {
    SERVICE.with(|s| {
        let borrow = s.borrow();
        match borrow.as_ref() {
            Some(service) => f(service),
            None => fallback,
        }
    })
}

// ---------------------------------------------------------------------------
// WASM exports — stateless API
// ---------------------------------------------------------------------------

/// Initialize the service and shell state.
///
/// If `state_json` is non-empty, attempts to restore the VFS from the
/// provided JSON; otherwise creates a fresh default VFS.
///
/// Returns the initial shell state as a JSON string.  The frontend must
/// store this string and pass it to every subsequent call.
#[wasm_bindgen]
pub fn init(state_json: &str) -> String {
    init_with_username(state_json, "user")
}

/// Initialize with a custom username.
///
/// Used after login when the user has set their own username.  Returns the
/// initial shell state as a JSON string.
#[wasm_bindgen]
pub fn init_with_username(state_json: &str, username: &str) -> String {
    // Build the service (immutable registry).
    let service = Service::new();

    // Build the initial state.
    let state = if state_json.is_empty() {
        ShellState::new(Vfs::new())
    } else {
        ShellState::from_state_json(state_json, username).unwrap_or_else(|| {
            let mut s = ShellState::new(Vfs::new());
            s.username = username.to_string();
            s
        })
    };

    // Store the service for future calls.
    SERVICE.with(|s| {
        *s.borrow_mut() = Some(service);
    });

    // Serialize the full state so the frontend can store and pass it back.
    serde_json::to_string(&state).unwrap_or_default()
}

/// Execute a command string against the given state.
///
/// `state_json` is the current shell state (as returned by a previous call
/// to `init` or `execute_command`).  `input` is the raw command line.
///
/// Returns a JSON object: `{"stdout": "...", "stderr": "...", "exit_code": 0, "state": "...", "action": null}`.
/// The frontend must parse this, display stdout/stderr appropriately, and
/// store the new state.
///
/// If a panic occurs during execution, the error is caught and returned as
/// stderr with exit code 1 instead of crashing the WASM module.
#[wasm_bindgen]
pub fn execute_command(state_json: &str, input: &str) -> String {
    ensure_panic_hook();

    // Deserialize the state.
    let mut state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => {
            return serde_json::json!({
                "stdout": "",
                "stderr": "Error: invalid shell state.\n",
                "exit_code": 1,
                "state": state_json,
                "action": null,
            })
            .to_string();
        }
    };

    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        with_service(
            CommandOutput::error("nexsh", "service not initialized"),
            |service| {
                // Build a composite HostFs from all registered mount handles.
                HOST_FS_REGISTRY.with(|registry| {
                    let reg = registry.borrow();
                    if reg.is_empty() {
                        service.execute_command(&mut state, input, None)
                    } else {
                        let host_fs: Option<&dyn HostFs> =
                            reg.values().next().map(|b| b.as_ref());
                        service.execute_command(&mut state, input, host_fs)
                    }
                })
            },
        )
    }));

    let output = match result {
        Ok(output) => output,
        Err(payload) => {
            let msg = panic_message(&payload);
            CommandOutput {
                stdout: String::new(),
                stderr: format!("NexOS: internal error — {}\n", msg),
                exit_code: 1,
                action: None,
            }
        }
    };

    // Return stdout, stderr, exit_code, action, and the updated state.
    let state_out = serde_json::to_string(&state).unwrap_or_default();
    serde_json::json!({
        "stdout": output.stdout,
        "stderr": output.stderr,
        "exit_code": output.exit_code,
        "state": state_out,
        "action": output.action,
    })
    .to_string()
}

/// Get the current prompt string (with ANSI colour codes).
///
/// `state_json` is the current shell state.  Returns the formatted prompt.
/// Returns `"$ "` on invalid state or panic.
#[wasm_bindgen]
pub fn get_prompt(state_json: &str) -> String {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return "$ ".to_string(),
    };
    let result = panic::catch_unwind(AssertUnwindSafe(|| state.get_prompt()));
    result.unwrap_or_else(|_| "$ ".to_string())
}

/// Get tab completion candidates for the given partial input.
///
/// `state_json` is the current shell state.  Returns matching command names.
/// Returns an empty list on invalid state or panic.
#[wasm_bindgen]
pub fn get_completions(state_json: &str, partial: &str) -> Vec<String> {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        with_service(vec![], |service| service.get_completions(&state, partial))
    }));
    result.unwrap_or_default()
}

/// Get the command history from the given state.
///
/// Returns an empty list on invalid state or panic.
#[wasm_bindgen]
pub fn get_history(state_json: &str) -> Vec<String> {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        with_service(vec![], |service| service.get_history(&state))
    }));
    result.unwrap_or_default()
}

/// Serialize the VFS from the given state to JSON for OPFS persistence.
///
/// Returns the VFS JSON, or an empty string if deserialization fails.
/// This is a convenience alias — the frontend could also extract the VFS
/// from the full state JSON directly.
#[wasm_bindgen]
pub fn get_state_json(state_json: &str) -> String {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    state.to_json()
}

/// Get the list of dirty (modified/created) file paths as a JSON array.
///
/// Returns `"[]"` if deserialization fails.
#[wasm_bindgen]
pub fn get_dirty_files_json(state_json: &str) -> String {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return "[]".to_string(),
    };
    serde_json::to_string(&state.vfs.get_dirty_files()).unwrap_or_else(|_| "[]".to_string())
}

/// Get the list of deleted file paths as a JSON array.
///
/// Returns `"[]"` if deserialization fails.
#[wasm_bindgen]
pub fn get_deleted_files_json(state_json: &str) -> String {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return "[]".to_string(),
    };
    serde_json::to_string(&state.vfs.get_deleted_files()).unwrap_or_else(|_| "[]".to_string())
}

/// Get the content of a single file from the given state.
///
/// Returns the file content as a plain string, or an empty string if the
/// file does not exist or deserialization fails.
#[wasm_bindgen]
pub fn get_file_content(state_json: &str, path: &str) -> String {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    state.vfs.read_file(path).unwrap_or_default()
}

/// Mark all files in the state as dirty (used during migration from the
/// legacy single-file persistence format).
///
/// Returns the updated state JSON, or the input unchanged on error.
#[wasm_bindgen]
pub fn mark_all_dirty(state_json: &str) -> String {
    ensure_panic_hook();
    let mut state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return state_json.to_string(),
    };
    let paths = state.vfs.collect_all_file_paths();
    for path in paths {
        state.vfs.mark_dirty(&path);
    }
    serde_json::to_string(&state).unwrap_or_else(|_| state_json.to_string())
}

/// Serialize the VFS tree structure with empty file contents.
///
/// Used for incremental storage — the tree skeleton is saved separately
/// from individual file contents.  Returns empty string on error.
#[wasm_bindgen]
pub fn get_tree_json(state_json: &str) -> String {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    state.vfs.to_tree_json()
}

/// Get which non-VFS state fields have changed since the last save.
///
/// Returns a JSON object: `{"history": bool, "env_vars": bool}`.
/// Returns `{}` on deserialization failure.
#[wasm_bindgen]
pub fn get_state_dirty_flags(state_json: &str) -> String {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return "{}".to_string(),
    };
    serde_json::json!({
        "history": state.dirty_state.history,
        "env_vars": state.dirty_state.env_vars,
    })
    .to_string()
}

/// Clear all dirty flags (VFS file dirty flags and non-VFS state dirty flags).
///
/// Returns the updated state JSON with all dirty flags cleared.
/// Call this after a successful OPFS save.
#[wasm_bindgen]
pub fn mark_state_clean(state_json: &str) -> String {
    ensure_panic_hook();
    let mut state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return state_json.to_string(),
    };
    state.vfs.mark_clean();
    state.dirty_state = StateDirtyFlags::default();
    serde_json::to_string(&state).unwrap_or_else(|_| state_json.to_string())
}

/// Serialize non-VFS state (history, env_vars, hostname) to JSON.
///
/// Returns a JSON object with those three fields, suitable for
/// persistence in `nexos_state.json`.
/// Returns `"{}"` on deserialization failure.
#[wasm_bindgen]
pub fn get_non_vfs_state_json(state_json: &str) -> String {
    ensure_panic_hook();
    let state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return "{}".to_string(),
    };
    state.to_state_json()
}

/// Merge saved non-VFS state (history, env_vars, hostname) into the
/// current ShellState.  Missing or empty fields in `saved_state_json`
/// are skipped (defaults are preserved).
///
/// Returns the updated state JSON.
#[wasm_bindgen]
pub fn apply_saved_state(state_json: &str, saved_state_json: &str) -> String {
    ensure_panic_hook();
    let mut state: ShellState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return state_json.to_string(),
    };

    if saved_state_json.is_empty() {
        return state_json.to_string();
    }

    let saved: serde_json::Value = match serde_json::from_str(saved_state_json) {
        Ok(v) => v,
        Err(_) => return state_json.to_string(),
    };

    // Restore history
    if let Some(history) = saved.get("history").and_then(|v| v.as_array()) {
        state.history = history
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
    }

    // Restore env_vars (merge; saved values override defaults)
    if let Some(env) = saved.get("env_vars").and_then(|v| v.as_object()) {
        for (key, value) in env {
            if let Some(val_str) = value.as_str() {
                state.env_vars.insert(key.clone(), val_str.to_string());
            }
        }
        // Re-apply USER to ensure it matches the current login
        state
            .env_vars
            .insert("USER".to_string(), state.username.clone());
    }

    // Restore hostname
    if let Some(hostname) = saved.get("hostname").and_then(|v| v.as_str()) {
        state.hostname = hostname.to_string();
        state
            .env_vars
            .insert("HOSTNAME".to_string(), hostname.to_string());
    }

    serde_json::to_string(&state).unwrap_or_else(|_| state_json.to_string())
}

// ---------------------------------------------------------------------------
// Host filesystem registration
// ---------------------------------------------------------------------------

/// Register a host filesystem adapter for a mounted directory.
///
/// `mount_id` is a unique identifier for this mount (typically the VFS path).
/// `callbacks` is a JS object with synchronous functions for each FS operation
/// (`list_dir`, `read_file`, `write_file`, `mkdir`, `touch`, `rm`, etc.).
///
/// The TypeScript side calls this after the user selects a directory via
/// `showDirectoryPicker()` and the cache is populated.
#[wasm_bindgen]
pub fn register_host_fs(mount_id: &str, callbacks: JsValue) -> String {
    ensure_panic_hook();
    let host_fs = WasmHostFs::new(&callbacks);
    HOST_FS_REGISTRY.with(|registry| {
        registry
            .borrow_mut()
            .insert(mount_id.to_string(), Box::new(host_fs));
    });
    "ok".to_string()
}

/// Unregister a host filesystem adapter.
///
/// Called when a directory is unmounted. The `mount_id` must match a
/// previously registered ID.
#[wasm_bindgen]
pub fn unregister_host_fs(mount_id: &str) -> String {
    ensure_panic_hook();
    HOST_FS_REGISTRY.with(|registry| {
        registry.borrow_mut().remove(mount_id);
    });
    "ok".to_string()
}
