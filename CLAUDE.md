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
5. Returns JSON `{"stdout", "stderr", "exit_code", "state", "action"}` — frontend displays streams, stores updated state, and handles any special action (mount/upload/download requests)

### Core Rust Modules (`src/`)

- **`lib.rs`** — WASM entry point. Exports 18 functions: `init`, `init_with_username`, `execute_command`, `get_prompt`, `get_completions`, `get_history`, `get_state_json`, `get_dirty_files_json`, `get_deleted_files_json`, `get_file_content`, `mark_all_dirty`, `get_tree_json`, `register_host_fs`, `unregister_host_fs`, `write_file_to_vfs`, `get_state_dirty_flags`, `mark_state_clean`, `get_non_vfs_state_json`, `apply_saved_state`. Holds immutable `Service` and `HOST_FS_REGISTRY` in `thread_local!` `RefCell`s. All exports accept `state_json: &str` — the frontend owns and passes state.
- **`vfs/mod.rs`** — Re-exports `Vfs`, `FsNode`, `FileNode`, `DirNode`, `ChunkedContent`, `HostFs`, `HostEntry`, `NodeMeta`, `UserDatabase`. Submodules: `host_fs` (trait), `host_fs_wasm` (JS callback bridge), `permissions` (Unix permission system), `user_db` (user/group/sudoers parsing).
- **`vfs/node.rs`** — Data types: `FsNode` enum (File/Directory), `FileNode` (name + `ChunkedContent` + `NodeMeta`), `DirNode` (name + HashMap children + `NodeMeta`). `ChunkedContent` provides chunked string storage (64 KiB chunks, 4 KiB inline threshold) with custom serde supporting legacy plain-string and new chunked-object formats. Each node carries `NodeMeta` (mode, uid, gid, mtime) with `#[serde(default)]` for backward compatibility.
- **`vfs/tree.rs`** — `Vfs` struct with path resolution (handles `.`, `..`, `~`), file/directory CRUD, dirty tracking (`dirty_files`/`deleted_files` HashSets, `serde(skip)`), partial-read methods (`read_file_lines`, `file_line_count`, `file_size`), `to_tree_json()` for skeleton-only serialization, and full JSON roundtrip. All paths are absolute strings internally. Default tree assigns root:root ownership, sticky bit on `/tmp`, and uid/gid 1000 on `/home/user`. Provides `mkdir_with_owner()`, `touch_with_owner()`, `write_file_with_owner()` variants.
- **`vfs/permissions.rs`** — Unix-style permission system: `NodeMeta` struct (mode u16, uid, gid, mtime), `AccessMode` enum (Read/Write/Execute), `check_access()` (enforces rwx bits, root bypasses all), `check_delete_in_sticky()` (sticky bit enforcement), `parse_octal_mode()`, `apply_symbolic_mode()` (supports `+x`, `-w`, `u+r`, `a+x`, `g-rwx`), `format_mode()` (10-char `rwxrwxrwx` string for `ls -l`).
- **`vfs/user_db.rs`** — Parses `/etc/passwd`, `/etc/group`, `/etc/sudoers` into `UserDatabase` struct. Provides `find_user_by_uid/name`, `find_group_by_gid/name`, `has_nopasswd_sudo`, `next_uid/gid`, `user_groups`. Supports simplified sudoers format (`user ALL=(ALL) NOPASSWD: ALL`). Rebuilt from VFS on every state load (not persisted directly).
- **`shell/mod.rs`** — `ShellState` (serializable: Vfs + username + hostname + uid + gid + euid + history + env_vars + last_exit_code + dirty_state) and `Service` (stateless: Registry only). `Service::execute_command()` returns `CommandOutput` (stdout, stderr, exit_code, action) and handles `&&` chaining. `StateDirtyFlags` tracks non-VFS state changes (history, env_vars) for incremental persistence. `to_state_json()` / `from_state_json_with_vfs()` enable separate persistence of non-VFS state to `nexos_state.json`. `bootstrap_permissions()` creates `/etc/passwd`, `/etc/group`, `/etc/sudoers` on first run. `refresh_user_db()` rebuilds the cached `UserDatabase` after user/group modifications.
- **`shell/pipeline.rs`** — `split_pipe_stages()` (splits by `|` respecting quotes), `extract_redirect()` (extracts `>`/`>>` targets).
- **`shell/dispatch.rs`** — `Service::execute_with_stdin()` method. Looks up commands via the Registry and creates a `CommandContext` for execution.
- **`command/mod.rs`** — `Command` trait (7 methods: `name()`, `description()`, `execute()` required; `accepts_stdin()`, `synopsis()`, `man_description()`, `examples()` with defaults), `CommandOutput` struct (stdout, stderr, exit_code, action), `CommandContext` struct, `Registry` struct. `CommandOutput` replaces the old `Result<String, String>` return type — `From<Result<String, String>>` is implemented for backward compatibility. The registry is built once at service init.
- **`command/*.rs`** — One file per command. Each exports a bare `execute()` function AND a struct implementing `Command` that delegates to it.

