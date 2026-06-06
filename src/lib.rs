use std::cell::RefCell;
use wasm_bindgen::prelude::*;

mod command;
mod shell;
mod vfs;

use shell::Shell;
use vfs::Vfs;

thread_local! {
    static SHELL: RefCell<Option<Shell>> = RefCell::new(None);
}

/// Initialize the VFS and shell. If `state_json` is non-empty, attempts to
/// restore from the provided JSON; otherwise creates a fresh default VFS.
/// Returns `true` if restored from persisted state.
#[wasm_bindgen]
pub fn init(state_json: &str) -> bool {
    init_with_username(state_json, "user")
}

/// Initialize with a custom username. Used after login when the user has set
/// their own username. Returns `true` if VFS was restored from persisted state.
#[wasm_bindgen]
pub fn init_with_username(state_json: &str, username: &str) -> bool {
    let (vfs, restored) = if state_json.is_empty() {
        (Vfs::new(), false)
    } else {
        match Vfs::from_json(state_json) {
            Ok(v) => (v, true),
            Err(_) => (Vfs::new(), false),
        }
    };

    SHELL.with(|s| {
        let mut shell = Shell::new(vfs);
        shell.username = username.to_string();
        *s.borrow_mut() = Some(shell);
    });

    restored
}

/// Execute a command and return the output text.
/// Automatically saves VFS state after execution (available via `get_state_json()`).
#[wasm_bindgen]
pub fn execute_command(input: &str) -> String {
    SHELL.with(|s| {
        let mut borrow = s.borrow_mut();
        match borrow.as_mut() {
            Some(shell) => shell.execute(input),
            None => "Error: shell not initialized. Call init() first.\n".to_string(),
        }
    })
}

/// Get the current prompt string (with ANSI color codes).
#[wasm_bindgen]
pub fn get_prompt() -> String {
    SHELL.with(|s| {
        let borrow = s.borrow();
        match borrow.as_ref() {
            Some(shell) => shell.get_prompt(),
            None => "$ ".to_string(),
        }
    })
}

/// Get tab completion candidates for the given partial input.
#[wasm_bindgen]
pub fn get_completions(partial: &str) -> Vec<String> {
    SHELL.with(|s| {
        let borrow = s.borrow();
        match borrow.as_ref() {
            Some(shell) => shell.get_completions(partial),
            None => vec![],
        }
    })
}

/// Get the command history.
#[wasm_bindgen]
pub fn get_history() -> Vec<String> {
    SHELL.with(|s| {
        let borrow = s.borrow();
        match borrow.as_ref() {
            Some(shell) => shell.get_history().clone(),
            None => vec![],
        }
    })
}

/// Serialize the current VFS state to JSON for OPFS persistence.
/// Returns empty string if shell is not initialized.
#[wasm_bindgen]
pub fn get_state_json() -> String {
    SHELL.with(|s| {
        let borrow = s.borrow();
        match borrow.as_ref() {
            Some(shell) => shell.vfs.to_json(),
            None => String::new(),
        }
    })
}
