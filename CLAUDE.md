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
- **`vfs.rs`** — Virtual filesystem. Tree of `FsNode` (enum: `File`/`Directory`). Path resolution handles `.`, `..`, `~`. Serialization via serde to JSON. All paths are absolute strings internally.
- **`shell.rs`** — Shell state (`Vfs` + username + hostname + history + env_vars). Command dispatch is a `match` on the command name in `execute_with_stdin()`. Handles `&&` chaining (stop on first error), `|` piping (stdin passed as synthetic argument to file-reading commands), and `>`/`>>` redirection (only on last pipeline stage).
- **`command/`** — One file per command. Each exports a single `execute()` function. Signature varies: most take `(&Vfs, &[&str])` or `(&mut Vfs, &[&str])`; some take `(&str, &[&str])` for stdin (e.g., `tr`).

### Adding a New Command

1. Create `src/command/<name>.rs` with `pub fn execute(...)` returning `Result<String, String>`
2. Add `pub mod <name>;` to `src/command/mod.rs`
3. Add match arm in `src/shell.rs` → `execute_with_stdin()`
4. Add to the commands list in `Shell::get_completions()`
5. If the command reads file content from stdin (piped input), add its name to the `file_reading_commands` array in `execute_with_stdin()`

### Frontend (`web/`)

- **`main.ts`** — App entry. Initializes xterm.js, loads WASM, sets up input handling and OPFS persistence.
- **`auth.ts`** — First-time setup vs. returning login flow. SHA-256 password hashing via Web Crypto API. Credentials stored in OPFS `user_config.json`.
- **`shell.ts`** — Frontend shell helpers.
- Vite config allows serving from `../pkg/` (the WASM bindings directory).

### Key Design Decisions

- Commands return `Result<String, String>` — `Ok` for success output, `Err` for error messages. The `&&` chain breaks on `Err`.
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