### Unix Permissions and User Management

NexOS implements a simulated Unix permission and multi-user system. All permission data is stored in VFS metadata and `/etc/` files, enforced at the command level.

**Permissions (`vfs/permissions.rs`):**
- Every VFS node (file/directory) carries `NodeMeta`: `mode` (u16 permission bits), `uid`, `gid`, `mtime` (epoch seconds).
- `check_access()` enforces standard rwx bits (owner → group → other). Root (uid 0) always bypasses.
- Sticky bit (`0o1000`) on `/tmp` — only file owner, dir owner, or root can delete.
- `chmod` supports both octal (`755`, `0644`) and symbolic (`+x`, `u+r`, `g-rwx`) modes. Only owner or root can change permissions.
- `chown` supports `owner[:group]` syntax with name or numeric resolution. Only root can change ownership.
- `ls -l` displays `rwxrwxrwx` strings, owner/group names (resolved via UserDatabase), size, and mtime.

**User/Group Database (`vfs/user_db.rs`):**
- `UserDatabase` parses `/etc/passwd`, `/etc/group`, `/etc/sudoers` into structured entries.
- `/etc/passwd` format: `username:x:uid:gid:gecos:home_dir:shell`
- `/etc/group` format: `groupname:x:gid:member1,member2`
- `/etc/sudoers` format: `user ALL=(ALL) NOPASSWD: ALL`
- Database is rebuilt from VFS on every state load (not persisted separately).

**Shell state fields:** `uid`, `gid`, `euid` (effective UID for sudo), `user_db` (cached, `#[serde(skip)]`).

**Bootstrap:** `bootstrap_permissions()` runs on init — creates `/etc/passwd` (root + current user), `/etc/group` (root + user group), `/etc/sudoers` (NOPASSWD ALL for current user) if missing. Default user gets uid/gid 1000.

**Commands:** `id`, `groups`, `useradd`/`adduser`, `groupadd`, `passwd`, `su`, `sudo`. See the command list below for details.

### Host Directory Mounting

NexOS supports mounting real directories from the host machine into the VFS using the browser's File System Access API (`showDirectoryPicker`).

**Architecture:**
- `src/vfs/host_fs.rs` — `HostFs` trait defining sync FS operations (`list_dir`, `read_file`, `write_file`, `mkdir`, `rm`, etc.)
- `src/vfs/host_fs_wasm.rs` — `WasmHostFs` struct implementing `HostFs` via `js_sys::Function` callbacks from TypeScript
- `src/command/mount.rs` — `mount` / `mount -u` command (lists mounts, requests picker, unmounts)
- `web/host-fs.ts` — `HostFsManager` class managing `FileSystemDirectoryHandle` objects with a pre-cache strategy

