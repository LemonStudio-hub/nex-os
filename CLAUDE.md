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
cargo test --target x86_64-unknown-linux-gnu
```

### Run a single Rust test
```bash
cargo test --target x86_64-unknown-linux-gnu <test_name>
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
2. Frontend calls `execute_command(state_json, input)` via wasm-bindgen
3. Rust `Service::execute_command()` parses and dispatches commands against the `ShellState`
4. Commands operate on the in-memory `Vfs` tree within `ShellState`
5. Output + updated state JSON returned to frontend, VFS serialized to OPFS

### Core Rust Modules (`src/`)

- **`lib.rs`** — WASM entry point. Exports 13 functions: `init`, `init_with_username`, `execute_command`, `get_prompt`, `get_completions`, `get_history`, `get_state_json`, `get_dirty_files_json`, `get_deleted_files_json`, `get_file_content`, `mark_all_dirty`, `get_tree_json`, `register_host_fs`, `unregister_host_fs`. Holds immutable `Service` and `HOST_FS_REGISTRY` in `thread_local!` `RefCell`s. All exports accept `state_json: &str` — the frontend owns and passes state.
- **`vfs/mod.rs`** — Re-exports `Vfs`, `FsNode`, `FileNode`, `DirNode`, `ChunkedContent`, `HostFs`, `HostEntry`. Submodules: `host_fs` (trait), `host_fs_wasm` (JS callback bridge).
- **`vfs/node.rs`** — Data types: `FsNode` enum (File/Directory), `FileNode` (name + `ChunkedContent`), `DirNode` (name + HashMap children). `ChunkedContent` provides chunked string storage (64 KiB chunks, 4 KiB inline threshold) with custom serde supporting legacy plain-string and new chunked-object formats.
- **`vfs/tree.rs`** — `Vfs` struct with path resolution (handles `.`, `..`, `~`), file/directory CRUD, dirty tracking (`dirty_files`/`deleted_files` HashSets, `serde(skip)`), partial-read methods (`read_file_lines`, `file_line_count`, `file_size`), `to_tree_json()` for skeleton-only serialization, and full JSON roundtrip. All paths are absolute strings internally.
- **`shell/mod.rs`** — `ShellState` (serializable: Vfs + username + hostname + history + env_vars) and `Service` (stateless: Registry only). `Service::execute_command()` handles `&&` chaining and delegates to pipeline/dispatch.
- **`shell/pipeline.rs`** — `split_pipe_stages()` (splits by `|` respecting quotes), `extract_redirect()` (extracts `>`/`>>` targets).
- **`shell/dispatch.rs`** — `Service::execute_with_stdin()` method. Looks up commands via the Registry and creates a `CommandContext` for execution.
- **`command/mod.rs`** — `Command` trait (7 methods: `name()`, `description()`, `execute()` required; `accepts_stdin()`, `synopsis()`, `man_description()`, `examples()` with defaults), `CommandContext` struct, `Registry` struct. The registry is built once at service init.
- **`command/*.rs`** — One file per command. Each exports a bare `execute()` function AND a struct implementing `Command` that delegates to it.

### Host Directory Mounting

NexOS supports mounting real directories from the host machine into the VFS using the browser's File System Access API (`showDirectoryPicker`).

**Architecture:**
- `src/vfs/host_fs.rs` — `HostFs` trait defining sync FS operations (`list_dir`, `read_file`, `write_file`, `mkdir`, `rm`, etc.)
- `src/vfs/host_fs_wasm.rs` — `WasmHostFs` struct implementing `HostFs` via `js_sys::Function` callbacks from TypeScript
- `src/command/mount.rs` — `mount` / `mount -u` command (lists mounts, requests picker, unmounts)
- `web/host-fs.ts` — `HostFsManager` class managing `FileSystemDirectoryHandle` objects with a pre-cache strategy

**How it works:**
1. User types `mount /mnt/project` → WASM creates VFS directory and mount metadata, returns `__MOUNT_REQUEST__` marker
2. Frontend detects marker, calls `showDirectoryPicker()` (user gesture required)
3. `HostFsManager.mount()` recursively caches directory contents into a `Map<string, string>`
4. Synchronous callback functions are registered with WASM via `register_host_fs()`
5. All subsequent VFS operations on mounted paths delegate to `HostFs` through `_with_host` method variants
6. Writes are queued as async promises and flushed after each command execution

