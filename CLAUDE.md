# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

NexOS is a browser-based Linux terminal environment. The core logic (VFS, command parsing/execution) is written in Rust compiled to WebAssembly. The frontend is TypeScript + xterm.js. Persistence uses the browser's Origin Private File System (OPFS). There is no backend — everything runs client-side.

## Build Commands

### Full WASM build (Rust → WASM → JS bindings)
```bash
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm
```

### Frontend dev server
```bash
cd web && npm install && npm run dev
```

### Frontend production build
```bash
cd web && npm run build
```

### Run Rust unit tests
```bash
cargo test
```

### Run a single Rust test
```bash
cargo test <test_name>
```

### Deploy (Cloudflare Pages)
```bash
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm
cd web && npm run build && cd ..
npx wrangler pages deploy web/dist/ --project-name=nexos
```

## Architecture

### Data Flow
1. User input → xterm.js → TypeScript frontend
2. Frontend calls `execute_command(input)` via wasm-bindgen
3. Rust `Shell::execute()` parses and dispatches commands
4. Commands operate on the in-memory `Vfs` tree
5. Output string returned to frontend, VFS serialized to JSON and saved to OPFS

### Core Rust Modules (`src/`)

- **`lib.rs`** — WASM entry point. Exports `init`, `init_with_username`, `execute_command`, `get_prompt`, `get_completions`, `get_history`, `get_state_json`. Holds global `Shell` in a `thread_local!` `RefCell`.
- **`vfs/mod.rs`** — Re-exports `Vfs`, `FsNode`, `FileNode`, `DirNode`.
- **`vfs/node.rs`** — Data types: `FsNode` enum (File/Directory), `FileNode`, `DirNode`.
- **`vfs/tree.rs`** — `Vfs` struct with path resolution (handles `.`, `..`, `~`), file/directory CRUD, JSON serialization. All paths are absolute strings internally.
- **`shell/mod.rs`** — `Shell` struct (Vfs + username + hostname + history + env_vars + Registry). Top-level `execute()` handles `&&` chaining and delegates to pipeline/dispatch.
- **`shell/pipeline.rs`** — `split_pipe_stages()` (splits by `|` respecting quotes), `extract_redirect()` (extracts `>`/`>>` targets).
- **`shell/dispatch.rs`** — `execute_with_stdin()` method on Shell. Looks up commands via the Registry and creates a `CommandContext` for execution.
- **`command/mod.rs`** — `Command` trait, `CommandContext` struct, `Registry` struct. The trait defines `name()`, `description()`, `accepts_stdin()`, and `execute()`. The registry is built once at shell init.
- **`command/*.rs`** — One file per command. Each exports a bare `execute()` function AND a struct implementing `Command` that delegates to it.

### Adding a New Command

1. Create `src/command/<name>.rs` with a struct implementing the `Command` trait (name, description, accepts_stdin, execute)
2. Add `pub mod <name>;` to `src/command/mod.rs`
3. Register the command in the `register_all()` function in `src/command/mod.rs`

The trait metadata (`name()`, `accepts_stdin()`) automatically handles tab completion and pipe stdin routing — no need to edit shell.rs.

### Frontend (`web/`)

- **`main.ts`** — Thin bootstrap. Imports modules, creates terminal, loads WASM, runs auth, initializes VFS, wires up input handler.
- **`terminal.ts`** — Terminal creation (`createTerminal()`), addon loading, resize handling (`setupResize()`).
- **`persistence.ts`** — OPFS helpers (`loadFromOPFS()`, `saveToOPFS()`) for VFS state persistence.
- **`input.ts`** — Keyboard input handler (`setupInputHandler()`). Manages input buffer, history, tab completion, command execution.
- **`auth.ts`** — First-time setup vs. returning login flow. SHA-256 password hashing via Web Crypto API. Credentials stored in OPFS `user_config.json`.
- Vite config allows serving from `../pkg/` (the WASM bindings directory).

### Key Design Decisions

- **Command trait + Registry**: All commands implement `Command` with uniform `execute(&self, ctx: &mut CommandContext) -> Result<String, String>`. The registry replaces the monolithic match statement. Adding a command requires only 3 steps (create file, add mod, register).
- Commands return `Result<String, String>` — `Ok` for success output, `Err` for error messages. The `&&` chain breaks on `Err`.
- `CommandContext` bundles all shell state (`vfs`, `stdin`, `args`, `username`, `hostname`, `history`, `env_vars`). Commands borrow only what they need.
- `accepts_stdin()` on the trait replaces the hardcoded `file_reading_commands` array. Commands like `tr` and `tee` access stdin directly via `ctx.stdin` and return `false`.
- Pipe support: stdin from a prior stage is appended as a trailing argument to file-reading commands when no explicit file arg is given.
- Redirection (`>`/`>>`) is only applied on the last stage of a pipeline.
- The `clear` command returns ANSI escape `\x1b[2J\x1b[H` — the frontend interprets this as a screen clear.
- VFS is serialized to JSON after every command execution for OPFS persistence.
- `chmod`/`chown` are simulated (no real permission enforcement).

## Toolchain

- Rust stable with `wasm32-unknown-unknown` target (see `rust-toolchain.toml`)
- `wasm-bindgen` CLI version must match the `wasm-bindgen` crate version in `Cargo.toml`
- Node.js 18+ for the frontend
- Release profile: `opt-level = "z"`, LTO enabled, single codegen unit, symbols stripped