**How it works:**
1. User types `mount /mnt/project` → WASM creates VFS directory and mount metadata, returns `CommandOutput` with `action: "mount_request:/mnt/project"`
2. Frontend detects the action in `input.ts`, calls `showDirectoryPicker()` (user gesture required)
3. `HostFsManager.mount()` recursively caches directory contents into a `Map<string, string>`
4. Synchronous callback functions are registered with WASM via `register_host_fs()`
5. All subsequent VFS operations on mounted paths delegate to `HostFs` through `_with_host` method variants
6. Writes are queued as async promises and flushed after each command execution

**Key design:** Mount metadata (`Vfs.mounts: HashMap<String, String>`) is serialized with the VFS and survives OPFS persistence. The `FileSystemDirectoryHandle` objects are ephemeral — on page reload, users must re-authorize previously mounted directories.

### Upload and Download Commands

NexOS supports transferring files between the host machine and the VFS using the browser's File System Access API.

**`upload [destination_path]`** — Opens the browser's file picker to select files from the host, then writes them into the VFS at the specified directory (defaults to cwd).

**`download <file_path>`** — Triggers a browser save/download of a VFS file to the host machine. Uses `showSaveFilePicker` with a blob fallback.

**How it works:**
1. Commands return `CommandOutput` with an `action` field (e.g. `upload_request:/home/user` or `download_request:file.txt\n/home/user/file.txt`)
2. Frontend detects the action in `input.ts` and calls the appropriate async handler
3. For upload: `showOpenFilePicker({multiple: true})` → read files → `write_file_to_vfs()` WASM export
4. For download: `get_file_content()` → `showSaveFilePicker()` or blob+anchor fallback

**`write_file_to_vfs(state_json, path, content)`** — New WASM export that writes file content directly into the VFS, creating parent directories as needed. Used by the upload flow after the frontend reads file content from the host.

### Command List (53 commands)

**Filesystem navigation:** `ls` (`-l`, `-a`, `-h`, `-R`, `-t`, `-r`, `-S`), `cd`, `pwd`, `mkdir` (`-p`), `touch`, `rm` (`-r`, `-f`), `cp` (`-r`), `mv`, `tree`, `ln` (`-s`)

**File content:** `cat` (`-n`), `echo`, `head` (`-n`), `tail` (`-n`)

**Text processing:** `grep` (`-i`, `-v`, `-n`, `-c`, `-l`, `-r`), `sort` (`-r`, `-n`, `-u`, `-k`, `-t`), `uniq` (`-c`, `-d`), `wc` (`-l`, `-w`, `-c`), `cut` (`-d`, `-f`), `tr`, `tee`, `comm` (`-1`, `-2`, `-3`), `nl`, `paste` (`-d`), `rev`, `seq`, `tac`, `yes` (`-n`), `printf`

**Diff:** `diff` (`-u`, `-y`, `--color`)

**Search:** `find` (`-name`, `-type`, `-mtime`, `-size`)

**Disk usage:** `du` (`-h`, `-s`)

**Permissions & ownership:** `chmod` (octal + symbolic), `chown` (`owner[:group]`)

**User & group management:** `id`, `groups`, `useradd`/`adduser` (`-m`, `-s`, `-g`), `groupadd` (`-g`), `passwd`, `su` (`-`), `sudo` (`-u`)

**System info:** `whoami`, `hostname`, `date`, `history`

**Environment:** `env`, `export`

**Path utilities:** `basename`, `dirname`

**Mount:** `mount` (`-u`)

**Upload/Download:** `upload`, `download`

**Documentation:** `man`

**Terminal:** `clear`, `help`, `exit`

### Adding a New Command

1. Create `src/command/<name>.rs` with a struct implementing the `Command` trait (name, description, accepts_stdin, execute returning `CommandOutput`)
2. Add `pub mod <name>;` to `src/command/mod.rs`
3. Register the command in the `register_all()` function in `src/command/mod.rs`

The trait metadata (`name()`, `accepts_stdin()`) automatically handles tab completion and pipe stdin routing — no need to edit shell.rs.

### Frontend (`web/`)