**Key design:** Mount metadata (`Vfs.mounts: HashMap<String, String>`) is serialized with the VFS and survives OPFS persistence. The `FileSystemDirectoryHandle` objects are ephemeral — on page reload, users must re-authorize previously mounted directories.

### Adding a New Command

1. Create `src/command/<name>.rs` with a struct implementing the `Command` trait (name, description, accepts_stdin, execute)
2. Add `pub mod <name>;` to `src/command/mod.rs`
3. Register the command in the `register_all()` function in `src/command/mod.rs`

The trait metadata (`name()`, `accepts_stdin()`) automatically handles tab completion and pipe stdin routing — no need to edit shell.rs.

### Frontend (`web/`)

- **`main.ts`** — Bootstrap. Creates terminal, loads WASM, runs auth, restores VFS from OPFS (with migration from legacy to incremental format), creates `HostFsManager`, prompts for re-mount on reload, hands off to input handler.
- **`terminal.ts`** — Terminal creation (`createTerminal()`), addon loading, resize handling (`setupResize()`).
- **`persistence.ts`** — OPFS incremental persistence: `loadFromOPFS()` tries new `nexos_tree.json` + `nexos_files/` format, falls back to legacy `vfs_state.json`; `saveToOPFS()` writes only dirty files individually, deletes removed files, saves tree skeleton. Path encoding uses `btoa(encodeURIComponent(path))` with filesystem-safe character replacement.
- **`input.ts`** — Keyboard input handler (`setupInputHandler()`). Manages input buffer, history, tab completion, command execution. Defines `WasmApi` interface matching all 13 WASM exports. Detects `__MOUNT_REQUEST__` markers and opens directory picker. Calls `onSaveState` and `hostFsManager.flushWrites()` after each command.
- **`auth.ts`** — First-time setup vs. returning login flow. Argon2id password hashing (64 MiB memory, 3 iterations, 16-byte salt) via `hash-wasm`. Legacy SHA-256 hashes are transparently migrated to Argon2id on successful login. Credentials stored in OPFS `user_config.json`.
- **`host-fs.ts`** — `HostFsManager` class: manages `FileSystemDirectoryHandle` objects, pre-caches directory contents for synchronous WASM access, queues async writes for flush after each command.
- Vite config allows serving from `../pkg/` (the WASM bindings directory).

### Key Design Decisions

- **Stateless Service + ShellState**: The `Service` holds only the immutable `Registry`. All mutable data lives in `ShellState` (Vfs, history, env_vars, identity). Every WASM export accepts `state_json: &str` and returns results alongside the updated state. This enables async execution and parallel Worker invocation — each Worker holds its own state independently.
- **Command trait + Registry**: All commands implement `Command` with uniform `execute(&self, ctx: &mut CommandContext) -> Result<String, String>`. The registry replaces the monolithic match statement. Adding a command requires only 3 steps (create file, add mod, register).
- Commands return `Result<String, String>` — `Ok` for success output, `Err` for error messages. The `&&` chain breaks on `Err`.
- `CommandContext` bundles all shell state via `state: &mut ShellState` (Vfs, env_vars, history, identity) plus `stdin`, `args`, and `registry`. Commands access fields through `ctx.state.*`.
- `accepts_stdin()` on the trait replaces the hardcoded `file_reading_commands` array. Commands like `tr` and `tee` access stdin directly via `ctx.stdin` and return `false`.
- Pipe support: stdin from a prior stage is appended as a trailing argument to file-reading commands when no explicit file arg is given.
- Redirection (`>`/`>>`) is only applied on the last stage of a pipeline.
- The `clear` command returns ANSI escape `\x1b[2J\x1b[H` — the frontend interprets this as a screen clear.
- Shell state is serialized to JSON after every command execution. OPFS persistence is incremental: dirty files are written individually to `nexos_files/`, deleted files are removed, and a tree skeleton (`nexos_tree.json`) is saved separately. This avoids rewriting the entire VFS on every command.
- `chmod`/`chown` are simulated (no real permission enforcement).

## Toolchain

- Rust stable with `wasm32-unknown-unknown` target (see `rust-toolchain.toml`)
- `wasm-bindgen` CLI version must match the `wasm-bindgen` crate version in `Cargo.toml`
- Node.js 18+ for the frontend
- Release profile: `opt-level = "z"`, LTO enabled, single codegen unit, symbols stripped