- **`main.ts`** — Bootstrap. Creates terminal, loads WASM, runs auth, restores VFS from OPFS (with migration from legacy to incremental format), creates `HostFsManager`, prompts for re-mount on reload, hands off to input handler.
- **`terminal.ts`** — Terminal creation (`createTerminal()`), addon loading, resize handling (`setupResize()`).
- **`persistence.ts`** — OPFS incremental persistence: `loadFromOPFS()` tries new `nexos_tree.json` + `nexos_files/` format, falls back to legacy `vfs_state.json`; also loads non-VFS state from `nexos_state.json`. `saveToOPFS()` writes only dirty files individually, deletes removed files, saves tree skeleton, and persists non-VFS state (history, env_vars, hostname) when dirty. Path encoding uses `btoa(encodeURIComponent(path))` with filesystem-safe character replacement.
- **`input.ts`** — Keyboard input handler (`setupInputHandler()`). Manages input buffer, history, tab completion, command execution. Defines `WasmApi` interface matching all 18 WASM exports. Detects `mount_request`, `upload_request`, and `download_request` actions from command output. Calls `onSaveState` and `hostFsManager.flushWrites()` after each command.
- **`auth.ts`** — First-time setup vs. returning login flow. Argon2id password hashing (64 MiB memory, 3 iterations, 16-byte salt) via `hash-wasm`. Legacy SHA-256 hashes are transparently migrated to Argon2id on successful login. Credentials stored in OPFS `user_config.json`.
- **`host-fs.ts`** — `HostFsManager` class: manages `FileSystemDirectoryHandle` objects, pre-caches directory contents for synchronous WASM access, queues async writes for flush after each command.
- Vite config allows serving from `../pkg/` (the WASM bindings directory).

### Key Design Decisions

- **Stateless Service + ShellState**: The `Service` holds only the immutable `Registry`. All mutable data lives in `ShellState` (Vfs, history, env_vars, identity, last_exit_code, dirty_state). Every WASM export accepts `state_json: &str` and returns results alongside the updated state. This enables async execution and parallel Worker invocation — each Worker holds its own state independently.
- **Command trait + Registry**: All commands implement `Command` with uniform `execute(&self, ctx: &mut CommandContext) -> CommandOutput`. The registry replaces the monolithic match statement. Adding a command requires only 3 steps (create file, add mod, register).
- Commands return `CommandOutput` with separate stdout, stderr, exit_code, and an optional action string. Legacy `Result<String, String>` is supported via `From` conversion. The `&&` chain stops when `exit_code != 0`.
- `CommandContext` bundles all shell state via `state: &mut ShellState` (Vfs, env_vars, history, identity) plus `stdin`, `args`, and `registry`. Commands access fields through `ctx.state.*`.
- `accepts_stdin()` on the trait replaces the hardcoded `file_reading_commands` array. Commands like `tr` and `tee` access stdin directly via `ctx.stdin` and return `false`.
- Pipe support: stdin from a prior stage is appended as a trailing argument to file-reading commands when no explicit file arg is given.
- Redirection (`>`/`>>`) is only applied on the last stage of a pipeline.
- The `clear` command returns ANSI escape `\x1b[2J\x1b[H` — the frontend interprets this as a screen clear.
- Shell state is serialized to JSON after every command execution. OPFS persistence is incremental: dirty files are written individually to `nexos_files/`, deleted files are removed, a tree skeleton (`nexos_tree.json`) is saved separately, and non-VFS state (history, env_vars, hostname) is persisted to `nexos_state.json` when dirty. `StateDirtyFlags` tracks which non-VFS fields have changed. This avoids rewriting the entire VFS on every command.
- `chmod`/`chown` enforce permission bits and ownership via `check_access()`. Root (uid 0) bypasses all checks. The sticky bit is enforced on `/tmp`.

## Toolchain

- Rust stable with `wasm32-unknown-unknown` target (see `rust-toolchain.toml`)
- `wasm-bindgen` CLI version must match the `wasm-bindgen` crate version in `Cargo.toml`
- Node.js 18+ for the frontend
- Release profile: `opt-level = "z"`, LTO enabled, single codegen unit, symbols stripped
